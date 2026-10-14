//! Gemeinsame, versionierte JSON-Regelwerke für Oberfläche und Rechenkern.
use crate::flaechen::{Anrechnung, BauNvo, Bauteiltyp};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::OnceLock};
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BauteilRegel {
    pub bezeichnung: String,
    pub anrechnung: Anrechnung,
}
#[derive(Debug, Deserialize, Serialize)]
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
