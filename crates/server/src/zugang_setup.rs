//! Lokale Bereitstellung eines Benutzerzugangs durch den Serverbetreiber.
use base64::Engine;
use core_lib::{
    CoreServices,
    benutzer::{BenutzerAenderung, Rolle},
    storage::ContentHash,
};
use server::api::zugang::{Zugaenge, Zugang};
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};
pub async fn ausfuehren(
    core: &CoreServices,
    args: &[String],
) -> Result<bool, Box<dyn std::error::Error>> {
    if args.get(1).is_none_or(|a| a != "zugang-anlegen") {
        return Ok(false);
    }
    if args.len() != 5 {
        return Err("Aufruf: bauprojekt-server zugang-anlegen BENUTZER ROLLE TOKEN-DATEI".into());
    }
    let name = &args[2];
    let rolle: Rolle = args[3].parse()?;
    let tokenpfad = PathBuf::from(&args[4]);
    if !tokenpfad.is_absolute() {
        return Err("Token-Datei mit absolutem Pfad angeben".into());
    }
    let listepfad = PathBuf::from(
        std::env::var("BAUPROJEKT_ZUGANGSDATEI").map_err(|_| "BAUPROJEKT_ZUGANGSDATEI fehlt")?,
    );
    let mut liste = match std::fs::read(&listepfad) {
        Ok(raw) => serde_json::from_slice::<Zugaenge>(&raw)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Zugaenge {
            schema_version: 1,
            zugaenge: vec![],
        },
        Err(e) => return Err(e.into()),
    };
    if liste.schema_version != 1 {
        return Err("Unbekannte Zugangsschemaversion".into());
    }
    if tokenpfad.exists() {
        return Err(
            "Token-Datei existiert bereits; vorhandene Schlüssel werden nicht überschrieben".into(),
        );
    }
    let benutzer = core.repos.benutzer.list().await?;
    let administratoren: Vec<_> = benutzer
        .iter()
        .filter(|b| b.aktiv && b.rolle == Rolle::Administrator)
        .collect();
    if administratoren.is_empty() && rolle != Rolle::Administrator {
        return Err("Der erste Serverzugang muss ein Administrator sein".into());
    }
    if rolle != Rolle::Administrator
        && administratoren.len() == 1
        && administratoren[0].anmeldename.eq_ignore_ascii_case(name)
    {
        return Err("Der letzte aktive Administrator darf nicht herabgestuft werden".into());
    }
    let b = core.anmelden(name, name).await?;
    let b = core
        .repos
        .benutzer
        .update(BenutzerAenderung {
            id: b.id,
            anzeigename: b.anzeigename,
            rolle,
            aktiv: true,
        })
        .await?;
    let mut zufall = [0u8; 32];
    getrandom::fill(&mut zufall).map_err(|e| format!("Zufallsquelle nicht verfügbar: {e}"))?;
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(zufall);
    if let Some(parent) = tokenpfad.parent() {
        std::fs::create_dir_all(parent)?;
    }
    geheim_schreiben(&tokenpfad, token.as_bytes())?;
    liste.zugaenge.retain(|z| z.benutzer_id != b.id);
    liste.zugaenge.push(Zugang {
        benutzer_id: b.id,
        token_sha256: ContentHash::of(token.as_bytes()),
        gueltig_bis: None,
    });
    let speichern = (|| -> Result<(), Box<dyn std::error::Error>> {
        let parent = listepfad.parent().ok_or("Zugangsverzeichnis fehlt")?;
        std::fs::create_dir_all(parent)?;
        let tmp = parent.join(format!(".zugang-{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            geheim_schreiben(&tmp, &serde_json::to_vec_pretty(&liste)?)?;
            std::fs::rename(&tmp, &listepfad)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(tmp);
        }
        result
    })();
    if let Err(e) = speichern {
        let _ = std::fs::remove_file(&tokenpfad);
        return Err(e);
    }
    println!(
        "Zugang für {} eingerichtet. Token liegt ausschließlich in {}. Frühere Tokens dieses Benutzers sind widerrufen.",
        b.anzeigename,
        tokenpfad.display()
    );
    Ok(true)
}
fn geheim_schreiben(p: &Path, data: &[u8]) -> std::io::Result<()> {
    let mut o = OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    let mut f = o.open(p)?;
    f.write_all(data)?;
    f.sync_all()
}
