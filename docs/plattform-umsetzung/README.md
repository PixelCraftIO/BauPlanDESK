# Umsetzung der Plattformanalyse

Arbeitsbranch: `codex/plattform-umsetzung`. Enthält auch die neue Projektnavigation und die ursprüngliche Analyse. Main wird nicht automatisch geändert.

## Umgesetzt und geprüft

- Grundstückskontext mit mehreren Einheiten, nachvollziehbaren Flächen und stabilen Mitgliedsidentitäten. Grundbuchkorrekturen verlieren keine Berechnungszuordnung; Teilung/Zusammenführung benötigt eine neue Zuordnung.
- Erhaltung von §34/§35 bei Einzelnachweisen; Abstandsflächenergebnisse werden bei Eingabeänderungen als veraltet angezeigt.
- Atomare Speicherung von Projekt, Dokument, Einreichung und Fachstand mit Audit; eigene Fachrevisionen und Historie. Rollback bei Auditfehler getestet.
- Unabhängiges Speichern von Stammdaten, Grundlagen, Festsetzungen und Grundstücken. Gleichzeitige Änderungen desselben Feldes erhalten einen Konflikt; andere Bereiche können weiterarbeiten.
- Gebäudemodell mit festen Geschossidentitäten und begründeter Vollgeschosseinstufung. Geschossigkeit je Gebäude; keine Addition der Geschosszahlen mehrerer Häuser.
- Gebäude, Aufgaben/Fristen, Unterlagenregister und Raumbuch als eigenständige Fachmodule; zentrale Registrierung statt zusätzlicher Root-Sonderfälle. Lokale Entwurfswiederherstellung je Benutzer und Projekt.
- Unveränderliche Nachweisstände mit Eingaben, Regelquelltext/Hash, Rechenstand und separater Freigabe. Archivierte Abstands- und BauNVO-Ergebnisse können nach einem Regelupdate mit den ursprünglichen Skripten bzw. JSON-Regeln wiederholt werden. Regelhash und Metadaten werden geprüft; nicht unterstützte Rechenkernstände werden ausdrücklich abgewiesen.
- Formulare werden im Projekt ausgefüllt. Wiederholter Bibliotheks-Speichervorgang behält eine Dokumentidentität; Quellenwahl bei mehreren Bauherren/Grundstücken. Jede neue Formularversion besitzt atomar gespeicherte Feldwerte, Quelldaten und eine inhaltsadressierte Originalvorlage. Veraltete Quellvorschläge werden vor dem Speichern erkannt.
- Gemeinsame Statusübersicht für Fristen, Unterlagen, Nachweise und Einreichungen. Konfigurierbare Bürocheckliste aus JSON; erneutes Anwenden überschreibt keine Projektangaben. Dokumentangaben mit eigener Revision direkt in der Dateiverwaltung.
- Projektkennzeichen, Phase und Verantwortlichkeit werden unabhängig von Rechtsgrundlagen gespeichert. Einreichungspakete archivieren die gelesenen Fachrevisionen, Anforderungen und Nachweisfreigaben. Vorschau der Auswahl vor dem Export; Vorgangskennung verhindert doppelte Pakete bei technischen Wiederholungen.
- Dauerhafte Ausgabeaufträge mit Bearbeitungsfrist, Wiederholungsabstand und Fehlerstatus. Der Desktop setzt neue Dateiausgaben nach einem Neustart fort. Ein veralteter Versuch kann einen neu beanspruchten Auftrag nicht bestätigen. Früher konfigurierte Ziele werden nicht im Hintergrund erneut beschrieben; gezielte Ausgabe am aktuellen Ziel ist möglich.
- Einzelplatzsicherung mit SQLite-Snapshot einschließlich WAL, allen DMS-Versionen, Formularquellen, Vorlagen und lokalen Einstellungen/Regeln. Hashprüfung und Wiederherstellung in ein neues Profil; Umschalten erst nach ausdrücklicher Auswahl und Neustart. Das bisherige Profil bleibt erhalten. Beschädigte Archive und Pfadmanipulationen werden abgewiesen.
- HTTP-Server startet ohne konfigurierte Anmeldung nicht. Eigener verifizierter Benutzer je Anfrage, Projektmitgliedschaften, sofort wirksamer Widerruf und Kontodeaktivierung. API auch unter `/api/v1` verfügbar. `/health` prüft Datenbankzugriff und tatsächliches Schreiben/Lesen/Löschen im Storage (Cache maximal 30 Sekunden). Entzogene Rechte gelten auch an den Core-Schreibgrenzen bereits geöffneter Desktop-Arbeitsplätze.
- Rust-generierte Plattformtypen und CI-Prüfung auf Abweichungen. Browserregressionen einschließlich echter PDF-Testdateien sowie neuer Fachmodule.

## Prüfung

Core-, SQLite-, PostgreSQL-, Storage- und HTTP-Integrationstests; Clippy für den gesamten Workspace; Svelte-Typprüfung und Produktionsbuild. Browserprüfungen verwenden die gebaute Oberfläche mit einem Testtransport, ersetzen also keinen nativen Windows-/NAS-Pilot. Die zusätzlichen Modultests prüfen Entwurfsrettung, Fehlererhalt und Übergaben zwischen Gebäude und Raumbuch.

Das Raumbuch summiert erfasste Raumflächen und Volumen. Es behauptet keine WoFlV-/DIN-277-Berechnung. Die vorhandenen Rechtsregelansätze benötigen eine unabhängige fachliche Freigabe.

## Noch offen in der vollständigen Analyse

Vollständiger Desktop-Transport zum Büroserver; begrenzter IFC-Import und externe Freigabe-/Kommentaransicht. Native Windows-, Netzwerk- und Praxistests bleiben getrennte Abnahmen. Dieser Branch ist daher ein überprüfbarer Zwischenstand und noch keine abgeschlossene Umsetzung aller Maßnahmen A–D.

## Serverzugänge

Der Betreiber setzt `BAUPROJEKT_ZUGANGSDATEI` auf einen absoluten JSON-Pfad und verwendet dieselbe Datenbank-/Speicherkonfiguration wie beim Serverstart. Bereitstellung:

```sh
bauprojekt-server zugang-anlegen buero-admin administrator /absoluter/privater/pfad/admin.token
```

Der Token steht ausschließlich in der neu angelegten Datei, unter Unix mit Modus 0600. Die Zugangsliste speichert nur SHA-256. Wiederholte Bereitstellung für denselben Benutzer widerruft dessen frühere Tokens. HTTP-Aufrufe verwenden `Authorization: Bearer …`; außerhalb von localhost ist TLS erforderlich. Tokens gehören weder in das Repository noch in Browser-LocalStorage. Mitgliedschaften werden über `/api/v1/projekte/{id}/mitglieder` von berechtigten Projektleitern vergeben. Der Desktop nutzt derzeit weiterhin seine bisherigen lokalen/DB-Adapter; ein vollständiger Servermodus ist noch offen.

### Prüfung von Projekt-, Dokument- und Einreichungspfaden

Die Zugangs-Middleware prüft die Projektberechtigung auf den **dekodierten** Pfadsegmenten, also mit derselben Sicht wie der Router, der Pfadparameter vor dem Handler dekodiert. Eine prozentkodierte ID (`/api/projekte/%33…`) wird wie die unkodierte geprüft. Unter `/api/…` und `/api/v1/…` gilt: Steht nach `projekte`, `dokumente` oder `einreichungen` keine lesbare UUID oder ist der Pfad ungültig kodiert, antwortet der Server mit 400, ohne den Handler zu erreichen. Alle projektbezogenen Routen (auch `…/verfahren/{v}/portale/…`) tragen die Projekt-ID an zweiter Stelle; Projekt- oder Dokumentkennungen in Query-Parametern und Bodies (z. B. `vorgang`, `Dokumentbezug`) prüft der Core zusätzlich gegen das Projekt des Pfads. `HEAD` gilt als schreibende Methode und verlangt daher Bearbeitungsrecht.

### Fehlermeldungen, Upload-Grenzen und Exporte

- 5xx-Antworten enthalten nur „Interner Serverfehler …“ und eine `kennung` (UUID). Die Fehlerdetails (Pfade, Datenbankmeldungen) stehen mit derselben Kennung im Serverprotokoll. 4xx-Meldungen bleiben fachlich aussagekräftig.
- Body-Grenzen: `POST /projekte/{id}/druckausgabe/unterlage` nimmt bis `druckausgabe::MAX_DRUCKPDF` (128 MiB) an, wie der Core. Fachobjekte und Suchtexte (Core-Grenze 2 MiB Inhalt) erlauben 4 MiB Body für Hülle und Escapes. Alle anderen Routen behalten die axum-Vorgabe von 2 MB; Lizenzdateien und JSON-Aufträge liegen deutlich darunter.
- Aktenexport (`GET /projekte/{id}/aktenexport`) und Einreichungs-ZIP (`GET /einreichungen/{id}/zip`) werden in eine anonyme temporäre Datei geschrieben (Verzeichnis über `TMPDIR`) und gestreamt; das Gesamtarchiv liegt nicht im Speicher. Je Prozess laufen höchstens `exportgrenze::MAX_GLEICHZEITIGE_EXPORTE` (2) große Exporte gleichzeitig, weitere warten. Der Aktenexport in eine Datei ist bis 2 GiB unkomprimiert möglich (Archiv sicher unter 4 GiB), der Speicherweg `verfahren_aktenexport` bleibt bei 512 MB. Der Desktop schreibt beide Exporte über eine Teildatei `Ziel.teil` und benennt sie erst nach Erfolg um.

### Sichtbarkeit von Kundendaten

`GET /kunden`, `GET /kunden/{id}` und der Desktop-Befehl `kunden_auflisten` verwenden die Core-Regel in `zugriff.rs`: Wer Kunden pflegen darf (Bearbeitung, Projektleitung, Administration), sieht die gesamte Kundendatenbank. Benutzer mit der Rolle Lesen sehen nur Kunden, die als Bauherr oder Beteiligter in einem Projekt eingetragen sind, das sie lesen dürfen; ein anderer Kunde liefert 403.


## Sicherungen und Profile

Unter Einstellungen → Speicherorte stehen Sicherung, Archivprüfung und Wiederherstellung zur Verfügung. Sicherungen werden mit neuem Dateinamen angelegt. Die SQLite-Kopie wird vor dem Einlesen der unveränderlichen Dateien erzeugt; alle in der Kopie referenzierten Dokumente, Pakete und Formularquellen müssen in der Ablage vorhanden sein und ihre erwarteten Hashes erfüllen. Zusätzliche, erst später erzeugte Blobs können enthalten sein, ändern aber den Datenbankstand nicht.

Die Wiederherstellung entpackt zuerst in ein temporäres Verzeichnis und prüft alle Manifestdateien. Danach wird die SQLite-Integrität geprüft und die Ablagepfade werden auf das neue Profil gesetzt. Erst „Wiederhergestelltes Profil nach Neustart verwenden“ ändert die Profilwahl. „Nach Neustart zum Standardprofil zurückkehren“ bleibt auch für Leser verfügbar, damit ein abweichender Benutzerbestand in einem wiederhergestellten Profil den Rückweg nicht sperrt. Betriebssystemrechte schützen die lokalen Profile. Nicht übernommene externe Arbeitskopien gehören nicht zum DMS-Stand und müssen vor der Sicherung übernommen werden.

Für den Büroserver ist das Einzelplatzarchiv kein Ersatz für eine koordinierte PostgreSQL-/Storage-Sicherung. Betreiber müssen PostgreSQL etwa mit `pg_dump --format=custom` sichern, die unveränderliche zentrale Ablage einschließlich Vorlagen und Serverkonfiguration sichern und die Kombination in einer getrennten Datenbank/Storage-Wurzel wiederherstellen. Zugangstokens werden getrennt verwahrt bzw. neu bereitgestellt. Ein produktiver Windows-/NAS-Wiederherstellungspilot steht noch aus.

Browseransichten mit Testtransport: [Raumbuch](screenshots/raumbuch.png) · [Sicherungen](screenshots/sicherungen.png).
