//! Projektbezogenes Lesemodell: keine eigene Berechnung, Freigabe oder Einreichungsentscheidung.
use crate::{
    CoreError, CoreResult, CoreServices,
    benutzer::Benutzer,
    domain::ProjectId,
    plattform::{
        ObjektArt,
        nachweise::NachweisEingabe,
        objekte::{Aufgabe, AufgabenStatus, Unterlage, UnterlagenStatus},
    },
    verfahren::{
        Erforderlichkeit, Punktstatus, Verfahrensakte, Verfahrensstatus,
        abgabe::{Versandstatus, versandstand},
    },
    zugriff::ProjektRecht,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Dringlichkeit {
    Ueberfaellig,
    Heute,
    Handeln,
    Pruefen,
    Warten,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Themenbereich {
    Grundlagen,
    Aufgaben,
    Unterlagen,
    Nachweise,
    Verfahren,
    Einreichungen,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct UebersichtsZiel {
    pub bereich: String,
    pub objekt: Option<Uuid>,
    pub register: Option<String>,
    pub eintrag: Option<Uuid>,
}
impl UebersichtsZiel {
    fn neu(b: &str) -> Self {
        Self {
            bereich: b.into(),
            objekt: None,
            register: None,
            eintrag: None,
        }
    }
    fn objekt(mut self, id: Uuid) -> Self {
        self.objekt = Some(id);
        self
    }
    fn register(mut self, s: &str) -> Self {
        self.register = Some(s.into());
        self
    }
    fn eintrag(mut self, id: Uuid) -> Self {
        self.eintrag = Some(id);
        self
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Arbeitspunkt {
    pub id: String,
    pub thema: Themenbereich,
    pub dringlichkeit: Dringlichkeit,
    pub titel: String,
    pub hinweis: String,
    pub faellig: Option<NaiveDate>,
    pub verantwortlich: Option<Uuid>,
    pub ziel: UebersichtsZiel,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
pub struct UebersichtsZahlen {
    pub offene_aufgaben: usize,
    pub fehlende_unterlagen: usize,
    pub nachweise_pruefen: usize,
    pub offene_nachforderungen: usize,
    pub offene_auflagen: usize,
    pub ueberfaellig: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct VerfahrensUeberblick {
    pub id: Uuid,
    pub bezeichnung: String,
    pub behoerde: String,
    pub aktenzeichen: String,
    pub status: String,
    pub offene_punkte: usize,
    pub offene_auflagen: usize,
    pub abgaben: usize,
    pub versendet: usize,
    pub bescheide: usize,
    pub entscheidung: Option<crate::verfahren::Entscheidung>,
    pub entscheidungsdatum: Option<NaiveDate>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct AbgabeUeberblick {
    pub id: Uuid,
    pub titel: String,
    pub empfaenger: String,
    pub quelle: String,
    pub status: String,
    pub datum: Option<NaiveDate>,
    pub dokumente: usize,
    pub ziel: UebersichtsZiel,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct EmpfaengerDokument {
    pub dokument_id: Uuid,
    pub dateiname: String,
    pub eingereichte_version: i64,
    pub aktuelle_version: Option<i64>,
    pub eingereicht_am: NaiveDate,
    pub status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct EmpfaengerUeberblick {
    pub empfaenger: String,
    pub dokumente: Vec<EmpfaengerDokument>,
    pub ziel: UebersichtsZiel,
}
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
pub struct Projektuebersicht {
    pub projekt_id: ProjectId,
    pub projekt_version: i64,
    pub ermittelt_am: DateTime<Utc>,
    pub stichtag: NaiveDate,
    pub zahlen: UebersichtsZahlen,
    pub punkte: Vec<Arbeitspunkt>,
    pub verfahren: Vec<VerfahrensUeberblick>,
    pub abgaben: Vec<AbgabeUeberblick>,
    pub empfaenger: Vec<EmpfaengerUeberblick>,
    pub hinweise: Vec<String>,
}
fn decode<T: serde::de::DeserializeOwned>(v: serde_json::Value) -> CoreResult<T> {
    serde_json::from_value(v).map_err(|e| CoreError::InvalidInput(format!("Projektübersicht: {e}")))
}
fn id(s: &str) -> CoreResult<Uuid> {
    Uuid::parse_str(s).map_err(|e| CoreError::InvalidInput(e.to_string()))
}
fn frist(d: Option<NaiveDate>, heute: NaiveDate, sonst: Dringlichkeit) -> Dringlichkeit {
    match d {
        Some(d) if d < heute => Dringlichkeit::Ueberfaellig,
        Some(d) if d == heute => Dringlichkeit::Heute,
        _ => sonst,
    }
}
impl Projektuebersicht {
    fn punkt(
        &mut self,
        key: impl Into<String>,
        thema: Themenbereich,
        d: Dringlichkeit,
        titel: impl Into<String>,
        hinweis: impl Into<String>,
        ziel: UebersichtsZiel,
    ) {
        self.punkte.push(Arbeitspunkt {
            id: key.into(),
            thema,
            dringlichkeit: d,
            titel: titel.into(),
            hinweis: hinweis.into(),
            faellig: None,
            verantwortlich: None,
            ziel,
        });
    }
    fn termin(&mut self, datum: Option<NaiveDate>, person: Option<Uuid>) {
        if let Some(p) = self.punkte.last_mut() {
            p.faellig = datum;
            p.verantwortlich = person;
        }
    }
}
impl CoreServices {
    pub async fn projektuebersicht(
        &self,
        p: ProjectId,
        wer: &Benutzer,
    ) -> CoreResult<Projektuebersicht> {
        self.projektuebersicht_am(p, crate::kalender::heute(), wer)
            .await
    }
    /// Interner Stichtag für deterministische Vertrags-/Integrationsprüfungen.
    pub async fn projektuebersicht_am(
        &self,
        p: ProjectId,
        heute: NaiveDate,
        wer: &Benutzer,
    ) -> CoreResult<Projektuebersicht> {
        self.projekt_zugriff(p, wer, ProjektRecht::Lesen).await?;
        let projekt = self.get_project(p).await?;
        let docs = self.list_documents(p).await?;
        let mut aktuell = BTreeMap::new();
        for d in docs {
            let e = aktuell.entry(d.document_id.0).or_insert_with(|| d.clone());
            if d.version > e.version {
                *e = d;
            }
        }
        let mut s=Projektuebersicht{projekt_id:p,projekt_version:projekt.version,ermittelt_am:Utc::now(),stichtag:heute,zahlen:Default::default(),punkte:vec![],verfahren:vec![],abgaben:vec![],empfaenger:vec![],hinweise:vec!["Die Übersicht bewertet gespeicherte Angaben und erfasste Anforderungen. Nicht erfasste Pflichten und ungespeicherte Entwürfe sind nicht geprüft. Export bedeutet keine Einreichung; eine Bürofreigabe ersetzt keine behördliche Entscheidung.".into()]};
        use Dringlichkeit::*;
        use Themenbereich::*;
        if projekt.planungsrecht.rechtsgrundlage.is_none() {
            s.punkt(
                "recht",
                Grundlagen,
                Handeln,
                "Rechtsgrundlage festlegen",
                "Planungsrechtliche Einordnung und anwendbare Grundlagen vervollständigen.",
                UebersichtsZiel::neu("rechtsgrundlagen"),
            );
        }
        if projekt.planungsrecht.rechtsgrundlage == Some(crate::domain::Rechtsgrundlage::Par30)
            && projekt.planungsrecht.bau_nvo.is_none()
        {
            s.punkt("baunvo",Grundlagen,Handeln,"BauNVO-Fassung des Bebauungsplans festlegen","Ohne zeitlich passende Grundlage können Nutzungsnachweise nicht zuverlässig eingeordnet werden.",UebersichtsZiel::neu("rechtsgrundlagen"));
        }
        if self.bauherren(p).await?.is_empty() {
            s.punkt(
                "bauherr",
                Grundlagen,
                Handeln,
                "Bauherr zuordnen",
                "Im Stammblatt die Beteiligten des Vorhabens ergänzen.",
                UebersichtsZiel::neu("uebersicht"),
            );
        }
        match crate::grundstueckskontext::ermitteln(&projekt) {
            Ok(g) if !g.is_empty() && g.iter().all(|g| g.bezugsflaeche_m2.is_some()) => {}
            Ok(_) => s.punkt(
                "grundstueck",
                Grundlagen,
                Handeln,
                "Baugrundstücke und Bezugsflächen ermitteln",
                "Grundlage für grundstücksbezogene Berechnungen fehlt oder ist unvollständig.",
                UebersichtsZiel::neu("grundstueck"),
            ),
            Err(e) => s.punkt(
                "grundstueck",
                Grundlagen,
                Handeln,
                "Baugrundstücksermittlung aktualisieren",
                e.to_string(),
                UebersichtsZiel::neu("grundstueck"),
            ),
        }
        for f in self.fachobjekte(p, ObjektArt::Aufgaben).await? {
            let a: Aufgabe = decode(f.daten)?;
            if a.status == AufgabenStatus::Erledigt {
                continue;
            }
            let aid = id(&f.objekt)?;
            s.zahlen.offene_aufgaben += 1;
            s.punkt(
                format!("aufgabe:{aid}"),
                Aufgaben,
                frist(a.faellig, heute, Handeln),
                a.titel,
                a.beschreibung,
                UebersichtsZiel::neu("aufgaben").objekt(aid),
            );
            s.termin(a.faellig, a.verantwortlich.map(|p| p.0));
        }
        let doc_aktuell = |d: &crate::plattform::objekte::Dokumentbezug| {
            aktuell
                .get(&d.dokument_id.0)
                .is_some_and(|v| v.version == d.version)
        };
        let mut unterlagen = 0;
        for f in self.fachobjekte(p, ObjektArt::Unterlagen).await? {
            let u: Unterlage = decode(f.daten)?;
            if !u.erforderlich {
                continue;
            }
            unterlagen += 1;
            let uid = id(&f.objekt)?;
            let grund = match &u.dokument {
                None => Some((Handeln, "Erforderliche Unterlage fehlt")),
                Some(d) if !doc_aktuell(d) => Some((
                    Pruefen,
                    "Zugeordnete Dokumentversion ist nicht aktuell oder fehlt",
                )),
                Some(_) if u.status != UnterlagenStatus::Freigegeben => {
                    Some((Pruefen, "Dokument liegt vor; fachliche Freigabe fehlt"))
                }
                _ => None,
            };
            if let Some((d, grund)) = grund {
                s.zahlen.fehlende_unterlagen += 1;
                s.punkt(
                    format!("unterlage:{uid}"),
                    Unterlagen,
                    d,
                    u.bezeichnung,
                    grund,
                    UebersichtsZiel::neu("unterlagen").objekt(uid),
                );
            }
        }
        // Historische Nachweise bleiben im Archiv. Für Handlungsbedarf zählt nur der neueste Stand je Produzent/Bezugsumfang.
        let mut neueste = BTreeMap::new();
        for n in self.nachweise(p).await? {
            let key = nachweisschluessel(&n.stand)?;
            let replace = neueste.get(&key).is_none_or(
                |alt: &crate::plattform::nachweise::NachweisAnsicht| {
                    (n.stand.erstellt_am, n.stand.id) > (alt.stand.erstellt_am, alt.stand.id)
                },
            );
            if replace {
                neueste.insert(key, n);
            }
        }
        for n in neueste.values() {
            let grund = if !n.aktuell {
                Some("Grundlagen oder Eingaben wurden geändert; neuen Nachweis erstellen")
            } else if !n.stand.vollstaendig {
                Some("Berechnung enthält offene Prüfpunkte")
            } else if n.freigabe.is_none() {
                Some("Berechnung ist vollständig; fachliche Bürofreigabe fehlt")
            } else {
                None
            };
            if let Some(grund) = grund {
                s.zahlen.nachweise_pruefen += 1;
                s.punkt(
                    format!("nachweis:{}", n.stand.id),
                    Nachweise,
                    Pruefen,
                    n.stand.bezeichnung.clone(),
                    grund,
                    UebersichtsZiel::neu("nachweise").eintrag(n.stand.id),
                );
            }
        }
        let akten = self.fachobjekte(p, ObjektArt::Verfahren).await?;
        if unterlagen == 0 && akten.is_empty() {
            s.punkt("anforderungen",Unterlagen,Handeln,"Anforderungen an den Bauantrag erfassen","Eine Verfahrensakte mit Vorhabenprofil anlegen oder die Bürocheckliste im Unterlagenregister verwenden. Ohne Anforderungen lässt sich der Unterlagenstand nicht bewerten.",UebersichtsZiel::neu("verfahren"));
        }
        // Pro Behörde getrennt: (Empfängeridentität, Dokument) → jüngster tatsächlich versendeter Originalstand.
        let mut empfangene: BTreeMap<(Uuid, Uuid), Empfang> = BTreeMap::new();
        for f in akten {
            let a: Verfahrensakte = decode(f.daten)?;
            let vid = id(&f.objekt)?;
            let zurueck = a.status == Verfahrensstatus::Zurueckgenommen;
            if !zurueck {
                for h in crate::portale::arbeitsbedarf(&a) {
                    let mut portalziel = UebersichtsZiel::neu("portale")
                        .objekt(vid)
                        .register(&h.vorgang.to_string());
                    if let Some(abgabe) = h.abgabe {
                        portalziel = portalziel.eintrag(abgabe);
                    }
                    s.punkt(
                        format!(
                            "portal:{vid}:{}:{}",
                            h.vorgang,
                            h.abgabe.map(|id| id.to_string()).unwrap_or_default()
                        ),
                        Einreichungen,
                        Pruefen,
                        h.titel,
                        h.hinweis,
                        portalziel,
                    );
                }
            }
            let mut offen = 0;
            let mut auflagen = 0;
            let ziel = |register: &str| {
                UebersichtsZiel::neu("verfahren")
                    .objekt(vid)
                    .register(register)
            };
            if !zurueck {
                if a.anforderungen.is_empty() {
                    s.punkt(
                        format!("anforderungen:{vid}"),
                        Verfahren,
                        Handeln,
                        format!("{}: Anforderungen erfassen", a.bezeichnung),
                        "Erforderliche Unterlagen und Nachweise sind noch nicht festgelegt.",
                        ziel("anforderungen"),
                    );
                }
                if a.profil.as_ref().is_some_and(|p| p.bestaetigt_am.is_none()) {
                    s.punkt(
                        format!("profil:{vid}"),
                        Verfahren,
                        Pruefen,
                        format!("{}: Vorhabenprofil prüfen", a.bezeichnung),
                        "Die Anwendbarkeit der Profilgrundlagen ist noch nicht bestätigt.",
                        ziel("profil"),
                    );
                }
                for r in &a.anforderungen {
                    let grund = match r.erforderlich {
                        Erforderlichkeit::Ungeklaert => {
                            Some((Handeln, "Erforderlichkeit ist noch ungeklärt"))
                        }
                        Erforderlichkeit::Erforderlich if r.dokumente.is_empty() => Some((
                            Handeln,
                            "Erforderliche Unterlage / Nachweis noch nicht zugeordnet",
                        )),
                        _ if !r.freigegeben => {
                            Some((Pruefen, "Fachliche Bewertung noch nicht freigegeben"))
                        }
                        Erforderlichkeit::Erforderlich if !r.dokumente.iter().all(&doc_aktuell) => {
                            Some((
                                Pruefen,
                                "Zugeordnete Unterlagen wurden geändert oder fehlen",
                            ))
                        }
                        _ => None,
                    };
                    if let Some((d, grund)) = grund {
                        s.zahlen.fehlende_unterlagen += 1;
                        s.punkt(
                            format!("anforderung:{vid}:{}", r.id),
                            Unterlagen,
                            d,
                            r.titel.clone(),
                            format!("{} · {grund}", a.bezeichnung),
                            ziel("anforderungen").eintrag(r.id),
                        );
                        s.termin(None, r.verantwortlich);
                    }
                }
                // Mit der Entscheidung enden Nachforderungen; Auflagen bleiben.
                {
                    for r in crate::verfahren::termine::offene_forderungspunkte(&a) {
                        offen += 1;
                        let warten = r.status == Punktstatus::Eingereicht;
                        let datum = if warten {
                            None
                        } else {
                            r.frist.as_ref().map(|f| f.datum)
                        };
                        s.punkt(
                            format!("nachforderung:{vid}:{}", r.id),
                            Verfahren,
                            frist(datum, heute, if warten { Warten } else { Handeln }),
                            r.titel.clone(),
                            format!(
                                "{} · {}",
                                a.bezeichnung,
                                if warten {
                                    "Antwort eingereicht; behördliche Bestätigung steht aus"
                                } else {
                                    "Nachforderung beantworten und Antwortunterlagen zuordnen"
                                }
                            ),
                            ziel("nachforderungen").eintrag(r.id),
                        );
                        s.termin(datum, r.verantwortlich);
                    }
                }
                for r in a.schritte.iter().filter(|_| !a.status.abgeschlossen()) {
                    let anhoerung = r.art == crate::verfahren::Schrittart::Anhoerung;
                    if (anhoerung && r.stellungnahme.is_some())
                        || (!anhoerung && r.folgeschreiben.is_some())
                    {
                        continue;
                    }
                    let datum = r.frist.as_ref().map(|f| f.datum);
                    // Nach ihrem Ende ist die Zurückstellung nicht überfällig:
                    // Die Entscheidung der Behörde steht aus.
                    let abgelaufen = !anhoerung && datum.is_some_and(|d| d < heute);
                    s.punkt(
                        format!("schritt:{vid}:{}", r.id),
                        Verfahren,
                        if abgelaufen {
                            Warten
                        } else {
                            frist(datum, heute, if anhoerung { Handeln } else { Warten })
                        },
                        if anhoerung {
                            "Stellungnahme zur Anhörung abgeben"
                        } else if abgelaufen {
                            "Zurückstellung beendet – Entscheidung erwarten"
                        } else {
                            "Zurückstellung läuft"
                        },
                        format!(
                            "{} · {}",
                            a.bezeichnung,
                            if anhoerung {
                                "Die Behörde hört vor ihrer Entscheidung an; Stellungnahme als ausgehende Kommunikation zuordnen"
                            } else if abgelaufen {
                                "Die Zurückstellung ist abgelaufen; die Behörde hat nun zu entscheiden"
                            } else {
                                "Die Entscheidung ist bis zum Fristende zurückgestellt"
                            }
                        ),
                        ziel("schritte").eintrag(r.id),
                    );
                    s.termin(if abgelaufen { None } else { datum }, r.verantwortlich);
                }
                // Nur Auflagen und Bedingungen sind Pflichten; Befristung und
                // Vorbehalte erscheinen als Ablauftermin (siehe unten).
                {
                    for (_, r) in crate::verfahren::termine::offene_pflichten(&a) {
                        auflagen += 1;
                        let datum = r.frist.as_ref().map(|f| f.datum);
                        s.punkt(
                            format!("auflage:{vid}:{}", r.id),
                            Verfahren,
                            frist(datum, heute, Handeln),
                            r.titel.clone(),
                            format!(
                                "{} · {}: {}",
                                a.bezeichnung,
                                r.art.bezeichnung(),
                                r.ausloeser
                            ),
                            ziel("bescheide").eintrag(r.id),
                        );
                        s.termin(datum, r.verantwortlich);
                    }
                }
            }
            // Ablauf von Befristungen und berechnete Bescheidfristen ab ihrem Vorlauf.
            for t in crate::verfahren::termine::termine(&a, vid, projekt.bundesland) {
                use crate::verfahren::termine::Terminart as T;
                if !matches!(
                    t.art,
                    T::Ablauf | T::Rechtsbehelfsfrist | T::Geltungsdauer | T::Fertigstellung
                ) || t.erinnerung(heute).is_none()
                {
                    continue;
                }
                let sonst = if matches!(t.art, T::Geltungsdauer | T::Fertigstellung) {
                    Handeln
                } else {
                    Pruefen
                };
                s.punkt(
                    format!("termin:{vid}:{}:{:?}", t.objekt, t.art),
                    Verfahren,
                    if t.endet_mit_ablauf && t.datum > heute {
                        sonst
                    } else {
                        frist(Some(t.datum), heute, sonst)
                    },
                    t.titel.clone(),
                    format!("{} · {}", a.bezeichnung, t.hinweis),
                    ziel("bescheide").eintrag(t.objekt),
                );
                s.termin(Some(t.datum), t.verantwortlich);
            }
            let bescheide = a
                .bescheide
                .iter()
                .filter(|b| !a.bescheide.iter().any(|n| n.ersetzt == Some(b.id)))
                .count();
            let letzter_bescheid = a
                .bescheide
                .iter()
                .filter(|b| !a.bescheide.iter().any(|n| n.ersetzt == Some(b.id)))
                .max_by_key(|b| (b.datum, b.id));
            let versendet = a
                .abgaben
                .iter()
                .filter(|b| {
                    matches!(
                        versandstand(b),
                        Some(Versandstatus::Versendet | Versandstatus::Eingegangen)
                    )
                })
                .count();
            s.zahlen.offene_nachforderungen += offen;
            s.zahlen.offene_auflagen += auflagen;
            s.verfahren.push(VerfahrensUeberblick {
                id: vid,
                bezeichnung: a.bezeichnung.clone(),
                behoerde: a.behoerde.name.clone(),
                aktenzeichen: a.aktenzeichen.clone(),
                status: a.status.bezeichnung().into(),
                offene_punkte: offen,
                offene_auflagen: auflagen,
                abgaben: a.abgaben.len(),
                versendet,
                bescheide,
                entscheidung: letzter_bescheid.map(|b| b.entscheidung.clone()),
                entscheidungsdatum: letzter_bescheid.map(|b| b.datum),
            });
            for b in &a.abgaben {
                let stand = versandstand(b);
                let datum = crate::verfahren::abgabe::letztes_versandereignis(b).map(|e| e.datum);
                let label = match stand {
                    None => "Zusammengestellt · Versand unbestätigt",
                    Some(Versandstatus::Versendet) => "Versendet",
                    Some(Versandstatus::Eingegangen) => "Eingang bestätigt",
                    Some(Versandstatus::Zurueckgewiesen) => "Zurückgewiesen",
                    Some(Versandstatus::Ungeklaert) => "Übergabe / Dateiabgleich ungeklärt",
                };
                s.abgaben.push(AbgabeUeberblick {
                    id: b.id,
                    titel: b.titel.clone(),
                    empfaenger: a.behoerde.name.clone(),
                    quelle: a.bezeichnung.clone(),
                    status: label.into(),
                    datum,
                    dokumente: b.dokumente.len(),
                    ziel: ziel("abgaben").eintrag(b.id),
                });
                if !zurueck
                    && matches!(
                        stand,
                        None | Some(Versandstatus::Zurueckgewiesen | Versandstatus::Ungeklaert)
                    )
                {
                    s.punkt(
                        format!("versand:{vid}:{}", b.id),
                        Einreichungen,
                        if stand.is_none() { Pruefen } else { Handeln },
                        b.titel.clone(),
                        format!(
                            "{} · {label}. Export/Erstellung belegt keinen erfolgten Versand.",
                            a.behoerde.name
                        ),
                        ziel("abgaben").eintrag(b.id),
                    );
                }
                if matches!(
                    stand,
                    Some(Versandstatus::Versendet | Versandstatus::Eingegangen)
                ) {
                    for d in &b.dokumente {
                        let date = datum
                            .ok_or_else(|| CoreError::InvalidInput("Versanddatum fehlt".into()))?;
                        let key = (a.behoerde.id, d.dokument.dokument_id.0);
                        let neue = Empfang {
                            datum: date,
                            erstellt_am: b.erstellt_am,
                            abgabe: b.id,
                            version: d.dokument.version,
                            name: d.name.clone(),
                            empfaenger: a.behoerde.name.clone(),
                            verfahren: vid,
                        };
                        if empfangene.get(&key).is_none_or(|alt| {
                            (neue.datum, neue.erstellt_am, neue.abgabe)
                                > (alt.datum, alt.erstellt_am, alt.abgabe)
                        }) {
                            empfangene.insert(key, neue);
                        }
                    }
                }
            }
        }
        let mut gruppen: BTreeMap<Uuid, EmpfaengerUeberblick> = BTreeMap::new();
        for (
            (beh, did),
            Empfang {
                datum,
                version,
                name,
                empfaenger,
                verfahren: vid,
                ..
            },
        ) in empfangene
        {
            let aktuell = aktuell.get(&did).map(|d| d.version);
            let status = if aktuell.is_none() {
                "Dokument im Projekt nicht verfügbar"
            } else if aktuell.is_some_and(|v| v > version) {
                "Neuere Projektversion vorhanden"
            } else if aktuell.is_some_and(|v| v < version) {
                "Historischer Vergleich prüfen"
            } else {
                "Gleiche Version"
            };
            let g = gruppen.entry(beh).or_insert_with(|| EmpfaengerUeberblick {
                empfaenger,
                ziel: UebersichtsZiel::neu("verfahren")
                    .objekt(vid)
                    .register("abgaben"),
                dokumente: vec![],
            });
            g.dokumente.push(EmpfaengerDokument {
                dokument_id: did,
                dateiname: name.clone(),
                eingereichte_version: version,
                aktuelle_version: aktuell,
                eingereicht_am: datum,
                status: status.into(),
            });
            if aktuell.is_none_or(|v| v > version) {
                s.punkt(format!("nachreichung:{beh}:{did}"),Einreichungen,Pruefen,name,"Im Projekt liegt eine neuere Version als im dokumentierten Behördenstand vor. Erneute Einreichung prüfen.",g.ziel.clone());
            }
        }
        s.empfaenger.extend(gruppen.into_values());
        let pakete = self.einreichungen(p).await?;
        let mut empfaenger = std::collections::BTreeSet::new();
        for p in &pakete {
            if p.einreichung.is_some() {
                empfaenger.insert(p.stand.empfaenger.trim().to_lowercase());
            }
        }
        for emp in empfaenger {
            let mut dokumente = vec![];
            for d in crate::einreichung::vergleichen(
                p,
                &emp,
                aktuell.values().cloned().collect(),
                &pakete,
            ) {
                if let (Some(alt), Some(datum)) = (d.zuletzt_eingereicht, d.eingereicht_am) {
                    let label = match d.status {
                        crate::einreichung::VergleichsStatus::Geaendert => "Inhalt geändert",
                        crate::einreichung::VergleichsStatus::GleicherInhalt => {
                            "Gleicher Inhalt · andere Version"
                        }
                        crate::einreichung::VergleichsStatus::Aelter => "Ältere Projektversion",
                        _ => "Gleiche Version",
                    };
                    if matches!(
                        d.status,
                        crate::einreichung::VergleichsStatus::Geaendert
                            | crate::einreichung::VergleichsStatus::Aelter
                    ) {
                        s.punkt(format!("paketversion:{emp}:{}",d.aktuell.document_id.0),Einreichungen,Pruefen,d.aktuell.file_name.clone(),format!("{emp}: {label} gegenüber der bestätigten Einreichung. Nachreichung prüfen."),UebersichtsZiel::neu("einreichungen"));
                    }
                    dokumente.push(EmpfaengerDokument {
                        dokument_id: d.aktuell.document_id.0,
                        dateiname: d.aktuell.file_name,
                        eingereichte_version: alt.version,
                        aktuelle_version: Some(d.aktuell.version),
                        eingereicht_am: datum,
                        status: label.into(),
                    });
                }
            }
            let name = pakete
                .iter()
                .find(|p| p.stand.empfaenger.trim().eq_ignore_ascii_case(&emp))
                .map(|p| p.stand.empfaenger.clone())
                .unwrap_or(emp);
            s.empfaenger.push(EmpfaengerUeberblick {
                empfaenger: format!("{name} · bestätigte Pakete"),
                dokumente,
                ziel: UebersichtsZiel::neu("einreichungen"),
            });
        }
        for e in pakete {
            s.abgaben.push(AbgabeUeberblick {
                id: e.stand.id,
                titel: e.stand.titel.clone(),
                empfaenger: e.stand.empfaenger.clone(),
                quelle: "Einreichungspaket".into(),
                status: if e.einreichung.is_some() {
                    "Einreichung bestätigt"
                } else {
                    "Nur exportiert · Einreichung unbestätigt"
                }
                .into(),
                datum: e.einreichung.as_ref().map(|b| b.datum),
                dokumente: e.stand.dokumente.len(),
                ziel: UebersichtsZiel::neu("einreichungen").eintrag(e.stand.id),
            });
            if e.einreichung.is_none() {
                s.punkt(format!("paket:{}",e.stand.id),Einreichungen,Pruefen,e.stand.titel,"Das Paket wurde exportiert; eine tatsächliche Einreichung wurde noch nicht bestätigt.",UebersichtsZiel::neu("einreichungen").eintrag(e.stand.id));
            }
        }
        if s.verfahren.is_empty() {
            s.hinweise.push("Noch keine Verfahrensakte erfasst. Behördliche Nachforderungen, Bescheide und Versandstände können deshalb nicht bewertet werden.".into());
        }
        s.punkte.sort_by(|a, b| {
            a.dringlichkeit
                .cmp(&b.dringlichkeit)
                .then(
                    a.faellig
                        .unwrap_or(NaiveDate::MAX)
                        .cmp(&b.faellig.unwrap_or(NaiveDate::MAX)),
                )
                .then(a.titel.cmp(&b.titel))
                .then(a.id.cmp(&b.id))
        });
        s.zahlen.ueberfaellig = s
            .punkte
            .iter()
            .filter(|p| p.dringlichkeit == Ueberfaellig)
            .count();
        s.abgaben
            .sort_by(|a, b| b.datum.cmp(&a.datum).then(a.titel.cmp(&b.titel)));
        s.verfahren
            .sort_by(|a, b| a.bezeichnung.cmp(&b.bezeichnung));
        // Ein Projektwechsel während der Ermittlung darf nicht als konsistenter aktueller Stand erscheinen.
        let neu = self.get_project(p).await?;
        if neu.version != projekt.version {
            return Err(CoreError::InvalidInput(
                "Projekt während der Auswertung geändert. Übersicht aktualisieren.".into(),
            ));
        }
        Ok(s)
    }
}
fn nachweisschluessel(n: &crate::plattform::nachweise::Nachweisstand) -> CoreResult<String> {
    Ok(match &n.eingabe {
        NachweisEingabe::Fach { objekt, .. } => format!("fach:{objekt}"),
        NachweisEingabe::AbstandProjekt { objekt, .. } => format!("abstand:{objekt}"),
        NachweisEingabe::Nutzung { fach, plan } => {
            let mut flurstuecke = plan
                .flurstuecke
                .iter()
                .map(|f| f.id.clone())
                .collect::<Vec<_>>();
            flurstuecke.sort();
            format!("nutzung:{:?}:{}", fach, flurstuecke.join("/"))
        }
        NachweisEingabe::Abstand { gegenstand, .. } => format!("alt:{gegenstand}"),
    })
}

struct Empfang {
    datum: NaiveDate,
    erstellt_am: DateTime<Utc>,
    abgabe: Uuid,
    version: i64,
    name: String,
    empfaenger: String,
    verfahren: Uuid,
}
