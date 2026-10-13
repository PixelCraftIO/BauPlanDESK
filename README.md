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
crates/lizenz-server       Lizenzserver des Herstellers (Abo-Tickets, Stripe, Updates)
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

## Lizenzverwaltung

Lizenzen sind signierte Dateien (`.bplizenz`, Ed25519). Die App enthält nur
den öffentlichen Schlüssel des Herstellers und kann Lizenzen prüfen, aber
nicht erzeugen. Ein Administrator spielt die Datei unter *Hilfe → Lizenz …*
ein. Sie liegt danach in der Datenbank und gilt für alle Arbeitsplätze, die
diese Datenbank nutzen.

- **Testversion:** Ohne Lizenz läuft BauProjektIO 30 Tage mit bis zu 3
  Arbeitsplätzen.
- **Arbeitsplätze:** Jeder Rechner belegt beim Start einen Platz und meldet
  sich alle 5 Minuten. Beim Beenden wird der Platz frei, nach einem Absturz
  spätestens nach 15 Minuten. Sind alle Plätze belegt, ist dieser Rechner nur
  lesend nutzbar, bis einer frei wird.
- **Ablauf:** Ist die Testversion oder eine befristete Lizenz abgelaufen,
  bleiben alle Projekte und Dokumente lesbar und exportierbar. Nur Änderungen
  sind gesperrt. Benutzer verwalten und Lizenz einspielen geht immer.
- Geprüft wird in `core-lib` (`CoreServices::pruefe`), also in Desktop und
  Server gleich. Der Server zählt selbst keine Arbeitsplätze.

**Lizenzen ausstellen** (nur beim Hersteller, nie beim Kunden):

```sh
cargo run -p lizenz-werkzeug -- schluessel hersteller.key   # einmalig, sicher aufbewahren!
cargo run -p lizenz-werkzeug -- ausstellen hersteller.key kunde.bplizenz \
    --lizenznehmer "Architekturbüro Muster" --edition buero --arbeitsplaetze 5 \
    --wartung-bis 2027-12-31          # optional: --gueltig-bis für Mietlizenzen
cargo run -p lizenz-werkzeug -- pruefen kunde.bplizenz <öffentlicher Schlüssel>
```

Der öffentliche Schlüssel kommt beim Release-Build über die
Umgebungsvariable `BAUPROJEKT_LIZENZ_SCHLUESSEL` in die App (in GitHub als
Repository-Variable gleichen Namens; mehrere Schlüssel durch Komma getrennt).
Debug-Builds vertrauen zusätzlich einem festen Testschlüssel, mit dem sich
Lizenzen für die Entwicklung erzeugen lassen:

```sh
cargo run -p core-lib --features testdaten --example testlizenz -- test.bplizenz 3
```

### Abo-Lizenzen über den Lizenzserver

Statt einer Datei kann der Administrator einen Lizenzschlüssel eingeben
(*Hilfe → Lizenz …*, `BPIO-XXXXX-…`). Er liegt danach in der gemeinsamen
Datenbank, und jeder Rechner holt sich beim [Lizenzserver](crates/lizenz-server)
selbst ein **Ticket**:

- **Offline prüfbar:** Das Ticket ist mit dem Ticket-Schlüssel des Servers
  signiert (Ed25519) und enthält Lizenzdaten und Ablaufzeitpunkt. Die App
  prüft es lokal gegen den eingebauten öffentlichen Schlüssel
  (`BAUPROJEKT_TICKET_SCHLUESSEL`), ohne Internet.
- **Rollierend:** Ein Ticket gilt 30 Tage. Die App erneuert es im
  Hintergrund einmal am Tag, sobald der Server erreichbar ist. Ist ein
  Rechner länger offline, bleibt alles lesbar und exportierbar, nur
  Änderungen sind gesperrt. Lehnt der Server ab (Abo beendet, gesperrt),
  verfällt das Ticket sofort.
- **An den Rechner gebunden:** Das Ticket enthält die Machine-ID (SHA-256 der
  Rechnerkennung des Betriebssystems: Windows `MachineGuid`, macOS
  Hardware-UUID, Linux `/etc/machine-id`). Auf einen anderen Rechner kopiert,
  gilt es nicht. Der Server zählt die Rechner je Lizenz; wer sich länger als
  die Ticketdauer nicht meldet, gibt seinen Platz automatisch frei.
  *Diesen Rechner abmelden* gibt ihn sofort frei (Rechnerwechsel).
- **Stripe:** Der Server bekommt Zahlungen und Abo-Änderungen per Webhook
  und schaltet die Lizenz aktiv oder inaktiv. Plätze = Menge im Abo.

Der Ticket-Schlüssel ist ein eigenes Schlüsselpaar (`lizenz-werkzeug
schluessel ticket.key`). Der Server kann damit nur kurzlebige,
rechnergebundene Tickets ausstellen, keine Lizenzdateien. Betrieb und
Konfiguration: [`crates/lizenz-server`](crates/lizenz-server/src/main.rs),
Container: [`Dockerfile`](crates/lizenz-server/Dockerfile).

Der Lizenzserver liefert unter `/updates/` auch `latest.json` und die
Installer für den Auto-Updater aus. Der Workflow
[`updates.yml`](.github/workflows/updates.yml) lädt sie nach dem
Veröffentlichen eines Releases hoch.

## XBau-Export

Aus einem Projekt entsteht die Nachricht *Beantragung einer Baugenehmigung*
(`baugenehmigung.antrag.0200`) nach **XBau-Hochbau 2.7** (Kernmodul 1.4.0),
zusammen mit den ausgewählten Bauvorlagen als ZIP-Paket. Bedienung: Ansicht
*Bauantrag* (Strg+Umschalt+B), Export über *Datei → Bauantrag als XBau
exportieren …* (Strg+Umschalt+E).

- **Büroangaben** (*Bearbeiten → Büroangaben …*, nur Projektleitung): Name,
  Anschrift und Absenderkennung des Büros. Das Büro ist Absender (`autor`)
  und Entwurfsverfasser jeder Nachricht.
- **Bauantrag je Projekt:** Bauaufsichtsbehörde mit DVDV-Kennung (`leser`),
  Bauherren (sie sind die Antragsteller), Gebührenpflichtiger, Art der
  Maßnahme, Grundstück mit Flurstücken und die Anlagen aus den Dokumenten.
  Was für den Export fehlt, zeigt die Ansicht vorher an.
- **Codelisten:** Typ-3/4-Codelisten werden nicht mitgeliefert, weil sie sich
  ändern. Die Genericode-Dateien vom [XRepository](https://www.xrepository.de)
  gehören in den Ordner [`codelisten/`](codelisten) (wird mit der App
  ausgeliefert) oder in `<App-Daten>/codelisten`; der Server liest
  `BAUPROJEKT_CODELISTEN_DIR`. Nötig sind *Anlagen*
  (`urn:xoev-de:xbau:codeliste:anlagen`) und *MIME-Types*
  (`urn:xoev-de:xbau:codeliste:xbau-mimetypes`).
- **Schemas:** [`crates/core-lib/xbau/2.7`](crates/core-lib/xbau/2.7). Die
  Tests prüfen jede erzeugte Nachricht mit `xmllint` dagegen (in CI Pflicht).
- Code: [`crates/core-lib/src/xbau`](crates/core-lib/src/xbau).

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
  2. Inhalt der `.pub`-Datei als Repository-Variable `BAUPROJEKT_UPDATER_PUBKEY`
     hinterlegen (ersetzt `plugins.updater.pubkey` aus `tauri.conf.json`).
  3. Repository-Variable `BAUPROJEKT_LIZENZSERVER` setzen; die App fragt
     `<Lizenzserver>/updates/latest.json` ab.
  4. Privaten Schlüssel als Secret `TAURI_SIGNING_PRIVATE_KEY` hinterlegen; der
     Workflow [`release.yml`](.github/workflows/release.yml) baut und signiert,
     [`updates.yml`](.github/workflows/updates.yml) lädt nach dem
     Veröffentlichen auf den Server hoch.
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
