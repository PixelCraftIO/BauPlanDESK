# BauProjektIO

Projektmanagement- und Bauantrags-Software für Architekten: Berechnungen nach
BauGB/BauNVO und den 16 Landesbauordnungen, amtliche PDF-Formulare, CAD-Viewer,
XBau-Export und DMS. Läuft als native Desktop-App (Einzelplatz), als
NAS-/Büro-Server und optional in der Cloud.

## Architektur

```
                ┌───────────────────────── core-lib ─────────────────────────┐
                │ Domänenmodell · BauNVO-Berechnungen · LBO-Engine (Rhai)    │
                │ Traits: ProjectRepository, DocumentRepository, Storage,    │
                │         LicenseProvider            → CoreServices          │
                └──────────────▲──────────────────────────────▲──────────────┘
                               │ unverändert eingebunden       │
      ┌────────────────────────┴───────────┐     ┌─────────────┴──────────────┐
      │ apps/desktop (Tauri + Svelte)      │     │ crates/server (axum)       │
      │  Einzelplatz: db-sqlite + local    │     │  db-postgres | db-sqlite   │
      │  Netzwerk:    db-postgres + NAS/S3 │     │  local | nas | s3          │
      │               └─ client-cache      │     │                            │
      └────────────────────────────────────┘     └────────────────────────────┘
```

Die vier Architekturprinzipien und wo sie umgesetzt sind:

| Prinzip | Umsetzung |
|---|---|
| **Library-First** | Sämtliche Geschäftslogik in [`crates/core-lib`](crates/core-lib). Tauri-Commands ([`commands.rs`](apps/desktop/src-tauri/src/commands.rs)) und axum-Handler ([`api.rs`](crates/server/src/api.rs)) rufen nur `CoreServices` auf. core-lib hängt weder von sqlx noch von Tauri oder axum ab. |
| **Datenbank-Abstraktion** | Traits in [`repository.rs`](crates/core-lib/src/repository.rs); Implementierungen in [`db-sqlite`](crates/db-sqlite) und [`db-postgres`](crates/db-postgres), injiziert als `Arc<dyn …>`. Optimistic Locking über eine `version`-Spalte (`UPDATE … WHERE id = ? AND version = ?`). |
| **Storage-Agnostik** | Trait `Storage` in [`storage.rs`](crates/core-lib/src/storage.rs); Backends `local`, `nas`, `s3` in [`crates/storage`](crates/storage), ausgewählt über `StorageConfig`. Jedes Backend speichert den SHA-256 mit. |
| **Lokales Caching** | [`crates/client-cache`](crates/client-cache): `CachedStorage` umhüllt im Netzwerkmodus jedes entfernte `Storage`. Inhaltsadressiert (Blob = SHA-256), Hash-Prüfung bei jedem Download und beim ersten Zugriff nach App-Start, LRU-Begrenzung, Treffer ohne Netzwerkzugriff. |

## Verzeichnisse

```
Cargo.toml                 Workspace
crates/core-lib            Geschäftslogik (Domäne, calc, lbo, Traits, CoreServices)
crates/db-sqlite           SQLite-Repositories + Migrationen
crates/db-postgres         PostgreSQL-Repositories + Migrationen
crates/storage             Storage-Backends (lokal/NAS, S3-kompatibel)
crates/client-cache        Hash-validierter Datei-Cache für die Desktop-App
crates/server              axum-Server (NAS/Büro/Cloud)
apps/desktop               Svelte-Frontend
apps/desktop/src-tauri     Tauri-App (Fenster, Updater, Bootstrap, Commands)
lbo-rules                  LBO-Regelpakete (Rhai), eines je Bundesland
```

## Entwicklung

Voraussetzungen: Rust (stable, ≥ 1.90), Node.js 22, unter Linux zusätzlich
`libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libsoup-3.0-dev`.

```sh
cargo test --workspace                       # alle Tests
cargo run -p core-lib --example lbo_engine   # Beispiel: LBO-Engine initialisieren

cd apps/desktop && npm install
npm run tauri dev                            # Desktop-App mit Hot-Reload
npm run tauri build                          # Installer (.exe/.msi, .app/.dmg, .deb/.rpm/.AppImage)

cargo run -p server                          # Server (Konfiguration siehe crates/server/src/main.rs)
```

## LBO-Engine

Bundesrecht (BauNVO §§ 17, 19–21) ist nativ in Rust umgesetzt
([`calc.rs`](crates/core-lib/src/calc.rs)). Landesrecht liegt in Rhai-Skripten
unter [`lbo-rules/`](lbo-rules), die zur Laufzeit geladen und ohne
Neukompilierung ausgetauscht werden:

```rust
let engine = LboEngine::new();                         // Sandbox + native Hilfsfunktionen
let (geladen, fehler) = engine.load_directory(dir)?;   // by.rhai, bw.rhai, nw.rhai, …
let r = engine.abstandsflaeche(Bundesland::By, &AbstandsflaecheInput {
    wandhoehe_m: 9.5,
    gebiet: Baugebiet::WA,
})?;
// r.ergebnis.tiefe_m == 3.80, r.regelpaket.version == "2026.1"
```

Ein Regelpaket definiert `meta()` und `abstandsflaeche(input)`. Beim Laden wird
es kompiliert, gegen das erwartete Bundesland geprüft und mit einem Probelauf
getestet; ein fehlerhaftes Update ersetzt nie ein funktionierendes Paket.
Skripte laufen ohne Datei-/Netzzugriff und mit Operations- und Speichergrenzen.
Die Desktop-App lädt zuerst die mitgelieferten Pakete (Installer-Ressourcen),
dann Updates aus `<App-Daten>/lbo-rules/`.

> Die enthaltenen Regelpakete (BY, BW, NW) sind **beispielhaft** und müssen vor
> produktivem Einsatz fachlich und rechtlich geprüft werden.

## PDF-Formulare

[`crates/core-lib/src/formular`](crates/core-lib/src/formular) liest die
Formularfelder (AcroForm) amtlicher PDF-Formulare, schlägt Werte aus den
Projektdaten vor und schreibt das befüllte PDF als neues Dokument ins DMS;
das leere Original bleibt erhalten. Welche Projektdaten in welches Feld gehören,
steht in Vorlagen unter [`formulare/`](formulare) (JSON, ohne Neukompilierung
aktualisierbar). In der App: *Dokumente → Formular ausfüllen …*, danach
*Exportieren …* zum Einreichen.

```sh
cargo run -p core-lib --features testdaten --example musterformular -- muster.pdf
```

Noch offen: eigene Darstellungs-Streams (derzeit setzt die App
`NeedAppearances`, die meisten Viewer zeichnen die Werte dann selbst),
Signaturfelder und verschlüsselte Formulare.

## CAD-Pläne

[`crates/core-lib/src/cad`](crates/core-lib/src/cad) liest DXF-Dateien (ASCII
und binär, AutoCAD R12 bis 2018, über das Crate `dxf`, MIT) in ein neutrales
2D-Modell aus Linienzügen und Beschriftungen. Bögen, Kreise, Ellipsen, Splines
und Polylinien mit Wölbung werden dabei aufgelöst, Blöcke (auch verschachtelt,
gedreht, skaliert, als Array) und Bemaßungen eingesetzt. Längen und Flächen
rechnet `cad::messen` in Meter um; die Einheit kommt aus `$INSUNITS` oder wird
im Viewer gewählt.

In der App: *Ansicht → Pläne* (Strg+Umschalt+P) oder *Dokumente → Plan
ansehen*. Der Viewer zeichnet auf ein Canvas (Mausrad zoomt, Ziehen
verschiebt), blendet Ebenen ein und aus und misst Längen und Flächen mit
Punktfang auf Eckpunkte. *Fläche wählen* übernimmt eine geschlossene Linie wie
die Grundstücksgrenze; ihre Fläche lässt sich als Grundstücksfläche des
Projekts speichern und steht dann unter *Maß der Nutzung* bereit.

```sh
cargo run -p core-lib --features testdaten --example musterplan -- Lageplan.dxf
cargo run -p core-lib --example plan_info -- Lageplan.dxf   # was der Import erkennt
```

**DWG** wird nicht direkt gelesen: Die freien DWG-Bibliotheken (LibreDWG)
stehen unter GPL und sind damit ausgeschlossen. Pläne bitte im CAD-Programm als
DXF exportieren. Noch offen: Schraffuren (`HATCH`, vom Crate `dxf` nicht
unterstützt), Rasterbilder und PDF-Pläne mit Maßstabskalibrierung.

## Mehrbenutzerbetrieb (DMS)

Mehrere Arbeitsplätze arbeiten auf derselben Datenbank (SQLite im Netzlaufwerk
für kleine Büros, PostgreSQL für größere) und demselben Storage. Die Regeln
stehen in [`core-lib`](crates/core-lib/src/benutzer.rs) und gelten so für
Desktop-App und Server gleich.

- **Anmeldung** über das Benutzerkonto des Betriebssystems (`whoami`).
  `BUERO\Daniel`, `daniel@buero.local` und `daniel` sind dasselbe Konto. Wer
  sich als Erster anmeldet, wird Administrator, alle Späteren bekommen die
  Rolle *Lesen*, bis ein Administrator sie hochstuft (*Bearbeiten →
  Benutzerverwaltung …* oder Klick auf den Namen in der Statusleiste).
- **Rollen** (aufsteigend): *Lesen* (ansehen, rechnen), *Bearbeitung*
  (Projektdaten ändern, Dokumente ablegen und sperren), *Projektleitung*
  (Projekte anlegen, Sperren anderer aufheben), *Administrator* (Benutzer
  verwalten). Der letzte aktive Administrator kann sich nicht herabstufen.
- **Versionen**: Jedes Ablegen erzeugt eine neue Version, alte bleiben
  erhalten. In der *Historie* lassen sich ältere Versionen exportieren oder als
  neue Version wiederherstellen.
- **Konflikte** (Optimistic Locking): Wer eine neue Version ablegt, nennt die
  Version, auf der er aufbaut. Hat inzwischen jemand anderes abgelegt, lehnt
  die Datenbank ab, statt zu überschreiben, und die App sagt das verständlich.
- **Sperren**: Ein Dokument lässt sich für die Dauer einer längeren
  Bearbeitung sperren; andere sehen, wer es seit wann hat, und können nichts
  ablegen. Die Projektleitung kann fremde Sperren aufheben.
- **Protokoll** (*Ansicht → Protokoll*, Strg+Umschalt+L): wer wann was im
  Projekt geändert hat.

Der Server hat noch keine eigene Anmeldung und handelt bis dahin als
Benutzer `system` (Administrator-Rechte, zählt aber nicht als Administrator
für die Benutzerverwaltung). Beide Datenbank-Backends prüft dieselbe
Testsuite (`core_lib::testsuite`); für PostgreSQL mit
`BAUPROJEKT_TEST_POSTGRES=postgres://… cargo test -p db-postgres`.

## Desktop-App

* **Betriebsart** über `<App-Konfiguration>/config.json`; ohne Datei startet die
  Einzelplatz-Version (SQLite + lokaler Dateiordner). Netzwerkmodus:
  ```json
  { "modus": "netzwerk",
    "database_url": "postgres://bp:geheim@server/bauprojekt",
    "storage": { "type": "nas", "root": "\\\\nas\\bauprojekt" },
    "cache_max_gb": 20 }
  ```
* **Menüleiste:** native Menüs *Datei* (Neues Projekt, Projekt öffnen, Zuletzt
  geöffnet, Projekt schließen, Dokument importieren, Beenden), *Bearbeiten*,
  *Ansicht* (Dokumente, Pläne, Zoom, Vollbild), *Berechnung* und *Hilfe*; definiert in
  [`menu.rs`](apps/desktop/src-tauri/src/menu.rs). Projektbezogene Einträge sind
  nur bei geöffnetem Projekt aktiv; unter macOS gibt es zusätzlich das App-Menü.
* **Fenster:** native Dekorationen, Mindestgröße, gespeicherte Fensterposition
  (`tauri-plugin-window-state`), nur eine Instanz (`tauri-plugin-single-instance`).
* **Auto-Update:** `tauri-plugin-updater`. Die App prüft beim Start im
  Hintergrund und installiert erst nach Bestätigung. Vor dem ersten Release:
  1. Schlüsselpaar erzeugen: `npm run tauri signer generate -- -w ~/.tauri/bauprojektio.key`
  2. Öffentlichen Schlüssel in `tauri.conf.json` → `plugins.updater.pubkey` eintragen.
  3. Update-Endpunkt in `plugins.updater.endpoints` eintragen (z. B. das
     `latest.json` des GitHub-Releases oder ein eigener Update-Server).
  4. Privaten Schlüssel als Secret `TAURI_SIGNING_PRIVATE_KEY` hinterlegen; der
     Workflow [`release.yml`](.github/workflows/release.yml) baut und signiert.
* **Icons:** Platzhalter in `apps/desktop/src-tauri/icons/`; mit
  `npm run tauri icon <logo.png>` durch das finale Logo ersetzen.

## Lizenzen

Die Software wird kommerziell vertrieben. Erlaubt sind ausschließlich
permissive Lizenzen (MIT, Apache-2.0, BSD, ISC, …); GPL, AGPL und LGPL sind
ausgeschlossen. Geprüft wird automatisch in der CI:

```sh
cargo deny check            # Rust (deny.toml)
cd apps/desktop && npm run licenses   # npm (scripts/check-licenses.mjs)
```

MPL-2.0 ist als schwaches, dateibezogenes Copyleft zugelassen. Es kommt
transitiv über Tauri (`cssparser`, `selectors`, `dtoa-short`, `option-ext`) und
Rhai (`smartstring`) und ist mit proprietärem Vertrieb vereinbar, solange diese
Dateien selbst nicht verändert werden. Mehrfach lizenzierte Crates wie `r-efi`
(MIT/Apache-2.0/LGPL) werden unter MIT/Apache genutzt.
