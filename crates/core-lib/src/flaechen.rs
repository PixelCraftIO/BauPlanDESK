//! Flächenaufstellungen für §§ 19, 20 BauNVO. Sonderanrechnungen sind explizit.
use crate::{CoreError, CoreResult, calc::MassDerNutzung};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BauNvo {
    #[serde(rename = "1962")]
    V1962,
    #[serde(rename = "1968")]
    V1968,
    #[serde(rename = "1977")]
    V1977,
    #[serde(rename = "1990")]
    V1990,
    #[serde(rename = "2017")]
    V2017,
}
impl BauNvo {
    pub fn modern(self) -> bool {
        crate::baunvo::regelwerk(self).grz_ii
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bauteiltyp {
    Hauptanlage,
    TerrasseHauptanlage,
    Garage,
    Stellplatz,
    Zufahrt,
    Nebenanlage14,
    Unterirdisch,
    BalkonLoggia,
    TerrasseNebenanlage,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Anrechnung {
    Grz,
    GrzIi,
    Keine,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bauteil {
    pub bezeichnung: String,
    pub typ: Bauteiltyp,
    pub flaeche_m2: f64,
    pub abweichende_anrechnung: Option<Anrechnung>,
    pub begruendung: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Geschoss {
    pub bezeichnung: String,
    pub vollgeschoss: bool,
    pub aussenmass_m2: f64,
    pub aufenthalt_m2: f64,
    pub waende_m2: f64,
    pub erschliessung_m2: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlaechenAufstellung {
    pub bau_nvo: BauNvo,
    #[serde(default)]
    pub grundstueck: Option<GrundstueckAufstellung>,
    pub bauteile: Vec<Bauteil>,
    pub geschosse: Vec<Geschoss>,
    /// Planfestsetzung zur Anrechnung von Nichtvollgeschossen (0..1).
    pub nichtvollgeschoss_faktor: f64,
    pub nichtvollgeschoss_festsetzung: String,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FlaechenErgebnis {
    pub regelwerk_id: String,
    pub regelwerk_version: String,
    pub grundstueck: GrundstueckErgebnis,
    pub mass: MassDerNutzung,
    pub vollgeschosse: u32,
    pub grz_ii: bool,
    pub bauteile: Vec<BauteilAnsatz>,
    pub geschosse: Vec<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BauteilAnsatz {
    pub anrechnung: Anrechnung,
    pub flaeche_m2: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gemeinschaftsanteil {
    pub bezeichnung: String,
    pub gesamtflaeche_m2: f64,
    pub anteil_prozent: f64,
    pub festsetzung: String,
    pub im_plangebiet: bool,
    pub ausserhalb_baugrundstueck: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrundstueckAufstellung {
    pub basisflaeche_m2: f64,
    pub nicht_massgebend_m2: f64,
    pub abzug_begruendung: String,
    pub gemeinschaftsanteile: Vec<Gemeinschaftsanteil>,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GrundstueckErgebnis {
    pub basis_m2: f64,
    pub abzug_m2: f64,
    pub zuschlag_m2: f64,
    pub massgebend_m2: f64,
    pub anteile_m2: Vec<f64>,
}
impl GrundstueckAufstellung {
    fn berechne(&self, regel: &crate::baunvo::Regelwerk) -> CoreResult<GrundstueckErgebnis> {
        wert(self.basisflaeche_m2)?;
        wert(self.nicht_massgebend_m2)?;
        if self.basisflaeche_m2 <= 0.0 || self.nicht_massgebend_m2 > self.basisflaeche_m2 {
            return Err(CoreError::InvalidInput("Grundstücksbasis muss positiv sein; Abzug darf die Basisfläche nicht überschreiten".into()));
        }
        if self.nicht_massgebend_m2 > 0.0 && self.abzug_begruendung.trim().is_empty() {
            return Err(CoreError::InvalidInput(
                "Nicht maßgebende Grundstücksflächen benötigen eine Begründung".into(),
            ));
        }
        if self.gemeinschaftsanteile.len() > 1000
            || (!regel.gemeinschaftsanteile && !self.gemeinschaftsanteile.is_empty())
        {
            return Err(CoreError::InvalidInput("Gemeinschaftsflächen-Zurechnung für dieses Regelwerk nicht konfiguriert oder zu viele Positionen".into()));
        }
        let mut anteile = vec![];
        let mut namen = std::collections::HashSet::new();
        for a in &self.gemeinschaftsanteile {
            wert(a.gesamtflaeche_m2)?;
            wert(a.anteil_prozent)?;
            if a.anteil_prozent > 100.0
                || a.bezeichnung.trim().is_empty()
                || a.bezeichnung.len() > 500
                || a.festsetzung.trim().is_empty()
                || a.festsetzung.len() > 4000
                || !a.im_plangebiet
                || !a.ausserhalb_baugrundstueck
            {
                return Err(CoreError::InvalidInput("Gemeinschaftsanteil benötigt eine eindeutige Bezeichnung, Anteil von 0 bis 100 % und Planfestsetzung; Fläche muss im Plangebiet außerhalb des Baugrundstücks liegen".into()));
            }
            if !namen.insert(a.bezeichnung.trim().to_lowercase()) {
                return Err(CoreError::InvalidInput(
                    "Gemeinschaftsanlage wurde mehrfach erfasst".into(),
                ));
            }
            anteile.push(a.gesamtflaeche_m2 * (a.anteil_prozent / 100.0));
        }
        let zuschlag = anteile.iter().sum::<f64>();
        let massgebend = self.basisflaeche_m2 - self.nicht_massgebend_m2 + zuschlag;
        if !massgebend.is_finite() || massgebend <= 0.0 {
            return Err(CoreError::InvalidInput(
                "Maßgebende Grundstücksfläche muss endlich und positiv sein".into(),
            ));
        }
        Ok(GrundstueckErgebnis {
            basis_m2: self.basisflaeche_m2,
            abzug_m2: self.nicht_massgebend_m2,
            zuschlag_m2: zuschlag,
            massgebend_m2: massgebend,
            anteile_m2: anteile,
        })
    }
}

/// Planfestsetzungen werden zentral angewendet, auch bei HTTP-Eingaben.
pub fn berechne_aufstellung(
    e: &crate::festsetzungen::NutzungsEingabe,
    p: &crate::domain::Planungsrecht,
) -> CoreResult<Option<FlaechenErgebnis>> {
    let Some(quelle) = &e.aufstellung else {
        return Ok(None);
    };
    let mut a = quelle.clone();
    let mit_plan = matches!(
        p.bebauungsplan_art,
        crate::domain::BebauungsplanArt::Qualifiziert
            | crate::domain::BebauungsplanArt::Einfach
            | crate::domain::BebauungsplanArt::Vorhabenbezogen
    );
    if a.grundstueck
        .as_ref()
        .is_some_and(|g| !g.gemeinschaftsanteile.is_empty())
        && !mit_plan
    {
        return Err(CoreError::InvalidInput(
            "Zugeordnete Gemeinschaftsflächen benötigen einen Bebauungsplan".into(),
        ));
    }
    if mit_plan && let Some(f) = p.festgesetzte_werte.nichtvollgeschoss_faktor {
        a.nichtvollgeschoss_faktor = f;
        a.nichtvollgeschoss_festsetzung =
            p.festgesetzte_werte.nichtvollgeschoss_festsetzung.clone();
    }
    a.berechne(e.mass.grundstuecksflaeche_m2, e.mass.baumasse_m3)
        .map(Some)
}

fn wert(v: f64) -> CoreResult<()> {
    if !v.is_finite() || v < 0.0 {
        return Err(CoreError::InvalidInput(
            "Flächen müssen endlich und mindestens 0 sein".into(),
        ));
    }
    Ok(())
}
impl FlaechenAufstellung {
    pub fn berechne(
        &self,
        grundstueck: f64,
        baumasse: Option<f64>,
    ) -> CoreResult<FlaechenErgebnis> {
        let regel = crate::baunvo::regelwerk(self.bau_nvo);
        let grundstueck = match &self.grundstueck {
            Some(g) => g.berechne(regel)?,
            None => GrundstueckErgebnis {
                basis_m2: grundstueck,
                abzug_m2: 0.0,
                zuschlag_m2: 0.0,
                massgebend_m2: grundstueck,
                anteile_m2: vec![],
            },
        };
        if self.bauteile.is_empty() || self.geschosse.is_empty() {
            return Err(CoreError::InvalidInput(
                "Mindestens ein Bauteil und ein Geschoss erfassen".into(),
            ));
        }
        if self.bauteile.len() > 1000 || self.geschosse.len() > 1000 {
            return Err(CoreError::InvalidInput(
                "Höchstens 1000 Bauteile bzw. Geschosse zulässig".into(),
            ));
        }
        if !self.nichtvollgeschoss_faktor.is_finite()
            || !(0.0..=1.0).contains(&self.nichtvollgeschoss_faktor)
        {
            return Err(CoreError::InvalidInput(
                "Anrechnungsfaktor muss zwischen 0 und 1 liegen".into(),
            ));
        }
        if self.bau_nvo.modern()
            && self.nichtvollgeschoss_faktor > 0.0
            && self.nichtvollgeschoss_festsetzung.trim().is_empty()
        {
            return Err(CoreError::InvalidInput(
                "Anrechnung von Nichtvollgeschossen benötigt die Planfestsetzung".into(),
            ));
        }
        let nichtvollgeschoss_faktor = if regel.nichtvollgeschoss_planabweichung
            && !self.nichtvollgeschoss_festsetzung.trim().is_empty()
        {
            self.nichtvollgeschoss_faktor
        } else {
            regel.nichtvollgeschoss_faktor
        };
        let mut r = FlaechenErgebnis {
            regelwerk_id: regel.id.clone(),
            regelwerk_version: regel.version.clone(),
            grundstueck: grundstueck.clone(),
            mass: MassDerNutzung {
                grundstuecksflaeche_m2: grundstueck.massgebend_m2,
                grundflaeche_m2: 0.0,
                grundflaeche_nebenanlagen_m2: 0.0,
                geschossflaeche_m2: 0.0,
                baumasse_m3: baumasse,
            },
            vollgeschosse: 0,
            grz_ii: self.bau_nvo.modern(),
            bauteile: vec![],
            geschosse: vec![],
        };
        for b in &self.bauteile {
            wert(b.flaeche_m2)?;
            if b.bezeichnung.trim().is_empty()
                || b.bezeichnung.len() > 500
                || b.begruendung.len() > 4000
            {
                return Err(CoreError::InvalidInput(
                    "Bauteil benötigt eine Bezeichnung (max. 500 Zeichen)".into(),
                ));
            }
            let standard = regel.bauteile[&b.typ].anrechnung;
            let a = b.abweichende_anrechnung.unwrap_or(standard);
            if b.abweichende_anrechnung.is_some() && b.begruendung.trim().is_empty() {
                return Err(CoreError::InvalidInput(
                    "Abweichende Bauteilanrechnung benötigt eine Begründung".into(),
                ));
            }
            if !self.bau_nvo.modern() && a == Anrechnung::GrzIi {
                return Err(CoreError::InvalidInput(
                    "Diese BauNVO-Fassung kennt keine GRZ II".into(),
                ));
            }
            match a {
                Anrechnung::Grz => r.mass.grundflaeche_m2 += b.flaeche_m2,
                Anrechnung::GrzIi => r.mass.grundflaeche_nebenanlagen_m2 += b.flaeche_m2,
                Anrechnung::Keine => {}
            }
            r.bauteile.push(BauteilAnsatz {
                anrechnung: a,
                flaeche_m2: if a == Anrechnung::Keine {
                    0.0
                } else {
                    b.flaeche_m2
                },
            });
        }
        for g in &self.geschosse {
            if g.bezeichnung.trim().is_empty() || g.bezeichnung.len() > 500 {
                return Err(CoreError::InvalidInput(
                    "Geschoss benötigt eine Bezeichnung (max. 500 Zeichen)".into(),
                ));
            }
            for v in [
                g.aussenmass_m2,
                g.aufenthalt_m2,
                g.waende_m2,
                g.erschliessung_m2,
            ] {
                wert(v)?;
            }
            let teil = g.aufenthalt_m2 + g.waende_m2 + g.erschliessung_m2;
            if !g.vollgeschoss
                && nichtvollgeschoss_faktor > 0.0
                && (!teil.is_finite() || teil > g.aussenmass_m2 + 1e-9)
            {
                return Err(CoreError::InvalidInput("Aufenthaltsräume mit Wänden und Erschließung dürfen die Geschossfläche nach Außenmaßen nicht überschreiten".into()));
            }
            let gf = if g.vollgeschoss {
                r.vollgeschosse += 1;
                g.aussenmass_m2
            } else {
                teil * nichtvollgeschoss_faktor
            };
            r.mass.geschossflaeche_m2 += gf;
            r.geschosse.push(gf);
        }
        let k = crate::calc::kennzahlen(&r.mass)?;
        if [k.grz, k.grz_gesamt, k.gfz, k.bmz.unwrap_or(0.0)]
            .iter()
            .any(|v| !v.is_finite())
        {
            return Err(CoreError::InvalidInput(
                "Flächensummen außerhalb des Zahlenbereichs".into(),
            ));
        }
        Ok(r)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn aufstellung(v: BauNvo) -> FlaechenAufstellung {
        FlaechenAufstellung {
            bau_nvo: v,
            grundstueck: None,
            bauteile: [
                (Bauteiltyp::Hauptanlage, 100.0),
                (Bauteiltyp::Garage, 20.0),
                (Bauteiltyp::Nebenanlage14, 10.0),
                (Bauteiltyp::Zufahrt, 30.0),
                (Bauteiltyp::TerrasseHauptanlage, 15.0),
            ]
            .into_iter()
            .map(|(typ, flaeche_m2)| Bauteil {
                bezeichnung: "Teil".into(),
                typ,
                flaeche_m2,
                abweichende_anrechnung: None,
                begruendung: String::new(),
            })
            .collect(),
            geschosse: vec![
                Geschoss {
                    bezeichnung: "EG".into(),
                    vollgeschoss: true,
                    aussenmass_m2: 100.0,
                    aufenthalt_m2: 0.0,
                    waende_m2: 0.0,
                    erschliessung_m2: 0.0,
                },
                Geschoss {
                    bezeichnung: "DG".into(),
                    vollgeschoss: false,
                    aussenmass_m2: 80.0,
                    aufenthalt_m2: 30.0,
                    waende_m2: 10.0,
                    erschliessung_m2: 5.0,
                },
            ],
            nichtvollgeschoss_faktor: 0.0,
            nichtvollgeschoss_festsetzung: String::new(),
        }
    }
    #[test]
    fn gemeinschaftsflaechen_erhoehen_bezugsflaeche_ohne_doppelte_zurechnung() {
        let mut a = aufstellung(BauNvo::V2017);
        a.grundstueck = Some(GrundstueckAufstellung {
            basisflaeche_m2: 500.0,
            nicht_massgebend_m2: 50.0,
            abzug_begruendung: "Nicht im Bauland".into(),
            gemeinschaftsanteile: vec![Gemeinschaftsanteil {
                bezeichnung: "Spielplatz Flurstück 2".into(),
                gesamtflaeche_m2: 400.0,
                anteil_prozent: 25.0,
                festsetzung: "Textfestsetzung 5: Anteil 1/4".into(),
                im_plangebiet: true,
                ausserhalb_baugrundstueck: true,
            }],
        });
        let r = a.berechne(500.0, None).unwrap();
        assert_eq!(r.grundstueck.massgebend_m2, 550.0);
        assert_eq!(r.grundstueck.anteile_m2, vec![100.0]);
        let k = crate::calc::kennzahlen(&r.mass).unwrap();
        assert!((k.grz - 115.0 / 550.0).abs() < 1e-10);
        assert!((k.gfz - 100.0 / 550.0).abs() < 1e-10);
        // Bereits berechnete Summen (z. B. nach Speichern) erhöhen die Basis nicht erneut.
        assert_eq!(
            a.berechne(r.mass.grundstuecksflaeche_m2, None)
                .unwrap()
                .grundstueck
                .massgebend_m2,
            550.0
        );
    }
    #[test]
    fn gemeinschaftsanteile_brauchen_gueltige_zuordnung() {
        let mut g = GrundstueckAufstellung {
            basisflaeche_m2: 500.0,
            nicht_massgebend_m2: 0.0,
            abzug_begruendung: String::new(),
            gemeinschaftsanteile: vec![Gemeinschaftsanteil {
                bezeichnung: "Spielplatz".into(),
                gesamtflaeche_m2: 400.0,
                anteil_prozent: 25.0,
                festsetzung: "TF 5".into(),
                im_plangebiet: true,
                ausserhalb_baugrundstueck: true,
            }],
        };
        let regel = crate::baunvo::regelwerk(BauNvo::V2017);
        assert!(g.berechne(regel).is_ok());
        g.gemeinschaftsanteile[0].anteil_prozent = 101.0;
        assert!(g.berechne(regel).is_err());
        g.gemeinschaftsanteile[0].anteil_prozent = 25.0;
        g.gemeinschaftsanteile[0].im_plangebiet = false;
        assert!(g.berechne(regel).is_err());
        g.gemeinschaftsanteile[0].im_plangebiet = true;
        g.gemeinschaftsanteile[0].ausserhalb_baugrundstueck = false;
        assert!(g.berechne(regel).is_err());
        g.gemeinschaftsanteile[0].ausserhalb_baugrundstueck = true;
        let doppelt = g.gemeinschaftsanteile[0].clone();
        g.gemeinschaftsanteile.push(doppelt);
        assert!(g.berechne(regel).is_err());
        g.gemeinschaftsanteile.pop();
        g.gemeinschaftsanteile[0].festsetzung.clear();
        assert!(g.berechne(regel).is_err());
    }
    #[test]
    fn alle_fuenf_fassungen() {
        for v in [
            BauNvo::V1962,
            BauNvo::V1968,
            BauNvo::V1977,
            BauNvo::V1990,
            BauNvo::V2017,
        ] {
            let r = aufstellung(v).berechne(500.0, None).unwrap();
            assert_eq!(
                r.mass.grundflaeche_m2,
                if v.modern() { 115.0 } else { 120.0 }
            );
            assert_eq!(
                r.mass.grundflaeche_nebenanlagen_m2,
                if v.modern() { 60.0 } else { 0.0 }
            );
            assert_eq!(
                r.mass.geschossflaeche_m2,
                if v.modern() { 100.0 } else { 145.0 }
            );
            assert_eq!(r.vollgeschosse, 1);
        }
    }
    #[test]
    fn planfestsetzung_teilanrechnung() {
        let mut a = aufstellung(BauNvo::V1990);
        a.nichtvollgeschoss_faktor = 0.5;
        assert!(a.berechne(500.0, None).is_err());
        a.nichtvollgeschoss_festsetzung = "Textfestsetzung 3".into();
        assert_eq!(
            a.berechne(500.0, None).unwrap().mass.geschossflaeche_m2,
            122.5
        );
    }
    #[test]
    fn sonderansatz_nur_mit_begruendung() {
        let mut a = aufstellung(BauNvo::V1968);
        a.bauteile[1].abweichende_anrechnung = Some(Anrechnung::Keine);
        assert!(a.berechne(500.0, None).is_err());
        a.bauteile[1].begruendung = "§ 21a Abs. 3: konkret geprüfter Nichtanrechnungsumfang".into();
        assert_eq!(a.berechne(500.0, None).unwrap().mass.grundflaeche_m2, 100.0);
    }
    #[test]
    fn ungueltige_und_ueberlaufende_flaechen() {
        let mut a = aufstellung(BauNvo::V1977);
        a.geschosse[1].aufenthalt_m2 = 90.0;
        assert!(a.berechne(500.0, None).is_err());
        a.geschosse[1].aufenthalt_m2 = 30.0;
        a.bauteile[0].flaeche_m2 = f64::MAX;
        a.bauteile[1].flaeche_m2 = f64::MAX;
        assert!(a.berechne(1.0, None).is_err());
    }
}
