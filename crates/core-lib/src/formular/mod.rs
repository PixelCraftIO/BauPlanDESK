//! Amtliche PDF-Formulare: Felder auslesen, mit Projektdaten befüllen,
//! als PDF exportieren.
//!
//! * [`pdf`] liest und schreibt AcroForm-Felder (über `lopdf`, MIT).
//! * [`vorlage`] ordnet Formularfelder Projektdaten zu. Die Zuordnungen
//!   liegen als JSON-Dateien neben der App (`formulare/*.json`) und können
//!   wie die LBO-Regelpakete ohne Neukompilierung aktualisiert werden, wenn
//!   ein Land sein Formular ändert.

pub mod bibliothek;
pub mod editor;
pub mod instanz;
pub mod pdf;
pub(crate) mod schrift;
#[cfg(any(test, feature = "testdaten"))]
pub mod testdaten;
pub mod vorlage;

pub use pdf::{Feldart, Feldwert, Formularfeld};
pub use vorlage::{Formularkatalog, Formularvorlage, Projektkontext};
