//! Planlayout: eigene Zeichnungen maßstäblich auf DIN-Blätter mit Plankopf,
//! Nordpfeil, Maßstabsleiste und Legende; Plankopf-Stempel für PDF-Pläne aus
//! dem Büro-CAD. Der Plankopf liest Projekt, Bauherren, Flurstücke, Büro und
//! die Planliste (Plannummer, Index); nichts davon wird hier doppelt gepflegt.
use crate::{
    CoreError, CoreResult, CoreServices,
    benutzer::Benutzer,
    domain::{DocumentId, DocumentVersion, ProjectId},
    druckausgabe::planlayout::{
        Fuellung, Legendeneintrag, Planblatt, Plankopf, Planlinie, Plantext, Stift, blatt, stempeln,
    },
    planaufmass::{
        Darstellung, Linienart, Oeffnung, Oeffnungsart, Planaufmass, Plangeometrie, Plansymbol,
        Schraffur, Zeichenform,
    },
    plattform::ObjektArt,
    zugriff::ProjektRecht,
};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// DIN-A-Formate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Blattformat {
    A4,
    A3,
    A2,
    A1,
    A0,
}
impl Blattformat {
    /// Breite × Höhe im Hochformat (mm).
    pub fn mm(self) -> (f64, f64) {
        match self {
            Self::A4 => (210., 297.),
            Self::A3 => (297., 420.),
            Self::A2 => (420., 594.),
            Self::A1 => (594., 841.),
            Self::A0 => (841., 1189.),
        }
    }
    pub fn titel(self) -> &'static str {
        match self {
            Self::A4 => "A4",
            Self::A3 => "A3",
            Self::A2 => "A2",
            Self::A1 => "A1",
            Self::A0 => "A0",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Layoutwunsch {
    /// Eigene Zeichnung (Planaufmaß ohne Unterlage).
    pub aufmass_id: Uuid,
    #[ts(type = "number")]
    pub revision: i64,
    pub format: Blattformat,
    pub hochformat: bool,
    /// Maßstabszahl, z. B. 100 für 1 : 100.
    pub massstab: u32,
    /// Nordpfeil: Richtung Norden in Grad gegen den Uhrzeigersinn; `None` = ohne.
    pub nordwinkel_grad: Option<f64>,
    /// Planinhalt im Plankopf; leer = Titel aus der Planliste bzw. Name der Zeichnung.
    pub planinhalt: String,
    /// Plan der Planliste, dessen Nummer und Index der Plankopf trägt; mit
    /// `ablegen` wird die Ausgabe neue Version dieses Dokuments.
    pub dokument_id: Option<DocumentId>,
    pub legende: bool,
    pub nur_sichtbare_ebenen: bool,
    /// Ausgabe zusätzlich im Projekt ablegen.
    pub ablegen: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Stempelwunsch {
    pub dokument_id: DocumentId,
    #[ts(type = "number")]
    pub version: i64,
    pub seite: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Wolkenwunsch {
    pub dokument_id: DocumentId,
    #[ts(type = "number")]
    pub version: i64,
    pub seite: u32,
    /// Bereiche als [x, y, Breite, Höhe] in Seitenpunkten (links oben, y nach unten).
    pub bereiche: Vec<[f64; 4]>,
    /// Index im Dreieck neben jeder Wolke, z. B. „b“.
    pub index: String,
}

const MM_JE_M: f64 = 1000.;

fn farbe(hex: Option<&str>, standard: [f64; 3]) -> [f64; 3] {
    let Some(h) = hex.filter(|h| h.len() == 7 && h.starts_with('#')) else {
        return standard;
    };
    let k = |i: usize| {
        u8::from_str_radix(&h[i..i + 2], 16)
            .map(|v| v as f64 / 255.)
            .unwrap_or(0.)
    };
    [k(1), k(3), k(5)]
}
/// Wirksame Darstellung: Element vor Ebene vor Standard.
fn stift(element: Option<&Darstellung>, ebene: Option<&Darstellung>, standard_mm: f64) -> Stift {
    let feld = |f: fn(&Darstellung) -> Option<&Linienart>| {
        element.and_then(f).or_else(|| ebene.and_then(f))
    };
    let breite = element
        .and_then(|d| d.stift_mm)
        .or_else(|| ebene.and_then(|d| d.stift_mm))
        .unwrap_or(standard_mm);
    let f = element
        .and_then(|d| d.farbe.as_deref())
        .or_else(|| ebene.and_then(|d| d.farbe.as_deref()))
        .or_else(|| element.and_then(|d| d.bauzustand).map(|b| b.farbe()))
        .or_else(|| ebene.and_then(|d| d.bauzustand).map(|b| b.farbe()));
    let muster = match feld(|d| d.linienart.as_ref()) {
        Some(Linienart::Strichlinie) => vec![3., 1.5],
        Some(Linienart::Strichpunktlinie) => vec![6., 1.2, 0.3, 1.2],
        Some(Linienart::Punktlinie) => vec![0.3, 1.],
        _ => vec![],
    };
    Stift {
        breite_mm: breite,
        farbe: farbe(f, [0.1, 0.1, 0.1]),
        muster,
    }
}
fn fuellung(element: Option<&Darstellung>, ebene: Option<&Darstellung>) -> Fuellung {
    let s = element
        .and_then(|d| d.schraffur)
        .or_else(|| ebene.and_then(|d| d.schraffur));
    let linien = |w: f64, a: f64, k: bool| Fuellung::Linien {
        winkel_grad: w,
        abstand_mm: a,
        kreuz: k,
    };
    match s {
        None | Some(Schraffur::Keine) => Fuellung::Keine,
        Some(Schraffur::Voll) => Fuellung::Voll(0.35),
        Some(Schraffur::Diagonal) => linien(45., 2., false),
        Some(Schraffur::Kreuz) => linien(45., 2., true),
        Some(Schraffur::Mauerwerk) => linien(45., 1., false),
        Some(Schraffur::Beton) => linien(45., 3., true),
        Some(Schraffur::Daemmung) => linien(0., 1.5, false),
        Some(Schraffur::Erdreich) => linien(135., 2., false),
    }
}
fn ringe(g: &Plangeometrie) -> (Vec<Vec<[f64; 2]>>, bool) {
    match g {
        Plangeometrie::Flaeche { polygone } => (
            polygone
                .iter()
                .flat_map(|p| std::iter::once(p.aussen.clone()).chain(p.innen.clone()))
                .collect(),
            true,
        ),
        Plangeometrie::Polygon { punkte } => (vec![punkte.clone()], true),
        Plangeometrie::Rechteck { von: a, bis: b } => {
            (vec![vec![*a, [b[0], a[1]], *b, [a[0], b[1]]]], true)
        }
        Plangeometrie::Kreis { mitte, rand } => {
            let r = (rand[0] - mitte[0]).hypot(rand[1] - mitte[1]);
            (
                vec![
                    (0..96)
                        .map(|i| {
                            let a = i as f64 * std::f64::consts::TAU / 96.;
                            [mitte[0] + r * a.cos(), mitte[1] + r * a.sin()]
                        })
                        .collect(),
                ],
                true,
            )
        }
        Plangeometrie::Strecke { von, bis } => (vec![vec![*von, *bis]], false),
        Plangeometrie::Hoehenpunkt { punkt: p } => (
            vec![
                vec![[p[0] - 0.15, p[1]], [p[0] + 0.15, p[1]]],
                vec![[p[0], p[1] - 0.15], [p[0], p[1] + 0.15]],
            ],
            false,
        ),
    }
}
/// Wand mit Öffnungen: geschlossene Wandstücke zwischen den Öffnungen sowie
/// Linien für Fensterglas, Türblatt und Öffnungsbogen. `m_je_einheit` rechnet
/// Öffnungsmaße (m) in Seitenkoordinaten um (eigene Zeichnung: 1).
pub struct Wandbild {
    pub stuecke: Vec<Vec<[f64; 2]>>,
    pub linien: Vec<Vec<[f64; 2]>>,
}
pub fn wandbild(
    von: [f64; 2],
    bis: [f64; 2],
    dicke_m: f64,
    oeffnungen: &[Oeffnung],
    m_je_einheit: f64,
) -> Wandbild {
    let l = (bis[0] - von[0]).hypot(bis[1] - von[1]).max(1e-9);
    let u = [(bis[0] - von[0]) / l, (bis[1] - von[1]) / l];
    let d = dicke_m / m_je_einheit / 2.;
    let n = [-u[1] * d, u[0] * d];
    let at = |t: f64, s: f64| [von[0] + u[0] * t + n[0] * s, von[1] + u[1] * t + n[1] * s];
    let mut os: Vec<_> = oeffnungen.iter().collect();
    os.sort_by(|a, b| a.lage_m.total_cmp(&b.lage_m));
    let mut stuecke = Vec::new();
    let mut linien = Vec::new();
    let mut t0 = 0.;
    for o in os {
        let a = (o.lage_m / m_je_einheit).clamp(t0, l);
        let b = ((o.lage_m + o.breite_m) / m_je_einheit).clamp(a, l);
        if a > t0 + 1e-9 {
            stuecke.push(vec![at(t0, 1.), at(a, 1.), at(a, -1.), at(t0, -1.)]);
        }
        match o.art {
            Oeffnungsart::Fenster => {
                for s in [1., 0.25, -0.25, -1.] {
                    linien.push(vec![at(a, s), at(b, s)]);
                }
            }
            Oeffnungsart::Tuer => {
                // Drehflügel am Öffnungsanfang, Bogen bis zum Öffnungsende.
                let s = if o.links { 1. } else { -1. };
                let w = b - a;
                let ns = [n[0] / d.max(1e-12) * s, n[1] / d.max(1e-12) * s];
                let h = at(a, s);
                linien.push(vec![h, [h[0] + ns[0] * w, h[1] + ns[1] * w]]);
                linien.push(
                    (0..=16)
                        .map(|i| {
                            let phi = i as f64 / 16. * std::f64::consts::FRAC_PI_2;
                            [
                                h[0] + (ns[0] * phi.cos() + u[0] * phi.sin()) * w,
                                h[1] + (ns[1] * phi.cos() + u[1] * phi.sin()) * w,
                            ]
                        })
                        .collect(),
                );
            }
        }
        t0 = b;
    }
    if l > t0 + 1e-9 {
        stuecke.push(vec![at(t0, 1.), at(l, 1.), at(l, -1.), at(t0, -1.)]);
    }
    Wandbild { stuecke, linien }
}
/// Linienzüge eines Planzeichens: (Punkte, geschlossen, gefüllt).
pub fn symbolbild(
    s: Plansymbol,
    p: [f64; 2],
    g: f64,
    grad: f64,
) -> Vec<(Vec<[f64; 2]>, bool, bool)> {
    let (sn, cs) = (-grad.to_radians()).sin_cos();
    let at = |x: f64, y: f64| [p[0] + (x * cs - y * sn) * g, p[1] + (x * sn + y * cs) * g];
    let kreis = |r: f64| {
        (0..32)
            .map(|i| {
                let a = i as f64 / 32. * std::f64::consts::TAU;
                at(r * a.cos(), r * a.sin())
            })
            .collect::<Vec<_>>()
    };
    match s {
        Plansymbol::BaumBestand => vec![(kreis(0.5), true, false), (kreis(0.05), true, true)],
        Plansymbol::BaumNeu => vec![(kreis(0.5), true, false), (kreis(0.2), true, false)],
        Plansymbol::BaumFaellung => vec![
            (kreis(0.5), true, false),
            (vec![at(-0.45, -0.45), at(0.45, 0.45)], false, false),
            (vec![at(-0.45, 0.45), at(0.45, -0.45)], false, false),
        ],
        Plansymbol::Stellplatz => vec![
            (
                vec![
                    at(-0.25, -0.5),
                    at(0.25, -0.5),
                    at(0.25, 0.5),
                    at(-0.25, 0.5),
                ],
                true,
                false,
            ),
            (
                vec![
                    at(-0.08, 0.15),
                    at(-0.08, -0.15),
                    at(0.05, -0.15),
                    at(0.1, -0.08),
                    at(0.05, 0.),
                    at(-0.08, 0.),
                ],
                false,
                false,
            ),
        ],
        Plansymbol::Zufahrt => vec![
            (vec![at(0., 0.5), at(0., -0.5)], false, false),
            (
                vec![at(-0.2, -0.25), at(0., -0.5), at(0.2, -0.25)],
                true,
                true,
            ),
        ],
        Plansymbol::Hauseingang => {
            vec![(
                vec![at(-0.3, 0.25), at(0.3, 0.25), at(0., -0.25)],
                true,
                true,
            )]
        }
        Plansymbol::Grenzpunkt => vec![(kreis(0.15), true, false), (kreis(0.04), true, true)],
    }
}
fn zahl(v: f64) -> String {
    let s = format!("{v:.2}");
    s.replace('.', ",")
}
/// Zeichnung → Planblatt-Inhalte (Linien, Texte, Legende) in Blattmetern.
pub fn inhalte(
    p: &Planaufmass,
    massstab: f64,
    // Meter je Blatteinheit: 1 bei eigenen Zeichnungen, sonst aus der Kalibrierung.
    faktor: f64,
    nur_sichtbare: bool,
    raeume: &std::collections::HashMap<Uuid, (String, String, f64)>,
) -> (Vec<Planlinie>, Vec<Plantext>, Vec<Legendeneintrag>) {
    let ebene = |id: Uuid| p.ebenen.iter().find(|e| e.id == id);
    let zeigen = |id: Uuid| ebene(id).is_some_and(|e| !nur_sichtbare || e.sichtbar);
    let mut linien = Vec::new();
    let mut texte = Vec::new();
    let ebenenname = |id: Uuid| ebene(id).map(|e| e.bezeichnung.clone()).unwrap_or_default();
    for m in p.markierungen.iter().filter(|m| zeigen(m.ebene_id)) {
        let name = ebenenname(m.ebene_id);
        let d = ebene(m.ebene_id).and_then(|e| e.darstellung.as_ref());
        let (rs, zu) = ringe(&m.geometrie);
        for r in rs {
            linien.push(Planlinie {
                ebene: name.clone(),
                punkte: r,
                geschlossen: zu,
                stift: stift(None, d, 0.35),
                fuellung: if zu {
                    fuellung(None, d)
                } else {
                    Fuellung::Keine
                },
            });
        }
    }
    for e in p.elemente.iter().filter(|e| zeigen(e.ebene_id)) {
        let name = ebenenname(e.ebene_id);
        let d = ebene(e.ebene_id).and_then(|l| l.darstellung.as_ref());
        let s = stift(e.darstellung.as_ref(), d, 0.5);
        match &e.form {
            Zeichenform::Wand {
                von,
                bis,
                dicke_m,
                oeffnungen,
            } => {
                let w = wandbild(*von, *bis, *dicke_m, oeffnungen, faktor);
                for k in w.stuecke {
                    linien.push(Planlinie {
                        ebene: name.clone(),
                        punkte: k,
                        geschlossen: true,
                        stift: s.clone(),
                        fuellung: fuellung(e.darstellung.as_ref(), d),
                    });
                }
                for l in w.linien {
                    linien.push(Planlinie {
                        ebene: name.clone(),
                        punkte: l,
                        geschlossen: false,
                        stift: Stift {
                            breite_mm: 0.18,
                            ..s.clone()
                        },
                        fuellung: Fuellung::Keine,
                    });
                }
            }
            Zeichenform::Masskette { von, bis, abstand } => {
                // Abstand in Seitenpunkten der Zeichnung = Meter.
                let l = (bis[0] - von[0]).hypot(bis[1] - von[1]).max(1e-9);
                let u = [(bis[0] - von[0]) / l, (bis[1] - von[1]) / l];
                let n = [-u[1] * abstand, u[0] * abstand];
                let a = [von[0] + n[0], von[1] + n[1]];
                let b = [bis[0] + n[0], bis[1] + n[1]];
                let fein = Stift {
                    breite_mm: 0.18,
                    ..s.clone()
                };
                linien.push(Planlinie {
                    ebene: name.clone(),
                    punkte: vec![a, b],
                    geschlossen: false,
                    stift: fein.clone(),
                    fuellung: Fuellung::Keine,
                });
                // Hilfslinien und Schrägstriche (1,5 mm auf dem Papier).
                let t = 1.5 / MM_JE_M * massstab / faktor;
                for (q, h) in [(von, a), (bis, b)] {
                    linien.push(Planlinie {
                        ebene: name.clone(),
                        punkte: vec![*q, h],
                        geschlossen: false,
                        stift: fein.clone(),
                        fuellung: Fuellung::Keine,
                    });
                    let w = [(u[0] - u[1]) * t, (u[1] + u[0]) * t];
                    linien.push(Planlinie {
                        ebene: name.clone(),
                        punkte: vec![[h[0] - w[0], h[1] - w[1]], [h[0] + w[0], h[1] + w[1]]],
                        geschlossen: false,
                        stift: Stift {
                            breite_mm: 0.35,
                            ..s.clone()
                        },
                        fuellung: Fuellung::Keine,
                    });
                }
                let mut grad = -(u[1].atan2(u[0])).to_degrees();
                if grad > 90. || grad <= -90. {
                    grad += 180.;
                }
                let mitte = [(a[0] + b[0]) / 2., (a[1] + b[1]) / 2.];
                texte.push(Plantext {
                    ebene: name.clone(),
                    punkt: [mitte[0] - u[0] * t * 4., mitte[1] - u[1] * t * 4.],
                    text: zahl(l * faktor),
                    hoehe_mm: 2.5,
                    drehung_grad: grad,
                    farbe: s.farbe,
                });
            }
            Zeichenform::Text {
                punkt,
                inhalt,
                hoehe,
                drehung_grad,
            } => texte.push(Plantext {
                ebene: name.clone(),
                punkt: *punkt,
                text: inhalt.clone(),
                hoehe_mm: hoehe * faktor * MM_JE_M / massstab,
                drehung_grad: *drehung_grad,
                farbe: s.farbe,
            }),
            Zeichenform::Symbol {
                punkt,
                symbol,
                groesse,
                drehung_grad,
            } => {
                for (ps, zu, voll) in symbolbild(*symbol, *punkt, *groesse, *drehung_grad) {
                    linien.push(Planlinie {
                        ebene: name.clone(),
                        punkte: ps,
                        geschlossen: zu,
                        stift: s.clone(),
                        fuellung: if voll {
                            Fuellung::Voll(1.)
                        } else {
                            Fuellung::Keine
                        },
                    });
                }
            }
            Zeichenform::Raumstempel {
                punkt,
                raum_id,
                hoehe,
                ..
            } => {
                let (kopf, flaeche) = match raeume.get(raum_id) {
                    Some((nr, name, f)) => (
                        [nr.as_str(), name.as_str()]
                            .into_iter()
                            .filter(|t| !t.is_empty())
                            .collect::<Vec<_>>()
                            .join(" "),
                        format!("{} m²", zahl(*f)),
                    ),
                    None => ("Raum fehlt im Raumbuch".into(), String::new()),
                };
                let h = hoehe * faktor * MM_JE_M / massstab;
                texte.push(Plantext {
                    ebene: name.clone(),
                    punkt: *punkt,
                    text: kopf,
                    hoehe_mm: h,
                    drehung_grad: 0.,
                    farbe: s.farbe,
                });
                texte.push(Plantext {
                    ebene: name.clone(),
                    punkt: [punkt[0], punkt[1] + hoehe * 1.4],
                    text: flaeche,
                    hoehe_mm: h * 0.85,
                    drehung_grad: 0.,
                    farbe: s.farbe,
                });
            }
        }
    }
    let legende = p
        .ebenen
        .iter()
        .filter(|e| !nur_sichtbare || e.sichtbar)
        .filter(|e| {
            p.markierungen.iter().any(|m| m.ebene_id == e.id)
                || p.elemente.iter().any(|x| x.ebene_id == e.id)
        })
        .map(|e| Legendeneintrag {
            name: e.bezeichnung.clone(),
            stift: stift(None, e.darstellung.as_ref(), 0.35),
            fuellung: fuellung(None, e.darstellung.as_ref()),
        })
        .collect();
    (linien, texte, legende)
}

impl CoreServices {
    /// Raumnummer, Name und Fläche aller Räume der Raumbücher (für Raumstempel).
    pub async fn raumstempel_daten(
        &self,
        pid: ProjectId,
    ) -> CoreResult<std::collections::HashMap<Uuid, (String, String, f64)>> {
        let mut m = std::collections::HashMap::new();
        for f in self.fachobjekte(pid, ObjektArt::Raumbuch).await? {
            if let Ok(r) = serde_json::from_value::<crate::plattform::objekte::Raumbuch>(f.daten) {
                for raum in r.raeume {
                    m.insert(raum.id, (raum.nummer, raum.bezeichnung, raum.flaeche_m2));
                }
            }
        }
        Ok(m)
    }
    /// Plankopf aus Projekt, Bauherren, Flurstücken, Büro und Planliste.
    pub async fn plankopf(
        &self,
        pid: ProjectId,
        dokument: Option<DocumentId>,
        version: Option<i64>,
        planinhalt: &str,
        massstab: Option<u32>,
        format: &str,
    ) -> CoreResult<Plankopf> {
        let projekt = self.repos.projects.get(pid).await?;
        let bauherr = self
            .bauherren(pid)
            .await?
            .iter()
            .map(|k| k.anzeigename())
            .collect::<Vec<_>>()
            .join("; ");
        let antrag = self.bauantrag(pid).await.ok();
        let vorhaben = antrag
            .as_ref()
            .map(|a| a.antrag.bezeichnung.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| projekt.name.clone());
        let grundstueck = crate::grundstuecksinformationen::informationen(&projekt.planungsrecht)
            .vorhaben()
            .map(|f| f.bezeichnung())
            .collect::<Vec<_>>()
            .join("; ");
        let buero = self.buero().await.ok();
        let planverfasser = match &buero {
            Some(b) => {
                let ev = antrag
                    .as_ref()
                    .and_then(|a| a.antrag.entwurfsverfasser_von(b))
                    .or_else(|| b.entwurfsverfasser.first());
                [
                    b.name.trim().to_string(),
                    ev.map(|e| e.anzeigename()).unwrap_or_default(),
                ]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" · ")
            }
            None => String::new(),
        };
        let eintrag = match dokument {
            Some(d) => self
                .planeintraege(pid)
                .await?
                .into_iter()
                .map(|(_, e)| e)
                .find(|e| e.dokument_id == d),
            None => None,
        };
        let revision = eintrag.as_ref().and_then(|e| match version {
            Some(v) => e.revisionen.iter().rev().find(|r| r.version <= v),
            None => e.revisionen.last(),
        });
        let massstab = massstab.or(eintrag.as_ref().and_then(|e| e.massstab));
        Ok(Plankopf {
            bauherr,
            vorhaben,
            grundstueck,
            planinhalt: if planinhalt.trim().is_empty() {
                eintrag
                    .as_ref()
                    .map(|e| e.titel.clone())
                    .unwrap_or_default()
            } else {
                planinhalt.trim().to_string()
            },
            planverfasser,
            plannummer: eintrag
                .as_ref()
                .map(|e| e.plannummer.clone())
                .unwrap_or_default(),
            index: revision.map(|r| r.index.clone()).unwrap_or_default(),
            massstab: massstab.map(|m| format!("1 : {m}")).unwrap_or_default(),
            datum: revision
                .map(|r| r.datum)
                .unwrap_or_else(|| chrono::Local::now().date_naive())
                .format("%d.%m.%Y")
                .to_string(),
            format: format.into(),
        })
    }

    /// Planblatt einer eigenen Zeichnung als PDF; optional im Projekt abgelegt
    /// (als neue Version des gewählten Plans der Planliste oder als neues Dokument).
    pub async fn planlayout(
        &self,
        pid: ProjectId,
        w: &Layoutwunsch,
        wer: &Benutzer,
    ) -> CoreResult<(Vec<u8>, Option<DocumentVersion>)> {
        self.projekt_zugriff(pid, wer, ProjektRecht::Lesen).await?;
        if !(1..=100_000).contains(&w.massstab) {
            return Err(CoreError::InvalidInput(
                "Maßstab zwischen 1 und 100000".into(),
            ));
        }
        if w.nordwinkel_grad.is_some_and(|v| !v.is_finite()) {
            return Err(CoreError::InvalidInput("Ungültiger Nordwinkel".into()));
        }
        crate::plattform::text(&w.planinhalt, 300).or_else(|e| {
            if w.planinhalt.is_empty() {
                Ok(())
            } else {
                Err(e)
            }
        })?;
        let f = self
            .fachobjekt_lesen(pid, ObjektArt::Planaufmass, w.aufmass_id)
            .await?
            .ok_or_else(|| CoreError::NotFound("Zeichnung nicht vorhanden".into()))?;
        if f.revision != w.revision {
            return Err(CoreError::VersionConflict {
                expected: w.revision,
                actual: f.revision,
            });
        }
        let p: Planaufmass =
            serde_json::from_value(f.daten).map_err(|e| CoreError::InvalidInput(e.to_string()))?;
        if !p.ist_zeichnung() {
            return Err(CoreError::InvalidInput(
                "Das Planlayout gibt eigene Zeichnungen aus. Für vorhandene PDF-Pläne „Plankopf stempeln“ in der Planliste verwenden.".into(),
            ));
        }
        p.messen()?;
        let (b, h) = w.format.mm();
        let (breite_mm, hoehe_mm) = if w.hochformat { (b, h) } else { (h, b) };
        let massstab = w.massstab as f64;
        let raeume = self.raumstempel_daten(pid).await?;
        let (linien, texte, legende) = inhalte(&p, massstab, 1., w.nur_sichtbare_ebenen, &raeume);
        let planinhalt = if w.planinhalt.trim().is_empty() && w.dokument_id.is_none() {
            p.bezeichnung.clone()
        } else {
            w.planinhalt.clone()
        };
        let kopf = self
            .plankopf(
                pid,
                w.dokument_id,
                None,
                &planinhalt,
                Some(w.massstab),
                w.format.titel(),
            )
            .await?;
        let pdf = blatt(&Planblatt {
            breite_mm,
            hoehe_mm,
            massstab,
            linien,
            texte,
            kopf: kopf.clone(),
            nordwinkel_grad: w.nordwinkel_grad,
            legende: if w.legende { legende } else { vec![] },
        })?;
        let abgelegt = if w.ablegen {
            self.projekt_zugriff(pid, wer, ProjektRecht::Bearbeiten)
                .await?;
            let (basis, name) = match w.dokument_id {
                Some(d) => {
                    let v = self.repos.documents.latest(d).await?;
                    (Some(v.version), v.file_name)
                }
                None => (
                    None,
                    format!(
                        "{}.pdf",
                        if kopf.plannummer.is_empty() {
                            &kopf.planinhalt
                        } else {
                            &kopf.plannummer
                        }
                    ),
                ),
            };
            Some(
                self.store_document(
                    pid,
                    w.dokument_id,
                    basis,
                    &name,
                    Bytes::from(pdf.clone()),
                    wer,
                )
                .await?,
            )
        } else {
            None
        };
        Ok((pdf, abgelegt))
    }

    /// DXF einer eigenen Zeichnung oder eines kalibrierten Aufmaßes (Meter, y nach oben).
    pub async fn planaufmass_dxf(
        &self,
        pid: ProjectId,
        aufmass_id: Uuid,
        revision: i64,
        nur_sichtbare_ebenen: bool,
        wer: &Benutzer,
    ) -> CoreResult<Vec<u8>> {
        self.projekt_zugriff(pid, wer, ProjektRecht::Lesen).await?;
        let f = self
            .fachobjekt_lesen(pid, ObjektArt::Planaufmass, aufmass_id)
            .await?
            .ok_or_else(|| CoreError::NotFound("Aufmaß nicht vorhanden".into()))?;
        if f.revision != revision {
            return Err(CoreError::VersionConflict {
                expected: revision,
                actual: f.revision,
            });
        }
        let p: Planaufmass =
            serde_json::from_value(f.daten).map_err(|e| CoreError::InvalidInput(e.to_string()))?;
        p.messen()?;
        let faktor = if p.ist_zeichnung() {
            1.
        } else {
            p.kalibrierung_wirksam()
                .ok_or_else(|| {
                    CoreError::InvalidInput("Für die DXF-Ausgabe zuerst den Maßstab setzen".into())
                })?
                .meter_pro_punkt()?
        };
        let raeume = self.raumstempel_daten(pid).await?;
        let (linien, texte, _) = inhalte(&p, 100., faktor, nur_sichtbare_ebenen, &raeume);
        crate::cad::dxfexport::schreiben(&linien, &texte, faktor, 100.)
    }
    /// Revisionswolken um geänderte Bereiche einer Planseite (Seitenpunkte, links
    /// oben, y nach unten) mit Indexdreieck; liefert die markierte Datei.
    pub async fn plan_revisionswolken(
        &self,
        pid: ProjectId,
        w: &Wolkenwunsch,
        wer: &Benutzer,
    ) -> CoreResult<Vec<u8>> {
        self.projekt_zugriff(pid, wer, ProjektRecht::Lesen).await?;
        if w.bereiche.is_empty() || w.bereiche.len() > 200 {
            return Err(CoreError::InvalidInput(
                "1 bis 200 Änderungsbereiche angeben".into(),
            ));
        }
        if w.bereiche.iter().flatten().any(|v| !v.is_finite()) || w.index.chars().count() > 10 {
            return Err(CoreError::InvalidInput(
                "Ungültiger Bereich oder Index".into(),
            ));
        }
        let (d, bytes) = self.load_version(w.dokument_id, w.version).await?;
        if d.project_id != pid {
            return Err(CoreError::NotFound("Dokument nicht im Projekt".into()));
        }
        crate::druckausgabe::planlayout::wolken(&bytes, w.seite.max(1), &w.bereiche, &w.index)
    }
    /// Plankopf auf einer Seite eines PDF-Plans; liefert die gestempelte Datei.
    /// Das Original im DMS bleibt unverändert.
    pub async fn plankopf_stempeln(
        &self,
        pid: ProjectId,
        w: &Stempelwunsch,
        wer: &Benutzer,
    ) -> CoreResult<Vec<u8>> {
        self.projekt_zugriff(pid, wer, ProjektRecht::Lesen).await?;
        let (d, bytes) = self.load_version(w.dokument_id, w.version).await?;
        if d.project_id != pid {
            return Err(CoreError::NotFound("Dokument nicht im Projekt".into()));
        }
        if !d.file_name.to_lowercase().ends_with(".pdf") {
            return Err(CoreError::InvalidInput("Plankopf nur auf PDF-Pläne".into()));
        }
        let kopf = self
            .plankopf(pid, Some(w.dokument_id), Some(w.version), "", None, "")
            .await?;
        stempeln(&bytes, w.seite.max(1), &kopf)
    }
}
