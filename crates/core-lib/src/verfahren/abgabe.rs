//! Ein eingefrorener Stand und seine originalen Ausgabe-Artefakte. Export liest
//! nur diese Bytes; neue DMS-Versionen beeinflussen frühere Ausgaben nicht.
use super::*;
use crate::{
    domain::{DocumentVersion, Project},
    plattform::objekte::Dokumentbezug,
    storage::{ContentHash, StorageKey},
    xbau::{Bauantrag, Beteiligter, Buero},
};
use bytes::Bytes;
use std::io::{Cursor, Write};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Versandstatus {
    Versendet,
    Eingegangen,
    Zurueckgewiesen,
    Ungeklaert,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Versandereignis {
    pub id: Uuid,
    pub status: Versandstatus,
    pub datum: NaiveDate,
    pub weg: String,
    pub referenz: String,
    pub notiz: String,
    pub beleg: Option<Dokumentbezug>,
    pub berichtigt: Option<Uuid>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Abgabedokument {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub kategorie: Option<crate::dokumentkategorien::KategorieStand>,
    /// Beim Festhalten im Anlagenverzeichnis ausgewiesener Planindex (Index bei
    /// Einreichung als nächster Stand). Maßgebend ist die Ableitung in
    /// [`super::planindex::ableiten`]; Abweichungen meldet sie. Fehlt in älteren
    /// Abgabeständen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub planindex: Option<String>,
    pub dokument: Dokumentbezug,
    pub name: String,
    pub sha256: String,
    #[ts(type = "number")]
    pub bytes: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Artefakt {
    pub format: String,
    pub dateiname: String,
    pub storage_key: String,
    pub sha256: String,
    #[ts(type = "number")]
    pub bytes: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Abgabegrundlagen {
    pub projekt: Project,
    pub antrag: Bauantrag,
    pub bauherren: Vec<Beteiligter>,
    pub buero: Buero,
    /// Weitere Projektbeteiligte (Eigentümer, Bevollmächtigte, Fachplaner) beim
    /// Festhalten; fehlen in älteren Abgabeständen.
    #[serde(default)]
    pub beteiligte: Vec<crate::kunden::Projektbeteiligter>,
    /// Verfahrensart beim Festhalten; fehlt in älteren Abgabeständen.
    #[serde(default)]
    pub art: Option<Verfahrensart>,
    #[serde(default)]
    pub antragsangaben: Antragsangaben,
    /// Behörde der Verfahrensakte beim Festhalten; maßgebend für Empfänger
    /// von XBau und Druck. Fehlt in älteren Abgabeständen.
    #[serde(default)]
    pub behoerde: Option<Behoerde>,
    /// Aktenzeichen der Verfahrensakte beim Festhalten; fehlt in älteren
    /// Abgabeständen.
    #[serde(default)]
    pub aktenzeichen: Option<String>,
    /// Gebäudeklasse aus dem Vorhabenprofil der Akte beim Festhalten; fehlt
    /// in älteren Abgabeständen und ohne Profil.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gebaeudeklasse: Option<u32>,
    /// Änderungsantrag: Aktenzeichen des Vorgängerverfahrens (erteilte
    /// Baugenehmigung) beim Festhalten; fehlt in älteren Abgabeständen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hauptgenehmigung: Option<String>,
    /// Beantragte Abweichungen, Ausnahmen und Befreiungen beim Festhalten
    /// (ohne entfallene); fehlen in älteren Abgabeständen.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub abweichungen: Vec<super::abweichungen::Abweichung>,
    /// Folgenachricht dieses Stands (Baubeginnsanzeige …); fehlt bei
    /// Anträgen, Nachreichungen und älteren Abgabeständen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folge: Option<Folgewunsch>,
}
/// Größte Summe der Unterlagen eines Abgabestands. Dieselbe Grenze gilt für
/// den entpackten Inhalt eines XBau-Pakets, damit eine Abgabe mit XBau nicht
/// erst nach dem Zusammenstellen scheitert.
pub const MAX_ABGABE: u64 = crate::xbau::austausch::MAX_ENTPACKT;
fn megabyte(b: u64) -> String {
    format!("{:.1}", b as f64 / (1024.0 * 1024.0)).replace('.', ",")
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Abgabestand {
    pub id: Uuid,
    pub auftrag_hash: String,
    pub titel: String,
    pub erstellt_am: DateTime<Utc>,
    pub erstellt_von: Uuid,
    #[ts(type = "JsonValue")]
    pub grundlagen: Abgabegrundlagen,
    pub dokumente: Vec<Abgabedokument>,
    pub anforderungen: Vec<Anforderung>,
    pub hinweise: Vec<String>,
    pub artefakte: Vec<Artefakt>,
    pub versand: Vec<Versandereignis>,
    /// Vom Core aus Originalereignissen und Berichtigungen abgeleitet.
    pub versandstatus: Option<Versandstatus>,
}
/// Folgenachricht im laufenden Verfahren statt der Antragsnachricht.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "art", rename_all = "snake_case", deny_unknown_fields)]
pub enum Folgewunsch {
    /// Stellungnahme zur Anhörung (0204); die Beanstandungen des Schritts mit
    /// ihrer Reaktion, sonst das Anschreiben.
    Stellungnahme { schritt: Uuid },
    /// Isolierter Antrag auf Abweichung (0220) mit den Abweichungen der Akte.
    Abweichungsantrag,
    /// Antrag auf Verlängerung der Geltungsdauer (0260).
    Verlaengerung { bis: Option<NaiveDate> },
    /// Baubeginnsanzeige (0900).
    Baubeginn {
        datum: NaiveDate,
        pruefung_standsicherheit: bool,
    },
    /// Anzeige der Nutzungsaufnahme (0910).
    Nutzungsaufnahme {
        datum: NaiveDate,
        standsicherheit_bescheinigt: bool,
        brandschutz_bescheinigt: bool,
        wiederkehrende_pruefungen: bool,
    },
    /// Rücknahme des Antrags (1130); das Anschreiben ist die Erklärung.
    Ruecknahme,
}
impl Folgewunsch {
    pub fn bezeichnung(&self) -> &'static str {
        match self {
            Self::Stellungnahme { .. } => "Stellungnahme zur Anhörung",
            Self::Abweichungsantrag => "Antrag auf Abweichung",
            Self::Verlaengerung { .. } => "Antrag auf Verlängerung",
            Self::Baubeginn { .. } => "Baubeginnsanzeige",
            Self::Nutzungsaufnahme { .. } => "Anzeige der Nutzungsaufnahme",
            Self::Ruecknahme => "Rücknahme",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Abgabewunsch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub xbau_bezug: Option<Uuid>,
    /// XBau-Folgenachricht statt des Antrags; fehlt bei Anträgen und
    /// Nachreichungen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub xbau_folge: Option<Folgewunsch>,
    pub id: Uuid,
    pub titel: String,
    pub dokumente: Vec<Dokumentbezug>,
    pub xbau: bool,
    pub druck: bool,
    pub anschreiben: String,
}
pub fn versandstand(b: &Abgabestand) -> Option<Versandstatus> {
    letztes_versandereignis(b).map(|e| e.status.clone())
}
/// Maßgebendes Ereignis nach Berichtigungen; Datum und Status stammen aus derselben Entscheidung.
pub fn letztes_versandereignis(b: &Abgabestand) -> Option<&Versandereignis> {
    b.versand
        .iter()
        .enumerate()
        .filter(|(_, e)| e.berichtigt.is_none())
        .map(|(index, root)| {
            let mut e = root;
            for _ in 0..b.versand.len() {
                if let Some(neu) = b.versand.iter().find(|n| n.berichtigt == Some(e.id)) {
                    e = neu;
                } else {
                    break;
                }
            }
            (e.datum, index, e)
        })
        .max_by_key(|(datum, index, _)| (*datum, *index))
        .map(|(_, _, ereignis)| ereignis)
}
/// Datum einer XBau-Anlage (`datum`: Erstellung bzw. letzte Bearbeitung der
/// Bauvorlage): das Plandatum der Dokumentdaten, wenn sie genau diese Version
/// beschreiben; sonst der Ablagetag der Version im DMS.
pub(crate) fn anlagedatum(
    d: &DocumentVersion,
    plandaten: &std::collections::HashMap<
        crate::domain::DocumentId,
        crate::plattform::objekte::Dokumentdaten,
    >,
) -> chrono::NaiveDate {
    plandaten
        .get(&d.document_id)
        .filter(|m| m.dokument.version == d.version)
        .and_then(|m| m.datum)
        .unwrap_or_else(|| crate::kalender::datum_deutsch(d.created_at))
}
impl CoreServices {
    /// Folgenachricht (Dateiname, XML) aus der Akte und dem eingefrorenen Stand.
    #[allow(clippy::too_many_arguments)]
    async fn folgenachricht(
        &self,
        p: ProjectId,
        a: &Verfahrensakte,
        f: &Folgewunsch,
        daten: &crate::xbau::Nachrichtendaten<'_>,
        aktenzeichen: &str,
        anschreiben: &str,
        wer: &Benutzer,
    ) -> CoreResult<(String, String)> {
        use crate::xbau::folge::{Folgeart, Stellungnahmepunkt, folgenachricht};
        let genehmigt = || {
            super::termine::wirksame_bescheide(a).any(|b| {
                matches!(
                    b.entscheidung,
                    Entscheidung::Genehmigt | Entscheidung::TeilweiseGenehmigt
                )
            })
        };
        let kopf;
        let punkte: Vec<Stellungnahmepunkt>;
        let art = match f {
            Folgewunsch::Stellungnahme { schritt } => {
                let s = a
                    .schritte
                    .iter()
                    .find(|s| s.id == *schritt && s.art == Schrittart::Anhoerung)
                    .ok_or_else(|| fehler("Anhörung nicht in dieser Akte"))?;
                kopf = self
                    .xbau_nachrichten(p, wer)
                    .await?
                    .into_iter()
                    .find(|n| crate::xbau::postfach::kommunikation_id(n.id) == s.kommunikation)
                    .map(|n| n.nachricht.pruefung.kopf)
                    .ok_or_else(|| {
                        fehler(
                            "Die Anhörung kam nicht per XBau; Stellungnahme ohne XBau festhalten",
                        )
                    })?;
                punkte = if s.gruende.is_empty() {
                    vec![Stellungnahmepunkt {
                        regelverstoss: "Anhörung".into(),
                        stellungnahme: anschreiben.into(),
                    }]
                } else {
                    s.gruende
                        .iter()
                        .map(|g| Stellungnahmepunkt {
                            regelverstoss: g.text.chars().take(500).collect(),
                            stellungnahme: g.reaktion.clone(),
                        })
                        .collect()
                };
                Folgeart::Stellungnahme {
                    anhoerung: &kopf,
                    punkte: &punkte,
                }
            }
            Folgewunsch::Abweichungsantrag => {
                if daten.vorhaben.abweichungen.iter().all(|x| x.erleichterung) {
                    return Err(fehler(
                        "Keine beantragte Abweichung mit XBau-Code in dieser Akte; Abweichung erfassen und Codeliste „Abweichung Art“ einspielen",
                    ));
                }
                Folgeart::Abweichungsantrag {
                    abweichungen: &daten.vorhaben.abweichungen,
                }
            }
            Folgewunsch::Verlaengerung { bis } => {
                if !genehmigt() {
                    return Err(fehler(
                        "Eine Verlängerung setzt eine erteilte Genehmigung in dieser Akte voraus",
                    ));
                }
                Folgeart::Verlaengerung { bis: *bis }
            }
            Folgewunsch::Baubeginn {
                datum,
                pruefung_standsicherheit,
            } => {
                if !genehmigt() && a.art != Verfahrensart::Freistellung {
                    return Err(fehler(
                        "Die Baubeginnsanzeige setzt eine erteilte Genehmigung oder eine Genehmigungsfreistellung voraus",
                    ));
                }
                Folgeart::Baubeginn {
                    datum: *datum,
                    pruefung_standsicherheit: *pruefung_standsicherheit,
                }
            }
            Folgewunsch::Nutzungsaufnahme {
                datum,
                standsicherheit_bescheinigt,
                brandschutz_bescheinigt,
                wiederkehrende_pruefungen,
            } => Folgeart::Nutzungsaufnahme {
                datum: *datum,
                standsicherheit_bescheinigt: *standsicherheit_bescheinigt,
                brandschutz_bescheinigt: *brandschutz_bescheinigt,
                gebaeudeklasse4: daten.vorhaben.gebaeudeklasse == Some(4),
                wiederkehrende_pruefungen: *wiederkehrende_pruefungen,
            },
            Folgewunsch::Ruecknahme => {
                if anschreiben.trim().is_empty() {
                    return Err(fehler("Die Rücknahme braucht die Erklärung im Anschreiben"));
                }
                Folgeart::Ruecknahme {
                    information: anschreiben,
                }
            }
        };
        Ok((art.dateiname(), folgenachricht(daten, art, aktenzeichen)?))
    }
    /// Hinweise, wenn die Landes-Verfahrensart des Bauantrags der Akte widerspricht.
    pub(crate) fn verfahrensart_abgleichen(
        &self,
        projekt: &Project,
        antrag: &Bauantrag,
        akte: &Verfahrensakte,
    ) -> Vec<String> {
        use super::abgleich::Landesverfahrensart;
        let uri = crate::xbau::codeliste::verfahrensart_uri(projekt.bundesland);
        let codelisten = self.codelisten.aktuell();
        let beschreibung = match codelisten.verfahrensart(projekt.bundesland, &antrag.verfahrensart)
        {
            Ok(Some((liste, code))) => liste
                .eintraege
                .iter()
                .find(|e| e.code == code)
                .map(|e| e.beschreibung.clone()),
            Ok(None) => None,
            // Fehlende oder ungültige Codeliste meldet der Bauantrag selbst.
            Err(_) => return vec![],
        };
        let landesart = match &beschreibung {
            Some(b) => Landesverfahrensart::Gewaehlt(b),
            None if codelisten.liste(&uri).is_none() => Landesverfahrensart::KeineListe(&uri),
            None => Landesverfahrensart::NichtGewaehlt,
        };
        super::abgleich::verfahrensart_hinweise(&akte.art, akte.pruefverfahren.as_ref(), landesart)
    }
    /// Dokumentdaten (Plannummer, Index, Plandatum) je Dokument des Projekts.
    /// Unlesbare Einträge bleiben unberücksichtigt.
    async fn plandaten(
        &self,
        p: ProjectId,
    ) -> CoreResult<
        std::collections::HashMap<
            crate::domain::DocumentId,
            crate::plattform::objekte::Dokumentdaten,
        >,
    > {
        Ok(self
            .fachobjekte(p, crate::plattform::ObjektArt::Dokumentdaten)
            .await?
            .into_iter()
            .filter_map(|f| {
                serde_json::from_value::<crate::plattform::objekte::Dokumentdaten>(f.daten).ok()
            })
            .map(|d| (d.dokument.dokument_id, d))
            .collect())
    }
    pub async fn verfahren_abgabe_festhalten(
        &self,
        p: ProjectId,
        vid: Uuid,
        basis: i64,
        w: Abgabewunsch,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        self.projekt_zugriff(p, wer, crate::zugriff::ProjektRecht::Bearbeiten)
            .await?;
        let auftrag_hash =
            ContentHash::of(&serde_json::to_vec(&w).map_err(|e| fehler(e.to_string()))?)
                .to_string();
        let (f, mut a) = self.verfahrensakte(p, vid).await?;
        if let Some(b) = a.abgaben.iter().find(|b| b.id == w.id) {
            if b.auftrag_hash == auftrag_hash {
                return Ok(f);
            }
            return Err(fehler(
                "Vorgangskennung bereits mit anderem Abgabestand belegt",
            ));
        }
        if f.revision != basis {
            return Err(CoreError::VersionConflict {
                expected: basis,
                actual: f.revision,
            });
        }
        crate::plattform::text(&w.titel, 500)?;
        if !w.xbau && (w.xbau_bezug.is_some() || w.xbau_folge.is_some()) {
            return Err(fehler("XBau-Bezug erfordert eine XBau-Ausgabe"));
        }
        if w.xbau_bezug.is_some() && w.xbau_folge.is_some() {
            return Err(fehler(
                "Eine Folgenachricht ist keine Nachreichung zum Antrag; Bezug entfernen",
            ));
        }
        if w.dokumente.is_empty() || w.dokumente.len() > 100 || w.anschreiben.len() > 16000 {
            return Err(fehler(
                "1 bis 100 Unterlagen und höchstens 16000 Zeichen Anschreiben",
            ));
        }
        let projekt = self.get_project(p).await?;
        // Der Bauantrag liefert die Flurstücke bereits aus den zentralen Grundstücksinformationen.
        let mut antrag = self.bauantrag(p).await?.antrag;
        // Die Verfahrensakte ist maßgebend für Behörde und Aktenzeichen; die
        // Bauaufsicht im Bauantrag ist nur noch ein Altwert.
        let mut hinweise = Vec::new();
        let behoerde = &a.behoerde;
        if w.xbau && behoerde.kennung.trim().is_empty() {
            return Err(fehler(
                "Für die XBau-Ausgabe die Behördenkennung (DVDV) in der Verfahrensakte erfassen",
            ));
        }
        if !behoerde.kennung.trim().is_empty() || !behoerde.name.trim().is_empty() {
            let alt = &antrag.bauaufsicht;
            if !alt.kennung.trim().is_empty() && alt.kennung.trim() != behoerde.kennung.trim() {
                hinweise.push(format!(
                    "Bauaufsicht im Bauantrag ({}) weicht von der Behörde der Verfahrensakte ({}) ab. Maßgebend ist die Verfahrensakte.",
                    alt.kennung.trim(),
                    behoerde.kennung.trim()
                ));
            }
            antrag.bauaufsicht = crate::xbau::Bauaufsicht {
                name: behoerde.name.trim().into(),
                kennung: behoerde.kennung.trim().into(),
            };
        }
        hinweise.extend(self.verfahrensart_abgleichen(&projekt, &antrag, &a));
        let aktenzeichen = a.aktenzeichen.trim().to_string();
        // Ein Änderungsantrag ist ein Nachtrag zur Baugenehmigung des
        // Vorgängerverfahrens; XBau nennt deren Aktenzeichen.
        let hauptgenehmigung = match (&a.art, a.vorgaenger) {
            (Verfahrensart::Aenderungsantrag, Some(v)) => self
                .verfahrensakte(p, v)
                .await
                .ok()
                .map(|(_, v)| v.aktenzeichen.trim().to_string())
                .filter(|az| !az.is_empty()),
            _ => None,
        };
        if w.xbau && a.art == Verfahrensart::Aenderungsantrag && hauptgenehmigung.is_none() {
            hinweise.push("Änderungsantrag ohne Bezug auf die erteilte Baugenehmigung: In der Akte das Vorgängerverfahren mit Aktenzeichen eintragen, damit die XBau-Nachricht den Nachtrag zuordnet.".into());
        }
        let mut grundlagen = Abgabegrundlagen {
            bauherren: self.bauherren_fuer_antrag(p, &antrag).await?,
            buero: self.buero().await?,
            projekt: projekt.clone(),
            antrag,
            beteiligte: self.repos.kunden.beteiligte(p).await?,
            art: Some(a.art.clone()),
            antragsangaben: a.antragsangaben.clone(),
            behoerde: Some(a.behoerde.clone()),
            aktenzeichen: Some(aktenzeichen.clone()),
            gebaeudeklasse: a.profil.as_ref().and_then(|p| p.vorhaben.gebaeudeklasse),
            hauptgenehmigung,
            abweichungen: a
                .abweichungen
                .iter()
                .filter(|e| e.aktiv())
                .cloned()
                .collect(),
            folge: w.xbau_folge.clone(),
        };
        // Jede Unterschreitung im aktuellen Planungsstand ist beantragt oder
        // begründet nicht erforderlich; sonst wird nicht festgehalten.
        if super::abweichungen::betroffen(&a.art)
            && w.xbau_folge
                .as_ref()
                .is_none_or(|f| *f == Folgewunsch::Abweichungsantrag)
        {
            let abgleich = self.verfahren_abweichungen(p, vid).await?;
            hinweise.extend(super::abweichungen::abgabe_pruefen(&a, &abgleich)?);
        }
        let fragen = grundlagen.antragsangaben.fragen();
        let xbau_art = if w.xbau && w.xbau_folge.is_none() {
            Some(a.art.xbau_antrag(&grundlagen.antragsangaben, &fragen)?)
        } else {
            None
        };
        let mut seen = std::collections::HashSet::new();
        let mut docs = Vec::new();
        let mut inhalte = Vec::new();
        let mut summe = 0u64;
        for d in &w.dokumente {
            if !seen.insert(d.dokument_id) {
                return Err(fehler("Je Dokument eine Version wählen"));
            }
            let meta = self
                .document_history(d.dokument_id)
                .await?
                .into_iter()
                .find(|v| v.version == d.version && v.project_id == p)
                .ok_or_else(|| fehler("Abgabeunterlage gehört nicht zum Projekt"))?;
            summe = summe.saturating_add(meta.size_bytes);
            if summe > MAX_ABGABE {
                return Err(fehler(format!(
                    "Die Unterlagen sind zusammen größer als {} MB. Abgabe aufteilen.",
                    megabyte(MAX_ABGABE)
                )));
            }
            if w.xbau && meta.size_bytes > crate::xbau::austausch::MAX_PAKET {
                return Err(fehler(format!(
                    "{} ist {} MB groß; eine XBau-Anlage darf höchstens {} MB groß sein. Datei verkleinern oder Abgabe ohne XBau festhalten.",
                    meta.file_name,
                    megabyte(meta.size_bytes),
                    megabyte(crate::xbau::austausch::MAX_PAKET)
                )));
            }
            let (_, daten) = self.load_version(d.dokument_id, d.version).await?;
            docs.push(meta);
            inhalte.push(daten);
        }
        // Abgeleitete Anforderungen nach dem aktuellen Stand des Projekts.
        hinweise.extend(self.anforderungen_abgleichen(p, vid, &mut a).await?);
        for (d, daten) in docs.iter().zip(&inhalte) {
            let aktuell = self.repos.documents.latest(d.document_id).await?;
            if aktuell.version != d.version {
                hinweise.push(format!("{}: ausgewählt ist Version {}, im DMS existiert Version {}. Historischen Stand bewusst prüfen.", d.file_name, d.version, aktuell.version));
            }
            // Formularwerte gegen die aktuellen Projektdaten; nur Hinweis.
            hinweise.extend(
                self.formular_hinweis(d.document_id, d.version, &d.file_name, daten)
                    .await,
            );
        }
        hinweise.extend(super::optionen::vorgaenger_hinweise(self, p, &a).await?);
        let phase = match &w.xbau_folge {
            Some(Folgewunsch::Baubeginn { .. }) => Some(super::ausfuehrung::Phase::Baubeginn),
            Some(Folgewunsch::Nutzungsaufnahme { .. }) => {
                Some(super::ausfuehrung::Phase::Nutzungsaufnahme)
            }
            _ => None,
        };
        if let Some(phase) = phase {
            let c = super::ausfuehrung::checkliste(
                &a,
                projekt.bundesland,
                crate::kalender::heute(),
                phase,
                &self.verfahren_planunterlagen(p, vid).await?,
            );
            hinweise.extend(
                c.voraussetzungen
                    .iter()
                    .filter(|v| v.erfuellt != Some(true) && !v.titel.ends_with("angezeigt"))
                    .map(|v| format!("Voraussetzung offen: {} ({})", v.titel, v.hinweis)),
            );
            hinweise.extend(c.hinweise);
        }
        // Nachweise, deren Grundlagen sich seit dem letzten Stand geändert
        // haben (etwa durch eine Planänderung), gehören nicht ungeprüft in
        // eine Abgabe.
        let mut nachweise = self.nachweise(p).await?;
        nachweise.sort_by_key(|n| std::cmp::Reverse(n.stand.erstellt_am));
        let mut gesehen = std::collections::HashSet::new();
        for n in nachweise {
            if gesehen.insert(n.stand.bezeichnung.clone()) && !n.aktuell {
                hinweise.push(format!(
                    "Nachweis „{}“ beruht nicht mehr auf dem aktuellen Planungsstand (z. B. nach einer Planänderung). Neu erstellen, bevor er eingereicht wird.",
                    n.stand.bezeichnung
                ));
            }
        }
        for r in &a.anforderungen {
            if r.erforderlich == Erforderlichkeit::Ungeklaert {
                hinweise.push(format!("Anforderung ungeklärt: {}", r.titel));
            } else if r.erforderlich == Erforderlichkeit::Erforderlich
                && (!r.freigegeben
                    || r.dokumente.is_empty()
                    || r.dokumente.iter().any(|d| !w.dokumente.contains(d)))
            {
                hinweise.push(format!(
                    "Nicht vollständig enthalten oder ungeprüft: {}",
                    r.titel
                ));
            }
        }
        // Ohne Wahl im Bauantrag gilt die Anlagenart der Dokumentkategorie; der
        // eingefrorene Stand hält sie fest, damit das Paket nachvollziehbar bleibt.
        if w.xbau {
            let vorschlaege = self.anlagenart_vorschlaege(p).await?;
            let anlagen = &mut grundlagen.antrag.anlagen;
            for d in &docs {
                let wahl = anlagen.iter_mut().find(|a| a.dokument == d.document_id);
                if wahl
                    .as_ref()
                    .is_some_and(|a| !a.anlageart.trim().is_empty())
                {
                    continue;
                }
                let Some(v) = vorschlaege.get(&d.document_id) else {
                    continue;
                };
                match wahl {
                    Some(a) => a.anlageart = v.anlageart.clone(),
                    None => anlagen.push(crate::xbau::Anlagenwahl {
                        dokument: d.document_id,
                        anlageart: v.anlageart.clone(),
                        bezeichnung: String::new(),
                    }),
                }
                hinweise.push(format!(
                    "{}: Anlagenart {} aus der Dokumentkategorie „{}“ übernommen (im Bauantrag nicht gewählt).",
                    d.file_name, v.anlageart, v.kategorie
                ));
            }
        }
        // Weitere XBau-Inhalte aus dem eingefrorenen Stand und dem aktuellen,
        // nicht veralteten Berechnungsstand des Projekts.
        let vorhaben = if w.xbau {
            let g = &grundlagen;
            // Gleicher Rechenweg wie Berechnung und Nachweis (mit Gebäuden).
            let (wirksam, gebaeudehinweise) = self.projekt_mit_gebaeuden(&g.projekt).await?;
            hinweise.extend(gebaeudehinweise);
            let (flaechen, hinweis) = crate::xbau::FlaechenBauNvo::aus_projekt(&wirksam);
            hinweise.extend(hinweis);
            crate::xbau::Vorhabenangaben {
                beteiligte: crate::kunden::antragsbeteiligte(
                    &g.projekt,
                    &g.bauherren,
                    &g.beteiligte,
                )?,
                gebaeudeklasse: g.gebaeudeklasse,
                hauptgenehmigung: g.hauptgenehmigung.clone(),
                flaechen,
                abweichungen: {
                    let codelisten = self.codelisten.aktuell();
                    let mut liste = Vec::new();
                    for e in &g.abweichungen {
                        match super::abweichungen::xbau_code(e, &codelisten) {
                            Some((uri, version, code))
                                if matches!(a.art, Verfahrensart::Bauantrag
                                    | Verfahrensart::Aenderungsantrag
                                    | Verfahrensart::Nutzungsaenderung) =>
                            {
                                liste.push(crate::xbau::BeantragteAbweichung {
                                    id: e.id,
                                    erleichterung: e.art
                                        == super::abweichungen::Abweichungsart::Erleichterung,
                                    liste: uri,
                                    version,
                                    code,
                                    vorschrift: format!(
                                        "{}\n{}",
                                        e.vorschrift.trim(),
                                        e.rechtsgrundlage.trim()
                                    ),
                                    beschreibung: format!(
                                        "{}\nSoll: {}\nIst: {}",
                                        e.gegenstand.trim(),
                                        e.soll.trim(),
                                        e.ist.trim()
                                    ),
                                    begruendung: e.begruendung.clone(),
                                });
                            }
                            Some(_) => {}
                            None => hinweise.push(format!(
                                "{} „{}“ steht nicht in der XBau-Nachricht: Codeliste {} einspielen oder Code wählen. Antrag und Begründung als Anlage beifügen.",
                                e.art.bezeichnung(),
                                e.gegenstand.trim(),
                                e.art.codeliste()
                            )),
                        }
                    }
                    liste
                },
            }
        } else {
            crate::xbau::Vorhabenangaben::default()
        };
        let kategorien = self.kategoriesnapshots(p).await?;
        // Index, den die Unterlagen bei Einreichung dieses Stands erhalten.
        let planindizes = super::planindex::index_bei_einreichung(&a, &w.dokumente);
        let mut stand = Abgabestand {
            id: w.id,
            auftrag_hash,
            titel: w.titel,
            erstellt_am: Utc::now(),
            erstellt_von: wer.id.0,
            grundlagen,
            dokumente: docs
                .iter()
                .zip(&planindizes)
                .map(|(d, index)| Abgabedokument {
                    planindex: Some(index.clone()),
                    kategorie: kategorien
                        .get(&d.document_id.0.to_string())
                        .cloned()
                        .flatten(),
                    dokument: Dokumentbezug {
                        dokument_id: d.document_id,
                        version: d.version,
                    },
                    name: d.file_name.clone(),
                    sha256: d.sha256.to_string(),
                    bytes: d.size_bytes,
                })
                .collect(),
            anforderungen: a.anforderungen.clone(),
            hinweise,
            artefakte: vec![],
            versand: vec![],
            versandstatus: None,
        };
        stand.hinweise.extend(super::landesrecht::abgabe_hinweise(
            projekt.bundesland,
            &stand.dokumente,
            &a.unterschriften,
        )?);
        let mut ausgaben = Vec::new();
        let zip = zip(&stand, &docs, &inhalte)?;
        ausgaben.push(("zip", "Unterlagen.zip", zip));
        if w.xbau {
            let namen =
                crate::xbau::paket::eindeutige_namen(docs.iter().map(|d| d.file_name.as_str()));
            let plandaten = self.plandaten(p).await?;
            let anlagen = docs
                .iter()
                .zip(&namen)
                .map(|(d, name)| {
                    // Bezeichnung und Anlageart aus der Anlagenwahl des Bauantrags.
                    let wahl = stand
                        .grundlagen
                        .antrag
                        .anlagen
                        .iter()
                        .find(|a| a.dokument == d.document_id);
                    crate::xbau::Anlagendatei {
                        dokumentid: d.document_id.0,
                        bezeichnung: wahl
                            .map(|a| a.bezeichnung.trim())
                            .filter(|b| !b.is_empty())
                            .unwrap_or(&d.file_name)
                            .to_string(),
                        versionsnummer: d.version,
                        datum: anlagedatum(d, &plandaten),
                        anlageart: wahl.map(|a| a.anlageart.clone()).unwrap_or_default(),
                        dateiname: name.clone(),
                    }
                })
                .collect::<Vec<_>>();
            let codelisten = self.codelisten.aktuell();
            let daten = crate::xbau::Nachrichtendaten {
                projekt: &projekt,
                antrag: &stand.grundlagen.antrag,
                bauherren: &stand.grundlagen.bauherren,
                buero: &stand.grundlagen.buero,
                vorhaben: &vorhaben,
                anlagen: &anlagen,
                codelisten: &codelisten,
                nachricht_id: w.id,
                zeitpunkt: stand.erstellt_am,
            };
            let (nachricht, xml) = if let Some(f) = &w.xbau_folge {
                self.folgenachricht(p, &a, f, &daten, &aktenzeichen, &w.anschreiben, wer)
                    .await?
            } else if let Some(id) = w.xbau_bezug {
                let art = xbau_art.expect("Antragsart ohne Folgenachricht");
                let n = self
                    .xbau_nachrichten(p, wer)
                    .await?
                    .into_iter()
                    .find(|n| n.id == id)
                    .ok_or_else(|| fehler("XBau-Bezug nicht in diesem Projekt"))?;
                let mut kopf = n.nachricht.pruefung.kopf.clone();
                match kopf.aktenzeichen.as_deref().map(str::trim) {
                    Some(az)
                        if !az.is_empty() && !aktenzeichen.is_empty() && az != aktenzeichen =>
                    {
                        return Err(fehler(
                            "Aktenzeichen der Bezugsnachricht passt nicht zur Verfahrensakte",
                        ));
                    }
                    Some(az) if !az.is_empty() => {}
                    // Fehlt es in der Bezugsnachricht, nennt die Nachreichung das der Akte.
                    _ if !aktenzeichen.is_empty() => kopf.aktenzeichen = Some(aktenzeichen.clone()),
                    _ => {}
                }
                (
                    art.dateiname(true)?,
                    crate::xbau::antrag_geaendert(&daten, art, &kopf)?,
                )
            } else {
                let art = xbau_art.expect("Antragsart ohne Folgenachricht");
                (art.dateiname(false)?, crate::xbau::antrag(&daten, art)?)
            };
            let bytes = crate::xbau::paket::packen_nachricht(
                &nachricht,
                &xml,
                &namen
                    .iter()
                    .zip(&inhalte)
                    .map(|(n, b)| (n.clone(), b.as_ref()))
                    .collect::<Vec<_>>(),
            )?;
            if bytes.len() as u64 > crate::xbau::austausch::MAX_PAKET {
                return Err(fehler(format!(
                    "Das XBau-Paket wäre {} MB groß; zulässig sind höchstens {} MB. Unterlagen auf mehrere Abgaben aufteilen (Nachreichung) oder Abgabe ohne XBau festhalten.",
                    megabyte(bytes.len() as u64),
                    megabyte(crate::xbau::austausch::MAX_PAKET)
                )));
            }
            crate::xbau::austausch::pruefen("XBau.zip", &bytes, self.xbau_pruefer.as_ref()).await?;
            ausgaben.push(("xbau", "XBau.zip", bytes));
        }
        if w.druck {
            let indizes = w
                .dokumente
                .iter()
                .zip(&planindizes)
                .map(|(d, i)| ((d.dokument_id, d.version), i.clone()))
                .collect();
            let druck = self
                .druckausgabe_mit_planindex(
                    p,
                    crate::druckausgabe::Druckauftrag {
                        gestaltung: None,
                        aktenzeichen: aktenzeichen.clone(),
                        datum: None,
                        titel: stand.titel.clone(),
                        absender: stand.grundlagen.buero.name.clone(),
                        empfaenger: a.behoerde.name.clone(),
                        anschreiben: w.anschreiben.clone(),
                        mit_anschreiben: !w.anschreiben.trim().is_empty(),
                        inhaltsverzeichnis: true,
                        // Behördenpaket: Deckblatt und Trennblätter machen jede
                        // Originalunterlage über ihre Anlagennummer zuordenbar.
                        deckblatt: true,
                        trennblaetter: true,
                        aufgestellt_von: None,
                        projekt_version: projekt.version,
                        quellen: w
                            .dokumente
                            .iter()
                            .map(|d| crate::druckausgabe::Druckquelle::Dokument {
                                dokument_id: d.dokument_id,
                                version: d.version,
                                seiten: vec![],
                            })
                            .collect(),
                    },
                    &indizes,
                )
                .await?;
            ausgaben.push(("pdf", "Druckausgabe.pdf", druck.pdf));
        }
        if self.get_project(p).await?.version != projekt.version {
            return Err(fehler(
                "Projekt während Zusammenstellung geändert; erneut prüfen",
            ));
        }
        let mut keys = Vec::new();
        let resultat = async {
            for (format, name, bytes) in ausgaben {
                let key = StorageKey::new(format!(
                    "projekte/{p}/verfahrensausgaben/{}/{}-{name}",
                    w.id,
                    Uuid::new_v4()
                ))?;
                let meta = self.storage.save(&key, Bytes::from(bytes)).await?;
                keys.push(key.clone());
                stand.artefakte.push(Artefakt {
                    format: format.into(),
                    dateiname: name.into(),
                    storage_key: key.to_string(),
                    sha256: meta.sha256.to_string(),
                    bytes: meta.size_bytes,
                });
            }
            a.abgaben.push(stand);
            self.verfahren_commit(p, vid, basis, a, wer, true).await
        }
        .await;
        if resultat.is_err() {
            for k in keys {
                let _ = self.storage.delete(&k).await;
            }
        }
        resultat
    }
    /// Übernimmt das eingefrorene XBau-Artefakt einer Abgabe als ausgehende
    /// Nachricht ins XBau-Journal. Mehrfaches Erfassen, auch über das Portal,
    /// ergibt dieselbe Nachricht.
    pub async fn verfahren_xbau_erfassen(
        &self,
        p: ProjectId,
        vid: Uuid,
        abgabe: Uuid,
        wer: &Benutzer,
    ) -> CoreResult<crate::xbau::postfach::Nachrichtenstand> {
        self.pruefe(wer, Recht::DokumentAblegen).await?;
        self.projekt_zugriff(p, wer, crate::zugriff::ProjektRecht::Bearbeiten)
            .await?;
        let (_, a) = self.verfahrensakte(p, vid).await?;
        let artefakt = a
            .abgaben
            .iter()
            .find(|b| b.id == abgabe)
            .ok_or_else(|| fehler("Verfahrensabgabe nicht gefunden"))?
            .artefakte
            .iter()
            .find(|a| a.format == "xbau")
            .ok_or_else(|| fehler("Dieser eingefrorene Stand enthält kein XBau-Artefakt"))?;
        let bytes = self
            .storage
            .load_verified(
                &StorageKey::new(artefakt.storage_key.clone())?,
                &ContentHash::try_from(artefakt.sha256.clone())?,
            )
            .await?;
        let pruefung =
            crate::xbau::austausch::pruefen("XBau.zip", &bytes, self.xbau_pruefer.as_ref()).await?;
        if pruefung.kopf.id != abgabe {
            return Err(fehler(
                "XBau-Nachrichten-ID passt nicht zur eingefrorenen Abgabe",
            ));
        }
        let d = self
            .ablegen_vorgang(
                p,
                None,
                None,
                "XBau-Abgabestand.zip",
                bytes,
                wer,
                crate::domain::Aktion::DokumentAbgelegt,
                " · eingefrorenes XBau der Abgabe",
                // Derselbe Schlüssel wie früher beim Portal hält beide Wege idempotent.
                Some(Uuid::new_v5(&abgabe, b"portal.xbau.original")),
                vec![],
            )
            .await?;
        self.xbau_aufnehmen(
            p,
            crate::xbau::postfach::Nachrichtenaufnahme {
                original: Dokumentbezug {
                    dokument_id: d.document_id,
                    version: d.version,
                },
                richtung: crate::xbau::postfach::Richtung::Ausgang,
                zuordnung_bestaetigt: true,
            },
            wer,
        )
        .await
    }
    pub async fn verfahren_artefakt(
        &self,
        p: ProjectId,
        vid: Uuid,
        id: Uuid,
        format: &str,
        wer: &Benutzer,
    ) -> CoreResult<(String, Bytes)> {
        self.projekt_zugriff(p, wer, crate::zugriff::ProjektRecht::Lesen)
            .await?;
        let (_, a) = self.verfahrensakte(p, vid).await?;
        let f = a
            .abgaben
            .iter()
            .find(|b| b.id == id)
            .and_then(|b| b.artefakte.iter().find(|f| f.format == format))
            .ok_or_else(|| fehler("Ausgabeformat nicht im Abgabestand enthalten"))?;
        Ok((
            f.dateiname.clone(),
            self.storage
                .load_verified(
                    &StorageKey::new(f.storage_key.clone())?,
                    &ContentHash::try_from(f.sha256.clone())?,
                )
                .await?,
        ))
    }
}
fn zip(stand: &Abgabestand, docs: &[DocumentVersion], bytes: &[Bytes]) -> CoreResult<Vec<u8>> {
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opt = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    z.start_file("Abgabestand.json", opt)
        .map_err(|e| fehler(e.to_string()))?;
    z.write_all(&serde_json::to_vec_pretty(stand).map_err(|e| fehler(e.to_string()))?)
        .map_err(|e| fehler(e.to_string()))?;
    for (d, b) in docs.iter().zip(bytes) {
        z.start_file(
            format!(
                "Unterlagen/{}/v{}-{}",
                d.document_id.0,
                d.version,
                crate::xbau::paket::eindeutige_namen([d.file_name.as_str()])[0]
            ),
            opt,
        )
        .map_err(|e| fehler(e.to_string()))?;
        z.write_all(b).map_err(|e| fehler(e.to_string()))?;
    }
    Ok(z.finish().map_err(|e| fehler(e.to_string()))?.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plattform::objekte::Dokumentdaten;

    #[test]
    fn anlagedatum_ist_plandatum_der_version() {
        let d = DocumentVersion {
            document_id: crate::domain::DocumentId(Uuid::new_v4()),
            project_id: ProjectId(Uuid::new_v4()),
            version: 2,
            file_name: "Grundrisse.pdf".into(),
            storage_key: String::new(),
            sha256: ContentHash::of(b"x"),
            size_bytes: 1,
            created_by: String::new(),
            created_at: "2026-10-21T08:00:00Z".parse().unwrap(),
        };
        let ablage = chrono::NaiveDate::from_ymd_opt(2026, 10, 21).unwrap();
        let plan = chrono::NaiveDate::from_ymd_opt(2026, 9, 8).unwrap();
        let mut m = std::collections::HashMap::new();
        assert_eq!(anlagedatum(&d, &m), ablage, "ohne Dokumentdaten: Ablagetag");
        let mut daten = Dokumentdaten {
            dokument: Dokumentbezug {
                dokument_id: d.document_id,
                version: 2,
            },
            dokumentart: "Grundriss".into(),
            plannummer: "A-101".into(),
            revisionsindex: "a".into(),
            massstab: "1:100".into(),
            verfasser: String::new(),
            datum: Some(plan),
        };
        m.insert(d.document_id, daten.clone());
        assert_eq!(anlagedatum(&d, &m), plan);
        // Dokumentdaten einer anderen Version gelten nicht für diese.
        daten.dokument.version = 1;
        m.insert(d.document_id, daten.clone());
        assert_eq!(anlagedatum(&d, &m), ablage);
        daten.dokument.version = 2;
        daten.datum = None;
        m.insert(d.document_id, daten);
        assert_eq!(anlagedatum(&d, &m), ablage);
    }
}
