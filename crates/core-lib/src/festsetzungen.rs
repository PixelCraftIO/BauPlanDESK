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

/// Vom Planungsbüro erfasste Vergleichswerte der näheren Umgebung (§ 34 Abs. 1 BauGB).
/// Sie dienen nur der Orientierung: Ob sich das Vorhaben einfügt, bleibt eine fachliche
/// Beurteilung. Die Prüfung meldet daher Abweichungen, aber kein „eingehalten“.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Umgebungsreferenz {
    pub grz: Option<f64>,
    pub gfz: Option<f64>,
    pub vollgeschosse: Option<u32>,
    /// Referenzbebauung und Quelle, z. B. „Musterweg 3–9, Lageplan vom 02.03.2026“.
    pub grundlage: String,
}

impl Umgebungsreferenz {
    pub fn validate(&self) -> CoreResult<()> {
        for (name, wert) in [("GRZ", self.grz), ("GFZ", self.gfz)] {
            if let Some(v) = wert
                && (!v.is_finite() || v < 0.0)
            {
                return Err(CoreError::InvalidInput(format!(
                    "Umgebung {name}: endlicher Wert ab 0 erforderlich"
                )));
            }
        }
        if self.grz.is_some_and(|v| v > 1.0) {
            return Err(CoreError::InvalidInput(
                "Umgebung GRZ darf 1,0 nicht überschreiten".into(),
            ));
        }
        if self.grundlage.chars().count() > 4000 {
            return Err(CoreError::InvalidInput(
                "Grundlage der Umgebungswerte: höchstens 4000 Zeichen".into(),
            ));
        }
        if (self.grz.is_some() || self.gfz.is_some() || self.vollgeschosse.is_some())
            && self.grundlage.trim().is_empty()
        {
            return Err(CoreError::InvalidInput(
                "Umgebungswerte benötigen die Angabe der Referenzbebauung".into(),
            ));
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
    /// § 17 BauNVO richtet sich an die planende Gemeinde; reine Information ohne Urteil.
    Orientierungswert,
    BauNvo19,
    /// Vergleichswerte der näheren Umgebung (§ 34 BauGB); Orientierung ohne Urteil.
    Umgebung,
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
    // Nur Festsetzungen und § 19 BauNVO sind Zulässigkeitsmaßstäbe; Orientierungs- und
    // Umgebungswerte zeigen die Abweichung, ohne sie als Verstoß zu werten.
    let eingehalten = abweichung
        .filter(|_| !matches!(quelle, Quelle::Orientierungswert | Quelle::Umgebung))
        .map(|d| if zwingend { d.abs() < 1e-9 } else { d <= 1e-9 });
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
    /// Reihenfolge fest: grz, grz_gesamt, gfz, bmz, grundflaeche,
    /// geschossflaeche, baufensterflaeche (nur Information, nie ein Urteil),
    /// vollgeschosse (bei mehreren Gebäuden je Gebäude am Ende),
    /// baufenster_lage (Urteil aus dem Umrissvergleich, sonst „nicht geprüft“).
    pub nachweise: Vec<Nachweis>,
    pub bebauungsplan: bool,
    /// Fachliche Hinweise, z. B. fehlende BauNVO-Fassung des Plans.
    pub hinweise: Vec<String>,
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
    let mit_plan = p.mit_bebauungsplan();
    let f = if mit_plan {
        p.festgesetzte_werte.clone()
    } else {
        Festsetzungen::default()
    };
    let o = f.baugebiet.unwrap_or(e.gebiet).orientierungswerte();
    let innenbereich =
        !mit_plan && p.rechtsgrundlage == Some(crate::domain::Rechtsgrundlage::Par34);
    // Einfacher Bebauungsplan mit ergänzendem § 34 BauGB (§ 30 Abs. 3): Nicht
    // festgesetzte Maße richten sich nach der näheren Umgebung.
    let ergaenzend_innen = p.bebauungsplan_art == BebauungsplanArt::Einfach
        && p.ergaenzende_grundlage == Some(crate::domain::Rechtsgrundlage::Par34);
    let umgebung = p.umgebung.clone().unwrap_or_default();
    // In einem Plan keine unbekannten Festsetzungen durch Orientierungswerte ersetzen.
    // Im Innenbereich ist das Einfügen maßgebend, nicht § 17 BauNVO.
    let massstab = |fest: Option<f64>, umg: Option<f64>, orientierung: Option<f64>| {
        if !mit_plan && p.rechtsgrundlage == Some(crate::domain::Rechtsgrundlage::Par35) {
            (None, Quelle::Keine)
        } else if mit_plan {
            match (fest, umg) {
                (Some(v), _) => (Some(v), Quelle::Festsetzung),
                (None, Some(v)) if ergaenzend_innen => (Some(v), Quelle::Umgebung),
                _ => (None, Quelle::Festsetzung),
            }
        } else if innenbereich {
            (umg, Quelle::Umgebung)
        } else {
            (orientierung, Quelle::Orientierungswert)
        }
    };
    let (grz, quelle_grz) = massstab(f.grz, umgebung.grz, Some(o.grz));
    let (gfz, quelle_gfz) = massstab(f.gfz, umgebung.gfz, o.gfz);
    let (bmz, quelle_bmz) = massstab(f.bmz, None, o.bmz);
    let umgebung_vollgeschosse = innenbereich
        || (f.vollgeschosse.is_none() && ergaenzend_innen && umgebung.vollgeschosse.is_some());
    let (vollgeschosse_soll, vollgeschosse_quelle) = if umgebung_vollgeschosse {
        (umgebung.vollgeschosse, Quelle::Umgebung)
    } else {
        (f.vollgeschosse, Quelle::Festsetzung)
    };
    let vollgeschosse_zwingend =
        vollgeschosse_quelle == Quelle::Festsetzung && f.geschossigkeit == Geschossigkeit::Zwingend;
    // § 19 Abs. 4 BauNVO knüpft an die festgesetzte Grundfläche an, nicht an Umgebungswerte.
    let basis_gesamt = match (
        f.grz,
        f.grundflaeche_m2
            .map(|gr| gr / e.mass.grundstuecksflaeche_m2),
    ) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) | (None, Some(a)) => Some(a),
        _ => None,
    };
    let mut hinweise = Vec::new();
    // Bei einem Bebauungsplan gilt die Fassung des Plans; ohne sie kein Urteil.
    let fassung_fehlt = mit_plan && p.bau_nvo.is_none();
    if fassung_fehlt {
        hinweise.push(match &e.aufstellung {
            Some(a) => format!(
                "BauNVO-Fassung des Bebauungsplans erfassen. Bis dahin keine GRZ-II-Bewertung und kein Urteil zu den Flächenmaßen; Flächen vorläufig nach BauNVO {} angerechnet.",
                a.bau_nvo.jahr()
            ),
            None => "BauNVO-Fassung des Bebauungsplans erfassen. Bis dahin keine GRZ-II-Bewertung und kein Urteil zu den Flächenmaßen.".into(),
        });
    }
    let regel = crate::baunvo::aus_katalog(
        regeln,
        p.massgebende_bau_nvo(e.aufstellung.as_ref().map(|a| a.bau_nvo))
            .or(e.aufstellung.as_ref().map(|a| a.bau_nvo))
            .unwrap_or(crate::flaechen::BauNvo::AKTUELL),
    )?;
    let historisch =
        fassung_fehlt || !regel.grz_ii || aufstellung.as_ref().is_some_and(|a| !a.grz_ii);
    // § 19 Abs. 4 BauNVO begrenzt die zulässige Grundfläche eines Plans; ohne Plan gibt es keine GRZ-II-Grenze.
    let grz_gesamt = if historisch || !mit_plan {
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
    // § 23 BauNVO betrifft die Lage im Baufenster. Die Fläche des Baufensters
    // ist nur eine Information; das Urteil liefert der Umrissvergleich.
    let mut baufensterflaeche = n(
        "baufensterflaeche",
        Some(e.mass.grundflaeche_m2),
        f.ueberbaubare_flaeche_m2,
        Quelle::Festsetzung,
        false,
    );
    baufensterflaeche.eingehalten = None;
    let lage = match (&p.baugrenzen, mit_plan) {
        (Some(g), true) => match crate::cad::baugrenzen::pruefe(g) {
            Ok(b) => {
                let mut l = nachweis(
                    "baufenster_lage",
                    Some(b.ausserhalb_m2),
                    Some(0.0),
                    Quelle::Festsetzung,
                    false,
                );
                l.eingehalten = Some(b.eingehalten);
                l.gegenstand = Some("Vorhaben".into());
                l.flaechenueberschreitung_m2 = Some(b.ausserhalb_m2);
                l
            }
            Err(err) => {
                hinweise.push(format!("Lage im Baufenster nicht geprüft: {err}"));
                nachweis("baufenster_lage", None, None, Quelle::Keine, false)
            }
        },
        _ => nachweis("baufenster_lage", None, None, Quelle::Keine, false),
    };
    let mut r = NutzungsPruefung {
        aufstellung,
        projekt_version,
        kennzahlen: k.clone(),
        bebauungsplan: mit_plan,
        hinweise,
        nachweise: vec![
            n("grz", Some(k.grz), grz, quelle_grz, false),
            n(
                "grz_gesamt",
                if historisch { None } else { Some(k.grz_gesamt) },
                grz_gesamt,
                quelle_gesamt,
                false,
            ),
            n("gfz", Some(k.gfz), gfz, quelle_gfz, false),
            n("bmz", k.bmz, bmz, quelle_bmz, false),
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
            baufensterflaeche,
            n(
                "vollgeschosse",
                e.vollgeschosse.map(f64::from),
                vollgeschosse_soll.map(f64::from),
                vollgeschosse_quelle,
                vollgeschosse_zwingend,
            ),
            lage,
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
                vollgeschosse_soll.map(f64::from),
                if vollgeschosse_soll.is_some() {
                    vollgeschosse_quelle
                } else {
                    Quelle::Keine
                },
                vollgeschosse_zwingend,
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
        if fassung_fehlt
            && matches!(
                n.art,
                "grz" | "gfz" | "bmz" | "grundflaeche" | "geschossflaeche"
            )
        {
            n.eingehalten = None;
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
    #[test]
    fn alte_baunvo_ohne_aufstellung_erzeugt_keinen_grz_ii_vergleich() {
        use crate::flaechen::BauNvo;
        for fassung in [BauNvo::V1962, BauNvo::V1968, BauNvo::V1977] {
            let mut recht = plan(Festsetzungen {
                grz: Some(0.4),
                ..Default::default()
            });
            recht.bau_nvo = Some(fassung);
            let r = pruefe(&eingabe(), &recht, 1).unwrap();
            let grz = r.nachweise.iter().find(|n| n.art == "grz").unwrap();
            assert_eq!(grz.eingehalten, Some(true));
            let ii = r.nachweise.iter().find(|n| n.art == "grz_gesamt").unwrap();
            assert_eq!((ii.ist, ii.soll, ii.eingehalten), (None, None, None));
            assert_eq!(ii.quelle, Quelle::Keine);
        }
        for fassung in [BauNvo::V1990, BauNvo::V2017] {
            let mut recht = plan(Festsetzungen {
                grz: Some(0.4),
                ..Default::default()
            });
            recht.bau_nvo = Some(fassung);
            let r = pruefe(&eingabe(), &recht, 1).unwrap();
            let ii = r.nachweise.iter().find(|n| n.art == "grz_gesamt").unwrap();
            assert_eq!(ii.ist, Some(0.525));
            assert_eq!(ii.eingehalten, Some(true));
        }
    }
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
            rechtsgrundlage: Some(crate::domain::Rechtsgrundlage::Par30),
            bau_nvo: Some(crate::flaechen::BauNvo::V2017),
            festgesetzte_werte: f,
            ..Default::default()
        }
    }
    fn art<'a>(r: &'a NutzungsPruefung, art: &str) -> &'a Nachweis {
        r.nachweise.iter().find(|n| n.art == art).unwrap()
    }
    fn rechteck(x: f64, y: f64, w: f64, h: f64) -> Vec<crate::cad::Punkt> {
        use crate::cad::Punkt;
        vec![
            Punkt(x, y),
            Punkt(x + w, y),
            Punkt(x + w, y + h),
            Punkt(x, y + h),
        ]
    }
    #[test]
    fn plan_ohne_baunvo_fassung_ohne_urteil_und_ohne_rueckfall_auf_die_aufstellung() {
        let mut recht = plan(Festsetzungen {
            grz: Some(0.3),
            gfz: Some(0.6),
            vollgeschosse: Some(2),
            ..Default::default()
        });
        recht.bau_nvo = None;
        let mut e = eingabe();
        e.aufstellung = Some(crate::flaechen::FlaechenAufstellung {
            bau_nvo: crate::flaechen::BauNvo::V2017,
            bauteile: vec![],
            geschosse: vec![],
            nichtvollgeschoss_faktor: 0.0,
            nichtvollgeschoss_festsetzung: String::new(),
            grundstueck: None,
        });
        for e in [eingabe(), e] {
            let r = pruefe(&e, &recht, 1).unwrap();
            let ii = art(&r, "grz_gesamt");
            assert_eq!((ii.ist, ii.soll, ii.eingehalten), (None, None, None));
            for a in ["grz", "gfz", "bmz", "grundflaeche", "geschossflaeche"] {
                assert_eq!(art(&r, a).eingehalten, None, "{a}");
            }
            // Ist und Soll bleiben als Information sichtbar.
            assert_eq!(art(&r, "grz").soll, Some(0.3));
            assert!(r.hinweise[0].contains("BauNVO-Fassung des Bebauungsplans erfassen"));
        }
        // Mit Fassung wie bisher.
        recht.bau_nvo = Some(crate::flaechen::BauNvo::V2017);
        let r = pruefe(&eingabe(), &recht, 1).unwrap();
        assert_eq!(art(&r, "grz").eingehalten, Some(false));
        assert!((art(&r, "grz_gesamt").soll.unwrap() - 0.45).abs() < 1e-9);
        assert!(r.hinweise.is_empty());
        // Ohne Plan unverändert: keine Pflicht zur Fassung.
        let r = pruefe(&eingabe(), &Planungsrecht::default(), 1).unwrap();
        assert!(r.hinweise.is_empty());
    }
    #[test]
    fn baufensterflaeche_nur_information_lage_aus_baugrenzen() {
        let mut recht = plan(Festsetzungen {
            ueberbaubare_flaeche_m2: Some(400.0),
            ..Default::default()
        });
        let r = pruefe(&eingabe(), &recht, 1).unwrap();
        let fl = art(&r, "baufensterflaeche");
        assert_eq!(
            (fl.ist, fl.soll, fl.eingehalten),
            (Some(320.0), Some(400.0), None)
        );
        let lage = art(&r, "baufenster_lage");
        assert_eq!(
            (lage.soll, lage.eingehalten, lage.quelle),
            (None, None, Quelle::Keine)
        );
        recht.baugrenzen = Some(crate::cad::baugrenzen::BaugrenzenGeometrie {
            dokument_id: crate::domain::DocumentId::new(),
            dokument_version: 1,
            einheit: crate::cad::Laengeneinheit::Meter,
            baufenster: rechteck(0., 0., 30., 30.),
            vorhaben: rechteck(28., 0., 4., 10.),
        });
        let r = pruefe(&eingabe(), &recht, 1).unwrap();
        let lage = art(&r, "baufenster_lage");
        assert_eq!(lage.eingehalten, Some(false));
        assert!((lage.ist.unwrap() - 20.0).abs() < 1e-8);
        assert_eq!(lage.quelle, Quelle::Festsetzung);
        // Die große Baufensterfläche ergibt trotzdem kein „eingehalten“.
        assert_eq!(art(&r, "baufensterflaeche").eingehalten, None);
        recht.baugrenzen.as_mut().unwrap().vorhaben = rechteck(1., 1., 10., 10.);
        assert_eq!(
            art(&pruefe(&eingabe(), &recht, 1).unwrap(), "baufenster_lage").eingehalten,
            Some(true)
        );
        // Ohne Plan keine Baugrenze als Maßstab.
        recht.bebauungsplan_art = BebauungsplanArt::Kein;
        recht.rechtsgrundlage = Some(crate::domain::Rechtsgrundlage::Par34);
        assert_eq!(
            art(&pruefe(&eingabe(), &recht, 1).unwrap(), "baufenster_lage").eingehalten,
            None
        );
    }
    #[test]
    fn einfacher_plan_mit_ergaenzendem_par34_nutzt_umgebung_fuer_nicht_festgesetzte_masse() {
        let mut recht = plan(Festsetzungen {
            gfz: Some(0.6),
            ..Default::default()
        });
        recht.bebauungsplan_art = BebauungsplanArt::Einfach;
        recht.umgebung = Some(Umgebungsreferenz {
            grz: Some(0.25),
            gfz: Some(1.5),
            vollgeschosse: Some(2),
            grundlage: "Musterweg 3–9".into(),
        });
        // Ohne ergänzende Grundlage keine Umgebungswerte.
        let r = pruefe(&eingabe(), &recht, 1).unwrap();
        assert_eq!(art(&r, "grz").soll, None);
        recht.ergaenzende_grundlage = Some(crate::domain::Rechtsgrundlage::Par34);
        let r = pruefe(&eingabe(), &recht, 1).unwrap();
        let grz = art(&r, "grz");
        assert_eq!(
            (grz.soll, grz.quelle, grz.eingehalten),
            (Some(0.25), Quelle::Umgebung, None)
        );
        // Festsetzung geht der Umgebung vor.
        let gfz = art(&r, "gfz");
        assert_eq!(
            (gfz.soll, gfz.quelle, gfz.eingehalten),
            (Some(0.6), Quelle::Festsetzung, Some(false))
        );
        let vg = art(&r, "vollgeschosse");
        assert_eq!(
            (vg.soll, vg.quelle, vg.eingehalten),
            (Some(2.0), Quelle::Umgebung, None)
        );
        // Keine GRZ-II-Grenze aus Umgebungswerten.
        assert_eq!(art(&r, "grz_gesamt").soll, None);
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
        // § 17 BauNVO ist kein Zulässigkeitsmaßstab: kein Urteil, auch nicht „eingehalten“.
        assert!(r.nachweise.iter().all(|n| n.eingehalten.is_none()));
        assert!(r.nachweise[1].soll.is_none());
    }
    #[test]
    fn innenbereich_vergleicht_nur_mit_erfasster_umgebung_ohne_urteil() {
        let mut p = Planungsrecht {
            rechtsgrundlage: Some(crate::domain::Rechtsgrundlage::Par34),
            bebauungsplan_art: BebauungsplanArt::Kein,
            ..Default::default()
        };
        let r = pruefe(&eingabe(), &p, 1).unwrap();
        for art in ["grz", "grz_gesamt", "gfz", "bmz", "vollgeschosse"] {
            let n = r.nachweise.iter().find(|n| n.art == art).unwrap();
            assert_eq!(n.soll, None, "{art}");
            assert_eq!(n.eingehalten, None, "{art}");
            assert_eq!(n.quelle, Quelle::Keine, "{art}");
        }
        p.umgebung = Some(Umgebungsreferenz {
            grz: Some(0.25),
            gfz: Some(0.5),
            vollgeschosse: Some(1),
            grundlage: "Musterweg 3–9".into(),
        });
        let r = pruefe(&eingabe(), &p, 1).unwrap();
        let grz = r.nachweise.iter().find(|n| n.art == "grz").unwrap();
        assert_eq!(grz.soll, Some(0.25));
        assert_eq!(grz.quelle, Quelle::Umgebung);
        assert_eq!(grz.eingehalten, None);
        assert!(grz.abweichung.is_some());
        let vg = r
            .nachweise
            .iter()
            .find(|n| n.art == "vollgeschosse")
            .unwrap();
        assert_eq!(
            (vg.soll, vg.quelle, vg.eingehalten),
            (Some(1.0), Quelle::Umgebung, None)
        );
        assert!(
            r.nachweise
                .iter()
                .find(|n| n.art == "grz_gesamt")
                .unwrap()
                .soll
                .is_none()
        );
    }
    #[test]
    fn umgebungswerte_brauchen_referenzbebauung() {
        let mut u = Umgebungsreferenz {
            grz: Some(0.3),
            ..Default::default()
        };
        assert!(u.validate().is_err());
        u.grundlage = "Nachbarbebauung Nr. 5 und 7".into();
        assert!(u.validate().is_ok());
        u.grz = Some(1.2);
        assert!(u.validate().is_err());
        assert!(Umgebungsreferenz::default().validate().is_ok());
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
