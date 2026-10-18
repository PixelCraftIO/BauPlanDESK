//! PostgreSQL + eingebundene zentrale Ablage. Alle Ziele sind neue Archive bzw.
//! neue Verzeichnisse; Wiederherstellung nur in eine leere Datenbank.
use crate::sicherung::{self, Sicherungsmanifest, Sicherungsquelle};
use core_lib::{CoreError, CoreResult};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
fn err(s: &str) -> CoreError {
    CoreError::Storage(s.into())
}
fn postgres_url(url: &str) -> CoreResult<url::Url> {
    let u = url::Url::parse(url).map_err(|_| err("Ungültige PostgreSQL-Verbindungsadresse"))?;
    if !matches!(u.scheme(), "postgres" | "postgresql")
        || u.host_str().is_none()
        || u.path().trim_matches('/').is_empty()
    {
        return Err(err("PostgreSQL-Verbindungsadresse mit Server erforderlich"));
    }
    Ok(u)
}
fn dekodieren(s: &str) -> CoreResult<String> {
    let mut out = Vec::new();
    let mut it = s.as_bytes().iter();
    while let Some(c) = it.next() {
        if *c == b'%' {
            let a = it.next().and_then(|c| (*c as char).to_digit(16));
            let b = it.next().and_then(|c| (*c as char).to_digit(16));
            if let (Some(a), Some(b)) = (a, b) {
                out.push((a * 16 + b) as u8);
            } else {
                return Err(err("Ungültige URL-Kodierung"));
            }
        } else {
            out.push(*c);
        }
    }
    String::from_utf8(out).map_err(|_| err("Ungültiger UTF-8-Verbindungsparameter"))
}
fn pg(
    program: &str,
    args: &[&std::ffi::OsStr],
    url: &str,
    stdout: Option<&Path>,
) -> CoreResult<()> {
    let u = postgres_url(url)?;
    let mut c = Command::new(program);
    c.env("PGHOST", u.host_str().unwrap_or_default())
        .env("PGPORT", u.port().unwrap_or(5432).to_string())
        .env("PGDATABASE", dekodieren(u.path().trim_start_matches('/'))?);
    if !u.username().is_empty() {
        c.env("PGUSER", dekodieren(u.username())?);
    }
    if let Some(p) = u.password() {
        c.env("PGPASSWORD", dekodieren(p)?);
    }
    for (k, v) in u.query_pairs() {
        let env = match k.as_ref() {
            "sslmode" => "PGSSLMODE",
            "sslrootcert" => "PGSSLROOTCERT",
            "sslcert" => "PGSSLCERT",
            "sslkey" => "PGSSLKEY",
            "application_name" => "PGAPPNAME",
            "connect_timeout" => "PGCONNECT_TIMEOUT",
            "options" => "PGOPTIONS",
            "target_session_attrs" => "PGTARGETSESSIONATTRS",
            "channel_binding" => "PGCHANNELBINDING",
            _ => return Err(err("Nicht unterstützter PostgreSQL-URL-Parameter")),
        };
        c.env(env, v.as_ref());
    }
    c.args(args)
        .env("PGCONNECT_TIMEOUT", "10")
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    if let Some(p) = stdout {
        let f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(p)
            .map_err(|_| err("Temporäre Ausgabedatei nicht anlegbar"))?;
        c.stdout(f);
    } else {
        c.stdout(Stdio::null());
    }
    let mut child=c.spawn().map_err(|_|err("PostgreSQL-Clientprogramme pg_dump, pg_restore und psql müssen installiert und im PATH sein"))?;
    let start = Instant::now();
    loop {
        if let Some(s) = child
            .try_wait()
            .map_err(|_| err("PostgreSQL-Programmstatus nicht lesbar"))?
        {
            return if s.success() {
                Ok(())
            } else {
                Err(err(
                    "PostgreSQL-Sicherung/Wiederherstellung fehlgeschlagen. Verbindung, Zugriffsrechte und Clientversion prüfen.",
                ))
            };
        }
        if start.elapsed() > Duration::from_secs(300) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(err("PostgreSQL-Vorgang hat das Zeitlimit überschritten"));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
fn private_write(p: &Path, bytes: &[u8]) -> CoreResult<()> {
    use std::io::Write;
    let mut o = fs::OpenOptions::new();
    o.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(p)
        .and_then(|mut f| f.write_all(bytes))
        .map_err(|_| err("Private Konfigurationsdatei nicht schreibbar"))
}
pub fn erstellen(
    url: &str,
    root: &Path,
    config: &Path,
    vorlagen: Option<&Path>,
    ziel: &Path,
) -> CoreResult<Sicherungsmanifest> {
    postgres_url(url)?;
    if !root.is_dir() {
        return Err(err("Zentrale Ablage nicht erreichbar"));
    }
    let scratch = std::env::temp_dir().join(format!("bauprojekt-buero-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&scratch).map_err(|_| err("Temporäres Verzeichnis nicht anlegbar"))?;
    let result = (|| {
        let dump = scratch.join("postgres.dump");
        pg(
            "pg_dump",
            &[
                "--format=custom".as_ref(),
                "--no-owner".as_ref(),
                "--no-acl".as_ref(),
                "--no-password".as_ref(),
                "--file".as_ref(),
                dump.as_os_str(),
            ],
            url,
            None,
        )?;
        // Datenbanksnapshot zuerst, danach alle unveränderlichen Dateien kopieren.
        let mut quellen = vec![Sicherungsquelle {
            name: "postgres.dump".into(),
            pfad: dump,
            erwartet: None,
        }];
        // Nur `data/` und `meta/`; `tmp/` ändert sich bei laufender Arbeit.
        quellen.extend(sicherung::dateiablage(root, "dateien", Default::default())?);
        if config.is_dir() {
            quellen.extend(sicherung::verzeichnis(config, "einstellungen")?);
        }
        if let Some(v) = vorlagen
            && v.is_dir()
        {
            quellen.extend(sicherung::verzeichnis(v, "Formularvorlagen")?);
        }
        sicherung::erstellen(ziel, &quellen)?;
        sicherung::pruefen(ziel)
    })();
    let _ = fs::remove_dir_all(scratch);
    result
}
pub fn wiederherstellen(archiv: &Path, ziel: &Path, url: &str) -> CoreResult<PathBuf> {
    postgres_url(url)?;
    if !ziel.is_absolute() || ziel.exists() {
        return Err(err("Neues absolutes Zielverzeichnis erforderlich"));
    }
    let m = sicherung::pruefen(archiv)?;
    if !m.dateien.iter().any(|f| f.name == "postgres.dump") {
        return Err(err("Archiv enthält keine PostgreSQL-Bürosicherung"));
    }
    sicherung::wiederherstellen(archiv, ziel)?;
    let out = ziel.join("leere-datenbank.txt");
    pg("psql",&["-X".as_ref(),"-A".as_ref(),"-t".as_ref(),"--no-password".as_ref(),"-c".as_ref(),"SELECT count(*) FROM information_schema.tables WHERE table_schema NOT IN ('pg_catalog','information_schema')".as_ref()],url,Some(&out))?;
    let leer = fs::read_to_string(&out)
        .map_err(|_| err("Zieldatenbank nicht prüfbar"))?
        .trim()
        == "0";
    let _ = fs::remove_file(out);
    if !leer {
        return Err(err(
            "Wiederherstellung abgebrochen: Zieldatenbank ist nicht leer",
        ));
    }
    // Nur der Datenbankname ist ein Argument, Zugangsdaten bleiben in PG*.
    let dbname = dekodieren(postgres_url(url)?.path().trim_start_matches('/'))?;
    pg(
        "pg_restore",
        &[
            "--dbname".as_ref(),
            dbname.as_ref(),
            "--single-transaction".as_ref(),
            "--exit-on-error".as_ref(),
            "--no-owner".as_ref(),
            "--no-acl".as_ref(),
            "--no-password".as_ref(),
            ziel.join("postgres.dump").as_os_str(),
        ],
        url,
        None,
    )?;
    let config = ziel.join("einstellungen");
    fs::create_dir_all(&config).map_err(|_| err("Profilkonfiguration nicht anlegbar"))?;
    let neu = config.join("config-neu.json");
    private_write(&neu,&serde_json::to_vec_pretty(&serde_json::json!({"modus":"netzwerk","database_url":url,"storage":{"type":"nas","root":ziel.join("dateien")},"cache_max_gb":10})).map_err(|_|err("Profilkonfiguration ungültig"))?)?;
    fs::rename(&neu, config.join("config.json"))
        .map_err(|_| err("Neue Profilkonfiguration nicht aktivierbar"))?;
    for (datei, ordner) in [
        ("dateiablage.json", "dateien"),
        ("formularbibliothek.json", "Formularvorlagen"),
        ("projektordner.json", "Projektordner"),
    ] {
        fs::create_dir_all(ziel.join(ordner))
            .map_err(|_| err("Neuer Ablageordner nicht anlegbar"))?;
        let neu = config.join(format!("neu-{datei}"));
        private_write(
            &neu,
            &serde_json::to_vec(&serde_json::json!({"root":ziel.join(ordner)}))
                .map_err(|_| err("Ablagekonfiguration ungültig"))?,
        )?;
        fs::rename(neu, config.join(datei))
            .map_err(|_| err("Ablagekonfiguration nicht aktualisierbar"))?;
    }
    Ok(ziel.to_path_buf())
}
