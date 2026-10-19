//! DXF-Import in das neutrale [`Plan`]-Modell.

use std::collections::{BTreeMap, HashMap};
use std::f64::consts::{PI, TAU};
use std::io::Cursor;

use ::dxf::entities::{Entity, EntityType};
use ::dxf::enums::{AcadVersion, Units};
use ::dxf::{Block, Color, Drawing};

use super::{
    Ausrichtung, Beschriftung, Ebene, Laengeneinheit, Linienzug, Plan, Punkt, Raumpunkt, Rechteck,
};
use crate::error::{CoreError, CoreResult};

/// Verschachtelungstiefe für Blockreferenzen (schützt vor Zyklen).
const MAX_TIEFE: usize = 16;
/// Obergrenze für die Zahl der Punkte eines Plans (schützt vor Speicherbomben
/// durch riesige Block-Arrays).
const MAX_PUNKTE: usize = 5_000_000;
/// Größter Winkel eines Bogensegments bei der Auflösung in Polygone.
const BOGEN_SCHRITT: f64 = PI / 36.0; // 5°

/// Liest eine DXF-Datei (ASCII oder binär).
pub fn lesen(daten: &[u8]) -> CoreResult<Plan> {
    let drawing = laden(daten)?;
    // Ab AutoCAD 2007 sind Texte UTF-8. Bei ASCII-Dateien berücksichtigt
    // `dxf` das, bei binären liest es jedes Byte als eigenes Zeichen.
    let utf8_reparieren = daten.starts_with(BINAER) && drawing.header.version >= AcadVersion::R2007;
    Ok(Import::new(&drawing, utf8_reparieren).ausfuehren(&drawing))
}

fn laden(daten: &[u8]) -> CoreResult<Drawing> {
    match Drawing::load(&mut Cursor::new(daten)) {
        Ok(d) => Ok(d),
        // Ältere Programme schreiben Tabellen (v. a. Bemaßungsstile), an
        // denen `dxf` scheitert. Für die Darstellung genügt die Ebenentabelle.
        Err(e) if !daten.starts_with(BINAER) => nur_ebenentabelle(daten)
            .and_then(|bereinigt| Drawing::load(&mut Cursor::new(bereinigt)).ok())
            .ok_or_else(|| nicht_lesbar(e)),
        Err(e) => Err(nicht_lesbar(e)),
    }
}

const BINAER: &[u8] = b"AutoCAD Binary DXF";
const MAX_RAUMPUNKTE: usize = 20_000;

fn nicht_lesbar(e: ::dxf::DxfError) -> CoreError {
    CoreError::InvalidInput(format!("DXF-Datei nicht lesbar: {e}"))
}

/// Entfernt aus einer ASCII-DXF alle Tabellen außer `LAYER`. `None`, wenn
/// es nichts zu entfernen gibt.
fn nur_ebenentabelle(daten: &[u8]) -> Option<Vec<u8>> {
    let zeilen: Vec<&[u8]> = daten.split_inclusive(|&b| b == b'\n').collect();
    let wert = |i: usize| zeilen.get(i).map(|z| z.trim_ascii());
    let mut aus = Vec::with_capacity(daten.len());
    let mut entfernt = false;
    let mut i = 0;
    while i + 1 < zeilen.len() {
        let paar = (wert(i), wert(i + 1));
        if paar == (Some(b"0"), Some(b"TABLE"))
            && wert(i + 2) == Some(b"2")
            && wert(i + 3) != Some(b"LAYER")
        {
            // bis einschließlich "0 / ENDTAB" überspringen
            while i + 1 < zeilen.len()
                && (wert(i), wert(i + 1)) != (Some(b"0".as_slice()), Some(b"ENDTAB".as_slice()))
            {
                i += 2;
            }
            i += 2;
            entfernt = true;
            continue;
        }
        aus.extend_from_slice(zeilen[i]);
        aus.extend_from_slice(zeilen[i + 1]);
        i += 2;
    }
    for rest in &zeilen[i.min(zeilen.len())..] {
        aus.extend_from_slice(rest);
    }
    entfernt.then_some(aus)
}

/// 2D-Affintransformation `x' = a·x + c·y + e`, `y' = b·x + d·y + f`.
#[derive(Debug, Clone, Copy)]
struct Affin([f64; 6]);

impl Affin {
    const EINS: Affin = Affin([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    fn verschiebung(x: f64, y: f64) -> Self {
        Affin([1.0, 0.0, 0.0, 1.0, x, y])
    }

    fn drehung(grad: f64) -> Self {
        let (s, c) = grad.to_radians().sin_cos();
        Affin([c, s, -s, c, 0.0, 0.0])
    }

    fn skalierung(x: f64, y: f64) -> Self {
        Affin([x, 0.0, 0.0, y, 0.0, 0.0])
    }

    /// `self ∘ o`: erst `o`, dann `self`.
    fn nach(self, o: Affin) -> Affin {
        let [a, b, c, d, e, f] = self.0;
        let [oa, ob, oc, od, oe, of] = o.0;
        Affin([
            a * oa + c * ob,
            b * oa + d * ob,
            a * oc + c * od,
            b * oc + d * od,
            a * oe + c * of + e,
            b * oe + d * of + f,
        ])
    }

    fn punkt(&self, p: Punkt) -> Punkt {
        let [a, b, c, d, e, f] = self.0;
        Punkt(a * p.0 + c * p.1 + e, b * p.0 + d * p.1 + f)
    }

    /// Mittlerer Maßstab (für Texthöhen).
    fn massstab(&self) -> f64 {
        let [a, b, c, d, ..] = self.0;
        (a * d - b * c).abs().sqrt()
    }

    fn winkel(&self) -> f64 {
        self.0[1].atan2(self.0[0]).to_degrees()
    }
}

/// Zustand, den eine Blockreferenz an ihren Inhalt weitergibt.
#[derive(Clone, Copy)]
struct Kontext<'a> {
    transform: Affin,
    /// Ebene der Referenz; Elemente auf Ebene „0“ übernehmen sie.
    ebene: Option<&'a str>,
    /// Farbe der Referenz für Elemente mit Farbe „VonBlock“.
    farbe: Option<&'a Option<String>>,
    tiefe: usize,
}

struct Import<'d> {
    bloecke: HashMap<&'d str, &'d Block>,
    ebenen: Vec<Ebene>,
    ebenen_index: HashMap<String, usize>,
    linien: Vec<Linienzug>,
    texte: Vec<Beschriftung>,
    raumpunkte: Vec<Raumpunkt>,
    punkte: usize,
    uebersprungen: BTreeMap<&'static str, usize>,
    abgeschnitten: bool,
    zu_tief: bool,
    fehlende_bloecke: BTreeMap<String, usize>,
    utf8_reparieren: bool,
}

impl<'d> Import<'d> {
    fn new(drawing: &'d Drawing, utf8_reparieren: bool) -> Self {
        let mut import = Import {
            bloecke: drawing.blocks().map(|b| (b.name.as_str(), b)).collect(),
            ebenen: Vec::new(),
            ebenen_index: HashMap::new(),
            linien: Vec::new(),
            texte: Vec::new(),
            raumpunkte: Vec::new(),
            punkte: 0,
            uebersprungen: BTreeMap::new(),
            abgeschnitten: false,
            zu_tief: false,
            fehlende_bloecke: BTreeMap::new(),
            utf8_reparieren,
        };
        for layer in drawing.layers() {
            import.ebene_anlegen(
                &layer.name,
                aci_farbe(&layer.color),
                layer.is_layer_on && !layer.color.is_turned_off(),
            );
        }
        import
    }

    fn ausfuehren(mut self, drawing: &'d Drawing) -> Plan {
        let start = Kontext {
            transform: Affin::EINS,
            ebene: None,
            farbe: None,
            tiefe: 0,
        };
        for e in drawing.entities().filter(|e| !e.common.is_in_paper_space) {
            self.element(e, start);
        }

        let mut hinweise = Vec::new();
        let einheit = einheit(&drawing.header.default_drawing_units);
        if einheit.is_none() {
            hinweise.push(
                "Die Datei gibt keine Einheit an. Bitte die Einheit im Viewer prüfen, bevor gemessen wird."
                    .to_string(),
            );
        }
        for (typ, n) in &self.uebersprungen {
            hinweise.push(format!("{n} × {typ} wird nicht dargestellt."));
        }
        for (name, n) in &self.fehlende_bloecke {
            hinweise.push(format!(
                "Block „{name}“ fehlt in der Datei ({n} Referenzen)."
            ));
        }
        if self.zu_tief {
            hinweise.push(format!(
                "Blöcke sind tiefer als {MAX_TIEFE} Ebenen verschachtelt; tiefere Ebenen wurden ausgelassen."
            ));
        }
        if self.abgeschnitten {
            hinweise.push(format!(
                "Der Plan hat mehr als {MAX_PUNKTE} Punkte und wurde nur teilweise geladen."
            ));
        }

        let grenzen = grenzen(&self.linien, &self.texte);
        Plan {
            einheit,
            ebenen: self.ebenen,
            linien: self.linien,
            texte: self.texte,
            raumpunkte: self.raumpunkte,
            grenzen,
            hinweise,
        }
    }

    fn zeichen(&self, s: &str) -> String {
        if self.utf8_reparieren && s.chars().all(|c| u32::from(c) <= 0xff) {
            let bytes: Vec<u8> = s.chars().map(|c| c as u8).collect();
            if let Ok(utf8) = String::from_utf8(bytes) {
                return utf8;
            }
        }
        s.to_string()
    }

    fn ebene_anlegen(&mut self, name: &str, farbe: Option<String>, sichtbar: bool) -> usize {
        if let Some(&i) = self.ebenen_index.get(name) {
            return i;
        }
        let i = self.ebenen.len();
        self.ebenen.push(Ebene {
            name: self.zeichen(name),
            farbe,
            sichtbar,
        });
        self.ebenen_index.insert(name.to_string(), i);
        i
    }

    /// Höhenpunkt nur im Modellbereich selbst (nicht in Blöcken) und mit z ≠ 0.
    fn raumpunkt(&mut self, e: &Entity, k: Kontext, p: &::dxf::Point) {
        if k.tiefe > 0
            || p.z == 0.0
            || !p.z.is_finite()
            || e.common.layer.eq_ignore_ascii_case("defpoints")
            || self.raumpunkte.len() >= MAX_RAUMPUNKTE
        {
            return;
        }
        let ebene = self.ebene(e, k);
        self.raumpunkte.push(Raumpunkt {
            ebene,
            position: Punkt(p.x, p.y),
            z: p.z,
        });
    }

    fn ebene(&mut self, e: &Entity, k: Kontext) -> usize {
        let name = match (e.common.layer.as_str(), k.ebene) {
            ("0", Some(geerbt)) => geerbt,
            (eigen, _) => eigen,
        };
        // Ebenen, die nur in Elementen vorkommen, erscheinen in Vordergrundfarbe.
        self.ebene_anlegen(name, None, true)
    }

    fn farbe(&self, e: &Entity, k: Kontext) -> Option<String> {
        let c = &e.common;
        if c.color_24_bit > 0 {
            return Some(format!("#{:06x}", c.color_24_bit & 0xff_ffff));
        }
        if c.color.is_by_block() {
            return k.farbe.cloned().flatten();
        }
        aci_farbe(&c.color)
    }

    fn linie(&mut self, e: &Entity, k: Kontext, punkte: Vec<Punkt>, geschlossen: bool) {
        if punkte.len() < 2 || self.abgeschnitten {
            return;
        }
        if self.punkte + punkte.len() > MAX_PUNKTE {
            self.abgeschnitten = true;
            return;
        }
        self.punkte += punkte.len();
        let ebene = self.ebene(e, k);
        let farbe = self.farbe(e, k);
        self.linien.push(Linienzug {
            ebene,
            farbe,
            punkte: punkte.into_iter().map(|p| k.transform.punkt(p)).collect(),
            geschlossen,
        });
    }

    fn text(&mut self, e: &Entity, k: Kontext, lage: Textlage, inhalt: &str) {
        let inhalt = self.zeichen(inhalt.trim());
        if inhalt.is_empty() || !(lage.hoehe.is_finite() && lage.hoehe > 0.0) {
            return;
        }
        let zeilen = inhalt.lines().count().max(1) as f64;
        let abstand = lage.hoehe * ZEILENABSTAND * lage.zeilenfaktor;
        // Lage der ersten Grundlinie unterhalb des Einfügepunkts (in Höhen).
        let runter = match lage.vertikal {
            Vertikal::Grundlinie => 0.0,
            Vertikal::Oben => 1.0,
            Vertikal::Mitte => 1.0 - (lage.hoehe + (zeilen - 1.0) * abstand) / lage.hoehe / 2.0,
            Vertikal::Unten => -(zeilen - 1.0) * abstand / lage.hoehe,
        };
        let (sin, cos) = lage.drehung.to_radians().sin_cos();
        let d = runter * lage.hoehe;
        let pos = Punkt(lage.pos.0 + sin * d, lage.pos.1 - cos * d);

        let ebene = self.ebene(e, k);
        let farbe = self.farbe(e, k);
        let massstab = k.transform.massstab();
        self.texte.push(Beschriftung {
            ebene,
            farbe,
            position: k.transform.punkt(pos),
            hoehe: lage.hoehe * massstab,
            zeilenabstand: abstand * massstab,
            drehung: lage.drehung + k.transform.winkel(),
            ausrichtung: lage.ausrichtung,
            inhalt,
        });
    }

    fn element(&mut self, e: &'d Entity, k: Kontext) {
        if !e.common.is_visible || self.abgeschnitten {
            return;
        }
        let spiegeln = |n: &::dxf::Vector| n.z < 0.0;
        match &e.specific {
            EntityType::Line(l) => {
                self.linie(e, k, vec![p(&l.p1), p(&l.p2)], false);
            }
            EntityType::LwPolyline(l) => {
                let geschlossen = l.is_closed();
                let ecken: Vec<_> = l
                    .vertices
                    .iter()
                    .map(|v| (Punkt(v.x, v.y), v.bulge))
                    .collect();
                let mut punkte = mit_bogen(&ecken, geschlossen);
                if spiegeln(&l.extrusion_direction) {
                    punkte.iter_mut().for_each(|p| p.0 = -p.0);
                }
                self.linie(e, k, punkte, geschlossen);
            }
            EntityType::Polyline(l) => {
                // 3D-Netze (Flags 16/64) sind keine Grundrisslinien.
                if l.flags & (16 | 64) != 0 {
                    *self.uebersprungen.entry("3D-Netz").or_default() += 1;
                    return;
                }
                let geschlossen = l.is_closed();
                let ecken: Vec<_> = l
                    .vertices()
                    // Kontrollpunkte von Spline-Polylinien (Flag 16) überspringen.
                    .filter(|v| v.flags & 16 == 0)
                    .map(|v| (p(&v.location), v.bulge))
                    .collect();
                let mut punkte = mit_bogen(&ecken, geschlossen);
                if spiegeln(&l.normal) {
                    punkte.iter_mut().for_each(|p| p.0 = -p.0);
                }
                self.linie(e, k, punkte, geschlossen);
            }
            EntityType::Arc(a) => {
                let (mut start, mut ende) = (a.start_angle, a.end_angle);
                let mut mitte = p(&a.center);
                if spiegeln(&a.normal) {
                    mitte.0 = -mitte.0;
                    (start, ende) = (180.0 - ende, 180.0 - start);
                }
                let mut spanne = (ende - start).rem_euclid(360.0);
                if spanne == 0.0 {
                    spanne = 360.0;
                }
                let punkte = bogen(mitte, a.radius, start.to_radians(), spanne.to_radians());
                self.linie(e, k, punkte, false);
            }
            EntityType::Circle(c) => {
                let mut mitte = p(&c.center);
                if spiegeln(&c.normal) {
                    mitte.0 = -mitte.0;
                }
                let mut punkte = bogen(mitte, c.radius, 0.0, TAU);
                punkte.pop(); // geschlossen, letzter Punkt = erster Punkt
                self.linie(e, k, punkte, true);
            }
            EntityType::Ellipse(el) => {
                let punkte = ellipse(el);
                let voll = (el.end_parameter - el.start_parameter - TAU).abs() < 1e-9;
                let mut punkte = punkte;
                if voll {
                    punkte.pop();
                }
                self.linie(e, k, punkte, voll);
            }
            EntityType::Spline(s) => match spline(s) {
                Some(punkte) => {
                    let geschlossen = s.flags & 1 != 0;
                    self.linie(e, k, punkte, geschlossen);
                }
                None => *self.uebersprungen.entry("SPLINE (ungültig)").or_default() += 1,
            },
            EntityType::Solid(s) => {
                let ecken = vec![
                    p(&s.first_corner),
                    p(&s.second_corner),
                    p(&s.fourth_corner),
                    p(&s.third_corner),
                ];
                self.linie(e, k, ecken, true);
            }
            EntityType::Trace(s) => {
                let ecken = vec![
                    p(&s.first_corner),
                    p(&s.second_corner),
                    p(&s.fourth_corner),
                    p(&s.third_corner),
                ];
                self.linie(e, k, ecken, true);
            }
            EntityType::Text(t) => {
                let lage = Textlage::einzeilig(
                    &t.location,
                    &t.second_alignment_point,
                    t.horizontal_text_justification,
                    t.vertical_text_justification,
                    t.text_height,
                    t.rotation,
                );
                self.text(e, k, lage, &t.value);
            }
            EntityType::MText(t) => {
                let mut roh = t.extended_text.concat();
                roh.push_str(&t.text);
                let drehung = if t.x_axis_direction.x != 0.0 || t.x_axis_direction.y != 0.0 {
                    t.x_axis_direction
                        .y
                        .atan2(t.x_axis_direction.x)
                        .to_degrees()
                } else {
                    t.rotation_angle
                };
                use ::dxf::enums::AttachmentPoint as A;
                let (vertikal, ausrichtung) = match t.attachment_point {
                    A::TopLeft => (Vertikal::Oben, Ausrichtung::Links),
                    A::TopCenter => (Vertikal::Oben, Ausrichtung::Mitte),
                    A::TopRight => (Vertikal::Oben, Ausrichtung::Rechts),
                    A::MiddleLeft => (Vertikal::Mitte, Ausrichtung::Links),
                    A::MiddleCenter => (Vertikal::Mitte, Ausrichtung::Mitte),
                    A::MiddleRight => (Vertikal::Mitte, Ausrichtung::Rechts),
                    A::BottomLeft => (Vertikal::Unten, Ausrichtung::Links),
                    A::BottomCenter => (Vertikal::Unten, Ausrichtung::Mitte),
                    A::BottomRight => (Vertikal::Unten, Ausrichtung::Rechts),
                };
                let lage = Textlage {
                    pos: p(&t.insertion_point),
                    hoehe: t.initial_text_height,
                    drehung,
                    ausrichtung,
                    vertikal,
                    zeilenfaktor: if t.line_spacing_factor > 0.0 {
                        t.line_spacing_factor
                    } else {
                        1.0
                    },
                };
                self.text(e, k, lage, &mtext_klartext(&roh));
            }
            EntityType::Attribute(a) => self.attribut(e, k, a),
            EntityType::Insert(i) => {
                // Vermessungssymbole tragen die Höhe oft als z der Einfügung.
                self.raumpunkt(e, k, &i.location);
                self.referenz(e, k, &i.name, |basis| {
                    let mut ts = Vec::new();
                    let (zeilen, spalten) = (i.row_count.max(1), i.column_count.max(1));
                    for r in 0..zeilen {
                        for s in 0..spalten {
                            ts.push(
                                Affin::verschiebung(i.location.x, i.location.y)
                                    .nach(Affin::drehung(i.rotation))
                                    .nach(Affin::verschiebung(
                                        f64::from(s) * i.column_spacing,
                                        f64::from(r) * i.row_spacing,
                                    ))
                                    .nach(Affin::skalierung(i.x_scale_factor, i.y_scale_factor))
                                    .nach(Affin::verschiebung(-basis.0, -basis.1)),
                            );
                        }
                    }
                    ts
                });
                for a in i.attributes() {
                    self.attribut(e, k, a);
                }
            }
            EntityType::RotatedDimension(d) => self.bemassung(e, k, &d.dimension_base.block_name),
            EntityType::RadialDimension(d) => self.bemassung(e, k, &d.dimension_base.block_name),
            EntityType::DiameterDimension(d) => self.bemassung(e, k, &d.dimension_base.block_name),
            EntityType::AngularThreePointDimension(d) => {
                self.bemassung(e, k, &d.dimension_base.block_name)
            }
            EntityType::OrdinateDimension(d) => self.bemassung(e, k, &d.dimension_base.block_name),
            EntityType::Leader(l) => {
                let punkte = l.vertices.iter().map(p).collect();
                self.linie(e, k, punkte, false);
            }
            // Punkte in Blöcken sind fast immer Hilfspunkte von Bemaßungen;
            // freie Punkte mit Höhe sind Höhenpunkte.
            EntityType::ModelPoint(m) => self.raumpunkt(e, k, &m.location),
            EntityType::Seqend(_) | EntityType::Vertex(_) | EntityType::AttributeDefinition(_) => {}
            andere => *self.uebersprungen.entry(typname(andere)).or_default() += 1,
        }
    }

    fn attribut(&mut self, e: &Entity, k: Kontext, a: &::dxf::entities::Attribute) {
        // Flag 1: unsichtbares Attribut
        if a.flags & 1 != 0 {
            return;
        }
        let lage = Textlage::einzeilig(
            &a.location,
            &a.second_alignment_point,
            a.horizontal_text_justification,
            a.vertical_text_justification,
            a.text_height,
            a.rotation,
        );
        self.text(e, k, lage, &a.value);
    }

    /// Bemaßungen speichern ihr fertiges Erscheinungsbild als anonymen Block.
    fn bemassung(&mut self, e: &'d Entity, k: Kontext, block: &str) {
        if block.is_empty() {
            *self
                .uebersprungen
                .entry("Bemaßung ohne Darstellung")
                .or_default() += 1;
            return;
        }
        self.referenz(e, k, block, |_| vec![Affin::EINS]);
    }

    fn referenz(
        &mut self,
        e: &'d Entity,
        k: Kontext,
        name: &str,
        transformationen: impl FnOnce(Punkt) -> Vec<Affin>,
    ) {
        let Some(&block) = self.bloecke.get(name) else {
            *self.fehlende_bloecke.entry(name.to_string()).or_default() += 1;
            return;
        };
        if k.tiefe >= MAX_TIEFE {
            self.zu_tief = true;
            return;
        }
        let ebene = match (e.common.layer.as_str(), k.ebene) {
            ("0", Some(geerbt)) => geerbt,
            (eigen, _) => eigen,
        };
        let farbe = self.farbe(e, k);
        for t in transformationen(p(&block.base_point)) {
            let kind = Kontext {
                transform: k.transform.nach(t),
                ebene: Some(ebene),
                farbe: Some(&farbe),
                tiefe: k.tiefe + 1,
            };
            for be in &block.entities {
                self.element(be, kind);
            }
        }
    }
}

/// Zeilenabstand von AutoCAD bei Faktor 1, in Texthöhen.
const ZEILENABSTAND: f64 = 5.0 / 3.0;

#[derive(Clone, Copy)]
enum Vertikal {
    Grundlinie,
    Unten,
    Mitte,
    Oben,
}

struct Textlage {
    pos: Punkt,
    hoehe: f64,
    drehung: f64,
    ausrichtung: Ausrichtung,
    vertikal: Vertikal,
    zeilenfaktor: f64,
}

impl Textlage {
    /// TEXT und ATTRIB: Bei anderer Ausrichtung als „links, Grundlinie“ gilt
    /// der zweite Ausrichtungspunkt.
    fn einzeilig(
        einfuege: &::dxf::Point,
        ausricht: &::dxf::Point,
        h: ::dxf::enums::HorizontalTextJustification,
        v: ::dxf::enums::VerticalTextJustification,
        hoehe: f64,
        drehung: f64,
    ) -> Self {
        use ::dxf::enums::HorizontalTextJustification as H;
        use ::dxf::enums::VerticalTextJustification as V;
        let standard = matches!(h, H::Left) && matches!(v, V::Baseline);
        let ausrichtung = match h {
            H::Left => Ausrichtung::Links,
            H::Right => Ausrichtung::Rechts,
            H::Center | H::Middle | H::Aligned | H::Fit => Ausrichtung::Mitte,
        };
        let vertikal = match (h, v) {
            (H::Middle, _) | (_, V::Middle) => Vertikal::Mitte,
            (_, V::Top) => Vertikal::Oben,
            _ => Vertikal::Grundlinie,
        };
        Textlage {
            pos: p(if standard { einfuege } else { ausricht }),
            hoehe,
            drehung,
            ausrichtung,
            vertikal,
            zeilenfaktor: 1.0,
        }
    }
}

fn p(pt: &::dxf::Point) -> Punkt {
    Punkt(pt.x, pt.y)
}

fn einheit(u: &Units) -> Option<Laengeneinheit> {
    Some(match u {
        Units::Millimeters => Laengeneinheit::Millimeter,
        Units::Centimeters => Laengeneinheit::Zentimeter,
        Units::Decimeters => Laengeneinheit::Dezimeter,
        Units::Meters => Laengeneinheit::Meter,
        Units::Kilometers => Laengeneinheit::Kilometer,
        Units::Inches => Laengeneinheit::Zoll,
        Units::Feet => Laengeneinheit::Fuss,
        _ => return None,
    })
}

/// Punkte eines Kreisbogens ab `start` über `spanne` (Bogenmaß, positiv = CCW).
fn bogen(mitte: Punkt, radius: f64, start: f64, spanne: f64) -> Vec<Punkt> {
    let n = ((spanne.abs() / BOGEN_SCHRITT).ceil() as usize).clamp(2, 720);
    (0..=n)
        .map(|i| {
            let w = start + spanne * i as f64 / n as f64;
            Punkt(mitte.0 + radius * w.cos(), mitte.1 + radius * w.sin())
        })
        .collect()
}

/// Löst Polylinien-Segmente mit Wölbung (Bulge = tan(Öffnungswinkel/4)) auf.
fn mit_bogen(ecken: &[(Punkt, f64)], geschlossen: bool) -> Vec<Punkt> {
    let mut punkte = Vec::with_capacity(ecken.len());
    let n = ecken.len();
    let segmente = if geschlossen { n } else { n.saturating_sub(1) };
    for i in 0..n {
        let (a, wb) = ecken[i];
        punkte.push(a);
        if i >= segmente || wb.abs() < 1e-12 {
            continue;
        }
        let b = ecken[(i + 1) % n].0;
        let sehne = (b.0 - a.0).hypot(b.1 - a.1);
        if sehne == 0.0 {
            continue;
        }
        let winkel = 4.0 * wb.atan();
        let radius = sehne / (2.0 * (winkel / 2.0).sin()).abs();
        // Mittelpunkt liegt auf der Mittelsenkrechten der Sehne.
        let mx = (a.0 + b.0) / 2.0;
        let my = (a.1 + b.1) / 2.0;
        let abstand = radius * (winkel / 2.0).cos();
        let (nx, ny) = (-(b.1 - a.1) / sehne, (b.0 - a.0) / sehne);
        let vz = if wb > 0.0 { 1.0 } else { -1.0 };
        let mitte = Punkt(mx + vz * nx * abstand, my + vz * ny * abstand);
        let start = (a.1 - mitte.1).atan2(a.0 - mitte.0);
        let mut b_punkte = bogen(mitte, radius, start, winkel);
        b_punkte.pop(); // Endpunkt folgt als nächste Ecke
        punkte.extend(b_punkte.into_iter().skip(1));
    }
    if geschlossen && punkte.len() > 1 && punkte.first() == punkte.last() {
        punkte.pop();
    }
    punkte
}

fn ellipse(el: &::dxf::entities::Ellipse) -> Vec<Punkt> {
    let m = p(&el.center);
    let (ax, ay) = (el.major_axis.x, el.major_axis.y);
    let vz = if el.normal.z < 0.0 { -1.0 } else { 1.0 };
    let (bx, by) = (
        -ay * el.minor_axis_ratio * vz,
        ax * el.minor_axis_ratio * vz,
    );
    let mut spanne = el.end_parameter - el.start_parameter;
    if spanne <= 0.0 {
        spanne += TAU;
    }
    let n = ((spanne / BOGEN_SCHRITT).ceil() as usize).clamp(4, 720);
    (0..=n)
        .map(|i| {
            let t = el.start_parameter + spanne * i as f64 / n as f64;
            let (s, c) = t.sin_cos();
            Punkt(m.0 + ax * c + bx * s, m.1 + ay * c + by * s)
        })
        .collect()
}

/// Wertet einen (rationalen) B-Spline mit dem Algorithmus von de Boor aus.
fn spline(s: &::dxf::entities::Spline) -> Option<Vec<Punkt>> {
    let grad = usize::try_from(s.degree_of_curve).ok()?;
    let kp = &s.control_points;
    let knoten = &s.knot_values;
    if grad == 0 || kp.len() <= grad || knoten.len() != kp.len() + grad + 1 {
        // Nur Stützpunkte (z. B. von ezdxf oder LibreCAD): glatte Kurve
        // durch die Stützpunkte annähern.
        let stuetz: Vec<Punkt> = s.fit_points.iter().map(p).collect();
        return (stuetz.len() >= 2).then(|| catmull_rom(&stuetz));
    }
    let gewicht = |i: usize| {
        s.weight_values
            .get(i)
            .copied()
            .filter(|w| *w > 0.0)
            .unwrap_or(1.0)
    };
    let (t0, t1) = (knoten[grad], knoten[kp.len()]);
    // Auch NaN-Knoten führen hierher.
    if t1.partial_cmp(&t0) != Some(std::cmp::Ordering::Greater) {
        return None;
    }
    let n = (kp.len() * 8).clamp(16, 2000);
    let mut punkte = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let t = t0 + (t1 - t0) * i as f64 / n as f64;
        // Knotenintervall k mit knoten[k] <= t < knoten[k+1]
        let mut k = grad;
        while k + 1 < kp.len() && knoten[k + 1] <= t {
            k += 1;
        }
        let mut d: Vec<[f64; 3]> = (0..=grad)
            .map(|j| {
                let idx = j + k - grad;
                let w = gewicht(idx);
                [kp[idx].x * w, kp[idx].y * w, w]
            })
            .collect();
        for r in 1..=grad {
            for j in (r..=grad).rev() {
                let links = knoten[j + k - grad];
                let rechts = knoten[j + 1 + k - r];
                let alpha = if rechts > links {
                    (t - links) / (rechts - links)
                } else {
                    0.0
                };
                let vorher = d[j - 1];
                for (wert, alt) in d[j].iter_mut().zip(vorher) {
                    *wert = (1.0 - alpha) * alt + alpha * *wert;
                }
            }
        }
        let [x, y, w] = d[grad];
        punkte.push(Punkt(x / w, y / w));
    }
    Some(punkte)
}

/// Kurve durch alle Stützpunkte (Catmull-Rom, Endpunkte verdoppelt).
fn catmull_rom(p: &[Punkt]) -> Vec<Punkt> {
    const SCHRITTE: usize = 12;
    let n = p.len();
    let mut aus = Vec::with_capacity((n - 1) * SCHRITTE + 1);
    for i in 0..n - 1 {
        let p0 = p[i.saturating_sub(1)];
        let (p1, p2) = (p[i], p[i + 1]);
        let p3 = p[(i + 2).min(n - 1)];
        for j in 0..SCHRITTE {
            let t = j as f64 / SCHRITTE as f64;
            let (t2, t3) = (t * t, t * t * t);
            let f = |a: f64, b: f64, c: f64, d: f64| {
                0.5 * (2.0 * b
                    + (c - a) * t
                    + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
                    + (3.0 * b - a - 3.0 * c + d) * t3)
            };
            aus.push(Punkt(f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1)));
        }
    }
    aus.push(p[n - 1]);
    aus
}

/// Entfernt MTEXT-Formatierungen (`\P` Absatz, `{\fArial;…}`, `\~` usw.).
fn mtext_klartext(roh: &str) -> String {
    let mut aus = String::with_capacity(roh.len());
    let mut zeichen = roh.chars().peekable();
    while let Some(c) = zeichen.next() {
        match c {
            '{' | '}' => {}
            '\\' => match zeichen.next() {
                Some('P') | Some('X') => aus.push('\n'),
                Some('~') => aus.push(' '),
                Some(c @ ('\\' | '{' | '}')) => aus.push(c),
                Some('S') => {
                    // Stapelbruch "\S1/2;" → "1/2"
                    for c in zeichen.by_ref() {
                        if c == ';' {
                            break;
                        }
                        aus.push(if c == '^' || c == '#' { '/' } else { c });
                    }
                }
                Some('L' | 'l' | 'O' | 'o' | 'K' | 'k') | None => {}
                // Formatcodes mit Parameter bis zum Semikolon
                Some(_) => {
                    for c in zeichen.by_ref() {
                        if c == ';' {
                            break;
                        }
                    }
                }
            },
            c => aus.push(c),
        }
    }
    aus.replace("%%c", "Ø")
        .replace("%%d", "°")
        .replace("%%p", "±")
        .replace("%%C", "Ø")
        .replace("%%D", "°")
        .replace("%%P", "±")
}

fn grenzen(linien: &[Linienzug], texte: &[Beschriftung]) -> Option<Rechteck> {
    let mut alle = linien
        .iter()
        .flat_map(|l| l.punkte.iter())
        .chain(texte.iter().map(|t| &t.position))
        .filter(|p| p.0.is_finite() && p.1.is_finite());
    let erster = *alle.next()?;
    let mut r = Rechteck {
        min: erster,
        max: erster,
    };
    for p in alle {
        r.min.0 = r.min.0.min(p.0);
        r.min.1 = r.min.1.min(p.1);
        r.max.0 = r.max.0.max(p.0);
        r.max.1 = r.max.1.max(p.1);
    }
    Some(r)
}

fn typname(e: &EntityType) -> &'static str {
    match e {
        EntityType::Face3D(_) => "3DFACE",
        EntityType::Solid3D(_) => "3DSOLID",
        EntityType::Body(_) => "BODY",
        EntityType::Region(_) => "REGION",
        EntityType::Image(_) => "IMAGE (Rasterbild)",
        EntityType::Ray(_) => "RAY",
        EntityType::XLine(_) => "XLINE",
        EntityType::MLine(_) => "MLINE",
        EntityType::Helix(_) => "HELIX",
        EntityType::Wipeout(_) => "WIPEOUT",
        EntityType::PdfUnderlay(_) | EntityType::DgnUnderlay(_) | EntityType::DwfUnderlay(_) => {
            "Unterlage (PDF/DGN/DWF)"
        }
        EntityType::OleFrame(_) | EntityType::Ole2Frame(_) => "OLE-Objekt",
        EntityType::ProxyEntity(_) => "Proxy-Objekt (benutzerdefiniertes Element)",
        EntityType::ArcAlignedText(_) => "ARCALIGNEDTEXT",
        EntityType::Light(_) => "LIGHT",
        EntityType::RText(_) => "RTEXT",
        EntityType::Section(_) => "SECTION",
        EntityType::Shape(_) => "SHAPE",
        EntityType::Tolerance(_) => "TOLERANCE",
        _ => "sonstiges Element",
    }
}

/// AutoCAD-Farbindex (ACI) als `#rrggbb`. ACI 7 (schwarz/weiß je nach
/// Hintergrund) ergibt `None`, also die Vordergrundfarbe des Viewers.
fn aci_farbe(c: &Color) -> Option<String> {
    let i = c.index()?;
    let rgb: (u8, u8, u8) = match i {
        1 => (255, 0, 0),
        2 => (255, 255, 0),
        3 => (0, 255, 0),
        4 => (0, 255, 255),
        5 => (0, 0, 255),
        6 => (255, 0, 255),
        7 => return None,
        8 => (128, 128, 128),
        9 => (192, 192, 192),
        10..=249 => {
            // Farbkreis in 15°-Schritten; je Farbton fünf Helligkeiten,
            // ungerade Indizes sind entsättigt.
            let farbton = f64::from((i - 10) / 10) * 15.0;
            let stufe = (i % 10) / 2;
            let hell = [1.0, 0.8, 0.6, 0.5, 0.3][usize::from(stufe)];
            let saettigung = if i % 2 == 0 { 1.0 } else { 0.5 };
            hsv(farbton, saettigung, hell)
        }
        250..=255 => {
            let g = [51, 91, 132, 173, 214, 255][usize::from(i - 250)];
            (g, g, g)
        }
        _ => return None,
    };
    Some(format!("#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2))
}

fn hsv(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let k = |f: f64| ((f + m) * 255.0).round() as u8;
    (k(r), k(g), k(b))
}

#[cfg(test)]
mod tests {
    use super::super::{Laengeneinheit, messen, testdaten};
    use super::*;

    fn plan() -> Plan {
        lesen(&testdaten::musterplan()).unwrap()
    }

    fn ebene<'a>(plan: &'a Plan, name: &str) -> impl Iterator<Item = &'a Linienzug> + 'a {
        let i = plan.ebenen.iter().position(|e| e.name == name).unwrap();
        plan.linien.iter().filter(move |l| l.ebene == i)
    }

    #[test]
    fn einheit_und_ebenen() {
        let plan = plan();
        assert_eq!(plan.einheit, Some(Laengeneinheit::Meter));
        let grenze = plan.ebenen.iter().find(|e| e.name == "Grundstück").unwrap();
        assert_eq!(grenze.farbe.as_deref(), Some("#ff0000"));
        assert!(plan.hinweise.is_empty(), "{:?}", plan.hinweise);
    }

    #[test]
    fn grundstueck_wird_als_geschlossene_flaeche_gemessen() {
        let plan = plan();
        let grundstueck: Vec<_> = ebene(&plan, "Grundstück").collect();
        assert_eq!(grundstueck.len(), 1);
        let l = grundstueck[0];
        assert!(l.geschlossen);
        let m = messen(&l.punkte, true, Laengeneinheit::Meter).unwrap();
        assert!((m.flaeche_m2.unwrap() - 600.0).abs() < 1e-9, "{m:?}");
    }

    #[test]
    fn wölbung_wird_als_bogen_aufgeloest() {
        // Gebäude 12 × 10 m mit halbrundem Erker (Radius 2 m) an der Südseite.
        let plan = plan();
        let haus = ebene(&plan, "Gebäude").find(|l| l.geschlossen).unwrap();
        let m = messen(&haus.punkte, true, Laengeneinheit::Meter).unwrap();
        let soll = 120.0 + PI * 2.0 * 2.0 / 2.0;
        assert!((m.flaeche_m2.unwrap() - soll).abs() < 0.05, "{m:?}");
        assert!(!m.selbstschneidend);
    }

    #[test]
    fn bloecke_werden_transformiert_und_ebene_0_geerbt() {
        // Baum-Block (Kreis r = 1 auf Ebene 0) zweimal eingefügt, einmal mit Faktor 2.
        let plan = plan();
        let baeume: Vec<_> = ebene(&plan, "Grün").collect();
        assert_eq!(baeume.len(), 2);
        let mut radien: Vec<f64> = baeume
            .iter()
            .map(|l| {
                let m = messen(&l.punkte, true, Laengeneinheit::Meter).unwrap();
                (m.flaeche_m2.unwrap() / PI).sqrt()
            })
            .collect();
        radien.sort_by(f64::total_cmp);
        assert!(
            (radien[0] - 1.0).abs() < 0.01 && (radien[1] - 2.0).abs() < 0.01,
            "{radien:?}"
        );
        // Farbe „VonBlock“ übernimmt die Farbe der Referenz (ACI 3 = grün).
        assert!(baeume.iter().all(|l| l.farbe.as_deref() == Some("#00ff00")));
    }

    #[test]
    fn texte() {
        let plan = plan();
        let t = plan
            .texte
            .iter()
            .find(|t| t.inhalt.contains("Flurstück"))
            .unwrap();
        assert_eq!(t.inhalt, "Flurstück 123/4\nGemarkung Lindenweg");
        // MTEXT hängt standardmäßig oben links am Einfügepunkt (2, 27):
        // Die erste Grundlinie liegt eine Texthöhe darunter.
        assert_eq!(t.position, Punkt(2.0, 27.0 - 0.8));
        assert_eq!(t.ausrichtung, Ausrichtung::Links);
        assert!((t.zeilenabstand - 0.8 * 5.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn grenzen_umfassen_den_plan() {
        let r = plan().grenzen.unwrap();
        assert!(r.min.0 <= 0.0 && r.min.1 <= 0.0 && r.max.0 >= 20.0 && r.max.1 >= 30.0);
    }

    #[test]
    fn binaere_dxf_mit_umlauten() {
        let d = Drawing::load(&mut Cursor::new(testdaten::musterplan())).unwrap();
        let mut binaer = Vec::new();
        d.save_binary(&mut binaer).unwrap();
        assert!(binaer.starts_with(BINAER));
        let plan = lesen(&binaer).unwrap();
        assert!(plan.ebenen.iter().any(|e| e.name == "Grundstück"));
        assert!(plan.texte.iter().any(|t| t.inhalt.starts_with("Flurstück")));
    }

    #[test]
    fn unlesbarer_bemassungsstil_wird_uebersprungen() {
        // Wie von älteren Programmen geschrieben: Gruppencode 5 im
        // Bemaßungsstil enthält einen Blocknamen statt eines Handles.
        let dxf = "  0\nSECTION\n  2\nTABLES\n\
                     0\nTABLE\n  2\nDIMSTYLE\n 70\n1\n\
                     0\nDIMSTYLE\n  2\nSTANDARD\n 70\n0\n  5\nARCHTICK\n\
                     0\nENDTAB\n\
                     0\nTABLE\n  2\nLAYER\n 70\n1\n\
                     0\nLAYER\n  2\nWand\n 70\n0\n 62\n1\n  6\nCONTINUOUS\n\
                     0\nENDTAB\n  0\nENDSEC\n\
                     0\nSECTION\n  2\nENTITIES\n\
                     0\nLINE\n  8\nWand\n 10\n0\n 20\n0\n 11\n5\n 21\n0\n\
                     0\nENDSEC\n  0\nEOF\n";
        assert!(Drawing::load(&mut Cursor::new(dxf.as_bytes())).is_err());
        let plan = lesen(dxf.as_bytes()).unwrap();
        assert_eq!(plan.linien.len(), 1);
        assert_eq!(plan.ebenen[plan.linien[0].ebene].name, "Wand");
        assert_eq!(plan.ebenen[0].farbe.as_deref(), Some("#ff0000"));
    }

    #[test]
    fn unlesbare_datei() {
        assert!(lesen(b"keine dxf").is_err());
    }

    #[test]
    fn mtext_formatierung() {
        assert_eq!(
            mtext_klartext(r"{\fArial|b1;Erdgeschoss}\PWfl. 85 m\S2^;"),
            "Erdgeschoss\nWfl. 85 m2/"
        );
        assert_eq!(mtext_klartext("%%c 100"), "Ø 100");
    }

    #[test]
    fn aci_palette() {
        assert_eq!(aci_farbe(&Color::from_index(1)).as_deref(), Some("#ff0000"));
        assert_eq!(aci_farbe(&Color::from_index(7)), None);
        assert_eq!(
            aci_farbe(&Color::from_index(10)).as_deref(),
            Some("#ff0000")
        );
        assert_eq!(aci_farbe(&Color::by_layer()), None);
    }

    #[test]
    fn spline_nur_mit_stuetzpunkten() {
        let s = ::dxf::entities::Spline {
            degree_of_curve: 3,
            fit_points: vec![
                ::dxf::Point::new(0.0, 0.0, 0.0),
                ::dxf::Point::new(1.0, 1.0, 0.0),
                ::dxf::Point::new(2.0, 0.0, 0.0),
            ],
            ..Default::default()
        };
        let punkte = spline(&s).unwrap();
        assert!(punkte.len() > 3);
        // Die Kurve läuft durch alle Stützpunkte.
        for st in [Punkt(0.0, 0.0), Punkt(1.0, 1.0), Punkt(2.0, 0.0)] {
            assert!(punkte.contains(&st), "{st:?}");
        }
    }

    #[test]
    fn spline_durch_kontrollpunkte() {
        // Quadratischer B-Spline mit offenem Knotenvektor beginnt und endet
        // in den äußeren Kontrollpunkten.
        let s = ::dxf::entities::Spline {
            degree_of_curve: 2,
            control_points: vec![
                ::dxf::Point::new(0.0, 0.0, 0.0),
                ::dxf::Point::new(1.0, 2.0, 0.0),
                ::dxf::Point::new(2.0, 0.0, 0.0),
            ],
            knot_values: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            ..Default::default()
        };
        let punkte = spline(&s).unwrap();
        assert_eq!(punkte.first(), Some(&Punkt(0.0, 0.0)));
        let ende = punkte.last().unwrap();
        assert!((ende.0 - 2.0).abs() < 1e-9 && ende.1.abs() < 1e-9);
        // Scheitel bei t = 0.5: (1, 1)
        let mitte = punkte[punkte.len() / 2];
        assert!((mitte.0 - 1.0).abs() < 1e-9 && (mitte.1 - 1.0).abs() < 1e-9);
    }
}
