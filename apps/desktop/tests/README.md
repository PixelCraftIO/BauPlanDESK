# UI-Regressionen für Projekttabs

Die Tests laufen gegen den **Produktionsbuild der echten Svelte-Anwendung**. Ein vor dem Start injizierter Tauri-Testtransport kontrolliert Antworten und simuliert insbesondere fehlgeschlagenes und verspätetes Speichern. Der Testtransport wird nicht in die Anwendung eingebunden.

Voraussetzungen: Repository-Frontendabhängigkeiten, Python mit Playwright und Chromium unter `/usr/bin/chromium`. Keine zusätzlichen Laufzeitabhängigkeiten der Anwendung.

Im Repository:

```sh
npm run check --prefix apps/desktop
npm run build --prefix apps/desktop
python -m http.server 1430 --bind 127.0.0.1 --directory apps/desktop/dist
```

In einer zweiten Shell:

```sh
python apps/desktop/tests/projekttabs.py
```

Die Tests prüfen getrennte Entwürfe, Dirty-Markierungen, Hintergrundtab schließen, verzögertes Speichern, Bauantragsdaten, Beenden, native Menüauslöser, leere Suchergebnisse, Leserrechte und schmale Fenster. Aufnahmen und Prüfergebnis werden unter `docs/ui-umsetzung` geschrieben. Die JSON-Fixture enthält ausschließlich lokale Beispieldaten.

Diese Suite überprüft die UI und ihre IPC-Zuordnung. Backend-Berechnungen und Persistenz werden durch die vorhandenen Rust-Tests überprüft; eine native Desktop-Probe ergänzt die UI-Tests.


## PDF-Vorlagen und Dateiablage

Mit dem Rust-Integrationstest zuerst echte PDFs und DMS-Metadaten erzeugen:

```sh
BAUPROJEKT_PDF_ARTEFAKTE=/tmp/bauprojekt-pdf cargo test -p storage --test formular_workflow --locked
```

Dann den Produktionsbuild wie oben bereitstellen und prüfen:

```sh
BAUPROJEKT_PDF_ARTEFAKTE=/tmp/bauprojekt-pdf BAUPROJEKT_UI_URL=http://127.0.0.1:1430 python apps/desktop/tests/pdf-ablage.py
```

Dieser UI-Test verwendet den echten PDF.js-Worker mit den erzeugten PDFs; nur die
Tauri-Aufrufe für die Bedienaktionen sind kontrolliert. Der Rust-Test prüft den
gesamten PDF-/DMS-/Ablagepfad mit SQLite und echten Dateisystemen. Screenshots und
Protokolle liegen unter `docs/klare-projektstruktur` und dessen Unterordner `pdf`.

`BAUPROJEKT_UI_URL` kann auch für `projekttabs.py` gesetzt werden.
`BAUPROJEKT_UI_ARTEFAKTE` legt dort einen alternativen Ausgabeordner fest, damit
frühere Dokumentationsaufnahmen beim erneuten Testen erhalten bleiben.

## Zentrale Formularbibliothek

Nach Erstellung der PDF-Testartefakte und `npm run build` gegen die lokal
bereitgestellte `dist`-Anwendung:

```bash
python3 apps/desktop/tests/formularbibliothek.py
```

`BAUPROJEKT_UI_URL` und `BAUPROJEKT_PDF_ARTEFAKTE` können den UI-Server bzw.
den Artefaktordner überschreiben. Die UI-Prüfung verwendet Test-IPC und echte
PDF.js-Darstellung. Reale Ordner-, Hash-, Berechtigungs- und projektübergreifende
DMS-Verträge prüft `cargo test -p storage --test formularbibliothek`.

## Dateiverwaltung und Einreichungen

```bash
BAUPROJEKT_EINREICHUNG_ARTEFAKTE=/workspace/review-main/einreichungen \
  cargo test -p storage --test einreichungen
python3 apps/desktop/tests/dateiverwaltung-einreichungen.py
```

Die UI-Prüfung nutzt die kompilierte Anwendung mit Test-IPC. Der UI-Server ist
über `BAUPROJEKT_UI_URL` einstellbar, der Artefaktordner über
`BAUPROJEKT_EINREICHUNG_ARTEFAKTE`. Reale HTTP-Grenzen prüft
`cargo test -p server --test einreichungen_api`. Der PostgreSQL-Repositorytest
benötigt `BAUPROJEKT_TEST_POSTGRES_URL` zu einer dedizierten Testdatenbank; ohne
diese Variable läuft er nicht gegen einen Server.

Für PDF-Regressionsläufe kann `BAUPROJEKT_PDF_UI_ARTEFAKTE` einen separaten
Ausgabeordner wählen, damit vorhandene Dokumentationsbilder erhalten bleiben.

## Hierarchische Projektakte und Formulare im Projekt

```sh
cargo test -p storage --test projekt_formulare --locked
BAUPROJEKT_UI_URL=http://127.0.0.1:1430 python3 apps/desktop/tests/klare-projektstruktur.py
```

Die Suite prüft bedingte Bereiche, getrennte Rechtsgrundlagen/Festsetzungen,
tabellarische Flurstücke, gemeinsame Eingaben der Berechnungsansichten,
Bibliotheksauswahl im Projekt ohne vorzeitige Dokumentanlage, Speicherfehler,
genau einen Dateieintrag, weitere Versionen, fehlenden Exportdialog, Theme und
Speicherorte. PDF-Testartefakte werden wie oben über `formular_workflow` erzeugt.
`pdf-ablage.py` führt diese Suite und `formularbibliothek.py` gemeinsam aus.
Der Desktop-Modultest in `projektordner.rs` prüft die automatische lesbare Ausgabe.
