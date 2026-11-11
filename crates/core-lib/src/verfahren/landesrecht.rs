//! Landesrechtliches Regelpaket (`workflow-regeln/landesrecht.json`):
//! Verfahrenswahl je Fassung der Landesbauordnung, Genehmigungsfiktion und
//! Unterschriftspflichten der Bauvorlagen.
//!
//! Die Verfahrenswahl entscheidet nicht. Sie nennt je Verfahren die
//! Bedingungen mit Fundstelle und wertet aus, was das Projekt belegt; was es
//! nicht belegt, bleibt offen. Maßgebend ist die am Stichtag (Antragseingang)
//! geltende Fassung.
use super::*;
use crate::domain::Bundesland;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Merkmal {
    Wohngebaeude,
    /// Gebäudeklasse höchstens `wert`; ohne Wert ist die Grenze zu prüfen.
    GebaeudeklasseBis,
    /// Lage im Geltungsbereich eines qualifizierten oder vorhabenbezogenen
    /// Bebauungsplans.
    Bebauungsplan,
    KeinePlanungsrechtlicheAbweichung,
    Erschliessung,
    GemeindeVerzicht,
    KeinGrosserSonderbau,
    GrosserSonderbau,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Bedingungsregel {
    pub merkmal: Merkmal,
    pub wert: Option<u32>,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Verfahrensregel {
    pub art: Verfahrensart,
    pub pruefverfahren: Option<Pruefverfahren>,
    pub titel: String,
    pub rechtsgrundlage: String,
    pub bedingungen: Vec<Bedingungsregel>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Fiktionsregel {
    pub rechtsgrundlage: String,
    pub regulaer_monate: u32,
    pub vereinfacht_wochen: u32,
    pub hinweis: String,
}
/// Amtliche Textquelle einer Fassung und ihr letzter Abruf. Ein Abruf ist
/// keine fachliche Abnahme (`Fassung::geprueft`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Rechtsquelle {
    /// Gesetz oder Verordnung mit Fassung, z. B. „BauPrüfVO, Fassung 26.11.2024“.
    pub titel: String,
    pub url: String,
    /// Tag, an dem der Text unter `url` gelesen und mit dem Regelpaket
    /// verglichen wurde; `None`, wenn der Abruf scheiterte.
    pub abgerufen: Option<NaiveDate>,
    /// Ergebnis des letzten Abrufversuchs, z. B. warum der Text fehlte.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    #[ts(as = "Option<String>", optional)]
    pub vermerk: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Fassung {
    pub land: Bundesland,
    pub titel: String,
    pub gilt_ab: Option<NaiveDate>,
    pub gilt_bis: Option<NaiveDate>,
    /// Am Gesetzestext fachlich abgenommen.
    pub geprueft: bool,
    pub verfahrensfrei: String,
    pub verfahren: Vec<Verfahrensregel>,
    pub fiktion: Option<Fiktionsregel>,
    /// Amtliche Textquellen dieser Fassung; leer, wenn keine erfasst sind.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[ts(as = "Option<Vec<Rechtsquelle>>", optional)]
    pub quellen: Vec<Rechtsquelle>,
}
impl Fassung {
    pub fn gilt(&self, tag: NaiveDate) -> bool {
        self.gilt_ab.is_none_or(|a| tag >= a) && self.gilt_bis.is_none_or(|b| tag <= b)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Unterschriftsrolle {
    Bauherrschaft,
    Entwurfsverfasser,
    Fachplaner,
}
impl Unterschriftsrolle {
    pub fn bezeichnung(&self) -> &'static str {
        match self {
            Self::Bauherrschaft => "Bauherrschaft",
            Self::Entwurfsverfasser => "Entwurfsverfasser/in",
            Self::Fachplaner => "Fachplaner/in",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Unterschriftsregel {
    pub land: Bundesland,
    /// Dokumentkategorien (auch als Hauptkategorie einer Unterkategorie).
    pub kategorien: Vec<String>,
    pub rollen: Vec<Unterschriftsrolle>,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Regelpaket {
    pub id: String,
    pub version: String,
    pub hinweis: String,
    pub fassungen: Vec<Fassung>,
    pub unterschriften: Vec<Unterschriftsregel>,
}
impl Regelpaket {
    pub fn mitgeliefert() -> CoreResult<Self> {
        serde_json::from_str(include_str!("../../../../workflow-regeln/landesrecht.json"))
            .map_err(|e| fehler(format!("Landesrechtliches Regelpaket: {e}")))
    }
    pub fn fassung(&self, land: Bundesland, tag: NaiveDate) -> Option<&Fassung> {
        self.fassungen
            .iter()
            .find(|f| f.land == land && f.gilt(tag))
    }
}

/// Art einer Unterschrift auf einer Bauvorlage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Unterschriftsart {
    Handschriftlich,
    /// Qualifizierte elektronische Signatur.
    Qualifiziert,
    /// Authentifizierte Einreichung über ein Portal.
    Portal,
}
/// Erfasste Unterschrift auf einer bestimmten Dokumentversion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Unterschrift {
    pub id: Uuid,
    pub dokument: Dokumentbezug,
    pub rolle: Unterschriftsrolle,
    pub name: String,
    pub art: Unterschriftsart,
    pub datum: NaiveDate,
}
/// Bedarf an Unterschriften einer Dokumentversion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Unterschriftsbedarf {
    pub dokument: Dokumentbezug,
    pub name: String,
    pub kategorie: String,
    pub erforderlich: Vec<Unterschriftsrolle>,
    pub fehlend: Vec<Unterschriftsrolle>,
    pub quelle: String,
}
/// Unterschriftsbedarf je Dokument aus Kategorie und Regeln des Landes.
pub fn unterschriftsbedarf(
    paket: &Regelpaket,
    land: Bundesland,
    dokumente: &[(
        Dokumentbezug,
        String,
        crate::dokumentkategorien::KategorieStand,
    )],
    erfasst: &[Unterschrift],
) -> Vec<Unterschriftsbedarf> {
    let mut r = Vec::new();
    for (d, name, k) in dokumente {
        let regeln: Vec<&Unterschriftsregel> = paket
            .unterschriften
            .iter()
            .filter(|u| {
                u.land == land
                    && (u.kategorien.contains(&k.id)
                        || k.hauptkategorie
                            .as_ref()
                            .is_some_and(|h| u.kategorien.contains(h)))
            })
            .collect();
        if regeln.is_empty() {
            continue;
        }
        let mut erforderlich: Vec<Unterschriftsrolle> = Vec::new();
        for rolle in regeln.iter().flat_map(|u| u.rollen.iter()) {
            if !erforderlich.contains(rolle) {
                erforderlich.push(*rolle);
            }
        }
        let fehlend = erforderlich
            .iter()
            .filter(|rolle| {
                !erfasst
                    .iter()
                    .any(|u| &u.dokument == d && u.rolle == **rolle)
            })
            .copied()
            .collect();
        r.push(Unterschriftsbedarf {
            dokument: d.clone(),
            name: name.clone(),
            kategorie: k.name.clone(),
            erforderlich,
            fehlend,
            quelle: regeln
                .iter()
                .map(|u| u.quelle.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        });
    }
    r
}

/// Was das Projekt zu den Merkmalen belegt; `None` = nicht belegt.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Vorhabenmerkmale {
    pub gebaeudeklasse: Option<u32>,
    pub sonderbau: Option<bool>,
    pub grosser_sonderbau: Option<bool>,
    pub wohngebaeude: Option<bool>,
    pub bebauungsplan: Option<bool>,
    pub planungsrechtliche_abweichung: Option<bool>,
    pub erschliessung: Option<bool>,
    pub gemeinde_verzicht: Option<bool>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum Eignung {
    /// Alle Bedingungen belegt.
    Moeglich,
    /// Mindestens eine Bedingung nicht erfüllt.
    Ausgeschlossen,
    /// Bedingungen offen.
    Pruefen,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Bedingungsergebnis {
    pub text: String,
    pub erfuellt: Option<bool>,
    pub quelle: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Verfahrensvorschlag {
    pub art: Verfahrensart,
    pub pruefverfahren: Option<Pruefverfahren>,
    pub titel: String,
    pub rechtsgrundlage: String,
    pub eignung: Eignung,
    pub bedingungen: Vec<Bedingungsergebnis>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
pub struct Verfahrenswahl {
    pub stichtag: NaiveDate,
    /// Angewandte Fassung; leer ohne Regelpaket für das Land.
    pub fassung: Option<String>,
    pub geprueft: bool,
    pub merkmale: Vorhabenmerkmale,
    pub vorschlaege: Vec<Verfahrensvorschlag>,
    /// Genehmigungsfiktion nach der Fassung, falls vorgesehen.
    pub fiktion: Option<Fiktionsregel>,
    pub hinweise: Vec<String>,
}
fn bedingung(b: &Bedingungsregel, m: &Vorhabenmerkmale) -> Bedingungsergebnis {
    let ja_nein = |v: Option<bool>| v;
    let (text, erfuellt) = match b.merkmal {
        Merkmal::Wohngebaeude => ("Wohngebäude".to_string(), ja_nein(m.wohngebaeude)),
        Merkmal::GebaeudeklasseBis => match b.wert {
            Some(n) => (
                format!("Gebäudeklasse 1 bis {n}"),
                m.gebaeudeklasse.map(|g| g <= n),
            ),
            None => ("Gebäudeklasse im Anwendungsbereich".to_string(), None),
        },
        Merkmal::Bebauungsplan => (
            "Im Geltungsbereich eines qualifizierten oder vorhabenbezogenen Bebauungsplans"
                .to_string(),
            m.bebauungsplan,
        ),
        Merkmal::KeinePlanungsrechtlicheAbweichung => (
            "Keine Ausnahme oder Befreiung erforderlich".to_string(),
            m.planungsrechtliche_abweichung.map(|a| !a),
        ),
        Merkmal::Erschliessung => ("Erschließung gesichert".to_string(), m.erschliessung),
        Merkmal::GemeindeVerzicht => (
            "Gemeinde verlangt kein Genehmigungsverfahren".to_string(),
            m.gemeinde_verzicht,
        ),
        Merkmal::KeinGrosserSonderbau => (
            "Kein großer Sonderbau".to_string(),
            match (m.grosser_sonderbau, m.sonderbau) {
                (Some(g), _) => Some(!g),
                (None, Some(false)) => Some(true),
                _ => None,
            },
        ),
        Merkmal::GrosserSonderbau => (
            "Großer Sonderbau".to_string(),
            match (m.grosser_sonderbau, m.sonderbau) {
                (Some(g), _) => Some(g),
                (None, Some(false)) => Some(false),
                _ => None,
            },
        ),
    };
    Bedingungsergebnis {
        text,
        erfuellt,
        quelle: b.quelle.clone(),
    }
}
/// Reine Verfahrenswahl für Land, Stichtag und Merkmale.
pub fn verfahrenswahl(
    paket: &Regelpaket,
    land: Bundesland,
    stichtag: NaiveDate,
    m: Vorhabenmerkmale,
) -> Verfahrenswahl {
    let Some(f) = paket.fassung(land, stichtag) else {
        return Verfahrenswahl {
            stichtag,
            fassung: None,
            geprueft: false,
            merkmale: m,
            vorschlaege: vec![],
            fiktion: None,
            hinweise: vec![format!(
                "Für {} liegt kein Regelpaket vor. Verfahrensfreiheit, Genehmigungsfreistellung, vereinfachtes und reguläres Verfahren nach der Landesbauordnung prüfen (vgl. §§ 61 bis 64 MBO).",
                land.name()
            )],
        };
    };
    let vorschlaege = f
        .verfahren
        .iter()
        .map(|v| {
            let bedingungen: Vec<_> = v.bedingungen.iter().map(|b| bedingung(b, &m)).collect();
            let eignung = if bedingungen.iter().any(|b| b.erfuellt == Some(false)) {
                Eignung::Ausgeschlossen
            } else if bedingungen.iter().all(|b| b.erfuellt == Some(true)) {
                Eignung::Moeglich
            } else {
                Eignung::Pruefen
            };
            Verfahrensvorschlag {
                art: v.art.clone(),
                pruefverfahren: v.pruefverfahren,
                titel: v.titel.clone(),
                rechtsgrundlage: v.rechtsgrundlage.clone(),
                eignung,
                bedingungen,
            }
        })
        .collect();
    let mut hinweise = vec![format!(
        "Vor dem Verfahren prüfen, ob das Vorhaben verfahrensfrei ist: {}.",
        f.verfahrensfrei
    )];
    if !f.geprueft {
        hinweise.push(
            "Die Bedingungen dieser Fassung sind nicht fachlich abgenommen; Fundstellen am Gesetzestext prüfen."
                .into(),
        );
        let fehlend: Vec<&str> = f
            .quellen
            .iter()
            .filter(|q| q.abgerufen.is_none())
            .map(|q| q.titel.as_str())
            .collect();
        if !fehlend.is_empty() {
            hinweise.push(format!(
                "Amtlicher Text bei der letzten Quellenprüfung nicht abrufbar: {}. Die Werte stammen aus Sekundärquellen.",
                fehlend.join("; ")
            ));
        }
    }
    if let Some(b) = paket
        .fassungen
        .iter()
        .find(|n| n.land == land && n.gilt_ab.is_some_and(|a| a > stichtag))
    {
        hinweise.push(format!(
            "Ab {} gilt „{}“; maßgebend ist der Eingang des Antrags.",
            b.gilt_ab.expect("geprüft").format("%d.%m.%Y"),
            b.titel
        ));
    }
    Verfahrenswahl {
        stichtag,
        fassung: Some(f.titel.clone()),
        geprueft: f.geprueft,
        merkmale: m,
        vorschlaege,
        fiktion: f.fiktion.clone(),
        hinweise,
    }
}
/// Ergänzende Angaben, die das Projekt nicht kennt.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Verfahrensangaben {
    pub wohngebaeude: Option<bool>,
    pub grosser_sonderbau: Option<bool>,
    pub erschliessung: Option<bool>,
    pub gemeinde_verzicht: Option<bool>,
    /// Stichtag (Antragseingang); ohne Angabe heute.
    pub stichtag: Option<NaiveDate>,
}
impl CoreServices {
    /// Verfahrenswahl aus Projekt, Vorhabenprofil der Akte, Abweichungsbedarf
    /// und ergänzenden Angaben.
    pub async fn verfahrenswahl(
        &self,
        p: ProjectId,
        vid: Option<Uuid>,
        angaben: Verfahrensangaben,
    ) -> CoreResult<Verfahrenswahl> {
        let projekt = self.get_project(p).await?;
        let akte = match vid {
            Some(v) => Some(self.verfahrensakte(p, v).await?.1),
            None => None,
        };
        let profil = akte
            .as_ref()
            .and_then(|a| a.profil.as_ref())
            .map(|p| &p.vorhaben);
        let (bedarf, _) = self.abweichungsbedarf(p).await?;
        let planungsrechtlich = bedarf.iter().any(|b| b.art.planungsrechtlich())
            || akte.as_ref().is_some_and(|a| {
                a.abweichungen
                    .iter()
                    .any(|e| e.aktiv() && e.art.planungsrechtlich())
            });
        let pr = &projekt.planungsrecht;
        let bebauungsplan = if pr.mit_bebauungsplan() {
            Some(matches!(
                pr.bebauungsplan_art,
                crate::domain::BebauungsplanArt::Qualifiziert
                    | crate::domain::BebauungsplanArt::Vorhabenbezogen
            ))
        } else if pr.rechtsgrundlage.is_some() {
            Some(false)
        } else {
            None
        };
        let m = Vorhabenmerkmale {
            gebaeudeklasse: profil.and_then(|p| p.gebaeudeklasse),
            sonderbau: profil.and_then(|p| p.sonderbau),
            grosser_sonderbau: angaben.grosser_sonderbau,
            wohngebaeude: angaben
                .wohngebaeude
                .or_else(|| profil.and_then(|p| p.wohngebaeude))
                .or_else(|| {
                    profil
                        .map(|p| p.nutzung.to_lowercase())
                        .filter(|n| !n.trim().is_empty())
                        .map(|n| n.contains("wohn"))
                }),
            bebauungsplan,
            planungsrechtliche_abweichung: Some(planungsrechtlich),
            erschliessung: angaben.erschliessung,
            gemeinde_verzicht: angaben.gemeinde_verzicht,
        };
        let paket = Regelpaket::mitgeliefert()?;
        let mut w = verfahrenswahl(
            &paket,
            projekt.bundesland,
            angaben.stichtag.unwrap_or_else(crate::kalender::heute),
            m,
        );
        if profil.is_none() {
            w.hinweise.push(
                "Gebäudeklasse, Sonderbau und Nutzung im Vorhabenprofil der Akte erfassen.".into(),
            );
        }
        Ok(w)
    }
    /// Unterschriftsbedarf der jeweils neuesten Dokumentversionen des
    /// Projekts mit Dokumentkategorie.
    pub async fn verfahren_unterschriften(
        &self,
        p: ProjectId,
        vid: Uuid,
    ) -> CoreResult<Vec<Unterschriftsbedarf>> {
        let (_, a) = self.verfahrensakte(p, vid).await?;
        let land = self.get_project(p).await?.bundesland;
        let kategorien = self.kategoriesnapshots(p).await?;
        let docs = self
            .list_documents(p)
            .await?
            .into_iter()
            .filter_map(|d| {
                let k = kategorien
                    .get(&d.document_id.0.to_string())
                    .cloned()
                    .flatten()?;
                Some((
                    Dokumentbezug {
                        dokument_id: d.document_id,
                        version: d.version,
                    },
                    d.file_name,
                    k,
                ))
            })
            .collect::<Vec<_>>();
        Ok(unterschriftsbedarf(
            &Regelpaket::mitgeliefert()?,
            land,
            &docs,
            &a.unterschriften,
        ))
    }
}
/// Hinweise zu fehlenden Unterschriften der Unterlagen eines Abgabestands.
pub(super) fn abgabe_hinweise(
    land: Bundesland,
    dokumente: &[abgabe::Abgabedokument],
    erfasst: &[Unterschrift],
) -> CoreResult<Vec<String>> {
    let paket = Regelpaket::mitgeliefert()?;
    let docs: Vec<_> = dokumente
        .iter()
        .filter_map(|d| Some((d.dokument.clone(), d.name.clone(), d.kategorie.clone()?)))
        .collect();
    Ok(unterschriftsbedarf(&paket, land, &docs, erfasst)
        .into_iter()
        .filter(|b| !b.fehlend.is_empty())
        .map(|b| {
            format!(
                "{}: Unterschrift {} nicht erfasst ({}).",
                b.name,
                b.fehlend
                    .iter()
                    .map(|r| r.bezeichnung())
                    .collect::<Vec<_>>()
                    .join(", "),
                b.quelle
            )
        })
        .collect())
}
pub(super) fn unterschriften_pruefen(
    a: &Verfahrensakte,
    dokumente: &mut Vec<Dokumentbezug>,
) -> CoreResult<()> {
    if a.unterschriften.len() > 2000 {
        return Err(fehler("Höchstens 2000 Unterschriften je Akte"));
    }
    for u in &a.unterschriften {
        crate::plattform::text(&u.name, 500)?;
        if u.datum > crate::kalender::heute() {
            return Err(fehler("Datum einer Unterschrift liegt in der Zukunft"));
        }
        dokumente.push(u.dokument.clone());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tag(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }
    #[test]
    fn regelpaket_ist_lesbar_und_fassungen_schliessen_lueckenlos() {
        let p = Regelpaket::mitgeliefert().unwrap();
        let alt = p.fassung(Bundesland::Nw, tag("2026-08-31")).unwrap();
        let neu = p.fassung(Bundesland::Nw, tag("2026-09-01")).unwrap();
        assert_ne!(alt.titel, neu.titel);
        assert!(alt.fiktion.is_none());
        assert_eq!(neu.fiktion.as_ref().unwrap().vereinfacht_wochen, 6);
        assert!(p.fassung(Bundesland::By, tag("2026-09-01")).is_none());
    }
    #[test]
    fn ungepruefte_fassung_nennt_nicht_abgerufene_quellen() {
        let p = Regelpaket::mitgeliefert().unwrap();
        for f in &p.fassungen {
            // Der Abruf ersetzt die fachliche Abnahme nicht.
            assert!(!f.geprueft, "{}", f.titel);
            assert!(!f.quellen.is_empty(), "{}", f.titel);
        }
        let w = verfahrenswahl(
            &p,
            Bundesland::Nw,
            tag("2026-09-01"),
            Vorhabenmerkmale::default(),
        );
        assert!(
            w.hinweise
                .iter()
                .any(|h| h.contains("nicht abrufbar") && h.contains("BauPrüfVO")),
            "{:?}",
            w.hinweise
        );
        // Ohne Quellen bleibt es beim bisherigen Hinweis; Altpakete bleiben lesbar.
        let mut alt: serde_json::Value =
            serde_json::from_str(include_str!("../../../../workflow-regeln/landesrecht.json"))
                .unwrap();
        for f in alt["fassungen"].as_array_mut().unwrap() {
            f.as_object_mut().unwrap().remove("quellen");
        }
        let alt: Regelpaket = serde_json::from_value(alt).unwrap();
        let w = verfahrenswahl(
            &alt,
            Bundesland::Nw,
            tag("2026-09-01"),
            Vorhabenmerkmale::default(),
        );
        assert!(!w.hinweise.iter().any(|h| h.contains("nicht abrufbar")));
        assert!(!serde_json::to_string(&alt).unwrap().contains("quellen"));
    }
    #[test]
    fn befreiungsbedarf_schliesst_freistellung_aus_und_offenes_bleibt_offen() {
        let p = Regelpaket::mitgeliefert().unwrap();
        let mut m = Vorhabenmerkmale {
            gebaeudeklasse: Some(2),
            sonderbau: Some(false),
            wohngebaeude: Some(true),
            bebauungsplan: Some(true),
            planungsrechtliche_abweichung: Some(false),
            ..Default::default()
        };
        let w = verfahrenswahl(&p, Bundesland::Nw, tag("2026-08-01"), m.clone());
        let frei = &w.vorschlaege[0];
        assert_eq!(frei.art, Verfahrensart::Freistellung);
        assert_eq!(frei.eignung, Eignung::Pruefen, "Erschließung offen");
        let vereinfacht = &w.vorschlaege[1];
        assert_eq!(vereinfacht.eignung, Eignung::Moeglich);
        assert_eq!(w.vorschlaege[2].eignung, Eignung::Ausgeschlossen);
        assert!(w.hinweise.iter().any(|h| h.contains("01.09.2026")));
        m.erschliessung = Some(true);
        m.gemeinde_verzicht = Some(true);
        let w = verfahrenswahl(&p, Bundesland::Nw, tag("2026-08-01"), m.clone());
        assert_eq!(w.vorschlaege[0].eignung, Eignung::Moeglich);
        m.planungsrechtliche_abweichung = Some(true);
        let w = verfahrenswahl(&p, Bundesland::Nw, tag("2026-08-01"), m.clone());
        assert_eq!(w.vorschlaege[0].eignung, Eignung::Ausgeschlossen);
        // Neue Fassung: Gebäudeklassengrenze ist zu prüfen, nicht geraten.
        m.planungsrechtliche_abweichung = Some(false);
        m.gebaeudeklasse = Some(4);
        let w = verfahrenswahl(&p, Bundesland::Nw, tag("2026-10-01"), m);
        assert_eq!(w.vorschlaege[0].eignung, Eignung::Pruefen);
        assert!(w.fiktion.is_some());
    }
    #[test]
    fn unterschriften_je_kategorie_und_version() {
        let p = Regelpaket::mitgeliefert().unwrap();
        let d = Dokumentbezug {
            dokument_id: crate::domain::DocumentId(Uuid::new_v4()),
            version: 2,
        };
        let k = |id: &str, haupt: Option<&str>| crate::dokumentkategorien::KategorieStand {
            id: id.into(),
            name: id.into(),
            hauptkategorie: haupt.map(String::from),
        };
        let docs = vec![
            (d.clone(), "Antrag.pdf".to_string(), k("bauantrag", None)),
            (d.clone(), "EG.pdf".to_string(), k("eg", Some("grundriss"))),
            (d.clone(), "Foto.jpg".to_string(), k("foto", None)),
        ];
        let mut u = vec![Unterschrift {
            id: Uuid::new_v4(),
            dokument: Dokumentbezug {
                version: 1,
                ..d.clone()
            },
            rolle: Unterschriftsrolle::Entwurfsverfasser,
            name: "A. Architektin".into(),
            art: Unterschriftsart::Qualifiziert,
            datum: tag("2026-05-01"),
        }];
        let b = unterschriftsbedarf(&p, Bundesland::Nw, &docs, &u);
        assert_eq!(b.len(), 2, "Foto braucht keine Unterschrift");
        assert_eq!(
            b[0].fehlend,
            vec![
                Unterschriftsrolle::Bauherrschaft,
                Unterschriftsrolle::Entwurfsverfasser
            ],
            "Unterschrift gilt nur für ihre Version"
        );
        u[0].dokument.version = 2;
        let b = unterschriftsbedarf(&p, Bundesland::Nw, &docs, &u);
        assert_eq!(b[0].fehlend, vec![Unterschriftsrolle::Bauherrschaft]);
        assert!(b[1].fehlend.is_empty(), "Unterkategorie erbt die Regel");
    }
}
