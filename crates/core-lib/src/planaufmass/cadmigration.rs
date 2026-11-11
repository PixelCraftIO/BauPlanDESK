//! Übernahme gespeicherter CAD-Aufmaße (`ObjektArt::Cadaufmass`, Punkte in
//! Zeichnungskoordinaten) in Aufmaße auf der DXF-Unterlage. Die alten
//! Fachobjekte bleiben unverändert lesbar; die Übernahme ist wiederholbar und
//! legt je CAD-Aufmaß höchstens ein Aufmaß an (feste Kennung).
use super::*;
use crate::{
    CoreServices,
    benutzer::Benutzer,
    cad::{Punkt, Rechteck, projektaufmass::CadProjektaufmass},
    domain::ProjectId,
    plattform::ObjektArt,
};

/// Seitenpunkt eines DXF-Zeichnungspunkts: Ursprung links oben am
/// Zeichnungsumfang, y nach unten, Einheit unverändert.
pub fn seitenpunkt(p: Punkt, grenzen: &Rechteck) -> Planpunkt {
    [p.0 - grenzen.min.0, grenzen.max.1 - p.1]
}

/// Ergebnis einer Übernahme; `vorhanden` zählt bereits übernommene CAD-Aufmaße.
#[derive(Debug, Clone, Default, Serialize, ts_rs::TS)]
pub struct CadUebernahme {
    pub uebernommen: Vec<String>,
    pub vorhanden: usize,
    pub uebersprungen: Vec<String>,
}

/// Kennung des Aufmaßes, das aus dem CAD-Aufmaß `cad_objekt` entsteht.
pub fn aufmass_id(cad_objekt: &str) -> Uuid {
    Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!("bauprojekt:planaufmass:cadaufmass:{cad_objekt}").as_bytes(),
    )
}

/// Wandelt ein CAD-Aufmaß in ein Aufmaß mit Bedeutung „unbestimmt“ um.
/// Geschlossene Linienzüge werden Polygone, zwei Punkte eine Strecke, offene
/// Linienzüge mit mehr Punkten eine Strecke je Abschnitt.
pub fn umwandeln(a: &CadProjektaufmass, datei: &str, grenzen: &Rechteck) -> Planaufmass {
    let ebene = Uuid::new_v5(
        &Uuid::NAMESPACE_URL,
        format!(
            "bauprojekt:planaufmass:cadaufmass-ebene:{}:{}",
            a.dokument.dokument_id.0, a.dokument.version
        )
        .as_bytes(),
    );
    let neu = |id, bezeichnung: String, geometrie| Planmarkierung {
        id,
        ebene_id: ebene,
        bezeichnung,
        bedeutung: Planbedeutung::Unbestimmt,
        geometrie,
        grundstueck_id: None,
        gebaeude_id: None,
        geschoss_id: None,
        ausschnitt_id: None,
        hoehenzone_von: None,
        kontur_von: None,
        flurstueck_id: None,
    };
    let mut markierungen = Vec::new();
    for m in &a.markierungen {
        let ps: Vec<Planpunkt> = m
            .punkte
            .iter()
            .map(|p| seitenpunkt(Punkt(p[0], p[1]), grenzen))
            .collect();
        if m.geschlossen && ps.len() >= 3 {
            markierungen.push(neu(
                m.id,
                m.bezeichnung.clone(),
                Plangeometrie::Polygon { punkte: ps },
            ));
        } else if ps.len() == 2 {
            markierungen.push(neu(
                m.id,
                m.bezeichnung.clone(),
                Plangeometrie::Strecke {
                    von: ps[0],
                    bis: ps[1],
                },
            ));
        } else {
            for (i, w) in ps.windows(2).enumerate() {
                let id = Uuid::new_v5(
                    &Uuid::NAMESPACE_URL,
                    format!("bauprojekt:planaufmass:cadaufmass-teil:{}:{i}", m.id).as_bytes(),
                );
                markierungen.push(neu(
                    id,
                    format!("{} · Teil {}", m.bezeichnung, i + 1),
                    Plangeometrie::Strecke {
                        von: w[0],
                        bis: w[1],
                    },
                ));
            }
        }
    }
    Planaufmass {
        bezeichnung: format!("{datei} · CAD-Aufmaß (übernommen)"),
        dokument: Some(a.dokument.clone()),
        seite: 1,
        kalibrierung: Some(Plankalibrierung::Einheit { einheit: a.einheit }),
        hoehenbezug: None,
        ebenen: vec![Planebene {
            id: ebene,
            bezeichnung: "CAD-Aufmaß (übernommen)".into(),
            sichtbar: true,
            gesperrt: false,
            darstellung: None,
        }],
        markierungen,
        ausschnitte: vec![],
        hintergruende: vec![],
        elemente: vec![],
    }
}

impl CoreServices {
    /// Legt für jedes CAD-Aufmaß des Projekts, das noch kein Aufmaß besitzt,
    /// ein Aufmaß auf derselben DXF-Dokumentversion an. Unlesbare oder leere
    /// Unterlagen werden mit Grund übersprungen, nicht still verworfen.
    pub async fn cadaufmass_uebernehmen(
        &self,
        pid: ProjectId,
        wer: &Benutzer,
    ) -> CoreResult<CadUebernahme> {
        let mut r = CadUebernahme::default();
        for f in self.fachobjekte(pid, ObjektArt::Cadaufmass).await? {
            let ziel = aufmass_id(&f.objekt);
            if self
                .fachobjekt_lesen(pid, ObjektArt::Planaufmass, ziel)
                .await?
                .is_some()
            {
                r.vorhanden += 1;
                continue;
            }
            let a: CadProjektaufmass = match serde_json::from_value(f.daten) {
                Ok(a) => a,
                Err(e) => {
                    r.uebersprungen.push(format!("{}: {e}", f.objekt));
                    continue;
                }
            };
            let (dokument, daten) = match self
                .load_version(a.dokument.dokument_id, a.dokument.version)
                .await
            {
                Ok(x) => x,
                Err(e) => {
                    r.uebersprungen.push(format!("{}: {e}", f.objekt));
                    continue;
                }
            };
            if dokument.project_id != pid {
                r.uebersprungen.push(format!(
                    "{}: Unterlage gehört nicht zum Projekt",
                    dokument.file_name
                ));
                continue;
            }
            let plan = match crate::cad::dxf::lesen(&daten) {
                Ok(p) => p,
                Err(e) => {
                    r.uebersprungen.push(format!("{}: {e}", dokument.file_name));
                    continue;
                }
            };
            let Some(grenzen) = plan.grenzen else {
                r.uebersprungen
                    .push(format!("{}: leere Zeichnung", dokument.file_name));
                continue;
            };
            let neu = umwandeln(&a, &dokument.file_name, &grenzen);
            let wert = serde_json::to_value(&neu).map_err(|e| fehler(e.to_string()))?;
            match self
                .fachobjekt_speichern(pid, ObjektArt::Planaufmass, ziel, 0, wert, wer)
                .await
            {
                Ok(_) => r.uebernommen.push(neu.bezeichnung),
                Err(e) => r.uebersprungen.push(format!("{}: {e}", neu.bezeichnung)),
            }
        }
        Ok(r)
    }
}
