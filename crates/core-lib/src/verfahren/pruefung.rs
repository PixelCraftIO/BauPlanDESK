use super::*;
use crate::plattform::text;
use crate::zugriff::ProjektRecht;
use std::collections::HashSet;
fn behalten<T>(alt: &[T], neu: &[T], id: impl Fn(&T) -> Uuid) -> CoreResult<()> {
    if alt.iter().any(|a| !neu.iter().any(|n| id(a) == id(n))) {
        return Err(fehler(
            "Bestehende Akteneinträge dürfen nicht gelöscht werden. Berichtigung oder Folgeeintrag verwenden.",
        ));
    }
    Ok(())
}
pub(super) async fn pruefen(
    core: &CoreServices,
    p: ProjectId,
    vid: Uuid,
    a: &Verfahrensakte,
    alt: Option<&Verfahrensakte>,
    wer: &Benutzer,
    kontext: AktenCommit,
) -> CoreResult<()> {
    core.projekt_zugriff(p, wer, ProjektRecht::Bearbeiten)
        .await?;
    core.get_project(p).await?;
    crate::portale::validieren(a, alt, kontext)?;
    text(&a.bezeichnung, 500)?;
    // In der Vorbereitung darf die Behörde noch offen sein (Akte entsteht mit
    // dem Projekt); vor jeder Abgabe wird sie verlangt.
    let behoerde_offen = a.status == Verfahrensstatus::Vorbereitung
        && a.abgaben.is_empty()
        && a.externe_einreichungen.is_empty()
        && a.behoerde.name.trim().is_empty();
    if !behoerde_offen {
        text(&a.behoerde.name, 500)
            .map_err(|_| fehler("Bauaufsichtsbehörde eintragen (Name mit 1 bis 500 Zeichen)"))?;
    }
    if a.behoerde.id.is_nil() || a.behoerde.kennung.len() > 500 {
        return Err(fehler(
            "Behörde benötigt eine Kennung und begrenzte Metadaten",
        ));
    }
    if a.aktenzeichen.len() > 500
        || a.behoerde.kontakt.len() > 4000
        || a.ereignisse.len() > 2000
        || a.abgaben.len() > 200
        || a.kommunikation.len() > 1000
        || a.nachforderungen.len() > 500
        || a.schritte.len() > 100
        || a.anforderungen.len() > 500
        || a.bescheide.len() > 100
        || a.antragsangaben.vorbescheidfragen.len() > 50
        || a.antragsangaben
            .vorbescheidfragen
            .iter()
            .any(|f| f.frage.len() > 16000)
    {
        return Err(fehler(
            "Verfahrensakte enthält zu viele Einträge oder zu lange Texte",
        ));
    }
    if a.art == Verfahrensart::Widerspruch {
        let v = a.vorgaenger.ok_or_else(|| {
            fehler("Ein Widerspruch benötigt das Vorgängerverfahren mit dem angefochtenen Bescheid")
        })?;
        if core.verfahrensakte(p, v).await?.1.bescheide.is_empty() {
            return Err(fehler(
                "Das Vorgängerverfahren enthält keinen Bescheid, gegen den sich der Widerspruch \
                 richten kann",
            ));
        }
    }
    let mut besucht = HashSet::from([vid]);
    let mut vorgaenger = a.vorgaenger;
    while let Some(v) = vorgaenger {
        if !besucht.insert(v) || besucht.len() > 200 {
            return Err(fehler("Vorgängerverfahren dürfen keinen Kreis bilden"));
        }
        vorgaenger = core.verfahrensakte(p, v).await?.1.vorgaenger;
    }
    let mut ids = HashSet::new();
    let mut add = |id: Uuid| -> CoreResult<()> {
        if id.is_nil() || !ids.insert(id) {
            Err(fehler("Akteneinträge benötigen eindeutige Kennungen"))
        } else {
            Ok(())
        }
    };
    for id in a
        .kommunikation
        .iter()
        .map(|v| v.id)
        .chain(a.anforderungen.iter().map(|v| v.id))
        .chain(a.abgaben.iter().map(|v| v.id))
        .chain(a.bescheide.iter().map(|v| v.id))
        .chain(a.nachforderungen.iter().map(|v| v.id))
        .chain(a.schritte.iter().map(|v| v.id))
        .chain(a.dokumentbeziehungen.iter().map(|v| v.id))
        .chain(a.ereignisse.iter().map(|v| v.id))
        .chain(a.antragsangaben.vorbescheidfragen.iter().map(|v| v.id))
        .chain(a.abweichungen.iter().map(|v| v.id))
        .chain(a.gebuehren.iter().map(|v| v.id))
        .chain(a.unterschriften.iter().map(|v| v.id))
        .chain(a.ablehnungsgruende.iter().map(|v| v.id))
        .chain(a.nachbarbekanntgaben.iter().map(|v| v.id))
        .chain(a.bautechnische_nachweise.iter().map(|v| v.id))
        .chain(a.schritte.iter().flat_map(|s| &s.gruende).map(|v| v.id))
    {
        add(id)?;
    }
    for n in &a.nachforderungen {
        for p in &n.punkte {
            add(p.id)?;
        }
    }
    for b in &a.bescheide {
        for p in &b.auflagen {
            add(p.id)?;
        }
    }
    for b in &a.abgaben {
        for e in &b.versand {
            add(e.id)?;
        }
    }
    for e in &a.abweichungen {
        for n in &e.nachbarn {
            add(n.id)?;
        }
    }
    let mut dokumente = Vec::new();
    let mut benutzer = Vec::new();
    benutzer.extend(a.verantwortlich);
    let mut bezuege = Vec::new();
    for c in &a.kommunikation {
        text(&c.betreff, 500)?;
        if c.inhalt.len() > 100000 || c.beteiligte.len() > 4000 {
            return Err(fehler("Kommunikation zu umfangreich"));
        }
        dokumente.extend(c.original.iter().cloned());
        bezuege.extend(c.bezuege.iter());
    }
    for n in &a.nachforderungen {
        if !a
            .kommunikation
            .iter()
            .any(|c| c.id == n.kommunikation && c.eingehend)
        {
            return Err(fehler(
                "Nachforderung benötigt zugeordnetes eingegangenes Schreiben oder Gespräch",
            ));
        }
        if n.punkte.is_empty() || n.punkte.len() > 200 {
            return Err(fehler("Nachforderung benötigt 1 bis 200 Punkte"));
        }
        for f in &n.punkte {
            text(&f.titel, 500)?;
            if f.antwort.len() > 16000 || f.grundlage.len() > 4000 {
                return Err(fehler("Antwort oder Grundlage zu lang"));
            }
            benutzer.extend(f.verantwortlich);
            dokumente.extend(f.dokumente.iter().cloned());
            frist(&f.frist, &mut dokumente)?;
            for id in &f.abgaben {
                a.abgaben
                    .iter()
                    .find(|b| b.id == *id)
                    .ok_or_else(|| fehler("Unbekannter Antwort-Abgabestand"))?;
            }
            let historisch = kontext == AktenCommit::Portal
                && alt.is_some_and(|a| {
                    a.nachforderungen
                        .iter()
                        .flat_map(|n| &n.punkte)
                        .any(|v| v == f)
                });
            if !historisch
                && !f.abgaben.is_empty()
                && f.dokumente.iter().any(|d| {
                    !f.abgaben
                        .iter()
                        .filter_map(|id| a.abgaben.iter().find(|b| b.id == *id))
                        .filter(|b| {
                            !matches!(f.status, Punktstatus::Eingereicht | Punktstatus::Bestaetigt)
                                || matches!(
                                    abgabe::versandstand(b),
                                    Some(
                                        abgabe::Versandstatus::Versendet
                                            | abgabe::Versandstatus::Eingegangen
                                    )
                                )
                        })
                        .any(|b| b.dokumente.iter().any(|e| &e.dokument == d))
                })
            {
                return Err(fehler(
                    "Antwortunterlagen fehlen in den zugeordneten Abgabeständen",
                ));
            }
            if !historisch
                && matches!(f.status, Punktstatus::Eingereicht | Punktstatus::Bestaetigt)
                && !f.abgaben.iter().any(|id| {
                    a.abgaben.iter().find(|b| b.id == *id).is_some_and(|b| {
                        matches!(
                            abgabe::versandstand(b),
                            Some(
                                abgabe::Versandstatus::Versendet
                                    | abgabe::Versandstatus::Eingegangen
                            )
                        )
                    })
                })
            {
                return Err(fehler(
                    "Eingereichter Forderungspunkt benötigt einen tatsächlich versendeten Antwortstand",
                ));
            }
            if f.status == Punktstatus::Bestaetigt
                && !a
                    .kommunikation
                    .iter()
                    .any(|c| Some(c.id) == f.bestaetigung && c.eingehend && c.id != n.kommunikation)
            {
                return Err(fehler(
                    "Bestätigung benötigt eine zugeordnete behördliche Rückmeldung",
                ));
            }
            if f.status == Punktstatus::Entfallen {
                text(&f.antwort, 16000)?;
            }
        }
    }
    for v in &a.schritte {
        let schreiben = a
            .kommunikation
            .iter()
            .find(|c| c.id == v.kommunikation && c.eingehend)
            .ok_or_else(|| {
                fehler(format!(
                    "{} benötigt das eingegangene Schreiben der Behörde in dieser Akte",
                    v.art.bezeichnung()
                ))
            })?;
        if let Some(id) = v.stellungnahme
            && !a.kommunikation.iter().any(|c| c.id == id && !c.eingehend)
        {
            return Err(fehler(
                "Stellungnahme muss eine ausgehende Kommunikation dieser Akte sein",
            ));
        }
        if v.art == Schrittart::Zurueckstellung
            && v.frist.as_ref().is_none_or(|f| f.datum <= schreiben.datum)
        {
            return Err(fehler(
                "Zurückstellung benötigt ihr Ende als Frist nach dem Datum des Schreibens",
            ));
        }
        benutzer.extend(v.verantwortlich);
        frist(&v.frist, &mut dokumente)?;
        beanstandungen(&v.gruende, a)?;
    }
    frist(&a.bearbeitungsfrist, &mut dokumente)?;
    for g in &a.gebuehren {
        if !a
            .kommunikation
            .iter()
            .any(|c| c.id == g.kommunikation && c.eingehend)
        {
            return Err(fehler(
                "Gebührenbescheid benötigt das eingegangene Schreiben der Behörde in dieser Akte",
            ));
        }
        if g.betrag_cent.is_some_and(|b| b < 0) || g.verwendungszweck.chars().count() > 500 {
            return Err(fehler("Ungültige Angaben zum Gebührenbescheid"));
        }
        if g.bezahlt.is_some_and(|d| d > crate::kalender::heute()) {
            return Err(fehler("Zahlungsdatum darf nicht in der Zukunft liegen"));
        }
        benutzer.extend(g.verantwortlich);
        frist(&g.frist, &mut dokumente)?;
    }
    for r in &a.anforderungen {
        text(&r.titel, 500)?;
        if r.quelle.len() > 4000 || r.grund.len() > 4000 {
            return Err(fehler("Anforderungsgrundlage zu lang"));
        }
        dokumente.extend(r.dokumente.iter().cloned());
        benutzer.extend(r.verantwortlich);
        if r.erforderlich == Erforderlichkeit::NichtErforderlich {
            text(&r.grund, 4000)?;
        }
        if r.freigegeben {
            if r.erforderlich == Erforderlichkeit::Ungeklaert
                || (r.erforderlich == Erforderlichkeit::Erforderlich && r.dokumente.is_empty())
            {
                return Err(fehler(
                    "Ungeklärte oder unbelegte Anforderungen können nicht freigegeben werden",
                ));
            }
            if !alt.is_some_and(|a| a.anforderungen.iter().any(|v| v == r)) {
                core.berechtigung_pruefen(wer, Recht::ProjektAnlegen)
                    .await?;
            }
        }
    }
    abweichungen::eintraege_pruefen(a, alt, &mut dokumente)?;
    landesrecht::unterschriften_pruefen(a, &mut dokumente)?;
    ausfuehrung::pruefen(a, &mut dokumente)?;
    for g in &a.ablehnungsgruende {
        let b = a
            .bescheide
            .iter()
            .find(|b| b.id == g.bescheid)
            .ok_or_else(|| fehler("Ablehnungsgrund benötigt den Bescheid dieser Akte"))?;
        if !matches!(
            b.entscheidung,
            Entscheidung::Abgelehnt | Entscheidung::TeilweiseGenehmigt
        ) {
            return Err(fehler(
                "Ablehnungsgründe gehören zu einem ablehnenden oder teilweise genehmigenden Bescheid",
            ));
        }
        if let Some(f) = g.folgeverfahren
            && f != vid
        {
            core.verfahrensakte(p, f).await?;
        }
    }
    beanstandungen(
        &a.ablehnungsgruende
            .iter()
            .map(|g| Beanstandung {
                id: g.id,
                text: g.text.clone(),
                grundlage: g.grundlage.clone(),
                reaktion: g.reaktion.clone(),
                abweichung: g.abweichung,
            })
            .collect::<Vec<_>>(),
        a,
    )?;
    for n in &a.nachbarbekanntgaben {
        text(&n.name, 500)?;
        let b = a
            .bescheide
            .iter()
            .find(|b| b.id == n.bescheid)
            .ok_or_else(|| fehler("Bekanntgabe benötigt den Bescheid dieser Akte"))?;
        if n.datum < b.datum || n.datum > crate::kalender::heute() {
            return Err(fehler(
                "Bekanntgabe liegt nicht vor dem Bescheid und nicht in der Zukunft",
            ));
        }
        if n.angefochten
            .is_some_and(|d| d < n.datum || d > crate::kalender::heute())
        {
            return Err(fehler(
                "Anfechtung liegt nach der Bekanntgabe und nicht in der Zukunft",
            ));
        }
        dokumente.extend(n.beleg.iter().cloned());
    }
    let heute = crate::kalender::heute();
    zukunft_pruefen(a, alt, heute)?;
    externe_einreichung::pruefen(a, alt, kontext, heute, &mut dokumente)?;
    // Tatsächlich eingereicht: versendeter Abgabestand oder nachgetragene
    // externe Einreichung, jeweils nicht in der Zukunft.
    let eingereicht = externe_einreichung::eingereicht(a, heute);
    let vorgelegt = externe_einreichung::eingereichte_dokumente(a, heute);
    for b in &a.bescheide {
        // Neue Bescheide setzen eine tatsächliche Einreichung voraus und
        // genehmigen nur Unterlagen, die der Behörde vorlagen. Ältere Bescheide
        // bleiben unverändert gültig.
        if !alt.is_some_and(|a| a.bescheide.iter().any(|v| v.id == b.id)) {
            if !eingereicht {
                return Err(fehler(
                    "Ein Bescheid setzt eine Einreichung voraus. Versand im Register Abgaben erfassen oder eine externe Einreichung nachtragen.",
                ));
            }
            if b.genehmigte_unterlagen
                .iter()
                .any(|d| !vorgelegt.contains(&d))
            {
                return Err(fehler(
                    "Genehmigte Unterlagen müssen aus einem versendeten Abgabestand oder einer externen Einreichung stammen",
                ));
            }
        }
        dokumente.push(b.original.clone());
        dokumente.extend(b.genehmigte_unterlagen.iter().cloned());
        if matches!(
            b.entscheidung,
            Entscheidung::Genehmigt | Entscheidung::TeilweiseGenehmigt
        ) && b.genehmigte_unterlagen.is_empty()
            && b.verlaengerung.is_none()
        {
            return Err(fehler(
                "Genehmigung benötigt den ausdrücklich zugeordneten genehmigten Unterlagenstand",
            ));
        }
        if b.zugestellt.is_some_and(|d| d < b.datum) {
            return Err(fehler("Zustellung darf nicht vor dem Bescheiddatum liegen"));
        }
        if let Some(v) = b.ersetzt {
            if v == b.id
                || !a.bescheide.iter().any(|b| b.id == v)
                || a.bescheide.iter().filter(|b| b.ersetzt == Some(v)).count() > 1
            {
                return Err(fehler("Ungültige Bescheidfolge"));
            }
            let mut seen = HashSet::new();
            let mut akt = Some(b.id);
            while let Some(id) = akt {
                if !seen.insert(id) {
                    return Err(fehler("Zyklische Bescheidfolge"));
                }
                akt = a
                    .bescheide
                    .iter()
                    .find(|b| b.id == id)
                    .and_then(|b| b.ersetzt);
            }
        }
        for f in &b.auflagen {
            text(&f.titel, 500)?;
            if f.ausloeser.len() > 4000 {
                return Err(fehler("Auflagenauslöser zu lang"));
            }
            benutzer.extend(f.verantwortlich);
            dokumente.extend(f.nachweise.iter().cloned());
            frist(&f.frist, &mut dokumente)?;
            if f.art == Nebenbestimmungsart::Hinweis && f.erledigt {
                return Err(fehler(
                    "Ein Hinweis begründet keine Pflicht und wird nicht erfüllt",
                ));
            }
            if f.art == Nebenbestimmungsart::Befristung && f.frist.is_none() {
                return Err(fehler("Befristung benötigt ihr Ablaufdatum als Frist"));
            }
            if f.erledigt && f.nachweise.is_empty() {
                return Err(fehler("Erledigte Auflage benötigt einen Erfüllungsbeleg"));
            }
        }
    }
    if a.status == Verfahrensstatus::Entschieden && a.bescheide.is_empty() {
        return Err(fehler("Entschiedenes Verfahren benötigt einen Bescheid"));
    }
    status_pruefen(a, alt, eingereicht)?;
    regeln::pruefen(a, alt, eingereicht)?;
    chronik::berichtigungen_pruefen(alt, a)?;
    for d in &a.dokumentbeziehungen {
        text(&d.grund, 4000)?;
        if d.vorher == d.nachher {
            return Err(fehler("Dokument kann sich nicht selbst ersetzen"));
        }
        dokumente.extend([d.vorher.clone(), d.nachher.clone()]);
        bezuege.extend(d.bezug.iter());
    }
    for b in &a.abgaben {
        let existiert = alt.is_some_and(|a| a.abgaben.iter().any(|v| v.id == b.id));
        if !existiert && kontext != AktenCommit::Abgabe {
            return Err(fehler("Abgabestände über Zusammenstellung festhalten"));
        }
        let bisher = alt
            .and_then(|a| a.abgaben.iter().find(|v| v.id == b.id))
            .map(|v| v.versand.len())
            .unwrap_or(0);
        for e in b.versand.iter().skip(bisher) {
            if e.datum < crate::kalender::datum_deutsch(b.erstellt_am) {
                return Err(fehler(
                    "Ein Versandereignis darf nicht vor dem Festhalten des Abgabestands liegen. Eine frühere Einreichung als externe Einreichung nachtragen.",
                ));
            }
        }
        for e in &b.versand {
            text(&e.notiz, 4000)?;
            dokumente.extend(e.beleg.iter().cloned());
            if e.status == abgabe::Versandstatus::Eingegangen && e.beleg.is_none() {
                return Err(fehler("Bestätigter Eingang benötigt einen Eingangsbeleg"));
            }
            if let Some(id) = e.berichtigt
                && (!b
                    .versand
                    .iter()
                    .take_while(|v| v.id != e.id)
                    .any(|v| v.id == id)
                    || b.versand
                        .iter()
                        .filter(|v| v.berichtigt == Some(id))
                        .count()
                        > 1)
            {
                return Err(fehler(
                    "Berichtigung benötigt ein früheres, noch nicht berichtigtes Versandereignis",
                ));
            }
        }
    }
    for e in &a.ereignisse {
        text(&e.text, 4000)?;
        dokumente.extend(e.beleg.iter().cloned());
        if let Some(id) = e.berichtigt
            && !a
                .ereignisse
                .iter()
                .take_while(|v| v.id != e.id)
                .any(|v| v.id == id)
        {
            return Err(fehler("Berichtigung benötigt ein früheres Aktenereignis"));
        }
    }
    for r in bezuege {
        match r {
            Bezug::Dokument { dokument } => dokumente.push(dokument.clone()),
            Bezug::Nachweis { id } => {
                if !core.nachweise(p).await?.iter().any(|n| n.stand.id == *id) {
                    return Err(fehler("Nachweis gehört nicht zum Projekt"));
                }
            }
            Bezug::Kommunikation { id } if a.kommunikation.iter().any(|n| n.id == *id) => {}
            Bezug::Anforderung { id } if a.anforderungen.iter().any(|n| n.id == *id) => {}
            Bezug::Abgabe { id } if a.abgaben.iter().any(|n| n.id == *id) => {}
            Bezug::Bescheid { id } if a.bescheide.iter().any(|n| n.id == *id) => {}
            Bezug::Nachforderung { id, punkt }
                if a.nachforderungen
                    .iter()
                    .any(|n| n.id == *id && n.punkte.iter().any(|p| p.id == *punkt)) => {}
            Bezug::Auflage { bescheid, id }
                if a.bescheide
                    .iter()
                    .any(|b| b.id == *bescheid && b.auflagen.iter().any(|p| p.id == *id)) => {}
            _ => return Err(fehler("Ungültiger Verfahrensbezug")),
        }
    }
    for id in benutzer {
        let b = core
            .repos
            .benutzer
            .get(crate::benutzer::BenutzerId(id))
            .await?;
        if !b.aktiv {
            return Err(fehler("Verantwortlicher Benutzer ist inaktiv"));
        }
    }
    let mut gesehen = HashSet::new();
    for d in dokumente {
        if gesehen.insert((d.dokument_id, d.version))
            && !core
                .document_history(d.dokument_id)
                .await?
                .iter()
                .any(|v| v.project_id == p && v.version == d.version)
        {
            return Err(fehler("Dokumentversion gehört nicht zum Projekt"));
        }
    }
    if let Some(alt) = alt {
        behalten(&alt.kommunikation, &a.kommunikation, |v| v.id)?;
        behalten(&alt.nachforderungen, &a.nachforderungen, |v| v.id)?;
        behalten(&alt.anforderungen, &a.anforderungen, |v| v.id)?;
        behalten(&alt.bescheide, &a.bescheide, |v| v.id)?;
        behalten(&alt.schritte, &a.schritte, |v| v.id)?;
        behalten(&alt.gebuehren, &a.gebuehren, |v| v.id)?;
        behalten(&alt.ablehnungsgruende, &a.ablehnungsgruende, |v| v.id)?;
        behalten(&alt.nachbarbekanntgaben, &a.nachbarbekanntgaben, |v| v.id)?;
        for v in &alt.ablehnungsgruende {
            let n = a
                .ablehnungsgruende
                .iter()
                .find(|n| n.id == v.id)
                .expect("geprüft");
            if n.text != v.text || n.grundlage != v.grundlage || n.bescheid != v.bescheid {
                return Err(fehler(
                    "Wortlaut eines gespeicherten Ablehnungsgrunds bleibt unverändert",
                ));
            }
        }
        for v in alt.schritte.iter().flat_map(|s| &s.gruende) {
            let n = a
                .schritte
                .iter()
                .flat_map(|s| &s.gruende)
                .find(|n| n.id == v.id)
                .ok_or_else(|| fehler("Beanstandungen werden nicht gelöscht"))?;
            if n.text != v.text || n.grundlage != v.grundlage {
                return Err(fehler(
                    "Wortlaut und Grundlage einer gespeicherten Beanstandung bleiben unverändert",
                ));
            }
        }
        for v in &alt.schritte {
            let n = a.schritte.iter().find(|n| n.id == v.id).expect("geprüft");
            if n.art != v.art || n.kommunikation != v.kommunikation {
                return Err(fehler(
                    "Art und Schreiben eines erfassten Verfahrensschritts bleiben unverändert",
                ));
            }
        }
        for c in &alt.kommunikation {
            let n = a
                .kommunikation
                .iter()
                .find(|n| n.id == c.id)
                .expect("geprüft");
            let mut kopie = n.clone();
            kopie.bezuege = c.bezuege.clone();
            if &kopie != c || c.bezuege.iter().any(|r| !n.bezuege.contains(r)) {
                return Err(fehler(
                    "Kommunikationsoriginal bleibt unverändert. Berichtigenden Folgeeintrag anlegen.",
                ));
            }
        }
        for n in &alt.nachforderungen {
            let neu = a
                .nachforderungen
                .iter()
                .find(|v| v.id == n.id)
                .expect("geprüft");
            if n.kommunikation != neu.kommunikation {
                return Err(fehler("Nachforderungsoriginal bleibt unverändert"));
            }
            behalten(&n.punkte, &neu.punkte, |v| v.id)?;
            // Wortlaut und Grundlage stammen aus dem behördlichen Schreiben.
            for v in &n.punkte {
                let p = neu.punkte.iter().find(|p| p.id == v.id).expect("geprüft");
                if p.titel != v.titel || p.grundlage != v.grundlage {
                    return Err(fehler(
                        "Wortlaut und Grundlage eines gespeicherten Forderungspunkts bleiben unverändert. Abweichenden Wortlaut als neuen Punkt erfassen und den bisherigen begründet als entfallen führen.",
                    ));
                }
            }
        }
        for b in &alt.bescheide {
            let n = a.bescheide.iter().find(|n| n.id == b.id).expect("geprüft");
            let mut kopie = n.clone();
            kopie.auflagen = b.auflagen.clone();
            // Ein später bekanntes Zustelldatum darf nachgetragen, ein erfasstes
            // nur mit begründeter Berichtigung geändert, aber nicht entfernt werden
            // (chronik::berichtigungen_pruefen).
            if b.zugestellt.is_none() || n.zugestellt.is_some() {
                kopie.zugestellt = b.zugestellt;
            }
            if &kopie != b {
                return Err(fehler(
                    "Bescheiddaten bleiben unverändert. Folgebescheid anlegen.",
                ));
            }
            behalten(&b.auflagen, &n.auflagen, |v| v.id)?;
            // Wortlaut und Auslöser stammen aus dem Bescheid.
            for v in &b.auflagen {
                let f = n.auflagen.iter().find(|f| f.id == v.id).expect("geprüft");
                if f.titel != v.titel || f.ausloeser != v.ausloeser {
                    return Err(fehler(
                        "Wortlaut und Auslöser einer gespeicherten Nebenbestimmung bleiben unverändert. Ein abweichender Wortlaut ist ein Folgebescheid.",
                    ));
                }
            }
        }
        if a.ereignisse.len() < alt.ereignisse.len()
            || a.ereignisse[..alt.ereignisse.len()] != alt.ereignisse
        {
            return Err(fehler("Aktenereignisse sind unveränderlich"));
        }
        for b in &alt.abgaben {
            let neu = a
                .abgaben
                .iter()
                .find(|n| n.id == b.id)
                .ok_or_else(|| fehler("Abgabestand darf nicht entfernt werden"))?;
            let mut kopie = neu.clone();
            kopie.versand = b.versand.clone();
            kopie.versandstatus = b.versandstatus.clone();
            kopie.lage = b.lage.clone();
            if &kopie != b
                || neu.versand.len() < b.versand.len()
                || neu.versand[..b.versand.len()] != b.versand
            {
                return Err(fehler(
                    "Abgabestände und frühere Versandereignisse bleiben unverändert",
                ));
            }
        }
        behalten(&alt.dokumentbeziehungen, &a.dokumentbeziehungen, |v| v.id)?;
        if alt
            .dokumentbeziehungen
            .iter()
            .any(|d| !a.dokumentbeziehungen.contains(d))
        {
            return Err(fehler("Frühere Ersatzbeziehungen bleiben unverändert"));
        }
    }
    Ok(())
}
/// Gilt die Akte als eingereicht? Versendeter oder eingegangener Abgabestand
/// oder nachgetragene externe Einreichung, jeweils nicht in der Zukunft.
pub(super) fn ist_eingereicht(a: &Verfahrensakte) -> bool {
    externe_einreichung::eingereicht(a, crate::kalender::heute())
}
/// Übergangsregeln des Verfahrensstatus. Bedingungen eines Status werden beim
/// Wechsel geprüft. Bleibt der Status, darf eine Speicherung einen bisher
/// erfüllten Stand nicht ungültig machen (etwa Rückweisung der einzigen
/// Abgabe oder Entfernen der Vollständigkeitsbestätigung); ältere, schon
/// widersprüchliche Akten bleiben bearbeitbar.
fn status_pruefen(
    a: &Verfahrensakte,
    alt: Option<&Verfahrensakte>,
    eingereicht: bool,
) -> CoreResult<()> {
    use Verfahrensstatus::*;
    let schreiben = |id: Uuid, eingehend: bool| {
        a.kommunikation
            .iter()
            .any(|c| c.id == id && (c.eingehend || !eingehend))
    };
    if let Some(v) = &a.vollstaendigkeit
        && !schreiben(v.kommunikation, true)
    {
        return Err(fehler(
            "Vollständigkeit benötigt ein eingegangenes Schreiben der Behörde in dieser Akte",
        ));
    }
    if let Some(r) = &a.ruecknahme
        && !schreiben(r.kommunikation, false)
    {
        return Err(fehler(
            "Rücknahme benötigt das zugehörige Schreiben in dieser Akte",
        ));
    }
    let vorher = alt.map(|a| &a.status);
    if vorher == Some(&a.status) {
        let alt = alt.expect("gleicher Status setzt alten Stand voraus");
        if status_bedingung(alt, ist_eingereicht(alt)).is_ok()
            && let Err(e) = status_bedingung(a, eingereicht)
        {
            return Err(fehler(format!(
                "{} Bearbeitungsstand zusammen mit dieser Änderung anpassen.",
                match e {
                    CoreError::InvalidInput(m) => m,
                    e => e.to_string(),
                }
            )));
        }
        return Ok(());
    }
    if let Some(v) = vorher {
        if v.abgeschlossen() {
            return Err(fehler(format!(
                "Ein {} Verfahren wird nicht wieder geöffnet. Folgeverfahren anlegen.",
                if *v == Entschieden {
                    "entschiedenes"
                } else {
                    "zurückgenommenes"
                }
            )));
        }
        // Ohne versendeten Stand (etwa nach Rückweisung der einzigen Abgabe)
        // ist der Antrag nicht eingereicht und darf zurück in die Vorbereitung.
        if a.status == Vorbereitung && eingereicht {
            return Err(fehler(
                "Ein eingereichtes Verfahren kehrt nicht in die Vorbereitung zurück",
            ));
        }
    }
    status_bedingung(a, eingereicht)
}
/// Voraussetzungen des aktuellen Bearbeitungsstands.
fn status_bedingung(a: &Verfahrensakte, eingereicht: bool) -> CoreResult<()> {
    use Verfahrensstatus::*;
    match a.status {
        Vorbereitung => {}
        Zurueckgenommen if a.ruecknahme.is_none() => {
            return Err(fehler("Zurücknahme benötigt Datum und Rücknahmeschreiben"));
        }
        Zurueckgenommen => {}
        _ if !eingereicht => {
            return Err(fehler(format!(
                "Status „{}“ setzt einen versendeten Abgabestand oder eine nachgetragene externe Einreichung voraus.",
                a.status.bezeichnung()
            )));
        }
        Unvollstaendig
            if !a
                .nachforderungen
                .iter()
                .flat_map(|n| &n.punkte)
                .any(|p| !matches!(p.status, Punktstatus::Bestaetigt | Punktstatus::Entfallen)) =>
        {
            return Err(fehler(
                "Status „Unvollständig“ benötigt eine Nachforderung mit offenen Punkten.",
            ));
        }
        Vollstaendig if a.vollstaendigkeit.is_none() => {
            return Err(fehler(
                "Status „Vollständig“ benötigt die behördliche Bestätigung mit Datum.",
            ));
        }
        Entschieden if !regeln::entscheidungsbescheid(a) => {
            return Err(fehler(
                "Status „Entschieden“ benötigt einen Bescheid mit Genehmigung oder Ablehnung; \
                 eine sonstige Mitteilung oder Verlängerung genügt nicht.",
            ));
        }
        _ => {}
    }
    Ok(())
}
pub(super) fn beanstandungen(g: &[Beanstandung], a: &Verfahrensakte) -> CoreResult<()> {
    if g.len() > 200 {
        return Err(fehler("Höchstens 200 Beanstandungen je Eintrag"));
    }
    for b in g {
        text(&b.text, 4000)?;
        if b.grundlage.chars().count() > 4000 || b.reaktion.chars().count() > 16000 {
            return Err(fehler("Beanstandung zu umfangreich"));
        }
        if let Some(id) = b.abweichung
            && !a.abweichungen.iter().any(|e| e.id == id)
        {
            return Err(fehler(
                "Beanstandung verweist auf eine unbekannte Abweichung",
            ));
        }
    }
    Ok(())
}
fn frist(f: &Option<Frist>, docs: &mut Vec<Dokumentbezug>) -> CoreResult<()> {
    if let Some(f) = f {
        text(&f.ausloeser, 4000)?;
        text(&f.grund, 4000)?;
        if f.erinnerung_tage > 365 {
            return Err(fehler("Erinnerung höchstens 365 Tage im Voraus"));
        }
        docs.extend(f.beleg.iter().cloned());
    }
    Ok(())
}
/// Fachliche Daten tatsächlicher Vorgänge liegen nicht nach dem heutigen
/// Ortsdatum. Geprüft werden nur neue oder geänderte Angaben, damit ältere
/// Akten bearbeitbar bleiben. Fristen dürfen in der Zukunft liegen.
fn zukunft_pruefen(
    a: &Verfahrensakte,
    alt: Option<&Verfahrensakte>,
    heute: NaiveDate,
) -> CoreResult<()> {
    let zukunft = |was: &str| {
        Err(fehler(format!(
            "{was} darf nicht in der Zukunft liegen (heute ist der {})",
            heute.format("%d.%m.%Y")
        )))
    };
    for c in &a.kommunikation {
        if c.datum > heute && !alt.is_some_and(|a| a.kommunikation.iter().any(|v| v.id == c.id)) {
            return zukunft("Das Datum der Kommunikation");
        }
    }
    for b in &a.abgaben {
        let bisher = alt
            .and_then(|a| a.abgaben.iter().find(|v| v.id == b.id))
            .map(|v| v.versand.len())
            .unwrap_or(0);
        if b.versand.iter().skip(bisher).any(|e| e.datum > heute) {
            return zukunft("Ein Versand-, Eingangs- oder Rückweisungsdatum");
        }
    }
    for b in &a.bescheide {
        let vorher = alt.and_then(|a| a.bescheide.iter().find(|v| v.id == b.id));
        if vorher.is_none() && b.datum > heute {
            return zukunft("Das Bescheiddatum");
        }
        if b.zugestellt.is_some_and(|d| d > heute)
            && vorher.is_none_or(|v| v.zugestellt != b.zugestellt)
        {
            return zukunft("Das Zustelldatum");
        }
    }
    if let Some(v) = &a.vollstaendigkeit
        && v.datum > heute
        && alt.is_none_or(|alt| alt.vollstaendigkeit.as_ref() != Some(v))
    {
        return zukunft("Das Datum der Vollständigkeitsbestätigung");
    }
    if let Some(r) = &a.ruecknahme
        && r.datum > heute
        && alt.is_none_or(|alt| alt.ruecknahme.as_ref() != Some(r))
    {
        return zukunft("Das Rücknahmedatum");
    }
    let bisher = alt.map(|a| a.ereignisse.len()).unwrap_or(0);
    if a.ereignisse.iter().skip(bisher).any(|e| e.datum > heute) {
        return zukunft("Das Datum eines Aktenereignisses");
    }
    Ok(())
}
