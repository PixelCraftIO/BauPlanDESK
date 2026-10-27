//! Aufmaß-Aufgaben: Welche Zielfelder der Flurstücke, Gebäude, manuellen
//! Abstandswände, Fachberechnungen und Raumbücher sind noch offen, welche
//! Bedeutung und Geometrie braucht eine Markierung dafür, und gibt es in diesem
//! Aufmaß bereits eine passende Markierung? Reine Auskunft für die Oberfläche;
//! Übernahme und Prüfung bleiben bei den bestehenden Diensten
//! (`planaufmass_flurstuecke`, `planaufmass_gebaeude`, `planaufmass_abstand`,
//! `planaufmass_einsetzen`, `planaufmass_raumbuch`).
//! Dazu die Werkzeugkiste: Vorlagen, die Werkzeug, Bedeutung und Zeichenebene
//! in einem Schritt vorbelegen.
use super::*;
use crate::{
    CoreServices,
    abstandsflaechen::AbstandsPlan,
    berechnungen::{BerechnungsArt, FachPlan},
    domain::ProjectId,
    plattform::ObjektArt,
    plattform::objekte::{Gebaeude, Raumbuch},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Aufgabengeometrie {
    Flaeche,
    Hoehenpunkt,
    Strecke,
}
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Planaufgabe {
    /// `flurstueck`, `abzug`, `bauteil`, `gebaeude`, `abstand`, `berechnung` oder `raumbuch`.
    pub verbraucher: String,
    /// Flurstücks-ID, Zeilenkennung des Abzugs bzw. Bauteils, Gebäude-ID, ID des
    /// Abstandsplans, der Fachberechnung oder des Raumbuchs.
    pub objekt: String,
    /// Modul einer Fachberechnung (`din277`, `wohnflaechen`, …), sonst `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub modul: Option<String>,
    pub objekt_name: String,
    /// Anzeigegruppe, z. B. „Lagerhalle · OG“ oder „Südwand“.
    pub gruppe: String,
    pub ziel: String,
    pub titel: String,
    pub groesse: Planmessgroesse,
    pub bedeutung: Planbedeutung,
    pub geometrie: Aufgabengeometrie,
    pub gebaeude_id: Option<Uuid>,
    pub geschoss_id: Option<Uuid>,
    pub grundstueck_id: Option<Uuid>,
    /// Flurstück, dem eine neue Markierung zugeordnet wird (Verbraucher `flurstueck`).
    pub flurstueck_id: Option<String>,
    /// Zielfeld trägt bereits einen (manuellen) Wert.
    pub wert_vorhanden: bool,
    /// Bereits verknüpfte Markierung (aus einem beliebigen Aufmaß).
    pub gebunden: Option<Uuid>,
    /// Passende, noch nicht verknüpfte Markierung dieses Aufmaßes.
    pub markierung: Option<Uuid>,
    pub hinweis: Option<String>,
}

fn gebaeudefeld(
    feld: &str,
    geschoss: bool,
) -> (Planbedeutung, Aufgabengeometrie, Option<&'static str>) {
    use Aufgabengeometrie::*;
    use Planbedeutung::*;
    match feld {
        "grundflaeche_m2" => (Gebaeudegrundflaeche, Flaeche, None),
        "flaeche_m2" if geschoss => (GeschossAussenkante, Flaeche, None),
        "nullpunkt_m_nhn" => (
            Nullpunkt,
            Hoehenpunkt,
            Some("Schnitt/Ansicht mit Bezugslinie im System „m ü. NHN“"),
        ),
        "gelaende_m" => (
            Gelaende,
            Hoehenpunkt,
            Some("Maßgebendes Gelände; Schnitt mit Bezugslinie in m ü. NHN"),
        ),
        "traufe_m" => (
            Traufe,
            Hoehenpunkt,
            Some("Schnitt/Ansicht mit Bezugslinie in m ü. NHN"),
        ),
        "first_m" => (
            First,
            Hoehenpunkt,
            Some("Schnitt/Ansicht mit Bezugslinie in m ü. NHN"),
        ),
        "fundamentsohle_m" => (
            Fundamentsohle,
            Hoehenpunkt,
            Some("Schnitt mit Bezugslinie in m ü. NHN"),
        ),
        "fussboden_m" => (
            Fussboden,
            Hoehenpunkt,
            Some("Schnitt mit Bezugslinie in m ü. NHN"),
        ),
        "decke_oberkante_m" => (
            Decke,
            Hoehenpunkt,
            Some("Schnitt mit Bezugslinie in m ü. NHN"),
        ),
        _ => (
            Hoehenmass,
            Strecke,
            Some("Senkrechte Strecke im Schnitt (Höhendifferenz)"),
        ),
    }
}
fn wandfeld(feld: &str) -> (Planbedeutung, Aufgabengeometrie, Option<&'static str>) {
    use Aufgabengeometrie::*;
    use Planbedeutung::*;
    match feld {
        "gelaende_a_m" | "gelaende_b_m" => (
            Gelaende,
            Hoehenpunkt,
            Some("Ansicht/Schnitt mit Bezugslinie im Höhensystem des Abstandsplans"),
        ),
        "abschluss_a_m" | "abschluss_b_m" => (
            Traufe,
            Hoehenpunkt,
            Some("Ansicht/Schnitt mit Bezugslinie im Höhensystem des Abstandsplans"),
        ),
        "dachhoehe_m" => (
            Dach,
            Strecke,
            Some("Senkrechte Strecke vom Wandabschluss bis zur Dachoberkante"),
        ),
        _ => (
            Dach,
            Strecke,
            Some("Strecke entlang der Dachhaut; Neigung zur Horizontalen"),
        ),
    }
}
fn passt(m: &Planmarkierung, geometrie: Aufgabengeometrie) -> bool {
    matches!(
        (geometrie, &m.geometrie),
        (
            Aufgabengeometrie::Flaeche,
            Plangeometrie::Polygon { .. }
                | Plangeometrie::Flaeche { .. }
                | Plangeometrie::Rechteck { .. }
                | Plangeometrie::Kreis { .. }
        ) | (
            Aufgabengeometrie::Hoehenpunkt,
            Plangeometrie::Hoehenpunkt { .. }
        ) | (Aufgabengeometrie::Strecke, Plangeometrie::Strecke { .. })
    )
}
fn gefuellt(v: &serde_json::Value, ziel: &str) -> bool {
    // Pfade sind `tabelle.id.feld` oder `geometrie.feld`.
    let t: Vec<&str> = ziel.split('.').collect();
    let feld = match t.as_slice() {
        [tabelle, id, feld] => v
            .get(tabelle)
            .and_then(|a| a.as_array())
            .and_then(|a| {
                a.iter()
                    .find(|z| z.get("id").and_then(|i| i.as_str()) == Some(id))
            })
            .and_then(|z| z.get("geometrie").or(Some(z)))
            .and_then(|z| z.get(feld))
            .cloned(),
        [gruppe, feld] => v.get(gruppe).and_then(|g| g.get(feld)).cloned(),
        _ => None,
    };
    feld.is_some_and(|f| !f.is_null())
}

/// Plausibler Bereich einer Höhenabmessung (m): Deckendicken sind dünn,
/// lichte Höhen und Geschosshöhen nicht.
fn hoehenmass_plausibel(feld: &str, h: f64) -> bool {
    match feld {
        "deckendicke_m" => h > 0. && h < 0.6,
        "lichte_hoehe_m" | "geschosshoehe_m" => h >= 0.6,
        _ => true,
    }
}
/// Namensbestandteile, an denen eine Höhenmaß-Markierung ihr Ziel erkennen lässt.
fn hoehenmass_name(feld: &str) -> Option<&'static [&'static str]> {
    match feld {
        "lichte_hoehe_m" => Some(&["licht"]),
        "geschosshoehe_m" => Some(&["geschosshöhe", "geschosshoehe"]),
        "deckendicke_m" => Some(&["decke", "kehlbalken"]),
        _ => None,
    }
}
fn benannt(m: &Planmarkierung, namen: &[&str]) -> bool {
    let b = m.bezeichnung.to_lowercase();
    namen.iter().any(|n| b.contains(n))
}

/// Zielfeld einer Fachberechnung: geforderte Bedeutung, Geometrie, Hinweis
/// und die Bedeutungen, die eine freie Markierung dafür tragen darf.
struct Berechnungsfeld {
    bedeutung: Planbedeutung,
    geometrie: Aufgabengeometrie,
    hinweis: Option<String>,
    passend: Vec<Planbedeutung>,
}
fn berechnungsfeld(
    modul: BerechnungsArt,
    tabelle: &str,
    feld: &str,
    zeile: Option<&serde_json::Value>,
    hoehenbezug: &str,
) -> Berechnungsfeld {
    use Aufgabengeometrie::*;
    use BerechnungsArt::*;
    use Planbedeutung::*;
    let text = |k: &str| {
        zeile
            .and_then(|z| z.get(k))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let einfach = |b: Planbedeutung, g, h: Option<&str>| Berechnungsfeld {
        bedeutung: b,
        geometrie: g,
        hinweis: h.map(Into::into),
        passend: vec![b],
    };
    let hoehenmass = |h: &str| einfach(Hoehenmass, Strecke, Some(h));
    match (modul, tabelle, feld) {
        (Din277, "positionen", _) => {
            let art = text("art");
            let b = match art.as_str() {
                "tf" => Tf,
                "vf" => Vf,
                "kgf" => Kgf,
                _ => Nuf,
            };
            Berechnungsfeld {
                bedeutung: b,
                geometrie: Flaeche,
                hinweis: Some(if art.is_empty() {
                    "Fläche im Grundriss; die Bedeutung NUF, TF, VF oder KGF der Markierung setzt die Art der Position".into()
                } else {
                    "Fläche im Grundriss, dem Gebäude der Berechnung zugeordnet".into()
                }),
                passend: if art.is_empty() {
                    vec![Nuf, Tf, Vf, Kgf]
                } else {
                    vec![b]
                },
            }
        }
        (Din277, "volumen", "hoehe_m") => {
            hoehenmass("Senkrechte Strecke im Schnitt: Höhe des Volumenkörpers")
        }
        (Din277, "volumen", _) => einfach(
            Gebaeudegrundflaeche,
            Flaeche,
            Some("Horizontale Projektion des Volumenkörpers im Grundriss oder Lageplan"),
        ),
        (Wohnflaechen, "positionen", "lichte_hoehe_m") => {
            hoehenmass("Senkrechte Strecke im Schnitt: lichte Höhe des Raums")
        }
        (Wohnflaechen, "positionen", _) => Berechnungsfeld {
            bedeutung: Raum,
            geometrie: Flaeche,
            hinweis: Some("Raumkontur im Grundriss, dem Gebäude der Berechnung zugeordnet".into()),
            passend: vec![Raum, Aufenthaltsraum],
        },
        (Wohnflaechen, "abzuege", _) => einfach(
            Kgf,
            Flaeche,
            Some("Grundfläche von Pfeiler, Schornstein, Vormauerung oder Treppe als Konstruktionsfläche"),
        ),
        (Freiflaechen | Entwaesserung, _, _) => {
            let typ = text("typ");
            match typ.as_str() {
                "dach" => Berechnungsfeld {
                    bedeutung: Gebaeudegrundflaeche,
                    geometrie: Flaeche,
                    hinweis: Some("Dachfläche als Gebäudegrundfläche (horizontale Projektion) eines Gebäudes, das eindeutig auf dem Baugrundstück der Bilanz steht".into()),
                    passend: vec![Gebaeudegrundflaeche, Vorhabenumriss],
                },
                "gruen" | "gruendach" => einfach(
                    PrivateGruenflaeche,
                    Flaeche,
                    Some("Grünfläche im Lageplan, dem Baugrundstück der Bilanz zugeordnet"),
                ),
                _ => Berechnungsfeld {
                    bedeutung: Versiegelung,
                    geometrie: Flaeche,
                    hinweis: Some("Befestigte oder versiegelte Fläche im Lageplan, dem Baugrundstück der Bilanz zugeordnet".into()),
                    passend: vec![Versiegelung, Zufahrt, Stellplatz, Terrasse],
                },
            }
        }
        (Vollgeschosse, "teilflaechen", "lichte_hoehe_m") => {
            hoehenmass("Senkrechte Strecke im Schnitt: lichte Höhe der Teilfläche")
        }
        (Vollgeschosse, "geschosse", "unterliegend_m2") => einfach(
            GeschossAussenkante,
            Flaeche,
            Some("Außenkante des darunterliegenden Geschosses im Grundriss"),
        ),
        (Vollgeschosse, _, _) => einfach(
            GeschossAussenkante,
            Flaeche,
            Some("Geschossfläche an der Außenkante im Grundriss, dem Geschoss zugeordnet"),
        ),
        (Hoehen, "daecher", _) => einfach(
            Dach,
            Strecke,
            Some("Strecke entlang der Dachhaut von Traufe bis First; die Horizontale wird gemessen"),
        ),
        (Hoehen, "punkte", "bezug_m") => einfach(
            Gelaende,
            Hoehenpunkt,
            Some(&format!(
                "Maßgebendes Gelände; Schnitt/Ansicht mit Bezugslinie im System „{hoehenbezug}“"
            )),
        ),
        (Hoehen, "punkte", _) => {
            let b = match text("art").as_str() {
                "traufe" => Traufe,
                _ => First,
            };
            einfach(
                b,
                Hoehenpunkt,
                Some(&format!(
                    "Schnitt/Ansicht mit Bezugslinie im System „{hoehenbezug}“"
                )),
            )
        }
        _ => einfach(Unbestimmt, Flaeche, None),
    }
}
/// Zeile `tabelle.id` der Eingaben einer Fachberechnung.
fn eingabezeile<'a>(eingaben: &'a serde_json::Value, ziel: &str) -> Option<&'a serde_json::Value> {
    let t: Vec<&str> = ziel.split('.').collect();
    let [tabelle, id, _] = t.as_slice() else {
        return None;
    };
    eingaben
        .get(tabelle)
        .and_then(|a| a.as_array())
        .and_then(|a| {
            a.iter()
                .find(|z| z.get("id").and_then(|i| i.as_str()) == Some(id))
        })
}
/// Ein Zahlenfeld gilt als erfasst, wenn es größer null ist.
fn zahl_erfasst(zeile: Option<&serde_json::Value>, feld: &str) -> bool {
    zeile
        .and_then(|z| z.get(feld))
        .is_some_and(|v| v.as_f64().is_some_and(|x| x > 0.))
}
/// Bedeutungen, die das Raumbuch als Raum übernimmt (siehe `planaufmass_raumbuch`).
fn raumbedeutung(b: Planbedeutung) -> bool {
    matches!(
        b,
        Planbedeutung::Raum
            | Planbedeutung::Aufenthaltsraum
            | Planbedeutung::Nuf
            | Planbedeutung::Tf
            | Planbedeutung::Vf
    )
}
fn nutzungsbedeutung(nutzung: &str) -> Planbedeutung {
    match nutzung.trim() {
        "Aufenthaltsraum" => Planbedeutung::Aufenthaltsraum,
        "NUF" => Planbedeutung::Nuf,
        "Technik" => Planbedeutung::Tf,
        "Verkehr" => Planbedeutung::Vf,
        _ => Planbedeutung::Raum,
    }
}

/// Vorlage der Werkzeugkiste: Ein Klick wählt Werkzeug, Bedeutung und
/// Zeichenebene zugleich; der Name neuer Markierungen beginnt mit `titel`.
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Werkzeugvorlage {
    pub id: String,
    /// Anzeigegruppe, z. B. „Grundstück“ oder „DIN 277“.
    pub gruppe: String,
    pub titel: String,
    pub bedeutung: Planbedeutung,
    pub geometrie: Aufgabengeometrie,
    /// Zeichenebene, die beim ersten Gebrauch angelegt wird.
    pub ebene: String,
    pub hinweis: String,
}
/// Feste Vorlagen für häufige Markierungen; Bedeutungen und Geometrien folgen
/// den Zielfeldern der Verbraucher.
pub fn werkzeugkiste() -> Vec<Werkzeugvorlage> {
    use Aufgabengeometrie::*;
    use Planbedeutung::*;
    let v = |id: &str, gruppe: &str, titel: &str, bedeutung, geometrie, ebene: &str, hinweis: &str| {
        Werkzeugvorlage {
            id: id.into(),
            gruppe: gruppe.into(),
            titel: titel.into(),
            bedeutung,
            geometrie,
            ebene: ebene.into(),
            hinweis: hinweis.into(),
        }
    };
    vec![
        v("flurstueck", "Grundstück", "Flurstück", Flurstueck, Flaeche, "Grundstück", "Flurstücksgrenze aus Flurkarte oder Lageplan; Flurstück in den Eigenschaften zuordnen"),
        v("baufenster", "Grundstück", "Baufenster", Baufenster, Flaeche, "Grundstück", "Baugrenzen oder Baulinien im Lageplan"),
        v("vorhabenumriss", "Grundstück", "Vorhabenumriss", Vorhabenumriss, Flaeche, "Grundstück", "Umriss des Vorhabens im Lageplan"),
        v("gruenflaeche", "Grundstück", "Private Grünfläche", PrivateGruenflaeche, Flaeche, "Grundstück", "Grünfläche für Abzug oder Freiflächenbilanz"),
        v("verkehrsflaeche", "Grundstück", "Verkehrsfläche", Verkehrsflaeche, Flaeche, "Grundstück", "Öffentliche Verkehrsfläche als Abzugsfläche"),
        v("versiegelt", "Grundstück", "Versiegelte Fläche", Versiegelung, Flaeche, "Grundstück", "Befestigte Fläche für Freiflächen- und Entwässerungsbilanz"),
        v("zufahrt", "GRZ-Bauteile", "Zufahrt", Zufahrt, Flaeche, "Bauteile", "Bauteil nach § 19 Abs. 4 BauNVO; Baugrundstück zuordnen"),
        v("stellplatz", "GRZ-Bauteile", "Stellplatz", Stellplatz, Flaeche, "Bauteile", "Bauteil nach § 19 Abs. 4 BauNVO; Baugrundstück zuordnen"),
        v("garage", "GRZ-Bauteile", "Garage / Carport", Garage, Flaeche, "Bauteile", "Bauteil nach § 19 Abs. 4 BauNVO; Baugrundstück zuordnen"),
        v("nebenanlage", "GRZ-Bauteile", "Nebenanlage", Nebenanlage, Flaeche, "Bauteile", "Nebenanlage nach § 14 BauNVO; Baugrundstück zuordnen"),
        v("terrasse", "GRZ-Bauteile", "Terrasse", Terrasse, Flaeche, "Bauteile", "Terrasse; Baugrundstück zuordnen"),
        v("unterirdisch", "GRZ-Bauteile", "Unterirdische Anlage", Unterirdisch, Flaeche, "Bauteile", "Unterirdische bauliche Anlage; Baugrundstück zuordnen"),
        v("gebaeudegrundflaeche", "Gebäude", "Gebäudegrundfläche", Gebaeudegrundflaeche, Flaeche, "Gebäude", "Horizontale Projektion des Gebäudes; Gebäude zuordnen"),
        v("geschossflaeche", "Gebäude", "Geschossfläche", GeschossAussenkante, Flaeche, "Gebäude", "Außenkante eines Geschosses; Gebäude und Geschoss zuordnen"),
        v("raum", "Räume", "Raum", Raum, Flaeche, "Räume", "Raumkontur für Raumbuch und Wohnfläche; Gebäude und Geschoss zuordnen"),
        v("aufenthaltsraum", "Räume", "Aufenthaltsraum", Aufenthaltsraum, Flaeche, "Räume", "Aufenthaltsraum für Raumbuch und Wohnfläche; Gebäude und Geschoss zuordnen"),
        v("nuf", "DIN 277", "DIN 277 NUF", Nuf, Flaeche, "DIN 277", "Nutzungsfläche; Gebäude und Geschoss zuordnen"),
        v("tf", "DIN 277", "DIN 277 TF", Tf, Flaeche, "DIN 277", "Technikfläche; Gebäude und Geschoss zuordnen"),
        v("vf", "DIN 277", "DIN 277 VF", Vf, Flaeche, "DIN 277", "Verkehrsfläche; Gebäude und Geschoss zuordnen"),
        v("kgf", "DIN 277", "DIN 277 KGF", Kgf, Flaeche, "DIN 277", "Konstruktions-Grundfläche; Gebäude und Geschoss zuordnen"),
        v("nullpunkt", "Höhen", "Nullpunkt ±0,00", Nullpunkt, Hoehenpunkt, "Höhen", "Höhenpunkt im Schnitt oder in der Ansicht mit Bezugslinie"),
        v("gelaende", "Höhen", "Gelände", Gelaende, Hoehenpunkt, "Höhen", "Maßgebendes Gelände; Höhenpunkt mit Bezugslinie"),
        v("traufe", "Höhen", "Traufe", Traufe, Hoehenpunkt, "Höhen", "Traufhöhe; Höhenpunkt mit Bezugslinie"),
        v("first", "Höhen", "First", First, Hoehenpunkt, "Höhen", "Firsthöhe; Höhenpunkt mit Bezugslinie"),
        v("fussboden", "Höhen", "OK Fußboden", Fussboden, Hoehenpunkt, "Höhen", "Oberkante Fertigfußboden eines Geschosses; Geschoss zuordnen"),
        v("decke", "Höhen", "OK Decke", Decke, Hoehenpunkt, "Höhen", "Oberkante Decke eines Geschosses; Geschoss zuordnen"),
        v("fundamentsohle", "Höhen", "Fundamentsohle", Fundamentsohle, Hoehenpunkt, "Höhen", "Unterkante Fundament; Höhenpunkt mit Bezugslinie"),
        v("hoehenmass", "Höhen", "Höhenmaß", Hoehenmass, Strecke, "Höhen", "Senkrechte Strecke im Schnitt: lichte Höhe, Geschosshöhe oder Deckendicke"),
        v("dach", "Höhen", "Dachneigung", Dach, Strecke, "Höhen", "Strecke entlang der Dachhaut von Traufe bis First"),
    ]
}

impl CoreServices {
    pub async fn planaufmass_aufgaben(
        &self,
        pid: ProjectId,
        plan: &Planaufmass,
    ) -> CoreResult<Vec<Planaufgabe>> {
        let mut aufgaben = Vec::new();
        // Gemessene Höhendifferenzen für die Plausibilität von Höhenabmessungen;
        // ein nicht messbares Aufmaß schränkt die Vorschläge nicht ein.
        let hoehen: std::collections::HashMap<Uuid, f64> = plan
            .messen()
            .map(|w| {
                w.into_iter()
                    .filter_map(|w| w.hoehe_m.map(|h| (w.id, h)))
                    .collect()
            })
            .unwrap_or_default();
        let frei = |gebunden: Option<Uuid>, geometrie, filter: &dyn Fn(&Planmarkierung) -> bool| {
            if gebunden.is_some() {
                return None;
            }
            plan.markierungen
                .iter()
                .find(|m| passt(m, geometrie) && filter(m))
                .map(|m| m.id)
        };
        // Flurstücke: eine Markierung „Flurstück“ je Flurstück, aus Flurkarte,
        // Liegenschaftskarte oder Lageplan. Ohne Zuordnung in den Eigenschaften
        // wird nichts vorgeschlagen; die Bezeichnung allein ist kein Nachweis.
        let projekt = self.get_project(pid).await?;
        let infos = crate::grundstuecksinformationen::informationen(&projekt.planungsrecht);
        let mut flurstueck_vergeben: std::collections::HashSet<Uuid> = infos
            .flurstuecke
            .iter()
            .filter_map(|f| f.planbindung.as_ref().map(|b| b.markierung_id))
            .collect();
        for (f, z) in infos.flurstuecke.iter().zip(flurstueckziele(&infos)) {
            let gebunden = f.planbindung.as_ref().map(|b| b.markierung_id);
            let markierung = frei(gebunden, Aufgabengeometrie::Flaeche, &|m| {
                m.bedeutung == Planbedeutung::Flurstueck
                    && m.flurstueck_id.as_deref() == Some(f.id.as_str())
                    && !flurstueck_vergeben.contains(&m.id)
            });
            flurstueck_vergeben.extend(markierung);
            aufgaben.push(Planaufgabe {
                verbraucher: "flurstueck".into(),
                objekt: f.id.clone(),
                modul: None,
                objekt_name: format!("Flurstück {}", f.bezeichnung()),
                gruppe: "Flurstücke".into(),
                titel: z.titel.rsplit(" · ").next().unwrap_or(&z.titel).to_string(),
                ziel: z.ziel,
                groesse: z.groesse,
                bedeutung: Planbedeutung::Flurstueck,
                geometrie: Aufgabengeometrie::Flaeche,
                gebaeude_id: None,
                geschoss_id: None,
                grundstueck_id: None,
                flurstueck_id: Some(f.id.clone()),
                wert_vorhanden: f.flaeche_m2 > 0. && !f.flaeche_aus_plan,
                gebunden,
                markierung,
                hinweis: Some(
                    "Flurstücksgrenze in Flurkarte, Liegenschaftskarte oder Lageplan; die Katasterfläche bleibt Rechenwert, solange sie erfasst ist"
                        .into(),
                ),
            });
        }
        // Abzugsflächen und GRZ-Bauteile: nur erfasste Zeilen mit Kennung. Neue
        // Zeilen entstehen aus Markierungen über „Aus Aufmaß übernehmen“
        // unter Berechnungen → Baugrundstück; dort wird auch gespeichert.
        if let Some(e) = crate::grundstuecksinformationen::ermittlung(&projekt.planungsrecht)? {
            let mut vergeben: std::collections::HashSet<Uuid> = e
                .ansaetze
                .iter()
                .flat_map(|a| a.abzuege.iter())
                .filter_map(|x| x.planbindung.as_ref().map(|b| b.markierung_id))
                .collect();
            let ziele = abzugsziele(&e, &infos);
            for a in &e.ansaetze {
                for x in &a.abzuege {
                    let (Some(id), Some(bedeutung)) =
                        (&x.id, Planbedeutung::fuer_abzugsart(&x.art))
                    else {
                        continue;
                    };
                    let Some(z) = ziele
                        .iter()
                        .find(|z| z.ziel == format!("abzuege.{id}.flaeche_m2"))
                    else {
                        continue;
                    };
                    let gebunden = x.planbindung.as_ref().map(|b| b.markierung_id);
                    let markierung = frei(gebunden, Aufgabengeometrie::Flaeche, &|m| {
                        m.bedeutung == bedeutung
                            && m.flurstueck_id.as_deref() == Some(a.flurstueck_id.as_str())
                            && !vergeben.contains(&m.id)
                    });
                    vergeben.extend(markierung);
                    let (objekt_name, titel) = z
                        .titel
                        .rsplit_once(" · ")
                        .map(|(a, b)| (a.to_string(), b.to_string()))
                        .unwrap_or_else(|| (z.titel.clone(), "Fläche".into()));
                    aufgaben.push(Planaufgabe {
                        verbraucher: "abzug".into(),
                        objekt: id.clone(),
                        modul: None,
                        objekt_name,
                        gruppe: "Abzugsflächen".into(),
                        titel,
                        ziel: z.ziel.clone(),
                        groesse: z.groesse,
                        bedeutung,
                        geometrie: Aufgabengeometrie::Flaeche,
                        gebaeude_id: None,
                        geschoss_id: None,
                        grundstueck_id: None,
                        flurstueck_id: Some(a.flurstueck_id.clone()),
                        wert_vorhanden: x.flaeche_m2 > 0.,
                        gebunden,
                        markierung,
                        hinweis: Some(
                            "Abzugsfläche im Lageplan; Übernahme unter Berechnungen → Baugrundstück „Abzugsflächen aus Aufmaß übernehmen“"
                                .into(),
                        ),
                    });
                }
            }
            for b in &e.berechnungen {
                let Some(a) = &b.eingabe.aufstellung else {
                    continue;
                };
                let grundstueck = crate::plattform::einbindung::grundstueck_id(pid, &b.schluessel);
                let mut vergeben: std::collections::HashSet<Uuid> = a
                    .bauteile
                    .iter()
                    .filter_map(|x| x.planbindung.as_ref().map(|p| p.markierung_id))
                    .collect();
                let zeilen = a
                    .bauteile
                    .iter()
                    .filter(|x| x.gebaeude_id.is_none() && x.id.is_some());
                for (x, z) in zeilen.zip(bauteilziele(a)) {
                    let Some(bedeutung) = Planbedeutung::fuer_bauteiltyp(x.typ) else {
                        continue;
                    };
                    let gebunden = x.planbindung.as_ref().map(|p| p.markierung_id);
                    let markierung = frei(gebunden, Aufgabengeometrie::Flaeche, &|m| {
                        m.bedeutung == bedeutung
                            && m.grundstueck_id == Some(grundstueck)
                            && !vergeben.contains(&m.id)
                    });
                    vergeben.extend(markierung);
                    let (objekt_name, titel) = z
                        .titel
                        .rsplit_once(" · ")
                        .map(|(a, b)| (a.to_string(), b.to_string()))
                        .unwrap_or_else(|| (z.titel.clone(), "Grundfläche".into()));
                    aufgaben.push(Planaufgabe {
                        verbraucher: "bauteil".into(),
                        objekt: x.id.clone().unwrap_or_default(),
                        modul: None,
                        objekt_name,
                        gruppe: "GRZ-Bauteile".into(),
                        titel,
                        ziel: z.ziel,
                        groesse: z.groesse,
                        bedeutung,
                        geometrie: Aufgabengeometrie::Flaeche,
                        gebaeude_id: None,
                        geschoss_id: None,
                        grundstueck_id: Some(grundstueck),
                        flurstueck_id: None,
                        wert_vorhanden: x.flaeche_m2 > 0.,
                        gebunden,
                        markierung,
                        hinweis: Some(
                            "Bauteil im Lageplan, dem Baugrundstück zugeordnet; Übernahme unter Berechnungen → Baugrundstück (GRZ) „Bauteile aus Aufmaß übernehmen“"
                                .into(),
                        ),
                    });
                }
            }
        }
        for f in self.fachobjekte(pid, ObjektArt::Gebaeude).await? {
            let id: Uuid = f
                .objekt
                .parse()
                .map_err(|_| fehler("Gebäude-ID ungültig"))?;
            let g: Gebaeude =
                serde_json::from_value(f.daten.clone()).map_err(|e| fehler(e.to_string()))?;
            // Eine Markierung versorgt höchstens ein Gebäudefeld: bereits gebundene
            // oder für ein anderes Ziel vorgeschlagene Markierungen sind vergeben.
            let mut vergeben: std::collections::HashSet<Uuid> =
                g.planbindungen.iter().map(|b| b.markierung_id).collect();
            for z in gebaeudeziele(&g) {
                let t: Vec<&str> = z.ziel.split('.').collect();
                let geschoss = t[0] == "geschosse";
                let geschoss_id = if geschoss { t[1].parse().ok() } else { None };
                let (bedeutung, geometrie, hinweis) = gebaeudefeld(t[t.len() - 1], geschoss);
                let gebunden = g
                    .planbindungen
                    .iter()
                    .find(|b| b.ziel == z.ziel)
                    .map(|b| b.markierung_id);
                let feld = t[t.len() - 1];
                let passend = |m: &Planmarkierung| {
                    m.gebaeude_id == Some(id)
                        && m.bedeutung == bedeutung
                        && (!geschoss || m.geschoss_id == geschoss_id)
                        && !vergeben.contains(&m.id)
                        && hoehen
                            .get(&m.id)
                            .is_none_or(|h| hoehenmass_plausibel(feld, *h))
                };
                // Unter mehreren passenden Markierungen die fachlich benannte bevorzugen.
                let markierung = frei(gebunden, geometrie, &|m| {
                    passend(m) && hoehenmass_name(feld).is_some_and(|n| benannt(m, n))
                })
                .or_else(|| frei(gebunden, geometrie, &passend));
                vergeben.extend(markierung);
                let gruppe = match geschoss_id
                    .and_then(|s| g.geschosse.iter().enumerate().find(|(_, x)| x.id == s))
                {
                    Some((i, s)) => format!(
                        "{} · {}",
                        g.bezeichnung,
                        if s.bezeichnung.trim().is_empty() {
                            format!("Geschoss {}", i + 1)
                        } else {
                            s.bezeichnung.clone()
                        }
                    ),
                    None => g.bezeichnung.clone(),
                };
                aufgaben.push(Planaufgabe {
                    verbraucher: "gebaeude".into(),
                    objekt: id.to_string(),
                    modul: None,
                    objekt_name: g.bezeichnung.clone(),
                    gruppe,
                    titel: z.titel.rsplit(" · ").next().unwrap_or(&z.titel).to_string(),
                    ziel: z.ziel.clone(),
                    groesse: z.groesse,
                    bedeutung,
                    geometrie,
                    gebaeude_id: Some(id),
                    geschoss_id,
                    grundstueck_id: None,
                    flurstueck_id: None,
                    wert_vorhanden: gefuellt(&f.daten, &z.ziel),
                    gebunden,
                    markierung,
                    hinweis: hinweis.map(Into::into),
                });
            }
        }
        for f in self.fachobjekte(pid, ObjektArt::Abstandsflaechen).await? {
            let id: Uuid = f
                .objekt
                .parse()
                .map_err(|_| fehler("Abstandsplan-ID ungültig"))?;
            let p: AbstandsPlan =
                serde_json::from_value(f.daten.clone()).map_err(|e| fehler(e.to_string()))?;
            for z in abstandsziele(&p) {
                let t: Vec<&str> = z.ziel.split('.').collect();
                let Some(w) = p.waende.iter().find(|w| w.id.to_string() == t[1]) else {
                    continue;
                };
                let (bedeutung, geometrie, hinweis) = wandfeld(t[2]);
                let gebunden = p
                    .planbindungen
                    .iter()
                    .find(|b| b.ziel == z.ziel)
                    .map(|b| b.markierung_id);
                let markierung = frei(gebunden, geometrie, &|m| {
                    m.gebaeude_id == w.gebaeude_id && m.bedeutung == bedeutung
                });
                let wert = match t[2] {
                    "gelaende_a_m" => w.gelaende_a_m,
                    "gelaende_b_m" => w.gelaende_b_m,
                    "abschluss_a_m" => w.abschluss_a_m,
                    "abschluss_b_m" => w.abschluss_b_m,
                    "dachhoehe_m" => w.dachhoehe_m,
                    "dachneigung_grad" => w.dachneigung_grad,
                    _ => w.dachneigung_b_grad,
                };
                aufgaben.push(Planaufgabe {
                    verbraucher: "abstand".into(),
                    objekt: id.to_string(),
                    modul: None,
                    objekt_name: p.bezeichnung.clone(),
                    gruppe: format!("{} · {}", p.bezeichnung, w.bezeichnung),
                    titel: z.titel.rsplit(" · ").next().unwrap_or(&z.titel).to_string(),
                    ziel: z.ziel.clone(),
                    groesse: z.groesse,
                    bedeutung,
                    geometrie,
                    gebaeude_id: w.gebaeude_id,
                    geschoss_id: None,
                    grundstueck_id: w.grundstueck_id,
                    flurstueck_id: None,
                    wert_vorhanden: wert.is_some(),
                    gebunden,
                    markierung,
                    hinweis: hinweis.map(Into::into),
                });
            }
        }
        // Fachberechnungen: je Position der Positivliste `planziele` eine Aufgabe.
        // Eingesetzt wird über `planaufmass_einsetzen`, gespeichert über die Fachgrenze.
        let vorlagen = crate::berechnungen::vorlagen()?;
        for art in [
            ObjektArt::Din277,
            ObjektArt::Wohnflaechen,
            ObjektArt::Freiflaechen,
            ObjektArt::Entwaesserung,
            ObjektArt::Vollgeschosse,
            ObjektArt::Hoehen,
        ] {
            for f in self.fachobjekte(pid, art).await? {
                let plan: FachPlan = serde_json::from_value(f.daten.clone())
                    .map_err(|e| fehler(e.to_string()))?;
                let modultitel = vorlagen
                    .iter()
                    .find(|v| v.plan.modul == plan.modul)
                    .map(|v| v.titel.clone())
                    .unwrap_or_else(|| art.modul().to_string());
                let objekt_name = if plan.bezeichnung.trim().is_empty() {
                    modultitel
                } else {
                    format!("{modultitel} · {}", plan.bezeichnung.trim())
                };
                let hoehenbezug = plan
                    .eingaben
                    .get("hoehenbezug")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                // Gebäudeübernahme und Aufmaß sind konkurrierende Quellen.
                let gesperrt = plan.gebaeudedaten_uebernehmen == Some(true);
                let mut vergeben: std::collections::HashSet<Uuid> =
                    plan.planbindungen.iter().map(|b| b.markierung_id).collect();
                for z in planziele(&plan)? {
                    let t: Vec<&str> = z.ziel.split('.').collect();
                    let zeile = eingabezeile(&plan.eingaben, &z.ziel);
                    let feld = berechnungsfeld(plan.modul, t[0], t[2], zeile, &hoehenbezug);
                    if feld.bedeutung == Planbedeutung::Unbestimmt {
                        continue;
                    }
                    let geschoss_id: Option<Uuid> = zeile
                        .and_then(|x| x.get("gebaeude_geschoss_id"))
                        .and_then(|v| v.as_str())
                        .and_then(|v| v.parse().ok());
                    let gebunden = plan
                        .planbindungen
                        .iter()
                        .find(|b| b.ziel == z.ziel)
                        .map(|b| b.markierung_id);
                    let uebernommen = plan.uebernahmen.iter().any(|u| u.ziel == z.ziel);
                    let passend = |m: &Planmarkierung| {
                        feld.passend.contains(&m.bedeutung)
                            && m.gebaeude_id == plan.gebaeude_id
                            && m.grundstueck_id == plan.grundstueck_id
                            && geschoss_id.is_none_or(|g| m.geschoss_id == Some(g))
                            && !vergeben.contains(&m.id)
                    };
                    // Grundstücksbilanzen dürfen die Grundfläche eines Gebäudes
                    // auf ihrem Baugrundstück binden; die Eindeutigkeit prüft die Übernahme.
                    let gebaeudeflaeche = |m: &Planmarkierung| {
                        feld.bedeutung == Planbedeutung::Gebaeudegrundflaeche
                            && plan.gebaeude_id.is_none()
                            && feld.passend.contains(&m.bedeutung)
                            && m.gebaeude_id.is_some()
                            && m.grundstueck_id.is_none_or(|g| Some(g) == plan.grundstueck_id)
                            && !vergeben.contains(&m.id)
                    };
                    let markierung = if gesperrt || uebernommen {
                        None
                    } else {
                        frei(gebunden, feld.geometrie, &passend)
                            .or_else(|| frei(gebunden, feld.geometrie, &gebaeudeflaeche))
                    };
                    vergeben.extend(markierung);
                    let hinweis = if gesperrt {
                        Some("Gebäudeübernahme aktiv: Flächen und Höhen im Gebäude aus dem Aufmaß übernehmen oder die Gebäudeübernahme in der Berechnung beenden".into())
                    } else if uebernommen {
                        Some("Wert stammt aus einem Nachweis; zuerst die Übernahme in der Berechnung lösen".into())
                    } else {
                        feld.hinweis
                    };
                    aufgaben.push(Planaufgabe {
                        verbraucher: "berechnung".into(),
                        objekt: f.objekt.clone(),
                        modul: Some(art.modul().into()),
                        objekt_name: objekt_name.clone(),
                        gruppe: objekt_name.clone(),
                        titel: z
                            .titel
                            .split_once(" · ")
                            .map(|(_, rest)| rest.to_string())
                            .unwrap_or_else(|| z.titel.clone()),
                        ziel: z.ziel.clone(),
                        groesse: z.groesse,
                        bedeutung: feld.bedeutung,
                        geometrie: feld.geometrie,
                        gebaeude_id: plan.gebaeude_id,
                        geschoss_id,
                        grundstueck_id: plan.grundstueck_id,
                        flurstueck_id: None,
                        wert_vorhanden: uebernommen || zahl_erfasst(zeile, t[2]),
                        gebunden,
                        markierung,
                        hinweis,
                    });
                }
            }
        }
        // Raumbücher: je Raum die Fläche; dazu „Räume aus dem Aufmaß“, solange
        // freie Raumkonturen für Gebäude und Geschoss vorliegen oder noch kein Raum erfasst ist.
        for f in self.fachobjekte(pid, ObjektArt::Raumbuch).await? {
            let r: Raumbuch =
                serde_json::from_value(f.daten.clone()).map_err(|e| fehler(e.to_string()))?;
            let gruppe = if r.bezeichnung.trim().is_empty() {
                "Raumbuch".to_string()
            } else {
                format!("Raumbuch · {}", r.bezeichnung.trim())
            };
            let raumfilter = |m: &Planmarkierung| {
                raumbedeutung(m.bedeutung)
                    && m.hoehenzone_von.is_none()
                    && m.gebaeude_id == Some(r.gebaeude_id)
                    && (r.geschoss_id.is_none() || m.geschoss_id == r.geschoss_id)
            };
            let mut vergeben: std::collections::HashSet<Uuid> = r
                .planbindungen
                .iter()
                .map(|b| b.markierung_id)
                .chain(r.raeume.iter().map(|x| x.id))
                .collect();
            for raum in &r.raeume {
                let ziel = format!("raeume.{}.flaeche_m2", raum.id);
                let gebunden = r
                    .planbindungen
                    .iter()
                    .find(|b| b.ziel == ziel)
                    .map(|b| b.markierung_id);
                let name = raum.bezeichnung.trim().to_lowercase();
                // Eine Raumkontur passt zu einem erfassten Raum, wenn sie seinen
                // Namen trägt (auch als Teil des von „Zeichnen“ vergebenen Namens).
                let markierung = frei(gebunden, Aufgabengeometrie::Flaeche, &|m| {
                    raumfilter(m)
                        && !vergeben.contains(&m.id)
                        && !name.is_empty()
                        && m.bezeichnung.to_lowercase().contains(&name)
                });
                vergeben.extend(markierung);
                aufgaben.push(Planaufgabe {
                    verbraucher: "raumbuch".into(),
                    objekt: f.objekt.clone(),
                    modul: None,
                    objekt_name: gruppe.clone(),
                    gruppe: gruppe.clone(),
                    titel: format!("Raum {} „{}“ · Fläche", raum.nummer, raum.bezeichnung),
                    ziel,
                    groesse: Planmessgroesse::Flaeche,
                    bedeutung: nutzungsbedeutung(&raum.nutzung),
                    geometrie: Aufgabengeometrie::Flaeche,
                    gebaeude_id: Some(r.gebaeude_id),
                    geschoss_id: r.geschoss_id,
                    grundstueck_id: None,
                    flurstueck_id: None,
                    wert_vorhanden: raum.flaeche_m2 > 0. && gebunden.is_none(),
                    gebunden,
                    markierung,
                    hinweis: Some(
                        "Raumkontur im Grundriss mit Gebäude und Geschoss; „Übernehmen“ setzt die Raumfläche, Nummer, Nutzung und Höhen bleiben"
                            .into(),
                    ),
                });
            }
            let frei_raum = plan
                .markierungen
                .iter()
                .find(|m| {
                    passt(m, Aufgabengeometrie::Flaeche) && raumfilter(m) && !vergeben.contains(&m.id)
                })
                .map(|m| m.id);
            if r.raeume.is_empty() || frei_raum.is_some() {
                aufgaben.push(Planaufgabe {
                    verbraucher: "raumbuch".into(),
                    objekt: f.objekt.clone(),
                    modul: None,
                    objekt_name: gruppe.clone(),
                    gruppe: gruppe.clone(),
                    titel: "Räume aus dem Aufmaß".into(),
                    ziel: "raeume".into(),
                    groesse: Planmessgroesse::Flaeche,
                    bedeutung: Planbedeutung::Raum,
                    geometrie: Aufgabengeometrie::Flaeche,
                    gebaeude_id: Some(r.gebaeude_id),
                    geschoss_id: r.geschoss_id,
                    grundstueck_id: None,
                    flurstueck_id: None,
                    wert_vorhanden: !r.raeume.is_empty(),
                    gebunden: None,
                    markierung: frei_raum,
                    hinweis: Some(
                        "Jede Raumkontur als Raum, Aufenthaltsraum, NUF, TF oder VF mit Gebäude und Geschoss; „Übernehmen“ legt daraus neue Räume an und aktualisiert bereits übernommene"
                            .into(),
                    ),
                });
            }
        }
        Ok(aufgaben)
    }
}
