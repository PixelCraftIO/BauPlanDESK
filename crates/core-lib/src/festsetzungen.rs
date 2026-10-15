//! Projektbezogene Festsetzungen und quantitative Vergleiche.
use crate::calc::{self, Baugebiet, Kennzahlen, MassDerNutzung};
use crate::domain::{BebauungsplanArt, Planungsrecht};
use crate::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Geschossigkeit {
    #[default]
    Maximal,
    Zwingend,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Festsetzungen {
    pub nichtvollgeschoss_faktor: Option<f64>,
    pub nichtvollgeschoss_festsetzung: String,
    pub baugebiet: Option<Baugebiet>,
    pub grz: Option<f64>,
    pub grz_gesamt: Option<f64>,
    pub gfz: Option<f64>,
    pub bmz: Option<f64>,
    pub grundflaeche_m2: Option<f64>,
    pub geschossflaeche_m2: Option<f64>,
    pub ueberbaubare_flaeche_m2: Option<f64>,
    pub vollgeschosse: Option<u32>,
    pub geschossigkeit: Geschossigkeit,
}

impl Festsetzungen {
    pub fn validate(&self) -> CoreResult<()> {
        if let Some(f) = self.nichtvollgeschoss_faktor
            && (!f.is_finite()
                || !(0.0..=1.0).contains(&f)
                || (f > 0.0 && self.nichtvollgeschoss_festsetzung.trim().is_empty()))
        {
            return Err(CoreError::InvalidInput(
                "Nichtvollgeschoss-Anrechnung: Faktor 0 bis 1 und Planfestsetzung erforderlich"
                    .into(),
            ));
        }
        for (name, wert) in [
            ("GRZ", self.grz),
            ("GRZ einschließlich Nebenanlagen", self.grz_gesamt),
            ("GFZ", self.gfz),
            ("BMZ", self.bmz),
            ("Grundfläche", self.grundflaeche_m2),
            ("Geschossfläche", self.geschossflaeche_m2),
            ("Baufensterfläche", self.ueberbaubare_flaeche_m2),
        ] {
            if let Some(v) = wert
                && (!v.is_finite() || v < 0.0)
            {
                return Err(CoreError::InvalidInput(format!(
                    "{name}: endlicher Wert ab 0 erforderlich"
                )));
            }
        }
        if self.grz.is_some_and(|v| v > 1.0) || self.grz_gesamt.is_some_and(|v| v > 1.0) {
            return Err(CoreError::InvalidInput(
                "GRZ darf 1,0 nicht überschreiten".into(),
            ));
        }
        if let (Some(i), Some(ii)) = (self.grz, self.grz_gesamt)
            && ii < i
        {
            return Err(CoreError::InvalidInput("Die zulässige GRZ einschließlich Nebenanlagen darf nicht kleiner als die GRZ der Hauptanlagen sein".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NutzungsEingabe {
    #[serde(default)]
    pub aufstellung: Option<crate::flaechen::FlaechenAufstellung>,
    pub mass: MassDerNutzung,
    #[serde(default)]
    pub vollgeschosse: Option<u32>,
    pub gebiet: Baugebiet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Quelle {
    Festsetzung,
    Orientierungswert,
    BauNvo19,
    Keine,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Nachweis {
    pub gegenstand: Option<String>,
    pub art: &'static str,
    pub ist: Option<f64>,
    pub soll: Option<f64>,
    pub quelle: Quelle,
    pub zwingend: bool,
    pub eingehalten: Option<bool>,
    /// Ist minus Soll, auch negative Abweichungen bei zwingender Geschossigkeit.
    pub abweichung: Option<f64>,
    pub ueberschreitung: Option<f64>,
    /// Bezogen auf den Sollwert, bei Soll=0 nicht definiert.
    pub ueberschreitung_prozent: Option<f64>,
    pub flaechenueberschreitung_m2: Option<f64>,
}

fn nachweis(
    art: &'static str,
    ist: Option<f64>,
    soll: Option<f64>,
    quelle: Quelle,
    zwingend: bool,
) -> Nachweis {
    let abweichung = ist.zip(soll).map(|(i, s)| i - s);
    let ueberschreitung = abweichung.map(|d| if d.abs() < 1e-9 { 0.0 } else { d.max(0.0) });
    let eingehalten = abweichung.map(|d| if zwingend { d.abs() < 1e-9 } else { d <= 1e-9 });
    let prozent = ueberschreitung
        .zip(soll)
        .and_then(|(u, s)| (s > 0.0).then(|| u / s * 100.0).filter(|v| v.is_finite()));
    Nachweis {
        gegenstand: None,
        art,
        ist,
        soll,
        quelle,
        zwingend,
        eingehalten,
        abweichung,
        ueberschreitung,
        ueberschreitung_prozent: prozent,
        flaechenueberschreitung_m2: None,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NutzungsPruefung {
    pub aufstellung: Option<crate::flaechen::FlaechenErgebnis>,
    pub projekt_version: i64,
    pub kennzahlen: Kennzahlen,
    pub nachweise: Vec<Nachweis>,
    pub bebauungsplan: bool,
}

pub fn pruefe(
    e: &NutzungsEingabe,
    p: &Planungsrecht,
    projekt_version: i64,
) -> CoreResult<NutzungsPruefung> {
    pruefe_mit_regeln(e, p, projekt_version, crate::baunvo::regelwerke())
}
pub fn pruefe_mit_regeln(
    e: &NutzungsEingabe,
    p: &Planungsrecht,
    projekt_version: i64,
    regeln: &[crate::baunvo::Regelwerk],
) -> CoreResult<NutzungsPruefung> {
    let aufstellung = crate::flaechen::berechne_aufstellung_mit_regeln(e, p, regeln)?;
    let mut e = e.clone();
    if let Some(a) = &aufstellung {
        e.mass = a.mass.clone();
        e.vollgeschosse = Some(a.vollgeschosse);
    }
    let historisch = aufstellung.as_ref().is_some_and(|a| !a.grz_ii);
    p.festgesetzte_werte.validate()?;
    let k = calc::kennzahlen(&e.mass)?;
    if [k.grz, k.grz_gesamt, k.gfz, k.bmz.unwrap_or(0.0)]
        .iter()
        .any(|v| !v.is_finite())
    {
        return Err(CoreError::InvalidInput(
            "Kennzahlen außerhalb des darstellbaren Zahlenbereichs".into(),
        ));
    }
    let mit_plan = matches!(
        p.bebauungsplan_art,
        BebauungsplanArt::Qualifiziert
            | BebauungsplanArt::Einfach
            | BebauungsplanArt::Vorhabenbezogen
    );
    let f = if mit_plan {
        p.festgesetzte_werte.clone()
    } else {
        Festsetzungen::default()
    };
    let o = f.baugebiet.unwrap_or(e.gebiet).orientierungswerte();
    // In einem Plan keine unbekannten Festsetzungen durch Orientierungswerte ersetzen.
    let (grz, gfz, bmz, quelle) =
        if !mit_plan && p.rechtsgrundlage == Some(crate::domain::Rechtsgrundlage::Par35) {
            (None, None, None, Quelle::Keine)
        } else if mit_plan {
            (f.grz, f.gfz, f.bmz, Quelle::Festsetzung)
        } else {
            (Some(o.grz), o.gfz, o.bmz, Quelle::Orientierungswert)
        };
    let basis_gesamt = match (
        grz,
        f.grundflaeche_m2
            .map(|gr| gr / e.mass.grundstuecksflaeche_m2),
    ) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) | (None, Some(a)) => Some(a),
        _ => None,
    };
    let regel = crate::baunvo::aus_katalog(
        regeln,
        p.bau_nvo
            .or(e.aufstellung.as_ref().map(|a| a.bau_nvo))
            .unwrap_or(crate::flaechen::BauNvo::V2017),
    )?;
    let grz_gesamt = if historisch {
        None
    } else {
        f.grz_gesamt.or_else(|| {
            basis_gesamt.map(|v| (v * regel.grz_ii_faktor).min(regel.grz_ii_kappung).max(v))
        })
    };
    let quelle_gesamt = if f.grz_gesamt.is_some() {
        Quelle::Festsetzung
    } else if mit_plan {
        Quelle::BauNvo19
    } else {
        Quelle::Orientierungswert
    };
    let n = |art, ist, soll, quelle, zwingend| {
        nachweis(
            art,
            ist,
            soll,
            if soll.is_some() {
                quelle
            } else {
                Quelle::Keine
            },
            zwingend,
        )
    };
    let mut r = NutzungsPruefung {
        aufstellung,
        projekt_version,
        kennzahlen: k.clone(),
        bebauungsplan: mit_plan,
        nachweise: vec![
            n("grz", Some(k.grz), grz, quelle, false),
            n(
                "grz_gesamt",
                if historisch { None } else { Some(k.grz_gesamt) },
                grz_gesamt,
                quelle_gesamt,
                false,
            ),
            n("gfz", Some(k.gfz), gfz, quelle, false),
            n("bmz", k.bmz, bmz, quelle, false),
            n(
                "grundflaeche",
                Some(e.mass.grundflaeche_m2),
                f.grundflaeche_m2,
                Quelle::Festsetzung,
                false,
            ),
            n(
                "geschossflaeche",
                Some(e.mass.geschossflaeche_m2),
                f.geschossflaeche_m2,
                Quelle::Festsetzung,
                false,
            ),
            n(
                "baufensterflaeche",
                Some(e.mass.grundflaeche_m2),
                f.ueberbaubare_flaeche_m2,
                Quelle::Festsetzung,
                false,
            ),
            n(
                "vollgeschosse",
                e.vollgeschosse.map(f64::from),
                f.vollgeschosse.map(f64::from),
                Quelle::Festsetzung,
                f.geschossigkeit == Geschossigkeit::Zwingend,
            ),
        ],
    };
    if let Some(a) = &r.aufstellung
        && a.vollgeschosse_je_gebaeude.len() > 1
    {
        r.nachweise.retain(|n| n.art != "vollgeschosse");
        for (id, anzahl) in &a.vollgeschosse_je_gebaeude {
            let mut n = nachweis(
                "vollgeschosse",
                Some(f64::from(*anzahl)),
                f.vollgeschosse.map(f64::from),
                Quelle::Festsetzung,
                f.geschossigkeit == Geschossigkeit::Zwingend,
            );
            n.gegenstand = Some(id.clone());
            r.nachweise.push(n);
        }
    }
    let fehlende_bauteile = r
        .aufstellung
        .as_ref()
        .is_some_and(|a| !a.bauteile_vorhanden);
    let fehlende_geschosse = r
        .aufstellung
        .as_ref()
        .is_some_and(|a| !a.geschosse_vorhanden);
    for n in &mut r.nachweise {
        if (fehlende_bauteile
            && matches!(
                n.art,
                "grz" | "grz_gesamt" | "grundflaeche" | "baufensterflaeche"
            ))
            || (fehlende_geschosse && matches!(n.art, "gfz" | "geschossflaeche" | "vollgeschosse"))
        {
            *n = nachweis(n.art, None, n.soll, n.quelle, n.zwingend);
        }
        if matches!(n.art, "grz" | "grz_gesamt" | "gfz") {
            n.flaechenueberschreitung_m2 = n
                .ueberschreitung
                .map(|d| d * e.mass.grundstuecksflaeche_m2)
                .filter(|v| v.is_finite());
        }
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn eingabe() -> NutzungsEingabe {
        NutzungsEingabe {
            aufstellung: None,
            mass: MassDerNutzung {
                grundstuecksflaeche_m2: 800.0,
                grundflaeche_m2: 320.0,
                grundflaeche_nebenanlagen_m2: 100.0,
                geschossflaeche_m2: 640.0,
                baumasse_m3: None,
            },
            vollgeschosse: Some(3),
            gebiet: Baugebiet::WA,
        }
    }
    fn plan(f: Festsetzungen) -> Planungsrecht {
        Planungsrecht {
            bebauungsplan_art: BebauungsplanArt::Qualifiziert,
            festgesetzte_werte: f,
            ..Default::default()
        }
    }
    #[test]
    fn grz_ii_grenze_stammt_aus_explizitem_katalog() {
        let recht = plan(Festsetzungen {
            grz: Some(0.4),
            ..Default::default()
        });
        let mut regel = crate::baunvo::regelwerk(crate::flaechen::BauNvo::V2017).clone();
        regel.grz_ii_faktor = 1.1;
        regel.grz_ii_kappung = 0.42;
        let r = pruefe_mit_regeln(&eingabe(), &recht, 1, &[regel]).unwrap();
        let grz_ii = r.nachweise.iter().find(|n| n.art == "grz_gesamt").unwrap();
        assert_eq!(grz_ii.soll, Some(0.42));
        let live = pruefe(&eingabe(), &recht, 1).unwrap();
        assert!(
            (live
                .nachweise
                .iter()
                .find(|n| n.art == "grz_gesamt")
                .unwrap()
                .soll
                .unwrap()
                - 0.6)
                .abs()
                < 1e-9
        );
    }
    #[test]
    fn festsetzungen_haben_vorrang_und_abweichungen_sind_quantifiziert() {
        let r = pruefe(
            &eingabe(),
            &plan(Festsetzungen {
                grz: Some(0.3),
                gfz: Some(0.6),
                vollgeschosse: Some(2),
                ..Default::default()
            }),
            4,
        )
        .unwrap();
        let grz = &r.nachweise[0];
        assert_eq!(grz.eingehalten, Some(false));
        assert!((grz.flaechenueberschreitung_m2.unwrap() - 80.0).abs() < 1e-8);
        assert!((grz.ueberschreitung.unwrap() - 0.1).abs() < 1e-9);
        assert!((grz.ueberschreitung_prozent.unwrap() - 100.0 / 3.0).abs() < 1e-9);
        assert_eq!(r.nachweise[7].ueberschreitung, Some(1.0));
        assert_eq!(r.projekt_version, 4);
    }
    #[test]
    fn unbekannte_planwerte_werden_nicht_als_eingehalten_ausgegeben() {
        let r = pruefe(&eingabe(), &plan(Festsetzungen::default()), 1).unwrap();
        assert_eq!(r.nachweise[0].soll, None);
        assert_eq!(r.nachweise[0].eingehalten, None);
    }
    #[test]
    fn zwingende_geschossigkeit_prueft_auch_unterschreitungen() {
        let r = pruefe(
            &eingabe(),
            &plan(Festsetzungen {
                vollgeschosse: Some(4),
                geschossigkeit: Geschossigkeit::Zwingend,
                ..Default::default()
            }),
            1,
        )
        .unwrap();
        assert_eq!(r.nachweise[7].eingehalten, Some(false));
        assert_eq!(r.nachweise[7].abweichung, Some(-1.0));
    }
    #[test]
    fn null_ist_eine_festsetzung_und_keine_fehlende_eingabe() {
        let r = pruefe(
            &eingabe(),
            &plan(Festsetzungen {
                grz: Some(0.0),
                ..Default::default()
            }),
            1,
        )
        .unwrap();
        assert_eq!(r.nachweise[0].soll, Some(0.0));
        assert_eq!(r.nachweise[0].ueberschreitung_prozent, None);
        assert_eq!(r.nachweise[0].eingehalten, Some(false));
    }
    #[test]
    fn grz_ii_kann_abweichend_festgesetzt_sein() {
        let r = pruefe(
            &eingabe(),
            &plan(Festsetzungen {
                grz: Some(0.3),
                grz_gesamt: Some(0.55),
                ..Default::default()
            }),
            1,
        )
        .unwrap();
        assert_eq!(r.nachweise[1].soll, Some(0.55));
        assert_eq!(r.nachweise[1].quelle, Quelle::Festsetzung);
    }
    #[test]
    fn ohne_plan_nur_orientierungswerte_auch_bei_alten_planangaben() {
        let p = Planungsrecht {
            festgesetzte_werte: Festsetzungen {
                grz: Some(0.1),
                ..Default::default()
            },
            ..Default::default()
        };
        let r = pruefe(&eingabe(), &p, 1).unwrap();
        assert_eq!(r.nachweise[0].soll, Some(0.4));
        assert_eq!(r.nachweise[0].quelle, Quelle::Orientierungswert);
        assert_eq!(r.nachweise[7].eingehalten, None);
    }
    #[test]
    fn absolute_grundflaeche_begrenzt_auch_nebenanlagen() {
        let r = pruefe(
            &eingabe(),
            &plan(Festsetzungen {
                grundflaeche_m2: Some(160.0),
                ..Default::default()
            }),
            1,
        )
        .unwrap();
        assert!((r.nachweise[1].soll.unwrap() - 0.3).abs() < 1e-9);
        assert_eq!(r.nachweise[4].ueberschreitung, Some(160.0));
    }

    #[test]
    fn negative_und_nicht_endliche_festsetzungen_werden_abgelehnt() {
        for v in [-0.1, f64::NAN, f64::INFINITY] {
            assert!(
                Festsetzungen {
                    gfz: Some(v),
                    ..Default::default()
                }
                .validate()
                .is_err()
            );
        }
        assert!(
            Festsetzungen {
                grz: Some(1.1),
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
    #[test]
    fn grundstueck_zuerst_flaechen_spaeter_keine_nullnachweise() {
        let mut e = eingabe();
        e.aufstellung = Some(crate::flaechen::FlaechenAufstellung {
            bau_nvo: crate::flaechen::BauNvo::V2017,
            bauteile: vec![],
            geschosse: vec![],
            nichtvollgeschoss_faktor: 0.0,
            nichtvollgeschoss_festsetzung: String::new(),
            grundstueck: None,
        });
        let r = pruefe(&e, &Planungsrecht::default(), 1).unwrap();
        for art in ["grz", "gfz", "vollgeschosse"] {
            assert!(
                r.nachweise
                    .iter()
                    .find(|n| n.art == art)
                    .unwrap()
                    .ist
                    .is_none()
            );
        }
        assert!(!r.aufstellung.unwrap().geschosse_vorhanden);
    }
    #[test]
    fn aussenbereich_erhaelt_keine_automatische_par17_vorgabe() {
        let p = Planungsrecht {
            rechtsgrundlage: Some(crate::domain::Rechtsgrundlage::Par35),
            ..Default::default()
        };
        let r = pruefe(&eingabe(), &p, 1).unwrap();
        for art in ["grz", "gfz", "bmz"] {
            assert!(
                r.nachweise
                    .iter()
                    .find(|n| n.art == art)
                    .unwrap()
                    .soll
                    .is_none()
            );
        }
    }
}
