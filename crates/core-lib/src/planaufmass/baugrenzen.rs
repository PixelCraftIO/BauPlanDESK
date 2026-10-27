//! Lage im Baufenster aus dem Aufmaß: die Markierungen „Baufenster“ und
//! „Vorhabenumriss“ eines gespeicherten Aufmaßstands liefern die Geometrie
//! für `planungsrecht.baugrenzen` ([`crate::cad::baugrenzen`]). Die
//! Lagebewertung (§ 23 BauNVO) bleibt dort; hier wird nur umgerechnet und
//! dem Projekt zugeordnet.
use super::*;
use crate::{
    CoreServices,
    benutzer::Benutzer,
    cad::{
        Laengeneinheit, Punkt,
        baugrenzen::{BaugrenzenGeometrie, BaugrenzenPruefung},
    },
    domain::{Project, ProjectId, ProjectUpdate},
    planaufmass::grundstueck::Aufmassstand,
};

/// Ergebnis der Zuordnung: gespeicherter Projektstand und Lageprüfung.
#[derive(Debug, Clone, Serialize)]
pub struct BaugrenzenUebernahme {
    pub projekt: Project,
    pub pruefung: BaugrenzenPruefung,
}

fn ring(m: &Planmarkierung) -> CoreResult<Vec<Planpunkt>> {
    match &m.geometrie {
        Plangeometrie::Polygon { punkte } => Ok(punkte.clone()),
        Plangeometrie::Rechteck { von, bis } => {
            Ok(vec![*von, [bis[0], von[1]], *bis, [von[0], bis[1]]])
        }
        Plangeometrie::Flaeche { polygone } => match polygone.as_slice() {
            [p] if p.innen.is_empty() => Ok(p.aussen.clone()),
            _ => Err(fehler(format!(
                "{}: Für die Lageprüfung einen einzelnen Umriss ohne Aussparungen zeichnen",
                m.bezeichnung
            ))),
        },
        _ => Err(fehler(format!(
            "{}: Baufenster und Vorhabenumriss sind Flächen (Polygon oder Rechteck)",
            m.bezeichnung
        ))),
    }
}

impl CoreServices {
    /// Baut aus genau einer Markierung „Baufenster“ und genau einer
    /// Markierung „Vorhabenumriss“ des gespeicherten Aufmaßstands die
    /// Baugrenzengeometrie in Metern (Maßstab bzw. Zeichnungseinheit des
    /// Aufmaßes). Das Baugrundstück folgt der Zuordnung des Vorhabenumrisses.
    pub async fn baugrenzen_aus_aufmass(
        &self,
        pid: ProjectId,
        stand: Aufmassstand,
    ) -> CoreResult<BaugrenzenGeometrie> {
        let a = self.aufmassstand(pid, stand).await?;
        let einzige = |b: Planbedeutung| -> CoreResult<&Planmarkierung> {
            let mut it = a.markierungen.iter().filter(|m| m.bedeutung == b);
            let m = it.next().ok_or_else(|| {
                fehler(format!(
                    "Im Aufmaß fehlt eine Markierung mit Bedeutung „{}“",
                    b.titel()
                ))
            })?;
            if it.next().is_some() {
                return Err(fehler(format!(
                    "Im Aufmaß gibt es mehrere Markierungen „{}“; für die Lageprüfung genau eine behalten",
                    b.titel()
                )));
            }
            Ok(m)
        };
        let bf = einzige(Planbedeutung::Baufenster)?;
        let vh = einzige(Planbedeutung::Vorhabenumriss)?;
        if bf.ausschnitt_id != vh.ausschnitt_id {
            return Err(fehler(
                "Baufenster und Vorhabenumriss im selben Ausschnitt bzw. auf dem Blatt zeichnen",
            ));
        }
        let kalibrierung = bf
            .ausschnitt_id
            .and_then(|id| a.ausschnitte.iter().find(|x| x.id == id))
            .and_then(|x| x.kalibrierung.as_ref())
            .or(a.kalibrierung.as_ref())
            .ok_or_else(|| {
                fehler("Für die Lageprüfung zuerst Maßstab bzw. Zeichnungseinheit festlegen")
            })?;
        let f = kalibrierung.meter_pro_punkt()?;
        // Seitenpunkte zählen y nach unten; die Lageprüfung ist davon unabhängig.
        let meter = |ps: Vec<Planpunkt>| {
            ps.into_iter()
                .map(|p| Punkt(p[0] * f, -p[1] * f))
                .collect::<Vec<_>>()
        };
        let grundstueck_schluessel = match vh.grundstueck_id {
            Some(id) => {
                let p = self.get_project(pid).await?;
                Some(
                    crate::grundstueckskontext::ermitteln(&p)?
                        .into_iter()
                        .find(|g| g.id == id)
                        .map(|g| g.schluessel)
                        .ok_or_else(|| {
                            fehler("Baugrundstück des Vorhabenumrisses fehlt im Projekt")
                        })?,
                )
            }
            None => None,
        };
        let g = BaugrenzenGeometrie {
            dokument_id: a.dokument.dokument_id,
            dokument_version: a.dokument.version,
            einheit: Laengeneinheit::Meter,
            baufenster: meter(ring(bf)?),
            vorhaben: meter(ring(vh)?),
            grundstueck_schluessel,
        };
        g.validate()?;
        Ok(g)
    }

    /// Ordnet die Lage im Baufenster dem Projekt zu (ersetzt eine bestehende
    /// Zuordnung) und liefert den neuen Projektstand mit der Lageprüfung.
    pub async fn planaufmass_baugrenzen(
        &self,
        pid: ProjectId,
        stand: Aufmassstand,
        wer: &Benutzer,
    ) -> CoreResult<BaugrenzenUebernahme> {
        let g = self.baugrenzen_aus_aufmass(pid, stand).await?;
        let pruefung = crate::cad::baugrenzen::pruefe(&g)?;
        let p = self.get_project(pid).await?;
        let mut planungsrecht = p.planungsrecht.clone();
        planungsrecht.baugrenzen = Some(g);
        let projekt = self
            .update_project(
                ProjectUpdate {
                    id: pid,
                    expected_version: p.version,
                    name: p.name.clone(),
                    bundesland: p.bundesland,
                    grundstuecksflaeche_m2: p.grundstuecksflaeche_m2,
                    planungsrecht: Some(planungsrecht),
                },
                wer,
            )
            .await?;
        Ok(BaugrenzenUebernahme { projekt, pruefung })
    }
}
