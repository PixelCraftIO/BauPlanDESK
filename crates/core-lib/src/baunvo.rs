//! Gemeinsame, versionierte JSON-Regelwerke für Oberfläche und Rechenkern.
use crate::flaechen::{Anrechnung, BauNvo, Bauteiltyp};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::OnceLock};
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BauteilRegel {
    pub bezeichnung: String,
    pub anrechnung: Anrechnung,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Regelwerk {
    pub id: String,
    pub version: String,
    pub fassung: BauNvo,
    pub grz_ii: bool,
    pub grz_ii_faktor: f64,
    pub grz_ii_kappung: f64,
    pub bauteile: HashMap<Bauteiltyp, BauteilRegel>,
    pub nichtvollgeschoss_faktor: f64,
    pub nichtvollgeschoss_planabweichung: bool,
    pub gemeinschaftsanteile: bool,
    pub gemeinschaftsanteile_rechtsgrundlage: String,
    pub quellen: Vec<String>,
}
const DATEIEN: [&str; 5] = [
    include_str!("../../../regelwerke/baunvo/1962.json"),
    include_str!("../../../regelwerke/baunvo/1968.json"),
    include_str!("../../../regelwerke/baunvo/1977.json"),
    include_str!("../../../regelwerke/baunvo/1990.json"),
    include_str!("../../../regelwerke/baunvo/2017.json"),
];
pub fn regelwerke() -> &'static [Regelwerk] {
    static WERKE: OnceLock<Vec<Regelwerk>> = OnceLock::new();
    WERKE.get_or_init(|| {
        DATEIEN
            .iter()
            .map(|s| serde_json::from_str(s).expect("Ungültiges ausgeliefertes BauNVO-Regelwerk"))
            .collect()
    })
}
pub fn regelwerk(v: BauNvo) -> &'static Regelwerk {
    regelwerke()
        .iter()
        .find(|r| r.fassung == v)
        .expect("BauNVO-Regelwerk fehlt")
}
/// Wählt ausschließlich aus dem übergebenen Katalog; kein Rückfall auf aktuelle Regeln.
pub fn aus_katalog(werke: &[Regelwerk], fassung: BauNvo) -> crate::CoreResult<&Regelwerk> {
    let mut passend = werke.iter().filter(|r| r.fassung == fassung);
    let r = passend.next().ok_or_else(|| {
        crate::CoreError::InvalidInput("Benötigtes BauNVO-Regelwerk fehlt im Katalog".into())
    })?;
    if passend.next().is_some() {
        return Err(crate::CoreError::InvalidInput(
            "Mehrdeutiger BauNVO-Katalog".into(),
        ));
    }
    r.validate()?;
    Ok(r)
}
impl Regelwerk {
    /// Strukturprüfung; die fachliche Freigabe eines Regelpakets erfolgt gesondert.
    pub fn validate(&self) -> crate::CoreResult<()> {
        let typen = [
            Bauteiltyp::Hauptanlage,
            Bauteiltyp::TerrasseHauptanlage,
            Bauteiltyp::Garage,
            Bauteiltyp::Stellplatz,
            Bauteiltyp::Zufahrt,
            Bauteiltyp::Nebenanlage14,
            Bauteiltyp::Unterirdisch,
            Bauteiltyp::BalkonLoggia,
            Bauteiltyp::TerrasseNebenanlage,
        ];
        if self.id.trim().is_empty()
            || self.version.trim().is_empty()
            || self.id.len() > 200
            || self.version.len() > 100
            || self.bauteile.len() != typen.len()
            || typen.iter().any(|t| {
                self.bauteile.get(t).is_none_or(|b| {
                    b.bezeichnung.trim().is_empty()
                        || (!self.grz_ii && b.anrechnung == Anrechnung::GrzIi)
                })
            })
            || !(0.0..=1.0).contains(&self.nichtvollgeschoss_faktor)
            || !self.grz_ii_faktor.is_finite()
            || self.grz_ii_faktor < 1.0
            || !(0.0..=1.0).contains(&self.grz_ii_kappung)
        {
            return Err(crate::CoreError::InvalidInput(
                "Ungültiges oder unvollständiges BauNVO-Regelwerk".into(),
            ));
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alle_fassungen_sind_vollstaendig_und_gueltig() {
        let typen = [
            Bauteiltyp::Hauptanlage,
            Bauteiltyp::TerrasseHauptanlage,
            Bauteiltyp::Garage,
            Bauteiltyp::Stellplatz,
            Bauteiltyp::Zufahrt,
            Bauteiltyp::Nebenanlage14,
            Bauteiltyp::Unterirdisch,
            Bauteiltyp::BalkonLoggia,
            Bauteiltyp::TerrasseNebenanlage,
        ];
        for v in [
            BauNvo::V1962,
            BauNvo::V1968,
            BauNvo::V1977,
            BauNvo::V1990,
            BauNvo::V2017,
        ] {
            let r = regelwerk(v);
            assert!(!r.id.is_empty() && !r.version.is_empty());
            assert_eq!(r.bauteile.len(), typen.len());
            for t in typen {
                let b = &r.bauteile[&t];
                assert!(!b.bezeichnung.is_empty());
                assert!(r.grz_ii || b.anrechnung != Anrechnung::GrzIi);
            }
            assert!((0.0..=1.0).contains(&r.nichtvollgeschoss_faktor));
            assert!(r.grz_ii_faktor.is_finite() && r.grz_ii_faktor >= 1.0);
            assert!((0.0..=1.0).contains(&r.grz_ii_kappung));
        }
    }
}
