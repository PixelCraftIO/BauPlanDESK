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
/// Präsentationsvertrag: Fachadapter liefern Absätze oder echte Tabellen.
/// Spaltenbreiten sind relative Gewichte; die PDF-Engine übernimmt Umbruch/Paginierung.
#[derive(Clone)]
pub(super) enum Zeile {
    Seitenwechsel,
    Text {
        text: String,
        titel: bool,
    },
    Tabelle {
        koepfe: Vec<String>,
        breiten: Vec<f32>,
        zeilen: Vec<Vec<String>>,
    },
}
pub(super) fn text(t: impl Into<String>) -> Zeile {
    Zeile::Text {
        text: t.into(),
        titel: false,
    }
}
pub(super) fn titel(t: impl Into<String>) -> Zeile {
    Zeile::Text {
        text: t.into(),
        titel: true,
    }
}
pub(super) fn tabelle(koepfe: &[&str], breiten: &[f32], zeilen: Vec<Vec<String>>) -> Zeile {
    Zeile::Tabelle {
        koepfe: koepfe.iter().map(|s| s.to_string()).collect(),
        breiten: breiten.to_vec(),
        zeilen,
    }
}
pub(super) fn angaben(z: &mut Vec<Zeile>, paare: Vec<(&str, String)>) {
    z.push(tabelle(
        &["Angabe", "Ansatz / Grundlage"],
        &[1., 2.4],
        paare.into_iter().map(|(k, v)| vec![k.into(), v]).collect(),
    ));
}
pub(super) fn ja(b: bool) -> &'static str {
    if b { "Ja" } else { "Nein / noch offen" }
}
fn land(l: crate::domain::Bundesland) -> String {
    use crate::domain::Bundesland::*;
    match l {
        Sh => "Schleswig-Holstein",
        Hh => "Hamburg",
        Ni => "Niedersachsen",
        Hb => "Bremen",
        Nw => "Nordrhein-Westfalen",
        He => "Hessen",
        Rp => "Rheinland-Pfalz",
        Bw => "Baden-Württemberg",
        By => "Bayern",
        Sl => "Saarland",
        Be => "Berlin",
        Bb => "Brandenburg",
        Mv => "Mecklenburg-Vorpommern",
        Sn => "Sachsen",
        St => "Sachsen-Anhalt",
        Th => "Thüringen",
    }
    .into()
}
pub(super) fn zahl(n: f64) -> String {
    format!("{n:.3}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}
pub(super) fn optional(n: Option<f64>) -> String {
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
        "Bauvorhaben: {} · {}",
        p.name,
        land(p.bundesland)
    ))];
    let name = match q {
        Druckquelle::Nachweis { id } => {
            let n = staende
                .iter()
                .find(|n| n.stand.id == *id)
                .ok_or_else(|| fehler("Nachweis nicht im Projekt gefunden"))?;
            z.push(titel(nachweisstatus(n)));
            z.push(text(format!(
                "Berechnungsdatum: {} · {}",
                n.stand.erstellt_am.format("%d.%m.%Y"),
                land(n.stand.land)
            )));
            if let Some(f) = &n.freigabe {
                z.push(text(format!(
                    "Bürofreigabe vom {}: {}",
                    f.freigegeben_am.format("%d.%m.%Y"),
                    f.bemerkung
                )));
            }
            let recht = &n.stand.rechtskontext;
            let grundlage = recht
                .rechtsgrundlage
                .map(|r| match r {
                    crate::domain::Rechtsgrundlage::Par30 => "§ 30 BauGB",
                    crate::domain::Rechtsgrundlage::Par34 => "§ 34 BauGB",
                    crate::domain::Rechtsgrundlage::Par35 => "§ 35 BauGB",
                })
                .unwrap_or("Nicht festgelegt");
            z.push(text(format!("Planungsrechtliche Grundlage: {grundlage}")));
            if !recht.plan_bezeichnung.is_empty() || !recht.plan_nummer.is_empty() {
                z.push(text(format!(
                    "Bebauungsplan: {} · Nr. {} · Rechtskraft: {}",
                    recht.plan_bezeichnung, recht.plan_nummer, recht.plan_rechtskraft
                )));
            }
            match &n.stand.eingabe {
                NachweisEingabe::AbstandProjekt { plan, .. } => {
                    let r: crate::abstandsflaechen::AbstandsErgebnis =
                        serde_json::from_value(n.stand.ergebnis.clone()).map_err(fehler)?;
                    let docs = n
                        .stand
                        .regeln
                        .iter()
                        .filter(|a| a.format == "abstandsquellen-json-1")
                        .map(|a| {
                            serde_json::from_str::<Vec<crate::domain::DocumentVersion>>(&a.inhalt)
                                .map_err(fehler)
                        })
                        .collect::<CoreResult<Vec<_>>>()?
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>();
                    super::abstandsbericht::abstand(&mut z, plan, &r, &docs)?;
                }
                NachweisEingabe::Abstand {
                    gegenstand,
                    eingabe,
                } => {
                    let r: crate::lbo::Berechnung<crate::lbo::AbstandsflaecheErgebnis> =
                        serde_json::from_value(n.stand.ergebnis.clone()).map_err(fehler)?;
                    z.push(titel(gegenstand));
                    z.push(text("Leistungsumfang: einfache Tiefenvorprüfung. Grundstücksgeometrie, Dach-/Geländeherleitung und besondere Verhältnisse sind hier nicht umfassend geprüft."));
                    angaben(
                        &mut z,
                        vec![
                            ("Wandhöhe H", format!("{} m", zahl(eingabe.wandhoehe_m))),
                            ("Baugebiet", format!("{:?}", eingabe.gebiet)),
                            ("Tiefenfaktor", zahl(r.ergebnis.faktor)),
                            (
                                "Mindesttiefe",
                                format!("{} m", zahl(r.ergebnis.mindesttiefe_m)),
                            ),
                            (
                                "Rechenweg",
                                format!(
                                    "max({} × {} m; {} m)",
                                    zahl(r.ergebnis.faktor),
                                    zahl(eingabe.wandhoehe_m),
                                    zahl(r.ergebnis.mindesttiefe_m)
                                ),
                            ),
                        ],
                    );
                    z.push(titel(format!(
                        "Abstandsflächentiefe: {} m",
                        zahl(r.ergebnis.tiefe_m)
                    )));
                    z.push(text(r.ergebnis.rechtsgrundlage));
                }
                NachweisEingabe::Nutzung { fach, plan } => {
                    let r: Nutzung =
                        serde_json::from_value(n.stand.ergebnis.clone()).map_err(fehler)?;
                    for (index, g) in r.baugrundstuecke.into_iter().enumerate() {
                        if index > 0 {
                            z.push(Zeile::Seitenwechsel);
                        }
                        z.push(titel(&g.grundstueck.bezeichnung));
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
                        let mut formeln = Vec::new();
                        match fach {
                            NutzungsArt::Grz => {
                                let ii = g.pruefung.aufstellung.as_ref().is_some_and(|a| a.grz_ii);
                                formeln.push(vec![
                                    if ii { "GRZ I" } else { "GRZ" }.into(),
                                    format!(
                                        "{} m² / {} m²",
                                        zahl(mass.grundflaeche_m2),
                                        zahl(mass.grundstuecksflaeche_m2)
                                    ),
                                    zahl(k.grz),
                                ]);
                                if ii {
                                    formeln.push(vec![
                                        "GRZ II (gesamt)".into(),
                                        format!(
                                            "({} + {}) m² / {} m²",
                                            zahl(mass.grundflaeche_m2),
                                            zahl(mass.grundflaeche_nebenanlagen_m2),
                                            zahl(mass.grundstuecksflaeche_m2)
                                        ),
                                        zahl(k.grz_gesamt),
                                    ]);
                                }
                            }
                            NutzungsArt::Gfz => formeln.push(vec![
                                "GFZ".into(),
                                format!(
                                    "{} m² / {} m²",
                                    zahl(mass.geschossflaeche_m2),
                                    zahl(mass.grundstuecksflaeche_m2)
                                ),
                                zahl(k.gfz),
                            ]),
                            NutzungsArt::Bmz => formeln.push(vec![
                                "BMZ".into(),
                                format!(
                                    "{} m³ / {} m²",
                                    optional(mass.baumasse_m3),
                                    zahl(mass.grundstuecksflaeche_m2)
                                ),
                                optional(k.bmz),
                            ]),
                            NutzungsArt::Geschossigkeit => formeln.push(vec![
                                "Vollgeschosse".into(),
                                "Anzahl der als Vollgeschoss eingeordneten Geschosse".into(),
                                g.pruefung
                                    .aufstellung
                                    .as_ref()
                                    .map(|a| a.vollgeschosse)
                                    .or(eingabe.eingabe.vollgeschosse)
                                    .map(|n| n.to_string())
                                    .unwrap_or_else(|| "Nicht ermittelt".into()),
                            ]),
                        }
                        z.push(tabelle(
                            &["Kenngröße", "Rechenansatz", "Ergebnis"],
                            &[1., 3., 1.],
                            formeln,
                        ));
                        if let Some(a) = &eingabe.eingabe.aufstellung {
                            z.push(text(format!(
                                "BauNVO: {}",
                                format!("{:?}", n.stand.rechtskontext.bau_nvo.unwrap_or(a.bau_nvo))
                                    .trim_start_matches('V')
                            )));
                            if matches!(fach, NutzungsArt::Grz) {
                                z.push(titel("Bauteilaufstellung"));
                                let mut reihen = Vec::new();
                                for (i, b) in a.bauteile.iter().enumerate() {
                                    let ansatz = g
                                        .pruefung
                                        .aufstellung
                                        .as_ref()
                                        .and_then(|r| r.bauteile.get(i));
                                    reihen.push(vec![
                                        (i + 1).to_string(),
                                        format!("{}\n{}", b.bezeichnung, bauteiltyp(b.typ)),
                                        zahl(b.flaeche_m2),
                                        optional(ansatz.map(|b| b.flaeche_m2)),
                                        ansatz
                                            .map(|a| anrechnung(a.anrechnung).to_owned())
                                            .unwrap_or_else(|| "Nicht ermittelt".into()),
                                        b.begruendung.clone(),
                                    ]);
                                }
                                z.push(tabelle(
                                    &[
                                        "Pos.",
                                        "Bauteil / Typ",
                                        "Fläche m²",
                                        "Ansatz m²",
                                        "Zuordnung",
                                        "Begründung",
                                    ],
                                    &[0.45, 1.8, 0.8, 0.8, 1.1, 1.5],
                                    reihen,
                                ));
                            }
                            if matches!(fach, NutzungsArt::Gfz | NutzungsArt::Geschossigkeit) {
                                z.push(titel("Geschossaufstellung"));
                                let mut reihen = Vec::new();
                                for (i, s) in a.geschosse.iter().enumerate() {
                                    let fl = g
                                        .pruefung
                                        .aufstellung
                                        .as_ref()
                                        .and_then(|r| r.geschosse.get(i))
                                        .copied();
                                    reihen.push(vec![
                                        s.bezeichnung.clone(),
                                        if s.vollgeschoss {
                                            "Vollgeschoss"
                                        } else {
                                            "Nichtvollgeschoss"
                                        }
                                        .into(),
                                        zahl(s.aussenmass_m2),
                                        format!(
                                            "{} / {} / {}",
                                            zahl(s.aufenthalt_m2),
                                            zahl(s.waende_m2),
                                            zahl(s.erschliessung_m2)
                                        ),
                                        optional(fl),
                                    ]);
                                }
                                z.push(tabelle(
                                    &[
                                        "Geschoss",
                                        "Einordnung",
                                        "Außenmaß m²",
                                        "Aufenthalt / Wände / Wege m²",
                                        "Ansatz m²",
                                    ],
                                    &[1.2, 1.3, 1., 1.8, 1.],
                                    reihen,
                                ));
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
                        let mut vergleiche = Vec::new();
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
                                vergleiche.push(vec![
                                    format!(
                                        "{} {}",
                                        vergleichname(&v.art),
                                        v.gegenstand.unwrap_or_default()
                                    ),
                                    optional(v.ist),
                                    v.soll
                                        .map(zahl)
                                        .unwrap_or_else(|| "Nicht festgelegt".into()),
                                    match v.eingehalten {
                                        Some(true) => "Eingehalten",
                                        Some(false) => "Abweichung – prüfen",
                                        None => "Nicht abschließend prüfbar",
                                    }
                                    .into(),
                                    vergleichquelle(&v.quelle).into(),
                                    format!(
                                        "{} / {} m²",
                                        optional(v.ueberschreitung),
                                        optional(v.flaechenueberschreitung_m2)
                                    ),
                                ]);
                            }
                        }
                        z.push(tabelle(
                            &[
                                "Kenngröße",
                                "Ist",
                                "Zulässig",
                                "Prüfergebnis",
                                "Grundlage",
                                "Überschreitung / Fläche",
                            ],
                            &[1.2, 0.7, 0.8, 1.4, 1.1, 1.3],
                            vergleiche,
                        ));
                    }
                }
            }
            z.push(titel("Regelgrundlagen und Prüfstand"));
            let mut regeln = Vec::new();
            for r in &n.stand.regeln {
                if r.format == "abstandsquellen-json-1" {
                    let docs: Vec<crate::domain::DocumentVersion> =
                        serde_json::from_str(&r.inhalt).map_err(fehler)?;
                    z.push(tabelle(
                        &["Verwendete Unterlage", "Dokumentstand"],
                        &[3., 1.],
                        docs.into_iter()
                            .map(|d| vec![d.file_name, format!("Version {}", d.version)])
                            .collect(),
                    ));
                } else {
                    regeln.push(vec![
                        regelname(&r.id),
                        r.version.clone(),
                        r.pruefstatus.clone(),
                    ]);
                }
            }
            z.push(tabelle(
                &["Regelgrundlage", "Ausgabe", "Fachlicher Prüfstand"],
                &[2., 0.8, 3.],
                regeln,
            ));
            z.push(text("Die Ausgabe gibt den gespeicherten Berechnungsstand wieder. Offene Angaben, Abweichungen und ausstehende Freigaben sind vor einer Einreichung fachlich zu klären."));
            n.stand.bezeichnung.clone()
        }
        Druckquelle::Baugrundstueck => {
            let g = crate::grundstueckskontext::ermitteln(p)?;
            if g.is_empty() {
                return Err(fehler("Keine Baugrundstücksermittlung gespeichert"));
            }
            let plan = crate::grundstuecksinformationen::plan(&p.planungsrecht)?;
            for (index, g) in g.into_iter().enumerate() {
                if index > 0 {
                    z.push(Zeile::Seitenwechsel);
                }
                z.push(titel(&g.bezeichnung));

                if let Some(plan) = &plan {
                    for f in plan
                        .flurstuecke
                        .iter()
                        .filter(|f| g.flurstueck_ids.contains(&f.id))
                    {
                        angaben(
                            &mut z,
                            vec![
                                (
                                    "Flurstück",
                                    format!(
                                        "Gemarkung {} · Flur {} · Nr. {}",
                                        f.gemarkung, f.flur, f.nummer
                                    ),
                                ),
                                ("Katasterfläche", format!("{} m²", zahl(f.flaeche_m2))),
                            ],
                        );
                        if !f.abzuege.is_empty() {
                            z.push(tabelle(
                                &["Abzugsfläche", "Fläche m²", "Rechtsgrundlage / Festsetzung"],
                                &[1.4, 0.8, 2.4],
                                f.abzuege
                                    .iter()
                                    .map(|a| {
                                        vec![
                                            abzugsart(&a.art),
                                            zahl(a.flaeche_m2),
                                            a.grundlage.clone(),
                                        ]
                                    })
                                    .collect(),
                            ));
                        }
                    }
                    if let Some(a) = plan
                        .berechnungen
                        .iter()
                        .find(|b| b.schluessel == g.schluessel)
                        .and_then(|b| b.eingabe.aufstellung.as_ref())
                        .and_then(|a| a.grundstueck.as_ref())
                    {
                        z.push(tabelle(
                            &[
                                "Gemeinschaftsfläche",
                                "Gesamt m²",
                                "Anteil %",
                                "Zuschlag m²",
                                "Festsetzung",
                            ],
                            &[1.4, 0.9, 0.8, 0.9, 1.8],
                            a.gemeinschaftsanteile
                                .iter()
                                .map(|a| {
                                    vec![
                                        a.bezeichnung.clone(),
                                        zahl(a.gesamtflaeche_m2),
                                        zahl(a.anteil_prozent),
                                        zahl(a.gesamtflaeche_m2 * a.anteil_prozent / 100.),
                                        a.festsetzung.clone(),
                                    ]
                                })
                                .collect(),
                        ));
                    }
                }
                z.push(tabelle(
                    &["Berechnungsschritt", "Fläche m²"],
                    &[3., 1.],
                    vec![
                        vec!["Summe Eigentumsflächen".into(), zahl(g.eigentumsflaeche_m2)],
                        vec!["− Flächenabzüge".into(), zahl(g.abzug_m2)],
                        vec!["+ zugeordnete Flächenanteile".into(), zahl(g.zuschlag_m2)],
                        vec![
                            "= Maßgebende Bezugsfläche".into(),
                            optional(g.bezugsflaeche_m2),
                        ],
                    ],
                ));
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
            let gebaeude = core
                .fachobjekte(p.id, ObjektArt::Gebaeude)
                .await?
                .into_iter()
                .find(|f| f.objekt == r.gebaeude_id.to_string())
                .and_then(|f| {
                    f.daten
                        .get("bezeichnung")
                        .and_then(|v| v.as_str())
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| "Nicht bezeichnet".into());
            z.push(text(format!(
                "Gebäude: {gebaeude} · Dokumentstand {revision}"
            )));
            z.push(tabelle(
                &[
                    "Raum",
                    "Bezeichnung",
                    "Nutzung",
                    "Fläche m²",
                    "Höhe m",
                    "Volumen m³",
                ],
                &[0.65, 1.6, 1.4, 0.9, 0.8, 1.],
                r.raeume
                    .iter()
                    .map(|r| {
                        vec![
                            r.nummer.clone(),
                            r.bezeichnung.clone(),
                            r.nutzung.clone(),
                            zahl(r.flaeche_m2),
                            optional(r.hoehe_m),
                            optional(r.hoehe_m.map(|h| h * r.flaeche_m2)),
                        ]
                    })
                    .collect(),
            ));
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
            angaben(
                &mut z,
                vec![
                    (
                        "Überbaubare Grundstücksfläche",
                        format!("{} m²", zahl(r.baufenster_m2)),
                    ),
                    (
                        "Grundfläche Vorhaben",
                        format!("{} m²", zahl(r.vorhaben_m2)),
                    ),
                    (
                        "Fläche außerhalb Baugrenze",
                        format!("{} m²", zahl(r.ausserhalb_m2)),
                    ),
                    (
                        "Überschreitungsanteil",
                        format!("{} %", zahl(r.ausserhalb_prozent)),
                    ),
                    (
                        "Rechenweg",
                        format!(
                            "{} m² / {} m² × 100",
                            zahl(r.ausserhalb_m2),
                            zahl(r.vorhaben_m2)
                        ),
                    ),
                    (
                        "Größter Eckpunktüberstand",
                        format!(
                            "{} m (kein Maximum entlang aller Kanten)",
                            zahl(r.eckpunkt_ueberstand_m)
                        ),
                    ),
                ],
            );
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
fn regelname(id: &str) -> String {
    if let Some(jahr) = id.strip_prefix("baunvo-") {
        format!("Baunutzungsverordnung {jahr}")
    } else if id.starts_with("abstand") {
        "Abstandsflächen – projektspezifischer Regelansatz".into()
    } else if id.starts_with("lbo-") {
        "Landesbauordnung".into()
    } else {
        "Projektbezogene Regelgrundlage".into()
    }
}
fn abzugsart(s: &str) -> String {
    match s {
        "private_gruenflaeche" => "Private Grünfläche",
        "oeffentliche_gruenflaeche" => "Öffentliche Grünfläche",
        "verkehrsflaeche" => "Verkehrsfläche",
        "strassenflaeche" => "Straßenfläche",
        "nicht_bauland" => "Nicht als Bauland ansetzbare Fläche",
        "wasserflaeche" => "Wasserfläche",
        _ => return s.replace('_', " "),
    }
    .into()
}
