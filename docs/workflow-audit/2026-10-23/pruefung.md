# Ausgeführte Prüfung am 23.10.2026

Produktionsbuild des Arbeitsstands `e80717f`, lokaler statischer Server,
Chromium `/usr/bin/chromium`, Python Playwright. Es wurden fünf bestehende
Browser-Suites und ein neues exploratives Prüfskript ausgeführt. Alle sechs
Läufe endeten erfolgreich. Die Suites sind Skripte mit mehreren Assertions;
die Laufzahl ist keine Anzahl einzelner Testfälle.

| Lauf | Wesentliche Aussage | Protokoll |
| --- | --- | --- |
| `arbeitsplatz-ui.py` | Prüfpunktnavigation, Entwurfserhalt, Zuständigkeit, exakte Dokumentversion, nicht verfügbarer Arbeitscheck, Hell/Dunkel und 100/125/150 % CSS-Zoom | [Ausgabe](belege/arbeitsplatz.txt) |
| `fachberechnungen.py` | Acht Fachmasken, Fixture-Prüfberichte, wiederholtes Archivieren mit derselben Kennung, Konflikterhalt; textlicher Inhalt vorhandener echter PDF-Fixtures | [Ausgabe](belege/fachberechnungen.txt) |
| `projekttabs.py` | Zwei Akten, getrennte Entwürfe, verzögertes/fehlgeschlagenes Speichern, Schließen/Abbrechen, aktives Importziel, Leserechte | [Ausgabe](belege/projekttabs.txt) |
| `ui-felder-navigation.py` | Leerwert/0, negative und unvollständige Eingaben, Speicher-/Rechensperre, Entwurfswiederherstellung, Geometriewechsel, Konflikte, Navigation und Hell/Dunkel | [Ausgabe](belege/felder-navigation.txt) |
| `druckartefakt.py` | Binäre Vorschau, gleiche Artefaktkennung für Export und Projektablage, Ressourcenfreigabe und Abbruch | [Ausgabe](belege/druckartefakt.txt) |
| `probe.py` | §34 ohne BauNVO, optionales Grundbuch in der UI, neuer Abstandsplan, leere Formularbibliothek, Aktenvoraussetzung und bereichsübergreifender Entwurfserhalt | [JSON](belege/probe.json) |

## Reproduktion

Vom Repositoryverzeichnis aus, bei bereits gestartetem Produktionsserver:

```sh
export BAUPROJEKT_UI_URL=http://127.0.0.1:1465
export BAUPROJEKT_CHROMIUM=/usr/bin/chromium
export BAUPROJEKT_ARBEITSPLATZ_ARTEFAKTE=/tmp/bauplan-workflow-audit/arbeitsplatz
export BAUPROJEKT_UI_ARTEFAKTE=/tmp/bauplan-workflow-audit/tabs
export BAUPROJEKT_UI_FELDER_ARTEFAKTE=/tmp/bauplan-workflow-audit/felder
export BAUPROJEKT_FACH_ARTEFAKTE=/tmp/bauplan-optimierung-fach
export BAUPROJEKT_FACH_UI_ARTEFAKTE=/tmp/bauplan-workflow-audit/fach
export BAUPROJEKT_DRUCK_ARTEFAKTE=/tmp/bauplan-optimierung-druck
python apps/desktop/tests/arbeitsplatz-ui.py
python apps/desktop/tests/fachberechnungen.py
python apps/desktop/tests/projekttabs.py
python apps/desktop/tests/ui-felder-navigation.py
python apps/desktop/tests/druckartefakt.py
python docs/workflow-audit/2026-10-23/probe.py
```

Die beiden `optimierung`-Verzeichnisse enthalten bereits zuvor erzeugte
Backend-Fixtures und sind nicht Bestandteil eines frischen Checkouts. Ihre
Erzeugung ist in den bestehenden Storage-Integrationstests
`crates/storage/tests/fachberechnungen.rs` bzw. `druckausgabe.rs` hinterlegt.
Vor einer Wiederholung die im jeweiligen Python-Skript verlangten Dateien und
Umgebungsvariablen prüfen. Die eigenständige neue `probe.py` verwendet dagegen
nur die eingecheckte Projektfixture, Regeln und den Testtransport.

## Abgrenzung

- Die Browser-IPC liefert kontrollierte Antworten. Deshalb bestätigen diese
  Läufe Bedienverhalten, nicht das erneute Ausführen aller Core-Berechnungen.
- Das PDF-Lesen kontrolliert vorhandene Core-Artefakte; die Fachprüfung ihrer
  Rechtsgrundlagen wurde nicht wiederholt.
- W02, Teile W03–W08 sowie W09–W11 enthalten zusätzlich Quellcodeprüfung und
  entworfene Abnahmeszenarien. Sie wurden nicht vollständig als native Abläufe
  vom Neuprojekt bis zur Behörde ausgeführt.
- Keine neuen Rust-, Server-, Lizenz- oder Buildprüfungen: Der Audit verändert
  nur Dokumentation und ein separates exploratives Prüfskript. Die vorhandene
  Produktionsoberfläche wurde verwendet.
- Keine realen CAD-/Office-Starts, Netzlaufwerks-/Mehrbenutzersitzungen,
  Portalübermittlungen, nativen Betriebssystemdialoge oder Screenreaderläufe.
- Keine Performance-/Zeitmessung mit realen Nutzern. Die in `probe.json`
  enthaltenen Elementzahlen sind reine Inventarisierung.

## Noch gezielt im Pilotbetrieb prüfen

1. Windows-Desktop mit Firmenserver und zwei tatsächlich gleichzeitig
   angemeldeten Nutzern; Konflikt lösen, ohne fremde Änderungen zu überschreiben.
2. Projekt mit mehreren hundert Dateien und gleichnamigen Planrevisionen;
   falsche Versionszuordnungen beobachten.
3. Echte kommunale Satzung und ein fachlich unabhängig gerechneter Vergleich
   für Stellplätze, Vollgeschosse und Abstandsflächen.
4. PDF mit vielen schlecht benannten AcroForm-Feldern, Schriftarten und
   mehrzeiligen Adressen; Originalausgabe und Druckseiten vergleichen.
5. Tatsächliche Portalabgabe mit Originalbeleg und Teilnachreichung, in einem
   zulässigen Test-/Pilotverfahren. Keine fingierten Echtanträge versenden.
6. Tastatur, Screenreader, native 200-Prozent-Skalierung und mehrere Monitore.

Die Softwareempfehlungen sind im [priorisierten Backlog](verbesserungen.md)
konkretisiert. Keine dieser Empfehlungen ist durch diese Dokumentation bereits
in der Anwendung umgesetzt.
