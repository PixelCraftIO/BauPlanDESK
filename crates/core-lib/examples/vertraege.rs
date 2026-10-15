//! Generiert neue Plattformverträge aus den serialisierbaren Rust-Domänentypen.
use core_lib::{
    benutzer::BenutzerId,
    commit::Fachstand,
    domain::{Bundesland, DocumentId, ProjectId},
    plattform::{ObjektArt, nachweise::*, objekte::*},
};
use ts_rs::TS;
fn main() {
    println!(
        "// Automatisch erzeugt: cargo run -p core-lib --example vertraege. Nicht von Hand ändern.\nimport type {{ BaugrundstuecksPlan, BaugrundstueckBerechnung, FlaechenAbzug, Planungsrecht }} from '../api';"
    );
    macro_rules! ausgabe {($($t:ty),* $(,)?)=>{$(println!("export {}",<$t as TS>::decl().lines().map(str::trim_end).collect::<Vec<_>>().join("\n"));)*};}
    ausgabe!(
        core_lib::grundstuecksinformationen::Grundstuecksinformationen,
        core_lib::grundstuecksinformationen::FlurstuecksInfo,
        core_lib::grundstuecksinformationen::FlurstuecksAnsatz,
        core_lib::grundstuecksinformationen::BaugrundstuecksErmittlung,
        core_lib::calc::Baugebiet,
        core_lib::flaechen::BauNvo,
        core_lib::flaechen::Bauteiltyp,
        core_lib::flaechen::Anrechnung,
        core_lib::lbo::AbstandsflaecheInput,
        ProjectId,
        DocumentId,
        Bundesland,
        BenutzerId,
        serde_json::Value,
        Fachstand,
        core_lib::zugriff::ProjektRecht,
        core_lib::zugriff::ProjektMitglied,
        core_lib::plattform::ausgabe::AusgabeStatus,
        core_lib::plattform::ausgabe::Ausgabeauftrag,
        core_lib::dokumentkategorien::Dokumentkategorie,
        core_lib::dokumentkategorien::KategorienKatalog,
        core_lib::dokumentkategorien::KategorieStand,
        core_lib::dokumentkategorien::KategorieZuordnung,
        core_lib::dokumentkategorien::DokumentKategorie,
        core_lib::email::MailAdresse,
        core_lib::email::MailAnhang,
        core_lib::email::EmailInhalt,
        ObjektArt,
        Projektphase,
        Projektorganisation,
        Dokumentbezug,
        Gebaeude,
        GebaeudeGeschoss,
        AufgabenStatus,
        Aufgabe,
        UnterlagenStatus,
        Unterlage,
        core_lib::plattform::ablauf::ChecklistenQuelle,
        core_lib::plattform::ablauf::Checklistenpunkt,
        core_lib::plattform::ablauf::Checkliste,
        core_lib::plattform::ablauf::Statushinweis,
        core_lib::plattform::ablauf::Projektstatus,
        Dokumentdaten,
        Raumbuch,
        Raum,
        RaumSummen,
        NutzungsArt,
        NachweisEingabe,
        RegelArchiv,
        Nachweisstand,
        Freigabe,
        NachweisAnsicht
    );
}
