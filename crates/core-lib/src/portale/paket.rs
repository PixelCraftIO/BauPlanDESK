use super::*;
use crate::{
    CoreServices,
    domain::ProjectId,
    storage::{ContentHash, StorageKey},
};
use std::io::{Cursor, Write};
fn csv(s: &str) -> String {
    let s = if s.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        format!("'{s}")
    } else {
        s.to_string()
    };
    format!("\"{}\"", s.replace('"', "\"\""))
}
pub(super) async fn erzeugen(
    core: &CoreServices,
    p: ProjectId,
    v: &Portalvorgang,
    b: &Portalabgabe,
) -> CoreResult<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(Cursor::new(vec![]));
    let opt = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default());
    let mut zeilen = String::from(
        "\u{feff}Uploadname;Originalname;Kategorie;Version;Bytes;SHA-256;Portal-Unterlagenart;Vergleich\r\n",
    );
    for d in &b.dateien {
        let (meta, bytes) = core
            .load_version(d.original.dokument.dokument_id, d.original.dokument.version)
            .await?;
        if meta.project_id != p
            || meta.sha256.as_str() != d.original.sha256
            || meta.size_bytes != d.original.bytes
        {
            return Err(fehler(
                "Originaldatei passt nicht zum eingefrorenen Abgabestand",
            ));
        }
        zip.start_file(format!("Unterlagen/{}", d.uploadname), opt)
            .map_err(|e| fehler(e.to_string()))?;
        zip.write_all(&bytes).map_err(|e| fehler(e.to_string()))?;
        zeilen.push_str(
            &[
                d.uploadname.clone(),
                d.original.name.clone(),
                d.original
                    .kategorie
                    .as_ref()
                    .map(|k| k.name.clone())
                    .unwrap_or_else(|| "Nicht erfasst".into()),
                d.original.dokument.version.to_string(),
                d.original.bytes.to_string(),
                d.original.sha256.clone(),
                d.unterlagenart
                    .clone()
                    .unwrap_or_else(|| "Nicht profiliert".into()),
                format!("{:?}", d.vergleich),
            ]
            .iter()
            .map(|s| csv(s))
            .collect::<Vec<_>>()
            .join(";"),
        );
        zeilen.push_str("\r\n");
    }
    let manifest=serde_json::to_vec_pretty(&serde_json::json!({"schema_version":1,"zweck":"Lokale Portalbereitstellung, kein bestätigtes Portalimportformat","projekt":p,"portalvorgang":v.id,"profil":v.profil,"ziel":v.ziel,"abgabestand":b.abgabe,"art":b.art,"vorgaenger":b.vorgaenger,"forderungen":b.forderungen,"dateien":b.dateien,"pruefung":b.pruefung})).map_err(|e|fehler(e.to_string()))?;
    let text = format!(
        "Portalunterlagen – lokale Bereitstellung\n\nZiel: {} / {}\nPortal: {}\nProfil: {} / {}\n\nDie Originaldateien liegen unter Unterlagen. Nur die vom Portal geforderten Dateien hochladen. Dieses ZIP, das Manifest und die CSV-Dateiliste sind keine bestätigten Portalimportformate.\n\nEs wurde kein Antrag versendet. Den Originalbeleg und die tatsächlich hochgeladenen Dateiversionen nach der verbindlichen Portalabgabe abgleichen. Änderungen im Portal sind nicht automatisch in diesem Paket enthalten.\n\nPrüfhinweise:\n{}\n",
        v.ziel.kommune,
        v.ziel.stelle,
        v.ziel.url,
        v.profil.titel,
        v.profil.version,
        b.pruefung.hinweise.join("\n")
    );
    for (name, bytes) in [
        ("Dateiliste.csv", zeilen.as_bytes()),
        ("Manifest.json", manifest.as_slice()),
        ("Liesmich.txt", text.as_bytes()),
    ] {
        zip.start_file(name, opt)
            .map_err(|e| fehler(e.to_string()))?;
        zip.write_all(bytes).map_err(|e| fehler(e.to_string()))?;
    }
    Ok(zip
        .finish()
        .map_err(|e| fehler(e.to_string()))?
        .into_inner())
}
pub(super) async fn laden(core: &CoreServices, art: &Artefakt) -> CoreResult<Vec<u8>> {
    let hash = ContentHash::try_from(art.sha256.clone())?;
    let b = core
        .storage
        .load_verified(&StorageKey::new(&art.storage_key)?, &hash)
        .await?;
    if b.len() as u64 != art.bytes {
        return Err(fehler("Bereitstellungsgröße stimmt nicht"));
    }
    Ok(b.to_vec())
}
