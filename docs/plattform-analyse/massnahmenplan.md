# Priorisierter Maßnahmenplan

Dieser Plan gehört zur [Plattformanalyse](README.md). Die Pakete sind Vorschläge
für folgende Entwicklungsaufträge; sie wurden mit der Analyse nicht bereits
implementiert. Fachliche Prüfung und Pilotbetrieb sind eigene Arbeitsschritte.

Prioritäten:

- **P0:** Vor verlässlicher fachlicher Nutzung beziehungsweise vor dem konkret
  genannten Betriebsmodell lösen.
- **P1:** Grundlage des nächsten Produktausbaus.
- **P2:** Auf dieser Grundlage den Funktionsumfang erweitern.

Die Aufwandseinordnung S/M/L ist relativ: lokal, mehrere Module oder grundlegender
Ausbau. Sie ist keine Termin- oder Kostenzusage. Die vorhandenen
[Entwicklungsregeln](../entwicklungsregeln.md) gelten für jedes Paket.

## Stufe A: Bestehende Abläufe fachlich absichern

### A1 — Rechtskontext vollständig weitergeben · P0 · S

**Problem:** §35 geht im Adapter zur Grundstücksprüfung verloren.

**Umfang:** Übergabe in `BaugrundstuecksPlan::pruefe` korrigieren; ausdrücklich
festlegen, welche Bestandteile des Rechtskontexts die Nutzungsprüfung benötigt.
Den Fix von einer späteren großen Datenmodelländerung getrennt ausliefern.

**Abnahme:** Direkte und gruppierte Prüfung liefern bei §35 ohne Plan keine
automatischen §17-Sollwerte. §30-Festsetzungen und §34-Informationswerte bleiben
als unterschiedliche Fälle abgedeckt. Eigene Grundstücksfestsetzungen und
BauNVO-Anrechnung bestehen weiterhin. Quelle und fachliche Grenze der Bewertung
sind sichtbar.

### A2 — Ergebnisaktualität in Abstandsflächen · P0 · S/M

**Problem:** Ergebnis bleibt nach Eingabeänderung ohne Hinweis stehen.

**Umfang:** Bindung an Land, Eingaben und Regelstand; Request-Zuordnung;
Veraltet-/Fehlerzustand. Danach gespeichertes Nachweisobjekt als Teil von B3.

**Abnahme:** Änderungen an H oder Gebiet kennzeichnen das alte Ergebnis sofort.
Eine verspätete Antwort zu Eingabe A erscheint nicht als Ergebnis zu Eingabe B.
Projektwechsel und fehlendes Regelpaket ergeben verständliche Zustände. Die
Oberfläche behauptet keine vollständige Abstandsflächenprüfung allein aus H.

### A3 — Grundstücksfläche für Verbraucher verbindlich machen · P0 · M

**Problem:** Grundstücksermittlung und PDF-/Projektkontext können verschiedene
Flächen ausgeben.

**Umfang:** Öffentlichen Lesedienst für Grundstückseinheiten und Bezugsflächen
einführen. PDF-Kontext, Projektanzeigen und Bauantragsübergabe verwenden diesen
Dienst beziehungsweise dokumentierte Adapter. Einheiten und Verwendungszweck der
Fläche explizit angeben.

**Abnahme:** Bei 800 m² aus Flurstücken und altem Projektwert 640 m² kann kein
Formular still 640 m² vorschlagen. Bei mehreren Grundstücken wird ausgewählt oder
die Verwendung einer Summe ausdrücklich begründet. Abzüge und Gemeinschaftsanteile
werden nicht zweimal angewendet. Fehlende Ermittlung wird als fehlend angezeigt.

### A4 — Verlässliche Commit-Rückmeldung · P0 für robuste Ablage · M

**Problem:** Protokollfehler können einen bereits erfolgten Commit als Fehler
zurückgeben; wiederholte Anlage kann Duplikate erzeugen.

**Umfang:** Fachliche Daten und Audit in einer passenden Repository-Transaktion
schreiben. Für getrennte Nebeneffekte eine dauerhaft gespeicherte Auftragsliste
nutzen. Anlegeoperationen mit einer Vorgangskennung wiederholbar machen.
Bereits vorhandenes Storage-Cleanup und Ordnerwarnungen berücksichtigen.

**Abnahme:** Fehler vor dem Commit hinterlassen keinen bestätigten Datensatz.
Nach erfolgreichem Commit wird derselbe Vorgang als gespeichert erkannt. Ein
wiederholter Request mit derselben Kennung legt kein zweites Dokument/Paket an.
Tests injizieren Audit-, Storage- und Ausgabeausfälle an den richtigen Grenzen.

### A5 — Bestehende Prüfungen in CI verbindlich ausführen · P1 · S

**Problem:** UI-Suiten stehen nicht im CI-Workflow. Der PostgreSQL-Einreichungstest
wartet auf `BAUPROJEKT_TEST_POSTGRES_URL`, CI setzt nur `BAUPROJEKT_TEST_POSTGRES`.

**Umfang:** Testkonfiguration vereinheitlichen, bedingte Prüfungen sichtbar machen,
Produktionsbuild mit Playwright und echten PDF-Testartefakten prüfen. Die in A1–A4
gefundenen Kombinationen als Regressionen hinzufügen.

**Abnahme:** CI führt die PostgreSQL-Paketprüfung tatsächlich aus. Fehlende
Voraussetzungen erzeugen im verpflichtenden Job einen Fehler statt eines grünen
Tests ohne Prüfung. UI-Protokolle/Screenshots werden bei Fehlern als Artefakte
bereitgestellt. Rust-Typcheck, Format, Clippy und Lizenzprüfungen bleiben bestehen.

## Stufe B: Unabhängige Fachmodule wirklich tragen lassen

### B1 — Fachliche Speichergrenzen und Entwürfe · P1 · M/L

**Voraussetzung:** A1 und A3 liefern die fachlich korrekten Übergaben.

**Umfang:** Stammdaten, Rechtskontext, Grundstückseingaben und Nachweisansätze über
eigene Commands/Revisionen ändern. Nicht sofort jede JSON-Struktur in viele
Tabellen zerlegen; zuerst Eigentümerschaft und Transaktionen festlegen. Für JSON
ausdrückliche Schemaversionen und getestete Migrationen einführen.

**Abnahme:** Eine Person kann zwei unabhängige Bereiche ändern und nacheinander
speichern. Echte Überschneidungen führen zu einem verständlichen Konfliktvergleich.
Ein Rechtswechsel speichert die Grundlage und markiert abhängige Nachweise als
zu prüfen. Ein alter Berechnungsfehler blockiert keine unabhängige Kontaktdaten-
oder Grundlagenkorrektur. Speicherung bleibt projektbezogen und versioniert.

### B2 — Dauerhafte Grundstücks- und Gebäudeidentitäten · P1 · L

**Voraussetzung:** Öffentlicher Grundstücksvertrag aus A3; Schreibgrenzen aus B1.

**Umfang:** Grundstücks-ID vom Grundbuch-Gruppenschlüssel trennen. Vorgänge für
Aufteilung/Zusammenführung und Neuzuordnung vorsehen. Gebäude und Geschosse mit
stabilen IDs, Grundstücksbezug und Herkunft einführen. Geometrie optional verknüpfen.

**Abnahme:** Änderung eines Grundbuchblatts vertauscht keine Nachweise. Zwei
Gebäude auf einem Grundstück können je zwei Vollgeschosse besitzen, ohne als ein
viergeschossiges Gebäude bewertet zu werden. GFZ-Summen und Geschossigkeitsprüfung
verwenden ihre jeweils ausdrücklich definierte Aggregation. Bestehende Ansätze
werden eindeutig zugeordnet; unklare Zuordnungen verlangen eine Entscheidung.

### B3 — Einheitlicher Nachweisstand und Regelbindung · P1 · L

**Voraussetzung:** B1; Gegenstandsidentitäten aus B2 können schrittweise ergänzt werden.

**Umfang:** Eingaberevisionen, Hashes, Modul-/Regelversion, Ergebnisstatus und
fachliche Freigabe als definierten Vertrag speichern. Rechtsstände historisieren,
Beispiel-/Prüfstatus führen. Abhängige Ergebnisse gezielt ungültig machen.

**Abnahme:** Derselbe archivierte Stand ist mit demselben Regelpaket reproduzierbar.
Ein neuer Regelstand ersetzt keinen archivierten Nachweis. Grundstückskorrekturen
betreffen nur abhängige Ergebnisse. Unvollständig, veraltet, Grenzwert überschritten
und freigegeben sind unterscheidbar. Fachliche Referenzfälle werden unabhängig
von den Implementierungsformeln geprüft.

### B4 — Modulregistrierung, Transport und Editorvertrag · P1 · M

**Umfang:** Gemeinsame Modulbeschreibung für Navigation/Fähigkeiten/Rechte,
einheitlicher Editorvertrag, fachbezogene Frontend-Clients, erzeugte
Rust-/TypeScript-Verträge und dokumentierte HTTP-Schnittstelle. An zwei vorhandenen
Modulen einführen, anschließend weitere schrittweise umstellen.

**Abnahme:** Ein Testmodul beziehungsweise neuer kleiner Nachweis wird mit demselben
Muster registriert. Nicht in App, Sidebar und mehreren Listen separat ergänzen
müssen. Der Ausbau des Grundstücksmoduls besteht ohne Änderungen an PDF-Interna;
ein komplexeres PDF-Schema besteht ohne Anpassung des Grundstücksalgorithmus.
Ein Transporttest prüft, dass Tauri und HTTP denselben Use Case abbilden.

## Stufe C: Durchgängiger Arbeitsalltag im Planungsbüro

### C1 — Projektübersicht und konsistente Fachmasken · P1 · M

**Voraussetzung:** Ergebnisstatus aus A2/B3 schrittweise nutzbar.

**Umfang:** Projektkennzeichen, Zuständigkeit, Arbeitsstand und direkte offene
Prüfpunkte. Kleine Designbibliothek für Felder, Tabellen, Status und Dialoge.
Lokale Entwurfsrettung, nachvollziehbare Speicheranzeigen und zentrale Prüfliste.

**Abnahme:** Anwender finden offene Voraussetzungen und die betroffene Eingabe
ohne Suche durch mehrere Masken. Tastatur, Fokus, 200-%-Zoom, Hell/Dunkel/System
und lange Texte werden geprüft. Ein Crash-/Neustarttest stellt lokale Entwürfe
wieder her. Usability-Aufgaben werden mit Planern beobachtet und dokumentiert.

### C2 — Formularinstanzen und Dokumentrevisionen · P1 · M/L

**Voraussetzung:** A3 und B3; Metadaten können vorher inkrementell beginnen.

**Umfang:** Vorlage und PDF-Version dauerhaft referenzieren; Feldzuordnung über
fachlichen Datenkatalog; Dokumentart, Plannummer und fachliche Revision ergänzen.
Pflichtfelder, Datentypen, Validierung und Quellenhinweise unterstützen.

**Abnahme:** Vorlagenänderung verändert keine bestehenden Formularinstanzen.
Eine Grundstücksänderung kennzeichnet abhängige Entwürfe. Der Anwender erkennt,
welcher Datenstand die erzeugte PDF befüllt hat. Historische eingereichte Dateien
bleiben unverändert. Ein Formular für mehrere Bauherren/Grundstücke besitzt eine
eindeutige und prüfbare Zuordnung.

### C3 — Unterlagenprüfung und Einreichungsablauf · P1/P2 · M

**Umfang:** Verfahrensabhängige Checklisten, Verantwortliche, Prüf-/Freigabestatus
und dokumentierte Abweichungen. Den vorhandenen Versionsvergleich um nachvollziehbare
Nachträge und empfängerbezogene Stände ergänzen. Aufgaben/Fristen an diese Objekte
anhängen.

**Abnahme:** Ein Paket kann mit fehlenden Unterlagen als Entwurf gespeichert
werden, wird aber nicht still als vollständig bezeichnet. Das neueste lokal
gespeicherte Dokument ist nicht automatisch beim Amt. Versand-/Einreichungsdatum
und tatsächlicher Empfang sind unterscheidbar. Ein Nachtrag nennt die konkret
ersetzten und ergänzten Revisionen.

### C4 — Gemeinsamer Büroserver · P0 vor Öffnung, ansonsten P1 · L

**Umfang:** Authentifizierter API-Zugang, Projektmitgliedschaften und Rechte,
zentrale Regel-/Vorlagenstände und Server-Dateizugriff. Netzwerkdesktop von
direktem PostgreSQL-Zugang auf API-Transport umstellen. Ereignis-/Änderungsabgleich
für offene Projektakten einführen; Konfliktauflösung verwendet B1.

**Abnahme:** Unangemeldete Anfragen können keine Projekte lesen oder anlegen.
Ein berechtigter Nutzer sieht nur freigegebene Projekte. Widerrufene Rechte gelten
auch bei bereits geöffneten Clients. Zwei Arbeitsplätze bearbeiten und aktualisieren
dasselbe Projekt ohne stillen Datenverlust. Keine Datenbankzugänge sind im
gewöhnlichen Netzwerkclient nötig. Betriebsrollen und Infrastrukturzugriffe sind
getrennt dokumentiert.

### C5 — Backup, Diagnose und Hintergrundaufträge · P1 · M/L

**Umfang:** Vollständiges Sicherungskonzept mit Wiederherstellungsassistent/-anleitung,
Auftragsstatus für Ordnerausgabe/Vorschau/ZIP, Bereitschaftsprüfung, Diagnose und
geprüfte Installations-/Updatewege. Zuständigkeit für gemeinsame Speicherorte festlegen.

**Abnahme:** Wiederherstellung auf frischem Rechner mit Datenbank, DMS, Vorlagen
und Regelständen. Nicht erreichbarer Serverordner wird später ohne doppelte
Dokumentanlage nachgeholt. Lange Aufgaben zeigen Fortschritt und lassen sich
kontrolliert abbrechen. Windows-Dateiverknüpfungen, Installation und Update werden
auf Windows geprüft; Linux-Ergebnisse ersetzen diese Prüfung nicht.

## Stufe D: Die Plattform fachlich erweitern

### D1 — IFC-/Planbezug und Themenbearbeitung · P2 · L

**Voraussetzung:** B2/B3 und belastbare Dokumentrevisionen.

**Umfang:** Begrenzter IFC-Leseimport mit expliziten Einheiten, Objekt-IDs und
Übernahmebericht. Später BCF-Themen, Planüberlagerung und Änderungsliste. Nicht
gleichzeitig einen kompletten BIM-Editor und alle Formate umsetzen.

**Abnahme:** Importgeometrie ist von bestätigten Fachwerten unterscheidbar.
Objektidentitäten bleiben bei Reimport zuordenbar. Geänderte Modellwerte markieren
abhängige Nachweise. Importfehler verändern keine freigegebenen Projektstände.

### D2 — Weitere Fachmodule und Integrationen · P2 · variabel

**Voraussetzung:** Einbauvertrag aus B4 an realen Features bewiesen.

**Umfang:** Nach Pilotbedarf zunächst ein Modul auswählen: Wohn-/Nutzflächen,
Flächen-/Raumbuch, Kostenstruktur oder Geodaten/XPlanung. Für externe Systeme
eigene Adapter und versionierte Mappingregeln verwenden.

**Abnahme:** Erweiterung verwendet gemeinsame Gegenstände und Quellen ohne
parallel gepflegte Stammdaten. Bestehende Grundstücks-, PDF- und Einreichungs-
Vertragstests bestehen weiter. Fachliche Freigabe und Betriebskosten sind geklärt.

### D3 — Webportal, externe Beteiligte oder SaaS · P2 · L

**Voraussetzung:** C4/C5; bei mehreren Büros zusätzlich geprüfte Mandantentrennung.

**Umfang:** Zuerst begrenzter Anwendungsfall, etwa freigegebene Unterlagen lesen
und kommentieren. Browserfähige Dateiabläufe, Einladung, Rechte und Ablaufdaten.
Später Portalübermittlung mit tatsächlich getesteten Empfängerschnittstellen.

**Abnahme:** Organisationen und Projekte sind über alle Daten-, Datei-, Such- und
Jobpfade getrennt. Ein Link erlaubt nur den freigegebenen Stand. Download, Änderung,
Widerruf und Empfang sind nachvollziehbar. Last-/Betriebsgrenzen sind gemessen.

## Empfohlene Startreihenfolge

1. A1 und A2 als kleine, eigenständig prüfbare Korrekturen.
2. A3 zusammen mit passenden Verbrauchertests; A4 für sichere Schreibvorgänge.
3. A5, damit die neuen Übergaben fortlaufend geprüft werden.
4. B1 und B4 an den bestehenden Grundlagen-/Grundstücks-/PDF-Modulen umsetzen.
5. B2/B3, danach C1–C3 für den zusammenhängenden Planungsablauf.
6. C4 vorziehen, sobald der gemeinsame Bürobetrieb beziehungsweise externe
   Zugriff das erste Produktziel ist. C5 vor einem ernsthaften Büro-Pilotbetrieb.
7. D1–D3 nach beobachtetem Bedarf und nachgewiesenen Grundlagen.

Nicht alle Pakete benötigen einen gemeinsamen Großumbau. Kleine Änderungen
behalten Adapter und dokumentierte Übergaben bei. Jeder Schritt endet mit einem
benutzbaren Stand, aussagekräftigen Tests und einer konkreten Abnahme.
