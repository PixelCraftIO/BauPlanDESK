use super::*;
use crate::{
    CoreServices,
    benutzer::{Benutzer, Recht},
};
const KEY: &str = "portale/profile/v1";
pub fn eingebaut() -> Vec<Portalprofil> {
    serde_json::from_str(include_str!("../../../../portalprofile/profile.json"))
        .expect("eingebaute Portalprofile")
}
pub fn pruefen(p: &Portalprofil) -> CoreResult<()> {
    if serde_json::to_vec(p)
        .map_err(|e| fehler(e.to_string()))?
        .len()
        > 256 * 1024
    {
        return Err(fehler("Portalprofil größer als 256 KiB"));
    }
    for s in [&p.id, &p.version, &p.titel, &p.quelle, &p.stand] {
        crate::plattform::text(s, 1000)?;
    }
    if p.id.len() > 100
        || p.version.len() > 100
        || p.hinweise.len() > 50
        || p.hinweise.iter().any(|s| s.len() > 4000)
        || p.unterlagenarten.len() > 200
        || p.verfahren.len() > 20
        || p.erlaubte_endungen.len() > 100
        || p.max_datei_bytes == Some(0)
        || p.max_gesamt_bytes == Some(0)
    {
        return Err(fehler("Ungültige Profilgrenzen"));
    }
    if let Some(u) = &p.portal_url {
        browser_url(u)?;
    }
    if p.erlaubte_endungen
        .iter()
        .any(|s| s.is_empty() || s.len() > 20 || !s.bytes().all(|c| c.is_ascii_alphanumeric()))
    {
        return Err(fehler("Dateiendungen ohne Punkt eingeben"));
    }
    let mut codes = std::collections::HashSet::new();
    for a in &p.unterlagenarten {
        crate::plattform::text(&a.code, 100)?;
        crate::plattform::text(&a.name, 500)?;
        if !codes.insert(&a.code)
            || a.kategorien.len() > 100
            || a.kategorien.iter().any(|s| s.len() > 100)
        {
            return Err(fehler(
                "Unterlagenarten benötigen eindeutige Codes und begrenzte Zuordnungen",
            ));
        }
    }
    Ok(())
}
impl CoreServices {
    pub async fn portalprofile(&self) -> CoreResult<Vec<Portalprofil>> {
        let mut p = eingebaut();
        if let Some(raw) = self.repos.lizenz.einstellung(KEY).await? {
            let eigene: Vec<Portalprofil> =
                serde_json::from_str(&raw).map_err(|e| fehler(e.to_string()))?;
            for x in eigene {
                pruefen(&x)?;
                if p.iter().any(|v| v.id == x.id && v.version == x.version) {
                    return Err(fehler("Doppeltes Portalprofil"));
                }
                p.push(x);
            }
        }
        Ok(p)
    }
    pub async fn portalprofil_hinzufuegen(
        &self,
        p: Portalprofil,
        wer: &Benutzer,
    ) -> CoreResult<Vec<Portalprofil>> {
        self.pruefe(wer, Recht::BueroangabenAendern).await?;
        pruefen(&p)?;
        let vorher = self.repos.lizenz.einstellung(KEY).await?;
        let mut alle = self.portalprofile().await?;
        if let Some(alt) = alle.iter().find(|v| v.id == p.id && v.version == p.version) {
            if alt == &p {
                return Ok(alle);
            }
            return Err(fehler("Geändertes Profil benötigt eine neue Version"));
        }
        if alle.len() >= 100 {
            return Err(fehler("Höchstens 100 Portalprofile"));
        }
        alle.push(p);
        let eigene: Vec<_> = alle
            .iter()
            .filter(|p| {
                !eingebaut()
                    .iter()
                    .any(|e| e.id == p.id && e.version == p.version)
            })
            .collect();
        let neu = serde_json::to_string(&eigene).map_err(|e| fehler(e.to_string()))?;
        if !self
            .repos
            .lizenz
            .einstellung_vergleichen(KEY, vorher.as_deref(), &neu)
            .await?
        {
            return Err(fehler(
                "Profilkatalog wurde gleichzeitig geändert; neu laden und erneut hinzufügen",
            ));
        }
        Ok(alle)
    }
}
