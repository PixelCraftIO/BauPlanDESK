//! Übernahme gezeichneter Flurstücksgrenzen (Flurkarte, Liegenschaftskarte,
//! Lageplan) in die Grundstücksinformationen. Die Katasterfläche bleibt der
//! Rechenwert; die gezeichnete Fläche wird daneben gespeichert und ersetzt sie
//! nur, wenn das Flurstück ausdrücklich „Fläche aus Plan“ führt (noch kein
//! amtlicher Wert). Identität (Gemarkung, Flur, Nummer) und Abzüge bleiben
//! unberührt; sie werden weiterhin erfasst bzw. ermittelt.
use super::*;
use crate::{
    CoreServices,
    domain::ProjectId,
    grundstuecksinformationen::{FlurstuecksInfo, Grundstuecksinformationen},
};

/// Zielfelder der Flurstücke: je Flurstück die Fläche aus einer Markierung
/// mit Bedeutung „Flurstück“, die diesem Flurstück zugeordnet ist.
pub fn flurstueckziele(infos: &Grundstuecksinformationen) -> Vec<Planziel> {
    infos
        .flurstuecke
        .iter()
        .map(|f| Planziel {
            ziel: f.planziel(),
            titel: format!("Flurstück {} · Fläche", f.bezeichnung()),
            groesse: Planmessgroesse::Flaeche,
        })
        .collect()
}

/// Gemessene Flächen werden auf 0,01 m² gerundet gespeichert; Katasterflächen
/// sind ganzzahlig oder auf Quadratmeter genau, feinere Stellen täuschen Genauigkeit vor.
fn gerundet(wert: f64) -> f64 {
    (wert * 100.).round() / 100.
}

impl CoreServices {
    /// Entwurfsübernahme: löst alle Flurstücksbindungen auf, setzt die
    /// gezeichneten Flächen ein und prüft die Angaben. Gespeichert wird
    /// anschließend über die Projektgrenze (`projekt_bereich_speichern`).
    pub async fn planaufmass_flurstuecke(
        &self,
        pid: ProjectId,
        mut infos: Grundstuecksinformationen,
    ) -> CoreResult<Grundstuecksinformationen> {
        self.planwerte_flurstuecke(pid, &mut infos).await?;
        infos.validate()?;
        Ok(infos)
    }
    pub(crate) async fn planwerte_flurstuecke(
        &self,
        pid: ProjectId,
        infos: &mut Grundstuecksinformationen,
    ) -> CoreResult<()> {
        for f in &mut infos.flurstuecke {
            let Some(b) = f.planbindung.clone() else {
                // Ohne Bindung gibt es keine gezeichnete Fläche; ein eingesetzter
                // Wert bleibt als manuelle Angabe erhalten.
                f.planflaeche_m2 = None;
                f.flaeche_aus_plan = false;
                continue;
            };
            let benannt = |t: &str| fehler(format!("Flurstück {}: {t}", f.bezeichnung()));
            if b.ziel != f.planziel() || b.groesse != Planmessgroesse::Flaeche {
                return Err(benannt(
                    "Aufmaßverknüpfung gehört zu einem anderen Ziel oder einer anderen Messgröße",
                ));
            }
            let (_, m, wert) = self
                .planbindung_aufloesen(pid, &b)
                .await
                .map_err(|e| match e {
                    CoreError::InvalidInput(t) => benannt(&t),
                    e => e,
                })?;
            if m.bedeutung != Planbedeutung::Flurstueck {
                return Err(benannt(
                    "Planmarkierung ist nicht als „Flurstück“ gekennzeichnet",
                ));
            }
            if m.flurstueck_id.as_deref() != Some(f.id.as_str()) {
                return Err(benannt(
                    "Planmarkierung ist diesem Flurstück nicht zugeordnet; Zuordnung im Aufmaß prüfen",
                ));
            }
            let wert = gerundet(wert);
            f.planflaeche_m2 = Some(wert);
            // Eine noch offene Fläche liefert jetzt die Zeichnung.
            if f.flaeche_offen {
                f.flaeche_offen = false;
                f.flaeche_aus_plan = true;
            }
            if f.flaeche_aus_plan {
                f.flaeche_m2 = wert;
            }
        }
        Ok(())
    }
    /// Verbraucherprüfung: gespeicherte Flurstücke mit Aufmaßbindung müssen
    /// dem aktuellen Stand ihrer Quelle entsprechen. Ohne Bindungen keine Prüfung.
    pub async fn flurstueck_planquellen_pruefen(
        &self,
        pid: ProjectId,
        infos: &Grundstuecksinformationen,
    ) -> CoreResult<()> {
        if infos.flurstuecke.iter().all(|f| f.planbindung.is_none()) {
            return Ok(());
        }
        let mut neu = infos.clone();
        let hinweis = |t: String| {
            fehler(format!(
                "{t} Unter Grundlagen → Grundstücksinformationen „Aktuellen Aufmaßstand übernehmen“ und speichern."
            ))
        };
        if let Err(e) = self.planwerte_flurstuecke(pid, &mut neu).await {
            return Err(match e {
                CoreError::InvalidInput(t) => hinweis(t),
                e => e,
            });
        }
        let abweichend: Vec<&FlurstuecksInfo> = neu
            .flurstuecke
            .iter()
            .zip(&infos.flurstuecke)
            .filter(|(n, a)| n != a)
            .map(|(_, a)| a)
            .collect();
        if let Some(f) = abweichend.first() {
            return Err(hinweis(format!(
                "Flurstück {}: Fläche weicht vom Aufmaß ab.",
                f.bezeichnung()
            )));
        }
        Ok(())
    }
}
