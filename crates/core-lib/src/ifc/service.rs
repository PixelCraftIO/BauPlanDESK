//! IFC-Modelle aus dem DMS laden (mit kleinem Zwischenspeicher je
//! Dokumentversion) und Modellakten prüfen.
use super::{Modellakte, Modellauszug, auszug, step::StepDatei};
use crate::{
    CoreError, CoreResult, CoreServices,
    domain::{DocumentId, ProjectId},
    plattform::objekte::Dokumentbezug,
};
use std::sync::{Arc, Mutex, OnceLock};

/// Gelesenes Modell: Datei (für das Zurückschreiben) und Auszug.
#[derive(Debug)]
pub struct Modell {
    pub datei: StepDatei,
    pub auszug: Modellauszug,
}

impl Modell {
    /// Liest `.ifc` (STEP) oder `.ifczip` (ZIP mit einer `.ifc`-Datei).
    pub fn lesen(dateiname: &str, daten: &[u8]) -> CoreResult<Self> {
        let endung = dateiname
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
            .unwrap_or_default();
        let entpackt;
        let daten = if endung == "ifczip" || daten.starts_with(b"PK\x03\x04") {
            entpackt = entpacken(daten)?;
            &entpackt[..]
        } else if endung == "ifcxml" {
            return Err(CoreError::InvalidInput(
                "IFC-XML wird nicht unterstützt. Bitte das Modell als IFC (STEP) exportieren."
                    .into(),
            ));
        } else {
            daten
        };
        let datei = StepDatei::lesen(daten)?;
        let auszug = auszug(&datei)?;
        Ok(Self { datei, auszug })
    }
}

fn entpacken(daten: &[u8]) -> CoreResult<Vec<u8>> {
    use std::io::Read;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(daten))
        .map_err(|e| CoreError::InvalidInput(format!("IFCZIP nicht lesbar: {e}")))?;
    for i in 0..zip.len() {
        let mut f = zip
            .by_index(i)
            .map_err(|e| CoreError::InvalidInput(format!("IFCZIP nicht lesbar: {e}")))?;
        if f.name().is_ok_and(|n| n.to_lowercase().ends_with(".ifc")) {
            if f.size() as usize > super::step::MAX_BYTES {
                return Err(CoreError::InvalidInput(
                    "IFC-Datei ist größer als 400 MB".into(),
                ));
            }
            let mut out = Vec::with_capacity(f.size() as usize);
            f.by_ref()
                .take(super::step::MAX_BYTES as u64 + 1)
                .read_to_end(&mut out)
                .map_err(|e| CoreError::InvalidInput(format!("IFCZIP nicht lesbar: {e}")))?;
            return Ok(out);
        }
    }
    Err(CoreError::InvalidInput(
        "IFCZIP enthält keine .ifc-Datei".into(),
    ))
}

type Schluessel = (DocumentId, i64);
type Speicher = Mutex<Vec<(Schluessel, Arc<Modell>)>>;
/// Zuletzt gelesene Modelle; Dokumentversionen sind unveränderlich.
fn zwischenspeicher() -> &'static Speicher {
    static Z: OnceLock<Speicher> = OnceLock::new();
    Z.get_or_init(|| Mutex::new(Vec::new()))
}
const ZWISCHENSPEICHER: usize = 4;

pub fn ist_ifc(dateiname: &str) -> bool {
    let n = dateiname.to_ascii_lowercase();
    n.ends_with(".ifc") || n.ends_with(".ifczip")
}

impl CoreServices {
    /// Liest das Modell einer Dokumentversion dieses Projekts.
    pub async fn ifc_modell(&self, p: ProjectId, d: &Dokumentbezug) -> CoreResult<Arc<Modell>> {
        let schluessel = (d.dokument_id, d.version);
        if let Some(m) = zwischenspeicher().lock().ok().and_then(|z| {
            z.iter()
                .find(|(k, _)| *k == schluessel)
                .map(|(_, m)| m.clone())
        }) {
            // Zugehörigkeit trotzdem prüfen: der Speicher ist projektübergreifend.
            self.ifc_dokument_pruefen(p, d).await?;
            return Ok(m);
        }
        let name = self.ifc_dokument_pruefen(p, d).await?;
        let (_, daten) = self.load_version(d.dokument_id, d.version).await?;
        let m = Arc::new(Modell::lesen(&name, &daten)?);
        if let Ok(mut z) = zwischenspeicher().lock() {
            z.retain(|(k, _)| *k != schluessel);
            z.insert(0, (schluessel, m.clone()));
            z.truncate(ZWISCHENSPEICHER);
        }
        Ok(m)
    }

    async fn ifc_dokument_pruefen(&self, p: ProjectId, d: &Dokumentbezug) -> CoreResult<String> {
        let v = self
            .repos
            .documents
            .history(d.dokument_id)
            .await?
            .into_iter()
            .find(|v| v.version == d.version && v.project_id == p)
            .ok_or_else(|| {
                CoreError::InvalidInput("Dokumentversion gehört nicht zu diesem Projekt".into())
            })?;
        if !ist_ifc(&v.file_name) {
            return Err(CoreError::InvalidInput(format!(
                "„{}“ ist keine IFC-Datei (.ifc oder .ifczip)",
                v.file_name
            )));
        }
        Ok(v.file_name)
    }

    /// Auszug, wie die Modellakte ihn sieht: das Modell ihrer Dokumentversion
    /// mit angewendeter Nacharbeit. Grundlage von Übernahme und Auswertung.
    pub async fn ifc_akte_auszug(
        &self,
        p: ProjectId,
        akte: &Modellakte,
    ) -> CoreResult<Modellauszug> {
        Ok(self.ifc_modell(p, &akte.dokument).await?.auszug.clone())
    }

    /// Modellauszug einer Dokumentversion (Struktur, Eigenschaften, Geometrie).
    pub async fn ifc_auszug(&self, p: ProjectId, d: &Dokumentbezug) -> CoreResult<Modellauszug> {
        Ok(self.ifc_modell(p, d).await?.auszug.clone())
    }

    /// Grundriss-Schnitt eines Geschosses (Meter, Weltsystem des Modells).
    pub async fn ifc_grundriss(
        &self,
        p: ProjectId,
        d: &Dokumentbezug,
        geschoss: &str,
        schnitthoehe_m: Option<f64>,
    ) -> CoreResult<super::Modellgrundriss> {
        super::grundriss::grundriss(
            &self.ifc_modell(p, d).await?.auszug,
            geschoss,
            schnitthoehe_m,
        )
    }

    pub async fn ifc_geschosse(
        &self,
        p: ProjectId,
        d: &Dokumentbezug,
    ) -> CoreResult<Vec<super::Modellgeschoss>> {
        Ok(super::geschossliste(&self.ifc_modell(p, d).await?.auszug))
    }

    /// Grundriss-Schnitt als Plan in Metern für IFC-Hintergründe im Zeicheneditor.
    pub async fn ifc_grundriss_plan(
        &self,
        p: ProjectId,
        d: &Dokumentbezug,
        geschoss: &str,
    ) -> CoreResult<crate::cad::Plan> {
        Ok(super::grundriss::als_plan(
            &self.ifc_grundriss(p, d, geschoss, None).await?,
        ))
    }

    /// STEP-Text der Modelldatei (bei `.ifczip` entpackt) für die 3D-Ansicht.
    pub async fn ifc_datei(&self, p: ProjectId, d: &Dokumentbezug) -> CoreResult<Vec<u8>> {
        let name = self.ifc_dokument_pruefen(p, d).await?;
        let (_, daten) = self.load_version(d.dokument_id, d.version).await?;
        if name.to_lowercase().ends_with(".ifczip") || daten.starts_with(b"PK\x03\x04") {
            entpacken(&daten)
        } else {
            Ok(daten.to_vec())
        }
    }

    /// Prüfung beim Speichern einer Modellakte: Bezeichnung, lesbares Modell.
    pub(crate) async fn modellakte_pruefen(&self, p: ProjectId, a: &Modellakte) -> CoreResult<()> {
        crate::plattform::text(&a.bezeichnung, 200)?;
        self.ifc_modell(p, &a.dokument).await?;
        Ok(())
    }
}
