//! Revisionssicheres projektgebundenes Nachrichtenjournal, ohne Transportbehauptung.
use super::austausch::{self, Pruefbericht, fehler};
use crate::{
    CoreError, CoreResult, CoreServices,
    benutzer::{Benutzer, Recht},
    commit::Fachstand,
    domain::{Aktion, ProjectId},
    plattform::objekte::Dokumentbezug,
    zugriff::ProjektRecht,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
const MODUL: &str = "xbau_nachrichten";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Richtung {
    Eingang,
    Ausgang,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Belegart {
    Versand,
    Transportquittung,
    FachlicheAnnahme,
    Rueckweisung,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Nachrichtenbeleg {
    pub id: Uuid,
    pub art: Belegart,
    pub datum: DateTime<Utc>,
    pub kanal: String,
    pub referenz: String,
    pub dokument: Dokumentbezug,
    pub notiz: String,
    pub berichtigt: Option<Uuid>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct XbauNachricht {
    pub original: Dokumentbezug,
    pub richtung: Richtung,
    pub pruefung: Pruefbericht,
    pub erfasst_am: DateTime<Utc>,
    pub erfasst_von: Uuid,
    pub belege: Vec<Nachrichtenbeleg>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Nachrichtenstand {
    pub id: Uuid,
    #[ts(type = "number")]
    pub revision: i64,
    pub nachricht: XbauNachricht,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Nachrichtenaufnahme {
    pub original: Dokumentbezug,
    pub richtung: Richtung,
    pub zuordnung_bestaetigt: bool,
}
impl CoreServices {
    pub async fn xbau_ausgang_vorbereiten(
        &self,
        p: ProjectId,
        wer: &Benutzer,
    ) -> CoreResult<Nachrichtenstand> {
        self.projekt_zugriff(p, wer, ProjektRecht::Bearbeiten)
            .await?;
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        self.xbau_ausgang_speichern(p, None, wer).await
    }
    pub async fn xbau_nachreichung_vorbereiten(
        &self,
        p: ProjectId,
        quelle: Uuid,
        wer: &Benutzer,
    ) -> CoreResult<Nachrichtenstand> {
        self.projekt_zugriff(p, wer, ProjektRecht::Bearbeiten)
            .await?;
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        let n = self
            .xbau_nachrichten(p, wer)
            .await?
            .into_iter()
            .find(|n| n.id == quelle)
            .ok_or_else(|| fehler("Bezugsnachricht nicht gefunden"))?;
        let k = &n.nachricht.pruefung.kopf;
        if (k.element == "baugenehmigung.antrag.0200" && n.nachricht.richtung != Richtung::Ausgang)
            || (k.element == "prozessnachrichten.formellePruefungBefundliste.1140"
                && n.nachricht.richtung != Richtung::Eingang)
        {
            return Err(fehler(
                "Nachreichung benötigt einen Ausgangsantrag oder eine eingehende behördliche Befundliste",
            ));
        }
        if ![
            "baugenehmigung.antrag.0200",
            "prozessnachrichten.formellePruefungBefundliste.1140",
        ]
        .contains(&k.element.as_str())
            || k.antragsreferenz != Some(self.bauantrag(p).await?.antrag.vorgang)
        {
            return Err(fehler(
                "Nachreichung benötigt den ursprünglichen Bauantrag oder eine Befundliste mit passender Antragsreferenz",
            ));
        }
        let antrag = self.bauantrag(p).await?.antrag;
        let behoerde = if n.nachricht.richtung == Richtung::Eingang {
            &k.absender
        } else {
            &k.empfaenger
        };
        if antrag.bauaufsicht.kennung.trim() != behoerde {
            return Err(fehler(
                "Behörde der Bezugsnachricht passt nicht zum gespeicherten Bauantrag",
            ));
        }
        self.xbau_ausgang_speichern(p, Some(k), wer).await
    }
    async fn xbau_ausgang_speichern(
        &self,
        p: ProjectId,
        bezug: Option<&super::austausch::Nachrichtenkopf>,
        wer: &Benutzer,
    ) -> CoreResult<Nachrichtenstand> {
        let export = self.xbau_paket(p, wer, bezug).await?;
        let bericht =
            austausch::pruefen("XBau.zip", &export.zip, self.xbau_pruefer.as_ref()).await?;
        let d = self
            .ablegen_vorgang(
                p,
                None,
                None,
                &export.dateiname,
                bytes::Bytes::from(export.zip),
                wer,
                Aktion::DokumentAbgelegt,
                " · XBau-Ausgang vorbereitet",
                Some(bericht.kopf.id),
                vec![],
            )
            .await?;
        self.xbau_aufnehmen(
            p,
            Nachrichtenaufnahme {
                original: Dokumentbezug {
                    dokument_id: d.document_id,
                    version: d.version,
                },
                richtung: Richtung::Ausgang,
                zuordnung_bestaetigt: true,
            },
            wer,
        )
        .await
    }
    pub async fn xbau_anlage_ablegen(
        &self,
        p: ProjectId,
        id: Uuid,
        dateiname: &str,
        wer: &Benutzer,
    ) -> CoreResult<crate::domain::DocumentVersion> {
        self.projekt_zugriff(p, wer, ProjektRecht::Bearbeiten)
            .await?;
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        let n = self
            .xbau_nachrichten(p, wer)
            .await?
            .into_iter()
            .find(|n| n.id == id)
            .ok_or_else(|| fehler("Nachricht nicht gefunden"))?
            .nachricht;
        let a = n
            .pruefung
            .anlagen
            .iter()
            .find(|a| a.dateiname == dateiname)
            .ok_or_else(|| fehler("Anlage nicht im Prüfbericht"))?;
        let (_, bytes) = self.xbau_original(p, &n.original).await?;
        if crate::storage::ContentHash::of(&bytes).as_str() != n.pruefung.sha256 {
            return Err(fehler("Original passt nicht zum Prüfbericht"));
        }
        let inhalt = austausch::anlage_lesen(&bytes, dateiname, &a.sha256)?;
        let vorgang = Uuid::new_v5(&id, format!("anlage:{dateiname}:{}", a.sha256).as_bytes());
        self.ablegen_vorgang(
            p,
            None,
            None,
            dateiname,
            bytes::Bytes::from(inhalt),
            wer,
            Aktion::DokumentAbgelegt,
            " · Anlage aus XBau-Original",
            Some(vorgang),
            vec![crate::commit::DokumentZusatz {
                modul: "xbau_anlagenquelle".into(),
                schema_version: 1,
                daten: serde_json::json!({"nachricht":id,"original":n.original,"anlage":a}),
            }],
        )
        .await
    }
    async fn xbau_original(
        &self,
        p: ProjectId,
        d: &Dokumentbezug,
    ) -> CoreResult<(String, bytes::Bytes)> {
        let meta = self
            .repos
            .documents
            .history(d.dokument_id)
            .await?
            .into_iter()
            .find(|v| v.project_id == p && v.version == d.version)
            .ok_or_else(|| fehler("Originalversion gehört nicht zu diesem Projekt"))?;
        if meta.size_bytes > austausch::MAX_PAKET {
            return Err(fehler("Original größer als 64 MiB"));
        }
        let bytes = self
            .storage
            .load(&crate::storage::StorageKey::new(meta.storage_key)?)
            .await?;
        meta.sha256.verify(&bytes)?;
        Ok((meta.file_name, bytes))
    }
    pub async fn xbau_pruefen(
        &self,
        p: ProjectId,
        d: &Dokumentbezug,
        wer: &Benutzer,
    ) -> CoreResult<Pruefbericht> {
        self.projekt_zugriff(p, wer, ProjektRecht::Lesen).await?;
        let (name, bytes) = self.xbau_original(p, d).await?;
        austausch::pruefen(&name, &bytes, self.xbau_pruefer.as_ref()).await
    }
    pub async fn xbau_nachrichten(
        &self,
        p: ProjectId,
        wer: &Benutzer,
    ) -> CoreResult<Vec<Nachrichtenstand>> {
        self.projekt_zugriff(p, wer, ProjektRecht::Lesen).await?;
        self.repos.projects.get(p).await?;
        let mut result = vec![];
        for f in self.repos.commit.fachstaende(p, MODUL).await? {
            if f.schema_version != 1 {
                return Err(fehler(
                    "Unbekannter Nachrichtenstand; Anwendung aktualisieren",
                ));
            }
            let n: XbauNachricht =
                serde_json::from_value(f.daten).map_err(|e| fehler(e.to_string()))?;
            let id = f
                .objekt
                .parse::<Uuid>()
                .map_err(|_| fehler("Nachrichtenidentität ungültig"))?;
            if n.pruefung.kopf.id != id {
                return Err(fehler("Nachrichtenidentität passt nicht zum Datensatz"));
            }
            result.push(Nachrichtenstand {
                id,
                revision: f.revision,
                nachricht: n,
            });
        }
        result.sort_by_key(|n| std::cmp::Reverse(n.nachricht.erfasst_am));
        Ok(result)
    }
    pub async fn xbau_aufnehmen(
        &self,
        p: ProjectId,
        w: Nachrichtenaufnahme,
        wer: &Benutzer,
    ) -> CoreResult<Nachrichtenstand> {
        self.projekt_zugriff(p, wer, ProjektRecht::Bearbeiten)
            .await?;
        self.pruefe(wer, Recht::ProjektBearbeiten).await?;
        if !w.zuordnung_bestaetigt {
            return Err(fehler(
                "Projektzuordnung und Richtung anhand des Originals bestätigen",
            ));
        }
        let pruefung = self.xbau_pruefen(p, &w.original, wer).await?;
        let id = pruefung.kopf.id;
        if let Some(alt) = self
            .xbau_nachrichten(p, wer)
            .await?
            .into_iter()
            .find(|n| n.id == id)
        {
            if alt.nachricht.pruefung.sha256 != pruefung.sha256
                || alt.nachricht.richtung != w.richtung
            {
                return Err(fehler(
                    "Nachrichten-ID ist bereits mit anderem Inhalt oder anderer Richtung archiviert",
                ));
            }
            return Ok(alt);
        }
        let n = XbauNachricht {
            original: w.original,
            richtung: w.richtung,
            pruefung,
            erfasst_am: Utc::now(),
            erfasst_von: wer.id.0,
            belege: vec![],
        };
        match self.xbau_commit(p, id, 0, n.clone(), wer).await {
            Ok(n) => Ok(n),
            Err(CoreError::VersionConflict { .. }) => {
                let alt = self
                    .xbau_nachrichten(p, wer)
                    .await?
                    .into_iter()
                    .find(|a| a.id == id)
                    .ok_or_else(|| fehler("Gleichzeitige Aufnahme; bitte neu laden"))?;
                if alt.nachricht.pruefung.sha256 == n.pruefung.sha256
                    && alt.nachricht.richtung == n.richtung
                {
                    Ok(alt)
                } else {
                    Err(fehler("Gleichzeitige Aufnahme mit abweichendem Inhalt"))
                }
            }
            Err(e) => Err(e),
        }
    }
    async fn xbau_commit(
        &self,
        p: ProjectId,
        id: Uuid,
        basis: i64,
        n: XbauNachricht,
        wer: &Benutzer,
    ) -> CoreResult<Nachrichtenstand> {
        let f = self
            .repos
            .commit
            .fachstand(
                Fachstand {
                    projekt_id: p,
                    modul: MODUL.into(),
                    objekt: id.to_string(),
                    schema_version: 1,
                    revision: basis,
                    daten: serde_json::to_value(&n).map_err(|e| fehler(e.to_string()))?,
                },
                basis,
                Self::audit(
                    wer,
                    Aktion::ProjektGeaendert,
                    Some(p),
                    Some(n.original.dokument_id),
                    format!(
                        "XBau {}: {} · {} Belege",
                        id,
                        n.pruefung.kopf.titel,
                        n.belege.len()
                    ),
                ),
            )
            .await?;
        Ok(Nachrichtenstand {
            id,
            revision: f.revision,
            nachricht: n,
        })
    }
    pub async fn xbau_beleg(
        &self,
        p: ProjectId,
        id: Uuid,
        basis: i64,
        b: Nachrichtenbeleg,
        wer: &Benutzer,
    ) -> CoreResult<Nachrichtenstand> {
        self.projekt_zugriff(p, wer, ProjektRecht::Bearbeiten)
            .await?;
        self.pruefe(wer, Recht::ProjektBearbeiten).await?;
        let mut alt = self
            .xbau_nachrichten(p, wer)
            .await?
            .into_iter()
            .find(|n| n.id == id)
            .ok_or_else(|| fehler("Nachricht nicht gefunden"))?;
        if let Some(vorher) = alt.nachricht.belege.iter().find(|a| a.id == b.id) {
            if vorher == &b {
                return Ok(alt);
            }
            return Err(fehler("Beleg-ID bereits verwendet"));
        }
        if basis != alt.revision {
            return Err(CoreError::VersionConflict {
                expected: basis,
                actual: alt.revision,
            });
        }
        for s in [&b.kanal, &b.referenz] {
            crate::plattform::text(s, 500)?;
        }
        if b.notiz.len() > 10000 || alt.nachricht.belege.len() >= 500 {
            return Err(fehler("Zu viele oder zu lange Belege"));
        }
        if b.datum > Utc::now() {
            return Err(fehler(
                "Übermittlungsbeleg darf nicht in der Zukunft liegen",
            ));
        }
        if let Some(ber) = b.berichtigt
            && (!alt.nachricht.belege.iter().any(|a| a.id == ber)
                || alt
                    .nachricht
                    .belege
                    .iter()
                    .any(|a| a.berichtigt == Some(ber)))
        {
            return Err(fehler(
                "Nur einen vorhandenen, noch nicht berichtigten Beleg berichtigen",
            ));
        }
        self.xbau_original(p, &b.dokument).await?;
        if b.dokument == alt.nachricht.original {
            return Err(fehler(
                "Die XBau-Nachricht selbst ist kein Zustellungs- oder Annahmebeleg",
            ));
        }
        alt.nachricht.belege.push(b);
        self.xbau_commit(p, id, basis, alt.nachricht, wer).await
    }
    /// Bewusste, wiederholbare Übernahme als Kommunikation. Keine automatische
    /// Frist-/Bescheidentscheidung und kein aus XML abgeleiteter Versandstatus.
    pub async fn xbau_in_verfahren(
        &self,
        p: ProjectId,
        id: Uuid,
        verfahren: Uuid,
        basis: i64,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        self.projekt_zugriff(p, wer, ProjektRecht::Bearbeiten)
            .await?;
        self.pruefe(wer, Recht::ProjektBearbeiten).await?;
        let n = self
            .xbau_nachrichten(p, wer)
            .await?
            .into_iter()
            .find(|n| n.id == id)
            .ok_or_else(|| fehler("Nachricht nicht gefunden"))?
            .nachricht;
        let (stand, mut akte) = self.verfahrensakte(p, verfahren).await?;
        let c = Uuid::new_v5(&id, b"bauprojektio-xbau-kommunikation");
        if akte.kommunikation.iter().any(|a| a.id == c) {
            return Ok(stand);
        }
        if stand.revision != basis {
            return Err(CoreError::VersionConflict {
                expected: basis,
                actual: stand.revision,
            });
        }
        let k = &n.pruefung.kopf;
        let behoerde = if n.richtung == Richtung::Eingang {
            &k.absender
        } else {
            &k.empfaenger
        };
        if !akte.behoerde.kennung.trim().is_empty() && akte.behoerde.kennung.trim() != behoerde {
            return Err(fehler(
                "Behördenkennung passt nicht zur ausgewählten Verfahrensakte",
            ));
        }
        if let Some(az) = &k.aktenzeichen
            && !akte.aktenzeichen.trim().is_empty()
            && akte.aktenzeichen.trim() != az
        {
            return Err(fehler("Aktenzeichen passt nicht zur Verfahrensakte"));
        }
        akte.kommunikation.push(crate::verfahren::Kommunikation{id:c,art:crate::verfahren::Kommunikationsart::Portal,eingehend:n.richtung==Richtung::Eingang,datum:k.erstellt.date_naive(),betreff:format!("XBau: {}",k.titel),beteiligte:format!("{} → {}",k.absender,k.empfaenger),inhalt:format!("{}\nNachrichten-ID: {}\nAntragsreferenz: {}\nAktenzeichen: {}\n{} Anlagen. XML erstellt am {}. Als Datei erfasst am {}.\nXSD-geprüft. Übermittlung, Authentizität, Fristen und Rechtswirkung gesondert anhand der Originale und Belege prüfen.",k.element,k.id,k.antragsreferenz.map(|s|s.to_string()).unwrap_or_default(),k.aktenzeichen.as_deref().unwrap_or(""),n.pruefung.anlagen.len(),k.erstellt,n.erfasst_am),original:Some(n.original),bezuege:vec![]});
        self.verfahren_commit(p, verfahren, basis, akte, wer, false)
            .await
    }
}
