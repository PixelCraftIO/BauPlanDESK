//! Einreichungsprüfung: eine Gesamtliste mit Ampel vor dem Absenden.
//!
//! Vertrag:
//! - Liest nur; rechnet keine eigene Fachprüfung. Die Punkte stammen aus der
//!   Projektübersicht (Unterlagenregister, Anforderungen der Verfahrensakte,
//!   Nachweisstände, Aufmaße, offene Nachforderungen), der Planliste, den
//!   Formularinstanzen (Abgleich mit den aktuellen Projektdaten) und den
//!   Lieferlisten der Fachplaner.
//! - Mit einem Verfahren werden Anforderungen, Nachforderungen und
//!   Lieferungen anderer Verfahren ausgeblendet; Planstände werden gegen die
//!   Behördenfassung dieses Verfahrens verglichen.
//! - Ampel je Punkt: rot = vor dem Einreichen zu erledigen, gelb = prüfen oder
//!   bewusst hinnehmen. Bereiche ohne Punkt sind grün. Die Gesamtampel ist die
//!   schlechteste Bereichsampel.
use crate::{
    CoreResult, CoreServices,
    benutzer::Benutzer,
    domain::ProjectId,
    planliste::Planliste,
    plattform::{ObjektArt, objekte::Fachplanerlieferung},
    projektuebersicht::{Arbeitspunkt, Dringlichkeit, Themenbereich, UebersichtsZiel},
    zugriff::ProjektRecht,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Ampel {
    Rot,
    Gelb,
    Gruen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Pruefbereich {
    Grundlagen,
    Unterlagen,
    Nachweise,
    Plaene,
    Formulare,
    Fachplaner,
    Verfahren,
}
impl Pruefbereich {
    pub const ALLE: [Self; 7] = [
        Self::Grundlagen,
        Self::Unterlagen,
        Self::Nachweise,
        Self::Plaene,
        Self::Formulare,
        Self::Fachplaner,
        Self::Verfahren,
    ];
    pub fn bezeichnung(self) -> &'static str {
        match self {
            Self::Grundlagen => "Grundlagen und Aufmaß",
            Self::Unterlagen => "Bauvorlagen",
            Self::Nachweise => "Nachweise und Berechnungen",
            Self::Plaene => "Pläne und Planindex",
            Self::Formulare => "Formulare",
            Self::Fachplaner => "Fachplaner",
            Self::Verfahren => "Verfahren",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Pruefpunkt {
    pub id: String,
    pub bereich: Pruefbereich,
    pub ampel: Ampel,
    pub titel: String,
    pub hinweis: String,
    pub faellig: Option<NaiveDate>,
    pub verantwortlich: Option<Uuid>,
    pub ziel: UebersichtsZiel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Bereichsstand {
    pub bereich: Pruefbereich,
    pub bezeichnung: String,
    pub ampel: Ampel,
    #[ts(type = "number")]
    pub rot: u32,
    #[ts(type = "number")]
    pub gelb: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Einreichungspruefung {
    pub stichtag: NaiveDate,
    pub verfahren: Option<Uuid>,
    pub gesamt: Ampel,
    pub bereiche: Vec<Bereichsstand>,
    /// Rot vor Gelb, innerhalb nach Bereich.
    pub punkte: Vec<Pruefpunkt>,
}

fn ziel(bereich: &str, objekt: Option<Uuid>) -> UebersichtsZiel {
    UebersichtsZiel {
        bereich: bereich.into(),
        objekt,
        register: None,
        eintrag: None,
    }
}

/// Verfahrenskennung aus einer Punktkennung der Projektübersicht
/// („anforderung:{verfahren}:{id}“, „nachforderung:{verfahren}:{id}“ …).
fn verfahren_von(id: &str) -> Option<Uuid> {
    id.split(':').nth(1).and_then(|s| Uuid::parse_str(s).ok())
}

/// Übernimmt die einreichungsrelevanten Punkte der Projektübersicht.
pub fn aus_uebersicht(punkte: &[Arbeitspunkt], verfahren: Option<Uuid>) -> Vec<Pruefpunkt> {
    punkte
        .iter()
        .filter_map(|a| {
            let art = a.id.split(':').next().unwrap_or_default();
            let bereich = match (a.thema, art) {
                (Themenbereich::Grundlagen, _) => Pruefbereich::Grundlagen,
                (Themenbereich::Unterlagen, _) => Pruefbereich::Unterlagen,
                (Themenbereich::Nachweise, _) => Pruefbereich::Nachweise,
                (Themenbereich::Verfahren, "anforderungen" | "profil" | "nachforderung") => {
                    Pruefbereich::Verfahren
                }
                _ => return None,
            };
            if let (Some(v), Some(eigen)) = (verfahren, verfahren_von(&a.id))
                && matches!(
                    art,
                    "anforderung" | "anforderungen" | "profil" | "nachforderung"
                )
                && v != eigen
            {
                return None;
            }
            let ampel = match a.dringlichkeit {
                Dringlichkeit::Ueberfaellig | Dringlichkeit::Heute | Dringlichkeit::Handeln => {
                    Ampel::Rot
                }
                Dringlichkeit::Pruefen | Dringlichkeit::Warten => Ampel::Gelb,
            };
            Some(Pruefpunkt {
                id: a.id.clone(),
                bereich,
                ampel,
                titel: a.titel.clone(),
                hinweis: a.hinweis.clone(),
                faellig: a.faellig,
                verantwortlich: a.verantwortlich,
                ziel: a.ziel.clone(),
            })
        })
        .collect()
}

/// Planliste: fehlender Index, neuere Datei ohne Index, entfernte Dokumente
/// und Pläne, deren aktueller Index der Behörde noch nicht vorliegt.
pub fn aus_planliste(l: &Planliste, verfahren: Option<Uuid>) -> Vec<Pruefpunkt> {
    let mut r = Vec::new();
    if l.zeilen.is_empty() {
        r.push(Pruefpunkt {
            id: "planliste".into(),
            bereich: Pruefbereich::Plaene,
            ampel: Ampel::Gelb,
            titel: "Planliste ist leer".into(),
            hinweis: "Pläne mit Nummer und Index erfassen, damit Planstände und Nachreichungen nachvollziehbar bleiben.".into(),
            faellig: None,
            verantwortlich: None,
            ziel: ziel("planliste", None),
        });
    }
    for z in &l.zeilen {
        let name = format!("{} {}", z.eintrag.plannummer, z.eintrag.titel);
        let did = z.eintrag.dokument_id.0;
        let mut punkt = |ampel, hinweis: String| {
            r.push(Pruefpunkt {
                id: format!("plan:{did}"),
                bereich: Pruefbereich::Plaene,
                ampel,
                titel: name.clone(),
                hinweis,
                faellig: None,
                verantwortlich: None,
                ziel: ziel("planliste", Some(did)),
            })
        };
        if z.aktuelle_version.is_none() {
            punkt(
                Ampel::Rot,
                "Die Plandatei wurde aus den Projektdateien entfernt.".into(),
            );
            continue;
        }
        let Some(index) = &z.index else {
            punkt(
                Ampel::Rot,
                "Kein Planindex vergeben; Index mit Datum und Änderung eintragen.".into(),
            );
            continue;
        };
        if z.ohne_index {
            punkt(
                Ampel::Rot,
                format!(
                    "Im DMS liegt eine neuere Version als die von Index {}; neuen Index vergeben oder alte Fassung einreichen.",
                    index.index
                ),
            );
            continue;
        }
        let behoerde = z
            .behoerde
            .iter()
            .filter(|b| verfahren.is_none_or(|v| v == b.verfahren));
        for b in behoerde {
            if let Some(alt) = b.planindex.as_ref().filter(|i| **i != index.index) {
                punkt(
                    Ampel::Gelb,
                    format!(
                        "{}: Behörde hat Index {alt}, aktuell ist Index {}. Mit der nächsten Abgabe nachreichen.",
                        b.bezeichnung, index.index
                    ),
                );
            }
        }
    }
    r
}

/// Lieferlisten: überfällig = rot; ausstehend = rot, wenn ohne Termin oder
/// wenn der Termin in den nächsten 7 Tagen liegt, sonst gelb; geliefert,
/// aber nicht geprüft = gelb.
pub fn aus_lieferungen(
    l: &[(Uuid, Fachplanerlieferung)],
    verfahren: Option<Uuid>,
    heute: NaiveDate,
) -> Vec<Pruefpunkt> {
    use crate::plattform::objekte::Lieferstatus as S;
    l.iter()
        .filter(|(_, x)| match (verfahren, x.verfahren) {
            (Some(v), Some(eigen)) => v == eigen,
            _ => true,
        })
        .filter_map(|(id, x)| {
            let offen: Vec<&str> = x
                .positionen
                .iter()
                .filter(|q| !q.geliefert)
                .map(|q| q.bezeichnung.as_str())
                .collect();
            let (ampel, hinweis) = match x.status {
                S::Geprueft | S::Entfallen => return None,
                S::Geliefert => (
                    Ampel::Gelb,
                    "Geliefert; Prüfung durch das Büro und Übernahme in die Bauvorlagen offen."
                        .to_string(),
                ),
                _ => {
                    let rot = x.faellig.is_none_or(|f| (f - heute).num_days() <= 7);
                    let termin = match x.faellig {
                        Some(f) if f < heute => format!("überfällig seit {}", f.format("%d.%m.%Y")),
                        Some(f) => format!("zugesagt bis {}", f.format("%d.%m.%Y")),
                        None => "ohne Liefertermin".into(),
                    };
                    let was = if offen.is_empty() {
                        String::new()
                    } else {
                        format!(" · ausstehend: {}", offen.join(", "))
                    };
                    (
                        if rot { Ampel::Rot } else { Ampel::Gelb },
                        format!("{}, {termin}{was}", x.status.bezeichnung()),
                    )
                }
            };
            Some(Pruefpunkt {
                id: format!("fachplaner:{id}"),
                bereich: Pruefbereich::Fachplaner,
                ampel,
                titel: format!("{}: {}", x.disziplin.bezeichnung(), x.fachplaner),
                hinweis,
                faellig: x.faellig,
                verantwortlich: x.verantwortlich.map(|b| b.0),
                ziel: ziel("fachplaner", Some(*id)),
            })
        })
        .collect()
}

/// Fasst die Punkte zur Gesamtliste mit Bereichsampeln zusammen.
pub fn zusammenfassen(
    stichtag: NaiveDate,
    verfahren: Option<Uuid>,
    mut punkte: Vec<Pruefpunkt>,
) -> Einreichungspruefung {
    punkte.sort_by(|a, b| {
        a.ampel
            .cmp(&b.ampel)
            .then(a.bereich.cmp(&b.bereich))
            .then(a.titel.cmp(&b.titel))
    });
    let bereiche: Vec<Bereichsstand> = Pruefbereich::ALLE
        .iter()
        .map(|b| {
            let zahl = |a| {
                punkte
                    .iter()
                    .filter(|p| p.bereich == *b && p.ampel == a)
                    .count() as u32
            };
            let (rot, gelb) = (zahl(Ampel::Rot), zahl(Ampel::Gelb));
            Bereichsstand {
                bereich: *b,
                bezeichnung: b.bezeichnung().into(),
                ampel: if rot > 0 {
                    Ampel::Rot
                } else if gelb > 0 {
                    Ampel::Gelb
                } else {
                    Ampel::Gruen
                },
                rot,
                gelb,
            }
        })
        .collect();
    Einreichungspruefung {
        stichtag,
        verfahren,
        gesamt: bereiche
            .iter()
            .map(|b| b.ampel)
            .min()
            .unwrap_or(Ampel::Gruen),
        bereiche,
        punkte,
    }
}

impl CoreServices {
    /// Gesamtliste vor dem Einreichen; `verfahren` grenzt auf eine Akte ein.
    pub async fn einreichungspruefung(
        &self,
        p: ProjectId,
        verfahren: Option<Uuid>,
        wer: &Benutzer,
    ) -> CoreResult<Einreichungspruefung> {
        self.einreichungspruefung_am(p, verfahren, crate::kalender::heute(), wer)
            .await
    }
    pub async fn einreichungspruefung_am(
        &self,
        p: ProjectId,
        verfahren: Option<Uuid>,
        heute: NaiveDate,
        wer: &Benutzer,
    ) -> CoreResult<Einreichungspruefung> {
        self.projekt_zugriff(p, wer, ProjektRecht::Lesen).await?;
        if let Some(v) = verfahren
            && self
                .repos
                .commit
                .fachobjekt(p, ObjektArt::Verfahren.modul(), &v.to_string())
                .await?
                .is_none()
        {
            return Err(crate::CoreError::InvalidInput(
                "Verfahrensakte gehört nicht zu diesem Projekt".into(),
            ));
        }
        let uebersicht = self.projektuebersicht_am(p, heute, wer).await?;
        let mut punkte = aus_uebersicht(&uebersicht.punkte, verfahren);
        punkte.extend(aus_planliste(&self.planliste(p, wer).await?, verfahren));
        let mut lieferungen = Vec::new();
        for f in self.fachobjekte(p, ObjektArt::Fachplanerlieferung).await? {
            let id = Uuid::parse_str(&f.objekt)
                .map_err(|e| crate::CoreError::InvalidInput(e.to_string()))?;
            let l: Fachplanerlieferung = serde_json::from_value(f.daten)
                .map_err(|e| crate::CoreError::InvalidInput(e.to_string()))?;
            lieferungen.push((id, l));
        }
        punkte.extend(aus_lieferungen(&lieferungen, verfahren, heute));
        // Formulare: neueste Fassung jeder Formularinstanz gegen die Projektdaten.
        let mut formulare = 0;
        for d in self.repos.documents.latest_for_project(p).await? {
            if self
                .formular_instanz(d.document_id, d.version)
                .await?
                .is_none()
            {
                continue;
            }
            formulare += 1;
            let (_, pdf) = self.load_version(d.document_id, d.version).await?;
            if let Some(h) = self
                .formular_hinweis(d.document_id, d.version, &d.file_name, &pdf)
                .await
            {
                punkte.push(Pruefpunkt {
                    id: format!("formular:{}", d.document_id.0),
                    bereich: Pruefbereich::Formulare,
                    ampel: Ampel::Gelb,
                    titel: d.file_name.clone(),
                    hinweis: h,
                    faellig: None,
                    verantwortlich: None,
                    ziel: ziel("formulare", Some(d.document_id.0)),
                });
            }
        }
        if formulare == 0 {
            punkte.push(Pruefpunkt {
                id: "formulare".into(),
                bereich: Pruefbereich::Formulare,
                ampel: Ampel::Gelb,
                titel: "Noch keine Antragsformulare erstellt".into(),
                hinweis: "Bauantragsformulare aus der Formularbibliothek ausfüllen; sie werden dann laufend gegen die Projektdaten geprüft.".into(),
                faellig: None,
                verantwortlich: None,
                ziel: ziel("formulare", None),
            });
        }
        Ok(zusammenfassen(heute, verfahren, punkte))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plattform::objekte::{Fachdisziplin, Lieferposition, Lieferstatus};

    fn tag(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }
    fn lieferung(status: Lieferstatus, faellig: Option<&str>) -> Fachplanerlieferung {
        Fachplanerlieferung {
            disziplin: Fachdisziplin::Tragwerksplanung,
            fachplaner: "Statikbüro Schmidt".into(),
            kontakt: String::new(),
            positionen: vec![
                Lieferposition {
                    bezeichnung: "Standsicherheitsnachweis".into(),
                    geliefert: false,
                },
                Lieferposition {
                    bezeichnung: "Positionsplan".into(),
                    geliefert: true,
                },
            ],
            angefragt_am: None,
            faellig: faellig.map(tag),
            status,
            verantwortlich: None,
            dokumente: vec![],
            verfahren: None,
            bemerkung: String::new(),
        }
    }

    #[test]
    fn lieferungen_bekommen_ampel_nach_termin_und_stand() {
        let heute = tag("2026-11-01");
        let l = vec![
            (
                Uuid::new_v4(),
                lieferung(Lieferstatus::Angefragt, Some("2026-10-30")),
            ),
            (
                Uuid::new_v4(),
                lieferung(Lieferstatus::Angefragt, Some("2026-12-15")),
            ),
            (Uuid::new_v4(), lieferung(Lieferstatus::Geliefert, None)),
            (Uuid::new_v4(), lieferung(Lieferstatus::Geprueft, None)),
        ];
        let p = aus_lieferungen(&l, None, heute);
        assert_eq!(p.len(), 3, "geprüfte Lieferung ist erledigt");
        assert_eq!(p[0].ampel, Ampel::Rot);
        assert!(p[0].hinweis.contains("überfällig seit 30.10.2026"));
        assert!(p[0].hinweis.contains("Standsicherheitsnachweis"));
        assert!(!p[0].hinweis.contains("Positionsplan"));
        assert_eq!(p[1].ampel, Ampel::Gelb);
        assert_eq!(p[2].ampel, Ampel::Gelb);
        // Lieferung für ein anderes Verfahren fällt heraus.
        let mut fremd = l[0].1.clone();
        fremd.verfahren = Some(Uuid::new_v4());
        assert!(
            aus_lieferungen(&[(Uuid::new_v4(), fremd)], Some(Uuid::new_v4()), heute).is_empty()
        );

        let e = zusammenfassen(heute, None, p);
        assert_eq!(e.gesamt, Ampel::Rot);
        assert_eq!(e.punkte[0].ampel, Ampel::Rot, "Rot zuerst");
        let f = e
            .bereiche
            .iter()
            .find(|b| b.bereich == Pruefbereich::Fachplaner)
            .unwrap();
        assert_eq!((f.rot, f.gelb, f.ampel), (1, 2, Ampel::Rot));
        assert!(
            e.bereiche
                .iter()
                .filter(|b| b.bereich != Pruefbereich::Fachplaner)
                .all(|b| b.ampel == Ampel::Gruen)
        );
        assert_eq!(zusammenfassen(heute, None, vec![]).gesamt, Ampel::Gruen);
    }

    #[test]
    fn uebersichtspunkte_anderer_verfahren_entfallen() {
        let v1 = Uuid::new_v4();
        let v2 = Uuid::new_v4();
        let punkt = |id: String, thema, d| Arbeitspunkt {
            id,
            thema,
            dringlichkeit: d,
            titel: "x".into(),
            hinweis: String::new(),
            faellig: None,
            verantwortlich: None,
            ziel: ziel("verfahren", None),
        };
        let a = vec![
            punkt(
                format!("anforderung:{v1}:{}", Uuid::new_v4()),
                Themenbereich::Unterlagen,
                Dringlichkeit::Handeln,
            ),
            punkt(
                format!("anforderung:{v2}:{}", Uuid::new_v4()),
                Themenbereich::Unterlagen,
                Dringlichkeit::Handeln,
            ),
            punkt(
                format!("nachforderung:{v1}:{}", Uuid::new_v4()),
                Themenbereich::Verfahren,
                Dringlichkeit::Warten,
            ),
            punkt(
                format!("unterlage:{}", Uuid::new_v4()),
                Themenbereich::Unterlagen,
                Dringlichkeit::Pruefen,
            ),
            punkt(
                format!("auflage:{v1}:{}", Uuid::new_v4()),
                Themenbereich::Verfahren,
                Dringlichkeit::Handeln,
            ),
            punkt(
                "aufgabe:1".into(),
                Themenbereich::Aufgaben,
                Dringlichkeit::Handeln,
            ),
        ];
        assert_eq!(
            aus_uebersicht(&a, None).len(),
            4,
            "Auflagen und Aufgaben gehören nicht dazu"
        );
        let r = aus_uebersicht(&a, Some(v1));
        assert_eq!(r.len(), 3);
        assert_eq!(
            r.iter().map(|p| p.ampel).collect::<Vec<_>>(),
            [Ampel::Rot, Ampel::Gelb, Ampel::Gelb]
        );
        assert_eq!(r[1].bereich, Pruefbereich::Verfahren);
    }
}
