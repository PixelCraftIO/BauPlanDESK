//! CAD-Pläne: Import in ein neutrales 2D-Modell und Aufmaß (Längen, Flächen).
//!
//! * [`dxf`] liest DXF-Dateien (ASCII und binär, über das Crate `dxf`, MIT)
//!   und zerlegt alle Elemente in Linienzüge und Beschriftungen. Bögen,
//!   Kreise, Ellipsen und Splines werden dabei in Polygone aufgelöst,
//!   Blockreferenzen aufgelöst. Der Viewer muss so nur Linienzüge zeichnen.
//! * [`messen`] berechnet Länge und Fläche eines gezeichneten oder
//!   angeklickten Linienzugs in Metern bzw. Quadratmetern.
//!
//! DWG wird bewusst nicht unterstützt: Die einzigen freien DWG-Bibliotheken
//! stehen unter GPL. Pläne werden daher als DXF ausgetauscht, was jedes
//! CAD-Programm exportieren kann.

pub mod dxf;
#[cfg(any(test, feature = "testdaten"))]
pub mod testdaten;

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

/// Punkt in Zeichnungseinheiten, serialisiert kompakt als `[x, y]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Punkt(pub f64, pub f64);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rechteck {
    pub min: Punkt,
    pub max: Punkt,
}

/// Längeneinheit der Zeichnung (DXF-Header `$INSUNITS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Laengeneinheit {
    Millimeter,
    Zentimeter,
    Dezimeter,
    Meter,
    Kilometer,
    Zoll,
    Fuss,
}

impl Laengeneinheit {
    /// Länge einer Zeichnungseinheit in Metern.
    pub fn meter(self) -> f64 {
        match self {
            Self::Millimeter => 0.001,
            Self::Zentimeter => 0.01,
            Self::Dezimeter => 0.1,
            Self::Meter => 1.0,
            Self::Kilometer => 1000.0,
            Self::Zoll => 0.0254,
            Self::Fuss => 0.3048,
        }
    }
}

/// Zeichnungsebene (Layer).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ebene {
    pub name: String,
    /// `#rrggbb`; `None` heißt Vordergrundfarbe (ACI 7, schwarz/weiß).
    pub farbe: Option<String>,
    /// In der Datei ausgeschaltet oder gefroren.
    pub sichtbar: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Linienzug {
    /// Index in [`Plan::ebenen`].
    pub ebene: usize,
    /// Eigene Farbe; `None` heißt Farbe der Ebene.
    pub farbe: Option<String>,
    pub punkte: Vec<Punkt>,
    pub geschlossen: bool,
}

/// Horizontale Ausrichtung einer Beschriftung zu ihrer Position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Ausrichtung {
    Links,
    Mitte,
    Rechts,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Beschriftung {
    pub ebene: usize,
    pub farbe: Option<String>,
    /// Anfang der Grundlinie der ersten Zeile (je nach Ausrichtung links,
    /// mittig oder rechts).
    pub position: Punkt,
    pub hoehe: f64,
    /// Abstand der Grundlinien mehrzeiliger Texte.
    pub zeilenabstand: f64,
    /// Drehung gegen den Uhrzeigersinn in Grad.
    pub drehung: f64,
    pub ausrichtung: Ausrichtung,
    /// Zeilen sind durch `\n` getrennt.
    pub inhalt: String,
}

/// Ein importierter Plan (nur Modellbereich).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Plan {
    /// Einheit laut Datei; `None`, wenn die Datei keine angibt.
    pub einheit: Option<Laengeneinheit>,
    pub ebenen: Vec<Ebene>,
    pub linien: Vec<Linienzug>,
    pub texte: Vec<Beschriftung>,
    /// `None` bei leeren Plänen.
    pub grenzen: Option<Rechteck>,
    /// Hinweise für den Anwender, z. B. übersprungene Elementtypen.
    pub hinweise: Vec<String>,
}

/// Ergebnis eines Aufmaßes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Messung {
    /// Länge des Linienzugs, bei geschlossenen Linienzügen der Umfang.
    pub laenge_m: f64,
    /// Nur bei geschlossenen Linienzügen mit mindestens drei Punkten.
    pub flaeche_m2: Option<f64>,
    /// Sich selbst schneidende Umrisse haben keine sinnvolle Fläche.
    pub selbstschneidend: bool,
}

/// Misst einen Linienzug in Zeichnungseinheiten und rechnet in Meter um.
pub fn messen(punkte: &[Punkt], geschlossen: bool, einheit: Laengeneinheit) -> CoreResult<Messung> {
    if punkte.len() < 2 {
        return Err(CoreError::InvalidInput(
            "Für ein Aufmaß werden mindestens zwei Punkte benötigt".into(),
        ));
    }
    if punkte.iter().any(|p| !(p.0.is_finite() && p.1.is_finite())) {
        return Err(CoreError::InvalidInput("Ungültige Koordinate".into()));
    }
    let m = einheit.meter();
    let kanten = kanten(punkte, geschlossen);
    let laenge: f64 = kanten
        .iter()
        .map(|(a, b)| (b.0 - a.0).hypot(b.1 - a.1))
        .sum();
    let flaeche = (geschlossen && punkte.len() >= 3).then(|| {
        // Gaußsche Trapezformel; relativ zum ersten Punkt gerechnet, damit
        // Landeskoordinaten (z. B. UTM mit Rechtswerten um 500 000 m) nicht
        // an Rechengenauigkeit verlieren.
        let o = punkte[0];
        let doppelt: f64 = kanten
            .iter()
            .map(|(a, b)| (a.0 - o.0) * (b.1 - o.1) - (b.0 - o.0) * (a.1 - o.1))
            .sum();
        doppelt.abs() / 2.0 * m * m
    });
    Ok(Messung {
        laenge_m: laenge * m,
        flaeche_m2: flaeche,
        selbstschneidend: geschlossen && schneidet_sich(&kanten),
    })
}

fn kanten(punkte: &[Punkt], geschlossen: bool) -> Vec<(Punkt, Punkt)> {
    let mut k: Vec<_> = punkte.windows(2).map(|w| (w[0], w[1])).collect();
    if geschlossen && punkte.len() > 2 && punkte.first() != punkte.last() {
        k.push((punkte[punkte.len() - 1], punkte[0]));
    }
    k.retain(|(a, b)| a != b);
    k
}

/// Prüft nicht benachbarte Kanten paarweise auf echte Schnitte.
fn schneidet_sich(kanten: &[(Punkt, Punkt)]) -> bool {
    let n = kanten.len();
    let kreuz =
        |o: Punkt, a: Punkt, b: Punkt| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    for i in 0..n {
        for j in i + 2..n {
            if i == 0 && j == n - 1 {
                continue; // benachbart über den Schluss
            }
            let ((p1, p2), (q1, q2)) = (kanten[i], kanten[j]);
            let d1 = kreuz(q1, q2, p1);
            let d2 = kreuz(q1, q2, p2);
            let d3 = kreuz(p1, p2, q1);
            let d4 = kreuz(p1, p2, q2);
            if d1 * d2 < 0.0 && d3 * d4 < 0.0 {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rechteck(b: f64, h: f64) -> Vec<Punkt> {
        vec![Punkt(0.0, 0.0), Punkt(b, 0.0), Punkt(b, h), Punkt(0.0, h)]
    }

    #[test]
    fn flaeche_und_umfang_eines_rechtecks() {
        let m = messen(&rechteck(20.0, 30.0), true, Laengeneinheit::Meter).unwrap();
        assert_eq!(m.flaeche_m2, Some(600.0));
        assert_eq!(m.laenge_m, 100.0);
        assert!(!m.selbstschneidend);
    }

    #[test]
    fn zentimeter_werden_umgerechnet() {
        let m = messen(&rechteck(2000.0, 3000.0), true, Laengeneinheit::Zentimeter).unwrap();
        assert!((m.flaeche_m2.unwrap() - 600.0).abs() < 1e-9);
        assert!((m.laenge_m - 100.0).abs() < 1e-9);
    }

    #[test]
    fn offener_linienzug_hat_keine_flaeche() {
        let m = messen(&rechteck(3.0, 4.0)[..3], false, Laengeneinheit::Meter).unwrap();
        assert_eq!(m.flaeche_m2, None);
        assert_eq!(m.laenge_m, 7.0);
    }

    #[test]
    fn schliessender_punkt_wird_nicht_doppelt_gezaehlt() {
        let mut p = rechteck(20.0, 30.0);
        p.push(p[0]);
        let m = messen(&p, true, Laengeneinheit::Meter).unwrap();
        assert_eq!((m.laenge_m, m.flaeche_m2), (100.0, Some(600.0)));
    }

    #[test]
    fn landeskoordinaten_bleiben_genau() {
        let p: Vec<_> = rechteck(20.0, 30.0)
            .into_iter()
            .map(|p| Punkt(p.0 + 32_512_345.678, p.1 + 5_403_210.987))
            .collect();
        let m = messen(&p, true, Laengeneinheit::Meter).unwrap();
        assert!((m.flaeche_m2.unwrap() - 600.0).abs() < 1e-6);
    }

    #[test]
    fn schleife_wird_erkannt() {
        let p = [
            Punkt(0.0, 0.0),
            Punkt(1.0, 1.0),
            Punkt(1.0, 0.0),
            Punkt(0.0, 1.0),
        ];
        assert!(
            messen(&p, true, Laengeneinheit::Meter)
                .unwrap()
                .selbstschneidend
        );
    }

    #[test]
    fn zu_wenige_punkte() {
        assert!(messen(&[Punkt(0.0, 0.0)], false, Laengeneinheit::Meter).is_err());
    }
}
