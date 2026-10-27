//! Aufmaßexport liest einen gespeicherten Stand über den Fachobjektvertrag.
//! Darstellung und PDF-Parser bleiben im Ausgabe-/Formularadapter.
use super::*;
use crate::{
    CoreServices,
    domain::ProjectId,
    druckausgabe::plananlage::{Plananlage, Plankontur},
    plattform::ObjektArt,
};
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Planexportformat {
    Pdf,
    Csv,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Planexportwunsch {
    pub aufmass_id: Uuid,
    #[ts(type = "number")]
    pub revision: i64,
    pub format: Planexportformat,
    #[serde(default)]
    pub nur_sichtbare_ebenen: bool,
}
fn ringe(g: &Plangeometrie) -> (Vec<Vec<Planpunkt>>, bool) {
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
            let r = abstand(mitte, rand);
            (
                vec![
                    (0..256)
                        .map(|i| {
                            let a = i as f64 * std::f64::consts::TAU / 256.;
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
                vec![[p[0] - 3., p[1]], [p[0] + 3., p[1]]],
                vec![[p[0], p[1] - 3.], [p[0], p[1] + 3.]],
            ],
            false,
        ),
    }
}
fn csvtext(s: &str) -> String {
    let sicher = if s.trim_start().starts_with(['=', '+', '-', '@']) {
        format!("'{s}")
    } else {
        s.into()
    };
    format!("\"{}\"", sicher.replace('"', "\"\""))
}
fn zahl(n: Option<f64>) -> String {
    n.map(|n| format!("{n:.6}").replace('.', ","))
        .unwrap_or_default()
}
fn lesbare_zahl(n: f64) -> String {
    format!("{n:.3}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}
/// Ausgabemaßstab eigener Zeichnungen: 1 : 100, also Seitenpunkte je Meter.
const ZEICHNUNG_MASSSTAB: f64 = 100.;
fn zeichnung_punkte_pro_meter() -> f64 {
    72. / 0.0254 / ZEICHNUNG_MASSSTAB
}
/// Leeres PDF-Blatt (Seitenpunkte) für die Ausgabe einer eigenen Zeichnung.
fn leeres_blatt(breite: f64, hoehe: f64) -> CoreResult<Vec<u8>> {
    use lopdf::{Dictionary, Document, Object, Stream, dictionary};
    let mut d = Document::with_version("1.5");
    let pages_id = d.new_object_id();
    let content = d.add_object(Stream::new(Dictionary::new(), Vec::new()));
    let page = d.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), Object::Real(breite as f32), Object::Real(hoehe as f32)],
        "Contents" => content,
        "Resources" => Dictionary::new(),
    });
    d.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page.into()],
            "Count" => 1,
        }),
    );
    let catalog = d.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    d.trailer.set("Root", catalog);
    let mut out = Vec::new();
    d.save_to(&mut out).map_err(|e| fehler(e.to_string()))?;
    Ok(out)
}
/// Zeichenelemente (Wand, Maßkette, Text) als Konturen der Plananlage.
fn elementkonturen(p: &Planaufmass, skalierung: f64) -> Vec<Plankontur> {
    let s = |q: &Planpunkt| [q[0] * skalierung, q[1] * skalierung];
    p.elemente
        .iter()
        .filter(|e| p.ebenen.iter().any(|l| l.id == e.ebene_id && l.sichtbar))
        .map(|e| match &e.form {
            Zeichenform::Wand { von, bis, dicke_m } => {
                let l = abstand(von, bis).max(1e-9);
                let n = [
                    -(bis[1] - von[1]) / l * dicke_m / 2.,
                    (bis[0] - von[0]) / l * dicke_m / 2.,
                ];
                Plankontur {
                    ringe: vec![vec![
                        s(&[von[0] + n[0], von[1] + n[1]]),
                        s(&[bis[0] + n[0], bis[1] + n[1]]),
                        s(&[bis[0] - n[0], bis[1] - n[1]]),
                        s(&[von[0] - n[0], von[1] - n[1]]),
                    ]],
                    geschlossen: true,
                    beschriftung: format!("Wand {} m", lesbare_zahl(*dicke_m)),
                }
            }
            Zeichenform::Masskette {
                von,
                bis,
                abstand: a,
            } => {
                let l = abstand(von, bis).max(1e-9);
                let n = [-(bis[1] - von[1]) / l * a, (bis[0] - von[0]) / l * a];
                Plankontur {
                    ringe: vec![vec![
                        s(&[von[0] + n[0], von[1] + n[1]]),
                        s(&[bis[0] + n[0], bis[1] + n[1]]),
                    ]],
                    geschlossen: false,
                    beschriftung: format!("{} m", lesbare_zahl(l)),
                }
            }
            Zeichenform::Text { punkt, inhalt, .. } => Plankontur {
                ringe: vec![vec![s(punkt), s(punkt)]],
                geschlossen: false,
                beschriftung: inhalt.clone(),
            },
        })
        .collect()
}
fn kalibrierung(k: Option<&Plankalibrierung>) -> String {
    match k {
        None => "nicht festgelegt".into(),
        Some(Plankalibrierung::Massstab { nenner }) => format!("1 : {}", lesbare_zahl(*nenner)),
        Some(Plankalibrierung::Strecke { meter, .. }) => {
            format!("kalibriert über {} m Referenzstrecke", lesbare_zahl(*meter))
        }
        Some(Plankalibrierung::Einheit { einheit }) => {
            format!("Zeichnungseinheit {}", einheit.titel())
        }
    }
}
impl CoreServices {
    pub async fn planaufmass_export(
        &self,
        pid: ProjectId,
        w: &Planexportwunsch,
    ) -> CoreResult<Vec<u8>> {
        let f = self
            .fachobjekt_lesen(pid, ObjektArt::Planaufmass, w.aufmass_id)
            .await?
            .ok_or_else(|| fehler("Aufmaß nicht vorhanden"))?;
        if f.revision != w.revision {
            return Err(CoreError::VersionConflict {
                expected: w.revision,
                actual: f.revision,
            });
        }
        if f.schema_version != 1 {
            return Err(fehler("Aufmaß-Datenversion nicht unterstützt"));
        }
        let p: Planaufmass = serde_json::from_value(f.daten).map_err(|e| fehler(e.to_string()))?;
        let werte = self.planaufmass_pruefen(pid, &p).await?;
        // Eigene Zeichnung: leeres Blatt im Ausgabemaßstab, Meter → Seitenpunkte.
        let skalierung = if p.ist_zeichnung() {
            zeichnung_punkte_pro_meter()
        } else {
            1.
        };
        let (d, pdf) = match &p.dokument {
            Some(dokument) => {
                let (d, pdf) = self
                    .load_version(dokument.dokument_id, dokument.version)
                    .await?;
                (Some(d), pdf.to_vec())
            }
            None => {
                let (mut bx, mut by) = (10f64, 10f64);
                for q in p
                    .markierungen
                    .iter()
                    .flat_map(|m| m.geometrie.punkte())
                    .chain(p.elemente.iter().flat_map(|e| e.form.punkte()))
                {
                    bx = bx.max(q[0] + 2.);
                    by = by.max(q[1] + 2.);
                }
                (None, leeres_blatt(bx * skalierung, by * skalierung)?)
            }
        };
        let ms: Vec<_> = p
            .markierungen
            .iter()
            .filter(|m| {
                !w.nur_sichtbare_ebenen || p.ebenen.iter().any(|e| e.id == m.ebene_id && e.sichtbar)
            })
            .collect();
        let bedeutung = |m: &Planmarkierung| {
            serde_json::to_value(m.bedeutung)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default()
        };
        let quelle = match &d {
            Some(d) => format!(
                "{} · V{} · Seite {} · SHA-256 {}",
                d.file_name,
                d.version,
                p.seite,
                d.sha256.as_str()
            ),
            None => format!(
                "Eigene Zeichnung in Metern · Ausgabe 1 : {}; Hintergründe werden nicht ausgegeben",
                lesbare_zahl(ZEICHNUNG_MASSSTAB)
            ),
        };
        match w.format {
            Planexportformat::Csv => {
                let mut out = String::from(
                    "\u{feff}Aufmass;Aufmass-ID;Revision;Unterlage;Dokument-ID;Dokumentversion;Seite;Markierungs-ID;Bezeichnung;Bedeutung;Ebene;Baugrundstueck-ID;Gebaeude-ID;Geschoss-ID;Flaeche_m2;Laenge_m;Horizontale_m;Hoehe_m;Neigung_grad;Offene_Angaben\r\n",
                );
                for m in ms {
                    let v = werte
                        .iter()
                        .find(|v| v.id == m.id)
                        .ok_or_else(|| fehler("Messwert fehlt"))?;
                    let texte = vec![
                        p.bezeichnung.clone(),
                        w.aufmass_id.to_string(),
                        w.revision.to_string(),
                        d.as_ref()
                            .map(|d| d.file_name.clone())
                            .unwrap_or_else(|| "Eigene Zeichnung".into()),
                        d.as_ref()
                            .map(|d| d.document_id.0.to_string())
                            .unwrap_or_default(),
                        d.as_ref()
                            .map(|d| d.version.to_string())
                            .unwrap_or_default(),
                        p.seite.to_string(),
                        m.id.to_string(),
                        m.bezeichnung.clone(),
                        bedeutung(m),
                        p.ebenen
                            .iter()
                            .find(|e| e.id == m.ebene_id)
                            .map(|e| e.bezeichnung.clone())
                            .unwrap_or_default(),
                        m.grundstueck_id
                            .map(|id| id.to_string())
                            .unwrap_or_default(),
                        m.gebaeude_id.map(|id| id.to_string()).unwrap_or_default(),
                        m.geschoss_id.map(|id| id.to_string()).unwrap_or_default(),
                    ];
                    let mut row: Vec<_> = texte.iter().map(|s| csvtext(s)).collect();
                    row.extend(
                        [
                            v.flaeche_m2,
                            v.laenge_m,
                            v.horizontale_m,
                            v.hoehe_m,
                            v.neigung_grad,
                        ]
                        .map(zahl),
                    );
                    row.push(csvtext(&v.offen.join(" · ")));
                    out.push_str(&row.join(";"));
                    out.push_str("\r\n");
                }
                Ok(out.into_bytes())
            }
            Planexportformat::Pdf => {
                let mut konturen = Vec::new();
                let mut legende = Vec::new();
                for (i, m) in ms.iter().enumerate() {
                    let v = werte
                        .iter()
                        .find(|v| v.id == m.id)
                        .ok_or_else(|| fehler("Messwert fehlt"))?;
                    let (mut ringe, geschlossen) = ringe(&m.geometrie);
                    if skalierung != 1. {
                        for r in &mut ringe {
                            for q in r.iter_mut() {
                                *q = [q[0] * skalierung, q[1] * skalierung];
                            }
                        }
                    }
                    konturen.push(Plankontur {
                        ringe,
                        geschlossen,
                        beschriftung: format!("{} · {}", i + 1, m.bezeichnung),
                    });
                    legende.push(vec![
                        format!("{} · {}", i + 1, m.bezeichnung),
                        format!(
                            "{} · {}",
                            m.bedeutung.titel(),
                            p.ebenen
                                .iter()
                                .find(|e| e.id == m.ebene_id)
                                .map(|e| e.bezeichnung.as_str())
                                .unwrap_or("")
                        ),
                        {
                            let mut teile = Vec::new();
                            for (name, wert, unit) in [
                                ("Fläche", v.flaeche_m2, "m²"),
                                (
                                    if v.flaeche_m2.is_some() {
                                        "Umfang"
                                    } else {
                                        "Länge"
                                    },
                                    v.laenge_m,
                                    "m",
                                ),
                                ("Höhe", v.hoehe_m, "m"),
                                ("Horizontale", v.horizontale_m, "m"),
                                ("Neigung", v.neigung_grad, "°"),
                            ] {
                                if let Some(wert) = wert {
                                    teile.push(format!("{name} {} {unit}", lesbare_zahl(wert)))
                                }
                            }
                            if teile.is_empty() {
                                teile.push("Keine kalibrierten Messwerte".into())
                            }
                            if !v.offen.is_empty() {
                                teile.push(format!("OFFEN: {}", v.offen.join("; ")))
                            }
                            teile.join("\n")
                        },
                    ]);
                }
                konturen.extend(elementkonturen(&p, skalierung));
                let mut herkunft=vec![quelle,format!("Aufmaß {} · Revision {}",w.aufmass_id,w.revision),format!("Ausgabe: {}",chrono::Utc::now().format("%d.%m.%Y %H:%M UTC")),"Planseite maßstäblich; Messwerte sind Rohmaße. Die fachliche Anrechnung erfolgt in der jeweiligen Berechnung.".into(),format!("Blattmaßstab: {}",kalibrierung(p.kalibrierung_wirksam().as_ref()))];
                for a in &p.ausschnitte {
                    herkunft.push(format!(
                        "Ausschnitt {}: {}; Höhenbezug {}",
                        a.bezeichnung,
                        a.kalibrierung
                            .as_ref()
                            .map(|k| kalibrierung(Some(k)))
                            .unwrap_or_else(|| "Blattmaßstab".into()),
                        a.bezugslinie
                            .as_ref()
                            .map(|b| format!("{} {}", lesbare_zahl(b.hoehe_m), b.system))
                            .unwrap_or_else(|| "nicht festgelegt".into())
                    ));
                }
                for (name, k) in std::iter::once(("Blatt", p.kalibrierung.as_ref())).chain(
                    p.ausschnitte
                        .iter()
                        .map(|a| (a.bezeichnung.as_str(), a.kalibrierung.as_ref())),
                ) {
                    if let Some(Plankalibrierung::Strecke { von, bis, meter }) = k {
                        konturen.push(Plankontur {
                            ringe: vec![vec![*von, *bis]],
                            geschlossen: false,
                            beschriftung: format!("Referenz {name}: {} m", lesbare_zahl(*meter)),
                        });
                    }
                }
                crate::druckausgabe::plananlage::erstellen(
                    &pdf,
                    &Plananlage {
                        seite: p.seite,
                        titel: format!("Aufmaßblatt · {}", p.bezeichnung),
                        herkunft,
                        legende,
                        konturen,
                    },
                )
            }
        }
    }
}
