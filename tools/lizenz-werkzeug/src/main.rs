//! Hersteller-Werkzeug für Lizenzen. Läuft nur beim Hersteller, nie beim
//! Kunden: Der geheime Schlüssel darf weder ins Repository noch in die App.
//!
//! ```text
//! lizenz-werkzeug schluessel <geheim.key>
//! lizenz-werkzeug ausstellen <geheim.key> <ausgabe.bplizenz> --lizenznehmer "Büro Muster"
//!     [--edition einzelplatz|buero|cloud] [--arbeitsplaetze 1]
//!     [--gueltig-bis JJJJ-MM-TT] [--wartung-bis JJJJ-MM-TT]
//! lizenz-werkzeug pruefen <datei.bplizenz> <öffentlicher Schlüssel (Base64)>
//! ```

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use chrono::{NaiveDate, Utc};
use core_lib::license::datei::{ausstellen, oeffentlich_base64};
use core_lib::license::{Edition, Lizenz, Lizenzpruefer};
use ed25519_dalek::SigningKey;

type Ergebnis<T> = Result<T, String>;

const HILFE: &str = "\
Verwendung:
  lizenz-werkzeug schluessel <geheim.key>
      Erzeugt ein neues Schlüsselpaar. Die Datei enthält den GEHEIMEN Schlüssel:
      sicher aufbewahren, nie einchecken. Der ausgegebene öffentliche Schlüssel
      kommt beim Release-Build in BAUPROJEKT_LIZENZ_SCHLUESSEL.

  lizenz-werkzeug ausstellen <geheim.key> <ausgabe.bplizenz> --lizenznehmer NAME
      [--edition einzelplatz|buero|cloud]   (Standard: einzelplatz)
      [--arbeitsplaetze N]                  (Standard: 1)
      [--gueltig-bis JJJJ-MM-TT]            (Standard: unbefristet)
      [--wartung-bis JJJJ-MM-TT]            (Updates bis, Standard: keine Angabe)

  lizenz-werkzeug pruefen <datei.bplizenz> <öffentlicher Schlüssel>
      Zeigt den Inhalt einer Lizenzdatei, wenn die Signatur passt.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match ausfuehren(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Fehler: {e}");
            ExitCode::FAILURE
        }
    }
}

fn ausfuehren(args: &[String]) -> Ergebnis<()> {
    match args.first().map(String::as_str) {
        Some("schluessel") => match &args[1..] {
            [datei] => schluessel_erzeugen(Path::new(datei)),
            _ => Err(HILFE.into()),
        },
        Some("ausstellen") => {
            let (pos, opt) = aufteilen(&args[1..])?;
            match pos.as_slice() {
                [geheim, ausgabe] => lizenz_ausstellen(Path::new(geheim), Path::new(ausgabe), &opt),
                _ => Err(HILFE.into()),
            }
        }
        Some("pruefen") => match &args[1..] {
            [datei, schluessel] => pruefen(Path::new(datei), schluessel),
            _ => Err(HILFE.into()),
        },
        _ => {
            println!("{HILFE}");
            Ok(())
        }
    }
}

/// Trennt Positionsargumente von `--name wert`.
fn aufteilen(args: &[String]) -> Ergebnis<(Vec<String>, HashMap<String, String>)> {
    let mut pos = Vec::new();
    let mut opt = HashMap::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if let Some(name) = a.strip_prefix("--") {
            let wert = it
                .next()
                .ok_or_else(|| format!("--{name} braucht einen Wert"))?;
            opt.insert(name.to_string(), wert.clone());
        } else {
            pos.push(a.clone());
        }
    }
    Ok((pos, opt))
}

fn schluessel_erzeugen(datei: &Path) -> Ergebnis<()> {
    let mut geheim = [0u8; 32];
    getrandom::fill(&mut geheim).map_err(|e| format!("Zufallszahlen: {e}"))?;
    let schluessel = SigningKey::from_bytes(&geheim);
    // create_new: einen vorhandenen Schlüssel nie überschreiben, sonst
    // passen alle damit ausgestellten Lizenzen nicht mehr zur App.
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(datei)
        .map_err(|e| format!("{}: {e}", datei.display()))?;
    writeln!(f, "{}", B64.encode(geheim)).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(datei, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    }
    println!("Geheimer Schlüssel: {}", datei.display());
    println!("Öffentlicher Schlüssel (für BAUPROJEKT_LIZENZ_SCHLUESSEL):");
    println!("{}", oeffentlich_base64(&schluessel));
    Ok(())
}

fn geheimer_schluessel(datei: &Path) -> Ergebnis<SigningKey> {
    let text = fs::read_to_string(datei).map_err(|e| format!("{}: {e}", datei.display()))?;
    let bytes: [u8; 32] = B64
        .decode(text.trim())
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or("Die Schlüsseldatei ist ungültig.")?;
    Ok(SigningKey::from_bytes(&bytes))
}

fn datum(opt: &HashMap<String, String>, name: &str) -> Ergebnis<Option<NaiveDate>> {
    opt.get(name)
        .map(|d| {
            NaiveDate::parse_from_str(d, "%Y-%m-%d")
                .map_err(|_| format!("--{name}: Datum als JJJJ-MM-TT angeben"))
        })
        .transpose()
}

fn lizenz_ausstellen(geheim: &Path, ausgabe: &Path, opt: &HashMap<String, String>) -> Ergebnis<()> {
    for name in opt.keys() {
        if ![
            "lizenznehmer",
            "edition",
            "arbeitsplaetze",
            "gueltig-bis",
            "wartung-bis",
        ]
        .contains(&name.as_str())
        {
            return Err(format!("unbekannte Option --{name}"));
        }
    }
    let lizenznehmer = opt
        .get("lizenznehmer")
        .filter(|s| !s.trim().is_empty())
        .ok_or("--lizenznehmer fehlt")?
        .trim()
        .to_string();
    let edition = match opt.get("edition").map(String::as_str) {
        None | Some("einzelplatz") => Edition::Einzelplatz,
        Some("buero") => Edition::Buero,
        Some("cloud") => Edition::Cloud,
        Some(e) => return Err(format!("unbekannte Edition: {e}")),
    };
    let arbeitsplaetze: u32 = opt
        .get("arbeitsplaetze")
        .map(|n| n.parse().map_err(|_| "--arbeitsplaetze: Zahl angeben"))
        .transpose()?
        .unwrap_or(1);
    if arbeitsplaetze == 0 {
        return Err("--arbeitsplaetze muss mindestens 1 sein".into());
    }
    let lizenz = Lizenz {
        lizenz_id: uuid::Uuid::new_v4(),
        lizenznehmer,
        edition,
        arbeitsplaetze,
        ausgestellt_am: Utc::now().date_naive(),
        gueltig_bis: datum(opt, "gueltig-bis")?,
        wartung_bis: datum(opt, "wartung-bis")?,
    };
    let schluessel = geheimer_schluessel(geheim)?;
    let inhalt = ausstellen(&lizenz, &schluessel);
    // Selbstprüfung mit dem passenden öffentlichen Schlüssel.
    Lizenzpruefer::new(vec![schluessel.verifying_key()])
        .pruefen(inhalt.as_bytes())
        .map_err(|e| e.to_string())?;
    fs::write(ausgabe, inhalt).map_err(|e| format!("{}: {e}", ausgabe.display()))?;
    println!(
        "Lizenz {} ausgestellt: {}",
        lizenz.lizenz_id,
        ausgabe.display()
    );
    anzeigen(&lizenz);
    Ok(())
}

fn pruefen(datei: &Path, schluessel: &str) -> Ergebnis<()> {
    let inhalt = fs::read(datei).map_err(|e| format!("{}: {e}", datei.display()))?;
    let lizenz = Lizenzpruefer::aus_base64([schluessel])
        .and_then(|p| p.pruefen(&inhalt))
        .map_err(|e| e.to_string())?;
    println!("Signatur gültig.");
    anzeigen(&lizenz);
    Ok(())
}

fn anzeigen(l: &Lizenz) {
    let datum = |d: Option<NaiveDate>, sonst: &str| {
        d.map_or(sonst.to_string(), |d| d.format("%d.%m.%Y").to_string())
    };
    println!("  Lizenznehmer:   {}", l.lizenznehmer);
    println!("  Edition:        {}", l.edition.name());
    println!("  Arbeitsplätze:  {}", l.arbeitsplaetze);
    println!("  Gültig bis:     {}", datum(l.gueltig_bis, "unbefristet"));
    println!("  Wartung bis:    {}", datum(l.wartung_bis, "–"));
}
