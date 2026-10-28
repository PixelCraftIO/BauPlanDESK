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
        let mut a = self.ifc_modell(p, &akte.dokument).await?.auszug.clone();
        super::nacharbeit::anwenden(&mut a, &akte.nacharbeit);
        Ok(a)
    }

    /// Grundriss-Schnitt der Modellakte (mit Nacharbeit).
    pub async fn ifc_akte_grundriss(
        &self,
        p: ProjectId,
        akte_id: uuid::Uuid,
        geschoss: &str,
        schnitthoehe_m: Option<f64>,
    ) -> CoreResult<super::Modellgrundriss> {
        let akte = self.modellakte(p, akte_id).await?;
        let a = self.ifc_akte_auszug(p, &akte).await?;
        super::grundriss(&a, geschoss, schnitthoehe_m)
    }

    /// Raumvorschläge aus den Wänden eines Geschosses der Modellakte.
    pub async fn ifc_raumvorschlaege(
        &self,
        p: ProjectId,
        akte_id: uuid::Uuid,
        geschoss: &str,
    ) -> CoreResult<Vec<super::Raumvorschlag>> {
        let akte = self.modellakte(p, akte_id).await?;
        let a = self.ifc_akte_auszug(p, &akte).await?;
        super::raumvorschlaege(&a, geschoss)
    }

    /// Prüft die Modellakte gegen das mitgelieferte IDS oder ein IDS aus den
    /// Projektdateien, dazu die eingebauten Regeln.
    pub async fn ifc_pruefen(
        &self,
        p: ProjectId,
        akte_id: uuid::Uuid,
        ids: Option<&Dokumentbezug>,
    ) -> CoreResult<super::Modellpruefung> {
        let akte = self.modellakte(p, akte_id).await?;
        let a = self.ifc_akte_auszug(p, &akte).await?;
        let text = match ids {
            None => super::BAUANTRAG_IDS.to_string(),
            Some(d) => {
                let v = self
                    .repos
                    .documents
                    .history(d.dokument_id)
                    .await?
                    .into_iter()
                    .find(|v| v.version == d.version && v.project_id == p)
                    .ok_or_else(|| {
                        CoreError::InvalidInput("IDS-Datei gehört nicht zu diesem Projekt".into())
                    })?;
                if !v.file_name.to_ascii_lowercase().ends_with(".ids") {
                    return Err(CoreError::InvalidInput(format!(
                        "„{}“ ist keine IDS-Datei (.ids)",
                        v.file_name
                    )));
                }
                let (_, daten) = self.load_version(d.dokument_id, d.version).await?;
                String::from_utf8(daten.to_vec())
                    .map_err(|_| CoreError::InvalidInput("IDS-Datei ist kein UTF-8".into()))?
            }
        };
        super::pruefen(&a, &text)
    }

    /// Gleicht eine neue Modellversion gegen das Modell der Akte ab.
    pub async fn ifc_abgleich(
        &self,
        p: ProjectId,
        akte_id: uuid::Uuid,
        neu: &Dokumentbezug,
    ) -> CoreResult<super::Modellabgleich> {
        let akte = self.modellakte(p, akte_id).await?;
        let alt = self.ifc_modell(p, &akte.dokument).await?;
        let neu = self.ifc_modell(p, neu).await?;
        Ok(super::abgleichen(
            &alt.auszug,
            &neu.auszug,
            &akte.nacharbeit,
        ))
    }

    /// Stellt die Modellakte auf die neue Version um; die Nacharbeit wandert
    /// mit, auf Wunsch ohne überholte oder verwaiste Einträge.
    pub async fn ifc_reimport(
        &self,
        p: ProjectId,
        w: &super::Reimportwunsch,
        wer: &crate::benutzer::Benutzer,
    ) -> CoreResult<crate::commit::Fachstand> {
        let f = self
            .fachobjekt_lesen(p, crate::plattform::ObjektArt::Ifcmodell, w.akte_id)
            .await?
            .ok_or_else(|| CoreError::InvalidInput("Modellakte fehlt im Projekt".into()))?;
        if f.revision != w.revision {
            return Err(CoreError::InvalidInput(
                "Modellakte wurde inzwischen geändert. Bitte neu laden.".into(),
            ));
        }
        let mut akte: Modellakte =
            serde_json::from_value(f.daten).map_err(|e| CoreError::InvalidInput(e.to_string()))?;
        if akte.dokument == w.dokument {
            return Err(CoreError::InvalidInput(
                "Die Modellakte verwendet diese Version bereits".into(),
            ));
        }
        let v = self.ifc_abgleich(p, w.akte_id, &w.dokument).await?;
        let alt = std::mem::take(&mut akte.nacharbeit);
        akte.nacharbeit = alt
            .into_iter()
            .zip(v.nacharbeit)
            .filter(|(_, s)| match s {
                super::Nacharbeitsstand::Ueberholt => !w.ueberholte_entfernen,
                super::Nacharbeitsstand::Verwaist => !w.verwaiste_entfernen,
                super::Nacharbeitsstand::Wirksam => true,
            })
            .map(|(c, _)| c)
            .collect();
        akte.dokument = w.dokument.clone();
        self.fachobjekt_speichern(
            p,
            crate::plattform::ObjektArt::Ifcmodell,
            w.akte_id,
            w.revision,
            serde_json::to_value(&akte).map_err(|e| CoreError::InvalidInput(e.to_string()))?,
            wer,
        )
        .await
    }

    /// Modellakte lesen.
    pub async fn modellakte(&self, p: ProjectId, id: uuid::Uuid) -> CoreResult<Modellakte> {
        let f = self
            .fachobjekt_lesen(p, crate::plattform::ObjektArt::Ifcmodell, id)
            .await?
            .ok_or_else(|| CoreError::InvalidInput("Modellakte fehlt im Projekt".into()))?;
        serde_json::from_value(f.daten).map_err(|e| CoreError::InvalidInput(e.to_string()))
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
        let m = self.ifc_modell(p, &a.dokument).await?;
        super::nacharbeit::pruefen(&m.auszug, &a.nacharbeit)
    }
}
