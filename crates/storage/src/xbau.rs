//! Plattformadapter für XSD 1.0. Nur eingebettete Schemas, keine fremden Pfade.
use async_trait::async_trait;
use core_lib::{CoreError, CoreResult, xbau::austausch::SchemaPruefer};
use std::{path::PathBuf, process::Stdio, sync::Arc};
use tokio::{io::AsyncReadExt, sync::Semaphore};
include!(concat!(env!("OUT_DIR"), "/xbau_schemas.rs"));
pub struct XmlLintPruefer {
    programm: PathBuf,
    parallel: Semaphore,
}
impl XmlLintPruefer {
    pub fn neu(programm: PathBuf) -> Self {
        Self {
            programm,
            parallel: Semaphore::new(2),
        }
    }
    pub fn standard() -> Arc<dyn SchemaPruefer> {
        Arc::new(Self::neu(
            std::env::var_os("BAUPROJEKT_XMLLINT")
                .map(PathBuf::from)
                .unwrap_or_else(|| "xmllint".into()),
        ))
    }
}
#[async_trait]
impl SchemaPruefer for XmlLintPruefer {
    async fn pruefen(&self, xml: &[u8]) -> CoreResult<String> {
        core_lib::xbau::austausch::vorpruefen(xml)?;
        let _slot = self
            .parallel
            .acquire()
            .await
            .map_err(|_| CoreError::Storage("XBau-Prüfer beendet".into()))?;
        if xml.len() as u64 > core_lib::xbau::austausch::MAX_XML {
            return Err(CoreError::InvalidInput("XBau-XML größer als 8 MiB".into()));
        }
        let dir = tempfile::tempdir().map_err(io)?;
        for (name, bytes) in SCHEMAS {
            let path = dir.path().join(name);
            if let Some(p) = path.parent() {
                tokio::fs::create_dir_all(p).await.map_err(io)?;
            }
            tokio::fs::write(path, bytes).await.map_err(io)?;
        }
        // stdout wird verworfen, Diagnosen landen in einer begrenzt gelesenen Datei.
        // Die zu prüfende Datei liegt getrennt von den vertrauenswürdigen Schemas.
        tokio::fs::write(dir.path().join("nachricht.xml"), xml)
            .await
            .map_err(io)?;
        let err = std::fs::File::create(dir.path().join("diagnose.txt")).map_err(io)?;
        let mut command = tokio::process::Command::new(&self.programm);
        command
            .current_dir(dir.path())
            .args([
                "--noout",
                "--nonet",
                "--maxmem",
                "67108864",
                "--schema",
                "austausch.xsd",
                "nachricht.xml",
            ])
            .env_remove("XML_CATALOG_FILES")
            .env_remove("SGML_CATALOG_FILES")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(err))
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        let mut child=command.spawn().map_err(|_|CoreError::InvalidInput("XBau-XSD-Prüfer nicht verfügbar. xmllint installieren oder BAUPROJEKT_XMLLINT auf eine vertrauenswürdige ausführbare Datei setzen.".into()))?;
        let status =
            match tokio::time::timeout(std::time::Duration::from_secs(15), child.wait()).await {
                Ok(r) => r.map_err(io)?,
                Err(_) => {
                    let _ = child.kill().await;
                    return Err(CoreError::InvalidInput(
                        "Zeitlimit der XBau-XSD-Prüfung überschritten".into(),
                    ));
                }
            };
        if !status.success() {
            let mut b = vec![];
            tokio::fs::File::open(dir.path().join("diagnose.txt"))
                .await
                .map_err(io)?
                .take(8192)
                .read_to_end(&mut b)
                .await
                .map_err(io)?;
            let diag = String::from_utf8_lossy(&b)
                .replace(dir.path().to_string_lossy().as_ref(), "[Schemas]");
            return Err(CoreError::InvalidInput(format!(
                "XBau-XSD-Prüfung fehlgeschlagen: {diag}"
            )));
        }
        Ok("libxml2/xmllint · XBau-Hochbau 2.7 / Kernmodul 1.4.0".into())
    }
}
fn io(e: std::io::Error) -> CoreError {
    CoreError::Storage(format!("XBau-Prüfer: {e}"))
}
