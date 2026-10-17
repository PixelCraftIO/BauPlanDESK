# Signierte Tauri-Updates einrichten und testen

Unter **Einstellungen → Anwendung und Updates** stehen App-Version,
Updatekonfiguration und „Updates prüfen“. Release-Builds prüfen außerdem beim
Start im Hintergrund. Die Anwendung zeigt eine verfügbare Version und deren
Releasehinweise an; sie installiert erst nach „Update installieren und neu
starten“. Keine Aktualisierung allein durch einen Download oder Programmstart.

## Einmalige Einrichtung vor dem ersten signierten Build

1. Im Verzeichnis `apps/desktop` ein Tauri-Signierschlüsselpaar erzeugen:

   ```sh
   npm ci
   npm run tauri signer generate -- -w /sicherer/pfad/bauprojektio.key
   ```

   Unter Windows einen eigenen sicheren Windows-Pfad verwenden. Privaten
   Schlüssel und Passwort sicher außerhalb des Repositorys aufbewahren. Den
   privaten Schlüssel weder committen noch in ein Supportticket kopieren.
2. In GitHub → Repository → Settings → Secrets and variables → Actions:

   | Typ | Name | Inhalt |
   | --- | --- | --- |
   | Variable | `BAUPROJEKT_UPDATER_PUBKEY` | Vollständiger Inhalt der erzeugten `.pub`-Datei |
   | Secret | `TAURI_SIGNING_PRIVATE_KEY` | Vollständiger Inhalt der privaten Schlüsseldatei |
   | Secret | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Beim Erzeugen gewähltes Passwort; bei bewusst unverschlüsseltem Schlüssel leer |

   Den gleichen öffentlichen Schlüssel in **beide** Testbuilds einbauen und mit
   demselben privaten Schlüssel signieren. Ein später neu erzeugter Schlüssel
   wird von bereits installierten Apps nicht automatisch akzeptiert.
3. Updatequelle wählen. Es gilt diese Reihenfolge, jeweils **beim Build**:

   - `BAUPROJEKT_UPDATE_URL`: explizite HTTPS-Adresse eines Tauri-Manifests.
   - Sonst `BAUPROJEKT_LIZENZSERVER`: `<Adresse>/updates/latest.json`.
   - Sonst öffentliche GitHub-Releases:
     `https://github.com/PixelCraftIO/BauProjektIO/releases/latest/download/latest.json`.

   Diese Variablen werden durch `release.yml` in den Build übernommen.
   Benutzer müssen auf die Quelle und Installer ohne GitHub-Anmeldung zugreifen
   können. **Private Repository-Releases funktionieren über die Standardadresse
   nicht ohne Weiteres.** Dafür den vorhandenen eigenen Update-/Lizenzserver
   verwenden; keinen GitHub-Token in eine ausgelieferte App einbauen.

Ohne öffentlichen Schlüssel läuft die App weiter. Die Updateprüfung meldet
„noch nicht eingerichtet“ und behauptet nicht, es gebe keine neuere Version.
Der Release-Workflow verlangt beide Signierschlüssel. Normale Cargo-/Frontend-
Entwicklung benötigt sie nicht. Es wurde kein produktiver Schlüssel erzeugt
oder im Repository hinterlegt.

## Test mit zwei Releases, beispielsweise 0.1.0 → 0.1.1

1. Änderungen dieses Branches in den zu bauenden Stand übernehmen. Die erste
   App-Version ist aktuell `0.1.0`. Änderungen committen, dann Tag `v0.1.0`
   erstellen/pushen, sofern dieser noch nicht existiert. Alternativ den Workflow
   **Release** manuell für diesen Stand starten.
2. Der Workflow baut für Windows, macOS und Linux und erstellt einen
   **Release-Entwurf** mit Installern, Updateartefakten, Signaturen und
   `latest.json`. Für Windows bevorzugt das Manifest den NSIS-Installer.
   Erst nach erfolgreichem Build aller Plattformen den Entwurf veröffentlichen.
3. Auf dem Testgerät den ersten Installer installieren und ein Testprojekt
   anlegen. Vor dem Installertest eine Sicherung erstellen.
4. Für die zweite Version im Verzeichnis `apps/desktop` ausführen:

   ```sh
   npm run version:app -- 0.1.1
   ```

   Das aktualisiert `tauri.conf.json`, `package.json` und die npm-Lockdatei.
   Die Cargo-Workspace-Version ist die Bibliotheksversion und wird dadurch
   nicht geändert; Anzeige, Installer und Updater verwenden die Tauri-App-Version.
   Änderungen committen und Tag `v0.1.1` pushen. Nochmals alle Release-Jobs
   abwarten und den neuen Entwurf veröffentlichen. Es muss eine **höhere** Version
   mit derselben App-Kennung und passendem Signierschlüssel sein.
5. In der installierten `0.1.0` unter Einstellungen „Updates prüfen“ wählen.
   `0.1.1` und Releasehinweise müssen erscheinen. Ein ungespeicherter Entwurf
   soll die Installation verhindern, auch in einem anderen Projekttab.
6. Eingaben speichern oder verwerfen, offene Dialoge/Aktionen abschließen und
   Aktualisierung starten. Downloadfortschritt, Signaturprüfung und Installation
   abwarten. Nach Neustart Version und gespeichertes Projekt prüfen.

Der Updater prüft die Signatur mit dem eingebauten öffentlichen Schlüssel.
Signatur-/Netzwerkfehler erscheinen als Fehler; derselbe vorgemerkte Stand kann
nach einem fehlgeschlagenen Versuch erneut angefordert werden. Es gibt keine
Umgehung der Signatur- oder HTTPS-Prüfung. Releases vor Veröffentlichung,
Prereleases und heruntergestufte Versionen gehören nicht zum stabilen Kanal.

## Eigener Update-/Lizenzserver

Der vorhandene Workflow `updates.yml` läuft nur mit konfiguriertem
`BAUPROJEKT_LIZENZSERVER`. Er benötigt außerdem `BAUPROJEKT_UPLOAD_TOKEN` als
Secret und einen entsprechend eingerichteten Server. Er lädt Artefakte zuerst
und das umgeschriebene Manifest zuletzt hoch. Für einen externen
`BAUPROJEKT_UPDATE_URL` außerhalb dieses Servers muss dessen Betreiber Manifest
und die darin verlinkten Artefakte selbst veröffentlichen.

Im eigenen Manifest sind mindestens `version`, `platforms` und je Plattform
`url` und die vom Tauri-Build erzeugte `signature` erforderlich; optional `notes`
und `pub_date`. Das automatisch erzeugte `latest.json` verwenden und keine
Signaturen erfinden. Eigene Manifest-/Assetadressen müssen über HTTPS mit einer
vertrauenswürdigen Zertifikatskette erreichbar sein. Die Installation bleibt
von Betriebssystemrechten und dem unterstützten Tauri-Paketformat abhängig.
Windows-/Apple-Codesigning ist getrennt von der Tauri-Updatesignatur und wird
hierdurch nicht eingerichtet.

## Schutz offener Arbeit und Modulgrenzen

Updateprüfung und Installation sind im nativen Adapter serialisiert.
Eine Installation muss sich auf genau die im UI angebotene Version beziehen.
Öffentliche Schlüssel und Quelle sind buildgebunden; beliebige Umgebungsvariablen
am Endgerät ersetzen sie nicht. Keine Fachberechnung oder Projektdatenmigration
wird im Updater nachgebaut; dies ist ausschließlich eine Plattformfunktion.

Die UI prüft alle offenen Projekteditoren, Kundeneingaben und Einstellungsaktionen
vor der Installation. Während Download/Installation sind Hauptarbeitsbereich,
native Menüaktionen und normales Fensterschließen gesperrt. Betriebssystem-
Abstürze oder erzwungenes Beenden sind davon nicht erfasst. Ein Update ersetzt
keine Datensicherung und führt keine automatische Datenrücksetzung durch.

## Tatsächlich ausgeführte Prüfungen

- Rust: Schlüssel-/Quellenkonfiguration und Desktop-Kommandobuild.
- Node: fehlende Signierkonfiguration, Versions-/Tagabweichung, ungültige Quelle.
- Browser: verfügbar/aktuell/nicht eingerichtet, Offlinefehler, Releasehinweise,
  Entwurfserhalt, Downloadfortschritt, Fenstersperre, Fehler und erneuter Versuch.
  Hier wird nur der native Transport simuliert, keine Installer-Ausführung.
- Svelte-Prüfung und Produktionsbuild.

**Noch auf dem Zielgerät auszuführen:** echte signierte Installer bauen,
veröffentlichen und den vollständigen Wechsel zwischen zwei Versionen testen.
Eine erfolgreiche Windows-/macOS-/Linux-Installation wurde durch die
Browserprüfung nicht nachgewiesen.

## ZIP-Sicherheitsaktualisierung vom 17.10.2026

Der CI-Job `lizenzen` führt mit `cargo deny check` auch Sicherheitsprüfungen aus.
Die neu erfassten Meldungen RUSTSEC-2026-0367, -0368 und -0369 betreffen
`zip 4.6.1`, einschließlich der Windows-Abhängigkeit des Tauri-Updaters.
Der Workspace verwendet deshalb mindestens `zip 9.0.4`. Fehler beim Lesen
von ZIP-Dateinamen werden in XBau-Import und Sicherungsprüfung weitergegeben;
die bestehenden Prüfungen auf Pfadmanipulation, doppelte Namen und Integrität
bleiben erhalten. Öffentliche Projekt- und Exportverträge ändern sich nicht.

Auch der veröffentlichte Updater `2.14.0` verwendet noch `zip 4`. Bis eine
bereinigte Version veröffentlicht ist, verwendet der Desktop den offiziellen
[Upstream-Fix](https://github.com/tauri-apps/plugins-workspace/commit/ba98bfadc4c868c9a7e1a226c83af528e1f11b89)
mit exakt dieser Git-Revision. Der Fix ändert die ZIP-Abhängigkeit auf Version 9.
`deny.toml` erlaubt dafür ausdrücklich dieses offizielle Git-Repository;
Sicherheitsmeldungen werden nicht ignoriert. Die Lockdatei enthält kein `zip 4`
mehr. Nach Veröffentlichung einer bereinigten stabilen Updater-Version die
Git-Bindung und deren Quellenfreigabe entfernen und erneut
`cargo deny --all-features check` sowie ZIP-/Sicherungs-/XBau-Tests ausführen.

Ein echter signierter Windows-Updatevorgang muss weiterhin mit zwei Release-Builds
auf dem Zielgerät geprüft werden.
