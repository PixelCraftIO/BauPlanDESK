//! Lesende Ausgabe: genaue Quellversionen, archivierte Nachweise und getrennte
//! Fachberichte. Erzeugt keine DMS-Version und bestätigt keine Einreichung.
mod abstandsbericht;
mod berichte;
mod layout;
mod pdf;
mod vorlage;
use crate::{
    CoreError, CoreResult, CoreServices,
    domain::{DocumentId, ProjectId},
    plattform::ObjektArt,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
pub use vorlage::Druckgestaltung;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "art", rename_all = "snake_case", deny_unknown_fields)]
pub enum Druckquelle {
    Dokument {
        dokument_id: DocumentId,
        #[ts(type = "number")]
        version: i64,
        #[serde(default)]
        seiten: Vec<u32>,
    },
    Nachweis {
        id: Uuid,
    },
    Baugrundstueck,
    Baugrenzen,
    Raumbuch {
        id: Uuid,
        #[ts(type = "number")]
        revision: i64,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Druckauftrag {
    /// Fehlend: gespeicherter Bürostandard. Explizit: Gestaltung dieser Ausgabe.
    #[serde(default)]
    pub gestaltung: Option<Druckgestaltung>,
    #[serde(default)]
    pub aktenzeichen: String,
    #[serde(default)]
    pub datum: Option<chrono::NaiveDate>,
    pub titel: String,
    pub absender: String,
    pub empfaenger: String,
    pub anschreiben: String,
    pub mit_anschreiben: bool,
    pub inhaltsverzeichnis: bool,
    /// Deckblatt mit Angaben zum Bauvorhaben und Unterschriftsfeldern.
    #[serde(default)]
    pub deckblatt: bool,
    /// Trennblatt mit Anlagennummer vor jeder unverändert beigefügten
    /// Originalunterlage (Formulare, Pläne, Fotos).
    #[serde(default)]
    pub trennblaetter: bool,
    /// Aufstellende Person für Aufstellungsvermerk und Deckblatt. Fehlend:
    /// Vorschlag aus Bauantrag und Büroangaben (siehe [`Druckkatalog::aufgestellt_von`]).
    #[serde(default)]
    pub aufgestellt_von: Option<String>,
    /// Schützt aktuelle Fachberichte vor Änderungen während der Zusammenstellung.
    #[ts(type = "number")]
    pub projekt_version: i64,
    pub quellen: Vec<Druckquelle>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Druckeintrag {
    pub titel: String,
    pub quelle: Druckquelle,
    pub hinweis: Option<String>,
    pub verfuegbar: bool,
    /// Gehört zum empfohlenen Berechnungsheft: aktueller, jüngster Stand je
    /// Berechnungsgegenstand in fachlicher Reihenfolge.
    #[serde(default)]
    pub empfohlen: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Druckkatalog {
    pub gestaltung: Druckgestaltung,
    #[ts(type = "number")]
    pub projekt_version: i64,
    pub absender: String,
    pub eintraege: Vec<Druckeintrag>,
    /// Vorschlag für die aufstellende Person; leer, wenn weder Bauantrag noch
    /// Büroangaben eine/n Entwurfsverfasser/in eindeutig benennen.
    #[serde(default)]
    pub aufgestellt_von: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Druckposition {
    pub titel: String,
    pub erste_seite: u32,
    pub seiten: u32,
}
/// Interne Ausgabebeschriftung; keine Ableitung aus formatierten Dateititeln.
struct Inhaltsangabe {
    titel: String,
    kategorie: String,
    name: String,
    version: String,
    /// Anlagennummer im Paket.
    anlage: String,
    /// Anlagen, auf denen die Unterlage aufbaut (z. B. „2, 3“).
    bezug: String,
    /// Unverändert beigefügtes Original (erhält bei Bedarf ein Trennblatt).
    original: bool,
}
impl From<String> for Inhaltsangabe {
    fn from(titel: String) -> Self {
        Self {
            name: titel.clone(),
            titel,
            kategorie: "Berechnung".into(),
            version: "Aktueller Stand".into(),
            anlage: String::new(),
            bezug: String::new(),
            original: false,
        }
    }
}
pub struct Druckergebnis {
    pub pdf: Vec<u8>,
    pub positionen: Vec<Druckposition>,
    pub seiten: u32,
}
pub(super) fn fehler(e: impl std::fmt::Display) -> CoreError {
    CoreError::InvalidInput(format!("Druckausgabe: {e}"))
}
pub(super) fn dokumenttyp(name: &str) -> bool {
    name.rsplit('.').next().is_some_and(|e| {
        ["pdf", "eml", "msg", "png", "jpg", "jpeg"].contains(&e.to_ascii_lowercase().as_str())
    })
}
impl CoreServices {
    pub async fn druckkatalog(&self, p: ProjectId) -> CoreResult<Druckkatalog> {
        let projekt = self.get_project(p).await?;
        // Fachliche Reihenfolge eines Berechnungshefts: Grundstück, Maß der
        // Nutzung, Abstandsflächen, Fachberechnungen, Raumbücher, Dokumente.
        let mut berechnungen: Vec<(u32, i64, Druckeintrag)> = Vec::new();
        let grund = crate::grundstueckskontext::ermitteln(&projekt);
        let grund_ok = grund.as_ref().is_ok_and(|g| !g.is_empty());
        berechnungen.push((
            0,
            0,
            Druckeintrag {
                titel: "Baugrundstücksermittlung".into(),
                quelle: Druckquelle::Baugrundstueck,
                verfuegbar: grund_ok,
                hinweis: grund.err().map(|e| e.to_string()),
                empfohlen: grund_ok,
            },
        ));
        if projekt.planungsrecht.baugrenzen.is_some() {
            berechnungen.push((
                1,
                0,
                Druckeintrag {
                    titel: "Baugrenzenprüfung".into(),
                    quelle: Druckquelle::Baugrenzen,
                    verfuegbar: true,
                    hinweis: None,
                    empfohlen: true,
                },
            ));
        }
        let nachweise = self.nachweise(p).await?;
        // Jüngster aktueller Stand je Berechnungsgegenstand.
        let gegenstand = |n: &crate::plattform::nachweise::NachweisAnsicht| {
            use crate::plattform::nachweise::NachweisEingabe as E;
            match &n.stand.eingabe {
                E::Nutzung { fach, .. } => (
                    match fach {
                        crate::plattform::nachweise::NutzungsArt::Grz => 10,
                        crate::plattform::nachweise::NutzungsArt::Gfz => 11,
                        crate::plattform::nachweise::NutzungsArt::Geschossigkeit => 12,
                        crate::plattform::nachweise::NutzungsArt::Bmz => 13,
                    },
                    format!("nutzung:{fach:?}"),
                ),
                E::AbstandProjekt { objekt, .. } => (20, objekt.to_string()),
                E::Abstand { gegenstand, .. } => (21, format!("abstand:{gegenstand}")),
                E::Fach { objekt, plan } => (
                    30 + crate::berechnungen::BerechnungsArt::ALLE
                        .iter()
                        .position(|m| *m == plan.modul)
                        .unwrap_or(99) as u32,
                    objekt.to_string(),
                ),
            }
        };
        let mut neueste = std::collections::HashMap::new();
        for n in nachweise.iter().filter(|n| n.aktuell) {
            let (_, key) = gegenstand(n);
            let e = neueste.entry(key).or_insert(n.stand.erstellt_am);
            if n.stand.erstellt_am > *e {
                *e = n.stand.erstellt_am;
            }
        }
        for n in &nachweise {
            let (rang, key) = gegenstand(n);
            let empfohlen = n.aktuell && neueste.get(&key) == Some(&n.stand.erstellt_am);
            let mut status = berichte::nachweisstatus(n);
            if !empfohlen && n.aktuell {
                status.push_str(" · durch neueren Stand ersetzt");
            }
            berechnungen.push((
                rang,
                -n.stand.erstellt_am.timestamp(),
                Druckeintrag {
                    titel: format!(
                        "{} · {}",
                        n.stand.bezeichnung,
                        n.stand.erstellt_am.format("%d.%m.%Y %H:%M UTC")
                    ),
                    quelle: Druckquelle::Nachweis { id: n.stand.id },
                    verfuegbar: true,
                    hinweis: Some(status),
                    empfohlen,
                },
            ));
        }
        for r in self.fachobjekte(p, ObjektArt::Raumbuch).await? {
            let rb: crate::plattform::objekte::Raumbuch =
                serde_json::from_value(r.daten).map_err(fehler)?;
            berechnungen.push((
                50,
                0,
                Druckeintrag {
                    titel: format!("Raumbuch: {}", rb.bezeichnung),
                    quelle: Druckquelle::Raumbuch {
                        id: Uuid::parse_str(&r.objekt).map_err(fehler)?,
                        revision: r.revision,
                    },
                    verfuegbar: true,
                    hinweis: None,
                    empfohlen: true,
                },
            ));
        }
        berechnungen.sort_by_key(|(rang, zeit, _)| (*rang, *zeit));
        let mut eintraege: Vec<Druckeintrag> =
            berechnungen.into_iter().map(|(_, _, e)| e).collect();
        for d in crate::projektablage::versionen(self.list_documents(p).await?, false) {
            let verfuegbar = dokumenttyp(&d.file_name);
            eintraege.push(Druckeintrag {titel:format!("{} · Version {}",d.file_name,d.version),
                quelle:Druckquelle::Dokument{dokument_id:d.document_id,version:d.version,seiten:vec![]},verfuegbar,
                hinweis:(!verfuegbar).then(||"Im zugeordneten Programm drucken oder als PDF ins Projekt übernehmen. Originaldatei kann einzeln exportiert werden.".into()),
                empfohlen: false});
        }
        let b = self.buero().await?;
        let absender = format!(
            "{}\n{} {}\n{} {}",
            b.name,
            b.anschrift.strasse,
            b.anschrift.hausnummer,
            b.anschrift.postleitzahl,
            b.anschrift.ort
        )
        .trim()
        .to_owned();
        Ok(Druckkatalog {
            gestaltung: self.druckgestaltung().await?,
            projekt_version: projekt.version,
            absender,
            eintraege,
            aufgestellt_von: self.aufsteller_vorschlag(&projekt).await?,
        })
    }
    pub async fn druckausgabe(&self, p: ProjectId, a: Druckauftrag) -> CoreResult<Druckergebnis> {
        self.druckausgabe_mit_planindex(p, a, &Default::default())
            .await
    }
    /// Wie [`Self::druckausgabe`]; das Inhaltsverzeichnis nennt zusätzlich den
    /// Planindex der Dokumentversionen (Anlagenverzeichnis eines Abgabestands).
    pub(crate) async fn druckausgabe_mit_planindex(
        &self,
        p: ProjectId,
        mut a: Druckauftrag,
        planindizes: &std::collections::HashMap<(DocumentId, i64), String>,
    ) -> CoreResult<Druckergebnis> {
        if a.quellen.is_empty()
            || a.quellen.len() > 100
            || a.titel.trim().is_empty()
            || a.titel.len() > 500
            || a.absender.len() > 4000
            || a.empfaenger.len() > 4000
            || a.anschreiben.len() > 16000
            || a.aktenzeichen.len() > 500
            || a.aufgestellt_von.as_ref().is_some_and(|t| t.len() > 500)
        {
            return Err(fehler(
                "Titel und 1 bis 100 Unterlagen erforderlich; Textgrenzen beachten",
            ));
        }
        let gestaltung = match a.gestaltung.take() {
            Some(g) => g,
            None => self.druckgestaltung().await?,
        };
        gestaltung.validate()?;
        a.gestaltung = Some(gestaltung);
        let projekt = self.get_project(p).await?;
        if projekt.version != a.projekt_version {
            return Err(fehler(
                "Projekt wurde geändert. Auswahl aktualisieren und erneut erstellen.",
            ));
        }
        let jetzt = chrono::Utc::now();
        let nachweise = self.nachweise(p).await?;
        let kategorien = self.kategoriesnapshots(p).await?;
        let (bezuege, deck) = self.paketbezuege(&projekt, &a).await?;
        let mut quellen = Vec::new();
        let mut bytes = 0usize;
        let mut druckdaten = 0usize;
        for (index, q) in a.quellen.iter().enumerate() {
            let anlage = (index + 1).to_string();
            let kopf = |gegenstand: &str, stand: String, zusatzblaetter: usize| layout::Blattkopf {
                bauvorhaben: projekt.name.clone(),
                gegenstand: gegenstand.into(),
                stand,
                anlage: Some(anlage.clone()),
                art: String::new(),
                zusatzblaetter,
            };
            let (titel, mut doc) = match q {
                Druckquelle::Dokument {
                    dokument_id,
                    version,
                    seiten,
                } => {
                    let meta = self
                        .document_history(*dokument_id)
                        .await?
                        .into_iter()
                        .find(|d| d.version == *version && d.project_id == p)
                        .ok_or_else(|| fehler("Dokumentversion gehört nicht zu diesem Projekt"))?;
                    if !dokumenttyp(&meta.file_name) {
                        return Err(fehler(format!(
                            "{} zuerst als PDF bereitstellen",
                            meta.file_name
                        )));
                    }
                    if meta.size_bytes > 64 * 1024 * 1024 {
                        return Err(fehler("Quelldatei größer als 64 MB"));
                    }
                    let (_, daten) = self.load_version(*dokument_id, *version).await?;
                    bytes = bytes
                        .checked_add(daten.len())
                        .ok_or_else(|| fehler("Druckpaket zu groß"))?;
                    if bytes > 128 * 1024 * 1024 {
                        return Err(fehler("Druckpaket größer als 128 MB; bitte aufteilen"));
                    }
                    let name = meta.file_name.to_ascii_lowercase();
                    let mut original = true;
                    let doc = if name.ends_with(".pdf") {
                        pdf::auswahl(&daten, seiten)?
                    } else if [".png", ".jpg", ".jpeg"].iter().any(|e| name.ends_with(e)) {
                        if !seiten.is_empty() {
                            return Err(fehler("Seitenauswahl nur bei PDF-Dateien"));
                        }
                        pdf::bild(&daten)?
                    } else {
                        if !seiten.is_empty() {
                            return Err(fehler("Seitenauswahl nur bei PDF-Dateien"));
                        }
                        original = false;
                        let mail = crate::email::lesen(&meta.file_name, &daten)?;
                        pdf::bericht(
                            &meta.file_name,
                            Some(&kopf(&meta.file_name, format!("Version {version}"), 0)),
                            &berichte::email(&mail),
                        )?
                    };
                    let kategorie = kategorien
                        .get(&dokument_id.0.to_string())
                        .and_then(|k| k.as_ref())
                        .map(|k| {
                            k.hauptkategorie
                                .as_ref()
                                .map(|h| format!("{h} / {}", k.name))
                                .unwrap_or_else(|| k.name.clone())
                        })
                        .unwrap_or_else(|| "Ohne Kategorie".into());
                    (
                        Inhaltsangabe {
                            titel: format!("{} · Version {}", meta.file_name, version),
                            kategorie,
                            name: meta.file_name,
                            version: match planindizes.get(&(*dokument_id, *version)) {
                                Some(i) => format!("Version {version} · Index {i}"),
                                None => format!("Version {version}"),
                            },
                            anlage: anlage.clone(),
                            bezug: String::new(),
                            original,
                        },
                        doc,
                    )
                }
                _ => {
                    let bericht =
                        berichte::erstellen(self, &projekt, q, &nachweise, &bezuege).await?;
                    let titel = bericht.verzeichnisname.clone();
                    // Die Vektorplanskizze ist das letzte Blatt dieser Anlage.
                    let skizze = match q {
                        Druckquelle::Nachweis { id } => nachweise
                            .iter()
                            .find(|n| n.stand.id == *id)
                            .and_then(|n| match &n.stand.eingabe {
                                crate::plattform::nachweise::NachweisEingabe::AbstandProjekt {
                                    plan,
                                    ..
                                } => Some((plan, &n.stand.ergebnis)),
                                _ => None,
                            })
                            .map(|(plan, e)| {
                                serde_json::from_value::<crate::abstandsflaechen::AbstandsErgebnis>(
                                    e.clone(),
                                )
                                .map(|r| (plan, r))
                                .map_err(fehler)
                            })
                            .transpose()?
                            .filter(|(plan, r)| {
                                crate::abstandsflaechen::bericht::hat_planskizze(plan, r)
                            }),
                        _ => None,
                    };
                    let mut doc = pdf::bericht(
                        &bericht.titel,
                        Some(&kopf(
                            &titel,
                            bericht.stand.clone(),
                            usize::from(skizze.is_some()),
                        )),
                        &bericht.zeilen,
                    )?;
                    if let Some((plan, r)) = &skizze {
                        let m = doc.get_pages().len() + 1;
                        crate::abstandsflaechen::bericht::planseite(
                            &mut doc,
                            plan,
                            r,
                            Some(&format!(
                                "Anlage {anlage} - Blatt {m} von {m} - {}",
                                projekt.name
                            )),
                        )?;
                    }
                    let version = match q {
                        Druckquelle::Nachweis { id } => nachweise
                            .iter()
                            .find(|n| n.stand.id == *id)
                            .map(|n| {
                                format!(
                                    "Nachweis vom {}",
                                    n.stand.erstellt_am.format("%d.%m.%Y %H:%M UTC")
                                )
                            })
                            .unwrap_or_else(|| "Nachweisstand".into()),
                        Druckquelle::Raumbuch { revision, .. } => format!("Revision {revision}"),
                        _ => format!("Projektstand {}", projekt.version),
                    };
                    let kategorie = match q {
                        Druckquelle::Baugrundstueck => "Berechnung / Baugrundstück",
                        Druckquelle::Baugrenzen => "Berechnung / Baugrenzen",
                        Druckquelle::Raumbuch { .. } => "Berechnung / Raumbuch",
                        Druckquelle::Nachweis { id } => match nachweise
                            .iter()
                            .find(|n| n.stand.id == *id)
                            .map(|n| &n.stand.eingabe)
                        {
                            Some(crate::plattform::nachweise::NachweisEingabe::Fach {
                                plan,
                                ..
                            }) => plan.modul.titel(),
                            Some(crate::plattform::nachweise::NachweisEingabe::Nutzung {
                                fach,
                                ..
                            }) => match fach {
                                crate::plattform::nachweise::NutzungsArt::Grz => "Berechnung / GRZ",
                                crate::plattform::nachweise::NutzungsArt::Gfz => "Berechnung / GFZ",
                                crate::plattform::nachweise::NutzungsArt::Bmz => "Berechnung / BMZ",
                                crate::plattform::nachweise::NutzungsArt::Geschossigkeit => {
                                    "Berechnung / Geschossigkeit"
                                }
                            },
                            _ => "Berechnung / Abstandsflächen",
                        },
                        Druckquelle::Dokument { .. } => unreachable!(),
                    };
                    (
                        Inhaltsangabe {
                            name: titel.clone(),
                            titel,
                            kategorie: kategorie.into(),
                            version,
                            anlage: anlage.clone(),
                            bezug: bericht
                                .verweise
                                .iter()
                                .map(u32::to_string)
                                .collect::<Vec<_>>()
                                .join(", "),
                            original: false,
                        },
                        doc,
                    )
                }
            };
            if doc.get_pages().is_empty() {
                return Err(fehler("PDF enthält keine Seiten"));
            }
            pdf::formulare_einfrieren(&mut doc)?;
            druckdaten = druckdaten.saturating_add(
                doc.objects
                    .values()
                    .filter_map(|o| o.as_stream().ok())
                    .map(|s| s.content.len())
                    .sum::<usize>(),
            );
            if druckdaten > 128 * 1024 * 1024 {
                return Err(fehler(
                    "Aufbereitete Druckdaten über 128 MB; bitte aufteilen",
                ));
            }
            quellen.push((titel, doc));
        }
        // Erneuter Vergleich verhindert vermischte aktuelle Projektgrundlagen.
        if self.get_project(p).await?.version != a.projekt_version {
            return Err(fehler(
                "Projekt während der Ausgabe geändert. Erneut erstellen.",
            ));
        }
        pdf::paket(&projekt.name, a, quellen, jetzt, Some(deck))
    }
    /// Entwurfsverfasser/in laut Bauantrag (oder die einzige des Büros) mit
    /// Berufsbezeichnung und Kammernummer, ergänzt um das Büro.
    async fn aufsteller_vorschlag(&self, projekt: &crate::domain::Project) -> CoreResult<String> {
        let buero = self.buero().await?;
        let antrag = self.bauantrag(projekt.id).await?.antrag;
        let person = antrag
            .entwurfsverfasser_von(&buero)
            .map(|e| {
                [
                    format!("{} {}", e.vorname, e.familienname)
                        .trim()
                        .to_string(),
                    e.berufsbezeichnung.trim().to_string(),
                    if e.mitgliedsnummer.trim().is_empty() {
                        String::new()
                    } else {
                        format!("Kammer-Nr. {}", e.mitgliedsnummer.trim())
                    },
                ]
                .into_iter()
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
                .join(", ")
            })
            .unwrap_or_default();
        if person.is_empty() {
            return Ok(String::new());
        }
        Ok([person, buero.name.trim().to_string()]
            .into_iter()
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" · "))
    }
    /// Anlagennummern der beigefügten Grundlagen, Aufstellende und Deckblattangaben.
    async fn paketbezuege(
        &self,
        projekt: &crate::domain::Project,
        a: &Druckauftrag,
    ) -> CoreResult<(berichte::Bezuege, pdf::Deckblatt)> {
        let mut b = berichte::Bezuege::default();
        for (i, q) in a.quellen.iter().enumerate() {
            let nr = i as u32 + 1;
            match q {
                Druckquelle::Nachweis { id } => {
                    b.nachweise.entry(*id).or_insert(nr);
                }
                Druckquelle::Raumbuch { id, .. } => {
                    b.raumbuecher.entry(*id).or_insert(nr);
                }
                Druckquelle::Baugrundstueck => {
                    b.baugrundstueck.get_or_insert(nr);
                }
                _ => {}
            }
        }
        let verfasser = match a.aufgestellt_von.as_deref().map(str::trim) {
            Some(t) if !t.is_empty() => t.replace('\n', ", "),
            _ => self.aufsteller_vorschlag(projekt).await?,
        };
        b.verfasser = vec![(
            "Aufgestellt (Entwurfsverfasser/in)".into(),
            verfasser.clone(),
        )];
        let bauherren = self
            .repos
            .kunden
            .bauherren(projekt.id)
            .await?
            .iter()
            .map(|k| k.anzeigename())
            .collect::<Vec<_>>()
            .join("\n");
        let infos = crate::grundstuecksinformationen::informationen(&projekt.planungsrecht);
        let grundstueck = infos
            .vorhaben()
            .map(|f| {
                format!(
                    "Gemarkung {}, Flur {}, Flurstück {}",
                    f.gemarkung,
                    if f.flur.is_empty() { "—" } else { &f.flur },
                    f.nummer
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        let r = &projekt.planungsrecht;
        let standort = [r.standort.trim(), r.gemeinde.trim()]
            .into_iter()
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        Ok((
            b,
            pdf::Deckblatt {
                standort,
                grundstueck,
                bauherren,
                entwurfsverfasser: verfasser,
            },
        ))
    }
}

/// Höchstgröße einer abgelegten Druckausgabe (PDF). Adapter richten ihre
/// Transportgrenze (z. B. HTTP-Body-Limit) danach aus.
pub const MAX_DRUCKPDF: usize = 128 * 1024 * 1024;

impl CoreServices {
    /// Übernimmt exakt die geprüfte Druckkopie als versionierte Projektunterlage.
    /// Die Vorgangskennung verhindert eine zweite Datei nach einem Antwortverlust.
    pub async fn druckpdf_ablegen(
        &self,
        p: ProjectId,
        name: &str,
        pdf: bytes::Bytes,
        vorgang: uuid::Uuid,
        wer: &crate::benutzer::Benutzer,
    ) -> CoreResult<crate::domain::DocumentVersion> {
        self.projekt_zugriff(p, wer, crate::zugriff::ProjektRecht::Bearbeiten)
            .await?;
        if pdf.len() > MAX_DRUCKPDF || !pdf.starts_with(b"%PDF-") || name.len() > 400 {
            return Err(fehler("Ungültige PDF-Ausgabe"));
        }
        let d = lopdf::Document::load_mem(&pdf).map_err(fehler)?;
        if d.get_pages().is_empty() {
            return Err(fehler("PDF-Ausgabe enthält keine Seiten"));
        }
        let k = self
            .kategorienkatalog()
            .await?
            .kategorien
            .iter()
            .any(|k| k.id == "berechnung" && k.aktiv)
            .then_some("berechnung");
        self.ablegen_vorgang(
            p,
            None,
            None,
            &format!("{}.pdf", name.trim_end_matches(".pdf")),
            pdf,
            wer,
            crate::domain::Aktion::DokumentAbgelegt,
            "Druckkopie für Verfahrensunterlagen",
            Some(vorgang),
            vec![crate::commit::DokumentZusatz {
                modul: "dokumentkategorie".into(),
                schema_version: 1,
                daten: serde_json::to_value(k).map_err(fehler)?,
            }],
        )
        .await
    }
}
