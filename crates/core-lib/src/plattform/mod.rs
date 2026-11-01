//! Fachmodule mit eigenen Schemas, Revisionen und öffentlichen Anwendungsfällen.
pub mod ablauf;
pub mod ausgabe;
pub mod einbindung;
mod einheit;
pub mod gebaeude;
pub mod nachweise;
pub mod objekte;
pub mod verweise;
use crate::{
    CoreError, CoreResult, CoreServices,
    benutzer::{Benutzer, Recht},
    commit::Fachstand,
    domain::{Aktion, ProjectId},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum ObjektArt {
    Stellplaetze,
    Wohnflaechen,
    Din277,
    Vollgeschosse,
    Hoehen,
    Freiflaechen,
    Entwaesserung,
    Ueberbaubarkeit,
    Abstandsflaechen,
    Verfahren,
    Projektorganisation,
    Gebaeude,
    Aufgaben,
    Unterlagen,
    Dokumentdaten,
    Raumbuch,
    Planaufmass,
    Cadaufmass,
    /// Importiertes IFC-Modell ([`crate::ifc::Modellakte`]).
    Ifcmodell,
    /// Projekttagebuch ([`objekte::Tagebucheintrag`]), ein Datensatz je Eintrag.
    Projekttagebuch,
}
impl ObjektArt {
    pub fn modul(self) -> &'static str {
        match self {
            Self::Stellplaetze => "stellplaetze",
            Self::Wohnflaechen => "wohnflaechen",
            Self::Din277 => "din277",
            Self::Vollgeschosse => "vollgeschosse",
            Self::Hoehen => "hoehen",
            Self::Freiflaechen => "freiflaechen",
            Self::Entwaesserung => "entwaesserung",
            Self::Ueberbaubarkeit => "ueberbaubarkeit",
            Self::Abstandsflaechen => "abstandsflaechen",
            Self::Verfahren => "verfahren",
            Self::Projektorganisation => "projektorganisation",
            Self::Gebaeude => "gebaeude",
            Self::Aufgaben => "aufgaben",
            Self::Unterlagen => "unterlagen",
            Self::Dokumentdaten => "dokumentdaten",
            Self::Raumbuch => "raumbuch",
            Self::Planaufmass => "planaufmass",
            Self::Cadaufmass => "cadaufmass",
            Self::Ifcmodell => "ifcmodell",
            Self::Projekttagebuch => "projekttagebuch",
        }
    }
}
impl CoreServices {
    /// Fachstände eines Moduls in der aktuellen Datenform: Ältere Datensätze
    /// werden über den typisierten Struct gelesen, damit Felder mit
    /// `#[serde(default)]` (z. B. `schritte`, `antragsangaben` der
    /// Verfahrensakte) ergänzt sind. So vergleichen Verbraucher Original und
    /// Eingabe in derselben Form, wie sie eine Speicherung ablegen würde. Die
    /// Normalisierung schreibt nichts zurück und ändert keine Revision.
    /// Begrenzte Liste für Editoren; vollständige Bestände bleiben für fachliche
    /// Auswertungen über `fachobjekte` verfügbar.
    pub async fn fachobjektseite(
        &self,
        p: ProjectId,
        art: ObjektArt,
        nach: Option<&str>,
        limit: u32,
    ) -> CoreResult<Vec<Fachstand>> {
        self.get_project(p).await?;
        self.repos
            .commit
            .fachseite(p, art.modul(), nach, limit)
            .await
    }
    pub async fn fachobjekt_lesen(
        &self,
        p: ProjectId,
        art: ObjektArt,
        id: Uuid,
    ) -> CoreResult<Option<Fachstand>> {
        self.get_project(p).await?;
        self.repos
            .commit
            .fachobjekt(p, art.modul(), &id.to_string())
            .await
    }
    pub async fn fachverlaufsseite(
        &self,
        p: ProjectId,
        art: ObjektArt,
        id: Uuid,
        vor: Option<i64>,
        limit: u32,
    ) -> CoreResult<Vec<Fachstand>> {
        self.get_project(p).await?;
        self.repos
            .commit
            .fachverlaufsseite(p, art.modul(), &id.to_string(), vor, limit)
            .await
    }
    pub async fn fachobjekte(&self, p: ProjectId, art: ObjektArt) -> CoreResult<Vec<Fachstand>> {
        self.repos.projects.get(p).await?;
        Ok(self
            .repos
            .commit
            .fachstaende(p, art.modul())
            .await?
            .into_iter()
            .map(|f| normalisiert(art, f))
            .collect())
    }
    pub async fn fachverlauf(
        &self,
        p: ProjectId,
        art: ObjektArt,
        id: Uuid,
    ) -> CoreResult<Vec<Fachstand>> {
        self.repos.projects.get(p).await?;
        self.repos
            .commit
            .fachverlauf(p, art.modul(), &id.to_string())
            .await
    }
    pub async fn fachobjekt_speichern(
        &self,
        p: ProjectId,
        art: ObjektArt,
        id: Uuid,
        basis: i64,
        daten: serde_json::Value,
        wer: &Benutzer,
    ) -> CoreResult<Fachstand> {
        if art == ObjektArt::Verfahren {
            let a = serde_json::from_value(daten).map_err(json_err)?;
            return self.verfahren_commit(p, id, basis, a, wer, false).await;
        }
        self.pruefe(wer, Recht::ProjektBearbeiten).await?;
        if basis < 0 {
            return Err(CoreError::InvalidInput("Ungültige Basisrevision".into()));
        }
        let projekt = self.repos.projects.get(p).await?;
        if serde_json::to_vec(&daten).map_err(json_err)?.len() > 2 * 1024 * 1024 {
            return Err(CoreError::InvalidInput("Fachdatensatz ist zu groß".into()));
        }
        let daten = if crate::berechnungen::BerechnungsArt::ALLE
            .iter()
            .any(|m| m.objekt() == art)
        {
            self.berechnungsplan_vorbereiten(p, id, basis, daten, wer)
                .await?
        } else if art == ObjektArt::Abstandsflaechen {
            self.abstandsplan_vorbereiten(p, id, basis, daten, wer)
                .await?
        } else {
            daten
        };
        let daten = if art == ObjektArt::Gebaeude {
            let mut g: objekte::Gebaeude = serde_json::from_value(daten).map_err(json_err)?;
            for geschoss in &mut g.geschosse {
                if let Some(q) = &geschoss.einstufungsquelle {
                    let (wert, grundlage) = self
                        .geschosseinstufung_aus_nachweis(p, q, id, geschoss.id, None)
                        .await?;
                    geschoss.vollgeschoss = Some(wert);
                    geschoss.grundlage = grundlage;
                }
            }
            serde_json::to_value(g).map_err(json_err)?
        } else {
            daten
        };
        let daten = objekte::validieren(self, &projekt, art, id, daten, wer).await?;
        if art == ObjektArt::Projektorganisation && id != p.0 {
            return Err(CoreError::InvalidInput(
                "Projektorganisation benötigt die Identität des Projekts".into(),
            ));
        }
        if art == ObjektArt::Dokumentdaten {
            let d: objekte::Dokumentdaten =
                serde_json::from_value(daten.clone()).map_err(json_err)?;
            if d.dokument.dokument_id.0 != id {
                return Err(CoreError::InvalidInput(
                    "Dokumentangaben benötigen die Identität des zugehörigen Dokuments".into(),
                ));
            }
        }
        let vorher = self
            .repos
            .commit
            .fachstaende(p, art.modul())
            .await?
            .into_iter()
            .find(|f| f.objekt == id.to_string());
        if let Some(alt) = &vorher {
            if alt.schema_version > 1 {
                return Err(CoreError::InvalidInput(
                    "Fachdatensatz wurde mit einer neueren Anwendung erstellt".into(),
                ));
            }
            if alt.daten == daten {
                return Ok(alt.clone());
            }
        }
        // Verknüpfungen mit dem Aufmaß (neu, aktualisiert, gelöst) samt altem
        // und neuem Wert gehören ins Protokoll; der Datensatz selbst behält nur
        // den aktuellen Wert.
        let aufmass =
            crate::planaufmass::planaenderungen(vorher.as_ref().map(|f| &f.daten), &daten);
        let f = Fachstand {
            projekt_id: p,
            modul: art.modul().into(),
            objekt: id.to_string(),
            schema_version: 1,
            revision: basis,
            daten,
        };
        let audit = Self::audit(
            wer,
            Aktion::ProjektGeaendert,
            Some(p),
            None,
            match aufmass {
                Some(a) => format!("{} {} gespeichert · {a}", art.modul(), id),
                None => format!("{} {} gespeichert", art.modul(), id),
            },
        );
        self.repos.commit.fachstand(f, basis, audit).await
    }
}
/// Liest einen Fachstand über den typisierten Struct seines Moduls und gibt ihn
/// in aktueller Form zurück. Datensätze einer neueren Schemaversion und solche,
/// die sich nicht lesen lassen, bleiben unverändert: Die Speichergrenze meldet
/// den Fehler dann ausdrücklich, statt ihn hier durch Standardwerte zu verdecken.
pub fn normalisiert(art: ObjektArt, mut f: Fachstand) -> Fachstand {
    fn rund<T: serde::de::DeserializeOwned + Serialize>(
        v: &serde_json::Value,
    ) -> Option<serde_json::Value> {
        serde_json::to_value(serde_json::from_value::<T>(v.clone()).ok()?).ok()
    }
    if f.schema_version != 1 {
        return f;
    }
    let neu = match art {
        ObjektArt::Stellplaetze
        | ObjektArt::Wohnflaechen
        | ObjektArt::Din277
        | ObjektArt::Vollgeschosse
        | ObjektArt::Hoehen
        | ObjektArt::Freiflaechen
        | ObjektArt::Entwaesserung
        | ObjektArt::Ueberbaubarkeit => rund::<crate::berechnungen::FachPlan>(&f.daten),
        ObjektArt::Abstandsflaechen => rund::<crate::abstandsflaechen::AbstandsPlan>(&f.daten),
        ObjektArt::Verfahren => rund::<crate::verfahren::Verfahrensakte>(&f.daten),
        ObjektArt::Projektorganisation => rund::<objekte::Projektorganisation>(&f.daten),
        ObjektArt::Gebaeude => rund::<objekte::Gebaeude>(&f.daten),
        ObjektArt::Aufgaben => rund::<objekte::Aufgabe>(&f.daten),
        ObjektArt::Unterlagen => rund::<objekte::Unterlage>(&f.daten),
        ObjektArt::Dokumentdaten => rund::<objekte::Dokumentdaten>(&f.daten),
        ObjektArt::Raumbuch => rund::<objekte::Raumbuch>(&f.daten),
        ObjektArt::Planaufmass => rund::<crate::planaufmass::Planaufmass>(&f.daten),
        ObjektArt::Cadaufmass => rund::<crate::cad::projektaufmass::CadProjektaufmass>(&f.daten),
        ObjektArt::Ifcmodell => rund::<crate::ifc::Modellakte>(&f.daten),
        ObjektArt::Projekttagebuch => rund::<objekte::Tagebucheintrag>(&f.daten),
    };
    if let Some(neu) = neu {
        f.daten = neu;
    }
    f
}
fn json_err(e: serde_json::Error) -> CoreError {
    CoreError::InvalidInput(e.to_string())
}
pub fn text(s: &str, max: usize) -> CoreResult<()> {
    if s.trim().is_empty() || s.chars().count() > max {
        Err(CoreError::InvalidInput(format!(
            "Text mit 1 bis {max} Zeichen erforderlich"
        )))
    } else {
        Ok(())
    }
}
