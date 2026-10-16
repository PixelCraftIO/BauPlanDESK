use super::*;
use crate::{
    CoreServices,
    benutzer::{Benutzer, Recht},
    commit::Fachstand,
    domain::ProjectId,
    storage::{ContentHash, StorageKey},
    verfahren::{AktenCommit, Bezug, Kommunikation, Kommunikationsart, Verfahrensakte},
    zugriff::ProjektRecht,
};
use bytes::Bytes;
impl CoreServices {
    async fn portalakte_laden(
        &self,
        p: ProjectId,
        id: Uuid,
        wer: &Benutzer,
        schreiben: bool,
    ) -> CoreResult<(Fachstand, Verfahrensakte)> {
        self.projekt_zugriff(
            p,
            wer,
            if schreiben {
                ProjektRecht::Bearbeiten
            } else {
                ProjektRecht::Lesen
            },
        )
        .await?;
        if schreiben {
            self.pruefe(wer, Recht::ProjektBearbeiten).await?;
        }
        self.verfahrensakte(p, id).await
    }
    pub async fn portal_vorgang_anlegen(
        &self,
        p: ProjectId,
        verfahren: Uuid,
        basis: i64,
        w: Portalvorgangswunsch,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        let (f, mut a) = self.portalakte_laden(p, verfahren, wer, true).await?;
        let profil = self
            .portalprofile()
            .await?
            .into_iter()
            .find(|p| p.id == w.profil_id && p.version == w.profil_version)
            .ok_or_else(|| fehler("Profilversion nicht vorhanden"))?;
        if let Some(v) = a
            .portal
            .iter()
            .flat_map(|p| &p.vorgaenge)
            .find(|v| v.id == w.id)
        {
            if v.profil == profil && v.ziel == w.ziel {
                return Ok(f);
            }
            return Err(fehler("Vorgangs-ID mit anderen Angaben belegt"));
        }
        basis_pruefen(f.revision, basis)?;
        let land = self.get_project(p).await?.bundesland;
        if profil.bundesland.is_some_and(|b| b != land)
            || (!profil.verfahren.is_empty() && !profil.verfahren.contains(&a.art))
        {
            return Err(fehler(
                "Profil passt nicht zu Bundesland oder Verfahrensart",
            ));
        }
        let mut ziel = w.ziel;
        ziel.url = browser_url(&ziel.url)?;
        if let Some(url) = &profil.portal_url
            && url::Url::parse(url)
                .ok()
                .and_then(|u| u.host_str().map(String::from))
                != url::Url::parse(&ziel.url)
                    .ok()
                    .and_then(|u| u.host_str().map(String::from))
        {
            return Err(fehler("Portaladresse passt nicht zum ausgewählten Profil"));
        }
        if !ziel.zuordnung_bestaetigt
            || ziel.behoerdenkennung.trim() != a.behoerde.kennung.trim()
            || ziel.stelle.trim() != a.behoerde.name.trim()
        {
            return Err(fehler(
                "Portalziel muss zur ausgewählten Verfahrensbehörde passen und bestätigt werden",
            ));
        }
        a.portal
            .get_or_insert_with(Default::default)
            .vorgaenge
            .push(Portalvorgang {
                id: w.id,
                profil,
                ziel,
                erstellt_am: Utc::now(),
                erstellt_von: wer.id.0,
                abgaben: vec![],
                rueckmeldungen: vec![],
            });
        self.verfahren_speichern_intern(p, verfahren, basis, a, wer, AktenCommit::Portal)
            .await
    }
    pub async fn portal_abgabe_pruefen(
        &self,
        p: ProjectId,
        verfahren: Uuid,
        vorgang: Uuid,
        w: &Portalabgabewunsch,
        wer: &Benutzer,
    ) -> CoreResult<Portalvorbereitung> {
        let (_, a) = self.portalakte_laden(p, verfahren, wer, false).await?;
        pruefung::vorbereiten(vorgang_ref(&a, vorgang)?, &a, w)
    }
    pub async fn portal_abgabe_festhalten(
        &self,
        p: ProjectId,
        verfahren: Uuid,
        vorgang: Uuid,
        basis: i64,
        w: Portalabgabewunsch,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        let (f, mut a) = self.portalakte_laden(p, verfahren, wer, true).await?;
        let hash = ContentHash::of(&serde_json::to_vec(&w).map_err(|e| fehler(e.to_string()))?)
            .to_string();
        if let Some(b) = vorgang_ref(&a, vorgang)?
            .abgaben
            .iter()
            .find(|b| b.id == w.abgabe)
        {
            if b.auftrag_hash == hash {
                return Ok(f);
            }
            return Err(fehler(
                "Eingefrorene Portalabgabe bereits mit anderer Zuordnung vorhanden",
            ));
        }
        basis_pruefen(f.revision, basis)?;
        let v = vorgang_ref(&a, vorgang)?.clone();
        let vorbereitung = pruefung::vorbereiten(&v, &a, &w)?;
        if !vorbereitung.pruefung.fehler.is_empty() {
            return Err(fehler(vorbereitung.pruefung.fehler.join("; ")));
        }
        let key = StorageKey::new(format!(
            "projekte/{p}/portalbereitstellungen/{}/{}.zip",
            w.abgabe,
            Uuid::new_v4()
        ))?;
        let mut b = Portalabgabe {
            id: w.abgabe,
            abgabe: w.abgabe,
            auftrag_hash: hash,
            art: w.art,
            vorgaenger: w.vorgaenger,
            forderungen: w.forderungen,
            dateien: vorbereitung.dateien,
            pruefung: vorbereitung.pruefung,
            erstellt_am: Utc::now(),
            bereitstellung: Artefakt {
                format: "portalbereitstellung".into(),
                dateiname: "Portalunterlagen.zip".into(),
                storage_key: key.to_string(),
                sha256: String::new(),
                bytes: 0,
            },
            ereignisse: vec![],
        };
        let bytes = paket::erzeugen(self, p, &v, &b).await?;
        let gespeichert = self.storage.save(&key, Bytes::from(bytes)).await?;
        b.bereitstellung.sha256 = gespeichert.sha256.to_string();
        b.bereitstellung.bytes = gespeichert.size_bytes;
        vorgang_mut(&mut a, vorgang)?.abgaben.push(b);
        match self
            .verfahren_speichern_intern(p, verfahren, basis, a, wer, AktenCommit::Portal)
            .await
        {
            Ok(f) => Ok(f),
            Err(e) => {
                if let Err(delete) = self.storage.delete(&key).await {
                    tracing::warn!(error=%delete,"Unbestätigte Portalbereitstellung konnte nicht entfernt werden");
                }
                Err(e)
            }
        }
    }
    pub async fn portal_bereitstellung(
        &self,
        p: ProjectId,
        verfahren: Uuid,
        vorgang: Uuid,
        abgabe: Uuid,
        wer: &Benutzer,
    ) -> CoreResult<(String, Vec<u8>)> {
        let (_, a) = self.portalakte_laden(p, verfahren, wer, false).await?;
        let b = abgabe_ref(vorgang_ref(&a, vorgang)?, abgabe)?;
        Ok((
            b.bereitstellung.dateiname.clone(),
            paket::laden(self, &b.bereitstellung).await?,
        ))
    }
    pub async fn portal_datei(
        &self,
        p: ProjectId,
        verfahren: Uuid,
        vorgang: Uuid,
        abgabe: Uuid,
        dokument: &Dokumentbezug,
        wer: &Benutzer,
    ) -> CoreResult<(String, Vec<u8>)> {
        let (_, a) = self.portalakte_laden(p, verfahren, wer, false).await?;
        let b = abgabe_ref(vorgang_ref(&a, vorgang)?, abgabe)?;
        let d = b
            .dateien
            .iter()
            .find(|d| &d.original.dokument == dokument)
            .ok_or_else(|| fehler("Datei nicht in der Portalabgabe"))?;
        let (m, bytes) = self
            .load_version(dokument.dokument_id, dokument.version)
            .await?;
        if m.project_id != p || m.sha256.as_str() != d.original.sha256 {
            return Err(fehler("Portaldatei passt nicht zum Originalstand"));
        }
        Ok((d.uploadname.clone(), bytes.to_vec()))
    }
    pub async fn portal_url(
        &self,
        p: ProjectId,
        verfahren: Uuid,
        vorgang: Uuid,
        wer: &Benutzer,
    ) -> CoreResult<String> {
        let (_, a) = self.portalakte_laden(p, verfahren, wer, false).await?;
        browser_url(&vorgang_ref(&a, vorgang)?.ziel.url)
    }
    #[allow(clippy::too_many_arguments)]
    pub async fn portal_ereignis_erfassen(
        &self,
        p: ProjectId,
        verfahren: Uuid,
        vorgang: Uuid,
        abgabe: Uuid,
        basis: i64,
        w: Portalereigniswunsch,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        let (f, mut a) = self.portalakte_laden(p, verfahren, wer, true).await?;
        let b = abgabe_ref(vorgang_ref(&a, vorgang)?, abgabe)?;
        if let Some(e) = b.ereignisse.iter().find(|e| e.id == w.id) {
            let neu = ereignis(&w, e.erfasst_am, e.erfasst_von);
            if e == &neu {
                return Ok(f);
            }
            return Err(fehler("Ereignis-ID bereits mit anderen Angaben verwendet"));
        }
        basis_pruefen(f.revision, basis)?;
        crate::plattform::text(&w.notiz, 3500)?;
        if w.referenz.len() > 500 || w.id.is_nil() || w.datum > crate::kalender::heute() {
            return Err(fehler("Ungültiges Datum, Referenz oder Ereignis-ID"));
        }
        if w.art != Portalereignisart::AusgangUnklar
            && (w.beleg.is_none() || w.referenz.trim().is_empty())
        {
            return Err(fehler(
                "Belegte Portalereignisse benötigen Originalbeleg und Portalreferenz",
            ));
        }
        if let Some(d) = &w.beleg {
            self.portal_dokument_pruefen(p, d).await?;
            if b.dateien.iter().any(|f| f.original.dokument == *d) {
                return Err(fehler("Eine Paketdatei selbst ist kein Einreichungsbeleg"));
            }
        }
        if let Some(id) = w.berichtigt
            && (!b.ereignisse.iter().any(|e| e.id == id)
                || b.ereignisse.iter().any(|e| e.berichtigt == Some(id)))
        {
            return Err(fehler(
                "Nur ein vorhandenes, noch nicht berichtigtes Ereignis berichtigen",
            ));
        }
        let mut refs = std::collections::HashSet::new();
        if w.bestaetigte_dateien.iter().any(|d| {
            !refs.insert((d.dokument_id, d.version))
                || !b.dateien.iter().any(|f| f.original.dokument == *d)
        }) {
            return Err(fehler(
                "Dateiabgleich enthält doppelte oder nicht zur Abgabe gehörende Versionen",
            ));
        }
        match w.dateiabgleich {
            Dateiabgleich::Ungeklaert if !w.bestaetigte_dateien.is_empty() => {
                return Err(fehler(
                    "Ungeklärter Dateiabgleich darf keine bestätigten Dateien behaupten",
                ));
            }
            Dateiabgleich::Vollstaendig if w.bestaetigte_dateien.len() != b.dateien.len() => {
                return Err(fehler(
                    "Vollständiger Dateiabgleich benötigt jede konkrete Paketversion",
                ));
            }
            _ => {}
        }
        if w.art == Portalereignisart::AusgangUnklar && w.dateiabgleich != Dateiabgleich::Ungeklaert
        {
            return Err(fehler(
                "Unklarer Versandausgang kann keine Dateiliste bestätigen",
            ));
        }
        let e = ereignis(&w, Utc::now(), wer.id.0);
        let z = vorgang_ref(&a, vorgang)?.ziel.clone();
        let projection = pruefung::projektion(&e, &z);
        let original = a
            .abgaben
            .iter_mut()
            .find(|b| b.id == abgabe)
            .ok_or_else(|| fehler("Abgabestand fehlt"))?;
        original.versand.push(projection);
        vorgang_mut(&mut a, vorgang)?
            .abgaben
            .iter_mut()
            .find(|b| b.id == abgabe)
            .ok_or_else(|| fehler("Portalabgabe fehlt"))?
            .ereignisse
            .push(e);
        self.verfahren_speichern_intern(p, verfahren, basis, a, wer, AktenCommit::Portal)
            .await
    }
    pub async fn portal_rueckmeldung_erfassen(
        &self,
        p: ProjectId,
        verfahren: Uuid,
        vorgang: Uuid,
        basis: i64,
        w: Portalrueckmeldungswunsch,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        let (f, mut a) = self.portalakte_laden(p, verfahren, wer, true).await?;
        let v = vorgang_ref(&a, vorgang)?;
        if let Some(r) = v.rueckmeldungen.iter().find(|r| r.id == w.id) {
            if r.betreff == w.betreff
                && r.datum == w.datum
                && r.referenz == w.referenz
                && r.original == w.original
                && r.inhalt == w.inhalt
                && r.abgabe == w.abgabe
                && r.xbau_nachricht == w.xbau_nachricht
            {
                return Ok(f);
            }
            return Err(fehler("Rückmeldungs-ID ist bereits belegt"));
        }
        basis_pruefen(f.revision, basis)?;
        crate::plattform::text(&w.betreff, 500)?;
        if w.id.is_nil()
            || w.datum > crate::kalender::heute()
            || w.inhalt.len() > 60000
            || w.referenz.len() > 500
        {
            return Err(fehler(
                "Rückmeldung enthält ungültige oder zu lange Angaben",
            ));
        }
        self.portal_dokument_pruefen(p, &w.original).await?;
        if let Some(id) = w.abgabe {
            abgabe_ref(v, id)?;
        }
        if let Some(id) = w.xbau_nachricht {
            let n = self
                .xbau_nachrichten(p, wer)
                .await?
                .into_iter()
                .find(|n| n.id == id)
                .ok_or_else(|| fehler("XBau-Nachricht nicht in diesem Projekt"))?;
            if n.nachricht.original != w.original
                || n.nachricht.richtung != crate::xbau::postfach::Richtung::Eingang
                || (!v.ziel.behoerdenkennung.is_empty()
                    && n.nachricht.pruefung.kopf.absender != v.ziel.behoerdenkennung)
                || n.nachricht
                    .pruefung
                    .kopf
                    .aktenzeichen
                    .as_ref()
                    .is_some_and(|az| !a.aktenzeichen.is_empty() && *az != a.aktenzeichen)
            {
                return Err(fehler(
                    "XBau-Original oder Behörden-/Verfahrensbezug passt nicht",
                ));
            }
        }
        let cid = pruefung::kommunikation_id(w.id);
        a.kommunikation.push(Kommunikation {
            id: cid,
            art: Kommunikationsart::Portal,
            eingehend: true,
            datum: w.datum,
            betreff: w.betreff.clone(),
            beteiligte: v.ziel.stelle.clone(),
            inhalt: format!(
                "Manuell zugeordnete Portalrückmeldung · {}\n{}",
                w.referenz, w.inhalt
            ),
            original: Some(w.original.clone()),
            bezuege: w
                .abgabe
                .map(|id| Bezug::Abgabe { id })
                .into_iter()
                .collect(),
        });
        vorgang_mut(&mut a, vorgang)?
            .rueckmeldungen
            .push(Portalrueckmeldung {
                id: w.id,
                betreff: w.betreff,
                datum: w.datum,
                referenz: w.referenz,
                original: w.original,
                inhalt: w.inhalt,
                abgabe: w.abgabe,
                xbau_nachricht: w.xbau_nachricht,
                erfasst_am: Utc::now(),
                erfasst_von: wer.id.0,
            });
        self.verfahren_speichern_intern(p, verfahren, basis, a, wer, AktenCommit::Portal)
            .await
    }
    pub async fn portal_xbau_archivieren(
        &self,
        p: ProjectId,
        verfahren: Uuid,
        vorgang: Uuid,
        abgabe: Uuid,
        wer: &Benutzer,
    ) -> CoreResult<crate::xbau::postfach::Nachrichtenstand> {
        let (_, a) = self.portalakte_laden(p, verfahren, wer, true).await?;
        abgabe_ref(vorgang_ref(&a, vorgang)?, abgabe)?;
        let quelle = a
            .abgaben
            .iter()
            .find(|b| b.id == abgabe)
            .ok_or_else(|| fehler("Verfahrensabgabe nicht gefunden"))?;
        let artefakt = quelle
            .artefakte
            .iter()
            .find(|a| a.format == "xbau")
            .ok_or_else(|| fehler("Dieser eingefrorene Stand enthält kein XBau-Artefakt"))?;
        let bytes = paket::laden(self, artefakt).await?;
        let pruefung =
            crate::xbau::austausch::pruefen("XBau.zip", &bytes, self.xbau_pruefer.as_ref()).await?;
        if pruefung.kopf.id != abgabe {
            return Err(fehler(
                "XBau-Nachrichten-ID passt nicht zur eingefrorenen Abgabe",
            ));
        }
        let d = self
            .ablegen_vorgang(
                p,
                None,
                None,
                "XBau-Abgabestand.zip",
                Bytes::from(bytes),
                wer,
                crate::domain::Aktion::DokumentAbgelegt,
                " · eingefrorenes Portal-XBau",
                Some(Uuid::new_v5(&abgabe, b"portal.xbau.original")),
                vec![],
            )
            .await?;
        self.xbau_aufnehmen(
            p,
            crate::xbau::postfach::Nachrichtenaufnahme {
                original: Dokumentbezug {
                    dokument_id: d.document_id,
                    version: d.version,
                },
                richtung: crate::xbau::postfach::Richtung::Ausgang,
                zuordnung_bestaetigt: true,
            },
            wer,
        )
        .await
    }
    async fn portal_dokument_pruefen(&self, p: ProjectId, d: &Dokumentbezug) -> CoreResult<()> {
        if !self
            .repos
            .documents
            .history(d.dokument_id)
            .await?
            .iter()
            .any(|v| v.project_id == p && v.version == d.version)
        {
            return Err(fehler(
                "Originalbeleg gehört nicht in dieser Version zum Projekt",
            ));
        }
        Ok(())
    }
}
fn basis_pruefen(aktuell: i64, basis: i64) -> CoreResult<()> {
    if aktuell != basis {
        return Err(CoreError::VersionConflict {
            expected: basis,
            actual: aktuell,
        });
    }
    Ok(())
}
fn vorgang_ref(a: &Verfahrensakte, id: Uuid) -> CoreResult<&Portalvorgang> {
    a.portal
        .iter()
        .flat_map(|p| &p.vorgaenge)
        .find(|v| v.id == id)
        .ok_or_else(|| fehler("Portalvorgang nicht in dieser Verfahrensakte"))
}
fn vorgang_mut(a: &mut Verfahrensakte, id: Uuid) -> CoreResult<&mut Portalvorgang> {
    a.portal
        .iter_mut()
        .flat_map(|p| &mut p.vorgaenge)
        .find(|v| v.id == id)
        .ok_or_else(|| fehler("Portalvorgang nicht in dieser Verfahrensakte"))
}
fn abgabe_ref(v: &Portalvorgang, id: Uuid) -> CoreResult<&Portalabgabe> {
    v.abgaben
        .iter()
        .find(|b| b.id == id)
        .ok_or_else(|| fehler("Portalabgabe nicht in diesem Vorgang"))
}
fn ereignis(w: &Portalereigniswunsch, am: DateTime<Utc>, von: Uuid) -> Portalereignis {
    Portalereignis {
        id: w.id,
        art: w.art,
        datum: w.datum,
        referenz: w.referenz.clone(),
        notiz: w.notiz.clone(),
        beleg: w.beleg.clone(),
        dateiabgleich: w.dateiabgleich,
        bestaetigte_dateien: w.bestaetigte_dateien.clone(),
        berichtigt: w.berichtigt,
        erfasst_am: am,
        erfasst_von: von,
    }
}
