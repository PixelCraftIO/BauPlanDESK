//! Behördenansicht des unveränderten archivierten Abstandsflächenergebnisses.
use super::{
    berichte::{Zeile, abschnitt, angaben, hinweis, ja, optional, tabelle, text, titel, zahl},
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
    z.push(abschnitt("Rechtliche und geometrische Grundlagen"));
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
            (
                "Wandhöhenansatz",
                match p.regeln.wandhoehe {
                    Some(a) => a.bezeichnung().into(),
                    None => format!(
                        "{} (Regelpaket ohne Angabe)",
                        Wandhoehenansatz::Maximal.bezeichnung()
                    ),
                },
            ),
            ("Höhenbezug", p.hoehenbezug.clone()),
            ("Koordinatenbezug (m)", p.koordinatenbezug.clone()),
            (
                "Umfangsprüfung",
                format!("{} · {}", ja(p.umfang_geprueft), p.umfang_vermerk),
            ),
        ],
    );
    z.push(text(&p.regeln.hinweis));
    regelwerk(z, p);
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
    if !p.umrisse.is_empty() {
        z.push(titel("Gebäudeumrisse"));
        z.push(text(
            "Die Wände der Umrisse sind Seiten in Umlaufrichtung; Lage, Außenseite und Geländehöhen an den Ecken stammen aus dem Umriss.",
        ));
        for (i, u) in p.umrisse.iter().enumerate() {
            z.push(tabelle(
                &["Umriss / Ecke", "x (m)", "y (m)", "Gelände (m)"],
                &[2., 1., 1., 1.],
                u.ecken
                    .iter()
                    .enumerate()
                    .map(|(k, e)| {
                        vec![
                            format!("U{} {} / Ecke {}", i + 1, u.bezeichnung, k + 1),
                            zahl(e.x),
                            zahl(e.y),
                            optional(e.gelaende_m),
                        ]
                    })
                    .collect(),
            ));
            z.push(text(format!(
                "U{}: Grundstück {} · Vorgabe Wandoberkante {} m · Quelle: {}",
                i + 1,
                grund(u.grundstueck_id),
                optional(u.oberkante_m),
                u.quelle
            )));
        }
    }
    z.push(abschnitt("Ergebnisübersicht der Wände"));
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
    z.push(abschnitt("Herleitung je Wand"));
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
                    "Höhenangaben",
                    if e.profil.is_empty() {
                        format!(
                            "Gelände A / B {} / {} m · Wandabschluss A / B {} / {} m",
                            optional(w.gelaende_a_m),
                            optional(w.gelaende_b_m),
                            optional(w.abschluss_a_m),
                            optional(w.abschluss_b_m)
                        )
                    } else {
                        format!(
                            "{} Geländepunkte, {} Oberkantenpunkte, {} Wandteil(e) – siehe Höhenprofil",
                            w.gelaende.len().max(2),
                            w.oberkante.len().max(2),
                            e.wandteile.len()
                        )
                    },
                ),
                ("Höhenquelle", w.hoehenquelle.clone()),
                (
                    "Tiefenfall",
                    match w.tiefenfall.unwrap_or_default() {
                        Tiefenfall::Regel => Tiefenfall::Regel.bezeichnung().into(),
                        f => format!("{} · {}", f.bezeichnung(), w.tiefenfall_grundlage),
                    },
                ),
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
                            "{} m / {}°{}{}",
                            optional(w.dachhoehe_m),
                            optional(w.dachneigung_grad),
                            w.dachneigung_b_grad
                                .map(|n| format!(" und {}° (zweite Dachseite)", zahl(n)))
                                .unwrap_or_default(),
                            w.gauben_laenge_m
                                .map(|l| format!(" · Gauben/Dachaufbauten {} m", zahl(l)))
                                .unwrap_or_default()
                        )
                    },
                ),
                ("Dachansatz", {
                    let grundlage = if w.dachgrundlage.trim().is_empty() {
                        String::new()
                    } else {
                        format!("{} · ", w.dachgrundlage)
                    };
                    match w.dachart {
                        Dachart::Flach => "Kein zusätzlicher Dachhöhenanteil".into(),
                        // Faktor aus dem Regelpaket; Herleitung nennt Faktor und Quelle.
                        Dachart::Dachflaeche => format!(
                            "{grundlage}Dachflächenregel des Regelpakets · Dachanteil {} m",
                            optional(e.dachanteil_m)
                        ),
                        Dachart::Giebel if p.regeln.giebel.is_some() => format!(
                            "{grundlage}Giebelregel des Regelpakets · Dachanteil {} m",
                            optional(e.dachanteil_m)
                        ),
                        _ => format!(
                            "{grundlage}manueller Faktor {}",
                            optional(w.dachfaktor_manuell)
                        ),
                    }
                }),
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
                            Wandfall::Nebengebaeude => "Nebengebäude",
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
            z.push(if h.starts_with("Hinweis") {
                hinweis(h)
            } else {
                text(h)
            });
        }
        if !e.wandteile.is_empty() {
            z.push(tabelle(
                &[
                    "Wandteil",
                    "Station m",
                    "H Kanten m",
                    "Wandhöhe m",
                    "H m",
                    "Tiefe m",
                ],
                &[0.8, 1.1, 1.2, 1., 0.8, 0.8],
                e.wandteile
                    .iter()
                    .enumerate()
                    .map(|(k, t)| {
                        vec![
                            (k + 1).to_string(),
                            format!("{} – {}", zahl(t.von_m), zahl(t.bis_m)),
                            format!("{} / {}", zahl(t.h_anfang_m), zahl(t.h_ende_m)),
                            zahl(t.wandhoehe_m),
                            optional(t.h_m),
                            optional(t.tiefe_m),
                        ]
                    })
                    .collect(),
            ));
            z.push(tabelle(
                &[
                    "Station m",
                    "Gelände m",
                    "Oberkante m",
                    "Wandhöhe m",
                    "Tiefe m",
                    "Teil",
                    "Quelle",
                ],
                &[0.8, 0.9, 0.9, 0.9, 0.7, 0.5, 2.],
                e.profil
                    .iter()
                    .map(|q| {
                        let quelle = w
                            .gelaende
                            .iter()
                            .chain(&w.oberkante)
                            .filter(|h| {
                                (h.station_m - q.station_m).abs() < 0.001 && !h.quelle.is_empty()
                            })
                            .map(|h| h.quelle.as_str())
                            .fold(Vec::new(), |mut v, q| {
                                if !v.contains(&q) {
                                    v.push(q);
                                }
                                v
                            });
                        vec![
                            zahl(q.station_m),
                            zahl(q.gelaende_m),
                            zahl(q.oberkante_m),
                            zahl(q.wandhoehe_m),
                            optional(q.tiefe_m),
                            q.wandteil.to_string(),
                            if quelle.is_empty() {
                                "Interpoliert bzw. Wandangabe".into()
                            } else {
                                quelle.join("; ")
                            },
                        ]
                    })
                    .collect(),
            ));
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
            z.push(hinweis(format!("Offener Prüfpunkt W{}: {o}", i + 1)));
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
        z.push(abschnitt("Sicherungen, öffentliche Flächen und Baukörper"));
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
        z.push(abschnitt("Überdeckungen"));
        if r.ueberdeckungen.iter().any(|u| u.zulaessig.is_none()) {
            z.push(hinweis(
                "Überdeckungen ohne Regelausnahme erfordern eine gesonderte Zulässigkeitsprüfung.",
            ));
        }
        z.push(tabelle(
            &[
                "Wand A",
                "Wand B",
                "Überdeckung m²",
                "Winkel",
                "Beurteilung",
            ],
            &[1.5, 1.5, 0.9, 0.7, 2.4],
            r.ueberdeckungen
                .iter()
                .map(|u| {
                    vec![
                        wand(u.wand_a),
                        wand(u.wand_b),
                        zahl(u.flaeche_m2),
                        u.winkel_grad
                            .map(|w| format!("{}°", zahl(w)))
                            .unwrap_or_else(|| "–".into()),
                        u.zulaessig
                            .clone()
                            .unwrap_or_else(|| "Gesondert prüfen".into()),
                    ]
                })
                .collect(),
        ));
    }
    if !r.offen.is_empty() {
        z.push(abschnitt("Offene Prüfpunkte"));
        for o in &r.offen {
            z.push(hinweis(format!("Offener Prüfpunkt: {o}")));
        }
    }
    z.push(abschnitt("Wandabwicklungen und Planskizze"));
    if r.waende.iter().any(|w| w.profil.len() >= 2) {
        z.push(text("Die Wandabwicklungen folgen nach diesem Bericht: je Wand Geländelinie, Wandoberkante, Wandteilgrenzen und Wandhöhen an den Profilstellen, maßstäblich in Wandrichtung."));
    }
    z.push(text("Die Vektorplanskizze folgt als letztes Blatt dieser Anlage. Planskizze: G = Grundstück, W = Wand / Abstandsfläche, F = zusätzliche Fläche. Konflikte rot, Überdeckungen violett. Maßstab nur bei Ausgabe mit 100 %. Die Planskizze ersetzt keinen amtlichen Lageplan."));
    Ok(())
}

/// Rechtsgrundlage aus der Regeldatei: Fassung, Fundstelle, Prüfstatus,
/// Wortlaut der angewandten Absätze und Prüfpunkte.
fn regelwerk(z: &mut Vec<Zeile>, p: &crate::abstandsflaechen::AbstandsPlan) {
    let (Some(b), Some(pr)) = (&p.regelwerk, crate::abstandsflaechen::regelwerk_pruefen(p)) else {
        return;
    };
    z.push(titel("Rechtsgrundlage (Regeldatei)"));
    let Some(r) = &pr.regelwerk else {
        z.push(hinweis(format!(
            "Regeldatei {} ist in dieser Programmversion nicht enthalten.",
            b.datei
        )));
        return;
    };
    let d = &r.inhalt;
    let tag = |t: Option<chrono::NaiveDate>| {
        t.map(|t| t.format("%d.%m.%Y").to_string())
            .unwrap_or_else(|| "offen".into())
    };
    let gueltig = match d.fassung.gueltig_ab {
        Some(_) => format!(
            "{} bis {}",
            tag(d.fassung.gueltig_ab),
            match d.fassung.gueltig_bis {
                Some(_) => tag(d.fassung.gueltig_bis),
                None => "auf Weiteres".into(),
            }
        ),
        None => "Pilotansatz ohne belegte Fassung".into(),
    };
    angaben(
        z,
        vec![
            (
                "Gesetz",
                format!("{}, {}", d.fassung.gesetz, d.fassung.paragraf),
            ),
            ("Fassung gültig", gueltig),
            ("Fundstelle", d.fassung.fundstelle.clone()),
            (
                "Stichtag",
                format!("{} ({})", b.stichtag.format("%d.%m.%Y"), b.stichtag_art),
            ),
            (
                "Fassungswahl",
                if pr.geltend.as_deref() == Some(b.datei.as_str()) {
                    "Am Stichtag geltende Fassung".into()
                } else {
                    format!("Abweichend vom Stichtag: {}", b.abweichung)
                },
            ),
            ("Übergangsrecht", d.fassung.uebergang.clone()),
            (
                "Regeldatei",
                format!("{} · SHA-256 {}", r.datei, b.pruefsumme),
            ),
            (
                "Quellen",
                d.quellen
                    .iter()
                    .map(|q| format!("{} (abgerufen {})", q.titel, tag(q.abgerufen)))
                    .collect::<Vec<_>>()
                    .join("; "),
            ),
            (
                "Fachliche Freigabe",
                if d.pruefung.geprueft {
                    format!(
                        "Ja · {} · {}",
                        d.pruefung.geprueft_von,
                        tag(d.pruefung.geprueft_am)
                    )
                } else {
                    "Nein".into()
                },
            ),
        ],
    );
    if !d.pruefung.geprueft {
        z.push(hinweis(
            "Regelwerk fachlich nicht freigegeben: Die Regeldatei ist aus dem Gesetzestext übertragen, aber vom Büro noch nicht abgenommen.",
        ));
    }
    if pr.datei_geaendert {
        z.push(hinweis(if pr.regeln_aktuell {
            "Die Regeldatei wurde seit der Übernahme geändert (Angaben oder Wortlaut); die Rechenregeln sind unverändert."
        } else {
            "Die Regeldatei wurde seit der Übernahme geändert; gerechnet wurde mit der im Nachweis gespeicherten Regelkopie."
        }));
    }
    let wortlaut: Vec<_> = d
        .wortlaut
        .iter()
        .filter(|a| pr.angewandte_absaetze.contains(&a.absatz))
        .collect();
    if !wortlaut.is_empty() {
        z.push(titel(format!(
            "Wortlaut der angewandten Absätze ({})",
            d.fassung.paragraf
        )));
        for a in wortlaut {
            z.push(text(format!("({}) {}", a.absatz, a.text)));
        }
    }
    if !d.pruefpunkte.is_empty() {
        z.push(titel("Nicht automatisch geprüfte Tatbestände"));
        z.push(tabelle(
            &["Abs.", "Prüffrage", "Erledigt", "Vermerk"],
            &[0.5, 3.2, 0.8, 2.],
            d.pruefpunkte
                .iter()
                .map(|q| {
                    let s = b.pruefpunkte.iter().find(|s| s.absatz == q.absatz);
                    vec![
                        q.absatz.to_string(),
                        q.frage.clone(),
                        ja(s.is_some_and(|s| s.erledigt)).into(),
                        s.map(|s| s.vermerk.clone()).unwrap_or_default(),
                    ]
                })
                .collect(),
        ));
    }
}
