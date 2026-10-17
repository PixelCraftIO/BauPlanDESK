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
            if !ids.insert(&r.id) || r.ab_gebaeudeklasse.is_some_and(|n| !(1..=5).contains(&n)) {
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
impl CoreServices {
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
        for r in katalog.vorschlaege(id, &profil)? {
            if let Some(alt) = a.anforderungen.iter_mut().find(|a| a.id == r.id) {
                if geaendert {
                    alt.titel = r.titel;
                    alt.quelle = r.quelle;
                    alt.freigegeben = false;
                }
            } else {
                a.anforderungen.push(r);
            }
        }
        // Verfügungsberechtigte, die nicht Bauherr sind, müssen zustimmen.
        let bauherren = self.repos.kunden.bauherren(p).await?;
        let beteiligte = self.repos.kunden.beteiligte(p).await?;
        for b in crate::kunden::zustimmung_erforderlich(&bauherren, &beteiligte) {
            let aid = Uuid::new_v5(
                &id,
                format!("anforderung/zustimmung/{}", b.kunde.id).as_bytes(),
            );
            if !a.anforderungen.iter().any(|x| x.id == aid) {
                a.anforderungen.push(Anforderung {
                    id: aid,
                    titel: format!("Zustimmungserklärung von {}", b.kunde.anzeigename()),
                    erforderlich: Erforderlichkeit::Ungeklaert,
                    grund: format!(
                        "{} ist nicht Bauherr; Form und Erforderlichkeit nach Landesrecht bestätigen",
                        b.rolle.name()
                    ),
                    quelle: "Projektbeteiligte".into(),
                    dokumente: vec![],
                    freigegeben: false,
                    verantwortlich: None,
                });
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
