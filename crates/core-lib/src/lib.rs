//! # core-lib
//!
//! Die gesamte Geschäftslogik von BauPlanIO. Dieses Crate wird
//! unverändert in die Tauri-Desktop-App (Einzelplatz) und in den
//! axum-Server (NAS/Büro/Cloud) eingebunden.
//!
//! Regeln für dieses Crate:
//! * keine Abhängigkeit auf eine konkrete Datenbank, ein Dateisystem, Tauri oder axum,
//! * Persistenz nur über die Traits in [`repository`],
//! * Dateien nur über den Trait [`storage::Storage`],
//! * landesrechtliche Regeln nur über Skripte der [`lbo`]-Engine.

pub mod anmeldung;
pub mod baugrundstuecke;
pub mod baunvo;
pub mod benutzer;
pub mod cad;
pub mod calc;
pub mod dateiname;
pub mod dateiverwaltung;
pub mod dokumentkategorien;
pub mod domain;
pub mod einreichung;
pub mod email;
pub mod error;
pub mod exportgrenze;
pub mod festsetzungen;
pub mod flaechen;
pub mod formular;
pub mod kalender;
pub mod kennwort;
pub mod kunden;
pub mod lbo;
pub mod license;
pub mod repository;
pub mod service;
pub mod storage;
#[cfg(any(test, feature = "testdaten"))]
pub mod testsuite;
pub mod xbau;

pub use error::{CoreError, CoreResult};
pub use service::CoreServices;

pub mod projektablage;

pub mod grundstuecksinformationen;
pub mod grundstueckskontext;

pub mod commit;

pub mod projektbereiche;

pub mod planaufmass;
pub mod plattform;

pub mod zugriff;

pub mod druckausgabe;

pub mod verfahren;

pub mod abstandsflaechen;

pub mod berechnungen;

pub mod projektuebersicht;

pub mod portale;

mod lesesitzung;

pub mod rechenjob;
