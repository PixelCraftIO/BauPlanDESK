//! Die Vollgeschossprüfung liefert die Einstufung; Gebäude/GFZ bleiben Verbraucher.
use super::*;
use crate::{CoreServices, domain::ProjectId};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct Einstufungsquelle {
    pub nachweis_id: Uuid,
    pub kennzahl: String,
}
pub(crate) const VOLLGESCHOSSQUELLE_VERALTET: &str =
    "Vollgeschossquelle ist veraltet oder unvollständig";
impl CoreServices {
    pub(crate) async fn geschosseinstufung_aus_nachweis(
        &self,
        pid: ProjectId,
        quelle: &Einstufungsquelle,
        gebaeude: Uuid,
        geschoss: Uuid,
        grund: Option<Uuid>,
    ) -> CoreResult<(bool, String, Uuid)> {
        use crate::plattform::nachweise::NachweisEingabe as E;
        let alle = self.nachweise(pid).await?;
        let mut n = alle
            .iter()
            .find(|n| n.stand.id == quelle.nachweis_id)
            .ok_or_else(|| err("Vollgeschossnachweis gehört nicht zum Projekt"))?;
        if !n.aktuell || !n.stand.vollstaendig {
            // Die Quelle meint die Vollgeschossprüfung, nicht einen bestimmten
            // archivierten Stand: ein neuerer gültiger Stand derselben Prüfung
            // tritt an die Stelle des veralteten.
            let E::Fach { objekt, .. } = &n.stand.eingabe else {
                return Err(err("Quelle ist kein typisierter Vollgeschossnachweis"));
            };
            n = alle
                .iter()
                .filter(|k| {
                    k.aktuell
                        && k.stand.vollstaendig
                        && matches!(&k.stand.eingabe, E::Fach { objekt: o, .. } if o == objekt)
                })
                .max_by_key(|k| (k.stand.erstellt_am, k.stand.id))
                .ok_or_else(|| err(VOLLGESCHOSSQUELLE_VERALTET))?;
        }
        let crate::plattform::nachweise::NachweisEingabe::Fach { plan, .. } = &n.stand.eingabe
        else {
            return Err(err("Quelle ist kein typisierter Vollgeschossnachweis"));
        };
        if plan.modul != BerechnungsArt::Vollgeschosse
            || plan.gebaeude_id != Some(gebaeude)
            || grund.is_some_and(|g| plan.grundstueck_id != Some(g))
        {
            return Err(err(
                "Vollgeschossnachweis gehört zu einer anderen Gebäude-/Grundstückseinheit",
            ));
        }
        let input: vollgeschosse::Eingabe = decode(&plan.eingaben)?;
        let row = quelle
            .kennzahl
            .strip_prefix("geschoss:")
            .and_then(|id| input.geschosse.iter().find(|g| g.id == id))
            .ok_or_else(|| err("Keine einzelne Geschosseinstufung ausgewählt"))?;
        if row.gebaeude_geschoss_id != Some(geschoss) {
            return Err(err(
                "Prüfgeschoss ist diesem Gebäudegeschoss nicht zugeordnet",
            ));
        }
        let ergebnis: FachErgebnis = decode(&n.stand.ergebnis)?;
        let k = ergebnis
            .kennzahlen
            .iter()
            .find(|k| k.id == quelle.kennzahl)
            .and_then(|k| k.wert)
            .ok_or_else(|| err("Geschosseinstufung ist nicht ermittelt"))?;
        if ![0., 1.].contains(&k) {
            return Err(err("Geschosseinstufung ist kein Ja-/Nein-Ergebnis"));
        }
        Ok((
            k == 1.,
            format!(
                "{} · {} · {}",
                n.stand.bezeichnung,
                n.stand.erstellt_am.format("%d.%m.%Y"),
                plan.regelkontext.bezeichnung
            ),
            n.stand.id,
        ))
    }
}
