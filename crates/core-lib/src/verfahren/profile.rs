//! Versionierte Bürokataloge mit dreistufigen Anforderungen. Keine automatische
//! Zusage gesetzlicher Vollständigkeit; die fachliche Bestätigung ist explizit.
use super::*;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Vorhabenprofil {
    pub bundesland: crate::domain::Bundesland,
    pub art: Verfahrensart,
    pub gebaeudeklasse: Option<u32>,
    pub sonderbau: Option<bool>,
    pub nutzung: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Profilregel {
    pub id: String,
    pub titel: String,
    pub quelle: String,
    pub sonderbau: Option<bool>,
    pub ab_gebaeudeklasse: Option<u32>,
    pub nutzung: Option<String>,
    /// Verfahrensarten, für die die Anforderung gilt; fehlt = alle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub arten: Option<Vec<Verfahrensart>>,
    /// Dokumentkategorien, deren Unterlagen die Anforderung belegen können;
    /// ihre neuesten Versionen werden beim Anwenden vorgeschlagen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<String>>", optional)]
    pub kategorien: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Anforderungskatalog {
    pub id: String,
    pub version: String,
    pub titel: String,
    pub hinweis: String,
    pub bundesland: Option<crate::domain::Bundesland>,
    pub art: Option<Verfahrensart>,
    pub regeln: Vec<Profilregel>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Profilstand {
    pub vorhaben: Vorhabenprofil,
    pub katalog: Anforderungskatalog,
    pub bestaetigt_von: Option<Uuid>,
    pub bestaetigt_am: Option<DateTime<Utc>>,
}
impl Anforderungskatalog {
    pub fn standard() -> CoreResult<Self> {
        serde_json::from_str(include_str!(
            "../../../../workflow-regeln/verfahrensprofil.json"
        ))
        .map_err(|e| fehler(e.to_string()))
    }
    /// Mitgelieferte Kataloge: Büroprofil und Landespakete.
    pub fn mitgeliefert() -> CoreResult<Vec<Self>> {
        Ok(vec![
            Self::standard()?,
            serde_json::from_str(include_str!(
                "../../../../workflow-regeln/nrw-bauvorlagen.json"
            ))
            .map_err(|e| fehler(e.to_string()))?,
        ])
    }
    pub fn pruefen(&self) -> CoreResult<()> {
        crate::plattform::text(&self.id, 120)?;
        crate::plattform::text(&self.version, 120)?;
        crate::plattform::text(&self.titel, 500)?;
        if self.regeln.is_empty() || self.regeln.len() > 200 || self.hinweis.len() > 8000 {
            return Err(fehler("Katalog benötigt 1 bis 200 Anforderungen"));
        }
        let mut ids = std::collections::HashSet::new();
        for r in &self.regeln {
            crate::plattform::text(&r.id, 120)?;
            crate::plattform::text(&r.titel, 500)?;
            crate::plattform::text(&r.quelle, 4000)?;
            if !ids.insert(&r.id)
                || r.ab_gebaeudeklasse.is_some_and(|n| !(1..=5).contains(&n))
                || r.arten.as_ref().is_some_and(|a| a.is_empty())
                || r.kategorien.len() > 50
            {
                return Err(fehler("Ungültige oder doppelte Profilregel"));
            }
        }
        Ok(())
    }
    pub fn vorschlaege(&self, vid: Uuid, profil: &Vorhabenprofil) -> CoreResult<Vec<Anforderung>> {
        self.pruefen()?;
        if profil.gebaeudeklasse.is_some_and(|n| !(1..=5).contains(&n))
            || profil.nutzung.len() > 500
            || self.bundesland.is_some_and(|l| l != profil.bundesland)
            || self.art.as_ref().is_some_and(|a| a != &profil.art)
        {
            return Err(fehler("Vorhaben passt nicht zum Anforderungskatalog"));
        }
        self.regeln
            .iter()
            .filter(|r| r.arten.as_ref().is_none_or(|a| a.contains(&profil.art)))
            .map(|r| {
                let passt = r.sonderbau.is_none_or(|s| profil.sonderbau == Some(s))
                    && r.ab_gebaeudeklasse
                        .is_none_or(|n| profil.gebaeudeklasse.is_some_and(|g| g >= n))
                    && r.nutzung
                        .as_ref()
                        .is_none_or(|n| n.eq_ignore_ascii_case(profil.nutzung.trim()));
                Ok(Anforderung {
                    id: Uuid::new_v5(&vid, format!("anforderung/{}/{}", self.id, r.id).as_bytes()),
                    titel: r.titel.clone(),
                    erforderlich: Erforderlichkeit::Ungeklaert,
                    grund: if passt {
                        "Profilbedingung erfüllt; Erforderlichkeit fachlich bestätigen".into()
                    } else {
                        "Profilbedingung nicht erfüllt oder Angaben fehlen; Erforderlichkeit prüfen"
                            .into()
                    },
                    quelle: format!(
                        "{} · Version {} · {}: {}",
                        self.id, self.version, r.id, r.quelle
                    ),
                    dokumente: vec![],
                    freigegeben: false,
                    verantwortlich: None,
                })
            })
            .collect()
    }
}
/// Kennung der Anforderung „Zustimmungserklärung“ eines Kunden in einem Verfahren.
fn zustimmungs_id(verfahren: Uuid, kunde: crate::kunden::KundeId) -> Uuid {
    Uuid::new_v5(
        &verfahren,
        format!("anforderung/zustimmung/{kunde}").as_bytes(),
    )
}
/// Kennung der Anforderung „Baulast prüfen“ eines Flurstücks in einem Verfahren.
fn baulast_id(verfahren: Uuid, flurstueck: &str) -> Uuid {
    Uuid::new_v5(
        &verfahren,
        format!("anforderung/baulast/{flurstueck}").as_bytes(),
    )
}
const ZUSTIMMUNG_QUELLE: &str = "Projektbeteiligte";
const BAULAST_QUELLE: &str = "Flurstücke";
impl CoreServices {
    /// Hält die aus Projektdaten abgeleiteten Anforderungen der Akte aktuell:
    /// Verfügungsberechtigte, die nicht Bauherr sind, brauchen eine
    /// Zustimmungserklärung; Flurstücke mit Rolle „Sonstiges“ (Zuwegung,
    /// Baulastfläche) eine Klärung der Baulast. Neue Anforderungen bleiben
    /// ungeklärt. Entfällt der Anlass, bleibt die Anforderung als Akteneintrag
    /// stehen; der Rückgabewert nennt sie zur fachlichen Prüfung.
    pub(crate) async fn anforderungen_abgleichen(
        &self,
        p: ProjectId,
        id: Uuid,
        a: &mut Verfahrensakte,
    ) -> CoreResult<Vec<String>> {
        let bauherren = self.repos.kunden.bauherren(p).await?;
        let beteiligte = self.repos.kunden.beteiligte(p).await?;
        let mut abgeleitet: Vec<Anforderung> = crate::kunden::zustimmung_erforderlich(
            &bauherren,
            &beteiligte,
        )
        .into_iter()
        .map(|b| Anforderung {
            id: zustimmungs_id(id, b.kunde.id),
            titel: format!("Zustimmungserklärung von {}", b.kunde.anzeigename()),
            erforderlich: Erforderlichkeit::Ungeklaert,
            grund: format!(
                "{} ist nicht Bauherr; Form und Erforderlichkeit nach Landesrecht bestätigen",
                b.rolle.name()
            ),
            quelle: ZUSTIMMUNG_QUELLE.into(),
            dokumente: vec![],
            freigegeben: false,
            verantwortlich: None,
        })
        .collect();
        let projekt = self.repos.projects.get(p).await?;
        abgeleitet.extend(
            crate::grundstuecksinformationen::informationen(&projekt.planungsrecht)
                .baulastflurstuecke()
                .map(|f| Anforderung {
                    id: baulast_id(id, &f.id),
                    titel: format!("Baulast für Flurstück {} klären", f.bezeichnung()),
                    erforderlich: Erforderlichkeit::Ungeklaert,
                    grund: crate::grundstuecksinformationen::BAULAST_GRUND.into(),
                    quelle: BAULAST_QUELLE.into(),
                    dokumente: vec![],
                    freigegeben: false,
                    verantwortlich: None,
                }),
        );
        let aktuell: Vec<Uuid> = abgeleitet.iter().map(|r| r.id).collect();
        for r in abgeleitet {
            if !a.anforderungen.iter().any(|x| x.id == r.id) {
                a.anforderungen.push(r);
            }
        }
        Ok(a.anforderungen
            .iter()
            .filter(|r| {
                [ZUSTIMMUNG_QUELLE, BAULAST_QUELLE].contains(&r.quelle.as_str())
                    && r.erforderlich != Erforderlichkeit::NichtErforderlich
                    && !aktuell.contains(&r.id)
            })
            .map(|r| {
                format!(
                    "{}: nach den aktuellen {} nicht mehr erforderlich. Anforderung prüfen.",
                    r.titel,
                    if r.quelle == ZUSTIMMUNG_QUELLE {
                        "Projektbeteiligten"
                    } else {
                        "Flurstücken"
                    }
                )
            })
            .collect())
    }
    pub async fn verfahren_profil_anwenden(
        &self,
        p: ProjectId,
        id: Uuid,
        basis: i64,
        profil: Vorhabenprofil,
        katalog: Anforderungskatalog,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        let (_, mut a) = self.verfahrensakte(p, id).await?;
        if a.art != profil.art || self.get_project(p).await?.bundesland != profil.bundesland {
            return Err(fehler(
                "Profil gehört nicht zu Bundesland und Verfahrensart",
            ));
        }
        if a.profil.as_ref().is_some_and(|s| {
            s.katalog.id == katalog.id
                && s.katalog.version == katalog.version
                && s.katalog != katalog
        }) {
            return Err(fehler("Geänderter Katalog benötigt eine neue Version"));
        }
        let geaendert = a
            .profil
            .as_ref()
            .is_some_and(|s| s.vorhaben != profil || s.katalog != katalog);
        let kategorien = self.kategoriesnapshots(p).await?;
        let dateien = self.list_documents(p).await?;
        for mut r in katalog.vorschlaege(id, &profil)? {
            // Unterlagen passender Kategorie vorschlagen; die Erforderlichkeit
            // bleibt ungeklärt und die Freigabe offen.
            let regel = katalog.regeln.iter().find(|x| {
                Uuid::new_v5(
                    &id,
                    format!("anforderung/{}/{}", katalog.id, x.id).as_bytes(),
                ) == r.id
            });
            if let Some(regel) = regel.filter(|x| !x.kategorien.is_empty()) {
                r.dokumente = dateien
                    .iter()
                    .filter(|d| {
                        kategorien
                            .get(&d.document_id.0.to_string())
                            .cloned()
                            .flatten()
                            .is_some_and(|k| {
                                regel.kategorien.contains(&k.id)
                                    || k.hauptkategorie
                                        .is_some_and(|h| regel.kategorien.contains(&h))
                            })
                    })
                    .map(|d| Dokumentbezug {
                        dokument_id: d.document_id,
                        version: d.version,
                    })
                    .collect();
            }
            if let Some(alt) = a.anforderungen.iter_mut().find(|a| a.id == r.id) {
                if geaendert {
                    alt.titel = r.titel;
                    alt.quelle = r.quelle;
                    alt.freigegeben = false;
                }
                if alt.dokumente.is_empty() {
                    alt.dokumente = r.dokumente;
                }
            } else {
                a.anforderungen.push(r);
            }
        }
        // Vorhandene Anforderungen und Entscheidungen bleiben erhalten und prüfbar.
        a.profil = Some(Profilstand {
            vorhaben: profil,
            katalog,
            bestaetigt_von: None,
            bestaetigt_am: None,
        });
        self.verfahren_commit(p, id, basis, a, wer, false).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nrw_katalog_filtert_nach_verfahrensart_und_kennt_die_kategorien() {
        let kataloge = Anforderungskatalog::mitgeliefert().unwrap();
        let nrw = kataloge.iter().find(|k| k.id == "nrw-bauvorlagen").unwrap();
        nrw.pruefen().unwrap();
        assert_eq!(nrw.bundesland, Some(crate::domain::Bundesland::Nw));
        let standard = crate::dokumentkategorien::KategorienKatalog::default();
        for r in &nrw.regeln {
            for k in &r.kategorien {
                assert!(standard.kategorie(k).is_ok(), "{}: {k}", r.id);
            }
        }
        let vid = Uuid::new_v4();
        let profil = |art| Vorhabenprofil {
            bundesland: crate::domain::Bundesland::Nw,
            art,
            gebaeudeklasse: Some(2),
            sonderbau: Some(false),
            nutzung: "Wohnen".into(),
        };
        let bauantrag = nrw
            .vorschlaege(vid, &profil(Verfahrensart::Bauantrag))
            .unwrap();
        let vorbescheid = nrw
            .vorschlaege(vid, &profil(Verfahrensart::Vorbescheid))
            .unwrap();
        assert!(
            bauantrag
                .iter()
                .any(|r| r.titel.contains("Baubeschreibung"))
        );
        assert!(
            !vorbescheid
                .iter()
                .any(|r| r.titel.contains("Baubeschreibung"))
        );
        assert!(
            vorbescheid
                .iter()
                .any(|r| r.quelle.contains("§ 16 BauPrüfVO"))
        );
        assert!(bauantrag.len() > vorbescheid.len());
    }
    #[test]
    fn bedingungen_lassen_fachentscheidungen_offen_und_identitaeten_stabil() {
        let mut k = Anforderungskatalog::standard().unwrap();
        let id = Uuid::new_v4();
        let mut p = Vorhabenprofil {
            bundesland: crate::domain::Bundesland::By,
            art: Verfahrensart::Bauantrag,
            gebaeudeklasse: None,
            sonderbau: None,
            nutzung: String::new(),
        };
        let offen = k.vorschlaege(id, &p).unwrap();
        assert!(
            offen
                .iter()
                .all(|r| r.erforderlich == Erforderlichkeit::Ungeklaert && !r.freigegeben)
        );
        let sonder = offen
            .iter()
            .find(|r| r.titel.contains("Sonderbauunterlagen"))
            .unwrap();
        assert!(sonder.grund.contains("Angaben fehlen"));
        p.sonderbau = Some(true);
        p.gebaeudeklasse = Some(4);
        let passend = k.vorschlaege(id, &p).unwrap();
        let neu = passend.iter().find(|r| r.id == sonder.id).unwrap();
        assert!(neu.grund.contains("erfüllt"));
        assert_eq!(neu.erforderlich, Erforderlichkeit::Ungeklaert);
        k.version = "2".into();
        assert_eq!(k.vorschlaege(id, &p).unwrap()[0].id, offen[0].id);
        p.gebaeudeklasse = Some(6);
        assert!(k.vorschlaege(id, &p).is_err());
        p.gebaeudeklasse = Some(4);
        k.bundesland = Some(crate::domain::Bundesland::Nw);
        assert!(k.vorschlaege(id, &p).is_err());
        k.regeln.push(k.regeln[0].clone());
        assert!(k.pruefen().is_err());
    }
}
