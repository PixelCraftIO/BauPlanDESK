//! Fachadapter liefern lesbare Berichtszeilen, ohne archivierte Ergebnisse neu zu berechnen.
use super::*;
use crate::{
    domain::Project,
    plattform::{
        nachweise::{NachweisAnsicht, NachweisEingabe, NutzungsArt},
        objekte::Raumbuch,
    },
};
use serde::Deserialize;
#[derive(Clone)]
pub(super) struct Zeile {
    pub text: String,
    pub titel: bool,
}
pub(super) fn text(t: impl Into<String>) -> Zeile {
    Zeile {
        text: t.into(),
        titel: false,
    }
}
fn titel(t: impl Into<String>) -> Zeile {
    Zeile {
        text: t.into(),
        titel: true,
    }
}
fn zahl(n: f64) -> String {
    format!("{n:.3}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}
fn optional(n: Option<f64>) -> String {
    n.map(zahl).unwrap_or_else(|| "Nicht ermittelt".into())
}
pub(super) fn nachweisstatus(n: &NachweisAnsicht) -> String {
    format!(
        "{} · {} · {}",
        if n.aktuell {
            "Aktuell"
        } else {
            "VERALTET – Grundlagen geändert"
        },
        if n.stand.vollstaendig {
            "Vollständig"
        } else {
            "UNVOLLSTÄNDIG"
        },
        if n.freigabe.is_some() {
            if n.aktuell {
                "Freigegeben"
            } else {
                "Freigabe gilt für früheren Stand"
            }
        } else {
            "Nicht freigegeben"
        }
    )
}
// Kleine Lesesicht des archivierten Ergebnisvertrags. Unbekannte Erweiterungen
// bleiben lesbar; fehlende erforderliche Ergebnisse ergeben ausdrücklich Fehler.
#[derive(Deserialize)]
struct Nutzung {
    baugrundstuecke: Vec<Einheit>,
}
#[derive(Deserialize)]
struct Einheit {
    grundstueck: crate::baugrundstuecke::BaugrundstueckGruppe,
    pruefung: Pruefung,
}
#[derive(Deserialize)]
struct Pruefung {
    kennzahlen: crate::calc::Kennzahlen,
    aufstellung: Option<crate::flaechen::FlaechenErgebnis>,
    nachweise: Vec<Vergleich>,
}
#[derive(Deserialize)]
struct Vergleich {
    art: String,
    gegenstand: Option<String>,
    ist: Option<f64>,
    soll: Option<f64>,
    eingehalten: Option<bool>,
    ueberschreitung: Option<f64>,
    flaechenueberschreitung_m2: Option<f64>,
    quelle: String,
}
pub(super) async fn erstellen(
    core: &CoreServices,
    p: &Project,
    q: &Druckquelle,
    staende: &[NachweisAnsicht],
) -> CoreResult<(String, Vec<Zeile>)> {
    let mut z = vec![text(format!(
        "Projektstand: {} · Bundesland: {}",
        p.version,
        p.bundesland.code()
    ))];
    let name = match q {
        Druckquelle::Nachweis { id } => {
            let n = staende
                .iter()
                .find(|n| n.stand.id == *id)
                .ok_or_else(|| fehler("Nachweis nicht im Projekt gefunden"))?;
            z.push(titel(nachweisstatus(n)));
            z[0] = text(format!(
                "Berechnet: {} · Bundesland: {} · Projektstand bei Berechnung: {}",
                n.stand.erstellt_am.format("%d.%m.%Y %H:%M UTC"),
                n.stand.land.code(),
                n.stand.projekt_version
            ));
            if let Some(f) = &n.freigabe {
                z.push(text(format!(
                    "Freigabe: {} · {}",
                    f.freigegeben_am.format("%d.%m.%Y %H:%M UTC"),
                    f.bemerkung
                )));
            }
            match &n.stand.eingabe {
                NachweisEingabe::Abstand {
                    gegenstand,
                    eingabe,
                } => {
                    let r: crate::lbo::Berechnung<crate::lbo::AbstandsflaecheErgebnis> =
                        serde_json::from_value(n.stand.ergebnis.clone()).map_err(fehler)?;
                    z.push(titel(gegenstand));
                    z.push(text(format!(
                        "Wandhöhe H: {} m · Baugebiet: {:?}",
                        zahl(eingabe.wandhoehe_m),
                        eingabe.gebiet
                    )));
                    z.push(text(format!(
                        "Faktor: {} · Mindesttiefe: {} m",
                        zahl(r.ergebnis.faktor),
                        zahl(r.ergebnis.mindesttiefe_m)
                    )));
                    z.push(titel(format!(
                        "Ermittelte Abstandsflächentiefe: {} m",
                        zahl(r.ergebnis.tiefe_m)
                    )));
                    z.push(text(r.ergebnis.rechtsgrundlage));
                }
                NachweisEingabe::Nutzung { fach, plan } => {
                    let r: Nutzung =
                        serde_json::from_value(n.stand.ergebnis.clone()).map_err(fehler)?;
                    for g in r.baugrundstuecke {
                        z.push(titel(&g.grundstueck.bezeichnung));
                        z.push(text(format!(
                            "Einheit: {} · Flurstücke: {}",
                            g.grundstueck.schluessel,
                            g.grundstueck.flurstueck_ids.join(", ")
                        )));
                        let eingabe = plan
                            .berechnungen
                            .iter()
                            .find(|b| b.schluessel == g.grundstueck.schluessel)
                            .ok_or_else(|| fehler("Archivierte Grundstückseingabe fehlt"))?;
                        let mass = g
                            .pruefung
                            .aufstellung
                            .as_ref()
                            .map(|a| &a.mass)
                            .unwrap_or(&eingabe.eingabe.mass);
                        z.push(text(format!(
                            "Bezugsfläche: {} m²",
                            zahl(mass.grundstuecksflaeche_m2)
                        )));
                        let k = &g.pruefung.kennzahlen;
                        match fach {
                            NutzungsArt::Grz => {
                                let ii = g.pruefung.aufstellung.as_ref().is_some_and(|a| a.grz_ii);
                                z.push(titel(format!(
                                    "{}: {}",
                                    if ii { "GRZ I" } else { "GRZ" },
                                    zahl(k.grz)
                                )));
                                z.push(text(format!(
                                    "Angerechnete Grundfläche: {} m² / {} m²",
                                    zahl(mass.grundflaeche_m2),
                                    zahl(mass.grundstuecksflaeche_m2)
                                )));
                                if ii {
                                    z.push(titel(format!("GRZ II: {}", zahl(k.grz_gesamt))));
                                    z.push(text(format!(
                                        "Zusätzliche Grundfläche nach § 19 Abs. 4: {} m²",
                                        zahl(mass.grundflaeche_nebenanlagen_m2)
                                    )));
                                }
                            }
                            NutzungsArt::Gfz => {
                                z.push(titel(format!("GFZ: {}", zahl(k.gfz))));
                                z.push(text(format!(
                                    "Angerechnete Geschossfläche: {} m² / {} m²",
                                    zahl(mass.geschossflaeche_m2),
                                    zahl(mass.grundstuecksflaeche_m2)
                                )));
                            }
                            NutzungsArt::Bmz => {
                                z.push(titel(format!("BMZ: {}", optional(k.bmz))));
                                z.push(text(format!(
                                    "Baumasse: {} m³",
                                    optional(mass.baumasse_m3)
                                )));
                            }
                            NutzungsArt::Geschossigkeit => {
                                z.push(titel(format!(
                                    "Vollgeschosse: {}",
                                    g.pruefung
                                        .aufstellung
                                        .as_ref()
                                        .map(|a| a.vollgeschosse)
                                        .or(eingabe.eingabe.vollgeschosse)
                                        .map(|n| n.to_string())
                                        .unwrap_or_else(|| "Nicht ermittelt".into())
                                )));
                            }
                        }
                        if let Some(a) = &eingabe.eingabe.aufstellung {
                            z.push(text(format!(
                                "BauNVO: {}",
                                format!("{:?}", n.stand.rechtskontext.bau_nvo.unwrap_or(a.bau_nvo))
                                    .trim_start_matches('V')
                            )));
                            if matches!(fach, NutzungsArt::Grz) {
                                z.push(titel(
                                    "Bauteilaufstellung: Eingabefläche / angerechnete Fläche",
                                ));
                                for (i, b) in a.bauteile.iter().enumerate() {
                                    let ansatz = g
                                        .pruefung
                                        .aufstellung
                                        .as_ref()
                                        .and_then(|r| r.bauteile.get(i));
                                    z.push(text(format!(
                                        "{} · {}: {} m² / {} m² · {}",
                                        b.bezeichnung,
                                        bauteiltyp(b.typ),
                                        zahl(b.flaeche_m2),
                                        optional(ansatz.map(|b| b.flaeche_m2)),
                                        ansatz
                                            .map(|a| anrechnung(a.anrechnung).to_owned())
                                            .unwrap_or_else(|| "Nicht ermittelt".into())
                                    )));
                                    if !b.begruendung.is_empty() {
                                        z.push(text(&b.begruendung));
                                    }
                                }
                            }
                            if matches!(fach, NutzungsArt::Gfz | NutzungsArt::Geschossigkeit) {
                                z.push(titel("Geschossaufstellung"));
                                for (i, s) in a.geschosse.iter().enumerate() {
                                    let fl = g
                                        .pruefung
                                        .aufstellung
                                        .as_ref()
                                        .and_then(|r| r.geschosse.get(i))
                                        .copied();
                                    z.push(text(format!(
                                        "{} · {} · Außenmaß: {} m² · angerechnet: {} m²",
                                        s.bezeichnung,
                                        if s.vollgeschoss {
                                            "Vollgeschoss"
                                        } else {
                                            "Nichtvollgeschoss"
                                        },
                                        zahl(s.aussenmass_m2),
                                        optional(fl)
                                    )));
                                    if !s.vollgeschoss {
                                        z.push(text(format!("Aufenthalt: {} m² · Wände: {} m² · Erschließung: {} m²",zahl(s.aufenthalt_m2),zahl(s.waende_m2),zahl(s.erschliessung_m2))));
                                    }
                                }
                                let fest = eingabe
                                    .eigene_festsetzungen
                                    .as_ref()
                                    .unwrap_or(&n.stand.rechtskontext.festgesetzte_werte);
                                z.push(text(format!(
                                    "Faktor Nichtvollgeschosse: {} · Festsetzung: {}",
                                    zahl(
                                        fest.nichtvollgeschoss_faktor
                                            .unwrap_or(a.nichtvollgeschoss_faktor)
                                    ),
                                    if fest.nichtvollgeschoss_faktor.is_some() {
                                        fest.nichtvollgeschoss_festsetzung.as_str()
                                    } else {
                                        a.nichtvollgeschoss_festsetzung.as_str()
                                    }
                                )));
                            }
                        }
                        z.push(titel("Vergleich mit Vorgaben"));
                        for v in g.pruefung.nachweise {
                            let relevant = match fach {
                                NutzungsArt::Grz => {
                                    v.art.contains("grz") || v.art == "grundflaeche"
                                }
                                NutzungsArt::Gfz => {
                                    v.art.contains("gfz") || v.art == "geschossflaeche"
                                }
                                NutzungsArt::Bmz => v.art == "bmz",
                                NutzungsArt::Geschossigkeit => v.art == "vollgeschosse",
                            };
                            if relevant {
                                z.push(text(format!(
                                    "{} {} · Ist: {} · Soll: {} · {} · Quelle: {}",
                                    vergleichname(&v.art),
                                    v.gegenstand.unwrap_or_default(),
                                    optional(v.ist),
                                    v.soll
                                        .map(zahl)
                                        .unwrap_or_else(|| "Nicht festgelegt".into()),
                                    match v.eingehalten {
                                        Some(true) => "Eingehalten",
                                        Some(false) => "Abweichung – prüfen",
                                        None => "Nicht abschließend prüfbar",
                                    },
                                    vergleichquelle(&v.quelle)
                                )));
                                if v.ueberschreitung.is_some_and(|n| n > 0.) {
                                    z.push(text(format!(
                                        "Überschreitung: {} · Flächenüberschreitung: {} m²",
                                        optional(v.ueberschreitung),
                                        optional(v.flaechenueberschreitung_m2)
                                    )));
                                }
                            }
                        }
                    }
                }
            }
            z.push(titel("Nachvollziehbarkeit und Regelgrundlagen"));
            z.push(text(format!(
                "Nachweis: {} · Rechenstand: {}",
                id, n.stand.rechenstand
            )));
            z.push(text(format!(
                "Grundlagen SHA-256: {}",
                n.stand.grundlagen_hash
            )));
            for r in &n.stand.regeln {
                z.push(text(format!(
                    "{} · Ausgabe {} · {}",
                    r.id, r.version, r.pruefstatus
                )));
                z.push(text(format!("SHA-256: {}", r.sha256)));
            }
            n.stand.bezeichnung.clone()
        }
        Druckquelle::Baugrundstueck => {
            let g = crate::grundstueckskontext::ermitteln(p)?;
            if g.is_empty() {
                return Err(fehler("Keine Baugrundstücksermittlung gespeichert"));
            }
            let plan = crate::grundstuecksinformationen::plan(&p.planungsrecht)?;
            for g in g {
                z.push(titel(&g.bezeichnung));
                z.push(text(format!("Einheit: {}", g.schluessel)));
                if let Some(plan) = &plan {
                    for f in plan
                        .flurstuecke
                        .iter()
                        .filter(|f| g.flurstueck_ids.contains(&f.id))
                    {
                        z.push(text(format!(
                            "Flurstück {} · Gemarkung {} · Flur {} · {} m²",
                            f.nummer,
                            f.gemarkung,
                            f.flur,
                            zahl(f.flaeche_m2)
                        )));
                        for a in &f.abzuege {
                            z.push(text(format!(
                                "Abzug {}: {} m² · {}",
                                a.art,
                                zahl(a.flaeche_m2),
                                a.grundlage
                            )));
                        }
                    }
                    if let Some(a) = plan
                        .berechnungen
                        .iter()
                        .find(|b| b.schluessel == g.schluessel)
                        .and_then(|b| b.eingabe.aufstellung.as_ref())
                        .and_then(|a| a.grundstueck.as_ref())
                    {
                        for a in &a.gemeinschaftsanteile {
                            z.push(text(format!(
                                "Gemeinschaftsfläche {}: {} m² × {} % · {}",
                                a.bezeichnung,
                                zahl(a.gesamtflaeche_m2),
                                zahl(a.anteil_prozent),
                                a.festsetzung
                            )));
                        }
                    }
                }
                z.push(text(format!(
                    "Eigentumsfläche: {} m²",
                    zahl(g.eigentumsflaeche_m2)
                )));
                z.push(text(format!(
                    "Abzüge: − {} m² · Zuschläge: + {} m²",
                    zahl(g.abzug_m2),
                    zahl(g.zuschlag_m2)
                )));
                z.push(titel(format!(
                    "Maßgebende Bezugsfläche: {} m²",
                    optional(g.bezugsflaeche_m2)
                )));
                z.push(text(g.quelle));
            }
            "Baugrundstücksermittlung".into()
        }
        Druckquelle::Raumbuch { id, revision } => {
            let f = core
                .fachobjekte(p.id, ObjektArt::Raumbuch)
                .await?
                .into_iter()
                .find(|f| f.objekt == id.to_string() && f.revision == *revision)
                .ok_or_else(|| fehler("Raumbuch geändert. Auswahl aktualisieren."))?;
            let r: Raumbuch = serde_json::from_value(f.daten).map_err(fehler)?;
            let s = r.summen()?;
            z.push(text(format!(
                "Raumbuch: {} · Revision {} · Gebäude {}",
                r.bezeichnung, revision, r.gebaeude_id
            )));
            for r in &r.raeume {
                z.push(text(format!(
                    "{} · {} · {} · {} m² · Höhe {} m",
                    r.nummer,
                    r.bezeichnung,
                    r.nutzung,
                    zahl(r.flaeche_m2),
                    optional(r.hoehe_m)
                )));
            }
            z.push(titel(format!(
                "{} Räume · Gesamtfläche: {} m² · Volumen: {} m³",
                s.anzahl,
                zahl(s.flaeche_m2),
                optional(s.volumen_m3)
            )));
            format!("Raumbuch: {}", r.bezeichnung)
        }
        Druckquelle::Baugrenzen => {
            let g = p
                .planungsrecht
                .baugrenzen
                .as_ref()
                .ok_or_else(|| fehler("Keine Baugrenzen gespeichert"))?;
            let d = core
                .document_history(g.dokument_id)
                .await?
                .into_iter()
                .find(|d| d.version == g.dokument_version && d.project_id == p.id)
                .ok_or_else(|| fehler("Baugrenzenplan gehört nicht zum Projekt"))?;
            let aktuell = core
                .document_history(g.dokument_id)
                .await?
                .iter()
                .map(|d| d.version)
                .max()
                == Some(g.dokument_version);
            let r = crate::cad::baugrenzen::pruefe(g)?;
            z.push(titel(if aktuell {
                "Gespeicherte Baugrenzenzuordnung"
            } else {
                "VERALTET – neuere Planversion vorhanden"
            }));
            z.push(text(format!(
                "Plan: {} · Version {}",
                d.file_name, g.dokument_version
            )));
            z.push(text(format!(
                "Baufenster: {} m² · Vorhaben: {} m²",
                zahl(r.baufenster_m2),
                zahl(r.vorhaben_m2)
            )));
            z.push(titel(format!(
                "Außerhalb: {} m² ({} %)",
                zahl(r.ausserhalb_m2),
                zahl(r.ausserhalb_prozent)
            )));
            z.push(text(format!(
                "Größter Eckpunktüberstand: {} m (kein Maximum entlang aller Kanten)",
                zahl(r.eckpunkt_ueberstand_m)
            )));
            z.push(text(if r.eingehalten {
                "Baugrenze eingehalten"
            } else {
                "Baugrenze überschritten – fachlich prüfen"
            }));
            "Baugrenzenprüfung".into()
        }
        Druckquelle::Dokument { .. } => {
            return Err(fehler("Dokument benötigt den Dokumentadapter"));
        }
    };
    Ok((name, z))
}
pub(super) fn email(m: &crate::email::EmailInhalt) -> Vec<Zeile> {
    let mut z = vec![titel(&m.betreff)];
    // Nur vom Mailmodul gelieferte Klartexte, niemals aktives HTML.
    z.push(text(format!(
        "Von: {}",
        m.absender
            .iter()
            .map(|a| format!("{} <{}>", a.name.clone(), a.adresse))
            .collect::<Vec<_>>()
            .join(", ")
    )));
    z.push(text(format!(
        "An: {}",
        m.empfaenger
            .iter()
            .map(|a| a.adresse.clone())
            .collect::<Vec<_>>()
            .join(", ")
    )));
    z.push(text(format!(
        "Datum: {}",
        m.datum.clone().unwrap_or_else(|| "Nicht angegeben".into())
    )));
    z.push(text(&m.text));
    for h in &m.hinweise {
        z.push(text(h));
    }
    for a in &m.anhaenge {
        z.push(text(format!("Anhang: {} (nicht mitgedruckt)", a.name)));
    }
    z
}

fn bauteiltyp(t: crate::flaechen::Bauteiltyp) -> &'static str {
    use crate::flaechen::Bauteiltyp::*;
    match t {
        Hauptanlage => "Hauptanlage",
        TerrasseHauptanlage => "Terrasse der Hauptanlage",
        Garage => "Garage",
        Stellplatz => "Stellplatz",
        Zufahrt => "Zufahrt",
        Nebenanlage14 => "Nebenanlage (§ 14)",
        Unterirdisch => "Unterirdische Anlage",
        BalkonLoggia => "Balkon / Loggia",
        TerrasseNebenanlage => "Terrasse als Nebenanlage",
    }
}
fn anrechnung(a: crate::flaechen::Anrechnung) -> &'static str {
    use crate::flaechen::Anrechnung::*;
    match a {
        Grz => "GRZ / GRZ I",
        GrzIi => "GRZ II",
        Keine => "Nicht angerechnet",
    }
}
fn vergleichname(a: &str) -> &str {
    match a {
        "grz" => "GRZ / GRZ I",
        "grz_gesamt" => "GRZ II",
        "gfz" => "GFZ",
        "bmz" => "BMZ",
        "grundflaeche" => "Grundfläche (m²)",
        "geschossflaeche" => "Geschossfläche (m²)",
        "vollgeschosse" => "Vollgeschosse",
        _ => a,
    }
}
fn vergleichquelle(a: &str) -> &str {
    match a {
        "festsetzung" => "Planfestsetzung",
        "orientierungswert" => "Orientierungswert",
        "bau_nvo19" => "§ 19 BauNVO",
        "keine" => "Keine Vorgabe",
        _ => a,
    }
}
