//! Übernahme gezeichneter Grundstücksflächen aus dem Lageplan in die
//! Baugrundstücksermittlung: Abzugsflächen (private/öffentliche Grünfläche,
//! Verkehrsfläche, Fläche außerhalb des Baulands) je Flurstück und GRZ-Bauteile
//! ohne Gebäude (Zufahrt, Stellplatz, Garage, Nebenanlage, Terrasse,
//! unterirdische Anlage) je Baugrundstück. Die Zeichnung liefert nur die
//! Fläche; Anrechnung, Abzugsart und rechtliche Grundlage bleiben Eingaben
//! und Regeln des Grundstücksmoduls.
use super::abgleich::{
    Abgleichsfrage, Abgleichskandidat, Zeilenzuordnung, vorschlag, wahl, zeilenschluessel,
};
use super::*;
use crate::{
    CoreServices,
    baugrundstuecke::FlaechenAbzug,
    domain::ProjectId,
    flaechen::{Bauteil, FlaechenAufstellung},
    grundstuecksinformationen::{BaugrundstuecksErmittlung, Grundstuecksinformationen},
    plattform::ObjektArt,
};

fn abzugsziel(id: &str) -> String {
    format!("abzuege.{id}.flaeche_m2")
}
fn bauteilziel(id: &str) -> String {
    format!("bauteile.{id}.flaeche_m2")
}
fn gerundet(wert: f64) -> f64 {
    (wert * 100.).round() / 100.
}
fn abzugsname(art: &str) -> String {
    crate::baugrundstuecke::abzugsregeln()
        .get(art)
        .map(|r| r.bezeichnung.clone())
        .unwrap_or_else(|| art.to_string())
}

fn bauteilname(b: Planbedeutung) -> &'static str {
    match b {
        Planbedeutung::Zufahrt => "Zufahrt",
        Planbedeutung::Stellplatz => "Stellplatz",
        Planbedeutung::Garage => "Garage",
        Planbedeutung::Nebenanlage => "Nebenanlage",
        Planbedeutung::Terrasse => "Terrasse",
        Planbedeutung::Unterirdisch => "Unterirdische Anlage",
        _ => "Bauteil",
    }
}
fn abzugsbindung(s: Aufmassstand, markierung: Uuid) -> Planbindung {
    Planbindung {
        aufmass_id: s.aufmass_id,
        revision: s.revision,
        markierung_id: markierung,
        groesse: Planmessgroesse::Flaeche,
        ziel: abzugsziel(&markierung.to_string()),
    }
}
fn bauteilbindung(s: Aufmassstand, markierung: Uuid) -> Planbindung {
    Planbindung {
        ziel: bauteilziel(&markierung.to_string()),
        ..abzugsbindung(s, markierung)
    }
}
/// Zeile, die schon zu dieser Markierung gehört: aus ihr angelegt (Kennung =
/// Markierungs-ID) oder mit ihr verknüpft.
fn abzug_zu(zeilen: &[FlaechenAbzug], markierung: Uuid) -> Option<usize> {
    let id = markierung.to_string();
    zeilen.iter().position(|x| {
        x.id.as_deref() == Some(id.as_str())
            || x.planbindung
                .as_ref()
                .is_some_and(|b| b.markierung_id == markierung)
    })
}
fn bauteil_zu(zeilen: &[Bauteil], markierung: Uuid) -> Option<usize> {
    let id = markierung.to_string();
    zeilen.iter().position(|x| {
        x.id.as_deref() == Some(id.as_str())
            || x.planbindung
                .as_ref()
                .is_some_and(|b| b.markierung_id == markierung)
    })
}
/// Ungebundene Handzeilen gleicher Abzugsart (Position, Abgleichsschlüssel).
fn abzugskandidaten(zeilen: &[FlaechenAbzug], art: &str) -> Vec<(usize, String)> {
    zeilen
        .iter()
        .enumerate()
        .filter(|(_, x)| x.planbindung.is_none() && x.art == art)
        .map(|(i, x)| (i, zeilenschluessel(x.id.as_deref(), i)))
        .collect()
}
/// Ungebundene Bauteile ohne Gebäudeverknüpfung, deren Typ zur Bedeutung der
/// Markierung passt (eine Garage in der Abstandsfläche ist gezeichnet „Garage“).
fn bauteilkandidaten(zeilen: &[Bauteil], bedeutung: Planbedeutung) -> Vec<(usize, String)> {
    zeilen
        .iter()
        .enumerate()
        .filter(|(_, x)| {
            x.planbindung.is_none()
                && x.gebaeude_id.is_none()
                && Planbedeutung::fuer_bauteiltyp(x.typ) == Some(bedeutung)
        })
        .map(|(i, x)| (i, zeilenschluessel(x.id.as_deref(), i)))
        .collect()
}

/// Zielfelder der erfassten Abzugsflächen (nur Zeilen mit Kennung).
pub fn abzugsziele(
    e: &BaugrundstuecksErmittlung,
    infos: &Grundstuecksinformationen,
) -> Vec<Planziel> {
    let mut z = Vec::new();
    for a in &e.ansaetze {
        let flurstueck = infos
            .flurstuecke
            .iter()
            .find(|f| f.id == a.flurstueck_id)
            .map(|f| f.bezeichnung())
            .unwrap_or_else(|| a.flurstueck_id.clone());
        for abzug in &a.abzuege {
            if let Some(id) = &abzug.id {
                z.push(Planziel {
                    ziel: abzugsziel(id),
                    titel: format!(
                        "Flurstück {flurstueck} · {} · Fläche",
                        abzugsname(&abzug.art)
                    ),
                    groesse: Planmessgroesse::Flaeche,
                });
            }
        }
    }
    z
}
/// Zielfelder der GRZ-Bauteile ohne Gebäude (nur Zeilen mit Kennung).
pub fn bauteilziele(a: &FlaechenAufstellung) -> Vec<Planziel> {
    a.bauteile
        .iter()
        .filter(|b| b.gebaeude_id.is_none())
        .filter_map(|b| {
            let id = b.id.as_ref()?;
            let name = match b.bezeichnung.trim() {
                "" => "Bauteil".to_string(),
                n => n.to_string(),
            };
            Some(Planziel {
                ziel: bauteilziel(id),
                titel: format!("{name} · Grundfläche"),
                groesse: Planmessgroesse::Flaeche,
            })
        })
        .collect()
}

/// Gespeicherter Aufmaßstand, aus dem neue Zeilen gebildet werden.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct Aufmassstand {
    pub aufmass_id: Uuid,
    #[ts(type = "number")]
    pub revision: i64,
}

impl CoreServices {
    pub(super) async fn aufmassstand(
        &self,
        pid: ProjectId,
        s: Aufmassstand,
    ) -> CoreResult<Planaufmass> {
        let f = self
            .fachobjekte(pid, ObjektArt::Planaufmass)
            .await?
            .into_iter()
            .find(|f| f.objekt == s.aufmass_id.to_string())
            .ok_or_else(|| fehler("Planaufmaß fehlt im Projekt"))?;
        if f.revision != s.revision {
            return Err(fehler(
                "Planaufmaß wurde zwischenzeitlich gespeichert; Liste neu laden und erneut übernehmen",
            ));
        }
        serde_json::from_value(f.daten).map_err(|e| fehler(e.to_string()))
    }

    /// Rückfragen vor der Übernahme der Abzugsflächen: Markierungen, die eine
    /// neue Zeile anlegen würden, obwohl am selben Flurstück schon eine
    /// ungebundene Zeile gleicher Art von Hand erfasst ist.
    pub async fn planaufmass_abzuege_abgleich(
        &self,
        pid: ProjectId,
        aufmass: Aufmassstand,
        e: &BaugrundstuecksErmittlung,
    ) -> CoreResult<Vec<Abgleichsfrage>> {
        let a = self.aufmassstand(pid, aufmass).await?;
        let projekt = self.get_project(pid).await?;
        let infos = crate::grundstuecksinformationen::informationen(&projekt.planungsrecht);
        let mut fragen = Vec::new();
        for m in &a.markierungen {
            let (Some(art), Some(flurstueck)) = (m.bedeutung.abzugsart(), &m.flurstueck_id) else {
                continue;
            };
            let Some(ansatz) = e.ansaetze.iter().find(|x| &x.flurstueck_id == flurstueck) else {
                continue;
            };
            if abzug_zu(&ansatz.abzuege, m.id).is_some() {
                continue;
            }
            let kandidaten: Vec<Abgleichskandidat> = abzugskandidaten(&ansatz.abzuege, art)
                .into_iter()
                .map(|(i, zeile)| Abgleichskandidat {
                    zeile,
                    bezeichnung: match ansatz.abzuege[i].grundlage.trim() {
                        "" => abzugsname(art),
                        g => format!("{} · {g}", abzugsname(art)),
                    },
                    flaeche_m2: ansatz.abzuege[i].flaeche_m2,
                })
                .collect();
            if kandidaten.is_empty() {
                continue;
            }
            let flaeche = self
                .planbindung_aufloesen(pid, &abzugsbindung(aufmass, m.id))
                .await
                .ok()
                .map(|(_, _, w)| gerundet(w));
            let name = infos
                .flurstuecke
                .iter()
                .find(|f| &f.id == flurstueck)
                .map(|f| f.bezeichnung())
                .unwrap_or_else(|| flurstueck.clone());
            fragen.push(Abgleichsfrage {
                markierung_id: m.id,
                markierung: m.bezeichnung.clone(),
                ziel: format!("Flurstück {name} · {}", abzugsname(art)),
                flaeche_m2: flaeche,
                vorschlag: vorschlag(flaeche, &kandidaten),
                kandidaten,
            });
        }
        Ok(fragen)
    }

    /// Entwurfsübernahme der Abzugsflächen: mit `aufmass` wird je Markierung
    /// mit Abzugsbedeutung und Flurstückszuordnung die zugehörige Zeile
    /// aktualisiert, eine Handzeile verknüpft (`zuordnung`, siehe
    /// [`Self::planaufmass_abzuege_abgleich`]) oder eine neue Zeile angelegt.
    /// Art und rechtliche Grundlage bestehender Zeilen bleiben erhalten, neue
    /// Zeilen tragen eine leere Grundlage, die vor dem Speichern zu ergänzen
    /// ist. Anschließend werden alle Bindungen aufgelöst.
    pub async fn planaufmass_abzuege(
        &self,
        pid: ProjectId,
        aufmass: Option<Aufmassstand>,
        zuordnung: &[Zeilenzuordnung],
        mut e: BaugrundstuecksErmittlung,
    ) -> CoreResult<BaugrundstuecksErmittlung> {
        if let Some(s) = aufmass {
            let a = self.aufmassstand(pid, s).await?;
            let mut getroffen = 0;
            for m in &a.markierungen {
                let (Some(art), Some(flurstueck)) = (m.bedeutung.abzugsart(), &m.flurstueck_id)
                else {
                    continue;
                };
                let Some(ansatz) = e
                    .ansaetze
                    .iter_mut()
                    .find(|x| &x.flurstueck_id == flurstueck)
                else {
                    continue;
                };
                getroffen += 1;
                if let Some(i) = abzug_zu(&ansatz.abzuege, m.id) {
                    let x = &mut ansatz.abzuege[i];
                    let id = x.id.clone().unwrap_or_else(|| m.id.to_string());
                    x.id = Some(id.clone());
                    x.planbindung = Some(Planbindung {
                        ziel: abzugsziel(&id),
                        ..abzugsbindung(s, m.id)
                    });
                    continue;
                }
                let kandidaten = abzugskandidaten(&ansatz.abzuege, art);
                match wahl(zuordnung, m, &kandidaten)? {
                    Some(i) => {
                        let x = &mut ansatz.abzuege[i];
                        let id = x.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
                        x.id = Some(id.clone());
                        x.planbindung = Some(Planbindung {
                            ziel: abzugsziel(&id),
                            ..abzugsbindung(s, m.id)
                        });
                    }
                    None => ansatz.abzuege.push(FlaechenAbzug {
                        id: Some(m.id.to_string()),
                        art: art.into(),
                        flaeche_m2: 0.,
                        grundlage: String::new(),
                        planbindung: Some(abzugsbindung(s, m.id)),
                    }),
                }
            }
            if getroffen == 0 {
                return Err(fehler(
                    "Im Aufmaß gibt es keine Abzugsfläche mit zugeordnetem Vorhabenflurstück (Bedeutung Grünfläche, Verkehrsfläche, außerhalb Bauland oder sonstiger Abzug; Flurstück in den Eigenschaften wählen)",
                ));
            }
        }
        self.planwerte_abzuege(pid, &mut e).await?;
        Ok(e)
    }
    pub(crate) async fn planwerte_abzuege(
        &self,
        pid: ProjectId,
        e: &mut BaugrundstuecksErmittlung,
    ) -> CoreResult<()> {
        for ansatz in &mut e.ansaetze {
            for abzug in &mut ansatz.abzuege {
                let Some(b) = abzug.planbindung.clone() else {
                    continue;
                };
                let benannt = |t: &str| {
                    fehler(format!(
                        "Flächenabzug {} (Flurstück {}): {t}",
                        abzugsname(&abzug.art),
                        ansatz.flurstueck_id
                    ))
                };
                let Some(id) = &abzug.id else {
                    return Err(benannt("Aufmaßverknüpfung ohne Zeilenkennung"));
                };
                if b.ziel != abzugsziel(id) || b.groesse != Planmessgroesse::Flaeche {
                    return Err(benannt("Aufmaßverknüpfung gehört zu einem anderen Ziel"));
                }
                let (_, m, wert) =
                    self.planbindung_aufloesen(pid, &b)
                        .await
                        .map_err(|e| match e {
                            CoreError::InvalidInput(t) => benannt(&t),
                            e => e,
                        })?;
                if m.bedeutung.abzugsart() != Some(abzug.art.as_str()) {
                    return Err(benannt(
                        "Bedeutung der Planmarkierung passt nicht zur Abzugsart; Art oder Markierung anpassen",
                    ));
                }
                if m.flurstueck_id.as_deref() != Some(ansatz.flurstueck_id.as_str()) {
                    return Err(benannt(
                        "Planmarkierung ist einem anderen Flurstück zugeordnet",
                    ));
                }
                abzug.flaeche_m2 = gerundet(wert);
            }
        }
        Ok(())
    }
    /// Gespeicherte Abzugsflächen mit Aufmaßbindung müssen dem Stand ihrer Quelle entsprechen.
    pub async fn abzug_planquellen_pruefen(
        &self,
        pid: ProjectId,
        e: &BaugrundstuecksErmittlung,
    ) -> CoreResult<()> {
        if !e
            .ansaetze
            .iter()
            .any(|a| a.abzuege.iter().any(|x| x.planbindung.is_some()))
        {
            return Ok(());
        }
        let mut neu = e.clone();
        let hinweis = |t: String| {
            fehler(format!(
                "{t} Unter Berechnungen → Baugrundstück „Aktuellen Aufmaßstand übernehmen“ und speichern."
            ))
        };
        if let Err(err) = self.planwerte_abzuege(pid, &mut neu).await {
            return Err(match err {
                CoreError::InvalidInput(t) => hinweis(t),
                err => err,
            });
        }
        if neu.ansaetze != e.ansaetze {
            return Err(hinweis("Abzugsflächen weichen vom Aufmaß ab.".into()));
        }
        Ok(())
    }

    /// Rückfragen vor der Übernahme der GRZ-Bauteile des Baugrundstücks
    /// `schluessel`: Markierungen, die eine neue Zeile anlegen würden, obwohl
    /// schon ein ungebundenes Bauteil gleichen Typs von Hand erfasst ist.
    pub async fn planaufmass_bauteile_abgleich(
        &self,
        pid: ProjectId,
        schluessel: &str,
        aufmass: Aufmassstand,
        a: &FlaechenAufstellung,
    ) -> CoreResult<Vec<Abgleichsfrage>> {
        let grundstueck = crate::plattform::einbindung::grundstueck_id(pid, schluessel);
        let plan = self.aufmassstand(pid, aufmass).await?;
        let mut fragen = Vec::new();
        for m in &plan.markierungen {
            if m.bedeutung.bauteiltyp().is_none() {
                continue;
            }
            if m.grundstueck_id != Some(grundstueck) || bauteil_zu(&a.bauteile, m.id).is_some() {
                continue;
            }
            let kandidaten: Vec<Abgleichskandidat> = bauteilkandidaten(&a.bauteile, m.bedeutung)
                .into_iter()
                .map(|(i, zeile)| Abgleichskandidat {
                    zeile,
                    bezeichnung: match a.bauteile[i].bezeichnung.trim() {
                        "" => "Bauteil ohne Bezeichnung".into(),
                        b => b.to_string(),
                    },
                    flaeche_m2: a.bauteile[i].flaeche_m2,
                })
                .collect();
            if kandidaten.is_empty() {
                continue;
            }
            let flaeche = self
                .planbindung_aufloesen(pid, &bauteilbindung(aufmass, m.id))
                .await
                .ok()
                .map(|(_, _, w)| gerundet(w));
            fragen.push(Abgleichsfrage {
                markierung_id: m.id,
                markierung: m.bezeichnung.clone(),
                ziel: format!("Baugrundstück · {}", bauteilname(m.bedeutung)),
                flaeche_m2: flaeche,
                vorschlag: vorschlag(flaeche, &kandidaten),
                kandidaten,
            });
        }
        Ok(fragen)
    }

    /// Entwurfsübernahme der GRZ-Bauteile eines Baugrundstücks (`schluessel`):
    /// mit `aufmass` wird je Markierung mit Bauteilbedeutung und passender
    /// Baugrundstückszuordnung die zugehörige Zeile aktualisiert, ein von Hand
    /// erfasstes Bauteil verknüpft (`zuordnung`, siehe
    /// [`Self::planaufmass_bauteile_abgleich`]) oder eine Zeile angelegt; Typ,
    /// Bezeichnung und Sonderansätze bestehender Zeilen bleiben erhalten.
    pub async fn planaufmass_bauteile(
        &self,
        pid: ProjectId,
        schluessel: &str,
        aufmass: Option<Aufmassstand>,
        zuordnung: &[Zeilenzuordnung],
        mut a: FlaechenAufstellung,
    ) -> CoreResult<FlaechenAufstellung> {
        let grundstueck = crate::plattform::einbindung::grundstueck_id(pid, schluessel);
        if let Some(s) = aufmass {
            let plan = self.aufmassstand(pid, s).await?;
            let mut getroffen = 0;
            for m in &plan.markierungen {
                let Some(typ) = m.bedeutung.bauteiltyp() else {
                    continue;
                };
                if m.grundstueck_id != Some(grundstueck) {
                    continue;
                }
                getroffen += 1;
                let gefunden = match bauteil_zu(&a.bauteile, m.id) {
                    Some(i) => Some(i),
                    None => wahl(zuordnung, m, &bauteilkandidaten(&a.bauteile, m.bedeutung))?,
                };
                match gefunden {
                    Some(i) => {
                        let x = &mut a.bauteile[i];
                        let id = x.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
                        x.id = Some(id.clone());
                        x.planbindung = Some(Planbindung {
                            ziel: bauteilziel(&id),
                            ..bauteilbindung(s, m.id)
                        });
                    }
                    None => a.bauteile.push(Bauteil {
                        id: Some(m.id.to_string()),
                        planbindung: Some(bauteilbindung(s, m.id)),
                        gebaeude_id: None,
                        bezeichnung: m.bezeichnung.clone(),
                        typ,
                        flaeche_m2: 0.,
                        abweichende_anrechnung: None,
                        begruendung: String::new(),
                    }),
                }
            }
            if getroffen == 0 {
                return Err(fehler(
                    "Im Aufmaß gibt es kein Bauteil für dieses Baugrundstück (Bedeutung Zufahrt, Stellplatz, Garage, Nebenanlage, Terrasse oder unterirdisch; Baugrundstück in den Eigenschaften wählen)",
                ));
            }
        }
        self.planwerte_bauteile(pid, schluessel, &mut a).await?;
        Ok(a)
    }
    pub(crate) async fn planwerte_bauteile(
        &self,
        pid: ProjectId,
        schluessel: &str,
        a: &mut FlaechenAufstellung,
    ) -> CoreResult<()> {
        let grundstueck = crate::plattform::einbindung::grundstueck_id(pid, schluessel);
        for b in &mut a.bauteile {
            let Some(bindung) = b.planbindung.clone() else {
                continue;
            };
            let benannt = |t: &str| fehler(format!("Bauteil „{}“: {t}", b.bezeichnung));
            let Some(id) = &b.id else {
                return Err(benannt("Aufmaßverknüpfung ohne Zeilenkennung"));
            };
            if b.gebaeude_id.is_some() {
                return Err(benannt(
                    "Gebäudeverknüpfung und Aufmaß sind konkurrierende Quellen",
                ));
            }
            if bindung.ziel != bauteilziel(id) || bindung.groesse != Planmessgroesse::Flaeche {
                return Err(benannt("Aufmaßverknüpfung gehört zu einem anderen Ziel"));
            }
            let (_, m, wert) =
                self.planbindung_aufloesen(pid, &bindung)
                    .await
                    .map_err(|e| match e {
                        CoreError::InvalidInput(t) => benannt(&t),
                        e => e,
                    })?;
            if Planbedeutung::fuer_bauteiltyp(b.typ) != Some(m.bedeutung) {
                return Err(benannt(
                    "Bedeutung der Planmarkierung passt nicht zum Bauteiltyp; Typ oder Markierung anpassen",
                ));
            }
            if m.grundstueck_id != Some(grundstueck) {
                return Err(benannt(
                    "Planmarkierung ist einem anderen Baugrundstück zugeordnet",
                ));
            }
            b.flaeche_m2 = gerundet(wert);
        }
        Ok(())
    }
    /// Gespeicherte Bauteile mit Aufmaßbindung müssen dem Stand ihrer Quelle entsprechen.
    pub async fn bauteil_planquellen_pruefen(
        &self,
        pid: ProjectId,
        schluessel: &str,
        a: &FlaechenAufstellung,
    ) -> CoreResult<()> {
        if a.bauteile.iter().all(|b| b.planbindung.is_none()) {
            return Ok(());
        }
        let mut neu = a.clone();
        let hinweis = |t: String| {
            fehler(format!(
                "{t} Unter Berechnungen → Baugrundstück (GRZ) „Aktuellen Aufmaßstand übernehmen“ und speichern."
            ))
        };
        if let Err(err) = self.planwerte_bauteile(pid, schluessel, &mut neu).await {
            return Err(match err {
                CoreError::InvalidInput(t) => hinweis(t),
                err => err,
            });
        }
        if neu.bauteile != a.bauteile {
            return Err(hinweis("GRZ-Bauteile weichen vom Aufmaß ab.".into()));
        }
        Ok(())
    }
    /// Alle Aufmaßbindungen der Baugrundstücksermittlung (Abzüge und Bauteile).
    pub async fn grundstueck_planquellen_pruefen(
        &self,
        pid: ProjectId,
        e: &BaugrundstuecksErmittlung,
    ) -> CoreResult<()> {
        self.abzug_planquellen_pruefen(pid, e).await?;
        for b in &e.berechnungen {
            if let Some(a) = &b.eingabe.aufstellung {
                self.bauteil_planquellen_pruefen(pid, &b.schluessel, a)
                    .await?;
            }
        }
        Ok(())
    }
}
