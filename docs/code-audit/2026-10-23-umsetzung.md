# Umsetzung der Codeanalyse

Ausgangspunkt: Main `74a9be6` und der [Bericht vom 22.10.2026](2026-10-22.md).
Arbeitsbranch: `codex/stabilitaet-performance`.

## Umsetzung je Befund

| Nr. | Änderung | Betroffene Grenze |
| --- | --- | --- |
| 1 | Ladegenerationen verhindern, dass alte Antworten oder Fehler neuere Listen überschreiben. Laufende Leseanfragen werden gezählt; Schreibaktionen behalten einen eigenen Beschäftigtstatus. | Gemeinsame `FachSitzung` |
| 2 | Ein erfolgreicher Backendcommit bleibt erfolgreich, auch wenn die lokale Entwurfsbereinigung scheitert. Verwerfen erhält den Entwurf bei einem Bereinigungsfehler. Wiederherstellung erkennt eine inzwischen abweichende Revision und erhält die Revisionsprüfung. | Editor → Backend / Entwurfsspeicher |
| 3 | Änderungsvergleiche verwenden abgeleitete, wiederverwendete Signaturen. Entwürfe werden nach 300 ms gebündelt in IndexedDB gesichert. Die vorhandenen JSON-Signaturen dienen als konsistente Schnappschüsse, ohne die reaktiven Objektbäume erneut zu klonen. | `entwurfspeicher.ts`, `FachSitzung`, `Fachliste` |
| 4 | Eine kurzlebige Lesesitzung lädt Quellenbestände je Modul nur einmal. Fingerprints lesen einzelne benötigte Objekte und aktuelle Dokumentstände. Versionen und Freigaben sind indiziert; die transitive Veraltung läuft über Rückwärtskanten statt wiederholter Gesamtdurchläufe. Abschließend werden Projekt-, Dokument- und schlanke Fachrevisionen erneut verglichen. | Nachweisprüfung / Repository |
| 5 | Technische Quellenfehler werden als `prueffehler` weitergegeben. Sie sperren eine positive Aktualitätsbestätigung und Freigabe und sind von einer fachlich geänderten Quelle unterscheidbar. Nachweisliste, Projektübersicht und Druckbericht zeigen den Unterschied. | Core-Vertrag → UI / PDF |
| 6 | PDF-Auswahl, Bilder, E-Mail-Berichte, Fachberichte, Formularaufbereitung, Paketbildung und PDF-Prüfung laufen als begrenzte Hintergrundjobs. Der Druck hat zusätzlich eine eigene Parallelitätsgrenze. Die Desktopoberfläche verwendet temporäre Artefakte und binäre Daten statt Base64-Rundtransport für Export und Projektablage. Fortschritt und kooperativer Abbruch sind angebunden. | Core → Tauri → Vorschau / Export / DMS |
| 7 | Druckexporte und lokale DMS-Schreibvorgänge verwenden einen gemeinsamen Helfer: temporäre Datei im Zielverzeichnis, vollständiges Schreiben, Synchronisieren, anschließendes Ersetzen. Fehler räumen temporäre Dateien auf. Bestehende Dateirechte bzw. Verzeichnis-/Umaskvorgaben werden berücksichtigt. | Dateisystemadapter |
| 8 | SQL-Abfragen unterstützen begrenzte Fachlisten, Einzellesen und Historien mit Cursorn. Editoren laden 100 Einträge, Fach- und Dokumenthistorien 50 Revisionen pro Seite. Dokumentenmanager und Übersicht fragen nur neueste Dokumentversionen ab. Suchtexte werden in kleinen Kandidatenseiten verarbeitet; SQLite nutzt einen Trigrammindex, PostgreSQL filtert normalisierte Inhalte vor der Übertragung. | Repository → Core → Adapter / UI |
| 9 | Sichtbarkeit wird pro Arbeitsbereich weitergegeben. Verdeckte Fach-, Aufmaß- und Quellenansichten merken Ereignisse vor und laden beim Aktivieren nach. Aufwendige PDF-Ansichten pausieren. Parallel laufende identische Leseabfragen werden gebündelt; Änderungen trennen Anfragegruppen. Wiedervorlagen prüfen weiterhin ihre Fristen, laden im Hintergrund aber keine Dokumenthistorien. | Lebensdauer der Projektbereiche |
| 10 | Fachkomponenten und Hilfetexte werden bedarfsgerecht geladen. Besuchte Editoren bleiben erhalten, einschließlich ihrer Entwürfe. Ladefehler eines Fachbereichs können erneut versucht werden. | Frontend-Bundle / Komponentenregister |
| 11 | Die Einstellungen bieten einen nicht destruktiven Ablageabgleich: Datenbankreferenzen, Dateiinhalte/Hashes, Sidecars und temporäre Dateien. Fehlende Metadaten können ausdrücklich aus referenzierten, hashgleichen Originalen ergänzt werden. Unbekannte Dateien und abweichende Inhalte werden niemals automatisch gelöscht oder ersetzt. | Administration / lokale und NAS-Ablage |

## Vertrags- und Betriebsdetails

- Die fachlichen Rechenverfahren und Regelstände werden durch diese Optimierungen nicht geändert. Grundstückseinheiten, Quellenidentitäten, CAS-Prüfung, Historien und archivierte Ergebnisse bleiben maßgebend.
- `NachweisAnsicht.prueffehler` ist ein zusätzliches optionales Vertragsfeld. Die TypeScript-Verträge sind aus Rust neu generiert. Alte Antworten ohne dieses Feld bleiben lesbar.
- Die Repository-Erweiterungen haben kompatible Standardimplementierungen. SQLite und PostgreSQL verwenden dafür direkte, begrenzte SQL-Abfragen. Vollständige Bestände bleiben für Fachauswertungen und Aktenexporte verfügbar.
- Die bestehenden HTTP-Fachlisten können optional per `nach`/`limit` abgefragt werden; Einzellesen und Historienseiten sind über `objekt` bzw. `vor` erreichbar. Die bestehenden vollständigen Aufrufe bleiben erhalten. Neue Tauri-Lesecommands prüfen dieselben Projektrechte wie die bisherigen Commands.
- SQLite erhält Migration `0014_suchtext_fts.sql`. Der ableitbare Trigrammindex wird zusammen mit den Fachständen aktualisiert. Neu indexierte Suchtexte enthalten eine interne normalisierte Suchprojektion. Altbestände ohne diese Projektion werden weiterhin vollständig geprüft und bleiben auffindbar; erneutes Indexieren übernimmt sie in den schnellen Pfad.
- Die PostgreSQL-Suche benötigt keine zusätzliche Servererweiterung. Sie filtert normalisierte Texte serverseitig; sie besitzt dadurch nicht die gleiche Trigrammbeschleunigung wie SQLite. Der fachliche Suchvertrag bleibt gleich.
- IndexedDB übernimmt vorhandene lokale Entwürfe im Schema 1. Neue Sicherungen verwenden Schema 2 mit JSON-Schnappschüssen. Beide Fassungen sind wiederherstellbar. Bei fehlendem Speicher wird der bearbeitbare Entwurf erhalten und eine Warnung angezeigt. Reguläres Speichern bleibt erforderlich; eine lokale Entwurfssicherung ersetzt keine Projektsicherung.
- Die Lesesitzung gilt nur für eine Auswertung. Sie ist keine Datenbanktransaktion über alle Repository-Traits. Ein zusätzlicher Revisionsvergleich erkennt zwischenzeitlich geänderte Quellen und fordert eine neue Auswertung an. Es gibt keinen dauerhaften Cache, der Quellen- oder Regeländerungen übergehen könnte.
- Höchstens zwei synchrone Datei-/PDF-Jobs sowie zwei Druckpakete laufen je Prozess gleichzeitig. Ein Parser wird nicht gewaltsam mitten in seiner Arbeit beendet; Abbruch erfolgt zwischen Quellen und Verarbeitungsschritten. Die Begrenzung bleibt auch bestehen, wenn ein wartender Aufrufer abbricht.
- Temporäre Druckartefakte gehören zu Benutzer und Projekt, sind auf acht offene Ausgaben mit jeweils höchstens 128 MiB begrenzt und werden beim Ersetzen/Schließen der Vorschau freigegeben. Beim Abmelden werden laufende Aufträge abgebrochen und Artefakte dieses Benutzers entfernt. Die bisherigen Base64-Commands bleiben kompatibel verfügbar.
- Die Ablageprüfung ist eine Momentaufnahme. Während paralleler Speicherungen können vorübergehend Dateien ohne Bezug gemeldet werden. Die Reparatur ergänzt ausschließlich fehlende Sidecars bei nachgewiesener Übereinstimmung mit der Datenbank. Sie verändert keine Originaldateien und löscht keine vermeintlichen Reste. S3 benötigt die Integritäts-/Sicherungswerkzeuge des Betreibers.

## Messungen

Der initiale JavaScript-Bundle sank im Produktionsbuild von rund **1.001 kB auf rund 638 kB**, gzip von rund **315 auf 201 kB**. Fachmodule und Handbuch werden separat geladen. Die Vite-Warnung für den weiterhin über 500 kB großen Hauptchunk bleibt sichtbar; ihre Grenze wurde nicht erhöht.

Die synthetische Editorprobe ist mit `apps/desktop/tests/performance-editor.py` wiederholbar. Die [Rohwerte nach Umsetzung](browserprobe-umsetzung.json) dokumentieren Entwicklungsbuild und Messbedingungen. Bei 10.000 Zeilen benötigte eine wiederholte Änderungsprüfung etwa 1–2 ms statt der vorher gemessenen ca. 104 ms. Die gebündelte asynchrone Sicherung lag in dieser Probe bei etwa 19 ms statt ca. 227 ms für die bisherige synchrone Sicherung.

Das sind Messungen auf einer gemeinsam genutzten Cloudmaschine, keine zugesagten Laufzeiten auf Bürohardware. Die erste Signatur eines sehr großen veränderten Editors bleibt linear zur Datenmenge und ist deutlich teurer als weitere Statusabfragen. Die Ergebnisse rechtfertigen keine Aussage, beliebig große Projekte seien ohne Verzögerung bearbeitbar.

Der Core protokolliert auf Debugstufe Laufzeiten und Mengen für Nachweisprüfung und Hintergrundjobs, ohne Dokumentinhalte zu protokollieren. Ein Regressionstest prüft ausdrücklich, dass 100 wiederholte Zugriffe in derselben Lesesitzung nur eine Nutzdatenabfrage für das Modul benötigen und eine neue Sitzung neue Revisionen liest.

## Validierung

Erfolgreich ausgeführt:

- `cargo fmt --all --check` und `cargo clippy --workspace --all-targets -- -D warnings`, einschließlich nativer Tauri-Commands und Testtargets.
- Generierte Plattformverträge gegen den Rust-Generator verglichen.
- 344 Bibliothekstests über Core (320), SQLite (6), PostgreSQL (1), Cache (4), Storage (12) und Server (1).
- 15 ausgewählte Storage-Integrationstests für Berechnungen, Abstandsflächen, Druck, Übersicht, Verfahrensakte und die neuen Stabilitätsfälle; die abschließend geänderten PDF-Pfade wurden nochmals gezielt geprüft.
- Alle vier PostgreSQL-Integrationssuiten gegen eine laufende PostgreSQL-17-Datenbank, einschließlich neuer Seiten-, Such- und Revisionsabfragen.
- Vier ausgewählte Server-Integrationstests für DMS und Projektzugriffe.
- 16 Node-Tests, Svelte-Prüfung mit 0 Fehlern/0 Warnungen, Produktionsbuild und npm-Lizenzprüfung (55 Pakete, 0 Verstöße).
- Zwölf Browserprüfungen: Projekttabs, Plattform/Entwurfsrettung, Fachberechnungen, Druckausgabe, binäre Artefakte, Projektübersicht, Handbuch, Arbeitsplatz, Eingabefelder/Navigation, verdeckte Ansichtsabfragen, PDF-Aufmaß und Gebäude/Flurstücke.
- Zusätzlich die Browserprobe `stabilitaet.py` mit gezielter Fehlersteuerung und die synthetische Editor-Messung.

Ein vollständiger Server-Integrationslauf wurde wegen erschöpftem Cloud-Speicherplatz abgebrochen. Nach Bereinigung ausschließlich generierter Compilerdateien wurden die genannten ausgewählten Serverprüfungen erfolgreich ausgeführt. Ein vollständiger `cargo test --workspace` einschließlich nativer Desktop-Laufzeittests wurde hier nicht ausgeführt.

Abgedeckt sind insbesondere:

- vertauschte Ladeantworten, verspätete Fehler und korrekter Beschäftigtstatus;
- Backendcommit mit fehlerhafter lokaler Bereinigung;
- Entwurfsmigration, große Entwürfe, Speicherquotenfehler, Versionskonflikte und ausstehende Sicherungstimer;
- Keyset-Seiten, Historien, Unicode-/Teilwortsuche, Suchindexänderungen und kompatible Alttexte;
- unveränderte Nachweis- und PDF-Verträge, inklusive Abstandsflächen und transitiver Quellenänderungen;
- atomarer Export bei Schreibfehlern, Artefaktisolierung und Begrenzung, kontrollierte Metadatenrekonstruktion;
- binäre Vorschau mit derselben Artefaktidentität für Export und DMS sowie Abbruch;
- pausierte verdeckte Bereiche und einmaliges Nachladen beim erneuten Aktivieren;
- Produktionsoberfläche, Projektwechsel, Darstellung und bedarfsgerecht geladene Hilfe.

Native Windows-Langzeittests, reale NAS-Ausfälle und große produktive Mehrbenutzerakten sind in dieser Linux-Cloudprüfung nicht enthalten. Ein erfolgreicher atomarer Austausch hängt außerdem von den zugesagten Eigenschaften des verwendeten Dateisystems ab; bei nicht unterstütztem Ersetzen meldet der Export einen Fehler und weicht nicht auf ein beschädigungsanfälliges Überschreiben aus.
