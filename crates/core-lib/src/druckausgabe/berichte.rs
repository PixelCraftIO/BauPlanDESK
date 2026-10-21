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
    /// Nummerierter Hauptabschnitt; `Text { titel: true }` wird darunter zum Unterabschnitt.
    Abschnitt(String),
    /// Hervorgehobener Hinweis für Status, Abweichungen und Vorbehalte.
    Hinweis(String),
    /// Unterschriftsfelder je Rolle: (Rolle, Name).
    Unterschrift(Vec<(String, String)>),
}
pub(super) fn abschnitt(t: impl Into<String>) -> Zeile {
    Zeile::Abschnitt(t.into())
}
pub(super) fn hinweis(t: impl Into<String>) -> Zeile {
    Zeile::Hinweis(t.into())
}
/// Querbezüge innerhalb eines Pakets: Anlagennummern der beigefügten Grundlagen
/// und die Personen für den Aufstellungsvermerk.
#[derive(Default)]
pub(super) struct Bezuege {
    pub nachweise: std::collections::HashMap<uuid::Uuid, u32>,
    pub raumbuecher: std::collections::HashMap<uuid::Uuid, u32>,
    pub baugrundstueck: Option<u32>,
    /// Beigefügte Dokumentversionen (z. B. Pläne, aus denen PDF-Aufmaße
    /// oder Quellunterlagen Berechnungen speisen).
    pub dokumente: std::collections::HashMap<(crate::domain::DocumentId, i64), u32>,
    pub verfasser: Vec<(String, String)>,
}
impl Bezuege {
    fn anlage(a: Option<&u32>) -> String {
        a.map(|a| format!("Anlage {a}"))
            .unwrap_or_else(|| "Nicht beigefügt".into())
    }
    fn dokument(&self, d: &crate::plattform::objekte::Dokumentbezug) -> Option<u32> {
        self.dokumente.get(&(d.dokument_id, d.version)).copied()
    }
}
/// Dokumentversion, auf der ein gebundener PDF-Aufmaßstand beruht.
async fn aufmassdokument(
    core: &CoreServices,
    p: &Project,
    b: &crate::planaufmass::Planbindung,
) -> CoreResult<Option<crate::plattform::objekte::Dokumentbezug>> {
    Ok(core
        .fachverlauf(p.id, crate::plattform::ObjektArt::Planaufmass, b.aufmass_id)
        .await?
        .into_iter()
        .find(|f| f.revision == b.revision)
        .and_then(|f| f.daten.get("dokument").cloned())
        .and_then(|d| serde_json::from_value(d).ok()))
}
/// Gebäude des Projekts nach Objekt-ID. Der Druck benennt damit Gebäude, die
/// archivierte Nutzungsnachweise nur über ihre ID führen; ein aktueller
/// Nachweis beruht auf genau diesem Gebäudestand.
async fn gebaeude(
    core: &CoreServices,
    p: &Project,
) -> CoreResult<std::collections::HashMap<String, crate::plattform::objekte::Gebaeude>> {
    Ok(core
        .fachobjekte(p.id, crate::plattform::ObjektArt::Gebaeude)
        .await?
        .into_iter()
        .filter_map(|f| Some((f.objekt, serde_json::from_value(f.daten).ok()?)))
        .collect())
}
/// Lesbarer Gegenstand eines Vergleichs: Gebäudebezeichnung statt Gebäude-ID.
fn gegenstandsname(
    g: Option<&str>,
    gebaeude: &std::collections::HashMap<String, crate::plattform::objekte::Gebaeude>,
) -> String {
    match g {
        None => String::new(),
        Some(id) => match gebaeude.get(id) {
            Some(g) => g.bezeichnung.clone(),
            None if uuid::Uuid::parse_str(id).is_ok() => "Gebäude (nicht mehr erfasst)".into(),
            None => id.to_string(),
        },
    }
}
/// Ergebnis eines Fachadapters.
pub(super) struct Fachbericht {
    pub titel: String,
    pub zeilen: Vec<Zeile>,
    /// Anlagen, auf denen dieser Bericht aufbaut.
    pub verweise: Vec<u32>,
    /// Stand für den Kopfblock.
    pub stand: String,
    /// Bezeichnung in Kopfblock und Anlagenverzeichnis (Fachtitel, ggf. mit
    /// der vergebenen Nachweisbezeichnung).
    pub verzeichnisname: String,
}
/// Amtlich verständlicher Titel je Nachweisart.
pub(super) fn fachtitel(e: &NachweisEingabe) -> &'static str {
    match e {
        NachweisEingabe::Nutzung { fach, .. } => match fach {
            NutzungsArt::Grz => "Nachweis der Grundflächenzahl (GRZ, § 19 BauNVO)",
            NutzungsArt::Gfz => "Nachweis der Geschossflächenzahl (GFZ, § 20 BauNVO)",
            NutzungsArt::Bmz => "Nachweis der Baumassenzahl (BMZ, § 21 BauNVO)",
            NutzungsArt::Geschossigkeit => "Nachweis der Zahl der Vollgeschosse",
        },
        NachweisEingabe::AbstandProjekt { .. } => "Abstandsflächennachweis",
        NachweisEingabe::Abstand { .. } => "Abstandsflächen – Tiefenvorprüfung",
        NachweisEingabe::Fach { plan, .. } => plan.modul.titel(),
    }
}
/// Fachtitel, ergänzt um eine aussagekräftige freie Bezeichnung.
pub(super) fn benennung(fach: &str, bezeichnung: &str) -> String {
    let b = bezeichnung.trim();
    let einfach = |t: &str| {
        t.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
    };
    let (f, k) = (einfach(fach), einfach(b));
    // Leere oder rein wiederholende Bezeichnungen („GRZ“, „Grz Berechnung“).
    let wiederholt = k.is_empty()
        || f.contains(&k)
        || ["berechnung", "nachweis"].iter().any(|w| {
            k.strip_suffix(w)
                .is_some_and(|r| !r.is_empty() && f.contains(r))
        });
    if wiederholt {
        fach.into()
    } else {
        format!("{fach} – {b}")
    }
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
    /// Fehlt in Nachweisständen vor Maßstab 3.
    #[serde(default)]
    hinweise: Vec<String>,
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
    bezuege: &Bezuege,
) -> CoreResult<Fachbericht> {
    let mut z = Vec::new();
    let mut verweise = Vec::new();
    let mut stand = format!("Projektstand {}", p.version);
    let mut verzeichnisname: Option<String> = None;
    let name = match q {
        Druckquelle::Nachweis { id } => {
            let n = staende
                .iter()
                .find(|n| n.stand.id == *id)
                .ok_or_else(|| fehler("Nachweis nicht im Projekt gefunden"))?;
            let datum = crate::kalender::datum_deutsch(n.stand.erstellt_am).format("%d.%m.%Y");
            stand = format!("Nachweis vom {datum} · {}", nachweisstatus(n));
            let mut warnung = Vec::new();
            if !n.aktuell {
                warnung.push("VERALTET: Die zugrunde liegenden Angaben wurden nach der Berechnung geändert. Vor einer Einreichung neu berechnen.");
            }
            if !n.stand.vollstaendig {
                warnung.push("UNVOLLSTÄNDIG: Offene Prüfpunkte und nicht abschließend bewertete Ergebnisse (z. B. Grenzfälle) sind unten ausgewiesen.");
            }
            if !warnung.is_empty() {
                z.push(hinweis(warnung.join(" ")));
            }
            z.push(abschnitt("Angaben zum Nachweis"));
            let recht = &n.stand.rechtskontext;
            let grundlage = recht
                .rechtsgrundlage
                .map(|r| match r {
                    crate::domain::Rechtsgrundlage::Par30 => "§ 30 BauGB",
                    crate::domain::Rechtsgrundlage::Par34 => "§ 34 BauGB",
                    crate::domain::Rechtsgrundlage::Par35 => "§ 35 BauGB",
                })
                .unwrap_or("Nicht festgelegt");
            let mut paare = vec![
                (
                    "Bauvorhaben",
                    format!("{} · {}", p.name, land(p.bundesland)),
                ),
                ("Nachweis", n.stand.bezeichnung.clone()),
                (
                    "Berechnungsdatum",
                    format!("{datum} · {}", land(n.stand.land)),
                ),
                ("Status", nachweisstatus(n)),
                ("Planungsrechtliche Grundlage", grundlage.into()),
            ];
            if !recht.plan_bezeichnung.is_empty() || !recht.plan_nummer.is_empty() {
                paare.push((
                    "Bebauungsplan",
                    format!(
                        "{} · Nr. {} · Rechtskraft: {}",
                        recht.plan_bezeichnung, recht.plan_nummer, recht.plan_rechtskraft
                    ),
                ));
            }
            if let Some(f) = &n.freigabe {
                paare.push((
                    "Bürofreigabe",
                    format!(
                        "{}: {}",
                        crate::kalender::datum_deutsch(f.freigegeben_am).format("%d.%m.%Y"),
                        f.bemerkung
                    ),
                ));
            }
            angaben(&mut z, paare);
            match &n.stand.eingabe {
                NachweisEingabe::Fach { plan, .. } => {
                    let r: crate::berechnungen::FachErgebnis =
                        serde_json::from_value(n.stand.ergebnis.clone()).map_err(fehler)?;
                    z.push(abschnitt("Regelgrundlagen"));
                    angaben(
                        &mut z,
                        vec![
                            ("Berechnung", plan.modul.titel().into()),
                            (
                                "Regelfassung",
                                format!(
                                    "{} · Version {}",
                                    plan.regelkontext.bezeichnung, plan.regelkontext.version
                                ),
                            ),
                            ("Geltungsbereich", plan.regelkontext.geltungsbereich.clone()),
                            ("Regelquelle", plan.regelkontext.quelle.clone()),
                            (
                                "Gültig ab / bis",
                                format!(
                                    "{} / {}",
                                    plan.regelkontext.gueltig_ab, plan.regelkontext.gueltig_bis
                                ),
                            ),
                            ("Prüfvermerk", plan.regelkontext.pruefvermerk.clone()),
                        ],
                    );
                    z.push(abschnitt("Eingangswerte und Rechengang"));
                    for t in &r.tabellen {
                        // A4 benötigt lesbare Spalten. Breite Fachverträge bleiben unverändert;
                        // der Druckadapter führt ihre erste Identifikationsspalte in jeder Teiltabelle fort.
                        if t.spalten.len() > 6 {
                            let gruppen = (1..t.spalten.len()).collect::<Vec<_>>();
                            for (teil, spalten) in gruppen.chunks(5).enumerate() {
                                z.push(titel(format!("{} · Teil {}", t.titel, teil + 1)));
                                let index = std::iter::once(0)
                                    .chain(spalten.iter().copied())
                                    .collect::<Vec<_>>();
                                let sp = index
                                    .iter()
                                    .map(|i| t.spalten[*i].as_str())
                                    .collect::<Vec<_>>();
                                let mut breiten = vec![1.; index.len()];
                                breiten[0] = 1.5;
                                let rows = t
                                    .zeilen
                                    .iter()
                                    .map(|r| index.iter().map(|i| r[*i].clone()).collect())
                                    .collect();
                                z.push(tabelle(&sp, &breiten, rows));
                            }
                        } else {
                            z.push(titel(&t.titel));
                            let sp = t.spalten.iter().map(String::as_str).collect::<Vec<_>>();
                            let breiten = t.spalten.iter().map(|_| 1.).collect::<Vec<_>>();
                            z.push(tabelle(&sp, &breiten, t.zeilen.clone()));
                        }
                    }
                    z.push(abschnitt("Ergebnisse"));
                    z.push(tabelle(
                        &["Kennzahl", "Wert", "Einheit"],
                        &[3., 1., 1.],
                        r.kennzahlen
                            .iter()
                            .map(|k| {
                                vec![
                                    k.bezeichnung.clone(),
                                    k.wert.map(zahl).unwrap_or_else(|| "Nicht ermittelt".into()),
                                    k.einheit.text().into(),
                                ]
                            })
                            .collect(),
                    ));
                    if !r.offen.is_empty() || !r.hinweise.is_empty() {
                        z.push(abschnitt("Offene Prüfpunkte und Hinweise"));
                        for h in &r.offen {
                            z.push(hinweis(format!("Offener Prüfpunkt: {h}")));
                        }
                        for h in &r.hinweise {
                            z.push(text(h));
                        }
                    }
                    if !plan.uebernahmen.is_empty()
                        || !plan.fachquellen.is_empty()
                        || !plan.quellen.is_empty()
                        || !plan.planbindungen.is_empty()
                    {
                        z.push(abschnitt("Übernommene Grundlagen und Quellen"));
                    }
                    if !plan.uebernahmen.is_empty() {
                        z.push(titel("Übernommene Berechnungsgrundlagen"));
                        let mut quellen = Vec::new();
                        for u in &plan.uebernahmen {
                            let quelle = staende
                                .iter()
                                .find(|s| s.stand.id == u.nachweis_id)
                                .ok_or_else(|| fehler("Archivierter Quellnachweis fehlt"))?;
                            let e: crate::berechnungen::FachErgebnis =
                                serde_json::from_value(quelle.stand.ergebnis.clone())
                                    .map_err(fehler)?;
                            let k = e
                                .kennzahlen
                                .iter()
                                .find(|k| k.id == u.kennzahl)
                                .ok_or_else(|| fehler("Archivierte Quellkennzahl fehlt"))?;
                            if let Some(a) = bezuege.nachweise.get(&u.nachweis_id) {
                                verweise.push(*a);
                            }
                            quellen.push(vec![
                                Bezuege::anlage(bezuege.nachweise.get(&u.nachweis_id)),
                                quelle.stand.bezeichnung.clone(),
                                quelle
                                    .stand
                                    .erstellt_am
                                    .format("%d.%m.%Y %H:%M UTC")
                                    .to_string(),
                                k.bezeichnung.clone(),
                                format!(
                                    "{} {}",
                                    k.wert.map(zahl).unwrap_or_else(|| "Nicht ermittelt".into()),
                                    k.einheit.text()
                                ),
                            ]);
                        }
                        z.push(tabelle(
                            &[
                                "Anlage",
                                "Quellnachweis",
                                "Berechnungsstand",
                                "Bezugsgröße",
                                "Übernommener Wert",
                            ],
                            &[0.8, 2., 1.5, 2., 1.],
                            quellen,
                        ));
                    }
                    if !plan.planbindungen.is_empty() {
                        // Herkunft aus dem gebundenen Aufmaßstand, nicht aus dem aktuellen.
                        z.push(titel("Messwerte aus PDF-Aufmaß"));
                        let ziele = crate::planaufmass::planziele(plan).unwrap_or_default();
                        let mut zeilen = Vec::new();
                        for b in &plan.planbindungen {
                            let stand = core
                                .fachverlauf(
                                    p.id,
                                    crate::plattform::ObjektArt::Planaufmass,
                                    b.aufmass_id,
                                )
                                .await?
                                .into_iter()
                                .find(|f| f.revision == b.revision)
                                .and_then(|f| {
                                    serde_json::from_value::<crate::planaufmass::Planaufmass>(
                                        f.daten,
                                    )
                                    .ok()
                                });
                            let anlage = stand.as_ref().and_then(|a| bezuege.dokument(&a.dokument));
                            if let Some(a) = anlage {
                                verweise.push(a);
                            }
                            let (aufmass, markierung, unterlage) = match &stand {
                                Some(a) => (
                                    a.bezeichnung.clone(),
                                    a.markierungen
                                        .iter()
                                        .find(|m| m.id == b.markierung_id)
                                        .map(|m| m.bezeichnung.clone())
                                        .unwrap_or_else(|| "Markierung fehlt".into()),
                                    match anlage {
                                        Some(n) => format!(
                                            "Version {} · Seite {} · Anlage {n}",
                                            a.dokument.version, a.seite
                                        ),
                                        None => format!(
                                            "Version {} · Seite {}",
                                            a.dokument.version, a.seite
                                        ),
                                    },
                                ),
                                None => ("Aufmaßstand fehlt".into(), String::new(), String::new()),
                            };
                            let ziel = ziele
                                .iter()
                                .find(|z| z.ziel == b.ziel)
                                .map(|z| z.titel.clone())
                                .unwrap_or_else(|| b.ziel.clone());
                            zeilen.push(vec![
                                aufmass,
                                markierung,
                                unterlage,
                                format!("R{}", b.revision),
                                ziel,
                            ]);
                        }
                        z.push(tabelle(
                            &[
                                "PDF-Aufmaß",
                                "Markierung",
                                "PDF-Unterlage",
                                "Stand",
                                "Zielfeld",
                            ],
                            &[1.8, 1.8, 1.4, 0.6, 2.4],
                            zeilen,
                        ));
                    }
                    for q in &plan.fachquellen {
                        let quelle = core
                            .fachverlauf(p.id, q.art, q.objekt)
                            .await?
                            .into_iter()
                            .find(|f| f.revision == q.revision)
                            .ok_or_else(|| fehler("Archivierte Raumdatenquelle fehlt"))?;
                        let raumbuch: crate::plattform::objekte::Raumbuch =
                            serde_json::from_value(quelle.daten).map_err(fehler)?;
                        if let Some(a) = bezuege.raumbuecher.get(&q.objekt) {
                            verweise.push(*a);
                        }
                        z.push(text(format!("Raumdatenquelle: {} · Revision {} ({}). Raumklassifikation und Anrechnung erfolgen in diesem Nachweis.", raumbuch.bezeichnung, q.revision, Bezuege::anlage(bezuege.raumbuecher.get(&q.objekt)))));
                    }
                    for d in &plan.quellen {
                        let m = core
                            .document_history(d.dokument_id)
                            .await?
                            .into_iter()
                            .find(|m| m.project_id == p.id && m.version == d.version)
                            .ok_or_else(|| fehler("Archivierte Berechnungsquelle fehlt"))?;
                        let anlage = bezuege.dokument(d);
                        if let Some(a) = anlage {
                            verweise.push(a);
                        }
                        z.push(text(format!(
                            "Quellunterlage: {} · Version {} ({})",
                            m.file_name,
                            d.version,
                            Bezuege::anlage(anlage.as_ref())
                        )));
                    }
                }
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
                    verweise.extend(docs.iter().filter_map(|d| {
                        bezuege.dokumente.get(&(d.document_id, d.version)).copied()
                    }));
                    super::abstandsbericht::abstand(&mut z, plan, &r, &docs)?;
                }
                NachweisEingabe::Abstand {
                    gegenstand,
                    eingabe,
                } => {
                    let r: crate::lbo::Berechnung<crate::lbo::AbstandsflaecheErgebnis> =
                        serde_json::from_value(n.stand.ergebnis.clone()).map_err(fehler)?;
                    z.push(abschnitt(format!("Tiefenvorprüfung: {gegenstand}")));
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
                    let gebaeude = gebaeude(core, p).await?;
                    for (index, g) in r.baugrundstuecke.into_iter().enumerate() {
                        if index > 0 {
                            z.push(Zeile::Seitenwechsel);
                        }
                        z.push(abschnitt(format!(
                            "Baugrundstück: {}",
                            g.grundstueck.bezeichnung
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
                        z.push(titel("Bezugsfläche, Rechenansatz und Ergebnis"));
                        z.push(text(format!(
                            "Maßgebende Bezugsfläche: {} m² · Herleitung: Baugrundstücksermittlung ({})",
                            zahl(mass.grundstuecksflaeche_m2),
                            Bezuege::anlage(bezuege.baugrundstueck.as_ref())
                        )));
                        if let Some(a) = bezuege.baugrundstueck {
                            verweise.push(a);
                        }
                        // Gebäude der Aufstellung mit ihren Plangrundlagen und,
                        // für GFZ und Geschossigkeit, der Vollgeschosseinstufung.
                        let ids: std::collections::BTreeSet<String> = eingabe
                            .eingabe
                            .aufstellung
                            .iter()
                            .flat_map(|a| {
                                a.bauteile
                                    .iter()
                                    .filter_map(|b| b.gebaeude_id.map(|i| i.to_string()))
                                    .chain(a.geschosse.iter().filter_map(|s| s.gebaeude_id.clone()))
                            })
                            .collect();
                        let mehrere_gebaeude = ids.len() > 1;
                        let geschossig =
                            matches!(fach, NutzungsArt::Gfz | NutzungsArt::Geschossigkeit);
                        let mut grundlagen = Vec::new();
                        for id in &ids {
                            let Some(geb) = gebaeude.get(id) else {
                                continue;
                            };
                            let mut plaene: Vec<u32> = geb
                                .quelle
                                .iter()
                                .filter_map(|d| bezuege.dokument(d))
                                .collect();
                            for b in &geb.planbindungen {
                                if let Some(d) = aufmassdokument(core, p, b).await?
                                    && let Some(a) = bezuege.dokument(&d)
                                {
                                    plaene.push(a);
                                }
                            }
                            plaene.sort_unstable();
                            plaene.dedup();
                            verweise.extend(&plaene);
                            let mut einstufung = Vec::new();
                            if geschossig {
                                let quellen: std::collections::BTreeSet<uuid::Uuid> = eingabe
                                    .eingabe
                                    .aufstellung
                                    .iter()
                                    .flat_map(|a| &a.geschosse)
                                    .filter(|s| s.gebaeude_id.as_deref() == Some(id.as_str()))
                                    .filter_map(|s| s.geschoss_id.as_ref())
                                    .filter_map(|sid| {
                                        geb.geschosse.iter().find(|x| x.id.to_string() == *sid)
                                    })
                                    .filter_map(|x| x.einstufungsquelle.as_ref())
                                    .map(|q| q.nachweis_id)
                                    .collect();
                                for q in quellen {
                                    let name = staende
                                        .iter()
                                        .find(|s| s.stand.id == q)
                                        .map(|s| s.stand.bezeichnung.clone())
                                        .unwrap_or_else(|| "Vollgeschossnachweis".into());
                                    let anlage = bezuege.nachweise.get(&q);
                                    if let Some(a) = anlage {
                                        verweise.push(*a);
                                    }
                                    einstufung
                                        .push(format!("{name} ({})", Bezuege::anlage(anlage)));
                                }
                            }
                            if plaene.is_empty() && einstufung.is_empty() {
                                continue;
                            }
                            grundlagen.push(vec![
                                geb.bezeichnung.clone(),
                                if plaene.is_empty() {
                                    "–".into()
                                } else {
                                    plaene
                                        .iter()
                                        .map(|a| format!("Anlage {a}"))
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                },
                                if einstufung.is_empty() {
                                    "–".into()
                                } else {
                                    einstufung.join("; ")
                                },
                            ]);
                        }
                        if !grundlagen.is_empty() {
                            z.push(tabelle(
                                &["Gebäude", "Planunterlage", "Vollgeschosseinstufung"],
                                &[1.2, 1.4, 2.4],
                                grundlagen,
                            ));
                        }
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
                            let recht = &n.stand.rechtskontext;
                            z.push(text(match recht.massgebende_bau_nvo(Some(a.bau_nvo)) {
                                Some(f) => format!("BauNVO: {}", f.jahr()),
                                None => format!(
                                    "BauNVO: Fassung des Bebauungsplans nicht erfasst (Flächen vorläufig nach {} angerechnet)",
                                    a.bau_nvo.jahr()
                                ),
                            }));
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
                                        if mehrere_gebaeude {
                                            format!(
                                                "{} · {}",
                                                gegenstandsname(
                                                    s.gebaeude_id
                                                        .as_deref()
                                                        .or(Some("Bestandsgebäude")),
                                                    &gebaeude
                                                ),
                                                s.bezeichnung
                                            )
                                        } else {
                                            s.bezeichnung.clone()
                                        },
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
                        for h in &g.pruefung.hinweise {
                            z.push(text(format!("Hinweis: {h}")));
                        }
                        let mut vergleiche = Vec::new();
                        for v in g.pruefung.nachweise {
                            let relevant = match fach {
                                NutzungsArt::Grz => {
                                    v.art.contains("grz")
                                        || v.art == "grundflaeche"
                                        || v.art == "baufenster_lage"
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
                                        gegenstandsname(v.gegenstand.as_deref(), &gebaeude)
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
            z.push(abschnitt("Regelgrundlagen und Prüfstand"));
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
                } else if r.format == crate::plattform::nachweise::ABSTANDSGEBAEUDE {
                    let stand: std::collections::BTreeMap<
                        String,
                        Option<crate::plattform::nachweise::AbstandsGebaeudehoehen>,
                    > = serde_json::from_str(&r.inhalt).map_err(fehler)?;
                    let m = |v: Option<f64>| {
                        v.map(|v| format!("{} m", zahl(v)))
                            .unwrap_or_else(|| "–".into())
                    };
                    z.push(tabelle(
                        &[
                            "Verknüpftes Gebäude",
                            "±0,00 in NHN",
                            "Gelände (rel.)",
                            "Traufe (rel.)",
                            "First (rel.)",
                        ],
                        &[2., 1., 1., 1., 1.],
                        stand
                            .into_values()
                            .map(|g| match g {
                                Some(g) => vec![
                                    g.bezeichnung,
                                    m(g.nullpunkt_m_nhn),
                                    m(g.gelaende_m),
                                    m(g.traufe_m),
                                    m(g.first_m),
                                ],
                                None => vec![
                                    "Gebäude nicht mehr vorhanden".into(),
                                    "–".into(),
                                    "–".into(),
                                    "–".into(),
                                    "–".into(),
                                ],
                            })
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
            // Einheitlicher Fachtitel; die freie Bezeichnung ergänzt ihn nur, wenn
            // sie zusätzliche Information trägt.
            let fach = fachtitel(&n.stand.eingabe);
            verzeichnisname = Some(benennung(fach, &n.stand.bezeichnung));
            fach.to_string()
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
                z.push(abschnitt(format!("Baugrundstück: {}", g.bezeichnung)));
                z.push(titel("Flurstücke und Flächenabzüge"));
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
            core.raumbuch_planquellen_pruefen(p.id, &r).await?;
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
            stand = format!("Raumbuch Revision {revision}");
            let mut plaene = Vec::new();
            for b in &r.planbindungen {
                if let Some(d) = aufmassdokument(core, p, b).await?
                    && let Some(a) = bezuege.dokument(&d)
                {
                    plaene.push(a);
                }
            }
            plaene.sort_unstable();
            plaene.dedup();
            verweise.extend(&plaene);
            z.push(abschnitt("Raumaufstellung"));
            z.push(text(format!(
                "Gebäude: {gebaeude} · Dokumentstand {revision}"
            )));
            if !plaene.is_empty() {
                z.push(text(format!(
                    "Flächen und Höhen per PDF-Aufmaß aus: {}",
                    plaene
                        .iter()
                        .map(|a| format!("Anlage {a}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
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
            z.push(abschnitt("Summen"));
            z.push(text(format!(
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
            if !aktuell {
                z.push(hinweis(
                    "VERALTET: Für den Plan liegt eine neuere Version vor. Umrisse neu zuordnen und prüfen.",
                ));
            }
            z.push(abschnitt("Lage im Baufenster (§ 23 BauNVO)"));
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
            z.push(abschnitt("Ergebnis"));
            z.push(if r.eingehalten {
                text("Baugrenze eingehalten")
            } else {
                hinweis("Baugrenze überschritten – fachlich prüfen")
            });
            "Baugrenzenprüfung".into()
        }
        Druckquelle::Dokument { .. } => {
            return Err(fehler("Dokument benötigt den Dokumentadapter"));
        }
    };
    // Jede Berechnung schließt mit einem Aufstellungsvermerk ab.
    z.push(abschnitt("Aufstellungsvermerk"));
    z.push(text("Aufgestellt auf Grundlage der genannten Unterlagen, Eingaben und Regelstände. Das Blatt gibt den gespeicherten Berechnungsstand wieder; offene Angaben, Abweichungen und Vorbehalte sind ausgewiesen. Die Ausgabe ersetzt keine fachliche Prüfung."));
    z.push(Zeile::Unterschrift(if bezuege.verfasser.is_empty() {
        vec![("Aufgestellt (Entwurfsverfasser/in)".into(), String::new())]
    } else {
        bezuege.verfasser.clone()
    }));
    verweise.sort_unstable();
    verweise.dedup();
    Ok(Fachbericht {
        verzeichnisname: verzeichnisname.unwrap_or_else(|| name.clone()),
        titel: name,
        zeilen: z,
        verweise,
        stand,
    })
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
        GarageAbstandsflaeche => "Grenzgarage / Garage in der Abstandsfläche",
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
        "baufensterflaeche" => "Baufensterfläche (Information)",
        "baufenster_lage" => "Lage im Baufenster (m² außerhalb)",
        _ => a,
    }
}
fn vergleichquelle(a: &str) -> &str {
    match a {
        "festsetzung" => "Planfestsetzung",
        "orientierungswert" => "Orientierungswert § 17 BauNVO",
        "umgebung" => "Nähere Umgebung (§ 34 BauGB)",
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gegenstand_nennt_gebaeudebezeichnung() {
        let id = uuid::Uuid::new_v4().to_string();
        let g: crate::plattform::objekte::Gebaeude = serde_json::from_value(serde_json::json!({
            "bezeichnung": "Garage", "grundstueck_ids": [], "geschosse": [], "quelle": null
        }))
        .unwrap();
        let m = std::collections::HashMap::from([(id.clone(), g)]);
        assert_eq!(gegenstandsname(Some(&id), &m), "Garage");
        assert_eq!(
            gegenstandsname(Some(&uuid::Uuid::new_v4().to_string()), &m),
            "Gebäude (nicht mehr erfasst)"
        );
        assert_eq!(gegenstandsname(Some("Vorhaben"), &m), "Vorhaben");
        assert_eq!(gegenstandsname(None, &m), "");
    }
    #[test]
    fn fachtitel_ersetzt_wiederholende_bezeichnungen() {
        let grz = "Nachweis der Grundflächenzahl (GRZ, § 19 BauNVO)";
        assert_eq!(benennung(grz, "Grz Berechnung"), grz);
        assert_eq!(benennung(grz, "GRZ"), grz);
        assert_eq!(benennung(grz, "  "), grz);
        assert_eq!(
            benennung(grz, "Variante B mit Tiefgarage"),
            format!("{grz} – Variante B mit Tiefgarage")
        );
        assert_eq!(
            benennung("Wohnflächenberechnung", "Wohnflächenberechnung"),
            "Wohnflächenberechnung"
        );
    }
}
