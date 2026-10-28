//! Geometrie aus IFC-Darstellungen, ausgewertet für Grundrisse und Höhen.
//!
//! Ausgewertet werden die Darstellungen, die Bauantrags-Modelle fast immer
//! verwenden: Extrusionen (`IfcExtrudedAreaSolid`) mit Rechteck-, Kreis- und
//! freien Profilen, Flächenmodelle (`IfcFacetedBrep`, `IfcPolygonalFaceSet`,
//! `IfcTriangulatedFaceSet`, Shell-/Face-Modelle) und abgebildete Darstellungen
//! (`IfcMappedItem`). Boolesche Ergebnisse werden über ihren ersten Operanden
//! angenähert und als Näherung gekennzeichnet; andere Darstellungen (z. B.
//! Rotationskörper, NURBS) bleiben unausgewertet und werden im Bericht genannt.
//!
//! Alle Ergebnisse sind in Metern im Weltsystem des Modells (x, y waagerecht,
//! z nach oben). Senkrechte Extrusionen werden exakt geführt ([`Teil::Prisma`]),
//! alle übrigen als Hülle ihrer Projektion mit Höhenbereich ([`Teil::Huelle`]).
use super::step::{Instanz, StepDatei, Wert};
use crate::planaufmass::{Planpolygon, Planpunkt};
use geo::{Area, BooleanOps, Coord, LineString, MultiPolygon, Orient, Polygon};

pub type V3 = [f64; 3];

/// Affine Abbildung: Spalten x, y, z und Verschiebung.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    pub x: V3,
    pub y: V3,
    pub z: V3,
    pub o: V3,
}
impl Matrix {
    pub const EINS: Matrix = Matrix {
        x: [1., 0., 0.],
        y: [0., 1., 0.],
        z: [0., 0., 1.],
        o: [0., 0., 0.],
    };
    pub fn punkt(&self, p: V3) -> V3 {
        add(self.o, self.richtung(p))
    }
    pub fn richtung(&self, p: V3) -> V3 {
        std::array::from_fn(|i| self.x[i] * p[0] + self.y[i] * p[1] + self.z[i] * p[2])
    }
    /// `self ∘ b`: erst `b`, dann `self`.
    pub fn mal(&self, b: &Matrix) -> Matrix {
        Matrix {
            x: self.richtung(b.x),
            y: self.richtung(b.y),
            z: self.richtung(b.z),
            o: self.punkt(b.o),
        }
    }
}

fn norm(v: V3) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}
fn normiert(v: V3) -> Option<V3> {
    let n = norm(v);
    (n > 1e-12 && n.is_finite()).then(|| [v[0] / n, v[1] / n, v[2] / n])
}
fn kreuz(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn skalar(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Ausgewerteter Teil eines Körpers in Metern (Weltsystem).
#[derive(Debug, Clone, PartialEq)]
pub enum Teil {
    /// Senkrechte Extrusion: Grundriss gilt für jede Höhe zwischen unten und oben.
    Prisma {
        grundriss: MultiPolygon<f64>,
        unten: f64,
        oben: f64,
    },
    /// Projektion einer beliebigen Geometrie mit ihrem Höhenbereich.
    Huelle {
        grundriss: MultiPolygon<f64>,
        unten: f64,
        oben: f64,
        volumen: Option<f64>,
    },
}
impl Teil {
    pub fn grundriss(&self) -> &MultiPolygon<f64> {
        match self {
            Self::Prisma { grundriss, .. } | Self::Huelle { grundriss, .. } => grundriss,
        }
    }
    pub fn hoehen(&self) -> (f64, f64) {
        match self {
            Self::Prisma { unten, oben, .. } | Self::Huelle { unten, oben, .. } => (*unten, *oben),
        }
    }
    pub fn volumen(&self) -> Option<f64> {
        match self {
            Self::Prisma {
                grundriss,
                unten,
                oben,
            } => Some(grundriss.unsigned_area() * (oben - unten)),
            Self::Huelle { volumen, .. } => *volumen,
        }
    }
}

/// Ergebnis der Auswertung einer Produktdarstellung.
#[derive(Debug, Clone, Default)]
pub struct Auswertung {
    pub teile: Vec<Teil>,
    /// Boolesche Operationen oder schräge Körper angenähert.
    pub naeherung: bool,
    /// Nicht ausgewertete Darstellungsarten (Entitätsnamen).
    pub unbekannt: Vec<String>,
}

pub(super) struct Geometrie<'a> {
    pub d: &'a StepDatei,
    /// Meter je Längeneinheit des Modells.
    pub m: f64,
}

const MAX_TIEFE: usize = 32;
/// Höchstzahl an Flächen je Element, die in die Projektion eingehen.
const MAX_FLAECHEN: usize = 20_000;

impl<'a> Geometrie<'a> {
    fn inst(&self, w: &Wert) -> Option<&'a Instanz> {
        self.d.get(w.referenz()?)
    }
    pub fn koordinaten(&self, w: &Wert) -> Option<V3> {
        let i = self.inst(w)?;
        if i.typ != "IFCCARTESIANPOINT" {
            return None;
        }
        let k: Vec<f64> = i.attr(0).liste().iter().filter_map(Wert::zahl).collect();
        let p = [
            *k.first()?,
            *k.get(1).unwrap_or(&0.),
            *k.get(2).unwrap_or(&0.),
        ];
        p.iter()
            .all(|v| v.is_finite())
            .then(|| [p[0] * self.m, p[1] * self.m, p[2] * self.m])
    }
    fn richtung(&self, w: &Wert) -> Option<V3> {
        let i = self.inst(w)?;
        if i.typ != "IFCDIRECTION" {
            return None;
        }
        let k: Vec<f64> = i.attr(0).liste().iter().filter_map(Wert::zahl).collect();
        normiert([
            *k.first()?,
            *k.get(1).unwrap_or(&0.),
            *k.get(2).unwrap_or(&0.),
        ])
    }
    /// `IfcAxis2Placement3D` / `IfcAxis2Placement2D`; fehlend = Identität.
    pub fn achsen(&self, w: &Wert) -> Matrix {
        let Some(i) = self.inst(w) else {
            return Matrix::EINS;
        };
        let o = self.koordinaten(i.attr(0)).unwrap_or([0.; 3]);
        match i.typ.as_str() {
            "IFCAXIS2PLACEMENT3D" => {
                let z = self.richtung(i.attr(1)).unwrap_or([0., 0., 1.]);
                let xr = self.richtung(i.attr(2)).unwrap_or(if z[0].abs() < 0.9 {
                    [1., 0., 0.]
                } else {
                    [0., 1., 0.]
                });
                // x senkrecht zu z machen (Gram-Schmidt), y = z × x.
                let d = skalar(xr, z);
                let x = normiert([xr[0] - d * z[0], xr[1] - d * z[1], xr[2] - d * z[2]])
                    .unwrap_or([1., 0., 0.]);
                Matrix {
                    x,
                    y: kreuz(z, x),
                    z,
                    o,
                }
            }
            "IFCAXIS2PLACEMENT2D" => {
                let x = self
                    .richtung(i.attr(1))
                    .and_then(|x| normiert([x[0], x[1], 0.]))
                    .unwrap_or([1., 0., 0.]);
                Matrix {
                    x,
                    y: [-x[1], x[0], 0.],
                    z: [0., 0., 1.],
                    o,
                }
            }
            _ => Matrix::EINS,
        }
    }
    /// Weltlage eines `IfcLocalPlacement` über die Kette `PlacementRelTo`.
    pub fn platzierung(&self, w: &Wert) -> Matrix {
        let mut m = Matrix::EINS;
        let mut aktuell = w.clone();
        for _ in 0..MAX_TIEFE {
            let Some(i) = self.inst(&aktuell) else { break };
            if i.typ != "IFCLOCALPLACEMENT" {
                break;
            }
            m = self.achsen(i.attr(1)).mal(&m);
            aktuell = i.attr(0).clone();
        }
        m
    }
    /// `IfcCartesianTransformationOperator3D` (auch nicht gleichmäßig skaliert).
    fn operator(&self, w: &Wert) -> Matrix {
        let Some(i) = self.inst(w) else {
            return Matrix::EINS;
        };
        let x = self.richtung(i.attr(0)).unwrap_or([1., 0., 0.]);
        let y = self.richtung(i.attr(1)).unwrap_or([0., 1., 0.]);
        let o = self.koordinaten(i.attr(2)).unwrap_or([0.; 3]);
        let s = i.attr(3).zahl().unwrap_or(1.);
        let z = self.richtung(i.attr(4)).unwrap_or_else(|| kreuz(x, y));
        let (s2, s3) = if i.typ == "IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM" {
            (i.attr(5).zahl().unwrap_or(s), i.attr(6).zahl().unwrap_or(s))
        } else {
            (s, s)
        };
        let f = |v: V3, s: f64| [v[0] * s, v[1] * s, v[2] * s];
        Matrix {
            x: f(x, s),
            y: f(y, s2),
            z: f(z, s3),
            o,
        }
    }

    /// Ebene Kurve als Punktfolge in Metern (Profilkoordinaten).
    pub fn kurve(&self, w: &Wert, tiefe: usize) -> Option<Vec<[f64; 2]>> {
        if tiefe > MAX_TIEFE {
            return None;
        }
        let i = self.inst(w)?;
        let pts = match i.typ.as_str() {
            "IFCPOLYLINE" => i
                .attr(0)
                .liste()
                .iter()
                .map(|p| self.koordinaten(p).map(|p| [p[0], p[1]]))
                .collect::<Option<Vec<_>>>()?,
            "IFCINDEXEDPOLYCURVE" => {
                let liste = self.inst(i.attr(0))?;
                let coords: Vec<[f64; 2]> = liste
                    .attr(0)
                    .liste()
                    .iter()
                    .map(|p| {
                        let k: Vec<f64> = p.liste().iter().filter_map(Wert::zahl).collect();
                        Some([*k.first()? * self.m, *k.get(1)? * self.m])
                    })
                    .collect::<Option<Vec<_>>>()?;
                let idx = |w: &Wert| -> Option<Vec<usize>> {
                    w.liste()
                        .iter()
                        .map(|v| {
                            v.zahl()
                                .map(|z| z as usize)
                                .filter(|&z| z >= 1 && z <= coords.len())
                        })
                        .collect()
                };
                match i.attr(1) {
                    Wert::Liste(segmente) => {
                        let mut out: Vec<[f64; 2]> = Vec::new();
                        for s in segmente {
                            let (typ, inner) = match s {
                                Wert::Typisiert(t, inner) => (t.as_str(), inner.as_ref()),
                                _ => return None,
                            };
                            let ix = idx(inner)?;
                            let neu: Vec<[f64; 2]> = if typ == "IFCARCINDEX" && ix.len() == 3 {
                                bogen(coords[ix[0] - 1], coords[ix[1] - 1], coords[ix[2] - 1])
                            } else {
                                ix.iter().map(|&k| coords[k - 1]).collect()
                            };
                            for p in neu {
                                if out.last() != Some(&p) {
                                    out.push(p);
                                }
                            }
                        }
                        out
                    }
                    _ => coords,
                }
            }
            "IFCCOMPOSITECURVE" => {
                let mut out: Vec<[f64; 2]> = Vec::new();
                for s in i.attr(0).liste() {
                    let seg = self.inst(s)?;
                    let gleich = !matches!(seg.attr(1).enum_wert(), Some("F"));
                    let mut p = self.kurve(seg.attr(2), tiefe + 1)?;
                    if !gleich {
                        p.reverse();
                    }
                    for q in p {
                        if out.last().is_none_or(|l| dist2(*l, q) > 1e-18) {
                            out.push(q);
                        }
                    }
                }
                out
            }
            "IFCTRIMMEDCURVE" => self.getrimmt(i)?,
            "IFCCIRCLE" => {
                let m = self.achsen(i.attr(0));
                let r = i.attr(1).zahl()? * self.m;
                (0..48)
                    .map(|k| {
                        let a = k as f64 / 48. * std::f64::consts::TAU;
                        let p = m.punkt([r * a.cos(), r * a.sin(), 0.]);
                        [p[0], p[1]]
                    })
                    .collect()
            }
            _ => return None,
        };
        (pts.len() >= 2).then_some(pts)
    }
    /// Getrimmter Kreisbogen oder getrimmte Linie mit kartesischen Trimmpunkten
    /// oder Parameterwinkeln.
    fn getrimmt(&self, i: &Instanz) -> Option<Vec<[f64; 2]>> {
        let basis = self.inst(i.attr(0))?;
        let gleich = !matches!(i.attr(3).enum_wert(), Some("F"));
        let trim = |w: &Wert| -> (Option<[f64; 2]>, Option<f64>) {
            let mut p = None;
            let mut t = None;
            for v in w.liste() {
                match v {
                    Wert::Ref(_) => p = self.koordinaten(v).map(|k| [k[0], k[1]]),
                    Wert::Typisiert(_, z) => t = z.zahl(),
                    _ => {}
                }
            }
            (p, t)
        };
        let (p1, t1) = trim(i.attr(1));
        let (p2, t2) = trim(i.attr(2));
        match basis.typ.as_str() {
            "IFCCIRCLE" => {
                let m = self.achsen(basis.attr(0));
                let r = basis.attr(1).zahl()? * self.m;
                let winkel = |p: Option<[f64; 2]>, t: Option<f64>| -> Option<f64> {
                    if let Some(p) = p {
                        let d = [p[0] - m.o[0], p[1] - m.o[1], 0.];
                        Some(skalar(d, m.y).atan2(skalar(d, m.x)))
                    } else {
                        // Parameter in Grad (IFC-Standardeinheit für ebene Winkel in den meisten Exporten).
                        t.map(f64::to_radians)
                    }
                };
                let a1 = winkel(p1, t1)?;
                let mut a2 = winkel(p2, t2)?;
                if gleich {
                    while a2 <= a1 {
                        a2 += std::f64::consts::TAU;
                    }
                } else {
                    while a2 >= a1 {
                        a2 -= std::f64::consts::TAU;
                    }
                }
                let n =
                    (((a2 - a1).abs() / std::f64::consts::TAU * 48.).ceil() as usize).clamp(2, 96);
                Some(
                    (0..=n)
                        .map(|k| {
                            let a = a1 + (a2 - a1) * k as f64 / n as f64;
                            let p = m.punkt([r * a.cos(), r * a.sin(), 0.]);
                            [p[0], p[1]]
                        })
                        .collect(),
                )
            }
            "IFCLINE" | "IFCPOLYLINE" => {
                let (a, b) = (p1?, p2?);
                Some(if gleich { vec![a, b] } else { vec![b, a] })
            }
            _ => None,
        }
    }

    /// Profil als Ringe (erster Ring außen) in Profilkoordinaten (Meter).
    pub fn profil(&self, w: &Wert) -> Option<Vec<Vec<[f64; 2]>>> {
        let i = self.inst(w)?;
        let lage = |n: usize| self.achsen(i.attr(n));
        let ringe = match i.typ.as_str() {
            "IFCRECTANGLEPROFILEDEF" | "IFCROUNDEDRECTANGLEPROFILEDEF" => {
                let (x, y) = (
                    i.attr(3).zahl()? * self.m / 2.,
                    i.attr(4).zahl()? * self.m / 2.,
                );
                vec![abbilden(&lage(2), &[[-x, -y], [x, -y], [x, y], [-x, y]])]
            }
            "IFCRECTANGLEHOLLOWPROFILEDEF" => {
                let (x, y) = (
                    i.attr(3).zahl()? * self.m / 2.,
                    i.attr(4).zahl()? * self.m / 2.,
                );
                let t = i.attr(5).zahl()? * self.m;
                let m = lage(2);
                let mut r = vec![abbilden(&m, &[[-x, -y], [x, -y], [x, y], [-x, y]])];
                if t < x && t < y {
                    let (a, b) = (x - t, y - t);
                    r.push(abbilden(&m, &[[-a, -b], [-a, b], [a, b], [a, -b]]));
                }
                r
            }
            "IFCCIRCLEPROFILEDEF" | "IFCCIRCLEHOLLOWPROFILEDEF" => {
                let r = i.attr(3).zahl()? * self.m;
                let m = lage(2);
                let kreis = |r: f64| -> Vec<[f64; 2]> {
                    (0..48)
                        .map(|k| {
                            let a = k as f64 / 48. * std::f64::consts::TAU;
                            [r * a.cos(), r * a.sin()]
                        })
                        .collect()
                };
                let mut ringe = vec![abbilden(&m, &kreis(r))];
                if i.typ == "IFCCIRCLEHOLLOWPROFILEDEF" {
                    let t = i.attr(4).zahl()? * self.m;
                    if t < r {
                        let mut innen = abbilden(&m, &kreis(r - t));
                        innen.reverse();
                        ringe.push(innen);
                    }
                }
                ringe
            }
            "IFCARBITRARYCLOSEDPROFILEDEF" => vec![self.kurve(i.attr(2), 0)?],
            "IFCARBITRARYPROFILEDEFWITHVOIDS" => {
                let mut r = vec![self.kurve(i.attr(2), 0)?];
                for v in i.attr(3).liste() {
                    r.push(self.kurve(v, 0)?);
                }
                r
            }
            // Übrige parametrische Profile (I, L, T, U …) über ihren Umriss angenähert.
            _ => return None,
        };
        let ringe: Vec<Vec<[f64; 2]>> = ringe.into_iter().map(geschlossen_ohne_doppelung).collect();
        ringe.iter().all(|r| r.len() >= 3).then_some(ringe)
    }

    /// Wertet die Körperdarstellung eines Produkts aus (`Representation`), in
    /// Weltlage `lage`.
    pub fn produkt(&self, darstellung: &Wert, lage: &Matrix) -> Auswertung {
        let mut a = Auswertung::default();
        let Some(pds) = self.inst(darstellung) else {
            return a;
        };
        let reps: Vec<&Instanz> = pds
            .attr(2)
            .liste()
            .iter()
            .filter_map(|r| self.inst(r))
            .filter(|r| r.typ == "IFCSHAPEREPRESENTATION")
            .collect();
        // Bevorzugt die Körperdarstellung; ohne Kennung jede dreidimensionale.
        let body: Vec<&Instanz> = reps
            .iter()
            .copied()
            .filter(|r| {
                r.attr(1)
                    .text()
                    .is_some_and(|t| t.eq_ignore_ascii_case("Body"))
            })
            .collect();
        let auswahl = if body.is_empty() {
            reps.into_iter()
                .filter(|r| {
                    !r.attr(1).text().is_some_and(|t| {
                        [
                            "Axis",
                            "FootPrint",
                            "Annotation",
                            "Box",
                            "Profile",
                            "Plan",
                            "Reference",
                        ]
                        .iter()
                        .any(|k| t.eq_ignore_ascii_case(k))
                    })
                })
                .collect()
        } else {
            body
        };
        for rep in auswahl {
            for item in rep.attr(3).liste() {
                self.element(item, lage, &mut a, 0);
            }
        }
        a.unbekannt.sort();
        a.unbekannt.dedup();
        a
    }

    fn element(&self, w: &Wert, lage: &Matrix, a: &mut Auswertung, tiefe: usize) {
        if tiefe > MAX_TIEFE {
            return;
        }
        let Some(i) = self.inst(w) else { return };
        match i.typ.as_str() {
            "IFCEXTRUDEDAREASOLID" | "IFCEXTRUDEDAREASOLIDTAPERED" => {
                if i.typ == "IFCEXTRUDEDAREASOLIDTAPERED" {
                    a.naeherung = true;
                }
                let (Some(profil), Some(tiefe_m)) = (self.profil(i.attr(0)), i.attr(3).zahl())
                else {
                    a.unbekannt.push(
                        self.inst(i.attr(0))
                            .map(|p| p.typ.clone())
                            .unwrap_or_else(|| i.typ.clone()),
                    );
                    return;
                };
                let m = lage.mal(&self.achsen(i.attr(1)));
                let r = self.richtung(i.attr(2)).unwrap_or([0., 0., 1.]);
                let v = m.richtung([
                    r[0] * tiefe_m * self.m,
                    r[1] * tiefe_m * self.m,
                    r[2] * tiefe_m * self.m,
                ]);
                let ringe3: Vec<Vec<V3>> = profil
                    .iter()
                    .map(|r| r.iter().map(|p| m.punkt([p[0], p[1], 0.])).collect())
                    .collect();
                let waagerecht = norm(kreuz(m.z, [0., 0., 1.])) < 1e-6;
                let senkrecht = v[0].abs() < 1e-9 && v[1].abs() < 1e-9;
                if waagerecht && senkrecht {
                    let z0 = ringe3[0][0][2];
                    let (unten, oben) = if v[2] >= 0. {
                        (z0, z0 + v[2])
                    } else {
                        (z0 + v[2], z0)
                    };
                    if let Some(g) = polygon_aus(&ringe3) {
                        a.teile.push(Teil::Prisma {
                            grundriss: g,
                            unten,
                            oben,
                        });
                    }
                } else {
                    // Schräger Körper: Hülle aus Boden, Deckel und Seitenflächen.
                    let mut flaechen: Vec<Vec<V3>> = Vec::new();
                    for r in &ringe3 {
                        let oben: Vec<V3> = r.iter().map(|p| add(*p, v)).collect();
                        for k in 0..r.len() {
                            let n = (k + 1) % r.len();
                            flaechen.push(vec![r[k], r[n], oben[n], oben[k]]);
                        }
                        flaechen.push(oben);
                        flaechen.push(r.clone());
                    }
                    let flaeche = profil_flaeche(&profil);
                    let hoehe = skalar(v, m.z).abs();
                    self.huelle(flaechen, Some(flaeche * hoehe), a);
                }
            }
            "IFCFACETEDBREP" | "IFCFACETEDBREPWITHVOIDS" => {
                let flaechen = self.schale(i.attr(0), lage);
                self.huelle(flaechen, None, a);
            }
            "IFCSHELLBASEDSURFACEMODEL" | "IFCFACEBASEDSURFACEMODEL" => {
                let mut f = Vec::new();
                for s in i.attr(0).liste() {
                    f.extend(self.schale(s, lage));
                }
                self.huelle(f, None, a);
            }
            "IFCPOLYGONALFACESET"
            | "IFCTRIANGULATEDFACESET"
            | "IFCTRIANGULATEDIRREGULARNETWORK" => {
                let f = self.flaechennetz(i, lage);
                self.huelle(f, None, a);
            }
            "IFCMAPPEDITEM" => {
                let Some(quelle) = self.inst(i.attr(0)) else {
                    return;
                };
                let m = lage
                    .mal(&self.operator(i.attr(1)))
                    .mal(&self.achsen(quelle.attr(0)));
                if let Some(rep) = self.inst(quelle.attr(1)) {
                    for item in rep.attr(3).liste() {
                        self.element(item, &m, a, tiefe + 1);
                    }
                }
            }
            "IFCBOOLEANCLIPPINGRESULT" | "IFCBOOLEANRESULT" => {
                // Vereinigung: beide Operanden; Differenz/Schnitt: erster Operand als Näherung.
                let op = i.attr(0).enum_wert().unwrap_or("");
                if op != "UNION" {
                    a.naeherung = true;
                }
                self.element(i.attr(1), lage, a, tiefe + 1);
                if op == "UNION" {
                    self.element(i.attr(2), lage, a, tiefe + 1);
                }
            }
            "IFCHALFSPACESOLID" | "IFCPOLYGONALBOUNDEDHALFSPACE" | "IFCBOXEDHALFSPACE" => {}
            "IFCBLOCK" => {
                let m = lage.mal(&self.achsen(i.attr(0)));
                let (x, y, z) = (
                    i.attr(1).zahl().unwrap_or(0.) * self.m,
                    i.attr(2).zahl().unwrap_or(0.) * self.m,
                    i.attr(3).zahl().unwrap_or(0.) * self.m,
                );
                let ecken = |zz: f64| -> Vec<V3> {
                    [[0., 0.], [x, 0.], [x, y], [0., y]]
                        .iter()
                        .map(|p| m.punkt([p[0], p[1], zz]))
                        .collect()
                };
                let (u, o) = (ecken(0.), ecken(z));
                let mut f = vec![u.clone(), o.clone()];
                for k in 0..4 {
                    let n = (k + 1) % 4;
                    f.push(vec![u[k], u[n], o[n], o[k]]);
                }
                self.huelle(f, Some(x * y * z), a);
            }
            _ => a.unbekannt.push(i.typ.clone()),
        }
    }

    fn schale(&self, w: &Wert, lage: &Matrix) -> Vec<Vec<V3>> {
        let Some(s) = self.inst(w) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for f in s.attr(0).liste().iter().take(MAX_FLAECHEN) {
            let Some(f) = self.inst(f) else { continue };
            for b in f.attr(0).liste() {
                let Some(b) = self.inst(b) else { continue };
                if b.typ != "IFCFACEOUTERBOUND" && f.attr(0).liste().len() > 1 {
                    // Löcher in Flächen ändern die Projektion der Hülle nicht.
                    continue;
                }
                let Some(loop_) = self.inst(b.attr(0)) else {
                    continue;
                };
                if loop_.typ != "IFCPOLYLOOP" {
                    continue;
                }
                let pts: Vec<V3> = loop_
                    .attr(0)
                    .liste()
                    .iter()
                    .filter_map(|p| self.koordinaten(p))
                    .map(|p| lage.punkt(p))
                    .collect();
                if pts.len() >= 3 {
                    out.push(pts);
                }
            }
        }
        out
    }

    fn flaechennetz(&self, i: &Instanz, lage: &Matrix) -> Vec<Vec<V3>> {
        let Some(liste) = self.inst(i.attr(0)) else {
            return Vec::new();
        };
        let coords: Vec<V3> = liste
            .attr(0)
            .liste()
            .iter()
            .filter_map(|p| {
                let k: Vec<f64> = p.liste().iter().filter_map(Wert::zahl).collect();
                Some(lage.punkt([
                    *k.first()? * self.m,
                    *k.get(1)? * self.m,
                    *k.get(2).unwrap_or(&0.) * self.m,
                ]))
            })
            .collect();
        let pn: Vec<usize> = i
            .attribute
            .iter()
            .rev()
            .find(|a| matches!(a, Wert::Liste(l) if l.first().is_some_and(|w| matches!(w, Wert::Ganz(_)))))
            .map(|l| l.liste().iter().filter_map(|v| v.zahl()).map(|z| z as usize).collect())
            .unwrap_or_default();
        let punkt = |k: usize| -> Option<V3> {
            let k = if pn.is_empty() {
                k
            } else {
                *pn.get(k.checked_sub(1)?)?
            };
            coords.get(k.checked_sub(1)?).copied()
        };
        let mut out = Vec::new();
        if i.typ == "IFCPOLYGONALFACESET" {
            for f in i.attr(2).liste().iter().take(MAX_FLAECHEN) {
                let Some(f) = self.inst(f) else { continue };
                let pts: Vec<V3> = f
                    .attr(0)
                    .liste()
                    .iter()
                    .filter_map(|v| v.zahl().and_then(|z| punkt(z as usize)))
                    .collect();
                if pts.len() >= 3 {
                    out.push(pts);
                }
            }
        } else {
            // Dreiecke: das Attribut mit Listen aus je drei Ganzzahlen (CoordIndex).
            let index = i.attribute.iter().find(|a| {
                a.liste()
                    .first()
                    .is_some_and(|t| t.liste().len() == 3 && matches!(t.liste()[0], Wert::Ganz(_)))
            });
            for t in index
                .map(|w| w.liste())
                .unwrap_or(&[])
                .iter()
                .take(MAX_FLAECHEN)
            {
                let pts: Vec<V3> = t
                    .liste()
                    .iter()
                    .filter_map(|v| v.zahl().and_then(|z| punkt(z as usize)))
                    .collect();
                if pts.len() == 3 {
                    out.push(pts);
                }
            }
        }
        out
    }

    fn huelle(&self, flaechen: Vec<Vec<V3>>, volumen: Option<f64>, a: &mut Auswertung) {
        if flaechen.is_empty() {
            return;
        }
        let (mut unten, mut oben) = (f64::INFINITY, f64::NEG_INFINITY);
        let mut polys = Vec::new();
        let mut v = 0.;
        for f in &flaechen {
            for p in f {
                unten = unten.min(p[2]);
                oben = oben.max(p[2]);
            }
            // Signiertes Volumen über die Flächen (geschlossene, gleich orientierte Hülle).
            for k in 1..f.len().saturating_sub(1) {
                v += skalar(f[0], kreuz(f[k], f[k + 1])) / 6.;
            }
            let ring: Vec<Coord<f64>> = f.iter().map(|p| Coord { x: p[0], y: p[1] }).collect();
            let poly = Polygon::new(LineString::from(ring), vec![]);
            if poly.unsigned_area() > 1e-8 {
                polys.push(poly);
            }
        }
        if polys.is_empty() || !unten.is_finite() {
            return;
        }
        let grundriss = vereinen(polys);
        a.teile.push(Teil::Huelle {
            grundriss,
            unten,
            oben,
            volumen: volumen.or((v.abs() > 1e-9).then_some(v.abs())),
        });
    }
}

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn dist2(a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
}
fn abbilden(m: &Matrix, pts: &[[f64; 2]]) -> Vec<[f64; 2]> {
    pts.iter()
        .map(|p| {
            let q = m.punkt([p[0], p[1], 0.]);
            [q[0], q[1]]
        })
        .collect()
}
fn geschlossen_ohne_doppelung(mut r: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    r.dedup_by(|a, b| dist2(*a, *b) < 1e-18);
    while r.len() > 1 && dist2(r[0], r[r.len() - 1]) < 1e-18 {
        r.pop();
    }
    r
}
/// Kreisbogen durch drei Punkte (Anfang, Mitte, Ende).
fn bogen(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> Vec<[f64; 2]> {
    let d = 2. * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    if d.abs() < 1e-12 {
        return vec![a, c];
    }
    let q = |p: [f64; 2]| p[0] * p[0] + p[1] * p[1];
    let ux = (q(a) * (b[1] - c[1]) + q(b) * (c[1] - a[1]) + q(c) * (a[1] - b[1])) / d;
    let uy = (q(a) * (c[0] - b[0]) + q(b) * (a[0] - c[0]) + q(c) * (b[0] - a[0])) / d;
    let r = dist2(a, [ux, uy]).sqrt();
    let w = |p: [f64; 2]| (p[1] - uy).atan2(p[0] - ux);
    let (w1, mut w2, wm) = (w(a), w(c), w(b));
    // Richtung so wählen, dass der Mittelpunkt auf dem Bogen liegt.
    let tau = std::f64::consts::TAU;
    let norm = |x: f64| x.rem_euclid(tau);
    let ccw = norm(wm - w1) < norm(w2 - w1);
    if ccw {
        while w2 <= w1 {
            w2 += tau;
        }
    } else {
        while w2 >= w1 {
            w2 -= tau;
        }
    }
    let n = ((w2 - w1).abs() / tau * 48.).ceil().max(2.) as usize;
    (0..=n)
        .map(|k| {
            let t = w1 + (w2 - w1) * k as f64 / n as f64;
            [ux + r * t.cos(), uy + r * t.sin()]
        })
        .collect()
}
fn profil_flaeche(ringe: &[Vec<[f64; 2]>]) -> f64 {
    let f = |r: &Vec<[f64; 2]>| {
        let mut s = 0.;
        for k in 0..r.len() {
            let n = (k + 1) % r.len();
            s += r[k][0] * r[n][1] - r[n][0] * r[k][1];
        }
        (s / 2.).abs()
    };
    let mut a = ringe.first().map(f).unwrap_or(0.);
    for r in ringe.iter().skip(1) {
        a -= f(r);
    }
    a.max(0.)
}

/// Waagerechte Ringe (erster außen) als bereinigtes Mehrfachpolygon.
fn polygon_aus(ringe: &[Vec<V3>]) -> Option<MultiPolygon<f64>> {
    let ls = |r: &Vec<V3>| LineString::from(r.iter().map(|p| (p[0], p[1])).collect::<Vec<_>>());
    let ring = |r: &Vec<V3>| Polygon::new(ls(r), vec![]).orient(geo::orient::Direction::Default);
    let mut m = MultiPolygon(vec![ring(ringe.first()?)]).union(&MultiPolygon::<f64>(vec![]));
    for r in ringe.iter().skip(1) {
        m = m.difference(&MultiPolygon(vec![ring(r)]));
    }
    (m.unsigned_area() > 1e-9).then_some(m)
}

/// Vereinigt Polygone robust (auch sich überlappende oder falsch orientierte).
pub fn vereinen(polys: Vec<Polygon<f64>>) -> MultiPolygon<f64> {
    if polys.is_empty() {
        return MultiPolygon(vec![]);
    }
    // Einheitlich ausrichten: gegenläufige Ringe hebt die Vereinigung sonst auf.
    let polys: Vec<Polygon<f64>> = polys
        .into_iter()
        .map(|p| p.orient(geo::orient::Direction::Default))
        .collect();
    // Kleine Gruppen zuerst vereinen: hält die Zwischenergebnisse klein.
    let mut ebene: Vec<MultiPolygon<f64>> = polys
        .chunks(64)
        .map(|c| geo::unary_union(c.iter()))
        .collect();
    while ebene.len() > 1 {
        ebene = ebene
            .chunks(2)
            .map(|p| {
                if p.len() == 2 {
                    p[0].union(&p[1])
                } else {
                    p[0].clone()
                }
            })
            .collect();
    }
    ebene.pop().unwrap_or(MultiPolygon(vec![]))
}

/// Mehrfachpolygon in Planpolygone (Meter, Weltsystem).
pub fn planpolygone(m: &MultiPolygon<f64>) -> Vec<Planpolygon> {
    let ring = |l: &LineString<f64>| -> Vec<Planpunkt> {
        let mut v: Vec<Planpunkt> = l.coords().map(|c| [rund(c.x), rund(c.y)]).collect();
        if v.len() > 1 && v.first() == v.last() {
            v.pop();
        }
        v
    };
    m.0.iter()
        .filter(|p| p.unsigned_area() > 1e-6)
        .map(|p| Planpolygon {
            aussen: ring(p.exterior()),
            innen: p
                .interiors()
                .iter()
                .filter(|i| Polygon::new((*i).clone(), vec![]).unsigned_area() > 1e-6)
                .map(ring)
                .collect(),
        })
        .collect()
}
/// Auf 0,1 mm runden: hält Ergebnisse stabil und Dateien klein.
pub fn rund(v: f64) -> f64 {
    (v * 10_000.).round() / 10_000.
}

pub fn multipolygon(ps: &[Planpolygon]) -> MultiPolygon<f64> {
    let ls =
        |r: &Vec<Planpunkt>| LineString::from(r.iter().map(|p| (p[0], p[1])).collect::<Vec<_>>());
    MultiPolygon(
        ps.iter()
            .filter(|p| p.aussen.len() >= 3)
            .map(|p| Polygon::new(ls(&p.aussen), p.innen.iter().map(ls).collect()))
            .collect(),
    )
}
