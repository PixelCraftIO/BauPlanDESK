# Umsetzung der Plattformanalyse

Arbeitsbranch: `codex/plattform-umsetzung`. Enthält auch die neue Projektnavigation und die ursprüngliche Analyse. Main wird nicht automatisch geändert.

## Umgesetzt und geprüft

- Grundstückskontext mit mehreren Einheiten, nachvollziehbaren Flächen und stabilen Mitgliedsidentitäten. Grundbuchkorrekturen verlieren keine Berechnungszuordnung; Teilung/Zusammenführung benötigt eine neue Zuordnung.
- Erhaltung von §34/§35 bei Einzelnachweisen; Abstandsflächenergebnisse werden bei Eingabeänderungen als veraltet angezeigt.
- Atomare Speicherung von Projekt, Dokument, Einreichung und Fachstand mit Audit; eigene Fachrevisionen und Historie. Rollback bei Auditfehler getestet.
- Unabhängiges Speichern von Stammdaten, Grundlagen, Festsetzungen und Grundstücken. Gleichzeitige Änderungen desselben Feldes erhalten einen Konflikt; andere Bereiche können weiterarbeiten.
- Gebäudemodell mit festen Geschossidentitäten und begründeter Vollgeschosseinstufung. Geschossigkeit je Gebäude; keine Addition der Geschosszahlen mehrerer Häuser.
- Gebäude, Aufgaben/Fristen, Unterlagenregister und Raumbuch als eigenständige Fachmodule; zentrale Registrierung statt zusätzlicher Root-Sonderfälle. Lokale Entwurfswiederherstellung je Benutzer und Projekt.
- Unveränderliche Nachweisstände mit Eingaben, Regelquelltext/Hash, Rechenstand und separater Freigabe. Archivierte Abstandsergebnisse können nach einem Regelupdate mit dem ursprünglichen Skript wiederholt werden.
- Formulare werden im Projekt ausgefüllt. Wiederholter Bibliotheks-Speichervorgang behält eine Dokumentidentität; Quellenwahl bei mehreren Bauherren/Grundstücken. Jede neue Formularversion besitzt atomar gespeicherte Feldwerte, Quelldaten und eine inhaltsadressierte Originalvorlage. Veraltete Quellvorschläge werden vor dem Speichern erkannt.
- Gemeinsame Statusübersicht für Fristen, Unterlagen, Nachweise und Einreichungen. Konfigurierbare Bürocheckliste aus JSON; erneutes Anwenden überschreibt keine Projektangaben. Dokumentangaben mit eigener Revision direkt in der Dateiverwaltung.
- HTTP-Server startet ohne konfigurierte Anmeldung nicht. Eigener verifizierter Benutzer je Anfrage, Projektmitgliedschaften, sofort wirksamer Widerruf und Kontodeaktivierung. API auch unter `/api/v1` verfügbar.
- Rust-generierte Plattformtypen und CI-Prüfung auf Abweichungen. Browserregressionen einschließlich echter PDF-Testdateien sowie neuer Fachmodule.

## Prüfung

Core-, SQLite-, PostgreSQL-, Storage- und HTTP-Integrationstests; Clippy der betroffenen Rust-Crates; Svelte-Typprüfung und Produktionsbuild. Browserprüfungen verwenden die gebaute Oberfläche mit einem Testtransport, ersetzen also keinen nativen Windows-/NAS-Pilot. Die zusätzlichen Modultests prüfen Entwurfsrettung, Fehlererhalt und Übergaben zwischen Gebäude und Raumbuch.

Das Raumbuch summiert erfasste Raumflächen und Volumen. Es behauptet keine WoFlV-/DIN-277-Berechnung. Die vorhandenen Rechtsregelansätze benötigen eine unabhängige fachliche Freigabe.

## Noch offen in der vollständigen Analyse

Projektorganisation; unveränderliche Paketschnappschüsse der Checklisten; vollständiger Desktop-Transport zum Büroserver; dauerhaft abgearbeitete Ausgabeaufträge; geprüfte Sicherung/Wiederherstellung; begrenzter IFC-Import und externe Freigabe-/Kommentaransicht. Native Windows-, Netzwerk- und Praxistests bleiben getrennte Abnahmen. Dieser Branch ist daher ein überprüfbarer Zwischenstand und noch keine abgeschlossene Umsetzung aller Maßnahmen A–D.

## Serverzugänge

Der Betreiber setzt `BAUPROJEKT_ZUGANGSDATEI` auf einen absoluten JSON-Pfad und verwendet dieselbe Datenbank-/Speicherkonfiguration wie beim Serverstart. Bereitstellung:

```sh
bauprojekt-server zugang-anlegen buero-admin administrator /absoluter/privater/pfad/admin.token
```

Der Token steht ausschließlich in der neu angelegten Datei, unter Unix mit Modus 0600. Die Zugangsliste speichert nur SHA-256. Wiederholte Bereitstellung für denselben Benutzer widerruft dessen frühere Tokens. HTTP-Aufrufe verwenden `Authorization: Bearer …`; außerhalb von localhost ist TLS erforderlich. Tokens gehören weder in das Repository noch in Browser-LocalStorage. Mitgliedschaften werden über `/api/v1/projekte/{id}/mitglieder` von berechtigten Projektleitern vergeben. Der Desktop nutzt derzeit weiterhin seine bisherigen lokalen/DB-Adapter; ein vollständiger Servermodus ist noch offen.
