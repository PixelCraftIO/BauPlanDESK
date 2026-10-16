//! Ableitbarer Volltextindex je unveränderlicher Dokumentversion; Originale
//! bleiben im DMS. Extraktionsherkunft wird sichtbar gespeichert.
use super::*;
use crate::storage::ContentHash;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Textquelle {
    Pdf,
    Email,
    Ocr,
    Manuell,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Suchtext {
    pub dokument: Dokumentbezug,
    pub sha256: String,
    pub quelle: Textquelle,
    pub text: String,
    pub indexiert_am: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Suchtreffer {
    pub titel: String,
    pub bereich: String,
    pub auszug: String,
    pub verfahren: Option<Uuid>,
    pub dokument: Option<Dokumentbezug>,
}
impl CoreServices {
    pub async fn suchtext_speichern(
        &self,
        p: ProjectId,
        d: Dokumentbezug,
        quelle: Textquelle,
        text: String,
        wer: &Benutzer,
    ) -> CoreResult<()> {
        self.pruefe(wer, Recht::ProjektBearbeiten).await?;
        self.projekt_zugriff(p, wer, crate::zugriff::ProjektRecht::Bearbeiten)
            .await?;
        if text.len() > 2 * 1024 * 1024 {
            return Err(fehler("Suchtext höchstens 2 MB"));
        }
        let meta = self
            .document_history(d.dokument_id)
            .await?
            .into_iter()
            .find(|v| v.project_id == p && v.version == d.version)
            .ok_or_else(|| fehler("Suchtext gehört nicht zur Projektdatei"))?;
        let key = format!("{}/{}", d.dokument_id.0, d.version);
        let alt = self
            .repos
            .commit
            .fachstaende(p, "suchtexte")
            .await?
            .into_iter()
            .find(|f| f.objekt == key);
        let basis = alt.as_ref().map(|f| f.revision).unwrap_or(0);
        let index = Suchtext {
            dokument: d,
            sha256: meta.sha256.to_string(),
            quelle,
            text,
            indexiert_am: Utc::now(),
        };
        let f = Fachstand {
            projekt_id: p,
            modul: "suchtexte".into(),
            objekt: key,
            schema_version: 1,
            revision: basis,
            daten: serde_json::to_value(index).map_err(|e| fehler(e.to_string()))?,
        };
        self.repos
            .commit
            .fachstand(
                f,
                basis,
                Self::audit(
                    wer,
                    crate::domain::Aktion::ProjektGeaendert,
                    Some(p),
                    Some(meta.document_id),
                    "Suchtext aktualisiert".into(),
                ),
            )
            .await?;
        Ok(())
    }
    pub async fn verfahren_suche(&self, p: ProjectId, suche: &str) -> CoreResult<Vec<Suchtreffer>> {
        self.get_project(p).await?;
        if !(2..=200).contains(&suche.chars().count()) {
            return Err(fehler("Suchbegriff mit 2 bis 200 Zeichen erforderlich"));
        }
        let q = suche.to_lowercase();
        let mut r = Vec::new();
        let mut add = |titel: String,
                       bereich: &str,
                       text: String,
                       verfahren: Option<Uuid>,
                       dokument: Option<Dokumentbezug>| {
            if r.len() < 100
                && (text.to_lowercase().contains(&q) || titel.to_lowercase().contains(&q))
            {
                let zeichen: Vec<char> = text.chars().collect();
                let start = text
                    .to_lowercase()
                    .find(&q)
                    .map(|b| text.to_lowercase()[..b].chars().count().saturating_sub(70))
                    .unwrap_or(0);
                let auszug = zeichen.into_iter().skip(start).take(300).collect();
                r.push(Suchtreffer {
                    titel,
                    bereich: bereich.into(),
                    auszug,
                    verfahren,
                    dokument,
                });
            }
        };
        for f in self.repos.commit.fachstaende(p, "verfahren").await? {
            let a: Verfahrensakte =
                serde_json::from_value(f.daten).map_err(|e| fehler(e.to_string()))?;
            let id = Uuid::parse_str(&f.objekt).map_err(|e| fehler(e.to_string()))?;
            add(
                a.bezeichnung.clone(),
                "Verfahren",
                format!(
                    "{} {} {}",
                    a.aktenzeichen, a.behoerde.name, a.behoerde.kontakt
                ),
                Some(id),
                None,
            );
            for c in a.kommunikation {
                add(
                    c.betreff,
                    "Kommunikation",
                    format!("{} {}", c.beteiligte, c.inhalt),
                    Some(id),
                    c.original,
                );
            }
            for n in a.nachforderungen {
                for p in n.punkte {
                    add(
                        p.titel,
                        "Nachforderung",
                        format!("{} {}", p.grundlage, p.antwort),
                        Some(id),
                        None,
                    );
                }
            }
            for e in a.ereignisse {
                add(e.text.clone(), "Chronik", e.text, Some(id), e.beleg);
            }
            for e in a.anforderungen {
                add(
                    e.titel,
                    "Anforderung",
                    format!("{} {}", e.grund, e.quelle),
                    Some(id),
                    None,
                );
            }
            for b in a.bescheide {
                for e in b.auflagen {
                    add(
                        e.titel,
                        "Auflage",
                        e.ausloeser,
                        Some(id),
                        Some(b.original.clone()),
                    );
                }
            }
        }
        let docs = self.list_documents(p).await?;
        for d in &docs {
            add(
                format!("{} · Version {}", d.file_name, d.version),
                "Dateiname",
                d.file_name.clone(),
                None,
                Some(Dokumentbezug {
                    dokument_id: d.document_id,
                    version: d.version,
                }),
            );
        }
        for f in self.repos.commit.fachstaende(p, "suchtexte").await? {
            let index: Suchtext =
                serde_json::from_value(f.daten).map_err(|e| fehler(e.to_string()))?;
            if let Some(d) = docs.iter().find(|d| {
                d.document_id == index.dokument.dokument_id
                    && d.version == index.dokument.version
                    && d.sha256.to_string() == index.sha256
            }) {
                add(
                    format!(
                        "{} · Version {} · {:?}",
                        d.file_name, d.version, index.quelle
                    ),
                    "Dokumentinhalt",
                    index.text,
                    None,
                    Some(index.dokument),
                );
            }
        }
        Ok(r)
    }
    /// Vollständiger portabler Aktenexport inklusive aller Dateiversionen,
    /// Fachhistorien und unveränderlicher Abgabe-Artefakte.
    pub async fn verfahren_aktenexport(&self, p: ProjectId) -> CoreResult<Vec<u8>> {
        use std::io::{Cursor, Write};
        let projekt = self.get_project(p).await?;
        let docs = self.list_documents(p).await?;
        if docs.len() > 8000 {
            return Err(fehler("Aktenexport höchstens 8000 Dateiversionen"));
        }
        let mut historie = Vec::new();
        let mut artefakte = Vec::new();
        for modul in [
            "projektorganisation",
            "gebaeude",
            "aufgaben",
            "unterlagen",
            "dokumentdaten",
            "raumbuch",
            "stellplaetze",
            "wohnflaechen",
            "din277",
            "vollgeschosse",
            "hoehen",
            "freiflaechen",
            "entwaesserung",
            "ueberbaubarkeit",
            "nachweise",
            "freigaben",
            "verfahren",
            "abstandsflaechen",
            "suchtexte",
            "dokumentkategorie",
        ] {
            for f in self.repos.commit.fachstaende(p, modul).await? {
                if modul == "verfahren" {
                    let a: Verfahrensakte = serde_json::from_value(f.daten.clone())
                        .map_err(|e| fehler(e.to_string()))?;
                    for b in a.abgaben {
                        for art in b.artefakte {
                            artefakte.push(art);
                        }
                    }
                }
                historie.extend(self.repos.commit.fachverlauf(p, modul, &f.objekt).await?);
            }
        }
        let einreichungen = self.einreichungen(p).await?;
        let mut zusaetze = Vec::new();
        for d in &docs {
            for modul in [
                "formularinstanz",
                "emailquelle",
                "dokumentkategorie",
                "ausgabeauftrag",
                "ausgabeereignis",
            ] {
                if let Some(z) = self
                    .repos
                    .commit
                    .dokumentzusatz(d.document_id, d.version, modul)
                    .await?
                {
                    zusaetze.push(serde_json::json!({"dokument_id":d.document_id,"version":d.version,"zusatz":z}));
                }
            }
        }
        let manifest = serde_json::json!({"schema_version":1,"projekt":projekt,"dokumente":docs,"dokumentzusatz":zusaetze,"kategorienkatalog":self.kategorienkatalog().await?,"fachhistorie":historie,"abgabe_artefakte":artefakte,"einreichungen":einreichungen,"bauantrag":self.bauantrag(p).await?,"bauherren":self.bauherren(p).await?,"protokoll":self.repos.protokoll.list_for_project(p,100000).await?});
        let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let opt = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let mut sum = 0u64;
        let mut put = |name: String, data: &[u8]| -> CoreResult<()> {
            sum = sum.saturating_add(data.len() as u64);
            if sum > 512 * 1024 * 1024 {
                return Err(fehler("Aktenexport über 512 MB"));
            }
            z.start_file(name, opt).map_err(|e| fehler(e.to_string()))?;
            z.write_all(data).map_err(|e| fehler(e.to_string()))?;
            Ok(())
        };
        put(
            "Projektakte.json".into(),
            &serde_json::to_vec_pretty(&manifest).map_err(|e| fehler(e.to_string()))?,
        )?;
        let mut index = String::from(
            "Projektakte\n\nAlle Dokumentversionen befinden sich im Ordner Dokumente. Projektakte.json enthält Fachhistorien, Originalbezüge, SHA-256-Prüfsummen und Protokoll.\n\n",
        );
        let mut vorlagen = std::collections::HashSet::new();
        for d in &docs {
            let pfad = format!(
                "Dokumente/{}/v{}-{}",
                d.document_id.0, d.version, d.file_name
            );
            index.push_str(&format!(
                "{} · Version {} · {}\n",
                d.file_name, d.version, pfad
            ));
            if let Some(i) = self.formular_instanz(d.document_id, d.version).await? {
                put(
                    format!("Formularquellen/{}/v{}.json", d.document_id.0, d.version),
                    &serde_json::to_vec_pretty(&i).map_err(|e| fehler(e.to_string()))?,
                )?;
                if vorlagen.insert(i.vorlagen_hash.to_string()) {
                    put(
                        format!("Formularquellen/{}.pdf", i.vorlagen_hash),
                        &self
                            .storage
                            .load_verified(
                                &crate::storage::StorageKey::new(i.vorlagen_storage_key)?,
                                &i.vorlagen_hash,
                            )
                            .await?,
                    )?;
                }
            }
            put(pfad, &self.load_version(d.document_id, d.version).await?.1)?;
        }
        put("Liesmich.txt".into(), index.as_bytes())?;
        for a in artefakte {
            put(
                format!("Abgaben/{}-{}", a.sha256, a.dateiname),
                &self
                    .storage
                    .load_verified(
                        &crate::storage::StorageKey::new(a.storage_key)?,
                        &ContentHash::try_from(a.sha256)?,
                    )
                    .await?,
            )?;
        }
        for e in einreichungen {
            put(
                format!("Einreichungen/{}.zip", e.stand.id),
                &self.einreichung_zip(e.stand.id).await?.1,
            )?;
        }
        Ok(z.finish().map_err(|e| fehler(e.to_string()))?.into_inner())
    }
}
