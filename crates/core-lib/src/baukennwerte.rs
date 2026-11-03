//! Baukennwerte je Projekt: Rohbauwert und Gebührenschätzung sowie die Angaben
//! zur Baustatistik.
//!
//! Vertrag:
//! - Die Projektangaben stehen in einem Datensatz [`ObjektArt::Baukennwerte`]
//!   (Objektkennung = Projekt-ID). Fehlt er, gelten die Vorgaben.
//! - Brutto-Rauminhalt, Nutzungsfläche, Wohnfläche, Wohnungen und
//!   Vollgeschosse kommen aus den vorhandenen Berechnungen (DIN 277,
//!   Wohnflächen, Vollgeschosse). Eine Eingabe im Datensatz überschreibt den
//!   Wert sichtbar („Eingabe“ statt Berechnungsname als Quelle).
//! - Rohbauwerte und Gebührensatz stammen aus `workflow-regeln/gebuehren.json`
//!   (`geprueft: false`). Das Ergebnis ist eine Schätzung und nennt das.
//! - Der Baustatistikbogen ist eine Arbeitshilfe zum Übertrag in den amtlichen
//!   Erhebungsbogen nach dem Hochbaustatistikgesetz und ersetzt ihn nicht.
use crate::{
    CoreError, CoreResult, CoreServices,
    benutzer::Benutzer,
    berechnungen::{BerechnungsArt, FachPlan},
    domain::{Bundesland, ProjectId},
    druckausgabe::brief::{Brief, Briefblock},
    plattform::ObjektArt,
    zugriff::ProjektRecht,
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Regelquelle {
    pub titel: String,
    pub url: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Gebaeudeartwert {
    pub id: String,
    pub bezeichnung: String,
    pub euro_je_m3: Option<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Gebuehrensatz {
    pub promille: f64,
    pub mindestgebuehr_euro: f64,
    pub rechtsgrundlage: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Landesgebuehren {
    pub land: String,
    pub titel: String,
    pub geprueft: bool,
    pub stand: String,
    pub gebuehr: Gebuehrensatz,
    pub gebaeudearten: Vec<Gebaeudeartwert>,
    pub quellen: Vec<Regelquelle>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Gebuehrenregeln {
    pub id: String,
    pub version: String,
    pub hinweis: String,
    pub laender: Vec<Landesgebuehren>,
}

/// Mitgeliefertes Regelpaket.
pub fn regeln() -> &'static Gebuehrenregeln {
    static R: OnceLock<Gebuehrenregeln> = OnceLock::new();
    R.get_or_init(|| {
        serde_json::from_str(include_str!("../../../workflow-regeln/gebuehren.json"))
            .expect("workflow-regeln/gebuehren.json ist gültig")
    })
}
fn landeskuerzel(b: Bundesland) -> String {
    serde_json::to_value(b)
        .ok()
        .and_then(|v| v.as_str().map(str::to_uppercase))
        .unwrap_or_default()
}
pub fn fuer_land(b: Bundesland) -> Option<&'static Landesgebuehren> {
    let k = landeskuerzel(b);
    regeln().laender.iter().find(|l| l.land == k)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Bautaetigkeit {
    Neubau,
    /// Baumaßnahme an einem bestehenden Gebäude (Umbau, Erweiterung).
    Bestand,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Bauherrenart {
    PrivaterHaushalt,
    Unternehmen,
    OeffentlicheHand,
    OrganisationOhneErwerbszweck,
}
impl Bauherrenart {
    pub fn bezeichnung(self) -> &'static str {
        match self {
            Self::PrivaterHaushalt => "Private Haushalte",
            Self::Unternehmen => "Unternehmen",
            Self::OeffentlicheHand => "Öffentlicher Bauherr",
            Self::OrganisationOhneErwerbszweck => "Organisation ohne Erwerbszweck",
        }
    }
}

/// Projektangaben zu Kosten und Statistik; leere Felder werden abgeleitet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Baukennwerte {
    /// Kennung einer Gebäudeart aus dem Regelpaket.
    pub gebaeudeart: String,
    /// Brutto-Rauminhalt; leer = aus der DIN-277-Berechnung.
    #[serde(default)]
    pub bri_m3: Option<f64>,
    /// Rohbauwert je m³; leer = Wert des Regelpakets.
    #[serde(default)]
    pub euro_je_m3: Option<f64>,
    /// Gebührensatz in Promille des Rohbauwerts; leer = Regelpaket.
    #[serde(default)]
    pub promille: Option<f64>,
    pub bautaetigkeit: Bautaetigkeit,
    /// Leer = aus der Kundenkategorie des ersten Bauherrn.
    #[serde(default)]
    pub bauherrenart: Option<Bauherrenart>,
    /// Leer = Wohneinheiten der Wohnflächenberechnung.
    #[serde(default)]
    #[ts(type = "number | null")]
    pub wohnungen: Option<u32>,
    #[serde(default)]
    pub heizung: String,
    #[serde(default)]
    pub energie_primaer: String,
    #[serde(default)]
    pub energie_sekundaer: String,
    #[serde(default)]
    pub baustoff: String,
    #[serde(default)]
    pub fertigteilbau: bool,
    /// Veranschlagte Kosten des Bauwerks (Kostengruppen 300 und 400).
    #[serde(default)]
    pub kosten_bauwerk_euro: Option<f64>,
}
impl Default for Baukennwerte {
    fn default() -> Self {
        Self {
            gebaeudeart: "wohngebaeude".into(),
            bri_m3: None,
            euro_je_m3: None,
            promille: None,
            bautaetigkeit: Bautaetigkeit::Neubau,
            bauherrenart: None,
            wohnungen: None,
            heizung: String::new(),
            energie_primaer: String::new(),
            energie_sekundaer: String::new(),
            baustoff: String::new(),
            fertigteilbau: false,
            kosten_bauwerk_euro: None,
        }
    }
}
impl Baukennwerte {
    pub fn pruefen(&self) -> CoreResult<()> {
        let f = |t: &str| Err(CoreError::InvalidInput(t.into()));
        let positiv =
            |v: Option<f64>, max: f64| v.is_none_or(|v| v.is_finite() && v > 0. && v <= max);
        if self.gebaeudeart.trim().is_empty() || self.gebaeudeart.len() > 100 {
            return f("Gebäudeart fehlt");
        }
        if !positiv(self.bri_m3, 10_000_000.) {
            return f("Brutto-Rauminhalt muss größer als 0 sein");
        }
        if !positiv(self.euro_je_m3, 100_000.) {
            return f("Rohbauwert je m³ muss größer als 0 sein");
        }
        if !positiv(self.promille, 1000.) {
            return f("Gebührensatz muss zwischen 0 und 1000 ‰ liegen");
        }
        if !positiv(self.kosten_bauwerk_euro, 1e11) {
            return f("Kosten des Bauwerks müssen größer als 0 sein");
        }
        if self.wohnungen.is_some_and(|w| w > 100_000) {
            return f("Zahl der Wohnungen ist unplausibel");
        }
        for t in [
            &self.heizung,
            &self.energie_primaer,
            &self.energie_sekundaer,
            &self.baustoff,
        ] {
            if t.chars().count() > 300 {
                return f("Angabe zu lang");
            }
        }
        Ok(())
    }
}

/// Wert mit Herkunft.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Kennwert {
    pub wert: Option<f64>,
    /// „Eingabe“, Berechnungsname oder Regelpaket.
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Gebuehrenschaetzung {
    pub land: String,
    pub gebaeudeart: String,
    pub bri_m3: Kennwert,
    pub euro_je_m3: Kennwert,
    pub rohbauwert_euro: Option<f64>,
    pub promille: Kennwert,
    pub mindestgebuehr_euro: f64,
    pub gebuehr_euro: Option<f64>,
    pub rechtsgrundlage: String,
    /// `false` = Regelwerte ohne fachliche Abnahme.
    pub geprueft: bool,
    pub stand: String,
    pub offen: Vec<String>,
    pub hinweise: Vec<String>,
    pub quellen: Vec<Regelquelle>,
    pub gebaeudearten: Vec<Gebaeudeartwert>,
}

fn eingabe(v: Option<f64>) -> Option<Kennwert> {
    v.map(|w| Kennwert {
        wert: Some(w),
        quelle: "Eingabe im Projekt".into(),
    })
}

/// Rohbauwert und Gebühr (rein; `bri` ist der aus den Berechnungen ermittelte Wert).
pub fn schaetzen(
    land: Option<&Landesgebuehren>,
    k: &Baukennwerte,
    bri: Kennwert,
) -> Gebuehrenschaetzung {
    let mut offen = Vec::new();
    let bri = eingabe(k.bri_m3).unwrap_or(bri);
    if bri.wert.is_none() {
        offen.push(
            "Brutto-Rauminhalt fehlt: DIN-277-Berechnung mit Volumen vervollständigen oder BRI eingeben"
                .to_string(),
        );
    }
    let Some(l) = land else {
        offen.push("Für dieses Land sind noch keine Rohbauwerte hinterlegt; Werte eingeben".into());
        let euro = eingabe(k.euro_je_m3).unwrap_or(Kennwert {
            wert: None,
            quelle: String::new(),
        });
        let promille = eingabe(k.promille).unwrap_or(Kennwert {
            wert: None,
            quelle: String::new(),
        });
        let rohbau = bri.wert.zip(euro.wert).map(|(b, e)| (b * e).round());
        return Gebuehrenschaetzung {
            land: String::new(),
            gebaeudeart: k.gebaeudeart.clone(),
            gebuehr_euro: rohbau
                .zip(promille.wert)
                .map(|(r, p)| (r * p / 1000.).round()),
            rohbauwert_euro: rohbau,
            bri_m3: bri,
            euro_je_m3: euro,
            promille,
            mindestgebuehr_euro: 0.,
            rechtsgrundlage: String::new(),
            geprueft: false,
            stand: String::new(),
            offen,
            hinweise: vec![regeln().hinweis.clone()],
            quellen: vec![],
            gebaeudearten: vec![],
        };
    };
    let art = l.gebaeudearten.iter().find(|a| a.id == k.gebaeudeart);
    if art.is_none() {
        offen.push("Gebäudeart ist im Regelpaket nicht bekannt".into());
    }
    let euro = eingabe(k.euro_je_m3).unwrap_or_else(|| Kennwert {
        wert: art.and_then(|a| a.euro_je_m3),
        quelle: format!("Regelpaket {} (Schätzwert)", l.titel),
    });
    if euro.wert.is_none() {
        offen.push("Rohbauwert je m³ für diese Gebäudeart eingeben".into());
    }
    let promille = eingabe(k.promille).unwrap_or_else(|| Kennwert {
        wert: Some(l.gebuehr.promille),
        quelle: l.gebuehr.rechtsgrundlage.clone(),
    });
    let rohbau = bri.wert.zip(euro.wert).map(|(b, e)| (b * e).round());
    let gebuehr = rohbau
        .zip(promille.wert)
        .map(|(r, p)| (r * p / 1000.).max(l.gebuehr.mindestgebuehr_euro).round());
    let mut hinweise = vec![regeln().hinweis.clone()];
    if !l.geprueft {
        hinweise.push(format!(
            "{}: {}. Werte vor der Weitergabe prüfen.",
            l.titel, l.stand
        ));
    }
    hinweise.push(
        "Weitere Gebühren (Prüfstatik, Abweichungen, Abnahmen, Beteiligung anderer Stellen) sind nicht enthalten."
            .into(),
    );
    Gebuehrenschaetzung {
        land: l.land.clone(),
        gebaeudeart: art
            .map(|a| a.bezeichnung.clone())
            .unwrap_or_else(|| k.gebaeudeart.clone()),
        bri_m3: bri,
        euro_je_m3: euro,
        rohbauwert_euro: rohbau,
        promille,
        mindestgebuehr_euro: l.gebuehr.mindestgebuehr_euro,
        gebuehr_euro: gebuehr,
        rechtsgrundlage: l.gebuehr.rechtsgrundlage.clone(),
        geprueft: l.geprueft,
        stand: l.stand.clone(),
        offen,
        hinweise,
        quellen: l.quellen.clone(),
        gebaeudearten: l.gebaeudearten.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Statistikmerkmal {
    pub gruppe: String,
    pub bezeichnung: String,
    /// Leer = noch keine Angabe.
    pub wert: String,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Baustatistikbogen {
    pub merkmale: Vec<Statistikmerkmal>,
    /// Bezeichnungen ohne Angabe.
    pub offen: Vec<String>,
    pub hinweis: String,
}

fn zahl(v: f64, nachkomma: usize) -> String {
    let s = format!("{v:.nachkomma$}");
    let (ganz, rest) = s.split_once('.').map_or((s.as_str(), ""), |(a, b)| (a, b));
    let (vz, ganz) = ganz.strip_prefix('-').map_or(("", ganz), |g| ("-", g));
    let mut g = String::new();
    for (i, c) in ganz.chars().enumerate() {
        if i > 0 && (ganz.len() - i) % 3 == 0 {
            g.push('.');
        }
        g.push(c);
    }
    if rest.is_empty() {
        format!("{vz}{g}")
    } else {
        format!("{vz}{g},{rest}")
    }
}
pub fn euro(v: f64) -> String {
    format!("{} €", zahl(v, 0))
}

/// Summe einer Kennzahl über alle Berechnungen einer Art mit Wert.
struct Summe {
    wert: Option<f64>,
    quelle: String,
}

impl CoreServices {
    async fn kennzahl(&self, p: ProjectId, art: BerechnungsArt, id: &str) -> CoreResult<Summe> {
        let mut summe: Option<f64> = None;
        let mut namen = Vec::new();
        for f in self.fachobjekte(p, art.objekt()).await? {
            let Ok(plan) = serde_json::from_value::<FachPlan>(f.daten) else {
                continue;
            };
            let name = plan.bezeichnung.clone();
            let Ok((_, e)) = self.berechnung_pruefen(p, plan).await else {
                continue;
            };
            if let Some(v) = e
                .kennzahlen
                .iter()
                .find(|k| k.id == id)
                .and_then(|k| k.wert)
            {
                summe = Some(summe.unwrap_or(0.) + v);
                namen.push(name);
            }
        }
        Ok(Summe {
            wert: summe,
            quelle: namen.join(" + "),
        })
    }
    pub async fn baukennwerte(&self, p: ProjectId) -> CoreResult<Baukennwerte> {
        match self
            .repos
            .commit
            .fachobjekt(p, ObjektArt::Baukennwerte.modul(), &p.0.to_string())
            .await?
        {
            Some(f) => {
                serde_json::from_value(f.daten).map_err(|e| CoreError::InvalidInput(e.to_string()))
            }
            None => Ok(Baukennwerte::default()),
        }
    }
    pub async fn gebuehrenschaetzung(
        &self,
        p: ProjectId,
        wer: &Benutzer,
    ) -> CoreResult<Gebuehrenschaetzung> {
        self.projekt_zugriff(p, wer, ProjektRecht::Lesen).await?;
        let projekt = self.get_project(p).await?;
        let k = self.baukennwerte(p).await?;
        let bri = self.kennzahl(p, BerechnungsArt::Din277, "bri").await?;
        Ok(schaetzen(
            fuer_land(projekt.bundesland),
            &k,
            Kennwert {
                wert: bri.wert,
                quelle: if bri.quelle.is_empty() {
                    String::new()
                } else {
                    format!("DIN 277: {}", bri.quelle)
                },
            },
        ))
    }
    pub async fn baustatistik(
        &self,
        p: ProjectId,
        wer: &Benutzer,
    ) -> CoreResult<Baustatistikbogen> {
        use crate::kunden::Kundenkategorie as K;
        self.projekt_zugriff(p, wer, ProjektRecht::Lesen).await?;
        let projekt = self.get_project(p).await?;
        let k = self.baukennwerte(p).await?;
        let s = self.gebuehrenschaetzung(p, wer).await?;
        let bauherren = self.bauherren(p).await?;
        let nuf = self.kennzahl(p, BerechnungsArt::Din277, "nuf").await?;
        let wfl = self
            .kennzahl(p, BerechnungsArt::Wohnflaechen, "wohnflaeche")
            .await?;
        let we = self
            .kennzahl(p, BerechnungsArt::Wohnflaechen, "wohneinheiten")
            .await?;
        let vg = self
            .kennzahl(p, BerechnungsArt::Vollgeschosse, "vollgeschosse")
            .await?;
        let grundstueck = crate::grundstuecksinformationen::informationen(&projekt.planungsrecht)
            .vorhaben()
            .map(|f| f.bezeichnung())
            .collect::<Vec<_>>()
            .join("; ");
        let bauherrenart = k
            .bauherrenart
            .map(|a| (a, "Eingabe im Projekt"))
            .or_else(|| {
                bauherren.first().map(|b| {
                    (
                        match b.daten.kategorie {
                            K::Privat => Bauherrenart::PrivaterHaushalt,
                            K::OeffentlicheHand => Bauherrenart::OeffentlicheHand,
                            _ => Bauherrenart::Unternehmen,
                        },
                        "Kundenkategorie des Bauherrn",
                    )
                })
            });
        let mut m = Vec::new();
        let mut merk = |gruppe: &str, bez: &str, wert: String, quelle: &str| {
            m.push(Statistikmerkmal {
                gruppe: gruppe.into(),
                bezeichnung: bez.into(),
                wert,
                quelle: quelle.into(),
            })
        };
        let mit = |v: Option<f64>, n: usize, e: &str| {
            v.map(|v| format!("{} {e}", zahl(v, n)).trim().to_string())
                .unwrap_or_default()
        };
        merk(
            "Bauvorhaben",
            "Bezeichnung",
            projekt.name.clone(),
            "Projekt",
        );
        merk("Bauvorhaben", "Grundstück", grundstueck, "Flurstücke");
        merk(
            "Bauvorhaben",
            "Art der Bautätigkeit",
            match k.bautaetigkeit {
                Bautaetigkeit::Neubau => "Errichtung eines neuen Gebäudes",
                Bautaetigkeit::Bestand => "Baumaßnahme an bestehendem Gebäude",
            }
            .into(),
            "Eingabe im Projekt",
        );
        merk(
            "Bauherr",
            "Name",
            bauherren
                .iter()
                .map(|b| b.anzeigename())
                .collect::<Vec<_>>()
                .join("; "),
            "Bauherren des Projekts",
        );
        merk(
            "Bauherr",
            "Art des Bauherrn",
            bauherrenart
                .map(|(a, _)| a.bezeichnung().to_string())
                .unwrap_or_default(),
            bauherrenart.map(|(_, q)| q).unwrap_or(""),
        );
        merk(
            "Gebäude",
            "Gebäudeart",
            s.gebaeudeart.clone(),
            "Eingabe im Projekt",
        );
        merk(
            "Gebäude",
            "Vollgeschosse",
            mit(vg.wert, 0, ""),
            &format!("Vollgeschossprüfung {}", vg.quelle),
        );
        merk(
            "Gebäude",
            "Brutto-Rauminhalt",
            mit(s.bri_m3.wert, 0, "m³"),
            &s.bri_m3.quelle,
        );
        merk(
            "Gebäude",
            "Nutzungsfläche (DIN 277)",
            mit(nuf.wert, 2, "m²"),
            &format!("DIN 277 {}", nuf.quelle),
        );
        merk(
            "Gebäude",
            "Wohnfläche",
            mit(wfl.wert, 2, "m²"),
            &format!("Wohnflächenberechnung {}", wfl.quelle),
        );
        let (wohnungen, wq) = match k.wohnungen {
            Some(w) => (Some(w as f64), "Eingabe im Projekt".to_string()),
            None => (we.wert, format!("Wohnflächenberechnung {}", we.quelle)),
        };
        merk("Gebäude", "Anzahl Wohnungen", mit(wohnungen, 0, ""), &wq);
        merk(
            "Gebäude",
            "Überwiegend verwendeter Baustoff",
            k.baustoff.clone(),
            "Eingabe im Projekt",
        );
        merk(
            "Gebäude",
            "Fertigteilbau",
            if k.fertigteilbau { "ja" } else { "nein" }.into(),
            "Eingabe im Projekt",
        );
        merk(
            "Heizung",
            "Art der Heizung",
            k.heizung.clone(),
            "Eingabe im Projekt",
        );
        merk(
            "Heizung",
            "Energie primär",
            k.energie_primaer.clone(),
            "Eingabe im Projekt",
        );
        merk(
            "Heizung",
            "Energie sekundär",
            k.energie_sekundaer.clone(),
            "Eingabe im Projekt",
        );
        merk(
            "Kosten",
            "Veranschlagte Kosten des Bauwerks (KG 300 + 400)",
            k.kosten_bauwerk_euro.map(euro).unwrap_or_default(),
            "Eingabe im Projekt",
        );
        let offen = m
            .iter()
            .filter(|x| x.wert.trim().is_empty() && x.bezeichnung != "Energie sekundär")
            .map(|x| x.bezeichnung.clone())
            .collect();
        Ok(Baustatistikbogen {
            merkmale: m,
            offen,
            hinweis: "Arbeitshilfe zum Übertrag in den amtlichen Erhebungsbogen für Baugenehmigungen (Hochbaustatistikgesetz). Ersetzt den Vordruck des Statistischen Landesamts nicht.".into(),
        })
    }
    /// Gebührenschätzung und Statistikangaben als Schreiben im Briefbogen.
    pub async fn baukennwerte_pdf(
        &self,
        p: ProjectId,
        statistik: bool,
        wer: &Benutzer,
    ) -> CoreResult<Vec<u8>> {
        let projekt = self.get_project(p).await?;
        let buero = self.buero().await.ok();
        let absender = buero
            .as_ref()
            .map(|b| {
                let a = &b.anschrift;
                format!(
                    "{}\n{} {}\n{} {}",
                    b.name, a.strasse, a.hausnummer, a.postleitzahl, a.ort
                )
            })
            .unwrap_or_default();
        let mut bloecke = Vec::new();
        let betreff;
        if statistik {
            let b = self.baustatistik(p, wer).await?;
            betreff = "Angaben zur Baustatistik".to_string();
            bloecke.push(Briefblock::Hinweis {
                text: b.hinweis.clone(),
            });
            bloecke.push(Briefblock::Tabelle {
                koepfe: vec![
                    "Bereich".into(),
                    "Merkmal".into(),
                    "Angabe".into(),
                    "Quelle".into(),
                ],
                zeilen: b
                    .merkmale
                    .iter()
                    .map(|m| {
                        vec![
                            m.gruppe.clone(),
                            m.bezeichnung.clone(),
                            if m.wert.is_empty() {
                                "– offen –".into()
                            } else {
                                m.wert.clone()
                            },
                            m.quelle.clone(),
                        ]
                    })
                    .collect(),
            });
        } else {
            let s = self.gebuehrenschaetzung(p, wer).await?;
            betreff = "Schätzung Rohbauwert und Baugenehmigungsgebühr".to_string();
            let w = |v: Option<f64>, f: &dyn Fn(f64) -> String| {
                v.map(f).unwrap_or_else(|| "– offen –".into())
            };
            bloecke.push(Briefblock::Tabelle {
                koepfe: vec!["Größe".into(), "Wert".into(), "Herkunft".into()],
                zeilen: vec![
                    vec!["Gebäudeart".into(), s.gebaeudeart.clone(), String::new()],
                    vec![
                        "Brutto-Rauminhalt".into(),
                        w(s.bri_m3.wert, &|v| format!("{} m³", zahl(v, 0))),
                        s.bri_m3.quelle.clone(),
                    ],
                    vec![
                        "Rohbauwert je m³".into(),
                        w(s.euro_je_m3.wert, &|v| format!("{} €/m³", zahl(v, 2))),
                        s.euro_je_m3.quelle.clone(),
                    ],
                    vec![
                        "Rohbauwert".into(),
                        w(s.rohbauwert_euro, &euro),
                        String::new(),
                    ],
                    vec![
                        "Gebührensatz".into(),
                        w(s.promille.wert, &|v| format!("{} ‰", zahl(v, 2))),
                        s.promille.quelle.clone(),
                    ],
                    vec![
                        "Geschätzte Baugenehmigungsgebühr".into(),
                        w(s.gebuehr_euro, &euro),
                        format!("mindestens {}", euro(s.mindestgebuehr_euro)),
                    ],
                ],
            });
            for o in &s.offen {
                bloecke.push(Briefblock::Hinweis {
                    text: format!("Offen: {o}"),
                });
            }
            for h in &s.hinweise {
                bloecke.push(Briefblock::Absatz { text: h.clone() });
            }
        }
        let brief = Brief {
            absender,
            empfaenger: String::new(),
            datum: crate::kalender::heute(),
            betreff: format!("{betreff} – {}", projekt.name),
            aktenzeichen: String::new(),
            bloecke,
        };
        self.brief_pdf(p, &brief).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regelpaket_ist_ungeprueft_und_rechnet() {
        let nrw = fuer_land(Bundesland::Nw).expect("NRW im Regelpaket");
        assert!(!nrw.geprueft, "nur das Büro setzt geprueft auf true");
        let k = Baukennwerte::default();
        let s = schaetzen(
            Some(nrw),
            &k,
            Kennwert {
                wert: Some(800.),
                quelle: "DIN 277".into(),
            },
        );
        let e = nrw.gebaeudearten[0].euro_je_m3.unwrap();
        assert_eq!(s.rohbauwert_euro, Some((800. * e).round()));
        assert_eq!(
            s.gebuehr_euro,
            Some(((800. * e).round() * nrw.gebuehr.promille / 1000.).round())
        );
        assert!(s.offen.is_empty());
        assert!(s.hinweise.iter().any(|h| h.contains("prüfen")));

        // Eingaben überschreiben; Mindestgebühr greift.
        let k = Baukennwerte {
            bri_m3: Some(10.),
            euro_je_m3: Some(50.),
            promille: Some(10.),
            ..Default::default()
        };
        let s = schaetzen(
            Some(nrw),
            &k,
            Kennwert {
                wert: None,
                quelle: String::new(),
            },
        );
        assert_eq!(s.rohbauwert_euro, Some(500.));
        assert_eq!(s.gebuehr_euro, Some(nrw.gebuehr.mindestgebuehr_euro));
        assert_eq!(s.bri_m3.quelle, "Eingabe im Projekt");

        // Ohne BRI keine Zahl, aber ein offener Punkt.
        let s = schaetzen(
            Some(nrw),
            &Baukennwerte::default(),
            Kennwert {
                wert: None,
                quelle: String::new(),
            },
        );
        assert_eq!(s.gebuehr_euro, None);
        assert!(s.offen[0].contains("Brutto-Rauminhalt"));
        // Land ohne Werte.
        let s = schaetzen(
            None,
            &Baukennwerte::default(),
            Kennwert {
                wert: Some(1.),
                quelle: String::new(),
            },
        );
        assert!(s.offen.iter().any(|o| o.contains("keine Rohbauwerte")));
        assert!(
            Baukennwerte {
                promille: Some(-1.),
                ..Default::default()
            }
            .pruefen()
            .is_err()
        );
    }

    #[test]
    fn zahlen_deutsch() {
        assert_eq!(zahl(1234567.891, 2), "1.234.567,89");
        assert_eq!(zahl(950., 0), "950");
        assert_eq!(euro(140000.), "140.000 €");
    }
}
