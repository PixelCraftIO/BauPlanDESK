//! Quellenwahl und unveränderlicher Datenstand einer ausgefüllten PDF-Version.
use super::{Feldwert, Projektkontext};
use crate::{
    CoreError, CoreResult, CoreServices,
    domain::ProjectId,
    kunden::KundeId,
    storage::{ContentHash, StorageKey},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormularBezug {
    pub grundstueck_id: Option<uuid::Uuid>,
    pub bauherr_id: Option<KundeId>,
    /// Verfahren, aus dessen Akte Behörde und Aktenzeichen stammen. Fehlt in
    /// älteren Datenständen und bleibt dann aus der Prüfsumme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verfahren_id: Option<uuid::Uuid>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormularDatenstand {
    pub projekt_id: ProjectId,
    pub projekt_version: i64,
    pub bezug: FormularBezug,
    pub daten_hash: ContentHash,
    pub werte: Projektkontext,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Quellenwahl {
    pub id: uuid::Uuid,
    pub bezeichnung: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormularQuellen {
    pub datenstand: FormularDatenstand,
    pub grundstuecke: Vec<Quellenwahl>,
    pub bauherren: Vec<Quellenwahl>,
    /// Fehlt in älteren Formularinstanzen.
    #[serde(default)]
    pub verfahren: Vec<Quellenwahl>,
    pub hinweise: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormularInstanz {
    pub vorlagen_name: String,
    pub vorlagen_hash: ContentHash,
    pub vorlagen_storage_key: String,
    pub quellen: FormularQuellen,
    pub feldwerte: BTreeMap<String, Feldwert>,
}
impl CoreServices {
    pub async fn formular_quellen(
        &self,
        projekt: ProjectId,
        mut bezug: FormularBezug,
    ) -> CoreResult<FormularQuellen> {
        let p = self.repos.projects.get(projekt).await?;
        // Grundstücksangaben nur aus der konkreten Auswahl, nie aus einer
        // zufälligen ersten Einheit.
        let mut werte = Projektkontext::projekt(&p);
        let mut hinweise = Vec::new();
        let grundstuecke = match crate::grundstueckskontext::ermitteln(&p) {
            Ok(g) => g,
            Err(e) => {
                hinweise.push(format!("Baugrundstücksdaten sind zu prüfen: {e}"));
                vec![]
            }
        };
        let bauherren = self.repos.kunden.bauherren(projekt).await?;
        let beteiligte = self.repos.kunden.beteiligte(projekt).await?;
        let mut verfahren = Vec::new();
        for f in self.repos.commit.fachstaende(projekt, "verfahren").await? {
            let (Ok(id), Ok(akte)) = (
                uuid::Uuid::parse_str(&f.objekt),
                serde_json::from_value::<crate::verfahren::Verfahrensakte>(f.daten),
            ) else {
                continue;
            };
            verfahren.push((id, akte));
        }
        if bezug.grundstueck_id.is_none() && grundstuecke.len() == 1 {
            bezug.grundstueck_id = Some(grundstuecke[0].id);
        }
        if bezug.bauherr_id.is_none() && bauherren.len() == 1 {
            bezug.bauherr_id = Some(bauherren[0].id);
        }
        if bezug.verfahren_id.is_none() && verfahren.len() == 1 {
            bezug.verfahren_id = Some(verfahren[0].0);
        }
        let gewaehlt = match bezug.grundstueck_id {
            Some(id) => Some(grundstuecke.iter().find(|g| g.id == id).ok_or_else(|| {
                CoreError::InvalidInput("Gewähltes Baugrundstück ist nicht mehr verfügbar".into())
            })?),
            None => None,
        };
        if let Some(g) = gewaehlt {
            hinweise.extend(werte.grundstueck(&p, g));
            if g.bezugsflaeche_m2.is_none() {
                hinweise
                    .push("Für das gewählte Baugrundstück fehlt eine gültige Bezugsfläche.".into());
            }
        } else if grundstuecke.len() > 1 {
            hinweise.push("Mehrere Baugrundstücke: Bitte die Quelle wählen oder die Formularwerte manuell eingeben.".into());
        }
        if let Some(id) = bezug.bauherr_id {
            werte.bauherr(bauherren.iter().find(|b| b.id == id).ok_or_else(|| {
                CoreError::InvalidInput("Gewählter Bauherr gehört nicht zum Projekt".into())
            })?);
        } else if bauherren.len() > 1 {
            hinweise.push("Mehrere Bauherren: Bitte die Quelle wählen oder die Formularwerte manuell eingeben.".into());
        }
        beteiligte_setzen(&mut werte, &p, gewaehlt, &bauherren, &beteiligte);

        let antrag = self.bauantrag(projekt).await?.antrag;
        let a = &antrag.grundstueck.anschrift;
        werte.anschrift(
            "grundstueck",
            &a.strasse,
            &a.hausnummer,
            &a.postleitzahl,
            &a.ort,
        );
        werte.setze(
            "vorhaben.bezeichnung",
            if antrag.bezeichnung.trim().is_empty() {
                &p.name
            } else {
                &antrag.bezeichnung
            },
        );
        let buero = self.buero().await?;
        werte.setze("buero.name", &buero.name);
        let a = &buero.anschrift;
        werte.anschrift("buero", &a.strasse, &a.hausnummer, &a.postleitzahl, &a.ort);
        match antrag.entwurfsverfasser_von(&buero) {
            Some(e) => {
                werte.setze("entwurfsverfasser.name", e.anzeigename());
                werte.setze("entwurfsverfasser.vorname", &e.vorname);
                werte.setze("entwurfsverfasser.familienname", &e.familienname);
                werte.setze("entwurfsverfasser.berufsbezeichnung", &e.berufsbezeichnung);
                werte.setze("entwurfsverfasser.mitgliedsnummer", &e.mitgliedsnummer);
                if let Some(l) = e.kammerland {
                    werte.setze("entwurfsverfasser.kammerland", l.name());
                }
            }
            None if buero.entwurfsverfasser.len() > 1 => hinweise.push(
                "Mehrere Entwurfsverfasser im Büro: Bitte im Bauantrag die verantwortliche Person wählen.".into(),
            ),
            None => {}
        }

        match bezug.verfahren_id {
            Some(id) => {
                let (_, akte) = verfahren.iter().find(|(v, _)| *v == id).ok_or_else(|| {
                    CoreError::InvalidInput("Gewähltes Verfahren gehört nicht zum Projekt".into())
                })?;
                werte.setze("verfahren.bezeichnung", &akte.bezeichnung);
                werte.setze("verfahren.aktenzeichen", &akte.aktenzeichen);
                werte.setze("behoerde.name", &akte.behoerde.name);
            }
            None => {
                if verfahren.len() > 1 {
                    hinweise.push("Mehrere Verfahren: Für Behörde und Aktenzeichen bitte das Verfahren wählen.".into());
                }
                werte.setze("behoerde.name", &antrag.bauaufsicht.name);
            }
        }
        werte.leere_entfernen();
        let daten_hash = ContentHash::of(
            &serde_json::to_vec(&(&bezug, werte.datenstand()))
                .map_err(|e| CoreError::InvalidInput(e.to_string()))?,
        );
        Ok(FormularQuellen {
            datenstand: FormularDatenstand {
                projekt_id: projekt,
                projekt_version: p.version,
                bezug,
                daten_hash,
                werte,
            },
            grundstuecke: grundstuecke
                .iter()
                .map(|g| Quellenwahl {
                    id: g.id,
                    bezeichnung: g.bezeichnung.clone(),
                })
                .collect(),
            bauherren: bauherren
                .iter()
                .map(|b| Quellenwahl {
                    id: b.id.0,
                    bezeichnung: b.anzeigename(),
                })
                .collect(),
            verfahren: verfahren
                .iter()
                .map(|(id, a)| Quellenwahl {
                    id: *id,
                    bezeichnung: if a.behoerde.name.trim().is_empty() {
                        a.bezeichnung.clone()
                    } else {
                        format!("{} ({})", a.bezeichnung, a.behoerde.name)
                    },
                })
                .collect(),
            hinweise,
        })
    }
    pub(crate) async fn formular_instanz_zusatz(
        &self,
        projekt: ProjectId,
        name: &str,
        pdf: &[u8],
        werte: &BTreeMap<String, Feldwert>,
        stand: Option<FormularDatenstand>,
    ) -> CoreResult<crate::commit::DokumentZusatz> {
        let quellen = self
            .formular_quellen(
                projekt,
                stand.as_ref().map(|s| s.bezug.clone()).unwrap_or_default(),
            )
            .await?;
        if let Some(s) = stand
            && (s.projekt_id != projekt || s.daten_hash != quellen.datenstand.daten_hash)
        {
            return Err(CoreError::InvalidInput("Die Formularquellen haben sich geändert. Bitte Projektdaten erneut laden und die Eingaben prüfen. Ihre Eingaben bleiben erhalten.".into()));
        }
        let hash = ContentHash::of(pdf);
        // Inhaltsadressierte Vorlage außerhalb der Projektdateiliste. Spätere Bibliotheksänderungen betreffen sie nicht.
        let key = StorageKey::new(format!("formularvorlagen/{hash}.pdf"))?;
        self.storage
            .save(&key, bytes::Bytes::copy_from_slice(pdf))
            .await?;
        let instanz = FormularInstanz {
            vorlagen_name: name.into(),
            vorlagen_hash: hash,
            vorlagen_storage_key: key.to_string(),
            quellen,
            feldwerte: werte.clone(),
        };
        Ok(crate::commit::DokumentZusatz {
            modul: "formularinstanz".into(),
            schema_version: 1,
            daten: serde_json::to_value(instanz)
                .map_err(|e| CoreError::InvalidInput(e.to_string()))?,
        })
    }
    pub async fn formular_instanz(
        &self,
        id: crate::domain::DocumentId,
        version: i64,
    ) -> CoreResult<Option<FormularInstanz>> {
        self.load_version(id, version).await?;
        self.repos
            .commit
            .dokumentzusatz(id, version, "formularinstanz")
            .await?
            .map(|z| {
                if z.schema_version != 1 {
                    return Err(CoreError::InvalidInput(
                        "Unbekannte Formularinstanzversion".into(),
                    ));
                }
                serde_json::from_value(z.daten).map_err(|e| CoreError::InvalidInput(e.to_string()))
            })
            .transpose()
    }
}

/// Eigentümer des gewählten Baugrundstücks (aus den Flurstücken, sonst alle
/// Grundstückseigentümer des Projekts) und weitere Beteiligte nach Rolle. Bei
/// einer Person stehen Name und Anschrift unter `rolle.*`, bei mehreren nur
/// die Namen in `rolle.name`.
fn beteiligte_setzen(
    werte: &mut Projektkontext,
    p: &crate::domain::Project,
    grundstueck: Option<&crate::grundstueckskontext::GrundstueckKontext>,
    bauherren: &[crate::kunden::Kunde],
    beteiligte: &[crate::kunden::Projektbeteiligter],
) {
    use crate::kunden::{Beteiligtenrolle, Kunde};
    fn setze(werte: &mut Projektkontext, praefix: &str, kunden: Vec<&Kunde>) {
        match kunden.as_slice() {
            [] => {}
            [k] => werte.person(praefix, k),
            _ => werte.setze(
                &format!("{praefix}.name"),
                kunden
                    .iter()
                    .map(|k| k.anzeigename())
                    .collect::<Vec<_>>()
                    .join("; "),
            ),
        }
    }
    let nach_rolle = |rolle| {
        beteiligte
            .iter()
            .filter(move |b| b.rolle == rolle)
            .map(|b| &b.kunde)
    };
    let infos = crate::grundstuecksinformationen::informationen(&p.planungsrecht);
    let flurstuecke: Vec<_> = infos
        .flurstuecke
        .iter()
        .filter(|f| grundstueck.is_some_and(|g| g.flurstueck_ids.contains(&f.id)))
        .collect();
    let mut eigentuemer: Vec<&Kunde> = Vec::new();
    for id in flurstuecke.iter().flat_map(|f| &f.eigentuemer_kunden) {
        if let Some(k) = bauherren
            .iter()
            .chain(beteiligte.iter().map(|b| &b.kunde))
            .find(|k| k.id == *id)
            && !eigentuemer.iter().any(|e| e.id == k.id)
        {
            eigentuemer.push(k);
        }
    }
    let freitext: Vec<_> = flurstuecke
        .iter()
        .map(|f| f.eigentuemer.trim())
        .filter(|e| !e.is_empty())
        .collect();
    if eigentuemer.is_empty() && !freitext.is_empty() {
        let mut namen: Vec<&str> = Vec::new();
        for e in freitext {
            if !namen.contains(&e) {
                namen.push(e);
            }
        }
        werte.setze("eigentuemer.name", namen.join("; "));
    } else {
        if eigentuemer.is_empty() {
            eigentuemer = nach_rolle(Beteiligtenrolle::Grundstueckseigentuemer).collect();
        }
        setze(werte, "eigentuemer", eigentuemer);
    }
    for (praefix, rolle) in [
        ("erbbauberechtigter", Beteiligtenrolle::Erbbauberechtigter),
        ("bevollmaechtigter", Beteiligtenrolle::Bevollmaechtigter),
        ("fachplaner", Beteiligtenrolle::Fachplaner),
    ] {
        setze(werte, praefix, nach_rolle(rolle).collect());
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormularSpeicheroptionen {
    pub vorgang: Option<uuid::Uuid>,
    pub datenstand: Option<FormularDatenstand>,
}
