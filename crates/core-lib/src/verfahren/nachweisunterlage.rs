//! Nachweisstände der Projektakte als Unterlage einer Anforderung.
//!
//! Eine Anforderung mit Nachweisbereichen (etwa „Berechnung des Maßes der
//! baulichen Nutzung“ mit GRZ und GFZ) wird durch die Nachweisstände dieser
//! Bereiche belegt. Der Core druckt die maßgebenden Stände in eine PDF,
//! legt sie als Projektunterlage ab und ordnet genau diese Version der
//! Anforderung zu. Nach einer Neuberechnung entsteht mit demselben Schritt
//! die nächste Version desselben Dokuments.
use super::*;
use crate::plattform::nachweise::NachweisAnsicht;

/// Maßgebende Nachweisstände der Bereiche: je Nachweis der jüngste, nicht
/// ersetzte Stand. `Err` nennt, was fehlt oder veraltet ist.
pub fn massgebende_staende(
    nachweise: &[NachweisAnsicht],
    bereiche: &[String],
) -> Result<Vec<Uuid>, String> {
    let mut staende = Vec::new();
    for bereich in bereiche {
        let titel = crate::bereichsstatus::bereichstitel(bereich);
        let mut im_bereich: Vec<&NachweisAnsicht> = nachweise
            .iter()
            .filter(|n| {
                n.ersetzt_durch.is_none()
                    && crate::bereichsstatus::nachweisbereich(&n.stand.eingabe) == bereich
            })
            .collect();
        if im_bereich.is_empty() {
            return Err(format!("Für „{titel}“ ist noch kein Nachweis erstellt."));
        }
        if let Some(alt) = im_bereich.iter().find(|n| !n.aktuell) {
            return Err(format!(
                "Nachweis „{}“ beruht nicht mehr auf dem aktuellen Planungsstand; erst neu erstellen.",
                alt.stand.bezeichnung
            ));
        }
        im_bereich.sort_by_key(|n| (n.stand.bezeichnung.clone(), n.stand.erstellt_am));
        staende.extend(im_bereich.iter().map(|n| n.stand.id));
    }
    Ok(staende)
}

impl CoreServices {
    /// Druckt die maßgebenden Nachweisstände der Anforderung, legt die PDF
    /// als Projektunterlage ab (bei Wiederholung als neue Version) und
    /// ordnet sie der Anforderung zu.
    pub async fn verfahren_nachweis_als_unterlage(
        &self,
        p: ProjectId,
        vid: Uuid,
        basis: i64,
        anforderung: Uuid,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        self.projekt_zugriff(p, wer, crate::zugriff::ProjektRecht::Bearbeiten)
            .await?;
        let (f, mut a) = self.verfahrensakte(p, vid).await?;
        if f.revision != basis {
            return Err(CoreError::VersionConflict {
                expected: basis,
                actual: f.revision,
            });
        }
        let r = a
            .anforderungen
            .iter()
            .find(|r| r.id == anforderung)
            .ok_or_else(|| fehler("Anforderung nicht gefunden"))?;
        if r.nachweise.is_empty() {
            return Err(fehler(
                "Diese Anforderung ist keinem Nachweis der Projektakte zugeordnet",
            ));
        }
        let staende =
            massgebende_staende(&self.nachweise(p).await?, &r.nachweise).map_err(fehler)?;
        let alt = r.nachweisunterlage.clone();
        if alt.as_ref().is_some_and(|u| u.staende == staende) {
            return Ok(f);
        }
        let katalog = self.druckkatalog(p).await?;
        let titel = r.titel.clone();
        let pdf = self
            .druckausgabe(
                p,
                crate::druckausgabe::Druckauftrag {
                    gestaltung: None,
                    aktenzeichen: a.aktenzeichen.trim().to_string(),
                    datum: None,
                    titel: titel.clone(),
                    absender: katalog.absender,
                    empfaenger: String::new(),
                    anschreiben: String::new(),
                    mit_anschreiben: false,
                    inhaltsverzeichnis: staende.len() > 1,
                    deckblatt: false,
                    trennblaetter: false,
                    aufgestellt_von: None,
                    projekt_version: katalog.projekt_version,
                    quellen: staende
                        .iter()
                        .map(|id| crate::druckausgabe::Druckquelle::Nachweis { id: *id })
                        .collect(),
                },
            )
            .await?
            .pdf;
        // Vorhandenes Dokument fortschreiben, solange es noch existiert.
        let bisher = match &alt {
            Some(u) => self
                .repos
                .documents
                .latest(u.dokument.dokument_id)
                .await
                .ok(),
            None => None,
        };
        let name: String = titel
            .chars()
            .map(|c| if "\\/:*?\"<>|".contains(c) { '-' } else { c })
            .take(120)
            .collect();
        let kategorie = self
            .kategorienkatalog()
            .await?
            .kategorien
            .iter()
            .any(|k| k.id == "berechnung" && k.aktiv)
            .then_some("berechnung");
        let v = self
            .ablegen_vorgang(
                p,
                bisher.as_ref().map(|d| d.document_id),
                bisher.as_ref().map(|d| d.version),
                &bisher
                    .as_ref()
                    .map(|d| d.file_name.clone())
                    .unwrap_or_else(|| format!("{}.pdf", name.trim())),
                bytes::Bytes::from(pdf),
                wer,
                crate::domain::Aktion::DokumentAbgelegt,
                "Nachweis als Unterlage der Verfahrensakte",
                None,
                if bisher.is_some() {
                    vec![]
                } else {
                    vec![crate::commit::DokumentZusatz {
                        modul: "dokumentkategorie".into(),
                        schema_version: 1,
                        daten: serde_json::to_value(kategorie)
                            .map_err(|e| fehler(e.to_string()))?,
                    }]
                },
            )
            .await?;
        let neu = Dokumentbezug {
            dokument_id: v.document_id,
            version: v.version,
        };
        let r = a
            .anforderungen
            .iter_mut()
            .find(|r| r.id == anforderung)
            .expect("oben gefunden");
        if let Some(u) = &alt {
            r.dokumente
                .retain(|d| d.dokument_id != u.dokument.dokument_id);
        }
        r.dokumente.push(neu.clone());
        r.nachweisunterlage = Some(Nachweisunterlage {
            dokument: neu,
            staende,
        });
        // Der Inhalt hat sich geändert: Die fachliche Freigabe ist neu zu geben.
        r.freigegeben = false;
        if r.erforderlich == Erforderlichkeit::Ungeklaert {
            r.erforderlich = Erforderlichkeit::Erforderlich;
        }
        self.verfahren_commit(p, vid, basis, a, wer, false).await
    }
}
