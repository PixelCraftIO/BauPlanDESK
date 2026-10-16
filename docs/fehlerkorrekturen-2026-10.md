# Korrekturen an Grundstücken, Desktop-Zugriff und Kalendertagen

## Verhalten und Verträge

- `plattform::objekte::geschosszuordnung` bildet Grundstücks-IDs aus Projekt-ID
  und Schlüssel des eingehenden Plans. Die Ableitung bleibt identisch zum
  öffentlichen Grundstückskontext. Veraltete gespeicherte Flurstücksquellen
  verhindern das erneute Speichern der Ermittlung nicht. Gebäude- und
  Geschosszuordnungen werden weiterhin je Grundstück geprüft.
- Desktop-Adapter prüfen Projektmitgliedschaften über `CoreServices` vor
  projektbezogenen Lese- und Schreiboperationen. Dokument- und Einreichungs-IDs
  werden über ihre tatsächliche Projektzuordnung geprüft. Projektlisten,
  Kunden-Projektlisten und die Liste zuletzt geöffneter Projekte werden gefiltert.
  Auch die automatische Ordnerausgabe prüft die Projektmitgliedschaft.
  Die globale Rolle und die Lizenz gelten weiterhin zusätzlich.
- Unter **Stammdaten → Projektmitglieder** können berechtigte Projektleiter
  Mitgliedschaften verwalten. Die neuen IPC-Befehle `projektmitglieder` und
  `projektmitglied_setzen` verwenden die vorhandenen typisierten Core-Verträge
  und deren Revisionsprüfung. Administratoren behalten projektübergreifenden
  Zugriff; neu angelegte Projekte besitzen bereits atomar die Mitgliedschaft
  ihres Erstellers. Bestehende Projekte erhalten keine pauschalen Freigaben.
- `kalender` definiert fachliche Kalendertage in `Europe/Berlin`, einschließlich
  Sommer-/Winterzeit. Bestätigungsdaten, Aufgaben und Fristen sowie die
  XBau-Dateinamen verwenden diese Sicht. Zeitstempel und Auditdaten bleiben UTC.
  Das Frontend verwendet dieselbe Zeitzone für Einreichungsfelder und Exportnamen.
- Historische BauNVO-Fassungen erzeugen auch ohne Flächenaufstellung keinen
  GRZ-II-Vergleich. Die vorhandene Antwortform bleibt erhalten; der nicht
  anwendbare Nachweis besitzt keine Ist-/Soll-/Erfüllungswerte. Gespeicherte
  Nutzungsnachweise verlangen bereits eine Flächenaufstellung und sind von
  diesem Fehler ohne Aufstellung nicht betroffen; ihr Rechenstand bleibt gleich.
- BauNVO-Vorgaben werden beim Laden auf Eingabe und Vergleichsstand angewendet.
  Das Öffnen erzeugt dadurch keine ungespeicherten Änderungen. Spätere
  Projektstände werden übernommen, solange kein eigener Entwurf vorhanden ist.
- Eine erfolgreiche Projektanlage wird auch bei fehlgeschlagener
  Bauherrnzuordnung geöffnet. Versionskonflikte behandeln auch Fehler beim
  Neuladen. Kundenverwaltung verwendet eine gemeinsame Instanz; Ladefehler bei
  der Bauherrnauswahl werden im Neuanlagedialog angezeigt. Die Statusleiste
  zeigt die durch die Lizenz wirksame Rolle.

## Kompatibilität und Verbraucher

Keine Datenmigration oder Änderung gespeicherter Projekt-/Vorlagenschemata.
Grundstücksidentitäten und bestehende Ergebnisstrukturen bleiben erhalten.
Core, Desktop und Server verwenden dieselbe korrigierte Nutzungsprüfung.
Bereits gespeicherte Nachweisstände werden nicht automatisch überschrieben.
Desktop-Commands bleiben Adapter; Berechtigungsentscheidungen und die
ID-basierte Zuordnung liegen im Core. Neue Hilfsmethoden sind additive Verträge.

## Validierung

- Core-Modultests einschließlich historischer BauNVO ohne Aufstellung und
  deutscher Mitternacht im Sommer/Winter sowie Zeitumstellungsgrenzen.
- Storage-Integrationstests: Grundstücksinformationen und Bestandsprojekte,
  Gebäudegrundlagen, Einreichungen, Portale und Projektübersicht sowie
  Archivkompatibilität der Nutzungsnachweise. Server: fünf Festsetzungstests.
- Desktop-Test `command_zugriff_tests`: tatsächliche Commands mit SQLite und
  DMS, fremde Projekte, Lesemitgliedschaft, Schreibmitgliedschaft,
  Mitgliedschaftsverwaltung, Erstellerzugriff und Export ohne unzulässige Ausgabe.
- `npm.cmd run check`, Produktionsbuild und Browserregression
  `apps/desktop/tests/fehler-regressionen.py` mit dem vorhandenen Testtransport.
  `node --test apps/desktop/tests/kalender.test.mjs` prüft deutsche Datumsgrenzen
  auch für die Frontend-Felder und Dateinamen.
