//! IFC schreiben, auf zwei Wegen:
//!
//! * **Nacharbeit zurückschreiben** ([`nacharbeit_schreiben`]): Die Datei der
//!   Modellakte wird mit ihrer Nacharbeit neu geschrieben, im Schema der
//!   Quelle. Unberührte Instanzen bleiben zeichengleich; geänderte Elemente
//!   werden ersetzt, neue Räume, Eigenschaften und Zuordnungen angehängt,
//!   ausgeblendete Elemente entfernt.
//! * **Eigenes IFC4-Modell** ([`entwurf_schreiben`]): Gebäude mit Geschossen,
//!   Wänden, Decken und Räumen aus den eigenen Zeichnungen. Die GlobalIds
//!   leiten sich aus den IDs der Zeichnungsobjekte ab und bleiben bei jedem
//!   Export gleich, damit ein Reimport beim Empfänger zuordnen kann.
//!
//! Beide Wege lesen die geschriebene Datei zur Selbstkontrolle wieder ein
//! ([`kontrolle`]).
use super::nacharbeit::{Eigenschaftswert, Modellaenderung};
use super::step::{Instanz, StepDatei, Wert, global_id, verschluesseln};
use super::{Modell, Modellauszug, klassentitel};
use crate::planaufmass::Planpolygon;
use crate::{CoreError, CoreResult};
use geo::Area;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use uuid::Uuid;

/// Was der Export geschrieben hat und was die Selbstkontrolle fand.
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Exportbericht {
    pub dateiname: String,
    pub schema: String,
    /// Elemente der geschriebenen Datei (wie der Auszug sie zählt).
    pub elemente: usize,
    pub geaendert: usize,
    pub neu: usize,
    pub entfernt: usize,
    pub hinweise: Vec<String>,
    /// Abweichungen der wieder eingelesenen Datei vom erwarteten Modell.
    pub abweichungen: Vec<String>,
}

fn fehler(s: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(s.into())
}

fn kopfzeilen(schema: &str, dateiname: &str, zeit: &str, quelle: Option<&str>) -> String {
    let sicht = if schema.starts_with("IFC2X3") {
        "ViewDefinition [CoordinationView_V2.0]"
    } else {
        "ViewDefinition [ReferenceView]"
    };
    format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(({}),'2;1');\nFILE_NAME({},{},(''),(''),{},{},'');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\n",
        verschluesseln(sicht),
        verschluesseln(dateiname),
        verschluesseln(zeit),
        verschluesseln("BauProjektIO"),
        verschluesseln(quelle.unwrap_or("BauProjektIO")),
    )
}

/// Attributzahl einer Bauteilklasse im Schema und Stelle von `PredefinedType`
/// (IFC2X3: `ShapeType` bei Dach, Treppe, Rampe). `None`: Klasse fehlt im Schema.
fn aufbau(schema: &str, klasse: &str) -> Option<(usize, Option<usize>)> {
    let x3 = schema.starts_with("IFC2X3");
    Some(match klasse {
        "IFCDOOR" | "IFCWINDOW" if x3 => (10, None),
        "IFCDOOR" | "IFCWINDOW" => (13, Some(10)),
        "IFCFURNISHINGELEMENT" => (8, None),
        "IFCCHIMNEY" | "IFCSHADINGDEVICE" if x3 => return None,
        "IFCBUILDINGELEMENTPROXY" if x3 => (9, None),
        "IFCWALL" | "IFCCOLUMN" | "IFCBEAM" | "IFCMEMBER" | "IFCPLATE" | "IFCCURTAINWALL" if x3 => {
            (8, None)
        }
        _ => (9, Some(8)),
    })
}

const RAEUMLICH: [&str; 4] = ["IFCSPACE", "IFCBUILDINGSTOREY", "IFCBUILDING", "IFCSITE"];

/// Arbeitsstand der Datei: ersetzte, entfernte und neue Instanzen.
struct Schreiber<'a> {
    d: &'a StepDatei,
    ersetzt: BTreeMap<u64, Instanz>,
    entfernt: HashSet<u64>,
    neu: BTreeMap<u64, Instanz>,
    naechste: u64,
    eigner: Wert,
    hinweise: Vec<String>,
    geaendert: HashSet<u64>,
}

impl<'a> Schreiber<'a> {
    fn neu(d: &'a StepDatei) -> Self {
        Self {
            d,
            ersetzt: BTreeMap::new(),
            entfernt: HashSet::new(),
            neu: BTreeMap::new(),
            naechste: d.hoechste_id() + 1,
            eigner: d
                .alle("IFCOWNERHISTORY")
                .next()
                .map_or(Wert::Null, |i| Wert::Ref(i.id)),
            hinweise: Vec::new(),
            geaendert: HashSet::new(),
        }
    }
    fn get(&self, id: u64) -> Option<Instanz> {
        if self.entfernt.contains(&id) {
            return None;
        }
        self.neu
            .get(&id)
            .or_else(|| self.ersetzt.get(&id))
            .or_else(|| self.d.get(id))
            .cloned()
    }
    fn setzen(&mut self, i: Instanz) {
        match self.neu.get_mut(&i.id) {
            Some(n) => *n = i,
            None => {
                self.ersetzt.insert(i.id, i);
            }
        }
    }
    fn add(&mut self, typ: &str, attribute: Vec<Wert>) -> u64 {
        let id = self.naechste;
        self.naechste += 1;
        self.neu.insert(
            id,
            Instanz {
                id,
                typ: typ.into(),
                attribute,
            },
        );
        id
    }
    /// IDs aller aktuellen Instanzen eines Typs (Quelle und neu).
    fn alle(&self, typ: &str) -> Vec<u64> {
        self.d
            .alle(typ)
            .map(|i| i.id)
            .chain(self.neu.values().filter(|i| i.typ == typ).map(|i| i.id))
            .filter(|id| !self.entfernt.contains(id))
            .collect()
    }
    fn gid(&self) -> Wert {
        Wert::Text(global_id(Uuid::new_v4()))
    }
    fn liste(ids: &[u64]) -> Wert {
        Wert::Liste(ids.iter().map(|i| Wert::Ref(*i)).collect())
    }
    fn punkt(&mut self, p: &[f64]) -> u64 {
        let k = Wert::Liste(p.iter().map(|v| Wert::Real(*v)).collect());
        self.add("IFCCARTESIANPOINT", vec![k])
    }
    /// Element `id` aus den Listen der Beziehungen `typ` (Attribut `liste`)
    /// nehmen, deren Bezugsobjekt (Attribut `bezug`) die Bedingung erfüllt.
    fn herausnehmen(
        &mut self,
        typ: &str,
        liste: usize,
        bezug: usize,
        id: u64,
        nur: &dyn Fn(&str) -> bool,
    ) {
        for r in self.alle(typ) {
            let Some(mut rel) = self.get(r) else { continue };
            let ziel = rel.attr(bezug).referenz().and_then(|z| self.get(z));
            if !ziel.is_some_and(|z| nur(&z.typ)) || !rel.attr(liste).referenzen().contains(&id) {
                continue;
            }
            let rest: Vec<u64> = rel
                .attr(liste)
                .referenzen()
                .into_iter()
                .filter(|x| *x != id)
                .collect();
            if rest.is_empty() {
                self.entfernt.insert(r);
            } else {
                rel.attribute[liste] = Self::liste(&rest);
                self.setzen(rel);
            }
        }
    }

    fn klassifizieren(&mut self, id: u64, klasse: &str, art: Option<&str>) {
        let Some(alt) = self.get(id) else { return };
        let schema = self.d.schema.clone();
        let (typ, objekttyp) = match aufbau(&schema, klasse) {
            Some(_) => (klasse.to_string(), None),
            None => {
                self.hinweise.push(format!(
                    "{} gibt es in {schema} nicht; als unklassifiziertes Bauteil mit Objekttyp „{}“ geschrieben.",
                    klassentitel(klasse),
                    klassentitel(klasse)
                ));
                (
                    "IFCBUILDINGELEMENTPROXY".to_string(),
                    Some(klassentitel(klasse)),
                )
            }
        };
        let (n, stelle) = aufbau(&schema, &typ).unwrap_or((9, None));
        let mut a: Vec<Wert> = (0..8).map(|i| alt.attr(i).clone()).collect();
        if let Some(o) = objekttyp {
            a[4] = Wert::Text(o);
        }
        let tuer = |t: &str| t == "IFCDOOR" || t == "IFCWINDOW";
        a.resize(n, Wert::Null);
        if tuer(&alt.typ) && tuer(&typ) {
            a[8] = alt.attr(8).clone();
            a[9] = alt.attr(9).clone();
        }
        if let Some(s) = stelle {
            a[s] = Wert::Enum(art.unwrap_or("NOTDEFINED").into());
        }
        if self
            .alle("IFCRELDEFINESBYTYPE")
            .iter()
            .filter_map(|r| self.get(*r))
            .any(|r| r.attr(4).referenzen().contains(&id))
        {
            self.hinweise.push(format!(
                "{} hat einen Typ der bisherigen Klasse; der Typ bleibt zugeordnet.",
                alt.attr(0).text().unwrap_or("")
            ));
        }
        self.setzen(Instanz {
            id,
            typ,
            attribute: a,
        });
    }

    fn benennen(&mut self, id: u64, name: &str, langname: Option<&str>) {
        let Some(mut i) = self.get(id) else { return };
        let text = |t: Option<&str>| {
            t.filter(|t| !t.is_empty())
                .map_or(Wert::Null, |t| Wert::Text(t.into()))
        };
        i.attribute.resize(i.attribute.len().max(8), Wert::Null);
        i.attribute[2] = text(Some(name));
        if RAEUMLICH.contains(&i.typ.as_str()) {
            i.attribute[7] = text(langname);
        } else if langname.is_some() {
            i.attribute[3] = text(langname);
        }
        self.setzen(i);
    }

    fn zuordnen(&mut self, id: u64, geschoss: u64) {
        let Some(e) = self.get(id) else { return };
        let raeumlich = |t: &str| RAEUMLICH.contains(&t);
        if e.typ == "IFCSPACE" {
            self.herausnehmen("IFCRELAGGREGATES", 5, 4, id, &raeumlich);
            let g = self.gid();
            let o = self.eigner.clone();
            self.add(
                "IFCRELAGGREGATES",
                vec![
                    g,
                    o,
                    Wert::Null,
                    Wert::Null,
                    Wert::Ref(geschoss),
                    Self::liste(&[id]),
                ],
            );
            return;
        }
        self.herausnehmen("IFCRELCONTAINEDINSPATIALSTRUCTURE", 4, 5, id, &raeumlich);
        self.enthalten(geschoss, &[id]);
    }

    /// Elemente in die Enthaltensein-Beziehung des Geschosses aufnehmen.
    fn enthalten(&mut self, geschoss: u64, ids: &[u64]) {
        let vorhanden = self
            .alle("IFCRELCONTAINEDINSPATIALSTRUCTURE")
            .into_iter()
            .filter_map(|r| self.get(r))
            .find(|r| r.attr(5).referenz() == Some(geschoss));
        match vorhanden {
            Some(mut r) => {
                let mut l = r.attr(4).referenzen();
                l.extend(
                    ids.iter()
                        .filter(|i| !l.contains(i))
                        .copied()
                        .collect::<Vec<_>>(),
                );
                r.attribute[4] = Self::liste(&l);
                self.setzen(r);
            }
            None => {
                let g = self.gid();
                let o = self.eigner.clone();
                self.add(
                    "IFCRELCONTAINEDINSPATIALSTRUCTURE",
                    vec![
                        g,
                        o,
                        Wert::Null,
                        Wert::Null,
                        Self::liste(ids),
                        Wert::Ref(geschoss),
                    ],
                );
            }
        }
    }

    fn eigenschaft(&mut self, id: u64, satz: &str, name: &str, wert: &Eigenschaftswert, m: f64) {
        // Direkt zugeordneter Satz gleichen Namens?
        let direkt = self
            .alle("IFCRELDEFINESBYPROPERTIES")
            .into_iter()
            .filter_map(|r| self.get(r))
            .find_map(|r| {
                if !r.attr(4).referenzen().contains(&id) {
                    return None;
                }
                let p = self.get(r.attr(5).referenz()?)?;
                (p.typ == "IFCPROPERTYSET" && p.attr(2).text() == Some(satz)).then_some((r, p))
            });
        let alt_wert = direkt.as_ref().and_then(|(_, p)| {
            p.attr(4).referenzen().into_iter().find_map(|x| {
                let v = self.get(x)?;
                (v.typ == "IFCPROPERTYSINGLEVALUE" && v.attr(0).text() == Some(name))
                    .then(|| (x, v.attr(2).clone()))
            })
        });
        let neu_wert = wert_schreiben(wert, alt_wert.as_ref().map(|(_, w)| w), m);
        let sv = self.add(
            "IFCPROPERTYSINGLEVALUE",
            vec![Wert::Text(name.into()), Wert::Null, neu_wert, Wert::Null],
        );
        let mut eigenschaften: Vec<u64> = direkt
            .as_ref()
            .map(|(_, p)| p.attr(4).referenzen())
            .unwrap_or_default();
        match alt_wert {
            Some((x, _)) => eigenschaften
                .iter_mut()
                .filter(|e| **e == x)
                .for_each(|e| *e = sv),
            None => eigenschaften.push(sv),
        }
        match direkt {
            // Satz gehört nur diesem Element: in der Datei ändern.
            Some((r, mut p)) if r.attr(4).referenzen() == [id] && self.nur_einmal(p.id) => {
                p.attribute[4] = Self::liste(&eigenschaften);
                self.setzen(p);
            }
            // Geteilter Satz: Element bekommt eine eigene Kopie.
            Some(_) => {
                self.herausnehmen("IFCRELDEFINESBYPROPERTIES", 4, 5, id, &|_| true);
                self.satz_anhaengen(id, satz, &eigenschaften);
            }
            None => self.satz_anhaengen(id, satz, &eigenschaften),
        }
    }
    fn nur_einmal(&self, satz: u64) -> bool {
        self.alle("IFCRELDEFINESBYPROPERTIES")
            .into_iter()
            .filter_map(|r| self.get(r))
            .filter(|r| r.attr(5).referenz() == Some(satz))
            .count()
            == 1
    }
    fn satz_anhaengen(&mut self, id: u64, satz: &str, eigenschaften: &[u64]) {
        let (g, o) = (self.gid(), self.eigner.clone());
        let p = self.add(
            "IFCPROPERTYSET",
            vec![
                g,
                o.clone(),
                Wert::Text(satz.into()),
                Wert::Null,
                Self::liste(eigenschaften),
            ],
        );
        let g = self.gid();
        self.add(
            "IFCRELDEFINESBYPROPERTIES",
            vec![
                g,
                o,
                Wert::Null,
                Wert::Null,
                Self::liste(&[id]),
                Wert::Ref(p),
            ],
        );
    }

    /// Kontext der Körperdarstellung (`Body`), sonst der Modellkontext.
    fn kontext(&self) -> Option<u64> {
        self.d
            .alle("IFCGEOMETRICREPRESENTATIONSUBCONTEXT")
            .find(|i| i.attr(0).text() == Some("Body"))
            .or_else(|| self.d.alle("IFCGEOMETRICREPRESENTATIONCONTEXT").next())
            .map(|i| i.id)
    }

    #[allow(clippy::too_many_arguments)]
    fn raum(
        &mut self,
        gid: &str,
        geschoss: u64,
        name: &str,
        langname: Option<&str>,
        umriss: &[[f64; 2]],
        unten_m: f64,
        hoehe_m: f64,
        m: f64,
    ) -> u64 {
        let darstellung = match self.kontext() {
            Some(ctx) => {
                let mut ring: Vec<u64> = umriss
                    .iter()
                    .map(|p| self.punkt(&[p[0] / m, p[1] / m]))
                    .collect();
                ring.push(ring[0]);
                let pl = self.add("IFCPOLYLINE", vec![Self::liste(&ring)]);
                let prof = self.add(
                    "IFCARBITRARYCLOSEDPROFILEDEF",
                    vec![Wert::Enum("AREA".into()), Wert::Null, Wert::Ref(pl)],
                );
                let o = self.punkt(&[0., 0., 0.]);
                let lage = self.add(
                    "IFCAXIS2PLACEMENT3D",
                    vec![Wert::Ref(o), Wert::Null, Wert::Null],
                );
                let r = self.add(
                    "IFCDIRECTION",
                    vec![Wert::Liste(vec![
                        Wert::Real(0.),
                        Wert::Real(0.),
                        Wert::Real(1.),
                    ])],
                );
                let k = self.add(
                    "IFCEXTRUDEDAREASOLID",
                    vec![
                        Wert::Ref(prof),
                        Wert::Ref(lage),
                        Wert::Ref(r),
                        Wert::Real(hoehe_m / m),
                    ],
                );
                let rep = self.add(
                    "IFCSHAPEREPRESENTATION",
                    vec![
                        Wert::Ref(ctx),
                        Wert::Text("Body".into()),
                        Wert::Text("SweptSolid".into()),
                        Self::liste(&[k]),
                    ],
                );
                Wert::Ref(self.add(
                    "IFCPRODUCTDEFINITIONSHAPE",
                    vec![Wert::Null, Wert::Null, Self::liste(&[rep])],
                ))
            }
            None => {
                self.hinweise.push(format!(
                    "Raum {name}: kein Darstellungskontext, ohne Geometrie geschrieben."
                ));
                Wert::Null
            }
        };
        let o = self.punkt(&[0., 0., unten_m / m]);
        let achse = self.add(
            "IFCAXIS2PLACEMENT3D",
            vec![Wert::Ref(o), Wert::Null, Wert::Null],
        );
        let lage = self.add("IFCLOCALPLACEMENT", vec![Wert::Null, Wert::Ref(achse)]);
        let eigner = self.eigner.clone();
        let raum = self.add(
            "IFCSPACE",
            vec![
                Wert::Text(gid.into()),
                eigner.clone(),
                Wert::Text(name.into()),
                Wert::Null,
                Wert::Null,
                Wert::Ref(lage),
                darstellung,
                langname.map_or(Wert::Null, |l| Wert::Text(l.into())),
                Wert::Enum("ELEMENT".into()),
                Wert::Enum("INTERNAL".into()),
                Wert::Null,
            ],
        );
        let g = self.gid();
        self.add(
            "IFCRELAGGREGATES",
            vec![
                g,
                eigner.clone(),
                Wert::Null,
                Wert::Null,
                Wert::Ref(geschoss),
                Self::liste(&[raum]),
            ],
        );
        // Basismengen, damit die Fläche auch in anderen Programmen ankommt.
        let flaeche = super::nacharbeit::umriss_flaeche(umriss);
        let x3 = self.d.schema.starts_with("IFC2X3");
        let menge = |n: &str, v: f64| {
            let mut a = vec![Wert::Text(n.into()), Wert::Null, Wert::Null, Wert::Real(v)];
            if !x3 {
                a.push(Wert::Null);
            }
            a
        };
        let qa = self.add("IFCQUANTITYAREA", menge("NetFloorArea", flaeche));
        let qh = self.add("IFCQUANTITYLENGTH", menge("Height", hoehe_m / m));
        let g = self.gid();
        let qs = self.add(
            "IFCELEMENTQUANTITY",
            vec![
                g,
                eigner.clone(),
                Wert::Text("Qto_SpaceBaseQuantities".into()),
                Wert::Null,
                Wert::Null,
                Self::liste(&[qa, qh]),
            ],
        );
        let g = self.gid();
        self.add(
            "IFCRELDEFINESBYPROPERTIES",
            vec![
                g,
                eigner,
                Wert::Null,
                Wert::Null,
                Self::liste(&[raum]),
                Wert::Ref(qs),
            ],
        );
        raum
    }

    /// Körperdarstellung durch eine senkrechte Extrusion des Grundrisses
    /// (Weltsystem, m) ersetzen; andere Darstellungen (Achse, Grundriss) bleiben.
    fn koerper_ersetzen(&mut self, id: u64, geo: &super::Modellgeometrie, m: f64) {
        let Some(mut e) = self.get(id) else { return };
        let gid = e.attr(0).text().unwrap_or("").to_string();
        let Some(ctx) = self.kontext() else {
            self.hinweise.push(format!(
                "{gid}: kein Darstellungskontext, Geometrie unverändert."
            ));
            return;
        };
        let l = super::geometrie::Geometrie { d: self.d, m }.platzierung(e.attr(5));
        if (l.z[2] - 1.).abs() > 1e-6 {
            self.hinweise.push(format!(
                "{gid}: Lage ist nicht senkrecht ausgerichtet, Geometrie unverändert."
            ));
            return;
        }
        let punkt = |v: [f64; 3], a: [f64; 3]| a[0] * v[0] + a[1] * v[1] + a[2] * v[2];
        let lokal = |p: [f64; 3]| {
            let d = [p[0] - l.o[0], p[1] - l.o[1], p[2] - l.o[2]];
            [punkt(l.x, d) / m, punkt(l.y, d) / m, punkt(l.z, d) / m]
        };
        let z = lokal([0., 0., geo.unten_m])[2];
        let oben = self.add(
            "IFCDIRECTION",
            vec![Wert::Liste(vec![
                Wert::Real(0.),
                Wert::Real(0.),
                Wert::Real(1.),
            ])],
        );
        let mut koerper = Vec::new();
        for poly in &geo.grundriss {
            let ring = |r: &[[f64; 2]], s: &mut Self| {
                let mut ids: Vec<u64> = r
                    .iter()
                    .map(|q| {
                        let p = lokal([q[0], q[1], geo.unten_m]);
                        s.punkt(&[p[0], p[1]])
                    })
                    .collect();
                ids.push(ids[0]);
                s.add("IFCPOLYLINE", vec![Self::liste(&ids)])
            };
            if poly.aussen.len() < 3 {
                continue;
            }
            let aussen = ring(&poly.aussen, self);
            let innen: Vec<u64> = poly
                .innen
                .iter()
                .filter(|r| r.len() >= 3)
                .map(|r| ring(r, self))
                .collect();
            let prof = if innen.is_empty() {
                self.add(
                    "IFCARBITRARYCLOSEDPROFILEDEF",
                    vec![Wert::Enum("AREA".into()), Wert::Null, Wert::Ref(aussen)],
                )
            } else {
                self.add(
                    "IFCARBITRARYPROFILEDEFWITHVOIDS",
                    vec![
                        Wert::Enum("AREA".into()),
                        Wert::Null,
                        Wert::Ref(aussen),
                        Self::liste(&innen),
                    ],
                )
            };
            let o = self.punkt(&[0., 0., z]);
            let lage = self.add(
                "IFCAXIS2PLACEMENT3D",
                vec![Wert::Ref(o), Wert::Null, Wert::Null],
            );
            koerper.push(self.add(
                "IFCEXTRUDEDAREASOLID",
                vec![
                    Wert::Ref(prof),
                    Wert::Ref(lage),
                    Wert::Ref(oben),
                    Wert::Real((geo.oben_m - geo.unten_m) / m),
                ],
            ));
        }
        let rep = self.add(
            "IFCSHAPEREPRESENTATION",
            vec![
                Wert::Ref(ctx),
                Wert::Text("Body".into()),
                Wert::Text("SweptSolid".into()),
                Self::liste(&koerper),
            ],
        );
        let mut darstellungen: Vec<u64> = e
            .attr(6)
            .referenz()
            .and_then(|x| self.get(x))
            .map(|pds| {
                pds.attr(2)
                    .referenzen()
                    .into_iter()
                    .filter(|r| {
                        self.get(*r)
                            .is_some_and(|x| x.attr(1).text() != Some("Body"))
                    })
                    .collect()
            })
            .unwrap_or_default();
        darstellungen.push(rep);
        let pds = self.add(
            "IFCPRODUCTDEFINITIONSHAPE",
            vec![Wert::Null, Wert::Null, Self::liste(&darstellungen)],
        );
        e.attribute.resize(e.attribute.len().max(7), Wert::Null);
        e.attribute[6] = Wert::Ref(pds);
        self.setzen(e);
    }

    /// Entfernte Elemente aus allen Beziehungen nehmen; Beziehungen, deren
    /// Bezugsobjekt fehlt oder deren Liste leer wird, entfallen. Teile eines
    /// entfernten Aggregats rücken in dessen Geschoss.
    fn aufraeumen(&mut self, weg: &HashSet<u64>) {
        for &id in weg {
            let teile: Vec<u64> = self
                .alle("IFCRELAGGREGATES")
                .into_iter()
                .filter_map(|r| self.get(r))
                .filter(|r| r.attr(4).referenz() == Some(id))
                .flat_map(|r| r.attr(5).referenzen())
                .collect();
            if teile.is_empty() {
                continue;
            }
            let behaelter = self
                .alle("IFCRELCONTAINEDINSPATIALSTRUCTURE")
                .into_iter()
                .filter_map(|r| self.get(r))
                .find(|r| r.attr(4).referenzen().contains(&id))
                .and_then(|r| r.attr(5).referenz());
            if let Some(g) = behaelter {
                self.enthalten(g, &teile);
            }
        }
        let rels: Vec<u64> = self
            .d
            .instanzen
            .values()
            .filter(|i| i.typ.starts_with("IFCREL"))
            .map(|i| i.id)
            .chain(
                self.neu
                    .values()
                    .filter(|i| i.typ.starts_with("IFCREL"))
                    .map(|i| i.id),
            )
            .collect();
        for r in rels {
            let Some(mut rel) = self.get(r) else { continue };
            let mut fallen = false;
            let mut geaendert = false;
            for a in rel.attribute.iter_mut() {
                match a {
                    Wert::Ref(x) if weg.contains(x) => fallen = true,
                    Wert::Liste(l)
                        if l.iter()
                            .any(|w| w.referenz().is_some_and(|x| weg.contains(&x))) =>
                    {
                        l.retain(|w| !w.referenz().is_some_and(|x| weg.contains(&x)));
                        fallen |= l.is_empty();
                        geaendert = true;
                    }
                    _ => {}
                }
            }
            if fallen {
                self.entfernt.insert(r);
            } else if geaendert {
                self.setzen(rel);
            }
        }
        self.entfernt.extend(weg.iter().copied());
    }
}

/// Wert einer Eigenschaft im Typ des bisherigen Werts, Längen in Dateieinheiten.
fn wert_schreiben(w: &Eigenschaftswert, alt: Option<&Wert>, m: f64) -> Wert {
    let alt_typ = match alt {
        Some(Wert::Typisiert(t, _)) => Some(t.as_str()),
        _ => None,
    };
    let typisiert = |t: &str, v: Wert| Wert::Typisiert(t.into(), Box::new(v));
    match w {
        Eigenschaftswert::Text { wert } => match alt_typ {
            Some(t @ ("IFCTEXT" | "IFCIDENTIFIER" | "IFCLABEL")) => {
                typisiert(t, Wert::Text(wert.clone()))
            }
            _ => typisiert("IFCLABEL", Wert::Text(wert.clone())),
        },
        Eigenschaftswert::Zahl { wert } => match alt_typ {
            Some(
                t @ ("IFCLENGTHMEASURE"
                | "IFCPOSITIVELENGTHMEASURE"
                | "IFCNONNEGATIVELENGTHMEASURE"),
            ) => typisiert(t, Wert::Real(wert / m)),
            Some("IFCINTEGER" | "IFCCOUNTMEASURE") if wert.fract() == 0. => {
                typisiert(alt_typ.unwrap_or("IFCINTEGER"), Wert::Ganz(*wert as i64))
            }
            Some(t) if t.ends_with("MEASURE") || t == "IFCREAL" => typisiert(t, Wert::Real(*wert)),
            _ => typisiert("IFCREAL", Wert::Real(*wert)),
        },
        Eigenschaftswert::Logisch { wert } => typisiert(
            if alt_typ == Some("IFCLOGICAL") {
                "IFCLOGICAL"
            } else {
                "IFCBOOLEAN"
            },
            Wert::Enum((if *wert { "T" } else { "F" }).into()),
        ),
    }
}

/// Schreibt die Datei des Modells mit der Nacharbeit neu (Schema der Quelle).
/// `soll` ist der Auszug mit angewendeter Nacharbeit (für Raumhöhen und die
/// Selbstkontrolle).
pub fn nacharbeit_schreiben(
    m: &Modell,
    soll: &Modellauszug,
    nacharbeit: &[Modellaenderung],
    dateiname: &str,
    zeit: &str,
) -> CoreResult<(String, Exportbericht)> {
    let d = &m.datei;
    let mj = m.auszug.meter_je_einheit;
    let mut index: HashMap<String, u64> = HashMap::new();
    for i in d.instanzen.values() {
        if let Some(g) = i.attr(0).text()
            && g.len() == 22
            && !i.typ.starts_with("IFCREL")
        {
            index.entry(g.to_string()).or_insert(i.id);
        }
    }
    let mut s = Schreiber::neu(d);
    let mut weg = HashSet::new();
    let mut neu = 0;
    let mut gesetzt: Vec<(String, String, String)> = Vec::new();
    let mut koerper: BTreeMap<u64, String> = BTreeMap::new();
    for c in nacharbeit {
        let gid = c.global_id();
        if let Modellaenderung::RaumBilden {
            global_id,
            geschoss,
            name,
            langname,
            umriss,
            ..
        } = c
        {
            let (Some(&g), Some(e)) = (index.get(geschoss), soll.element(global_id)) else {
                continue;
            };
            let Some(geo) = &e.geometrie else { continue };
            if index.contains_key(global_id) {
                continue;
            }
            s.raum(
                global_id,
                g,
                name,
                langname.as_deref(),
                umriss,
                geo.unten_m,
                geo.oben_m - geo.unten_m,
                mj,
            );
            neu += 1;
            continue;
        }
        let Some(&id) = index.get(gid) else { continue };
        match c {
            Modellaenderung::Klassifizieren {
                klasse,
                vordefiniert,
                ..
            } => s.klassifizieren(id, klasse, vordefiniert.as_deref()),
            Modellaenderung::GeschossZuordnen { geschoss, .. } => match index.get(geschoss) {
                Some(&g) => s.zuordnen(id, g),
                None => continue,
            },
            Modellaenderung::EigenschaftSetzen {
                satz, name, wert, ..
            } => {
                s.eigenschaft(id, satz, name, wert, mj);
                gesetzt.push((gid.to_string(), satz.clone(), name.clone()));
            }
            Modellaenderung::Benennen { name, langname, .. } => {
                s.benennen(id, name, langname.as_deref())
            }
            Modellaenderung::Ausblenden { .. } => {
                weg.insert(id);
                continue;
            }
            Modellaenderung::HoeheSetzen { .. } | Modellaenderung::UmrissSetzen { .. } => {
                koerper.insert(id, gid.to_string());
            }
            Modellaenderung::RaumBilden { .. } => unreachable!(),
        }
        s.geaendert.insert(id);
    }
    // Körper erst am Ende: der Endstand steht im Soll.
    for (id, gid) in &koerper {
        if let Some(geo) = soll.element(gid).and_then(|e| e.geometrie.as_ref())
            && !weg.contains(id)
        {
            s.koerper_ersetzen(*id, geo, mj);
        }
    }
    s.aufraeumen(&weg);
    let kopf = kopfzeilen(&d.schema, dateiname, zeit, d.system.as_deref());
    let neu_liste: Vec<Instanz> = s.neu.values().cloned().collect();
    let text = d.neu_schreiben(&s.ersetzt, &s.entfernt, &neu_liste, Some(&kopf));
    let (elemente, abweichungen) = kontrolle(&text, soll, &gesetzt);
    let bericht = Exportbericht {
        dateiname: dateiname.into(),
        schema: d.schema.clone(),
        elemente,
        geaendert: s.geaendert.len(),
        neu,
        entfernt: weg.len(),
        hinweise: s.hinweise,
        abweichungen,
    };
    Ok((text, bericht))
}

/// Liest die geschriebene Datei wieder ein und vergleicht sie mit dem
/// erwarteten Modell: Elemente, Klassen, Arten, Namen, Geschosse, Raumflächen
/// und die gesetzten Eigenschaften (GlobalId, Satz, Name).
pub fn kontrolle(
    text: &str,
    soll: &Modellauszug,
    eigenschaften: &[(String, String, String)],
) -> (usize, Vec<String>) {
    let ist = match Modell::lesen("export.ifc", text.as_bytes()) {
        Ok(m) => m.auszug,
        Err(e) => return (0, vec![format!("Geschriebene Datei nicht lesbar: {e}")]),
    };
    let mut ab = Vec::new();
    if ist.elemente.len() != soll.elemente.len() {
        ab.push(format!(
            "{} statt {} Elemente",
            ist.elemente.len(),
            soll.elemente.len()
        ));
    }
    let geschossname = |a: &Modellauszug, g: &Option<String>| {
        g.as_deref()
            .and_then(|g| a.geschoss(g))
            .map(|g| g.name.clone())
    };
    for e in &soll.elemente {
        let Some(x) = ist.element(&e.global_id) else {
            ab.push(format!("{} {} fehlt", klassentitel(&e.klasse), e.global_id));
            continue;
        };
        let mut d = Vec::new();
        let proxy_ersatz =
            x.klasse == "IFCBUILDINGELEMENTPROXY" && aufbau(&ist.schema, &e.klasse).is_none();
        if x.klasse != e.klasse && !proxy_ersatz {
            d.push(format!("Klasse {}", x.klasse));
        }
        if x.vordefiniert != e.vordefiniert
            && !proxy_ersatz
            && aufbau(&ist.schema, &x.klasse).is_some_and(|(_, s)| s.is_some())
        {
            d.push(format!("Art {:?}", x.vordefiniert));
        }
        if x.name != e.name {
            d.push(format!("Name „{}“", x.name));
        }
        if e.klasse == "IFCSPACE" && x.langname != e.langname {
            d.push(format!("Langname {:?}", x.langname));
        }
        if geschossname(&ist, &x.geschoss) != geschossname(soll, &e.geschoss) {
            d.push("Geschoss".into());
        }
        match (&e.geometrie, &x.geometrie) {
            (Some(a), Some(b)) => {
                if (a.grundflaeche_m2 - b.grundflaeche_m2).abs() > 0.01 {
                    d.push(format!("Grundfläche {:.2} m²", b.grundflaeche_m2));
                }
                if (a.unten_m - b.unten_m).abs() > 0.005 || (a.oben_m - b.oben_m).abs() > 0.005 {
                    d.push(format!("Höhe {:.3}–{:.3} m", b.unten_m, b.oben_m));
                }
            }
            (Some(_), None) => d.push("Geometrie fehlt".into()),
            _ => {}
        }
        if !d.is_empty() {
            ab.push(format!("{}: {}", e.global_id, d.join(", ")));
        }
    }
    for (g, satz, name) in eigenschaften {
        let wert = |a: &Modellauszug| {
            a.element(g)
                .map(|e| &e.eigenschaften)
                .or_else(|| {
                    a.geschosse
                        .iter()
                        .chain(&a.gebaeude)
                        .chain(&a.grundstuecke)
                        .find(|s| &s.global_id == g)
                        .map(|s| &s.eigenschaften)
                })
                .and_then(|l| {
                    l.iter()
                        .rev()
                        .find(|p| &p.satz == satz && &p.name == name && !p.vom_typ)
                })
                .map(|p| (p.zahl, p.wert.clone()))
        };
        match (wert(soll), wert(&ist)) {
            (Some((Some(a), _)), Some((Some(b), _))) if (a - b).abs() < 1e-6 => {}
            (Some((None, a)), Some((None, b))) if a == b => {}
            (_, x) => ab.push(format!("{g}: Eigenschaft {satz}.{name} = {x:?}")),
        }
    }
    for g in &soll.geschosse {
        match ist.geschoss(&g.global_id) {
            Some(x) if x.name == g.name && x.langname == g.langname => {}
            _ => ab.push(format!("Geschoss {} abweichend", g.name)),
        }
    }
    (ist.elemente.len(), ab)
}

// ---------------------------------------------------------------------------
// Eigenes IFC4-Modell aus den Zeichnungen

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Entwurfswand {
    pub id: Uuid,
    /// Achse in Modellkoordinaten (m, y nach Norden).
    pub von: [f64; 2],
    pub bis: [f64; 2],
    pub dicke_m: f64,
    pub aussen: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Entwurfsraum {
    pub id: Uuid,
    pub name: String,
    pub langname: Option<String>,
    pub umriss: Vec<Planpolygon>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Entwurfsgeschoss {
    pub id: Uuid,
    pub name: String,
    /// Ebene über ±0 (m).
    pub ebene_m: f64,
    pub geschosshoehe_m: f64,
    pub deckendicke_m: f64,
    pub lichte_hoehe_m: Option<f64>,
    pub waende: Vec<Entwurfswand>,
    pub raeume: Vec<Entwurfsraum>,
    /// Außenkante (Decke über dem Geschoss bzw. Dach im obersten Geschoss).
    pub aussenkante: Vec<Planpolygon>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Entwurf {
    pub id: Uuid,
    pub projekt: String,
    pub gebaeude: String,
    pub nullpunkt_m_nhn: Option<f64>,
    pub geschosse: Vec<Entwurfsgeschoss>,
}

/// Stabile GlobalId aus einer Objekt-ID und einem Zusatz.
fn stabil(id: Uuid, zusatz: &str) -> Wert {
    Wert::Text(global_id(Uuid::new_v5(&id, zusatz.as_bytes())))
}

struct Puffer {
    zeilen: Vec<String>,
}

impl Puffer {
    fn add(&mut self, typ: &str, a: &[Wert]) -> u64 {
        let id = self.zeilen.len() as u64 + 1;
        self.zeilen.push(super::step::instanz_text(&Instanz {
            id,
            typ: typ.into(),
            attribute: a.to_vec(),
        }));
        id
    }
    fn punkt(&mut self, p: &[f64]) -> u64 {
        self.add(
            "IFCCARTESIANPOINT",
            &[Wert::Liste(p.iter().map(|v| Wert::Real(*v)).collect())],
        )
    }
    fn richtung(&mut self, p: &[f64]) -> u64 {
        self.add(
            "IFCDIRECTION",
            &[Wert::Liste(p.iter().map(|v| Wert::Real(*v)).collect())],
        )
    }
    fn achse(&mut self, o: [f64; 3], x: Option<[f64; 2]>) -> u64 {
        let p = self.punkt(&o);
        let (z, x) = match x {
            Some(x) => (
                Wert::Ref(self.richtung(&[0., 0., 1.])),
                Wert::Ref(self.richtung(&[x[0], x[1], 0.])),
            ),
            None => (Wert::Null, Wert::Null),
        };
        self.add("IFCAXIS2PLACEMENT3D", &[Wert::Ref(p), z, x])
    }
    fn lage(&mut self, rel: Option<u64>, o: [f64; 3], x: Option<[f64; 2]>) -> u64 {
        let a = self.achse(o, x);
        self.add(
            "IFCLOCALPLACEMENT",
            &[rel.map_or(Wert::Null, Wert::Ref), Wert::Ref(a)],
        )
    }
    fn ring(&mut self, r: &[[f64; 2]]) -> u64 {
        let mut ids: Vec<Wert> = r.iter().map(|p| Wert::Ref(self.punkt(p))).collect();
        ids.push(ids[0].clone());
        self.add("IFCPOLYLINE", &[Wert::Liste(ids)])
    }
    fn profil(&mut self, p: &Planpolygon) -> u64 {
        let aussen = self.ring(&p.aussen);
        if p.innen.is_empty() {
            return self.add(
                "IFCARBITRARYCLOSEDPROFILEDEF",
                &[Wert::Enum("AREA".into()), Wert::Null, Wert::Ref(aussen)],
            );
        }
        let innen: Vec<Wert> = p.innen.iter().map(|r| Wert::Ref(self.ring(r))).collect();
        self.add(
            "IFCARBITRARYPROFILEDEFWITHVOIDS",
            &[
                Wert::Enum("AREA".into()),
                Wert::Null,
                Wert::Ref(aussen),
                Wert::Liste(innen),
            ],
        )
    }
    /// Körper aus Profilen, um `tiefe` nach oben extrudiert, ab Höhe `z`.
    fn koerper(&mut self, ctx: u64, profile: &[u64], z: f64, tiefe: f64) -> u64 {
        let oben = self.richtung(&[0., 0., 1.]);
        let koerper: Vec<Wert> = profile
            .iter()
            .map(|p| {
                let a = self.achse([0., 0., z], None);
                Wert::Ref(self.add(
                    "IFCEXTRUDEDAREASOLID",
                    &[
                        Wert::Ref(*p),
                        Wert::Ref(a),
                        Wert::Ref(oben),
                        Wert::Real(tiefe),
                    ],
                ))
            })
            .collect();
        let rep = self.add(
            "IFCSHAPEREPRESENTATION",
            &[
                Wert::Ref(ctx),
                Wert::Text("Body".into()),
                Wert::Text("SweptSolid".into()),
                Wert::Liste(koerper),
            ],
        );
        self.add(
            "IFCPRODUCTDEFINITIONSHAPE",
            &[Wert::Null, Wert::Null, Wert::Liste(vec![Wert::Ref(rep)])],
        )
    }
}

fn refs(ids: &[u64]) -> Wert {
    Wert::Liste(ids.iter().map(|i| Wert::Ref(*i)).collect())
}

fn text(t: &str) -> Wert {
    Wert::Text(t.into())
}

/// Schreibt ein IFC4-Modell (Meter) aus dem Entwurf.
pub fn entwurf_schreiben(e: &Entwurf, dateiname: &str, zeit: &str) -> CoreResult<String> {
    if e.geschosse.is_empty() {
        return Err(fehler("Mindestens ein Geschoss mit Zeichnung wählen"));
    }
    let mut b = Puffer { zeilen: Vec::new() };
    let n = Wert::Null;
    let einheiten: Vec<u64> = [
        ("LENGTHUNIT", "METRE"),
        ("AREAUNIT", "SQUARE_METRE"),
        ("VOLUMEUNIT", "CUBIC_METRE"),
        ("PLANEANGLEUNIT", "RADIAN"),
    ]
    .iter()
    .map(|(art, name)| {
        b.add(
            "IFCSIUNIT",
            &[
                Wert::Abgeleitet,
                Wert::Enum((*art).into()),
                Wert::Null,
                Wert::Enum((*name).into()),
            ],
        )
    })
    .collect();
    let einheiten = b.add("IFCUNITASSIGNMENT", &[refs(&einheiten)]);
    let welt = b.achse([0., 0., 0.], None);
    let nord = b.richtung(&[0., 1.]);
    let kontext = b.add(
        "IFCGEOMETRICREPRESENTATIONCONTEXT",
        &[
            n.clone(),
            text("Model"),
            Wert::Ganz(3),
            Wert::Real(1e-5),
            Wert::Ref(welt),
            Wert::Ref(nord),
        ],
    );
    let body = b.add(
        "IFCGEOMETRICREPRESENTATIONSUBCONTEXT",
        &[
            text("Body"),
            text("Model"),
            Wert::Abgeleitet,
            Wert::Abgeleitet,
            Wert::Abgeleitet,
            Wert::Abgeleitet,
            Wert::Ref(kontext),
            n.clone(),
            Wert::Enum("MODEL_VIEW".into()),
            n.clone(),
        ],
    );
    let projekt = b.add(
        "IFCPROJECT",
        &[
            stabil(e.id, "projekt"),
            n.clone(),
            text(&e.projekt),
            n.clone(),
            n.clone(),
            n.clone(),
            n.clone(),
            refs(&[kontext]),
            Wert::Ref(einheiten),
        ],
    );
    let site_lage = b.lage(None, [0., 0., 0.], None);
    let site = b.add(
        "IFCSITE",
        &[
            stabil(e.id, "grundstueck"),
            n.clone(),
            text("Grundstück"),
            n.clone(),
            n.clone(),
            Wert::Ref(site_lage),
            n.clone(),
            n.clone(),
            Wert::Enum("ELEMENT".into()),
            n.clone(),
            n.clone(),
            n.clone(),
            n.clone(),
            n.clone(),
        ],
    );
    let bau_lage = b.lage(Some(site_lage), [0., 0., 0.], None);
    let haus = b.add(
        "IFCBUILDING",
        &[
            stabil(e.id, "gebaeude"),
            n.clone(),
            text(&e.gebaeude),
            n.clone(),
            n.clone(),
            Wert::Ref(bau_lage),
            n.clone(),
            n.clone(),
            Wert::Enum("ELEMENT".into()),
            e.nullpunkt_m_nhn.map_or(n.clone(), Wert::Real),
            n.clone(),
            n.clone(),
        ],
    );
    b.add(
        "IFCRELAGGREGATES",
        &[
            stabil(e.id, "rel-projekt"),
            n.clone(),
            n.clone(),
            n.clone(),
            Wert::Ref(projekt),
            refs(&[site]),
        ],
    );
    b.add(
        "IFCRELAGGREGATES",
        &[
            stabil(e.id, "rel-grundstueck"),
            n.clone(),
            n.clone(),
            n.clone(),
            Wert::Ref(site),
            refs(&[haus]),
        ],
    );
    let mut storeys = Vec::new();
    let mut aussen_waende = Vec::new();
    let mut innen_waende = Vec::new();
    let letzte = e.geschosse.len() - 1;
    for (nr, g) in e.geschosse.iter().enumerate() {
        let lage = b.lage(Some(bau_lage), [0., 0., g.ebene_m], None);
        let st = b.add(
            "IFCBUILDINGSTOREY",
            &[
                stabil(g.id, "geschoss"),
                n.clone(),
                text(&g.name),
                n.clone(),
                n.clone(),
                Wert::Ref(lage),
                n.clone(),
                n.clone(),
                Wert::Enum("ELEMENT".into()),
                Wert::Real(g.ebene_m),
            ],
        );
        storeys.push(st);
        let wandhoehe = (g.geschosshoehe_m - g.deckendicke_m).max(0.1);
        let mut enthalten = Vec::new();
        for w in &g.waende {
            let (dx, dy) = (w.bis[0] - w.von[0], w.bis[1] - w.von[1]);
            let laenge = dx.hypot(dy);
            if laenge < 1e-3 {
                continue;
            }
            let wl = b.lage(
                Some(lage),
                [w.von[0], w.von[1], 0.],
                Some([dx / laenge, dy / laenge]),
            );
            let p = b.punkt(&[laenge / 2., 0.]);
            let pos = b.add("IFCAXIS2PLACEMENT2D", &[Wert::Ref(p), n.clone()]);
            let prof = b.add(
                "IFCRECTANGLEPROFILEDEF",
                &[
                    Wert::Enum("AREA".into()),
                    n.clone(),
                    Wert::Ref(pos),
                    Wert::Real(laenge),
                    Wert::Real(w.dicke_m),
                ],
            );
            let shape = b.koerper(body, &[prof], 0., wandhoehe);
            let wand = b.add(
                "IFCWALL",
                &[
                    stabil(w.id, "wand"),
                    n.clone(),
                    text(if w.aussen { "Außenwand" } else { "Innenwand" }),
                    n.clone(),
                    n.clone(),
                    Wert::Ref(wl),
                    Wert::Ref(shape),
                    n.clone(),
                    Wert::Enum("STANDARD".into()),
                ],
            );
            enthalten.push(wand);
            if w.aussen {
                aussen_waende.push(wand);
            } else {
                innen_waende.push(wand);
            }
        }
        if !g.aussenkante.is_empty() {
            let profile: Vec<u64> = g.aussenkante.iter().map(|p| b.profil(p)).collect();
            let sl = b.lage(Some(lage), [0., 0., 0.], None);
            let shape = b.koerper(body, &profile, wandhoehe, g.deckendicke_m);
            let dach = nr == letzte;
            enthalten.push(b.add(
                "IFCSLAB",
                &[
                    stabil(g.id, "decke"),
                    n.clone(),
                    text(&if dach {
                        "Dach".to_string()
                    } else {
                        format!("Decke über {}", g.name)
                    }),
                    n.clone(),
                    n.clone(),
                    Wert::Ref(sl),
                    Wert::Ref(shape),
                    n.clone(),
                    Wert::Enum((if dach { "ROOF" } else { "FLOOR" }).into()),
                ],
            ));
        }
        if !enthalten.is_empty() {
            b.add(
                "IFCRELCONTAINEDINSPATIALSTRUCTURE",
                &[
                    stabil(g.id, "rel-enthalten"),
                    n.clone(),
                    n.clone(),
                    n.clone(),
                    refs(&enthalten),
                    Wert::Ref(st),
                ],
            );
        }
        let raumhoehe = g.lichte_hoehe_m.unwrap_or(wandhoehe);
        let mut raeume = Vec::new();
        for r in &g.raeume {
            let profile: Vec<u64> = r
                .umriss
                .iter()
                .filter(|p| p.aussen.len() >= 3)
                .map(|p| b.profil(p))
                .collect();
            if profile.is_empty() {
                continue;
            }
            let rl = b.lage(Some(lage), [0., 0., 0.], None);
            let shape = b.koerper(body, &profile, 0., raumhoehe);
            let raum = b.add(
                "IFCSPACE",
                &[
                    stabil(r.id, "raum"),
                    n.clone(),
                    text(&r.name),
                    n.clone(),
                    n.clone(),
                    Wert::Ref(rl),
                    Wert::Ref(shape),
                    r.langname.as_deref().map_or(n.clone(), text),
                    Wert::Enum("ELEMENT".into()),
                    Wert::Enum("INTERNAL".into()),
                    n.clone(),
                ],
            );
            let flaeche = super::geometrie::multipolygon(&r.umriss).unsigned_area();
            let qa = b.add(
                "IFCQUANTITYAREA",
                &[
                    text("NetFloorArea"),
                    n.clone(),
                    n.clone(),
                    Wert::Real(super::geometrie::rund(flaeche)),
                    n.clone(),
                ],
            );
            let qh = b.add(
                "IFCQUANTITYLENGTH",
                &[
                    text("Height"),
                    n.clone(),
                    n.clone(),
                    Wert::Real(raumhoehe),
                    n.clone(),
                ],
            );
            let qs = b.add(
                "IFCELEMENTQUANTITY",
                &[
                    stabil(r.id, "mengen"),
                    n.clone(),
                    text("Qto_SpaceBaseQuantities"),
                    n.clone(),
                    n.clone(),
                    refs(&[qa, qh]),
                ],
            );
            b.add(
                "IFCRELDEFINESBYPROPERTIES",
                &[
                    stabil(r.id, "rel-mengen"),
                    n.clone(),
                    n.clone(),
                    n.clone(),
                    refs(&[raum]),
                    Wert::Ref(qs),
                ],
            );
            raeume.push(raum);
        }
        if !raeume.is_empty() {
            b.add(
                "IFCRELAGGREGATES",
                &[
                    stabil(g.id, "rel-raeume"),
                    n.clone(),
                    n.clone(),
                    n.clone(),
                    Wert::Ref(st),
                    refs(&raeume),
                ],
            );
        }
    }
    b.add(
        "IFCRELAGGREGATES",
        &[
            stabil(e.id, "rel-gebaeude"),
            n.clone(),
            n.clone(),
            n.clone(),
            Wert::Ref(haus),
            refs(&storeys),
        ],
    );
    for (aussen, ids) in [(true, &aussen_waende), (false, &innen_waende)] {
        if ids.is_empty() {
            continue;
        }
        let p = b.add(
            "IFCPROPERTYSINGLEVALUE",
            &[
                text("IsExternal"),
                n.clone(),
                Wert::Typisiert(
                    "IFCBOOLEAN".into(),
                    Box::new(Wert::Enum((if aussen { "T" } else { "F" }).into())),
                ),
                n.clone(),
            ],
        );
        let k = if aussen { "aussen" } else { "innen" };
        let ps = b.add(
            "IFCPROPERTYSET",
            &[
                stabil(e.id, &format!("pset-{k}")),
                n.clone(),
                text("Pset_WallCommon"),
                n.clone(),
                refs(&[p]),
            ],
        );
        b.add(
            "IFCRELDEFINESBYPROPERTIES",
            &[
                stabil(e.id, &format!("rel-pset-{k}")),
                n.clone(),
                n.clone(),
                n.clone(),
                refs(ids),
                Wert::Ref(ps),
            ],
        );
    }
    let mut out = kopfzeilen("IFC4", dateiname, zeit, None);
    out.push_str("DATA;\n");
    for z in &b.zeilen {
        out.push_str(z);
        out.push('\n');
    }
    out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    Ok(out)
}

/// Selbstkontrolle des Entwurfs: Geschosse, Wände, Räume und Raumflächen.
pub fn entwurf_kontrolle(text: &str, e: &Entwurf) -> (usize, Vec<String>) {
    let a = match Modell::lesen("entwurf.ifc", text.as_bytes()) {
        Ok(m) => m.auszug,
        Err(x) => return (0, vec![format!("Geschriebene Datei nicht lesbar: {x}")]),
    };
    let mut ab = Vec::new();
    if a.geschosse.len() != e.geschosse.len() {
        ab.push(format!(
            "{} statt {} Geschosse",
            a.geschosse.len(),
            e.geschosse.len()
        ));
    }
    for g in &e.geschosse {
        let gid = global_id(Uuid::new_v5(&g.id, b"geschoss"));
        let im = |k: &str| {
            a.elemente
                .iter()
                .filter(|x| x.klasse == k && x.geschoss.as_deref() == Some(gid.as_str()))
                .count()
        };
        let waende = g
            .waende
            .iter()
            .filter(|w| (w.bis[0] - w.von[0]).hypot(w.bis[1] - w.von[1]) >= 1e-3)
            .count();
        if im("IFCWALL") != waende {
            ab.push(format!(
                "{}: {} statt {waende} Wände",
                g.name,
                im("IFCWALL")
            ));
        }
        for r in &g.raeume {
            let rid = global_id(Uuid::new_v5(&r.id, b"raum"));
            let soll = super::geometrie::multipolygon(&r.umriss).unsigned_area();
            match a.element(&rid).and_then(|x| x.geometrie.as_ref()) {
                Some(geo) if (geo.grundflaeche_m2 - soll).abs() <= 0.01 => {}
                Some(geo) => ab.push(format!(
                    "Raum {}: {:.2} statt {soll:.2} m²",
                    r.name, geo.grundflaeche_m2
                )),
                None => ab.push(format!("Raum {} fehlt", r.name)),
            }
        }
    }
    (a.elemente.len(), ab)
}

// ---------------------------------------------------------------------------
// Export ins DMS

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Entwurfszuordnung {
    /// Geschoss des Gebäudes (Fachobjekt `gebaeude`).
    pub geschoss_id: Uuid,
    /// Eigene Zeichnung (Planaufmaß ohne Unterlage, Meter) dieses Geschosses.
    pub zeichnung_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "art", rename_all = "snake_case", deny_unknown_fields)]
pub enum Exportwunsch {
    /// Datei der Modellakte mit ihrer Nacharbeit neu schreiben.
    Nacharbeit { akte_id: Uuid },
    /// Eigenes IFC4-Modell aus Gebäude und Zeichnungen.
    Entwurf {
        gebaeude_id: Uuid,
        geschosse: Vec<Entwurfszuordnung>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Exportergebnis {
    /// Abgelegte IFC-Datei (neue Version, wenn es den Namen schon gibt).
    pub dokument: crate::plattform::objekte::Dokumentbezug,
    pub bericht: Exportbericht,
}

/// Standardwerte, wenn das Gebäude keine Höhen hat.
const GESCHOSSHOEHE_M: f64 = 2.75;
const DECKENDICKE_M: f64 = 0.2;

fn strecke_abstand(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l = dx * dx + dy * dy;
    let t = if l > 0. {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l).clamp(0., 1.)
    } else {
        0.
    };
    (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
}

/// Liegt die Wandachse an der Außenkante (beide Enden höchstens halbe Dicke + 5 cm entfernt)?
fn an_aussenkante(von: [f64; 2], bis: [f64; 2], dicke: f64, kante: &[Planpolygon]) -> bool {
    let nah = |p: [f64; 2]| {
        kante.iter().any(|k| {
            (0..k.aussen.len()).any(|i| {
                strecke_abstand(p, k.aussen[i], k.aussen[(i + 1) % k.aussen.len()])
                    <= dicke / 2. + 0.05
            })
        })
    };
    nah(von) && nah(bis)
}

/// Zeichnung (y nach unten) in Modellkoordinaten (y nach Norden).
fn gespiegelt(p: &Planpolygon) -> Planpolygon {
    let r = |r: &Vec<[f64; 2]>| r.iter().map(|q| [q[0], -q[1]]).collect();
    Planpolygon {
        aussen: r(&p.aussen),
        innen: p.innen.iter().map(r).collect(),
    }
}

/// „0.01 Wohnen“ → Nummer „0.01“ und Nutzung „Wohnen“.
fn raumname(t: &str) -> (String, Option<String>) {
    let t = t.trim();
    match t.split_once(' ') {
        Some((n, rest)) if n.chars().any(|c| c.is_ascii_digit()) && !rest.trim().is_empty() => {
            (n.into(), Some(rest.trim().into()))
        }
        _ => (t.into(), None),
    }
}

impl crate::CoreServices {
    /// IFC-Datei schreiben, selbst prüfen und im DMS ablegen.
    pub async fn ifc_exportieren(
        &self,
        p: crate::domain::ProjectId,
        w: &Exportwunsch,
        wer: &crate::benutzer::Benutzer,
    ) -> CoreResult<Exportergebnis> {
        let zeit = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        let dokumente = self.list_documents(p).await?;
        let (name, text, bericht) = match w {
            Exportwunsch::Nacharbeit { akte_id } => {
                let akte = self.modellakte(p, *akte_id).await?;
                let m = self.ifc_modell(p, &akte.dokument).await?;
                let soll = self.ifc_akte_auszug(p, &akte).await?;
                let quelle = dokumente
                    .iter()
                    .find(|d| d.document_id == akte.dokument.dokument_id)
                    .map_or("Modell.ifc", |d| d.file_name.as_str());
                let stamm = quelle
                    .rsplit_once('.')
                    .map_or(quelle, |(s, _)| s)
                    .trim_end_matches("-nachgearbeitet");
                let name = crate::dateiname::portabel(&format!("{stamm}-nachgearbeitet.ifc"), 180)
                    .unwrap_or_else(|| "Modell-nachgearbeitet.ifc".into());
                let (text, b) = nacharbeit_schreiben(&m, &soll, &akte.nacharbeit, &name, &zeit)?;
                (name, text, b)
            }
            Exportwunsch::Entwurf {
                gebaeude_id,
                geschosse,
            } => {
                let (e, hinweise) = self.entwurf(p, *gebaeude_id, geschosse).await?;
                let name = crate::dateiname::portabel(&format!("{}-Modell.ifc", e.gebaeude), 180)
                    .unwrap_or_else(|| "Gebaeude-Modell.ifc".into());
                let text = entwurf_schreiben(&e, &name, &zeit)?;
                let (elemente, abweichungen) = entwurf_kontrolle(&text, &e);
                let b = Exportbericht {
                    dateiname: name.clone(),
                    schema: "IFC4".into(),
                    elemente,
                    geaendert: 0,
                    neu: elemente,
                    entfernt: 0,
                    hinweise,
                    abweichungen,
                };
                (name, text, b)
            }
        };
        if bericht.elemente == 0 && !bericht.abweichungen.is_empty() {
            return Err(fehler(format!(
                "Export nicht abgelegt: {}",
                bericht.abweichungen.join("; ")
            )));
        }
        let vorhanden = dokumente
            .iter()
            .filter(|d| d.file_name == name)
            .max_by_key(|d| d.version);
        let v = self
            .store_document(
                p,
                vorhanden.map(|d| d.document_id),
                vorhanden.map(|d| d.version),
                &name,
                bytes::Bytes::from(text),
                wer,
            )
            .await?;
        Ok(Exportergebnis {
            dokument: crate::plattform::objekte::Dokumentbezug {
                dokument_id: v.document_id,
                version: v.version,
            },
            bericht,
        })
    }

    /// Entwurf aus dem Gebäude und je Geschoss einer eigenen Zeichnung.
    async fn entwurf(
        &self,
        p: crate::domain::ProjectId,
        gebaeude_id: Uuid,
        zuordnung: &[Entwurfszuordnung],
    ) -> CoreResult<(Entwurf, Vec<String>)> {
        use crate::planaufmass::{Planaufmass, Planbedeutung, Plangeometrie, Zeichenform};
        use crate::plattform::{ObjektArt, objekte::Gebaeude};
        if zuordnung.is_empty() {
            return Err(fehler("Mindestens ein Geschoss mit Zeichnung wählen"));
        }
        let f = self
            .fachobjekt_lesen(p, ObjektArt::Gebaeude, gebaeude_id)
            .await?
            .ok_or_else(|| fehler("Gebäude fehlt im Projekt"))?;
        let haus: Gebaeude = serde_json::from_value(f.daten).map_err(|e| fehler(e.to_string()))?;
        let mut hinweise = Vec::new();
        let mut liste: Vec<(usize, &Entwurfszuordnung)> = Vec::new();
        for z in zuordnung {
            let i = haus
                .geschosse
                .iter()
                .position(|g| g.id == z.geschoss_id)
                .ok_or_else(|| fehler("Geschoss gehört nicht zum Gebäude"))?;
            if liste.iter().any(|(j, _)| *j == i) {
                return Err(fehler("Jedes Geschoss nur einmal zuordnen"));
            }
            liste.push((i, z));
        }
        liste.sort_by_key(|(i, _)| *i);
        let mut geschosse = Vec::new();
        let mut ebene = 0.;
        for (i, z) in liste {
            let g = &haus.geschosse[i];
            let geo = g.geometrie.clone().unwrap_or_default();
            let hoehe = geo.geschosshoehe_m.filter(|h| *h > 0.5).unwrap_or_else(|| {
                hinweise.push(format!(
                    "{}: keine Geschosshöhe, {GESCHOSSHOEHE_M} m angenommen.",
                    g.bezeichnung
                ));
                GESCHOSSHOEHE_M
            });
            let decke = geo
                .deckendicke_m
                .filter(|d| *d > 0. && *d < hoehe)
                .unwrap_or(DECKENDICKE_M);
            let e = geo.fussboden_m.unwrap_or(ebene);
            ebene = e + hoehe;
            let fz = self
                .fachobjekt_lesen(p, ObjektArt::Planaufmass, z.zeichnung_id)
                .await?
                .ok_or_else(|| fehler("Zeichnung fehlt im Projekt"))?;
            let plan: Planaufmass =
                serde_json::from_value(fz.daten).map_err(|e| fehler(e.to_string()))?;
            if plan.dokument.is_some() {
                return Err(fehler(format!(
                    "„{}“ ist ein Aufmaß auf einer Unterlage. Als Modell lassen sich nur eigene Zeichnungen (in Metern) schreiben.",
                    plan.bezeichnung
                )));
            }
            let flaechen = |b: &[Planbedeutung]| -> Vec<(Uuid, String, Vec<Planpolygon>)> {
                plan.markierungen
                    .iter()
                    .filter(|m| b.contains(&m.bedeutung) && m.hoehenzone_von.is_none())
                    .filter_map(|m| match &m.geometrie {
                        Plangeometrie::Flaeche { polygone } => Some((
                            m.id,
                            m.bezeichnung.clone(),
                            polygone.iter().map(gespiegelt).collect(),
                        )),
                        _ => None,
                    })
                    .collect()
            };
            let aussenkante: Vec<Planpolygon> = flaechen(&[Planbedeutung::GeschossAussenkante])
                .into_iter()
                .flat_map(|(_, _, p)| p)
                .collect();
            let waende: Vec<Entwurfswand> = plan
                .elemente
                .iter()
                .filter_map(|el| match &el.form {
                    Zeichenform::Wand { von, bis, dicke_m } => {
                        let (von, bis) = ([von[0], -von[1]], [bis[0], -bis[1]]);
                        Some(Entwurfswand {
                            id: el.id,
                            von,
                            bis,
                            dicke_m: *dicke_m,
                            aussen: an_aussenkante(von, bis, *dicke_m, &aussenkante),
                        })
                    }
                    _ => None,
                })
                .collect();
            let raeume: Vec<Entwurfsraum> =
                flaechen(&[Planbedeutung::Raum, Planbedeutung::Aufenthaltsraum])
                    .into_iter()
                    .map(|(id, t, umriss)| {
                        let (name, langname) = raumname(&t);
                        Entwurfsraum {
                            id,
                            name,
                            langname,
                            umriss,
                        }
                    })
                    .collect();
            if aussenkante.is_empty() {
                hinweise.push(format!(
                    "{}: keine Geschoss-Außenkante in der Zeichnung, ohne Decke geschrieben.",
                    g.bezeichnung
                ));
            }
            if waende.is_empty() && raeume.is_empty() {
                hinweise.push(format!(
                    "{}: „{}“ enthält weder Wände noch Räume.",
                    g.bezeichnung, plan.bezeichnung
                ));
            }
            geschosse.push(Entwurfsgeschoss {
                id: g.id,
                name: g.bezeichnung.clone(),
                ebene_m: e,
                geschosshoehe_m: hoehe,
                deckendicke_m: decke,
                lichte_hoehe_m: geo.lichte_hoehe_m,
                waende,
                raeume,
                aussenkante,
            });
        }
        let projekt = self.get_project(p).await?;
        Ok((
            Entwurf {
                id: gebaeude_id,
                projekt: projekt.name.clone(),
                gebaeude: haus.bezeichnung.clone(),
                nullpunkt_m_nhn: haus.geometrie.as_ref().and_then(|g| g.nullpunkt_m_nhn),
                geschosse,
            },
            hinweise,
        ))
    }
}

/// Ergebnis von [`crate::CoreServices::ifc_dem_bauantrag_beifuegen`].
#[derive(Debug, Clone, PartialEq, Serialize, ts_rs::TS)]
pub struct Bauantragsanlage {
    /// `false`: Die Datei lag dem Bauantrag schon bei.
    pub neu: bool,
    /// Anlagenart (leer, wenn die Anlagenliste fehlt oder 2010 nicht kennt).
    pub anlageart: String,
    /// Zahl der Anlagen des Bauantrags.
    pub anlagen: usize,
}

impl crate::CoreServices {
    /// IFC-Datei des Projekts dem Bauantrag (XBau) als Anlage
    /// „BIM-Gebäudemodell“ beifügen. Die Abgabe nimmt die jeweils neueste
    /// Version des Dokuments.
    pub async fn ifc_dem_bauantrag_beifuegen(
        &self,
        p: crate::domain::ProjectId,
        dokument: crate::domain::DocumentId,
        bezeichnung: &str,
        wer: &crate::benutzer::Benutzer,
    ) -> CoreResult<Bauantragsanlage> {
        let d = self.repos.documents.latest(dokument).await?;
        if d.project_id != p {
            return Err(fehler("Dokument gehört zu einem anderen Projekt"));
        }
        if !super::ist_ifc(&d.file_name) {
            return Err(fehler(
                "Nur IFC-Dateien (.ifc, .ifczip) als BIM-Gebäudemodell beifügen",
            ));
        }
        if bezeichnung.chars().count() > 200 {
            return Err(fehler("Bezeichnung höchstens 200 Zeichen"));
        }
        let g = self.bauantrag(p).await?;
        if g.antrag.anlagen.iter().any(|a| a.dokument == dokument) {
            return Ok(Bauantragsanlage {
                neu: false,
                anlageart: g
                    .antrag
                    .anlagen
                    .iter()
                    .find(|a| a.dokument == dokument)
                    .map(|a| a.anlageart.clone())
                    .unwrap_or_default(),
                anlagen: g.antrag.anlagen.len(),
            });
        }
        let codelisten = self.codelisten.aktuell();
        let art = codelisten
            .liste(crate::xbau::codeliste::ANLAGEN)
            .filter(|l| l.enthaelt(crate::xbau::anlagenart::BIM_MODELL))
            .map_or(String::new(), |_| {
                crate::xbau::anlagenart::BIM_MODELL.to_string()
            });
        let mut antrag = g.antrag;
        antrag.anlagen.push(crate::xbau::Anlagenwahl {
            dokument,
            anlageart: art.clone(),
            bezeichnung: bezeichnung.trim().to_string(),
        });
        let s = self.bauantrag_speichern(p, antrag, g.version, wer).await?;
        Ok(Bauantragsanlage {
            neu: true,
            anlageart: art,
            anlagen: s.antrag.anlagen.len(),
        })
    }
}
