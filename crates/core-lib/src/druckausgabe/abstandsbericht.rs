//! Behördenansicht des unveränderten archivierten Abstandsflächenergebnisses.
use super::{
    berichte::{Zeile, angaben, ja, optional, tabelle, text, titel, zahl},
    *,
};
/// Eigener Präsentationsadapter statt technischer Debug-Ausgabe des Fachmoduls.
pub(super) fn abstand(
    z: &mut Vec<Zeile>,
    p: &crate::abstandsflaechen::AbstandsPlan,
    r: &crate::abstandsflaechen::AbstandsErgebnis,
    docs: &[crate::domain::DocumentVersion],
) -> CoreResult<()> {
    use crate::abstandsflaechen::*;
    let beleg = |b: Option<&crate::plattform::objekte::Dokumentbezug>| {
        b.map(|b| {
            docs.iter()
                .find(|d| d.document_id == b.dokument_id && d.version == b.version)
                .map(|d| format!("{} · Version {}", d.file_name, d.version))
                .unwrap_or_else(|| "Beleg nicht auflösbar – prüfen".into())
        })
        .unwrap_or_else(|| "Kein Beleg zugeordnet".into())
    };
    let grund = |id: Option<uuid::Uuid>| {
        p.grundstuecke
            .iter()
            .position(|g| Some(g.id) == id)
            .map(|i| format!("G{} · {}", i + 1, p.grundstuecke[i].bezeichnung))
            .unwrap_or_else(|| "Nicht zugeordnet".into())
    };
    let wand = |id: uuid::Uuid| {
        p.waende
            .iter()
            .position(|w| w.id == id)
            .map(|i| format!("W{} · {}", i + 1, p.waende[i].bezeichnung))
            .unwrap_or_else(|| "Wandzuordnung fehlt".into())
    };
    z.push(titel("Rechtliche und geometrische Grundlagen"));
    angaben(
        z,
        vec![
            ("Gemeinde", p.recht.gemeinde.clone()),
            ("Rechtsfassung", p.recht.fassung.clone()),
            ("Baugebiet", p.recht.gebiet.clone()),
            ("Anwendung / Übergangsrecht", p.recht.anwendung.clone()),
            ("Örtliche Satzung", p.recht.satzung.clone()),
            ("Planfestsetzungen", p.recht.planfestsetzung.clone()),
            (
                "Rechtsprüfung",
                format!("{} · {}", ja(p.recht.geprueft), p.recht.pruefvermerk),
            ),
            ("Regelstand", p.regeln.rechtsstand.clone()),
            ("Höhenbezug", p.hoehenbezug.clone()),
            ("Koordinatenbezug (m)", p.koordinatenbezug.clone()),
            (
                "Umfangsprüfung",
                format!("{} · {}", ja(p.umfang_geprueft), p.umfang_vermerk),
            ),
        ],
    );
    z.push(text(&p.regeln.hinweis));
    z.push(tabelle(
        &["Planbezug", "Grundstück", "Grundlage"],
        &[0.7, 2., 3.],
        p.grundstuecke
            .iter()
            .enumerate()
            .map(|(i, g)| {
                vec![
                    format!("G{}", i + 1),
                    g.bezeichnung.clone(),
                    g.grundlage.clone(),
                ]
            })
            .collect(),
    ));
    z.push(titel("Ergebnisübersicht der Wände"));
    let status = |s: &AbstandsStatus| match s {
        AbstandsStatus::Offen => "Offen",
        AbstandsStatus::Tiefenvorpruefung => "Nur Tiefenvorprüfung",
        AbstandsStatus::Eingehalten => "Eingehalten",
        AbstandsStatus::Konflikt => "Konflikt",
        AbstandsStatus::Grenzfall => "Grenzfall – prüfen",
    };
    z.push(tabelle(
        &[
            "Wand",
            "H m",
            "Faktor",
            "Tiefe m",
            "Fläche m²",
            "Außerhalb m²",
            "Ergebnis",
        ],
        &[1.8, 0.6, 0.6, 0.7, 0.8, 0.9, 1.3],
        p.waende
            .iter()
            .enumerate()
            .filter_map(|(i, w)| {
                r.waende.iter().find(|e| e.id == w.id).map(|e| {
                    vec![
                        format!("W{} · {} / {}", i + 1, w.gebaeude, w.bezeichnung),
                        optional(e.h_m),
                        optional(e.faktor),
                        optional(e.tiefe_m),
                        optional(e.flaeche_m2),
                        optional(e.ausserhalb_m2),
                        status(&e.status).into(),
                    ]
                })
            })
            .collect(),
    ));
    for (i, w) in p.waende.iter().enumerate() {
        let Some(e) = r.waende.iter().find(|e| e.id == w.id) else {
            return Err(fehler("Archiviertes Wandergebnis fehlt"));
        };
        z.push(titel(format!(
            "W{} · {} / {} – Herleitung",
            i + 1,
            w.gebaeude,
            w.bezeichnung
        )));
        angaben(
            z,
            vec![
                ("Grundstück", grund(w.grundstueck_id)),
                ("Wandlänge", format!("{} m", zahl(e.laenge_m))),
                (
                    "Wandkoordinaten A → B",
                    format!(
                        "({}, {}) → ({}, {}) m",
                        zahl(w.von.x),
                        zahl(w.von.y),
                        zahl(w.bis.x),
                        zahl(w.bis.y)
                    ),
                ),
                (
                    "Außenseite",
                    match w.aussen {
                        Aussenrichtung::Links => "Links",
                        Aussenrichtung::Rechts => "Rechts",
                    }
                    .into(),
                ),
                (
                    "Geländehöhen A / B",
                    format!(
                        "{} / {} m",
                        optional(w.gelaende_a_m),
                        optional(w.gelaende_b_m)
                    ),
                ),
                (
                    "Wandabschluss A / B",
                    format!(
                        "{} / {} m",
                        optional(w.abschluss_a_m),
                        optional(w.abschluss_b_m)
                    ),
                ),
                ("Höhenquelle", w.hoehenquelle.clone()),
                (
                    "Dachform",
                    match w.dachart {
                        Dachart::Flach => "Flachdach",
                        Dachart::Dachflaeche => "Dachfläche",
                        Dachart::Giebel => "Giebel",
                        Dachart::Manuell => "Manueller Ansatz",
                        Dachart::Ungeklaert => "Ungeklärt",
                    }
                    .into(),
                ),
                (
                    "Dachhöhe / Neigung",
                    if w.dachart == Dachart::Flach {
                        "Entfällt bei Flachdach".into()
                    } else {
                        format!(
                            "{} m / {}°",
                            optional(w.dachhoehe_m),
                            optional(w.dachneigung_grad)
                        )
                    },
                ),
                (
                    "Dachansatz",
                    if w.dachart == Dachart::Flach {
                        "Kein zusätzlicher Dachhöhenanteil".into()
                    } else {
                        format!(
                            "{} · manueller Faktor {}",
                            w.dachgrundlage,
                            optional(w.dachfaktor_manuell)
                        )
                    },
                ),
                (
                    "Wandhöhe / Dachanteil",
                    format!(
                        "{} m + {} m = H {} m",
                        optional(e.wandhoehe_m),
                        optional(e.dachanteil_m),
                        optional(e.h_m)
                    ),
                ),
                (
                    "Regeltiefe",
                    format!(
                        "max({} × {} m; {} m) · angesetzte Tiefe {} m",
                        optional(e.faktor),
                        optional(e.h_m),
                        optional(e.mindesttiefe_m),
                        optional(e.tiefe_m)
                    ),
                ),
                (
                    "Fall / Nutzung",
                    format!(
                        "{} · Nutzung geprüft: {} · Grenzabschnitt {}",
                        match w.fall {
                            Wandfall::Regel => "Regelfall",
                            Wandfall::Garage => "Garage",
                            Wandfall::Carport => "Carport",
                            Wandfall::Vorbau => "Vorbau",
                            Wandfall::Grenzbebauung => "Grenzbebauung",
                            Wandfall::Bestand => "Bestand",
                            Wandfall::Sonderbau => "Sonderbau",
                            Wandfall::Sonstiges => "Sonstiges",
                        },
                        if w.fall == Wandfall::Regel {
                            "Regelabstand ohne Nutzungsprivileg"
                        } else {
                            ja(w.nutzung_geprueft)
                        },
                        w.grenzkennung
                    ),
                ),
            ],
        );
        for h in &e.herleitung {
            z.push(text(h));
        }
        if let Some(s) = &w.sonderansatz {
            angaben(
                z,
                vec![
                    (
                        "Besonderer Tiefenansatz",
                        format!("{} m · wirksam: {}", zahl(s.tiefe_m), ja(s.wirksam)),
                    ),
                    ("Grundlage", s.grundlage.clone()),
                    ("Beleg", beleg(s.beleg.as_ref())),
                ],
            );
        }
        for o in &e.offen {
            z.push(text(format!("Offener Prüfpunkt W{}: {o}", i + 1)));
        }
        if !e.polygon.is_empty() {
            z.push(tabelle(
                &["Eckpunkt Abstandsfläche", "x (m)", "y (m)"],
                &[2., 1., 1.],
                e.polygon
                    .iter()
                    .enumerate()
                    .map(|(i, p)| vec![(i + 1).to_string(), zahl(p.x), zahl(p.y)])
                    .collect(),
            ));
        }
        for (j, k) in e.konflikte.iter().enumerate() {
            z.push(text(format!(
                "Konfliktfläche W{} / {} (rot in der Planskizze)",
                i + 1,
                j + 1
            )));
            for (ring, pts) in std::iter::once(("Außenring".to_owned(), &k.aussen)).chain(
                k.innen
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (format!("Innenring {}", i + 1), p)),
            ) {
                z.push(tabelle(
                    &["Ring / Punkt", "x (m)", "y (m)"],
                    &[2., 1., 1.],
                    pts.iter()
                        .enumerate()
                        .map(|(i, p)| vec![format!("{ring} / {}", i + 1), zahl(p.x), zahl(p.y)])
                        .collect(),
                ));
            }
        }
    }
    if !p.flaechen.is_empty() {
        z.push(titel("Sicherungen, öffentliche Flächen und Baukörper"));
        z.push(tabelle(
            &[
                "Planbezug",
                "Bezeichnung / Art",
                "Grundstück",
                "Wirksam",
                "Rechtsgrundlage",
            ],
            &[0.65, 1.8, 1.5, 0.8, 2.],
            p.flaechen
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    vec![
                        format!("F{}", i + 1),
                        format!(
                            "{} / {}",
                            f.bezeichnung,
                            match f.art {
                                Flaechenart::Sicherung => "Sicherungsfläche",
                                Flaechenart::OeffentlicheFlaeche => "Öffentliche Fläche",
                                Flaechenart::Baukoerper => "Baukörper",
                            }
                        ),
                        grund(Some(f.grundstueck_id)),
                        ja(f.wirksam).into(),
                        format!("{}\nBeleg: {}", f.grundlage, beleg(f.beleg.as_ref())),
                    ]
                })
                .collect(),
        ));
    }
    if !r.ueberdeckungen.is_empty() {
        z.push(titel(
            "Überdeckungen – gesonderte Zulässigkeitsprüfung erforderlich",
        ));
        z.push(tabelle(
            &["Wand A", "Wand B", "Überdeckung m²"],
            &[2., 2., 1.],
            r.ueberdeckungen
                .iter()
                .map(|u| vec![wand(u.wand_a), wand(u.wand_b), zahl(u.flaeche_m2)])
                .collect(),
        ));
    }
    for o in &r.offen {
        z.push(text(format!("Offener Prüfpunkt: {o}")));
    }
    z.push(text("Planskizze: G = Grundstück, W = Wand / Abstandsfläche, F = zusätzliche Fläche. Konflikte rot, Überdeckungen violett. Maßstab nur bei Ausgabe mit 100 %. Die Planskizze ersetzt keinen amtlichen Lageplan."));
    Ok(())
}
