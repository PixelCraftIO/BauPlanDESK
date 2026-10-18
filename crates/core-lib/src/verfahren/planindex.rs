//! Planindex und Einreichungsstatus je Planunterlage, abgeleitet aus der
//! Verfahrensakte.
//!
//! Vertrag:
//! - Eingabe: die Verfahrensakte (Abgabestände mit Versandereignissen,
//!   Bescheide, Dokumentbeziehungen) und optional die aktuelle DMS-Version je
//!   Dokument. Es wird nichts gespeichert; das Ergebnis ist bei jedem Lesen neu
//!   abgeleitet und damit nie veraltet.
//! - Planindex: Je Dokument erhält jede Version, die erstmals in einem
//!   versendeten oder eingegangenen Abgabestand enthalten war, den nächsten
//!   Buchstaben (A, B, …, Z, AA, …). Maßgebend ist die Reihenfolge der
//!   Einreichung (Datum des maßgebenden Versandereignisses, dann Erstellung des
//!   Abgabestands). Zurückgewiesene, ungeklärte und nicht versendete Stände
//!   vergeben keinen Index.
//! - Status: siehe [`Einreichungsstatus`]. Der Bürostatus einer Unterlage
//!   ([`crate::plattform::objekte::UnterlagenStatus`]: Entwurf, Prüfung,
//!   Freigegeben) bleibt ein eigenes, manuelles Merkmal der internen Freigabe.
//! - Der Revisionsindex der Dokumentdaten ist die Kennung des Planverfassers und
//!   wird hier nicht ausgewertet.
use super::*;
use crate::domain::DocumentId;
use std::collections::HashMap;

/// Abgeleiteter Stand einer Planunterlage im Verfahren.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Einreichungsstatus {
    /// In keinem versendeten Abgabestand enthalten.
    Entwurf,
    /// Erstmals mit dem ersten versendeten Abgabestand der Akte eingereicht.
    Eingereicht,
    /// Erstmals mit einem späteren Abgabestand eingereicht.
    Nachgereicht,
    /// Genehmigte Unterlage eines wirksamen (nicht ersetzten) Bescheids.
    Genehmigt,
    /// Durch eine später eingereichte Version oder eine Dokumentbeziehung
    /// abgelöst (und nicht als einzige genehmigte Fassung maßgebend).
    Ersetzt,
}
impl Einreichungsstatus {
    pub fn bezeichnung(self) -> &'static str {
        match self {
            Self::Entwurf => "Entwurf",
            Self::Eingereicht => "Eingereicht",
            Self::Nachgereicht => "Nachgereicht",
            Self::Genehmigt => "Genehmigt",
            Self::Ersetzt => "Ersetzt",
        }
    }
}

/// Eine Version einer Planunterlage im Verfahren.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Planstand {
    #[ts(type = "number")]
    pub version: i64,
    pub planindex: Option<String>,
    pub status: Einreichungsstatus,
    /// Datum der ersten Einreichung dieser Version.
    pub eingereicht_am: Option<NaiveDate>,
    /// Versendete Abgabestände, die diese Version enthielten, in Einreichungsfolge.
    pub abgaben: Vec<Uuid>,
    /// Wirksamer Bescheid, der diese Version genehmigt.
    pub bescheid: Option<Uuid>,
    pub ersetzt_durch: Option<Dokumentbezug>,
}

/// Planindex und Status einer Planunterlage (eines Dokuments).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Planunterlage {
    pub dokument_id: DocumentId,
    /// Dateiname aus dem jüngsten Abgabestand.
    pub name: String,
    /// Zuletzt vergebener Planindex; `None`, solange nichts eingereicht ist.
    pub planindex: Option<String>,
    /// Status der zuletzt eingereichten Version, sonst Entwurf.
    pub status: Einreichungsstatus,
    /// Neuere, noch nicht eingereichte Version (DMS oder vorbereiteter Abgabestand).
    #[ts(type = "number | null")]
    pub offene_version: Option<i64>,
    /// Nach Version aufsteigend.
    pub staende: Vec<Planstand>,
    /// Abweichungen, z. B. ein im Anlagenverzeichnis anders ausgewiesener Index.
    pub hinweise: Vec<String>,
}

/// A, B, …, Z, AA, AB, … (0-basiert).
pub fn indexbuchstaben(mut n: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push(b'A' + (n % 26) as u8);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    s.reverse();
    String::from_utf8(s).expect("ASCII")
}

fn versendet(b: &abgabe::Abgabestand) -> bool {
    matches!(
        abgabe::versandstand(b),
        Some(abgabe::Versandstatus::Versendet | abgabe::Versandstatus::Eingegangen)
    )
}

/// Versendete Abgabestände in Einreichungsfolge mit maßgebendem Datum.
fn einreichungsfolge(a: &Verfahrensakte) -> Vec<(&abgabe::Abgabestand, NaiveDate)> {
    let mut v: Vec<_> = a
        .abgaben
        .iter()
        .filter(|b| versendet(b))
        .filter_map(|b| abgabe::letztes_versandereignis(b).map(|e| (b, e.datum)))
        .collect();
    v.sort_by_key(|(b, d)| (*d, b.erstellt_am, b.id));
    v
}

/// Leitet Planindex und Status aller Unterlagen der Akte ab. `dms` nennt die
/// aktuelle DMS-Version je Dokument (darf leer sein).
pub fn ableiten(a: &Verfahrensakte, dms: &HashMap<DocumentId, i64>) -> Vec<Planunterlage> {
    #[derive(Default)]
    struct Roh {
        index: Option<usize>,
        eingereicht_am: Option<NaiveDate>,
        erstmals: Option<Uuid>,
        abgaben: Vec<Uuid>,
        bescheid: Option<Uuid>,
        ausgewiesen: Option<String>,
    }
    let mut reihenfolge: Vec<DocumentId> = Vec::new();
    let mut namen: HashMap<DocumentId, String> = HashMap::new();
    let mut roh: HashMap<(DocumentId, i64), Roh> = HashMap::new();
    let mut anzahl: HashMap<DocumentId, usize> = HashMap::new();
    let merken = |d: DocumentId, reihenfolge: &mut Vec<DocumentId>| {
        if !reihenfolge.contains(&d) {
            reihenfolge.push(d);
        }
    };
    let folge = einreichungsfolge(a);
    let erste = folge.first().map(|(b, _)| b.id);
    for (b, datum) in &folge {
        for d in &b.dokumente {
            let id = d.dokument.dokument_id;
            merken(id, &mut reihenfolge);
            namen.insert(id, d.name.clone());
            let r = roh.entry((id, d.dokument.version)).or_default();
            if r.index.is_none() {
                let n = anzahl.entry(id).or_default();
                r.index = Some(*n);
                *n += 1;
                r.eingereicht_am = Some(*datum);
                r.erstmals = Some(b.id);
                r.ausgewiesen = d.planindex.clone();
            }
            r.abgaben.push(b.id);
        }
    }
    // Vorbereitete oder nicht zugestellte Stände: Version bekannt, kein Index.
    for b in a.abgaben.iter().filter(|b| !versendet(b)) {
        for d in &b.dokumente {
            let id = d.dokument.dokument_id;
            merken(id, &mut reihenfolge);
            namen.entry(id).or_insert_with(|| d.name.clone());
            roh.entry((id, d.dokument.version)).or_default();
        }
    }
    // Nur wirksame, genehmigende Bescheide; ein ersetzter Bescheid genehmigt nichts mehr.
    for b in &a.bescheide {
        let ersetzt = a.bescheide.iter().any(|n| n.ersetzt == Some(b.id));
        if ersetzt
            || !matches!(
                b.entscheidung,
                Entscheidung::Genehmigt | Entscheidung::TeilweiseGenehmigt
            )
        {
            continue;
        }
        for d in &b.genehmigte_unterlagen {
            merken(d.dokument_id, &mut reihenfolge);
            roh.entry((d.dokument_id, d.version)).or_default().bescheid = Some(b.id);
        }
    }
    // Nachfolger: nächste eingereichte Version desselben Dokuments oder eine
    // eingereichte Fassung aus einer Dokumentbeziehung.
    let nachfolger = |id: DocumentId, version: i64| -> Option<Dokumentbezug> {
        let eigener = roh.get(&(id, version)).and_then(|r| r.index).and_then(|i| {
            roh.iter()
                .filter(|((d, _), r)| *d == id && r.index == Some(i + 1))
                .map(|((d, v), _)| Dokumentbezug {
                    dokument_id: *d,
                    version: *v,
                })
                .next()
        });
        eigener.or_else(|| {
            a.dokumentbeziehungen
                .iter()
                .filter(|b| b.vorher.dokument_id == id && b.vorher.version == version)
                .map(|b| b.nachher.clone())
                .find(|n| {
                    roh.get(&(n.dokument_id, n.version))
                        .is_some_and(|r| r.index.is_some())
                })
        })
    };
    let genehmigt = |d: &Dokumentbezug| {
        roh.get(&(d.dokument_id, d.version))
            .is_some_and(|r| r.bescheid.is_some())
    };
    let mut liste = Vec::new();
    for id in reihenfolge {
        let mut versionen: Vec<i64> = roh
            .keys()
            .filter(|(d, _)| *d == id)
            .map(|(_, v)| *v)
            .collect();
        versionen.sort_unstable();
        let mut staende = Vec::new();
        let mut hinweise = Vec::new();
        for v in versionen {
            let r = &roh[&(id, v)];
            let index = r.index.map(indexbuchstaben);
            let ersetzt_durch = nachfolger(id, v);
            let status = if r.bescheid.is_some() && !ersetzt_durch.as_ref().is_some_and(genehmigt) {
                Einreichungsstatus::Genehmigt
            } else if ersetzt_durch.is_some() {
                Einreichungsstatus::Ersetzt
            } else if r.index.is_none() {
                Einreichungsstatus::Entwurf
            } else if r.erstmals == erste {
                Einreichungsstatus::Eingereicht
            } else {
                Einreichungsstatus::Nachgereicht
            };
            if let (Some(aus), Some(i)) = (&r.ausgewiesen, &index)
                && aus != i
            {
                hinweise.push(format!(
                    "Version {v}: im Anlagenverzeichnis als Index {aus} ausgewiesen, nach der Einreichungsfolge Index {i}"
                ));
            }
            staende.push(Planstand {
                version: v,
                planindex: index,
                status,
                eingereicht_am: r.eingereicht_am,
                abgaben: r.abgaben.clone(),
                bescheid: r.bescheid,
                ersetzt_durch,
            });
        }
        let letzter = staende
            .iter()
            .filter(|s| s.planindex.is_some())
            .max_by_key(|s| roh[&(id, s.version)].index);
        let hoechste_eingereicht = staende
            .iter()
            .filter(|s| s.planindex.is_some())
            .map(|s| s.version)
            .max();
        let offene_version = staende
            .iter()
            .filter(|s| s.planindex.is_none())
            .map(|s| s.version)
            .chain(dms.get(&id).copied())
            .filter(|v| hoechste_eingereicht.is_none_or(|h| *v > h))
            .filter(|v| roh.get(&(id, *v)).is_none_or(|r| r.index.is_none()))
            .max();
        liste.push(Planunterlage {
            dokument_id: id,
            name: namen.get(&id).cloned().unwrap_or_default(),
            planindex: letzter.and_then(|s| s.planindex.clone()),
            status: letzter.map_or(Einreichungsstatus::Entwurf, |s| s.status),
            offene_version,
            staende,
            hinweise,
        });
    }
    liste
}

/// Index, den die Versionen eines neuen Abgabestands erhalten, wenn er als
/// nächster Stand eingereicht wird. Bereits eingereichte Versionen behalten
/// ihren Index. Für das Anlagenverzeichnis beim Festhalten; maßgebend bleibt
/// [`ableiten`].
pub fn index_bei_einreichung(a: &Verfahrensakte, neu: &[Dokumentbezug]) -> Vec<String> {
    let bisher = ableiten(a, &HashMap::new());
    neu.iter()
        .map(|d| {
            let u = bisher.iter().find(|u| u.dokument_id == d.dokument_id);
            u.and_then(|u| {
                u.staende
                    .iter()
                    .find(|s| s.version == d.version)
                    .and_then(|s| s.planindex.clone())
            })
            .unwrap_or_else(|| {
                indexbuchstaben(u.map_or(0, |u| {
                    u.staende.iter().filter(|s| s.planindex.is_some()).count()
                }))
            })
        })
        .collect()
}

impl CoreServices {
    /// Planindex und Einreichungsstatus der Unterlagen einer Verfahrensakte.
    pub async fn verfahren_planunterlagen(
        &self,
        p: ProjectId,
        vid: Uuid,
    ) -> CoreResult<Vec<Planunterlage>> {
        let (_, a) = self.verfahrensakte(p, vid).await?;
        let mut dms = HashMap::new();
        for u in ableiten(&a, &HashMap::new()) {
            // Gelöschte oder nicht mehr lesbare Dokumente bleiben ohne DMS-Stand.
            if let Ok(d) = self.repos.documents.latest(u.dokument_id).await {
                dms.insert(u.dokument_id, d.version);
            }
        }
        Ok(ableiten(&a, &dms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use abgabe::{Abgabedokument, Abgabestand, Versandereignis, Versandstatus};

    fn akte() -> Verfahrensakte {
        serde_json::from_value(serde_json::json!({
            "bezeichnung": "Bauantrag", "art": "bauantrag", "status": "vorbereitung",
            "behoerde": {"id": Uuid::nil(), "name": "", "kennung": "", "kontakt": ""}, "aktenzeichen": "",
            "verantwortlich": null, "vorgaenger": null, "profil": null, "kommunikation": [],
            "nachforderungen": [], "anforderungen": [], "dokumentbeziehungen": [],
            "abgaben": [], "bescheide": [], "ereignisse": []
        }))
        .unwrap()
    }
    fn tag(t: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 3, t).unwrap()
    }
    fn abgabe(docs: &[(DocumentId, i64)], status: Option<(Versandstatus, u32)>) -> Abgabestand {
        let grundlagen = serde_json::from_value(serde_json::json!({
            "projekt": {"id": Uuid::nil(), "name": "P", "bundesland": "BY",
                "grundstuecksflaeche_m2": null, "version": 1,
                "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"},
            "antrag": {}, "bauherren": [], "buero": {}
        }))
        .unwrap();
        Abgabestand {
            id: Uuid::new_v4(),
            auftrag_hash: String::new(),
            titel: "Abgabe".into(),
            erstellt_am: Utc::now(),
            erstellt_von: Uuid::nil(),
            grundlagen,
            dokumente: docs
                .iter()
                .map(|(d, v)| Abgabedokument {
                    kategorie: None,
                    planindex: None,
                    dokument: Dokumentbezug {
                        dokument_id: *d,
                        version: *v,
                    },
                    name: format!("{}.pdf", d.0),
                    sha256: String::new(),
                    bytes: 0,
                })
                .collect(),
            anforderungen: vec![],
            hinweise: vec![],
            artefakte: vec![],
            versand: status
                .map(|(s, t)| Versandereignis {
                    id: Uuid::new_v4(),
                    status: s,
                    datum: tag(t),
                    weg: "Portal".into(),
                    referenz: String::new(),
                    notiz: String::new(),
                    beleg: None,
                    berichtigt: None,
                })
                .into_iter()
                .collect(),
            versandstatus: None,
        }
    }
    fn bescheid(docs: &[(DocumentId, i64)]) -> Bescheid {
        Bescheid {
            id: Uuid::new_v4(),
            entscheidung: Entscheidung::Genehmigt,
            datum: tag(28),
            zugestellt: None,
            original: Dokumentbezug {
                dokument_id: DocumentId::new(),
                version: 1,
            },
            genehmigte_unterlagen: docs
                .iter()
                .map(|(d, v)| Dokumentbezug {
                    dokument_id: *d,
                    version: *v,
                })
                .collect(),
            auflagen: vec![],
            ersetzt: None,
            verlaengerung: None,
        }
    }
    fn stand(l: &[Planunterlage], d: DocumentId, v: i64) -> &Planstand {
        l.iter()
            .find(|u| u.dokument_id == d)
            .unwrap()
            .staende
            .iter()
            .find(|s| s.version == v)
            .unwrap()
    }

    #[test]
    fn indexbuchstaben_wie_spalten() {
        assert_eq!(indexbuchstaben(0), "A");
        assert_eq!(indexbuchstaben(25), "Z");
        assert_eq!(indexbuchstaben(26), "AA");
        assert_eq!(indexbuchstaben(27), "AB");
        assert_eq!(indexbuchstaben(701), "ZZ");
        assert_eq!(indexbuchstaben(702), "AAA");
    }

    #[test]
    fn index_nur_aus_versendeten_abgaben_und_status_aus_folge_und_bescheid() {
        let lage = DocumentId::new();
        let ansicht = DocumentId::new();
        let mut a = akte();
        // Vorbereitet, nicht versendet: kein Index.
        a.abgaben.push(abgabe(&[(lage, 1)], None));
        let l = ableiten(&a, &HashMap::new());
        assert_eq!(l[0].planindex, None);
        assert_eq!(l[0].status, Einreichungsstatus::Entwurf);
        // Erstantrag (versendet), Nachreichung (eingegangen), Zurückweisung zählt nicht.
        a.abgaben.push(abgabe(
            &[(lage, 2), (ansicht, 1)],
            Some((Versandstatus::Versendet, 2)),
        ));
        a.abgaben.push(abgabe(
            &[(lage, 4)],
            Some((Versandstatus::Zurueckgewiesen, 9)),
        ));
        a.abgaben
            .push(abgabe(&[(lage, 3)], Some((Versandstatus::Eingegangen, 10))));
        let dms = HashMap::from([(lage, 5), (ansicht, 1)]);
        let l = ableiten(&a, &dms);
        assert_eq!(stand(&l, lage, 2).planindex.as_deref(), Some("A"));
        assert_eq!(stand(&l, lage, 2).status, Einreichungsstatus::Ersetzt);
        assert_eq!(
            stand(&l, lage, 2).ersetzt_durch,
            Some(Dokumentbezug {
                dokument_id: lage,
                version: 3
            })
        );
        assert_eq!(stand(&l, lage, 3).planindex.as_deref(), Some("B"));
        assert_eq!(stand(&l, lage, 3).status, Einreichungsstatus::Nachgereicht);
        assert_eq!(stand(&l, lage, 3).eingereicht_am, Some(tag(10)));
        assert_eq!(stand(&l, lage, 4).planindex, None);
        assert_eq!(stand(&l, lage, 1).status, Einreichungsstatus::Entwurf);
        assert_eq!(
            stand(&l, ansicht, 1).status,
            Einreichungsstatus::Eingereicht
        );
        let u = l.iter().find(|u| u.dokument_id == lage).unwrap();
        assert_eq!(
            (u.planindex.as_deref(), u.status, u.offene_version),
            (Some("B"), Einreichungsstatus::Nachgereicht, Some(5))
        );
        // Genehmigung von Index B; Index A bleibt ersetzt.
        a.bescheide.push(bescheid(&[(lage, 3), (ansicht, 1)]));
        let l = ableiten(&a, &dms);
        assert_eq!(stand(&l, lage, 3).status, Einreichungsstatus::Genehmigt);
        assert_eq!(stand(&l, lage, 2).status, Einreichungsstatus::Ersetzt);
        assert_eq!(stand(&l, ansicht, 1).status, Einreichungsstatus::Genehmigt);
        // Neuer Index C nach Genehmigung (Tektur): B bleibt genehmigte Fassung.
        a.abgaben
            .push(abgabe(&[(lage, 5)], Some((Versandstatus::Versendet, 30))));
        let l = ableiten(&a, &dms);
        assert_eq!(stand(&l, lage, 3).status, Einreichungsstatus::Genehmigt);
        assert_eq!(stand(&l, lage, 5).planindex.as_deref(), Some("C"));
        assert_eq!(stand(&l, lage, 5).status, Einreichungsstatus::Nachgereicht);
        assert_eq!(
            l.iter()
                .find(|u| u.dokument_id == lage)
                .unwrap()
                .offene_version,
            None
        );
        // Ein ersetzender Bescheid genehmigt C: B ist nun ersetzt.
        let mut neu = bescheid(&[(lage, 5)]);
        neu.ersetzt = Some(a.bescheide[0].id);
        a.bescheide.push(neu);
        let l = ableiten(&a, &dms);
        assert_eq!(stand(&l, lage, 3).status, Einreichungsstatus::Ersetzt);
        assert_eq!(stand(&l, lage, 5).status, Einreichungsstatus::Genehmigt);
        // Der ersetzte Bescheid genehmigt die Ansicht nicht mehr.
        assert_eq!(
            stand(&l, ansicht, 1).status,
            Einreichungsstatus::Eingereicht
        );
    }

    #[test]
    fn dokumentbeziehung_ersetzt_ueber_dokumentgrenzen_und_index_bei_einreichung() {
        let alt = DocumentId::new();
        let neu = DocumentId::new();
        let mut a = akte();
        a.abgaben
            .push(abgabe(&[(alt, 1)], Some((Versandstatus::Versendet, 1))));
        a.dokumentbeziehungen.push(Dokumentbeziehung {
            id: Uuid::new_v4(),
            vorher: Dokumentbezug {
                dokument_id: alt,
                version: 1,
            },
            nachher: Dokumentbezug {
                dokument_id: neu,
                version: 1,
            },
            grund: "Neuer Lageplan".into(),
            bezug: None,
        });
        // Nachfolger noch nicht eingereicht: alter Plan bleibt eingereicht.
        let l = ableiten(&a, &HashMap::new());
        assert_eq!(stand(&l, alt, 1).status, Einreichungsstatus::Eingereicht);
        assert_eq!(
            index_bei_einreichung(
                &a,
                &[
                    Dokumentbezug {
                        dokument_id: alt,
                        version: 1
                    },
                    Dokumentbezug {
                        dokument_id: alt,
                        version: 2
                    },
                    Dokumentbezug {
                        dokument_id: neu,
                        version: 1
                    }
                ]
            ),
            ["A", "B", "A"]
        );
        let mut b = abgabe(&[(neu, 1)], Some((Versandstatus::Versendet, 5)));
        b.dokumente[0].planindex = Some("B".into());
        a.abgaben.push(b);
        let l = ableiten(&a, &HashMap::new());
        assert_eq!(stand(&l, alt, 1).status, Einreichungsstatus::Ersetzt);
        assert_eq!(stand(&l, neu, 1).status, Einreichungsstatus::Nachgereicht);
        // Abweichend ausgewiesener Index wird gemeldet, nicht verdeckt.
        let u = l.iter().find(|u| u.dokument_id == neu).unwrap();
        assert!(u.hinweise[0].contains("Index B"));
    }

    #[test]
    fn aeltere_abgabedokumente_ohne_planindex_bleiben_lesbar() {
        let d: Abgabedokument = serde_json::from_value(serde_json::json!({
            "dokument": {"dokument_id": Uuid::nil(), "version": 1},
            "name": "Lageplan.pdf", "sha256": "", "bytes": 0
        }))
        .unwrap();
        assert_eq!(d.planindex, None);
        assert!(!serde_json::to_string(&d).unwrap().contains("planindex"));
    }
}
