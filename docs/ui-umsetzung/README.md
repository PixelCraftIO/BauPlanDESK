# Moderne Projektakte – Umsetzung

Basis: Main `8b39a60`, Arbeitsbranch `codex/moderne-projektakte`.

Die Anwendung übernimmt das Arbeitsgerüst des freigegebenen Entwurfs 03: ruhige Typografie, helle Aktennavigation, Werkzeugleiste, flache Register und tabellarisches Projektverzeichnis. Die vorhandenen Fachfunktionen, Rollenprüfungen und BauNVO-Regelwerke bleiben eingebunden. Die Gestaltung funktioniert auch im dunklen Systemtheme und bei schmaleren Fenstern.

Mehrere Projekte können parallel geöffnet werden. Jede Akte hält eigene Formulare, Berechnungsergebnisse, Ergebnisaktualität, Dokumentansicht, CAD-Viewer und Scrollposition. Bereits offene Projekte werden aktiviert. Ein Punkt zeigt Änderungen; Speichern und native Import-/Export-Menüs arbeiten auf der jeweiligen Projektakte. Grundstücke haben ein Flurstücksverzeichnis mit ausgewählten Details und eine Auswahl der einzelnen Baugrundstücke.

Schließen einer Akte und Beenden der Anwendung berücksichtigen ungespeicherte Rechtsgrundlagen, Flurstücke/Nutzungsberechnung, CAD-Geometrie und Bauantrag. Speichern, Verwerfen und Abbrechen sind möglich. Fehlgeschlagenes Speichern aktiviert den betroffenen Fachbereich und erhält die Akte. Kundendaten bleiben beim Wechsel aus der globalen Kundenansicht erhalten und werden beim Beenden ebenfalls berücksichtigt. Laufende Aktionen verhindern vorzeitiges Schließen. Ein Bauantragskonflikt überschreibt den Entwurf nicht mehr; der aktuelle Stand kann bewusst neu geladen werden.

Projektlesezugriffe ändern den nativen Fenstertitel nicht mehr. Die Fenster-/Menüaktivierung erfolgt getrennt und geordnet, damit eine verspätete Antwort keine andere Akte aktiviert. Projektänderungen werden ausschließlich anhand der zurückgegebenen Projekt-ID der passenden Sitzung zugeordnet.

## Verifikation

- Svelte-Check: 0 Fehler, 0 Warnungen; Produktionsbuild erfolgreich.
- Rust: Desktop-Check und Desktop-Build erfolgreich; Clippy mit `-D warnings`, Formatprüfung.
- 103 core-lib-Tests und 2 SQLite-Tests bestanden. Keine Änderung der fachlichen Berechnungsregeln oder des Datenbankschemas.
- UI-Regressionen auf der kompilierten Anwendung mit injiziertem Tauri-Testtransport: unabhängige Entwürfe, Bereichswechsel, Hintergrundtab schließen, fehlgeschlagenes und verzögertes Speichern, Bauantragsentwürfe, native Import-/Export-Auslöser, letzter Tab, Beenden aller Akten und Leserrechte. Keine JavaScript-Fehler. 1024, 768 und 600 Pixel ohne horizontalen Seitenüberlauf.
- Native Desktop-Anwendung gestartet: echte Projektliste geladen, zwei vorhandene Projekte parallel geöffnet und zwischen den Akten gewechselt.

[UI-Prüfprotokoll](pruefung.json) · [Testanleitung](../../apps/desktop/tests/README.md)

## Aufnahmen

[Native Anwendung mit zwei Projektakten](09-native-projekttabs.png) · [Natives Verzeichnis](07-native-projektverzeichnis.png) · [Native Grundstücksansicht](10-native-grundstuecke.png) · [Grundstücksansicht mit Testdaten](02-grundstuecke-projekttabs.png) · [Dunkles Theme](06-dunkel.png)

Die Aufnahmen 01–06 stammen aus den UI-Regressionen mit Testtransport; 07–10 aus der nativen Anwendung mit bestehenden lokalen Projekten. Prototypen und frühere Analyse bleiben unter `docs/ui-analyse` erhalten.

## Grenzen

Offene Tabs und ungespeicherte Eingaben werden während der Anwendungssitzung gehalten. Eine Wiederherstellung von Entwürfen nach einem Absturz oder Neustart ist nicht enthalten. „Alle speichern“ speichert Fachbereiche nacheinander; bei einem Fehler bleiben die Akten offen, bereits erfolgreiche Speicherungen werden nicht rückgängig gemacht. Dokumentensperren bleiben ausdrücklich gesetzte, persistent verwaltete Sperren. Aktennummern und zusätzliche Bearbeitungsstatus aus dem Prototyp werden nicht als erfundene Produktdaten eingeführt.
