//! Flächenaufstellungen für §§ 19, 20 BauNVO. Sonderanrechnungen sind explizit.
use crate::{CoreError, CoreResult, calc::MassDerNutzung};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
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
#[derive(ts_rs::TS)]
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
#[derive(ts_rs::TS)]
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
    #[serde(default)]
    pub gebaeude_id: Option<String>,
    #[serde(default)]
    pub geschoss_id: Option<String>,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlaechenErgebnis {
    pub regelwerk_id: String,
    pub regelwerk_version: String,
    pub grundstueck: GrundstueckErgebnis,
    pub mass: MassDerNutzung,
    pub vollgeschosse: u32,
    pub vollgeschosse_je_gebaeude: std::collections::BTreeMap<String, u32>,
    pub grz_ii: bool,
    pub bauteile_vorhanden: bool,
    pub geschosse_vorhanden: bool,
    pub bauteile: Vec<BauteilAnsatz>,
    pub geschosse: Vec<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrundstueckErgebnis {
    pub basis_m2: f64,
    pub abzug_m2: f64,
    pub zuschlag_m2: f64,
    pub massgebend_m2: f64,
    pub anteile_m2: Vec<f64>,
}
impl GrundstueckAufstellung {
    pub(crate) fn berechne(
        &self,
        regel: &crate::baunvo::Regelwerk,
    ) -> CoreResult<GrundstueckErgebnis> {
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
    berechne_aufstellung_mit_regeln(e, p, crate::baunvo::regelwerke())
}
pub(crate) fn berechne_aufstellung_mit_regeln(
    e: &crate::festsetzungen::NutzungsEingabe,
    p: &crate::domain::Planungsrecht,
    regeln: &[crate::baunvo::Regelwerk],
) -> CoreResult<Option<FlaechenErgebnis>> {
    let Some(quelle) = &e.aufstellung else {
        return Ok(None);
    };
    let mut a = quelle.clone();
    if let Some(fassung) = p.bau_nvo {
        a.bau_nvo = fassung;
    }
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
    a.berechne_mit_regeln(e.mass.grundstuecksflaeche_m2, e.mass.baumasse_m3, regeln)
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
        self.berechne_mit_regeln(grundstueck, baumasse, crate::baunvo::regelwerke())
    }
    pub fn berechne_mit_regeln(
        &self,
        grundstueck: f64,
        baumasse: Option<f64>,
        regeln: &[crate::baunvo::Regelwerk],
    ) -> CoreResult<FlaechenErgebnis> {
        let regel = crate::baunvo::aus_katalog(regeln, self.bau_nvo)?;
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
        if regel.grz_ii
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
            vollgeschosse_je_gebaeude: Default::default(),
            grz_ii: regel.grz_ii,
            bauteile_vorhanden: !self.bauteile.is_empty(),
            geschosse_vorhanden: !self.geschosse.is_empty(),
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
            if !regel.grz_ii && a == Anrechnung::GrzIi {
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
        if self.geschosse.iter().any(|g| g.gebaeude_id.is_some())
            && self.geschosse.iter().any(|g| g.gebaeude_id.is_none())
        {
            return Err(CoreError::InvalidInput(
                "Bei mehreren Gebäuden alle Geschosse einem Gebäude zuordnen".into(),
            ));
        }
        let mut geschoss_ids = std::collections::HashSet::new();
        for g in &self.geschosse {
            if let Some(id) = &g.geschoss_id
                && (g.gebaeude_id.is_none() || !geschoss_ids.insert((g.gebaeude_id.as_deref(), id)))
            {
                return Err(CoreError::InvalidInput(
                    "Ein zugeordnetes Geschoss darf je Baugrundstück nur einmal erfasst werden"
                        .into(),
                ));
            }
            let gebaeude = g.gebaeude_id.as_deref().unwrap_or("Bestandsgebäude");
            if gebaeude.trim().is_empty() || gebaeude.len() > 120 {
                return Err(CoreError::InvalidInput(
                    "Gültige Gebäudezuordnung erforderlich".into(),
                ));
            }
            let anzahl = r
                .vollgeschosse_je_gebaeude
                .entry(gebaeude.into())
                .or_default();
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
                *anzahl += 1;
                g.aussenmass_m2
            } else {
                teil * nichtvollgeschoss_faktor
            };
            r.mass.geschossflaeche_m2 += gf;
            r.geschosse.push(gf);
        }
        r.vollgeschosse = r
            .vollgeschosse_je_gebaeude
            .values()
            .copied()
            .max()
            .unwrap_or(0);
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
                    gebaeude_id: None,
                    geschoss_id: None,
                    bezeichnung: "EG".into(),
                    vollgeschoss: true,
                    aussenmass_m2: 100.0,
                    aufenthalt_m2: 0.0,
                    waende_m2: 0.0,
                    erschliessung_m2: 0.0,
                },
                Geschoss {
                    gebaeude_id: None,
                    geschoss_id: None,
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
    fn expliziter_katalog_steuert_grz_und_nichtvollgeschosse_ohne_live_regeln() {
        let a = aufstellung(BauNvo::V2017);
        let aktuell = a.berechne(500., None).unwrap();
        let mut historisch = crate::baunvo::regelwerk(BauNvo::V2017).clone();
        historisch.version = "archiv-1".into();
        historisch
            .bauteile
            .get_mut(&Bauteiltyp::Garage)
            .unwrap()
            .anrechnung = Anrechnung::Keine;
        historisch.nichtvollgeschoss_faktor = 1.;
        let r = a
            .berechne_mit_regeln(500., None, &[historisch.clone()])
            .unwrap();
        assert_eq!(r.regelwerk_version, "archiv-1");
        assert_eq!(
            r.mass.grundflaeche_nebenanlagen_m2,
            aktuell.mass.grundflaeche_nebenanlagen_m2 - 20.
        );
        assert_eq!(
            r.mass.geschossflaeche_m2,
            aktuell.mass.geschossflaeche_m2 + 45.
        );
        assert!(a.berechne_mit_regeln(500., None, &[]).is_err());
        assert!(
            a.berechne_mit_regeln(500., None, &[historisch.clone(), historisch.clone()])
                .is_err()
        );
        historisch.bauteile.remove(&Bauteiltyp::Garage);
        assert!(a.berechne_mit_regeln(500., None, &[historisch]).is_err());
    }
    #[test]
    fn geschossigkeit_wird_nicht_ueber_mehrere_gebaeude_addiert() {
        let mut a = aufstellung(BauNvo::V2017);
        let vorlage = a.geschosse[0].clone();
        a.geschosse.clear();
        for gebaeude in ["Haus A", "Haus B"] {
            for etage in ["EG", "OG"] {
                a.geschosse.push(Geschoss {
                    gebaeude_id: Some(gebaeude.into()),
                    geschoss_id: Some(format!("{gebaeude}/{etage}")),
                    bezeichnung: etage.into(),
                    ..vorlage.clone()
                });
            }
        }
        let r = a.berechne(500., None).unwrap();
        assert_eq!(r.vollgeschosse, 2);
        assert_eq!(r.vollgeschosse_je_gebaeude.len(), 2);
        assert_eq!(r.mass.geschossflaeche_m2, 400.);
        let mut doppelt = a.clone();
        doppelt.geschosse.push(doppelt.geschosse[0].clone());
        assert!(doppelt.berechne(500., None).is_err());
        // Teilweise Gebäudezuordnung verschleiert keine weitere Einheit.
        a.geschosse[0].gebaeude_id = None;
        assert!(a.berechne(500., None).is_err());
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
    #[test]
    fn zentrale_baunvo_ueberschreibt_die_lokale_fassung() {
        let e = crate::festsetzungen::NutzungsEingabe {
            gebiet: crate::calc::Baugebiet::WA,
            mass: crate::calc::MassDerNutzung {
                grundstuecksflaeche_m2: 1000.0,
                grundflaeche_m2: 0.0,
                grundflaeche_nebenanlagen_m2: 0.0,
                geschossflaeche_m2: 0.0,
                baumasse_m3: None,
            },
            vollgeschosse: None,
            aufstellung: Some(FlaechenAufstellung {
                bau_nvo: BauNvo::V2017,
                bauteile: vec![],
                geschosse: vec![],
                nichtvollgeschoss_faktor: 0.0,
                nichtvollgeschoss_festsetzung: String::new(),
                grundstueck: None,
            }),
        };
        let p = crate::domain::Planungsrecht {
            bau_nvo: Some(BauNvo::V1962),
            ..Default::default()
        };
        let r = berechne_aufstellung(&e, &p).unwrap().unwrap();
        assert!(!r.grz_ii);
        assert_eq!(r.regelwerk_id, crate::baunvo::regelwerk(BauNvo::V1962).id);
    }
}
