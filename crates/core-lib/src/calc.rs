//! Bundesrechtliche Kennzahlen nach BauNVO (§§ 17, 19, 20, 21).
//!
//! Bundesrecht ändert sich selten und gilt überall gleich, daher ist es nativ
//! in Rust umgesetzt. Landesrecht (Abstandsflächen etc.) liegt in Skripten,
//! siehe [`crate::lbo`].

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

/// Art der baulichen Nutzung (§§ 2–11 BauNVO).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ts_rs::TS)]
pub enum Baugebiet {
    /// Kleinsiedlungsgebiet
    WS,
    /// Reines Wohngebiet
    WR,
    /// Allgemeines Wohngebiet
    WA,
    /// Besonderes Wohngebiet
    WB,
    /// Dorfgebiet
    MD,
    /// Dörfliches Wohngebiet
    MDW,
    /// Mischgebiet
    MI,
    /// Urbanes Gebiet
    MU,
    /// Kerngebiet
    MK,
    /// Gewerbegebiet
    GE,
    /// Industriegebiet
    GI,
    /// Sondergebiet, das der Erholung dient (Wochenendhausgebiet)
    SO,
}

/// Orientierungswerte für Obergrenzen nach § 17 BauNVO.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Orientierungswerte {
    pub grz: f64,
    pub gfz: Option<f64>,
    pub bmz: Option<f64>,
}

impl Baugebiet {
    pub fn orientierungswerte(self) -> Orientierungswerte {
        let (grz, gfz, bmz) = match self {
            Self::WS => (0.2, Some(0.4), None),
            Self::WR | Self::WA => (0.4, Some(1.2), None),
            Self::WB => (0.6, Some(1.6), None),
            Self::MD | Self::MDW | Self::MI => (0.6, Some(1.2), None),
            Self::MU => (0.8, Some(3.0), None),
            Self::MK => (1.0, Some(3.0), None),
            Self::GE | Self::GI => (0.8, Some(2.4), Some(10.0)),
            Self::SO => (0.2, Some(0.2), None),
        };
        Orientierungswerte { grz, gfz, bmz }
    }
}

/// Eingangsgrößen, typischerweise aus dem CAD-Viewer ermittelt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MassDerNutzung {
    /// Maßgebende Grundstücksfläche (§ 19 Abs. 3 BauNVO), m²
    pub grundstuecksflaeche_m2: f64,
    /// Grundfläche der Hauptanlagen, m²
    pub grundflaeche_m2: f64,
    /// Garagen, Stellplätze, Zufahrten, Nebenanlagen (§ 19 Abs. 4 BauNVO), m²
    pub grundflaeche_nebenanlagen_m2: f64,
    /// Geschossfläche in Vollgeschossen (§ 20 Abs. 3 BauNVO), m²
    pub geschossflaeche_m2: f64,
    /// Baumasse (§ 21 Abs. 2 BauNVO), m³
    pub baumasse_m3: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Kennzahlen {
    pub grz: f64,
    /// GRZ inkl. Anlagen nach § 19 Abs. 4 BauNVO ("GRZ II")
    pub grz_gesamt: f64,
    pub gfz: f64,
    pub bmz: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pruefergebnis {
    pub kennzahlen: Kennzahlen,
    pub grenzwerte: Orientierungswerte,
    /// § 19 Abs. 4 S. 2 BauNVO: Überschreitung bis 50 %, höchstens 0,8
    pub grz_gesamt_max: f64,
    pub grz_eingehalten: bool,
    pub grz_gesamt_eingehalten: bool,
    pub gfz_eingehalten: Option<bool>,
    pub bmz_eingehalten: Option<bool>,
}

pub fn kennzahlen(m: &MassDerNutzung) -> CoreResult<Kennzahlen> {
    let flaeche = m.grundstuecksflaeche_m2;
    if !(flaeche.is_finite() && flaeche > 0.0) {
        return Err(CoreError::InvalidInput(
            "Grundstücksfläche muss größer als 0 sein".into(),
        ));
    }
    for (name, wert) in [
        ("Grundfläche", m.grundflaeche_m2),
        ("Grundfläche Nebenanlagen", m.grundflaeche_nebenanlagen_m2),
        ("Geschossfläche", m.geschossflaeche_m2),
        ("Baumasse", m.baumasse_m3.unwrap_or(0.0)),
    ] {
        if !(wert.is_finite() && wert >= 0.0) {
            return Err(CoreError::InvalidInput(format!(
                "{name} darf nicht negativ sein"
            )));
        }
    }
    Ok(Kennzahlen {
        grz: m.grundflaeche_m2 / flaeche,
        grz_gesamt: (m.grundflaeche_m2 + m.grundflaeche_nebenanlagen_m2) / flaeche,
        gfz: m.geschossflaeche_m2 / flaeche,
        bmz: m.baumasse_m3.map(|bm| bm / flaeche),
    })
}

/// Prüft gegen die Orientierungswerte des § 17 BauNVO. Ein Bebauungsplan
/// kann abweichende Werte festsetzen; diese können über `grenzwerte`
/// übergeben werden.
pub fn pruefe(
    m: &MassDerNutzung,
    gebiet: Baugebiet,
    grenzwerte: Option<Orientierungswerte>,
) -> CoreResult<Pruefergebnis> {
    let k = kennzahlen(m)?;
    let g = grenzwerte.unwrap_or_else(|| gebiet.orientierungswerte());
    let regel = crate::baunvo::regelwerk(crate::flaechen::BauNvo::V2017);
    let grz_gesamt_max = (g.grz * regel.grz_ii_faktor)
        .min(regel.grz_ii_kappung)
        .max(g.grz);
    Ok(Pruefergebnis {
        grz_eingehalten: k.grz <= g.grz + f64::EPSILON,
        grz_gesamt_eingehalten: k.grz_gesamt <= grz_gesamt_max + f64::EPSILON,
        gfz_eingehalten: g.gfz.map(|max| k.gfz <= max + f64::EPSILON),
        bmz_eingehalten: match (k.bmz, g.bmz) {
            (Some(ist), Some(max)) => Some(ist <= max + f64::EPSILON),
            _ => None,
        },
        grz_gesamt_max,
        grenzwerte: g,
        kennzahlen: k,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn einfamilienhaus_im_wa() {
        let m = MassDerNutzung {
            grundstuecksflaeche_m2: 600.0,
            grundflaeche_m2: 150.0,
            grundflaeche_nebenanlagen_m2: 120.0,
            geschossflaeche_m2: 300.0,
            baumasse_m3: None,
        };
        let r = pruefe(&m, Baugebiet::WA, None).unwrap();
        assert!((r.kennzahlen.grz - 0.25).abs() < 1e-9);
        assert!((r.kennzahlen.grz_gesamt - 0.45).abs() < 1e-9);
        assert!((r.kennzahlen.gfz - 0.5).abs() < 1e-9);
        assert!((r.grz_gesamt_max - 0.6).abs() < 1e-9);
        assert!(r.grz_eingehalten && r.grz_gesamt_eingehalten);
        assert_eq!(r.gfz_eingehalten, Some(true));
    }

    #[test]
    fn grz_ueberschritten() {
        let m = MassDerNutzung {
            grundstuecksflaeche_m2: 400.0,
            grundflaeche_m2: 200.0,
            grundflaeche_nebenanlagen_m2: 0.0,
            geschossflaeche_m2: 400.0,
            baumasse_m3: None,
        };
        let r = pruefe(&m, Baugebiet::WR, None).unwrap();
        assert!(!r.grz_eingehalten);
    }

    #[test]
    fn ungueltige_flaeche() {
        let m = MassDerNutzung {
            grundstuecksflaeche_m2: 0.0,
            grundflaeche_m2: 1.0,
            grundflaeche_nebenanlagen_m2: 0.0,
            geschossflaeche_m2: 1.0,
            baumasse_m3: None,
        };
        assert!(kennzahlen(&m).is_err());
    }
}
