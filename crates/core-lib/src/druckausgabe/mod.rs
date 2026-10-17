//! Lesende Ausgabe: genaue Quellversionen, archivierte Nachweise und getrennte
//! Fachberichte. Erzeugt keine DMS-Version und bestätigt keine Einreichung.
mod abstandsbericht;
mod berichte;
mod layout;
mod pdf;
mod vorlage;
use crate::{
    CoreError, CoreResult, CoreServices,
    domain::{DocumentId, ProjectId},
    plattform::ObjektArt,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub use vorlage::Druckgestaltung;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "art", rename_all = "snake_case", deny_unknown_fields)]
pub enum Druckquelle {
    Dokument {
        dokument_id: DocumentId,
        #[ts(type = "number")]
        version: i64,
        #[serde(default)]
        seiten: Vec<u32>,
    },
    Nachweis {
        id: Uuid,
    },
    Baugrundstueck,
    Baugrenzen,
    Raumbuch {
        id: Uuid,
        #[ts(type = "number")]
        revision: i64,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Druckauftrag {
    /// Fehlend: gespeicherter Bürostandard. Explizit: Gestaltung dieser Ausgabe.
    #[serde(default)]
    pub gestaltung: Option<Druckgestaltung>,
    #[serde(default)]
    pub aktenzeichen: String,
    #[serde(default)]
    pub datum: Option<chrono::NaiveDate>,
    pub titel: String,
    pub absender: String,
    pub empfaenger: String,
    pub anschreiben: String,
    pub mit_anschreiben: bool,
    pub inhaltsverzeichnis: bool,
    /// Schützt aktuelle Fachberichte vor Änderungen während der Zusammenstellung.
    #[ts(type = "number")]
    pub projekt_version: i64,
    pub quellen: Vec<Druckquelle>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Druckeintrag {
    pub titel: String,
    pub quelle: Druckquelle,
    pub hinweis: Option<String>,
    pub verfuegbar: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Druckkatalog {
    pub gestaltung: Druckgestaltung,
    #[ts(type = "number")]
    pub projekt_version: i64,
    pub absender: String,
    pub eintraege: Vec<Druckeintrag>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Druckposition {
    pub titel: String,
    pub erste_seite: u32,
    pub seiten: u32,
}
/// Interne Ausgabebeschriftung; keine Ableitung aus formatierten Dateititeln.
struct Inhaltsangabe {
    titel: String,
    kategorie: String,
    name: String,
    version: String,
}
impl From<String> for Inhaltsangabe {
    fn from(titel: String) -> Self {
        Self {
            name: titel.clone(),
            titel,
            kategorie: "Berechnung".into(),
            version: "Aktueller Stand".into(),
        }
    }
}
pub struct Druckergebnis {
    pub pdf: Vec<u8>,
    pub positionen: Vec<Druckposition>,
    pub seiten: u32,
}
pub(super) fn fehler(e: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("Druckausgabe: {e}"))
}
pub(super) fn dokumenttyp(name: &str) -> bool {
    name.rsplit('.').next().is_some_and(|e| {
        ["pdf", "eml", "msg", "png", "jpg", "jpeg"].contains(&e.to_ascii_lowercase().as_str())
    })
}
impl CoreServices {
    pub async fn druckkatalog(&self, p: ProjectId) -> CoreResult<Druckkatalog> {
        let projekt = self.get_project(p).await?;
        let mut eintraege = Vec::new();
        let grund = crate::grundstueckskontext::ermitteln(&projekt);
        eintraege.push(Druckeintrag {
            titel: "Baugrundstücksermittlung".into(),
            quelle: Druckquelle::Baugrundstueck,
            verfuegbar: grund.as_ref().is_ok_and(|g| !g.is_empty()),
            hinweis: grund.err().map(|e| e.to_string()),
        });
        if projekt.planungsrecht.baugrenzen.is_some() {
            eintraege.push(Druckeintrag {
                titel: "Baugrenzenprüfung".into(),
                quelle: Druckquelle::Baugrenzen,
                verfuegbar: true,
                hinweis: None,
            });
        }
        for n in self.nachweise(p).await? {
            let status = berichte::nachweisstatus(&n);
            eintraege.push(Druckeintrag {
                titel: format!(
                    "{} · {}",
                    n.stand.bezeichnung,
                    n.stand.erstellt_am.format("%d.%m.%Y %H:%M UTC")
                ),
                quelle: Druckquelle::Nachweis { id: n.stand.id },
                verfuegbar: true,
                hinweis: Some(status),
            });
        }
        for r in self.fachobjekte(p, ObjektArt::Raumbuch).await? {
            let rb: crate::plattform::objekte::Raumbuch =
                serde_json::from_value(r.daten).map_err(fehler)?;
            eintraege.push(Druckeintrag {
                titel: format!("Raumbuch: {}", rb.bezeichnung),
                quelle: Druckquelle::Raumbuch {
                    id: Uuid::parse_str(&r.objekt).map_err(fehler)?,
                    revision: r.revision,
                },
                verfuegbar: true,
                hinweis: None,
            });
        }
        for d in crate::projektablage::versionen(self.list_documents(p).await?, false) {
            let verfuegbar = dokumenttyp(&d.file_name);
            eintraege.push(Druckeintrag {titel:format!("{} · Version {}",d.file_name,d.version),
                quelle:Druckquelle::Dokument{dokument_id:d.document_id,version:d.version,seiten:vec![]},verfuegbar,
                hinweis:(!verfuegbar).then(||"Im zugeordneten Programm drucken oder als PDF ins Projekt übernehmen. Originaldatei kann einzeln exportiert werden.".into())});
        }
        let b = self.buero().await?;
        let absender = format!(
            "{}\n{} {}\n{} {}",
            b.name,
            b.anschrift.strasse,
            b.anschrift.hausnummer,
            b.anschrift.postleitzahl,
            b.anschrift.ort
        )
        .trim()
        .to_owned();
        Ok(Druckkatalog {
            gestaltung: self.druckgestaltung().await?,
            projekt_version: projekt.version,
            absender,
            eintraege,
        })
    }
    pub async fn druckausgabe(
        &self,
        p: ProjectId,
        mut a: Druckauftrag,
    ) -> CoreResult<Druckergebnis> {
        if a.quellen.is_empty()
            || a.quellen.len() > 100
            || a.titel.trim().is_empty()
            || a.titel.len() > 500
            || a.absender.len() > 4000
            || a.empfaenger.len() > 4000
            || a.anschreiben.len() > 16000
            || a.aktenzeichen.len() > 500
        {
            return Err(fehler(
                "Titel und 1 bis 100 Unterlagen erforderlich; Textgrenzen beachten",
            ));
        }
        let gestaltung = match a.gestaltung.take() {
            Some(g) => g,
            None => self.druckgestaltung().await?,
        };
        gestaltung.validate()?;
        a.gestaltung = Some(gestaltung);
        let projekt = self.get_project(p).await?;
        if projekt.version != a.projekt_version {
            return Err(fehler(
                "Projekt wurde geändert. Auswahl aktualisieren und erneut erstellen.",
            ));
        }
        let jetzt = chrono::Utc::now();
        let nachweise = self.nachweise(p).await?;
        let kategorien = self.kategoriesnapshots(p).await?;
        let mut quellen = Vec::new();
        let mut bytes = 0usize;
        let mut druckdaten = 0usize;
        for q in &a.quellen {
            let (titel, mut doc) = match q {
                Druckquelle::Dokument {
                    dokument_id,
                    version,
                    seiten,
                } => {
                    let meta = self
                        .document_history(*dokument_id)
                        .await?
                        .into_iter()
                        .find(|d| d.version == *version && d.project_id == p)
                        .ok_or_else(|| fehler("Dokumentversion gehört nicht zu diesem Projekt"))?;
                    if !dokumenttyp(&meta.file_name) {
                        return Err(fehler(format!(
                            "{} zuerst als PDF bereitstellen",
                            meta.file_name
                        )));
                    }
                    if meta.size_bytes > 64 * 1024 * 1024 {
                        return Err(fehler("Quelldatei größer als 64 MB"));
                    }
                    let (_, daten) = self.load_version(*dokument_id, *version).await?;
                    bytes = bytes
                        .checked_add(daten.len())
                        .ok_or_else(|| fehler("Druckpaket zu groß"))?;
                    if bytes > 128 * 1024 * 1024 {
                        return Err(fehler("Druckpaket größer als 128 MB; bitte aufteilen"));
                    }
                    let name = meta.file_name.to_ascii_lowercase();
                    let doc = if name.ends_with(".pdf") {
                        pdf::auswahl(&daten, seiten)?
                    } else if [".png", ".jpg", ".jpeg"].iter().any(|e| name.ends_with(e)) {
                        if !seiten.is_empty() {
                            return Err(fehler("Seitenauswahl nur bei PDF-Dateien"));
                        }
                        pdf::bild(&daten)?
                    } else {
                        if !seiten.is_empty() {
                            return Err(fehler("Seitenauswahl nur bei PDF-Dateien"));
                        }
                        let mail = crate::email::lesen(&meta.file_name, &daten)?;
                        pdf::bericht(
                            &projekt.name,
                            &meta.file_name,
                            &berichte::email(&mail),
                            jetzt,
                        )?
                    };
                    let kategorie = kategorien
                        .get(&dokument_id.0.to_string())
                        .and_then(|k| k.as_ref())
                        .map(|k| {
                            k.hauptkategorie
                                .as_ref()
                                .map(|h| format!("{h} / {}", k.name))
                                .unwrap_or_else(|| k.name.clone())
                        })
                        .unwrap_or_else(|| "Ohne Kategorie".into());
                    (
                        Inhaltsangabe {
                            titel: format!("{} · Version {}", meta.file_name, version),
                            kategorie,
                            name: meta.file_name,
                            version: version.to_string(),
                        },
                        doc,
                    )
                }
                _ => {
                    let (titel, zeilen) =
                        berichte::erstellen(self, &projekt, q, &nachweise).await?;
                    let mut doc = pdf::bericht(&projekt.name, &titel, &zeilen, jetzt)?;
                    if let Druckquelle::Nachweis { id } = q
                        && let Some(n) = nachweise.iter().find(|n| n.stand.id == *id)
                        && let crate::plattform::nachweise::NachweisEingabe::AbstandProjekt {
                            plan,
                            ..
                        } = &n.stand.eingabe
                    {
                        let r = serde_json::from_value(n.stand.ergebnis.clone()).map_err(fehler)?;
                        crate::abstandsflaechen::bericht::planseite(&mut doc, plan, &r)?;
                    }
                    let version = match q {
                        Druckquelle::Nachweis { id } => nachweise
                            .iter()
                            .find(|n| n.stand.id == *id)
                            .map(|n| {
                                format!(
                                    "Nachweis vom {}",
                                    n.stand.erstellt_am.format("%d.%m.%Y %H:%M UTC")
                                )
                            })
                            .unwrap_or_else(|| "Nachweisstand".into()),
                        Druckquelle::Raumbuch { revision, .. } => format!("Revision {revision}"),
                        _ => format!("Projektstand {}", projekt.version),
                    };
                    let kategorie = match q {
                        Druckquelle::Baugrundstueck => "Berechnung / Baugrundstück",
                        Druckquelle::Baugrenzen => "Berechnung / Baugrenzen",
                        Druckquelle::Raumbuch { .. } => "Berechnung / Raumbuch",
                        Druckquelle::Nachweis { id } => match nachweise
                            .iter()
                            .find(|n| n.stand.id == *id)
                            .map(|n| &n.stand.eingabe)
                        {
                            Some(crate::plattform::nachweise::NachweisEingabe::Fach {
                                plan,
                                ..
                            }) => plan.modul.titel(),
                            Some(crate::plattform::nachweise::NachweisEingabe::Nutzung {
                                fach,
                                ..
                            }) => match fach {
                                crate::plattform::nachweise::NutzungsArt::Grz => "Berechnung / GRZ",
                                crate::plattform::nachweise::NutzungsArt::Gfz => "Berechnung / GFZ",
                                crate::plattform::nachweise::NutzungsArt::Bmz => "Berechnung / BMZ",
                                crate::plattform::nachweise::NutzungsArt::Geschossigkeit => {
                                    "Berechnung / Geschossigkeit"
                                }
                            },
                            _ => "Berechnung / Abstandsflächen",
                        },
                        Druckquelle::Dokument { .. } => unreachable!(),
                    };
                    (
                        Inhaltsangabe {
                            name: titel.clone(),
                            titel,
                            kategorie: kategorie.into(),
                            version,
                        },
                        doc,
                    )
                }
            };
            if doc.get_pages().is_empty() {
                return Err(fehler("PDF enthält keine Seiten"));
            }
            pdf::formulare_einfrieren(&mut doc)?;
            druckdaten = druckdaten.saturating_add(
                doc.objects
                    .values()
                    .filter_map(|o| o.as_stream().ok())
                    .map(|s| s.content.len())
                    .sum::<usize>(),
            );
            if druckdaten > 128 * 1024 * 1024 {
                return Err(fehler(
                    "Aufbereitete Druckdaten über 128 MB; bitte aufteilen",
                ));
            }
            quellen.push((titel, doc));
        }
        // Erneuter Vergleich verhindert vermischte aktuelle Projektgrundlagen.
        if self.get_project(p).await?.version != a.projekt_version {
            return Err(fehler(
                "Projekt während der Ausgabe geändert. Erneut erstellen.",
            ));
        }
        pdf::paket(&projekt.name, a, quellen, jetzt)
    }
}

impl CoreServices {
    /// Übernimmt exakt die geprüfte Druckkopie als versionierte Projektunterlage.
    /// Die Vorgangskennung verhindert eine zweite Datei nach einem Antwortverlust.
    pub async fn druckpdf_ablegen(
        &self,
        p: ProjectId,
        name: &str,
        pdf: bytes::Bytes,
        vorgang: uuid::Uuid,
        wer: &crate::benutzer::Benutzer,
    ) -> CoreResult<crate::domain::DocumentVersion> {
        self.projekt_zugriff(p, wer, crate::zugriff::ProjektRecht::Bearbeiten)
            .await?;
        if pdf.len() > 128 * 1024 * 1024 || !pdf.starts_with(b"%PDF-") || name.len() > 400 {
            return Err(fehler("Ungültige PDF-Ausgabe"));
        }
        let d = lopdf::Document::load_mem(&pdf).map_err(fehler)?;
        if d.get_pages().is_empty() {
            return Err(fehler("PDF-Ausgabe enthält keine Seiten"));
        }
        let k = self
            .kategorienkatalog()
            .await?
            .kategorien
            .iter()
            .any(|k| k.id == "berechnung" && k.aktiv)
            .then_some("berechnung");
        self.ablegen_vorgang(
            p,
            None,
            None,
            &format!("{}.pdf", name.trim_end_matches(".pdf")),
            pdf,
            wer,
            crate::domain::Aktion::DokumentAbgelegt,
            "Druckkopie für Verfahrensunterlagen",
            Some(vorgang),
            vec![crate::commit::DokumentZusatz {
                modul: "dokumentkategorie".into(),
                schema_version: 1,
                daten: serde_json::to_value(k).map_err(fehler)?,
            }],
        )
        .await
    }
}
