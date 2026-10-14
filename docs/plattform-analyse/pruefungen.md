# Fundstellen und Prüfprotokoll

Bezugsstand der Anwendung: `31a195796e6515a4ca83bfc26d9c3a046fde08fc`.
Datum: 14. Oktober 2026. Die Analyse ändert die Anwendungslogik nicht.

Die Verweise beziehen sich auf diesen Stand. „Reproduziert“ bedeutet, dass das
Verhalten in einer isolierten Probe tatsächlich beobachtet wurde. „Codebefund“
beschreibt den gelesenen Ablauf oder eine aus dem Modell erkennbare Grenze.
Eine Empfehlung ist eine noch nicht implementierte Zielentscheidung.

## 1. Reproduzierte Befunde

| ID | Befund und Probe | Fundstelle | Maßnahme |
| --- | --- | --- | --- |
| F01 | §35 ohne Plan: direkte GRZ-Prüfung `soll = None`, Grundstücksprüfung `soll = Some(0.4)` | [Kontextaufbau](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/baugrundstuecke.rs#L245), [Unterscheidung §35](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/festsetzungen.rs#L182) | A1 |
| F02 | Ermitteltes Bauland 800 m², alter Projektwert 640 m²: PDF-Kontext schlägt `640,00` vor | [Projektkontext](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/formular/vorlage.rs#L21), [Analyse](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/service.rs#L1221) | A3 |
| F03 | H=10 berechnet, anschließend H=20: sichtbares Ergebnis bleibt 4,00 m ohne Veraltet-Hinweis | [Ergebniszustand](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/apps/desktop/src/lib/Abstandsflaeche.svelte#L32) | A2 |
| F04 | Ein Benutzer ändert Rechtsgrundlagen, speichert zwischenzeitlich Festsetzungen, speichert dann Rechtsgrundlagen: Versionskonflikt | [Lokale Bereichskopien](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/apps/desktop/src/lib/Rechtsgrundlagen.svelte#L29), [Schreibvorgang](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/apps/desktop/src/lib/Rechtsgrundlagen.svelte#L106), [Gesamtes Projektupdate](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/domain.rs#L260) | B1 |
| F05 | Ohne Anmeldung: Projektliste HTTP 200, Projektanlage HTTP 201, protokollierter Akteur System | [Serverkonto](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/server/src/main.rs#L97), [Router](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/server/src/api.rs#L69) | C4 vor externer Öffnung |

### F01 und F02: Probe mit dem kompilierten Core

Eingaben: ein Flurstück mit 800 m², WA, kein Bebauungsplan, §35,
BauNVO 2017, leere Bauteil-/Geschossaufstellung. Anschließend wurde ein
Projektkontext mit demselben Grundstücksplan und abweichender alter Projektfläche
640 m² erzeugt.

Tatsächliche Ausgabe:

```text
Par35 direct GRZ Soll: None
Par35 grouped GRZ Soll: Some(0.4)
PDF context grundstueck.flaeche_m2: Some("640,00")
Calculated parcel Bauland m2: 800
```

[Verwendeter Probequelltext](fach-probe.rs.txt). Der Text wurde außerhalb der
Anwendung mit `rustc --edition=2024`, der zum Analysezeitpunkt gebauten
`libcore_lib-a91882633060d52b.rlib` und dem vorhandenen Dependency-Verzeichnis
kompiliert und ausgeführt. Dieser konkrete Artefaktname ist umgebungsspezifisch;
die Probe ist kein neu eingebundener, grün laufender Regressionstest.

### F03 und F04: Browserprobe

Produktionsbuild der Svelte-Anwendung mit kontrolliertem Tauri-Testtransport und
lokalen Beispieldaten. Die Probe bedient dieselben sichtbaren Komponenten wie die
bestehenden UI-Suiten. Sie führt keine Fachberechnung gegen ein echtes Bauvorhaben
aus; F03 prüft die fehlende Invalidierung eines vorhandenen Ergebnisses.

- [Maschinenlesbares Ergebnis](ui-befunde.json)
- [Veraltetes Abstandsflächenergebnis](01-veraltetes-abstandsergebnis.png)
- [Bereichskonflikt](02-konflikt-im-selben-projekt.png)

Bei F04 gehen die Eingaben nicht verloren. Das ist ein unnötiger Konflikt zwischen
Fachbereichen, kein Nachweis für einen stillen Datenverlust.

### F05: Echte lokale HTTP-Probe

Aktuellen `bauprojekt-server` gebaut und mit eigener SQLite-Testdatenbank,
lokaler Testablage und Bindung ausschließlich an `127.0.0.1` gestartet.
Ohne Cookie oder Authorization-Header Projektliste gelesen und ein Testprojekt
angelegt. Das Protokoll der Testdatenbank bestätigt den Akteur System. Danach
wurde der Testserver beendet.

[Maschinenlesbares Ergebnis](api-befund.json). Keine öffentlichen Server oder
fremden Benutzerkonten wurden geprüft. Die vorhandene lokale Bindung reduziert
die Erreichbarkeit; sie fügt der API keine Authentifizierung hinzu.

## 2. Strukturelle Codebefunde

| ID | Beobachtung | Fundstelle | Bedeutung |
| --- | --- | --- | --- |
| S01 | Recht, Berechnungseingaben, Grundstücksplan und Baugrenzen im selben JSON-Fachobjekt | [Planungsrecht](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/domain.rs#L161), [Validierung](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/domain.rs#L184) | Fachliche Änderungen und Prüfungen sind über gemeinsame Speichergrenze gekoppelt |
| S02 | Fachänderung beziehungsweise Dokumentversion wird vor separatem Audit-Aufruf gespeichert | [Projektupdate](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/service.rs#L406), [Dokumentcommit](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/service.rs#L1006), [Einreichung](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/einreichung/service.rs#L79) | Bei einem Auditfehler kann die Rückmeldung dem tatsächlichen Speicherstand widersprechen; Ausfallinjektion noch offen |
| S03 | Gruppenschlüssel wird aus veränderlichen Grundbuchangaben gebildet | [Schlüssel](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/baugrundstuecke.rs#L36) | Dauerhafte Identität und Split-/Merge-Vorgänge für spätere Erweiterungen nötig |
| S04 | Geschosse haben keine Gebäude-ID; Vollgeschosse werden in einer Liste gezählt | [Geschossmodell](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/flaechen.rs#L52), [Zählung](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/flaechen.rs#L333) | Mehrgebäudefälle brauchen ein explizites Modell und passende Aggregation |
| S05 | LBO-Engine hält ein aktives Paket je Bundesland | [Paketkatalog](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/lbo/mod.rs#L70), [Metadaten](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/lbo/rulepack.rs#L19) | Historische Regelbindung und parallele Rechtsstände fehlen als allgemeiner Vertrag |
| S06 | Drei LBO-Dateien sind als beispielhaft bezeichnet | [BY](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/lbo-rules/by.rhai), [BW](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/lbo-rules/bw.rhai), [NW](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/lbo-rules/nw.rhai) | Die Auswahl aller Bundesländer ist kein Nachweis vollständig geprüfter Länderregeln |
| S07 | BauNVO-Regeln sind gemeinsam eingebettet; Änderung benötigt Backend-/Frontend-Build | [Regelkatalog](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/baunvo.rs#L27), [Dokumentation](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/regelwerke/baunvo/README.md) | Gemeinsame Quelle ist gut; unabhängige Inhaltsupdates benötigen später einen versionierten Katalog |
| S08 | Netzwerkdesktop verbindet direkt mit PostgreSQL und Remote-Storage; Anmeldung über OS-Namen | [Bootstrap](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/apps/desktop/src-tauri/src/bootstrap.rs#L71), [Anmeldung](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/apps/desktop/src-tauri/src/bootstrap.rs#L147), [Normalisierung](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/benutzer.rs#L189) | API-basierter Bürobetrieb und verifizierte Identitäten als eigener Entwicklungsschritt |
| S09 | Frontend-Client ruft Tauri direkt auf; Server bietet nur einen Teil der Desktop-Operationen | [Client](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/apps/desktop/src/lib/api.ts), [HTTP-Routen](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/server/src/api.rs#L69) | Gemeinsamer Kern allein ergibt noch keinen austauschbaren Webclient |
| S10 | Editorlisten und Bereichsbedingungen sind an mehreren zentralen Stellen gepflegt | [Projektakte](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/apps/desktop/src/lib/Projektakte.svelte#L90), [Sidebar](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/apps/desktop/src/lib/Sidebar.svelte), [App](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/apps/desktop/src/App.svelte#L69), [Bereiche](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/apps/desktop/src/lib/projektSitzung.ts) | Gemeinsame Modul-/Editorverträge reduzieren Einbaufehler |
| S11 | Vollständige Projektlisten und wiederholte PDF-Bibliotheksanalyse | [Repositoryliste](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/db-sqlite/src/lib.rs#L173), [UI-Filter](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/apps/desktop/src/lib/ProjektVerzeichnis.svelte#L25), [Bibliothek](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/storage/src/formularbibliothek.rs#L62) | Kandidaten für Suche/Pagination und Metadatencache; Lastgrenzen nicht gemessen |
| S12 | Einreichungsstand bindet Dokumentversionen, Hash und Empfänger | [Einreichungsmodell](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/einreichung.rs), [Service](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/einreichung/service.rs) | Tragfähige Basis für Revisionen, Unterlagenprüfung und empfängerbezogene Nachträge |
| S13 | Projektordnerausgabe ist von DMS getrennt und Fehler nach Commit werden als Hinweis zurückgegeben | [Ablageadapter](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/apps/desktop/src-tauri/src/projektordner.rs), [Exportvertrag](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/core-lib/src/projektablage.rs) | Diesen Ansatz zu dauerhaften Aufträgen ausbauen, keine zweite schreibende Datenquelle einführen |
| S14 | CI setzt andere PostgreSQL-Variable als der Einreichungsintegrationstest | [CI](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/.github/workflows/ci.yml#L26), [Test](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/crates/db-postgres/tests/einreichungen.rs#L12) | Dieser Test kehrt im bestehenden CI-Setup vor der DB-Prüfung zurück |
| S15 | CI führt Check/Build, aber keine Python-/Playwright-Suite aus | [Frontend-CI](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/.github/workflows/ci.yml#L59), [UI-Suiten](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/apps/desktop/tests/README.md) | UI-Regressionen sind vorhanden, aber nicht automatisch verpflichtend |
| S16 | CAD liest CSS-Farben, lauscht für Themewechsel aber nur auf OS-Media-Query | [Zeichnung](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/apps/desktop/src/lib/PlanViewer.svelte#L101), [Listener](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/apps/desktop/src/lib/PlanViewer.svelte#L352), [Themeauswahl](https://github.com/PixelCraftIO/BauProjektIO/blob/31a195796e6515a4ca83bfc26d9c3a046fde08fc/apps/desktop/src/lib/darstellung.ts) | Explizite Auswahl sollte Canvas-Neuzeichnung ebenfalls auslösen; nicht separat als Laufzeitfehler reproduziert |

Die Haupt-README enthält außerdem noch ältere PDF-Menü-/Exportbeschreibungen
neben dem Verweis auf den aktuellen Ablauf. Eine spätere Dokumentationsbereinigung
sollte aktuelle Bedienung, technische Verträge und historische Entwürfe klar
auseinanderhalten.

## 3. Tatsächlich ausgeführte Validierung

Die bereits eingerichtete Umgebung wurde verwendet. Rust-/Node-Aktivierung über
`/workspace/.bauprojekt-setup/env.sh`; Paket-/Lockdateien und Anwendungsquelltexte
wurden für die Analyse nicht geändert.

| Prüfung | Ergebnis | Umfang |
| --- | --- | --- |
| `cargo test --workspace --locked --offline` | 168 gemeldete Tests bestanden, 0 fehlgeschlagen | Erster Lauf ohne PostgreSQL-Variablen; bedingte DB-Tests dabei nicht als tatsächliche DB-Prüfung gewertet |
| Derselbe Workspace-Lauf mit beiden PostgreSQL-Testvariablen | 168 bestanden, 0 fehlgeschlagen | Eigene Testdatenbank im vorhandenen lokalen PostgreSQL; Repository-Konformität, Paketpersistenz/atomare Bestätigung und Lizenzserver-Tests tatsächlich ausgeführt |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | Bestanden | Gesamter Workspace einschließlich Desktop |
| `npm run check --prefix apps/desktop` | 0 Fehler, 0 Warnungen | Svelte-/TypeScript-Prüfung |
| `npm run licenses --prefix apps/desktop` | 54 Pakete, 0 Verstöße | npm-Lizenzrichtlinie; kein Ersatz für Schwachstellenprüfung |
| `cargo build -p server --locked --offline` | Bestanden | Für die echte HTTP-Probe |
| `projekttabs.py` | 9 Prüfgruppen bestanden | Projektentwürfe, verzögertes/fehlgeschlagenes Speichern, Schließen, Menüadapter, Leserrolle, Fenster/Themes |
| `formularbibliothek.py` | 6 Prüfgruppen bestanden | Projektunabhängiger Vorlageneditor, echte PDF.js-Darstellung, Fehler/Abbruch |
| `klare-projektstruktur.py` | 5 Prüfgruppen bestanden | Bereichsfreigabe, Grundstückstabelle, Formularfluss, Versionen, Einstellungen |
| `dateiverwaltung-einreichungen.py` | 15 Prüfgruppen bestanden | Dateien, Arbeitskopien, Paketstände und Einreichungen |
| Zusätzliche Fach-, UI- und HTTP-Proben | F01–F05 reproduziert | Absichtlich ergänzte Prüfkombinationen außerhalb der bestehenden grünen Tests |

Die UI-Suiten liefen gegen den bereits für denselben Anwendungsstand gebauten
Produktionsbundle mit echtem PDF.js und Test-IPC. Ausgabeordner wurden nur für
diesen Auditlauf in ein separates Prüfverzeichnis umgeleitet. Testbehauptungen
oder Anwendungscode wurden dabei nicht geändert. Ein zunächst nicht mehr
antwortender lokaler HTTP-Dateiserver wurde durch einen neuen lokalen Testserver
ersetzt; danach bestanden alle vier Suiten.

Der PostgreSQL-Lauf verwendete sowohl `BAUPROJEKT_TEST_POSTGRES` als auch
`BAUPROJEKT_TEST_POSTGRES_URL`. Ein grüner Rust-Testzähler allein unterscheidet
hier nicht, ob ein bedingter Test seinen Prüfteil ausgeführt oder frühzeitig
zurückgekehrt ist. Die eigenständige Testdatenbank wurde ausschließlich für
diesen Lauf angelegt.

Der S3-Test verwendet `object_store::memory::InMemory`, keinen echten S3-Dienst.
XBau-Schematests verwenden das vorhandene `xmllint`; dessen Verfügbarkeit wurde
über die aktivierte Entwicklungsumgebung bereitgestellt. Keine Übermittlung an
ein reales Bauportal erfolgte.

## 4. Grenzen und noch nötige Nachweise

- Keine unabhängige juristische Vollprüfung der historischen BauNVO- und
  Länderregeln. Die im Repository genannten fachlichen Grenzen gelten weiter.
- Keine repräsentative Leistungs-/Speichermessung mit großen Bürobeständen.
- Keine Windows-/macOS-Installation oder Office-/AutoCAD-Anwendung in dieser
  Analyse gestartet. Die vorhandenen nativen Linux-Aufnahmen des Formularablaufs
  stammen aus der vorhergehenden Umsetzung desselben Anwendungsstands.
- Keine vollständige Barrierefreiheitsprüfung mit Screenreader und Nutzertests.
- Kein öffentlicher Cloudbetrieb, realer S3-Dienst, produktiver Mehrmandantenbetrieb
  oder vollständiger Angriffs-/Penetrationstest.
- Keine nachgewiesene Wiederherstellung auf frischer Installation und kein
  plattformübergreifender Update-/Rollback-Test.
- Keine Audit-Ausfallinjektion für S02. Die beschriebene Fehlerfolge ist eine
  Bewertung der tatsächlichen Reihenfolge von getrennten Repository-Aufrufen.

Bestehende grüne Tests und diese Befunde widersprechen sich nicht: Die Tests
sichern ihre abgedeckten Fälle. Die Analyse hat zusätzliche Kombinationen und
zukünftige Produktanforderungen untersucht.
