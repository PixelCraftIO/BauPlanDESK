//! Verfahrensregeln zwischen Bearbeitungsstand und behördlichen Einträgen:
//! zurückgenommene Akten, Entscheidungsbescheide, Nebenbestimmungen,
//! Verlängerungen, Zurückstellung und Bauausführung. Bestehende Einträge
//! älterer Akten bleiben unverändert speicherbar; die Regeln gelten für neue
//! oder geänderte Einträge.
use super::{Entscheidung, Schrittart, Verfahrensakte, Verfahrensart, Verfahrensstatus, fehler};
use crate::CoreResult;
use std::collections::HashSet;
use uuid::Uuid;

/// Bescheid, der über den Antrag entscheidet. „Sonstiges“ und
/// Verlängerungsbescheide genügen nur bei Freistellung und sonstigen
/// Verfahren, die oft ohne förmliche Entscheidung enden. Eine
/// Teilbaugenehmigung entscheidet nicht über den ganzen Antrag.
pub(super) fn entscheidungsbescheid(a: &Verfahrensakte) -> bool {
    a.bescheide.iter().any(|b| {
        matches!(
            a.art,
            Verfahrensart::Freistellung | Verfahrensart::Sonstiges
        ) || (b.verlaengerung.is_none()
            && !b.teilbaugenehmigung
            && matches!(
                b.entscheidung,
                Entscheidung::Genehmigt
                    | Entscheidung::TeilweiseGenehmigt
                    | Entscheidung::Abgelehnt
            ))
    })
}

pub(super) fn pruefen(
    a: &Verfahrensakte,
    alt: Option<&Verfahrensakte>,
    eingereicht: bool,
) -> CoreResult<()> {
    let bescheid_alt = |id: Uuid| alt.and_then(|x| x.bescheide.iter().find(|b| b.id == id));
    let auflage_alt = |id: Uuid| {
        alt.and_then(|x| {
            x.bescheide
                .iter()
                .flat_map(|b| &b.auflagen)
                .find(|n| n.id == id)
        })
    };
    // Eine zurückgenommene Akte erhält keine neuen behördlichen Einträge.
    if a.status == Verfahrensstatus::Zurueckgenommen {
        let ids = |x: Option<&Verfahrensakte>| -> HashSet<Uuid> {
            let Some(x) = x else { return HashSet::new() };
            x.bescheide
                .iter()
                .map(|b| b.id)
                .chain(x.bescheide.iter().flat_map(|b| &b.auflagen).map(|n| n.id))
                .chain(x.nachforderungen.iter().map(|n| n.id))
                .chain(
                    x.nachforderungen
                        .iter()
                        .flat_map(|n| &n.punkte)
                        .map(|p| p.id),
                )
                .chain(x.schritte.iter().map(|s| s.id))
                .chain(x.abgaben.iter().map(|b| b.id))
                .collect()
        };
        if !ids(Some(a)).is_subset(&ids(alt)) {
            return Err(fehler(
                "Ein zurückgenommenes Verfahren erhält keine neuen Bescheide, Nebenbestimmungen, \
                 Nachforderungen, Anhörungen oder Abgabestände. Folgeverfahren anlegen.",
            ));
        }
    }
    if a.ruecknahme.is_some()
        && a.status != Verfahrensstatus::Zurueckgenommen
        && alt.map(|x| &x.ruecknahme) != Some(&a.ruecknahme)
    {
        return Err(fehler(
            "Eine Rücknahme wird nur zusammen mit dem Bearbeitungsstand „Zurückgenommen“ erfasst",
        ));
    }
    for b in &a.bescheide {
        let neu = bescheid_alt(b.id).is_none();
        if b.teilbaugenehmigung && (neu || bescheid_alt(b.id) != Some(b)) {
            if !a.art.baugenehmigung() || a.art == Verfahrensart::Teilbaugenehmigung {
                return Err(fehler(
                    "Eine Teilbaugenehmigung gehört zu einem laufenden Bauantrag, Änderungsantrag \
                     oder einer Nutzungsänderung",
                ));
            }
            if !matches!(
                b.entscheidung,
                Entscheidung::Genehmigt | Entscheidung::TeilweiseGenehmigt
            ) || b.verlaengerung.is_some()
            {
                return Err(fehler(
                    "Eine Teilbaugenehmigung ist eine genehmigende Entscheidung und keine \
                     Verlängerung",
                ));
            }
        }
        if neu
            && b.verlaengerung.is_none()
            && !b.teilbaugenehmigung
            && b.entscheidung != Entscheidung::Sonstiges
            && a.status != Verfahrensstatus::Entschieden
        {
            return Err(fehler(
                "Ein Entscheidungsbescheid beendet das Verfahren: Bearbeitungsstand „Entschieden“ \
                 zusammen mit dem Bescheid setzen.",
            ));
        }
        for n in &b.auflagen {
            let unveraendert = auflage_alt(n.id) == Some(n);
            if b.entscheidung == Entscheidung::Abgelehnt
                && n.art != super::Nebenbestimmungsart::Hinweis
                && auflage_alt(n.id).is_none()
            {
                return Err(fehler(
                    "Ein ablehnender Bescheid trägt keine Nebenbestimmungen; nur Hinweise erfassen",
                ));
            }
            if n.erledigt && !n.art.pflicht() && !unveraendert {
                return Err(fehler(format!(
                    "{} begründet keine erfüllbare Pflicht; sie erinnert mit Frist vor ihrem \
                     Ablauf und endet danach",
                    n.art.bezeichnung()
                )));
            }
        }
        if let Some(v) = &b.verlaengerung {
            let ziel = a
                .bescheide
                .iter()
                .find(|n| n.id == v.bescheid && n.id != b.id)
                .ok_or_else(|| {
                    fehler("Verlängerung benötigt den verlängerten Bescheid dieser Akte")
                })?;
            if b.entscheidung != Entscheidung::Genehmigt
                || !matches!(
                    ziel.entscheidung,
                    Entscheidung::Genehmigt | Entscheidung::TeilweiseGenehmigt
                )
            {
                return Err(fehler(
                    "Eine Verlängerung ist eine genehmigende Entscheidung über eine Genehmigung",
                ));
            }
            if v.bis <= ziel.datum || b.datum < ziel.datum {
                return Err(fehler(
                    "Verlängerungsbescheid und neues Ende liegen nach dem verlängerten Bescheid",
                ));
            }
            if a.bescheide
                .iter()
                .filter(|n| {
                    n.verlaengerung
                        .as_ref()
                        .is_some_and(|w| w.bescheid == v.bescheid)
                })
                .count()
                > 1
            {
                return Err(fehler(
                    "Ein Bescheid wird höchstens einmal verlängert; weitere Verlängerung auf den \
                     letzten Verlängerungsbescheid beziehen",
                ));
            }
            let mut gesehen = HashSet::new();
            let mut akt = Some(b);
            while let Some(n) = akt {
                if !gesehen.insert(n.id) {
                    return Err(fehler("Zyklische Verlängerung"));
                }
                akt = n
                    .verlaengerung
                    .as_ref()
                    .and_then(|w| a.bescheide.iter().find(|x| x.id == w.bescheid));
            }
        }
    }
    if a.pruefverfahren.is_some() && !a.art.baugenehmigung() {
        return Err(fehler(
            "Regulär oder vereinfacht gilt nur für Baugenehmigungsverfahren",
        ));
    }
    for s in &a.schritte {
        if let Some(id) = s.folgeschreiben {
            if s.art != Schrittart::Zurueckstellung {
                return Err(fehler(
                    "Ein Folgeschreiben beendet nur eine Zurückstellung; die Anhörung endet mit der Stellungnahme",
                ));
            }
            if id == s.kommunikation || !a.kommunikation.iter().any(|c| c.id == id && c.eingehend) {
                return Err(fehler(
                    "Folgeschreiben muss ein weiteres eingegangenes Schreiben dieser Akte sein",
                ));
            }
        }
    }
    if let Some(x) = &a.ausfuehrung
        && alt.map(|v| &v.ausfuehrung) != Some(&a.ausfuehrung)
    {
        let heute = crate::kalender::heute();
        if !eingereicht {
            return Err(fehler("Baubeginn setzt eine erfasste Einreichung voraus"));
        }
        if x.baubeginn > heute || x.fertigstellung.is_some_and(|f| f > heute) {
            return Err(fehler(
                "Baubeginn und Fertigstellung liegen nicht in der Zukunft",
            ));
        }
        if x.fertigstellung.is_some_and(|f| f < x.baubeginn) {
            return Err(fehler("Fertigstellung liegt nicht vor dem Baubeginn"));
        }
    }
    Ok(())
}
