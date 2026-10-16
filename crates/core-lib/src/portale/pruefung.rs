use super::*;
use crate::verfahren::{
    AktenCommit, Verfahrensakte,
    abgabe::{Versandereignis, Versandstatus},
};
use std::collections::HashSet;
pub(super) fn versand_id(id: Uuid) -> Uuid {
    Uuid::new_v5(&id, b"bauprojektio.portal.versand")
}
pub(super) fn kommunikation_id(id: Uuid) -> Uuid {
    Uuid::new_v5(&id, b"bauprojektio.portal.kommunikation")
}
pub(super) fn projektion(e: &Portalereignis, z: &Portalziel) -> Versandereignis {
    let status = if e.dateiabgleich != Dateiabgleich::Vollstaendig {
        Versandstatus::Ungeklaert
    } else {
        match e.art {
            Portalereignisart::AbgabeBelegt => Versandstatus::Versendet,
            Portalereignisart::EingangBelegt => Versandstatus::Eingegangen,
            Portalereignisart::Zurueckgewiesen => Versandstatus::Zurueckgewiesen,
            Portalereignisart::AusgangUnklar => Versandstatus::Ungeklaert,
        }
    };
    Versandereignis {
        id: versand_id(e.id),
        status,
        datum: e.datum,
        weg: format!("Portal: {}", z.url),
        referenz: e.referenz.clone(),
        notiz: format!(
            "Manuell erfasst · {:?} · Dateiabgleich {:?}: {}",
            e.art, e.dateiabgleich, e.notiz
        ),
        beleg: e.beleg.clone(),
        berichtigt: e.berichtigt.map(versand_id),
    }
}
pub(crate) fn validieren(
    a: &Verfahrensakte,
    alt: Option<&Verfahrensakte>,
    kontext: AktenCommit,
) -> CoreResult<()> {
    if kontext != AktenCommit::Portal && a.portal.as_ref() != alt.and_then(|a| a.portal.as_ref()) {
        return Err(fehler(
            "Portalangaben über den Bereich Einreichen / Portal ändern",
        ));
    }
    let Some(p) = &a.portal else { return Ok(()) };
    if p.vorgaenge.len() > 20 {
        return Err(fehler("Höchstens 20 Portalvorgänge je Verfahren"));
    }
    let mut ids = HashSet::new();
    let mut abgaben = HashSet::new();
    for v in &p.vorgaenge {
        profile::pruefen(&v.profil)?;
        browser_url(&v.ziel.url)?;
        for s in [&v.ziel.kommune, &v.ziel.stelle] {
            crate::plattform::text(s, 500)?;
        }
        if !v.ziel.zuordnung_bestaetigt
            || !ids.insert(v.id)
            || v.id.is_nil()
            || v.abgaben.len() > 200
            || v.rueckmeldungen.len() > 500
        {
            return Err(fehler(
                "Portalvorgang benötigt bestätigtes Ziel, eindeutige ID und begrenzte Einträge",
            ));
        }
        for b in &v.abgaben {
            if b.id != b.abgabe
                || !ids.insert(b.id)
                || !abgaben.insert(b.abgabe)
                || b.ereignisse.len() > 500
            {
                return Err(fehler(
                    "Abgabestand kann nur einmal einem Portalvorgang zugeordnet sein",
                ));
            }
            let original = a
                .abgaben
                .iter()
                .find(|d| d.id == b.abgabe)
                .ok_or_else(|| fehler("Portalabgabe ohne Verfahrensabgabestand"))?;
            if b.dateien.len() != original.dokumente.len()
                || b.dateien
                    .iter()
                    .any(|d| !original.dokumente.contains(&d.original))
            {
                return Err(fehler(
                    "Portaldateien passen nicht zum eingefrorenen Abgabestand",
                ));
            }
            for e in &b.ereignisse {
                if !ids.insert(e.id) || e.id.is_nil() {
                    return Err(fehler("Portalereignis-ID doppelt oder leer"));
                }
                let erwartet = projektion(e, &v.ziel);
                if !original.versand.contains(&erwartet) {
                    return Err(fehler(
                        "Portalbeleg und Verfahrensversandstand müssen gemeinsam gespeichert werden",
                    ));
                }
            }
        }
        for r in &v.rueckmeldungen {
            if !ids.insert(r.id) || r.id.is_nil() {
                return Err(fehler("Rückmeldungs-ID doppelt oder leer"));
            }
            if !a
                .kommunikation
                .iter()
                .any(|c| c.id == kommunikation_id(r.id) && c.original.as_ref() == Some(&r.original))
            {
                return Err(fehler(
                    "Portalrückmeldung benötigt ihre Kommunikation in der Verfahrensakte",
                ));
            }
        }
    }
    if let Some(alt) = alt {
        for v in alt.portal.iter().flat_map(|p| &p.vorgaenge) {
            let neu = p
                .vorgaenge
                .iter()
                .find(|n| n.id == v.id)
                .ok_or_else(|| fehler("Portalvorgänge bleiben erhalten"))?;
            let mut kopie = neu.clone();
            kopie.abgaben = v.abgaben.clone();
            kopie.rueckmeldungen = v.rueckmeldungen.clone();
            if &kopie != v || !neu.rueckmeldungen.starts_with(&v.rueckmeldungen) {
                return Err(fehler(
                    "Portalziel, Profil und ursprüngliche Rückmeldungen bleiben unverändert",
                ));
            }
            for b in &v.abgaben {
                let neu = neu
                    .abgaben
                    .iter()
                    .find(|n| n.id == b.id)
                    .ok_or_else(|| fehler("Portalabgaben bleiben erhalten"))?;
                let mut kopie = neu.clone();
                kopie.ereignisse = b.ereignisse.clone();
                if &kopie != b || !neu.ereignisse.starts_with(&b.ereignisse) {
                    return Err(fehler(
                        "Portalpakete und frühere Ereignisse sind unveränderlich",
                    ));
                }
            }
        }
        if kontext != AktenCommit::Portal {
            for b in &a.abgaben {
                if abgaben.contains(&b.id)
                    && alt
                        .abgaben
                        .iter()
                        .find(|o| o.id == b.id)
                        .is_some_and(|o| o.versand != b.versand)
                {
                    return Err(fehler(
                        "Belege einer Portalabgabe im Portalbereich erfassen oder berichtigen",
                    ));
                }
            }
        }
    }
    Ok(())
}
pub(super) fn vorbereiten(
    v: &Portalvorgang,
    a: &Verfahrensakte,
    w: &Portalabgabewunsch,
) -> CoreResult<Portalvorbereitung> {
    let b = a
        .abgaben
        .iter()
        .find(|b| b.id == w.abgabe)
        .ok_or_else(|| fehler("Abgabestand nicht in dieser Verfahrensakte"))?;
    if w.dateien.len() != b.dokumente.len() || w.dateien.len() > 100 || w.forderungen.len() > 200 {
        return Err(fehler("Jede eingefrorene Datei genau einmal zuordnen"));
    }
    let vorher = w
        .vorgaenger
        .map(|id| {
            v.abgaben
                .iter()
                .find(|b| b.id == id)
                .ok_or_else(|| fehler("Vorgänger nicht in diesem Portalvorgang"))
        })
        .transpose()?;
    match w.art {
        Portalabgabeart::Erstantrag if vorher.is_some() => {
            return Err(fehler("Erstantrag hat keinen Vorgänger"));
        }
        Portalabgabeart::Nachreichung if vorher.is_none_or(|b| !dateistand_belegt(b)) => {
            return Err(fehler(
                "Nachreichung benötigt einen vollständig abgeglichenen, belegten Vorgänger",
            ));
        }
        _ => {}
    }
    let mut p = Portalpruefbericht {
        fehler: vec![],
        hinweise: v.profil.hinweise.clone(),
        profil_fachlich_geprueft: v.profil.freigabe == Profilfreigabe::FachlichGeprueft,
    };
    if !p.profil_fachlich_geprueft {
        p.hinweise.push("Kein fachlich bestätigtes Portalprofil: Annahmeregeln und Zuständigkeit am Portal prüfen. Die Bereitstellung bestätigt keine Kompatibilität.".into());
    }
    let (referenzdateien, offen) = belegter_dateistand(v, w.vorgaenger)?;
    if offen {
        p.hinweise.push("Mindestens ein früherer Abgabestand der Vorgängerkette ist nicht mehr vollständig belegt. Historischen Empfängerstand prüfen.".into());
    }
    p.hinweise.extend(b.hinweise.clone());
    let mut dateien = vec![];
    let mut gesehen = HashSet::new();
    let mut namen = HashSet::new();
    let mut ersetzte = HashSet::new();
    for z in &w.dateien {
        if !gesehen.insert(z.dokument.dokument_id) {
            return Err(fehler("Dokument mehrfach ausgewählt"));
        }
        let d = b
            .dokumente
            .iter()
            .find(|d| d.dokument == z.dokument)
            .ok_or_else(|| {
                fehler("Dateizuordnung verweist nicht auf eine eingefrorene Dokumentversion")
            })?;
        crate::xbau::austausch::dateiname(&z.uploadname)?;
        if !namen.insert(z.uploadname.to_lowercase()) {
            return Err(fehler(
                "Uploadnamen müssen ohne Beachtung der Großschreibung eindeutig sein",
            ));
        }
        if let Some(limit) = v.profil.max_datei_bytes
            && d.bytes > limit
        {
            p.fehler.push(format!(
                "{} überschreitet die profilierte Dateigröße",
                d.name
            ));
        }
        let endung = z
            .uploadname
            .rsplit_once('.')
            .map(|(_, s)| s.to_lowercase())
            .unwrap_or_default();
        if !v.profil.erlaubte_endungen.is_empty()
            && !v
                .profil
                .erlaubte_endungen
                .iter()
                .any(|e| e.eq_ignore_ascii_case(&endung))
        {
            p.fehler
                .push(format!("{}: Dateiformat im Profil nicht zulässig", d.name));
        }
        if d.name.rsplit('.').next().map(str::to_lowercase)
            != z.uploadname.rsplit('.').next().map(str::to_lowercase)
        {
            p.fehler.push(format!(
                "{}: Eine andere Dateiendung konvertiert die Datei nicht",
                d.name
            ));
        }
        if let Some(code) = &z.unterlagenart {
            if !v.profil.unterlagenarten.iter().any(|u| u.code == *code) {
                p.fehler.push(format!(
                    "{}: Unterlagenart nicht im gewählten Profil",
                    d.name
                ));
            }
        } else if !v.profil.unterlagenarten.is_empty() {
            p.fehler
                .push(format!("{}: Portal-Unterlagenart fehlt", d.name));
        }
        let explizit = z
            .ersetzt
            .as_ref()
            .map(|ref_| {
                referenzdateien
                    .iter()
                    .find(|d| &d.original.dokument == ref_)
                    .ok_or_else(|| fehler("Ersetztes Dokument nicht im belegten Vorgänger"))
            })
            .transpose()?;
        let alt = explizit.or_else(|| {
            referenzdateien
                .iter()
                .find(|x| x.original.dokument.dokument_id == d.dokument.dokument_id)
        });
        if let Some(alt) = alt
            && !ersetzte.insert((
                alt.original.dokument.dokument_id,
                alt.original.dokument.version,
            ))
        {
            return Err(fehler("Ein früheres Dokument nicht mehrfach ersetzen"));
        }
        let vergleich = match alt {
            None => Dateivergleich::Neu,
            Some(alt) if alt.original.sha256 == d.sha256 => Dateivergleich::Identisch,
            Some(alt) if alt.original.dokument.dokument_id != d.dokument.dokument_id => {
                Dateivergleich::AnderesDokument
            }
            Some(alt) if d.dokument.version < alt.original.dokument.version => {
                Dateivergleich::Aelter
            }
            Some(_) => Dateivergleich::Geaendert,
        };
        if vergleich == Dateivergleich::Aelter {
            p.fehler.push(format!(
                "{}: Ältere Version als im belegten Vorgänger ausgewählt",
                d.name
            ));
        }
        dateien.push(Portaldatei {
            original: d.clone(),
            uploadname: z.uploadname.clone(),
            unterlagenart: z.unterlagenart.clone(),
            vergleich,
            ersetzt: alt.map(|a| a.original.dokument.clone()),
        });
    }
    let sum: u64 = b.dokumente.iter().map(|d| d.bytes).sum();
    if v.profil.max_gesamt_bytes.is_some_and(|n| sum > n) {
        p.fehler
            .push("Profilierte Gesamtgröße überschritten".into());
    }
    for u in v.profil.unterlagenarten.iter().filter(|u| u.erforderlich) {
        if !dateien
            .iter()
            .any(|d| d.unterlagenart.as_ref() == Some(&u.code))
        {
            p.fehler
                .push(format!("Erforderliche Unterlagenart fehlt: {}", u.name));
        }
    }
    let mut punkte = HashSet::new();
    for r in &w.forderungen {
        if !punkte.insert(r.punkt)
            || !a
                .nachforderungen
                .iter()
                .any(|n| n.id == r.nachforderung && n.punkte.iter().any(|p| p.id == r.punkt))
        {
            return Err(fehler("Unbekannter oder mehrfacher Forderungspunkt"));
        }
    }
    Ok(Portalvorbereitung {
        referenzdateien,
        dateien,
        pruefung: p,
    })
}

fn belegter_dateistand(
    v: &Portalvorgang,
    start: Option<Uuid>,
) -> CoreResult<(Vec<Portaldatei>, bool)> {
    let mut id = start;
    let mut besucht = HashSet::new();
    let mut kette = vec![];
    while let Some(i) = id {
        if !besucht.insert(i) || besucht.len() > 200 {
            return Err(fehler("Ungültige Vorgängerkette"));
        }
        let b = v
            .abgaben
            .iter()
            .find(|b| b.id == i)
            .ok_or_else(|| fehler("Vorgänger der Portalabgabe fehlt"))?;
        kette.push(b);
        id = b.vorgaenger;
    }
    let mut stand = std::collections::BTreeMap::new();
    let mut offen = false;
    for b in kette.into_iter().rev() {
        if !dateistand_belegt(b) {
            offen = true;
            continue;
        }
        for d in &b.dateien {
            if let Some(alt) = &d.ersetzt
                && alt.dokument_id != d.original.dokument.dokument_id
            {
                stand.remove(&alt.dokument_id.0);
            }
            stand.insert(d.original.dokument.dokument_id.0, d.clone());
        }
    }
    Ok((stand.into_values().collect(), offen))
}
