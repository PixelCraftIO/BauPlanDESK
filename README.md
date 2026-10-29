# BauPlanIO

Projektmanagement- und Bauantrags-Software für Architekten: Berechnungen nach
BauGB/BauNVO und Landesbauordnungen, amtliche PDF-Formulare, CAD-Viewer,
XBau-Export und DMS. Läuft als native Desktop-App (Einzelplatz), als
NAS-/Büro-Server und optional in der Cloud.

## Bedienhandbuch und Hilfe

Das **[bebilderte Benutzerhandbuch](docs/handbuch/benutzerhandbuch/README.md)**
erklärt alle Funktionen und Standard-Workflows mit Bildschirmfotos aus dem
Beispielprojekt und wird kapitelweise fortgeschrieben. Als eine Datei:
[Benutzerhandbuch.pdf](docs/handbuch/benutzerhandbuch/Benutzerhandbuch.pdf).

Das [Bedienhandbuch für Architekten und Planer](docs/handbuch/Bedienhandbuch.md)
führt durch Projektanlage, Grundlagen, Berechnungen, Dokumente, Formulare,
Einreichung und Verfahrensdokumentation bis zur Genehmigung und Bauausführung.
In der Anwendung: **Hilfe / Handbuch**, **Hilfe → Bedienhandbuch (F1)** oder
**Hilfe zu diesem Bereich** im Aktenkopf. [Offline-Ausgaben und Pflege](docs/handbuch/README.md).

## Aktuelle Bedienoberfläche

[Hierarchische Projektakte, Formulare im Projekt und Speicherorte](docs/klare-projektstruktur/README.md)
beschreibt den aktuellen Ablauf mit Screenshots und Prüfprotokollen.
[Dark Mode, Formularabstände und weitere Prüfschwerpunkte](docs/ui-qualitaet/README.md)
mit sichtbaren Vorher-/Nachher-Beispielen.

[Zentrale Projektübersicht: Arbeitsschritte, Fristen und Behördenstand](docs/projektuebersicht/README.md)
führt direkt zu den betroffenen Aufgaben, Nachweisen und Verfahrensregistern.

## Plattformstrategie

[Analyse der Anwendung, Zielarchitektur und priorisierter Maßnahmenplan](docs/plattform-analyse/README.md)
bewertet Fachabläufe, Oberfläche, Modulverträge und den Ausbau zur Büroplattform
mit konkreten Fundstellen und Prüfprotokollen.

## Entwicklungsregeln

[Verbindliche Regeln für eigenständige Fachmodule und stabile Schnittstellen](docs/entwicklungsregeln.md)
legen fest, wie Berechnungen, Baugrundstücke, PDF-Formulare und weitere Features
unabhängig erweitert werden. [AGENTS.md](AGENTS.md) verankert diese Regeln für
Programmierarbeiten im Repository.

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
web                        Webseite bauplanio.de: Landingpage + Web-Handbuch (Next.js, Better Auth, siehe web/README.md)
lbo-rules                  LBO-Regelpakete (Rhai); ausgeliefert für BW, BY und NW
```

## Projektdateien und Einreichungen

[Verfahrensakte, Nachforderungen, genehmigte Stände und Bürosicherung](docs/verfahrensakte/README.md)

[Dateiverwaltung und Einreichungschronik](docs/dateiverwaltung-einreichungen/README.md)
ermöglichen das Öffnen von Projektdateien im zugeordneten Programm, die Übernahme
externer Änderungen als neue Version und ZIP-Pakete mit festgehaltenen
Unterlagenständen, Empfängervergleich und ausdrücklichem Einreichungsdatum.

## PDF-Vorlagen und Projektablage

[Bedienung, unterstützte Formulare und Ablagekonfiguration](docs/pdf-ablage/README.md)
und [Modulverträge](docs/pdf-ablage/architektur.md) beschreiben eigene PDF-Felder,
die automatische Eingabemaske, den PDF-Export und auswählbare Dateispeicher.
Die [zentrale Formularbibliothek](docs/formularbibliothek/README.md) verwaltet
PDF-Vorlagen projektunabhängig in einem eigenen lokalen oder freigegebenen Ordner.

## Fachberechnungen

[Acht modulare Nachweise: Stellplätze, Wohnflächen, DIN 277, Vollgeschosse, Höhen,
überbaubare Fläche, Freiflächen und Regenwasser](docs/fachberechnungen/README.md)
mit versionierten Regeln, typisierten Quellübernahmen und tabellarischer PDF-Ausgabe.

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

## Rechtsgrundlagen je Projekt

Der Projektbereich **Rechtsgrundlagen** erfasst die planungsrechtliche Einordnung
nach § 30, § 34 oder § 35 BauGB. Zum Bebauungsplan lassen sich die Art
(qualifiziert, einfach oder vorhabenbezogen), Bezeichnung, Nummer/Fassung,
Rechtskraft und Festsetzungen hinterlegen. Bei einfachen Bebauungsplänen kann
die ergänzende Beurteilung nach § 34 oder § 35 angegeben werden; im Außenbereich
ist die Unterscheidung zwischen privilegierten und sonstigen Vorhaben möglich.

Gemeinde, Grundstück/Standort, Nutzung, Vorhabenbeschreibung und offene Prüfpunkte
werden ebenfalls je Projekt gespeichert. Die Projektübersicht zeigt die gewählte
Rechtsgrundlage und den Bebauungsplan. Bestehende Projekte bleiben erhalten;
fehlende Angaben werden als noch nicht festgelegt angezeigt. Die Einordnung und Freitexte bleiben Projektdokumentation. Strukturierte
Festsetzungen für Baugebiet, GRZ, GRZ einschließlich Nebenanlagen, GFZ, BMZ,
Grundfläche (GR), Geschossfläche (GF), Baufensterfläche und Vollgeschosse werden
in **Maß der Nutzung** automatisch aus dem gespeicherten Projekt gelesen.
Vollgeschosse können als Höchstzahl oder zwingende Zahl angegeben werden.
Die Ergebnisse zeigen Ist, Soll, Quelle und absolute/prozentuale Überschreitung.
Fehlende Planfestsetzungen bleiben ungeprüft; ohne Bebauungsplan werden ausdrücklich
gekennzeichnete Orientierungswerte nach § 17 BauNVO ohne Urteil verwendet. Bei einem
Bebauungsplan ohne erfasste BauNVO-Fassung gibt es keine GRZ-II-Bewertung und kein
Urteil zu den Flächenmaßen, sondern den Hinweis, die Fassung zu erfassen; die Fassung
der Flächenaufstellung ist dort keine Grundlage. Die Baufensterfläche ist nur eine
Information; die Lage im Baufenster (§ 23 BauNVO) bewertet der Core aus den
zugeordneten Umrissen, ohne Umrisse heißt sie „nicht geprüft“. Beim einfachen
Bebauungsplan mit ergänzendem § 34 BauGB werden nicht festgesetzte Maße den
erfassten Umgebungswerten gegenübergestellt (ohne Urteil). Planart und
Rechtsgrundlage prüft der Core beim Speichern (z. B. kein § 34 bei einem
qualifizierten Plan); ältere widersprüchliche Stände bleiben ladbar und werden in
der Projektübersicht gemeldet. Berechnungseingaben
können je Projekt gespeichert werden; nach Änderungen werden alte Ergebnisse
nicht mehr als aktuell dargestellt.

Unter **Pläne** lassen sich geschlossene Umrisse auswählen oder zeichnen und als
Baufenster bzw. Vorhaben zuordnen. **Baugrenze prüfen** vergleicht die Polygone,
zeigt die Fläche außerhalb des Baufensters in m² und Prozent und markiert sie rot.
Zusätzlich wird der größte Überstand an den Gebäudeeckpunkten in Metern gezeigt;
dieser ist kein Maximum aller Kantenpunkte. Die Prüfung unterstützt auch konkave
Umrisse. Ein reiner Zahlenvergleich von Grundfläche und Baufensterfläche prüft
die Lage des Gebäudes nicht. Die Zuordnung wird zusammen mit Plan-Dokument,
Version und Zeichnungseinheit gespeichert und liefert im GRZ-Nachweis die Zeile
„Lage im Baufenster“; die gemessene Fläche wird nicht mehr in die Festsetzungen
geschrieben (bereits gespeicherte Werte bleiben lesbar). Eine neue Planversion erfordert eine neue
Zuordnung. Pro Prüfung werden ein Baufenster und ein Gebäudeumriss verwendet;
Ausnahmen, Befreiungen und besondere Bauteilregelungen werden fachlich beurteilt.

Der Server stellt die projektbezogene Nutzung unter
`POST /api/projekte/{id}/nutzung/pruefen` (NutzungsEingabe mit `mass`, `gebiet`
und optional `vollgeschosse`) und den geometrischen Vergleich unter
`POST /api/berechnung/baugrenzen` (BaugrenzenGeometrie) bereit. Die frühere
Schnittstelle `POST /api/berechnung/mass-der-nutzung` (und der Desktop-Befehl
`mass_der_nutzung`) ist entfallen: Sie urteilte nach § 17 BauNVO und fest nach der
Fassung 2017. Ersatz ist die projektbezogene Prüfung oben.

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

> Abstandsflächenregeln gibt es derzeit nur für **Baden-Württemberg, Bayern und
> Nordrhein-Westfalen**. Sie sind ein **ungeprüfter Pilotkatalog** und müssen vor
> produktivem Einsatz fachlich und rechtlich geprüft werden. In den übrigen
> 13 Ländern meldet die Berechnung „Regelpaket fehlt“. Bundesweit gelten die
> BauNVO-Berechnungen, die Geltungsdauer der Baugenehmigung und die
> Rechtsbehelfsfristen.

Maßgebend für die Tiefenregeln ist der Katalog
[`regelwerke/abstandsflaechen/pilot.json`](regelwerke/abstandsflaechen/pilot.json),
den der wandweise Abstandsnachweis verwendet. Die ausgelieferten Rhai-Pakete des
Schnellrechners müssen für jedes Baugebiet dieselben Faktoren und Mindesttiefen
liefern; der Test `shipped_lbo_pakete_entsprechen_dem_katalog` erzwingt das.
Eine Regeländerung wird deshalb immer in beiden Dateien vorgenommen.

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

- **Anmeldung im Netzwerkbetrieb** mit Anmeldename und Kennwort im
  Anmeldedialog beim Start; jeder Benutzer kann an jedem Arbeitsplatz arbeiten
  (*Anwendung → Abmelden / Benutzer wechseln …*). Kennwörter liegen nur als
  Argon2id-Hash in der Datenbank, nach wiederholten Fehlversuchen wird die
  Anmeldung vorübergehend gesperrt. Administratoren vergeben Erstkennwörter, die
  bei der ersten Anmeldung zu ändern sind. Für Büros aus älteren Versionen: Solange
  kein Administrator ein Kennwort hat, meldet sich ein Administrator einmalig über
  das Benutzerkonto des Rechners an und setzt sofort sein Kennwort. Details,
  Übergang und Grenzen: [Benutzeranmeldung](docs/benutzeranmeldung.md).
- **Anmeldung im Einzelplatzbetrieb** wie bisher über das Benutzerkonto des
  Betriebssystems (`whoami`), ohne Kennwort. `BUERO\Daniel`,
  `daniel@buero.local` und `daniel` sind dasselbe Konto. Wer sich als Erster
  anmeldet, wird Administrator, alle Späteren bekommen die Rolle *Lesen*, bis ein
  Administrator sie hochstuft (Benutzerverwaltung oder Klick auf den Namen in der
  Statusleiste).
- **Grenze:** Die Arbeitsplätze verbinden sich direkt mit der Datenbank. Rechte
  und Kennwörter gelten gegenüber der Anwendung, nicht gegenüber jemandem, der die
  Datenbank-Zugangsdaten aus der `config.json` direkt verwendet.
- **Rollen** (aufsteigend): *Lesen* (ansehen, rechnen), *Bearbeitung*
  (Projektdaten ändern, Dokumente ablegen und sperren), *Projektleitung*
  (Projekte anlegen und löschen, Sperren anderer aufheben), *Administrator*
  (Benutzer verwalten). Der letzte aktive Administrator kann sich nicht herabstufen.
- **Projekte löschen** (*Projektverzeichnis → Löschen*, Desktop-Befehl
  `projekt_loeschen`, Server `DELETE /api/projekte/{id}?version=…`): verlangt die
  Rolle *Projektleitung* und das Projektrecht *Leitung*. Entfernt die Projektakte
  mit Dokumenten und Versionen, Fachständen, Verfahrensdaten und Einreichungen
  sowie alle Ablagedateien unter `projekte/{id}/`. Solange andere Benutzer
  Dokumente des Projekts gesperrt haben, wird abgelehnt; eine veraltete
  Projektversion führt wie beim Ändern zum Konflikt. Der Löschvermerk bleibt im
  Protokoll; lesbare Projektordner auf dem Datenträger bleiben unverändert.
  Nicht entfernbare Ablagedateien werden gemeldet.
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

Der Projektserver benötigt eine Zugangsliste (`BAUPROJEKT_ZUGANGSDATEI`) und prüft Benutzer sowie Projektmitgliedschaft bei jeder Anfrage. Einrichtung und Widerruf: [Plattformumsetzung](docs/plattform-umsetzung/README.md#serverzugänge). Der Netzwerkdesktop verwendet derzeit weiterhin seine bisherigen Datenbankadapter.

## Lizenzverwaltung

Lizenzen sind signierte Dateien (`.bplizenz`, Ed25519). Die App enthält nur
den öffentlichen Schlüssel des Herstellers und kann Lizenzen prüfen, aber
nicht erzeugen. Ein Administrator spielt die Datei unter *Hilfe → Lizenz …*
ein. Sie liegt danach in der Datenbank und gilt für alle Arbeitsplätze, die
diese Datenbank nutzen.

- **Testversion:** Ohne Lizenz läuft BauPlanIO 30 Tage mit bis zu 3
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

## XBau-Austausch

Der Bereich **XBau-Nachrichten** prüft und archiviert Nachrichten und Anlagen,
übernimmt sie in die Verfahrensakte und dokumentiert Übermittlungsbelege.
Bauanträge 0200 und Nachreichungen 0202 können vorbereitet werden.
Ein direkter Behördenversand ist noch nicht angebunden.
Umfang, Installation des erforderlichen XSD-Prüfers und Grenzen:
[XBau-Nachrichten und Behördenanschluss](docs/xbau/README.md).

### Bauantrag und XBau-Abgabe

Die Ansicht *Bauantrag* (Strg+Umschalt+B) erfasst die Antragsangaben. Die
XBau-Nachricht nach **XBau-Hochbau 2.7** (Kernmodul 1.4.0), etwa *Beantragung
einer Baugenehmigung* (`baugenehmigung.antrag.0200`), entsteht mit den
ausgewählten Unterlagen als ZIP-Paket im Abgabestand der Verfahrensakte. Behörde
und Aktenzeichen stammen aus der Akte.

- **Büroangaben** (*Bearbeiten → Büroangaben …*, nur Projektleitung): Name,
  Anschrift und Absenderkennung des Büros. Das Büro ist Absender (`autor`)
  jeder Nachricht; Entwurfsverfasser ist eine bauvorlageberechtigte Person des Büros
  mit Kammer-Mitgliedsnummer oder Nachweis.
- **Bauantrag je Projekt:** Bauaufsichtsbehörde mit DVDV-Kennung (`leser`),
  die Bauherren des Projekts aus der Kundendatenbank (sie sind die
  Antragsteller), Gebührenpflichtiger, Art der
  Maßnahme, Grundstück mit Flurstücken und die Anlagen aus den Dokumenten.
  Was für die Abgabe fehlt, zeigt die Ansicht vorher an.
- **Codelisten:** Die mitgelieferten Typ-3/4-Listen stehen unter
  [`codelisten/`](codelisten). Deren Version muss zum Profil der empfangenden
  Behörde passen; die Anlagenliste ist derzeit als Entwurf gekennzeichnet.
  Bestätigte Genericode-Dateien vom [XRepository](https://www.xrepository.de)
  lassen sich unter `<App-Daten>/codelisten` ergänzen; der Server liest
  `BAUPROJEKT_CODELISTEN_DIR`. Nötig sind *Anlagen*
  (`urn:xoev-de:xbau:codeliste:anlagen`) und *MIME-Types*
  (`urn:xoev-de:xbau:codeliste:xbau-mimetypes`).
- **Schemas:** [`crates/core-lib/xbau/2.7`](crates/core-lib/xbau/2.7). Die
  Tests und Laufzeit prüfen erzeugte Nachrichten mit `xmllint` dagegen.
  Installation siehe [XBau](docs/xbau/README.md).
- Code: [`crates/core-lib/src/xbau`](crates/core-lib/src/xbau).

## Kundendatenbank

Bauherren, Bauträger, Investoren, Baufirmen usw. werden einmal unter
*Bearbeiten → Kunden …* (Strg+Umschalt+K) erfasst: Person oder Firma,
Kategorie, Kundennummer, Ansprechpartner, Anschrift, Telefon, E-Mail und
Notizen. Ein Projekt bekommt einen oder mehrere Kunden als Bauherren
(*Übersicht → Bauherren …*, beim Anlegen eines Projekts oder in der Ansicht
*Bauantrag*). Bauantrag, XBau-Abgabe und PDF-Formulare (`bauherr.*`, siehe
[`formulare/`](formulare)) lesen die Angaben von dort; eine neue Anschrift
gilt also sofort in allen Projekten.

- Weitere Beteiligte (Grundstückseigentümer, Erbbauberechtigte,
  Bevollmächtigte, Fachplaner) kommen ebenfalls aus der Kundendatenbank
  (*Projektakte → Weitere Beteiligte …* oder in der Ansicht *Bauantrag*). Ist
  ein Eigentümer oder Erbbauberechtigter nicht zugleich Bauherr, weist der
  Bauantrag auf die nötige Zustimmungserklärung hin (sperrt den Export nicht).
  Jede Speicherung einer Verfahrensakte und jeder Abgabestand legen sie als
  Anforderung an; entfällt sie später, nennt der Abgabestand die Anforderung
  zur Prüfung. Der Abgabestand friert die Beteiligten ein.
- Eigentümer am Flurstück werden als Kunden zugeordnet (*Grundlagen →
  Flurstücke*). Vorhabenflurstücke gehören einem Bauherrn oder einem
  Beteiligten mit der Rolle Grundstückseigentümer bzw. Erbbauberechtigter;
  Nachbarflurstücke beliebigen Kunden. Solange ein Kunde Eigentümer eines
  Vorhabenflurstücks ist, bleibt er Bauherr oder verfügungsberechtigter
  Beteiligter. Frühere Freitextangaben bleiben lesbar und lassen sich nur
  entfernen.
- Anlegen und Ändern: ab Rolle *Bearbeitung*; Löschen: *Projektleitung*, und
  nur, solange der Kunde keinem Projekt als Bauherr, Beteiligter oder
  Flurstückseigentümer zugeordnet ist.
- Die Zuordnung erhöht die Projektversion (Optimistic Locking wie bei den
  übrigen Projektdaten). Kundendaten haben eine eigene Version.
- Bauherren, die ältere Versionen direkt im Bauantrag gespeichert haben,
  werden beim Start einmalig als Kunden übernommen und den Projekten
  zugeordnet; gleiche Personen bzw. Firmen unter gleicher Anschrift nur
  einmal.
- Server: `GET/POST /api/kunden`, `GET/PUT/DELETE /api/kunden/{id}`,
  `GET /api/kunden/{id}/projekte`, `GET/PUT /api/projekte/{id}/bauherren`,
  `GET/PUT /api/projekte/{id}/beteiligte`.
- Code: [`crates/core-lib/src/kunden.rs`](crates/core-lib/src/kunden.rs).

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
* **Auto-Update:** Signierte Tauri-Updates mit Prüfung beim Start und in den
  Einstellungen, Releasehinweisen, Downloadfortschritt und Schutz offener
  Eingaben. Updatequelle: explizites HTTPS-Manifest, eigener Lizenzserver oder
  öffentliche GitHub-Releases. Einrichtung der Signierschlüssel und Test mit
  zwei Versionen: [Update-Anleitung](docs/updates/README.md).
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

### Flächenformulare nach BauNVO

Unter „Maß der Nutzung“ kann von direkten Summen auf eine Bauteil- und
Geschossaufstellung gewechselt werden. Die maßgebende BauNVO (1962, 1968, 1977,
1990 oder 2017) ist anhand des Plans und der Überleitungsvorschriften zu bestimmen.
Die Aufstellung wird im Projekt gespeichert und im gemeinsamen Rust-Rechenkern
für Desktop und HTTP ausgewertet. Manuelle Summen und Geschosszahlen werden bei
vorhandener Aufstellung nicht als Berechnungsquelle verwendet.

* 1962/1968/1977: eine GRZ, keine heutige GRZ-II-50-%-Regel. Hauptanlagen und
  Garagen werden grundsätzlich angesetzt; Nebenanlagen nach § 14, offene
  Stellplätze/Zufahrten, rein unterirdische Anlagen und privilegierte
  Balkone/Loggien/Terrassen bleiben im Regelansatz unberücksichtigt.
* 1990/2017: Hauptanlagen einschließlich zugehöriger Terrassen/Balkonprojektionen
  für GRZ I; Garagen, Stellplätze/Zufahrten, §-14-Nebenanlagen und zusätzliche
  unterirdische Umgriffe für GRZ II. GRZ II enthält die GRZ-I-Fläche.
* Vollgeschosse: Geschossfläche nach Außenmaßen. Nichtvollgeschosse:
  Aufenthaltsräume plus Umfassungswände plus zugehörige Erschließung/Treppenräume
  bei den älteren Fassungen; bei 1990/2017 standardmäßig keine Anrechnung,
  vollständige oder anteilige Anrechnung mit dokumentierter Planfestsetzung.

Die Vollgeschosseinstufung erfolgt durch den Anwender nach dem maßgebenden
Landesrecht. Flächenüberschneidungen sind vor der Eingabe abzuziehen. Sonderfälle,
insbesondere § 21a, abweichende Planfestsetzungen und die rechtliche Einordnung
von Terrassen, werden nicht aus einer Bezeichnung erraten: abweichende
Bauteilanrechnungen benötigen eine Begründung und den konkret geprüften Ansatz.
Nichtvollgeschoss-Teilflächen dürfen die Außenmaßfläche nicht überschreiten.
Die Ergebnisübersicht zeigt die Anrechnung je Position. Änderungen blenden alte
Ergebnisse aus. Leere Aufstellungen und ungültige Zahlen werden abgelehnt.

`POST /api/projekte/{id}/nutzung/pruefen` akzeptiert optional `aufstellung` mit
`bau_nvo`, `bauteile`, `geschosse`, `nichtvollgeschoss_faktor` und
`nichtvollgeschoss_festsetzung`. Die Antwort ergänzt `aufstellung` um die
berechneten Flächensummen und Einzelansätze. Alte Eingaben ohne dieses Feld
bleiben lesbar und verwenden die bisherige Summenberechnung.

### Vorkonfigurierte Flächenanrechnung und Gemeinschaftsanlagen

Die gemeinsamen Konfigurationen liegen in [`regelwerke/baunvo`](regelwerke/baunvo/README.md).
Neue Projekte starten mit der strukturierten Flächenaufstellung; bestehende
Summeneingaben bleiben erhalten. Die gewählte BauNVO steuert Bauteilansätze,
benötigte Geschossflächenfelder und GRZ-II-Grenzen. Die normale Eingabe benötigt
keine Auswahl der rechtlichen Anrechnung je Bauteil. Eine besondere
Nichtvollgeschoss-Festsetzung kann in den Rechtsgrundlagen hinterlegt werden.

Die Grundstücksaufstellung trennt eigene Basisfläche, nicht maßgebende Flächen
und durch den Bebauungsplan zugeordnete Anteile an Gemeinschaftsanlagen. Etwa
500 m² eigene Fläche und 25 % einer externen 400-m²-Gemeinschaftsanlage ergeben
600 m² Bezugsfläche für GRZ/GFZ/BMZ. Zuschläge werden nicht in die Basis zurückgeschrieben.
Die Originalfläche des Projekts und die geometrischen Baugrenzen bleiben erhalten.

### Flurstücke und einzelne Baugrundstücke

Unter „Maß der Nutzung“ ist die Grundstücksberechnung aus Flurstücken verfügbar.
Neue Projekte starten in diesem Modus. Jede Katasterposition erhält Gemarkung,
Flur/Nummer, Grundbuchbezirk, Blatt, laufende Nummer im Bestandsverzeichnis und
Eigentumsfläche. Gleiche Grundbuchzuordnung wird zusammengefasst; verschiedene
Blätter, Bezirke oder laufende Nummern werden getrennt. Die laufende Nummer
allein ist über mehrere Grundbuchblätter nicht eindeutig. Eine besondere
rechtliche Zuordnung kann mit Kennung und dokumentierter Grundlage hinterlegt
werden; sie ersetzt ausdrücklich die automatische grundbuchliche Gruppierung.

Abzüge werden je Flurstück mit Art, Fläche und Grundlage erfasst. Die Kategorien
stammen aus `regelwerke/grundstueck-abzuege.json` (private/öffentliche Grünfläche,
Verkehrsfläche, außerhalb Bauland, sonstige nicht maßgebende Fläche).
Planfestsetzungs-Abzüge benötigen einen Bebauungsplan. Sich überschneidende
Abzugsflächen müssen vor der Eingabe bereinigt werden. Die Lage außerhalb eines
Baufensters allein ist kein Grundstücksflächenabzug.

Jedes Baugrundstück bekommt eigene Bauteile, Geschosse, Baumasse und optional
abweichende Festsetzungen. GRZ, GFZ, BMZ, Geschossigkeit und Flächenlimits werden
getrennt nachgewiesen; es gibt keine Verrechnung der Kennzahlen über mehrere
Grundstückseinheiten. Zugeordnete Gemeinschaftsanlagen können die jeweilige
Bezugsfläche ergänzen. Eine gleich benannte Anlage darf über alle Einheiten
nicht mit mehr als 100 % ihrer Fläche zugeordnet werden. Eigene Festsetzungen
ersetzen die Projektfestsetzungen vollständig; unbekannte Werte bleiben ungeprüft.

Grundstücksflächen werden bei jeder Berechnung aus den Original-Flurstücken und
Abzügen abgeleitet. Gespeicherte Summen werden nicht erneut abgezogen. Änderungen
an der Grundbuchzuordnung lassen alte Berechnungen sichtbar als verwaiste
Zuordnungen stehen; sie werden nicht stillschweigend anderen Grundstücken
zugewiesen. Fehlende, verwaiste oder doppelte Berechnungen werden abgelehnt.
Die Sammlung wird unter `planungsrecht.baugrundstuecke` gespeichert. SQLite und
PostgreSQL benötigen dafür keine zusätzliche Spalte. Vorherige Gesamteingaben
bleiben erhalten; bei gespeicherten Einzelgrundstücken ist deren gepoolte
Berechnung gesperrt. Geometrische Baugrenzenprüfung und Abstandsflächen erfolgen
weiterhin in ihren bisherigen Bereichen und werden hier nicht über Grundstücke
zusammengefasst. Die automatische Gruppierung ersetzt keine rechtliche Prüfung
besonderer Grundstücksverhältnisse.

`POST /api/projekte/{id}/baugrundstuecke/pruefen` nimmt die Sammlung aus
`flurstuecke` und `berechnungen` entgegen und liefert die Grundstückseinheiten
mit den jeweiligen Nachweisen und der Projektversion.

Aktueller Implementierungsstand: [Plattformumsetzung](docs/plattform-umsetzung/README.md).

Berechnungen erweitern: [BauNVO-Regeln und Modulgrenzen](docs/baunvo-erweiterung.md).

Die Trennung zwischen Flurstücksfakten, optionalen Grundbuchangaben und der Baugrundstücksermittlung ist in [Grundstücksinformationen](docs/grundstuecksinformationen.md) beschrieben.

Projektbezogene Abstandsflächen mit Wandherleitung, Vektorplan und archiviertem PDF-Nachweis: [Bedienung und fachliche Grenzen](docs/abstandsflaechen/README.md).
