# Verfahrensakte und Büroabläufe

## Bedienung

Im Projekt unter **Organisation → Verfahrensakte** ein Verfahren anlegen.
Ein Projekt kann mehrere Anträge, Vorbescheide oder Änderungsverfahren enthalten;
Vorgängerverfahren und Behörden besitzen feste Identitäten.

1. **Verfahren:** Art, Behörde, Aktenzeichen, Bearbeitung und Zuständigkeit festlegen.
2. **Vorhabenprofil:** Bundesland aus dem Projekt, Verfahrensart, Gebäudeklasse,
   Sonderbau und Nutzung erfassen. Einen mitgelieferten oder eigenen JSON-Katalog
   anwenden. Vorhandene Zuordnungen bleiben erhalten. Bei geändertem Profil oder
   Regelstand werden betroffene Freigaben zurückgesetzt.
3. **Anforderungen:** Erforderlichkeit ausdrücklich als ungeklärt, erforderlich
   oder nicht erforderlich entscheiden; Quellen, Gründe und konkrete
   Dokumentversionen zuordnen. Projektleitung/Büroleitung geben fachlich frei.
4. **Kommunikation:** Eingehende Schreiben, archivierte E-Mails, Portalnachrichten,
   Telefonate und Besprechungen mit Originalbeleg und typisierten Aktenbezügen
   erfassen. Bereits gespeicherte Originalangaben werden durch Folgeeinträge
   berichtigt. E-Mail-Anhänge lassen sich in den Projektdateien gezielt als eigene
   Unterlagen übernehmen; ihre Nachrichtenversion bleibt als Herkunft erhalten.
5. **Nachforderungen:** Aus einer eingehenden Kommunikation einzelne Punkte
   anlegen. Jeder Punkt besitzt eigene Zuständigkeit, Antwort, Unterlagen,
   Frist, Erinnerung und Bearbeitungszustand. Teilantworten sind möglich.
6. **Abgabestände:** Exakte Dokumentversionen auswählen und verbindlich festhalten.
   ZIP, optional XBau-ZIP und Druck-PDF werden einmal erzeugt und gespeichert.
   Berechnungen können zuvor unter **Drucken / PDF-Ausgabe** erstellt und mit
   **Als Projektunterlage ablegen** ohne erneute Berechnung gespeichert werden.
7. **Übermittlung:** Versand, Eingang oder Zurückweisung einzeln mit Datum,
   Übermittlungsweg, Referenz und Beleg erfassen. Export bedeutet keinen Versand.
   Bestätigter Eingang benötigt einen Dokumentbeleg. Frühere Versandereignisse
   bleiben unveränderlich; begründete Berichtigungen sind eigene Ereignisse.
8. **Antwort zuordnen:** Ein eingereichter Forderungspunkt braucht einen tatsächlich
   versandten/eingegangenen Abgabestand, der seine Antwortunterlagen enthält.
   Eine behördliche Bestätigung benötigt eine weitere eingehende Kommunikation.
9. **Bescheide / Auflagen:** Originalbescheid, Zustellung und ausdrücklich
   genehmigte Dokumentversionen festhalten. Nebenbestimmungen mit Auslöser,
   Zuständigkeit und Nachweis überwachen. Ein späterer Entwurf ersetzt keine
   genehmigte Version. Folgebescheide ersetzen ausdrücklich benannte Bescheide.
10. **Korrekturen / Verfahrenschronik:** Dokumentersatz und Änderungsgründe
    verknüpfen, fachliche Ereignisse und sämtliche gespeicherten Stände nachsehen.

## Suche und Wiedervorlagen

**Organisation → Suche / Wiedervorlagen** zeigt fällige Forderungspunkte und
Nebenbestimmungen, einschließlich eingereichter, noch nicht bestätigter Punkte.
Auch nach einer Genehmigung bleiben offene Pflichten sichtbar. Zurückgenommene
Verfahren und ausdrücklich ersetzte Bescheide erzeugen keine aktiven Erinnerungen.
Der Projektarbeitsstand verweist ebenfalls auf offene Punkte und Verfahrensfristen.

Die Suche umfasst Kommunikation, Anforderungen, Antworten, Aktenereignisse,
Nebenbestimmungen, Dateinamen aller Versionen und indexierte Dokumentinhalte.
PDF-Texte sowie EML-/MSG-Inhalte lassen sich gezielt indexieren. Scan-PDFs können
seitenweise mit lokalem **Tesseract** (Sprachpakete `deu` und `eng`) erkannt werden.
Der Index gehört zur exakten Dokumentversion und ihrem SHA-256; er verändert
niemals das Original. OCR-Ergebnisse sind Suchhilfen und können Fehler enthalten.
Grenzen: PDF-Text maximal 300 Seiten, OCR maximal 50 Seiten pro Auftrag,
12 Millionen Pixel pro OCR-Bild; ein gleichzeitiger OCR-Prozess, 60 Sekunden je Bild.

Wiedervorlagen werden im geöffneten Suchbereich minütlich aktualisiert.
Optionale Systemhinweise benötigen eine ausdrückliche Freigabe in der Anwendung.
Es gibt keinen Hintergrunddienst und keinen automatischen E-Mail-Versand bei
beendeter Anwendung. Fristen werden manuell mit Auslöser, Grund und Beleg erfasst;
gesetzliche Fristen werden nicht aus unvollständigen Angaben erfunden.

## Konfigurationskataloge und fachliche Grenzen

`workflow-regeln/verfahrensprofil.json` ist ein versionierter, allgemeiner
Bürokatalog mit dreizehn Prüfpositionen. Eigene Kataloge können im Profilbereich
als JSON geladen werden (maximal 256 KB). Der öffentliche Vertrag
`Anforderungskatalog` wird aus Rust nach TypeScript generiert.

- Katalog: `id`, `version`, `titel`, `hinweis`, optional `bundesland` und `art`.
- Regeln: stabile `id`, `titel`, `quelle`; optionale Bedingungen `sonderbau`,
  `ab_gebaeudeklasse` und `nutzung`.
- Fehlende oder nicht passende Angaben führen zu einer offenen fachlichen Prüfung,
  niemals automatisch zu „nicht erforderlich“.
- Inhaltliche Katalogänderungen brauchen eine neue Version. Stabile Regelkennungen
  erhalten vorhandene Dokumentzuordnungen und begründete Entscheidungen;
  Freigaben müssen bei geändertem Profil/Regelstand erneut geprüft werden.
- Bundesland und Verfahrensart müssen zu Projekt und Katalog passen.

Der mitgelieferte Katalog ist **kein amtlich freigegebenes Landespaket** und
bescheinigt keine rechtliche Vollständigkeit. Ein fachlich geprüftes Pilotpaket
benötigt aktuelle Landesvorschriften, konkrete amtliche Formulare und eine
verantwortliche fachliche Abnahme. Auch die dokumentierten Grenzen der
bestehenden Berechnungsregelwerke bleiben bestehen. XBau verlangt weiterhin
passende Antragsdaten, Kunden-/Bürodaten und gültige Codelisten. Grundstücksfakten
werden für den eingefrorenen XBau-Stand aus den zentralen Flurstücksdaten bezogen,
sofern diese erfasst sind.

## Export und Wiederherstellung

**Vollständige Projektakte exportieren** erzeugt ein portables ZIP mit:

- allen Original-Dokumentversionen mit lesbaren Dateinamen;
- JSON-Manifest mit Projekt, Kunden/Bauherren, Antrag, Fachhistorien, Kategorien,
  Dokumentherkunft und Auditprotokoll;
- unveränderten Abgabe-Artefakten und bisherigen Einreichungspaketen;
- Quell-PDFs und Feld-/Quellenständen ausgefüllter Formulare;
- einer lesbaren Dateiübersicht (`Liesmich.txt`).

Grenzen: 8.000 Dokumentversionen und 512 MB unkomprimierter Exportinhalt.
Das Aktenarchiv dient der Übergabe/Dokumentation. Für eine vollständige
betriebsfähige Wiederherstellung die Profilsicherung verwenden.

Unter **Einstellungen → Sicherungen** unterstützt der Netzwerkmodus eine
PostgreSQL-Sicherung zusammen mit eingebundener NAS-/zentraler Dateiablage,
Vorlagen und Profilkonfiguration. Benötigt `pg_dump`, `pg_restore` und `psql`
im PATH, jeweils in der Hauptversion des Datenbankservers. Zugangsdaten werden
nicht als Prozessargumente weitergereicht. Archive können Zugangsdaten der
Profilkonfiguration enthalten und gehören in eine geschützte Büroablage.

Wiederherstellung erzeugt ein neues absolutes Profilverzeichnis und setzt eine
**neue, leere Zieldatenbank** voraus. Befüllte Datenbanken und bestehende Ziele
werden abgewiesen. Die Anwendung aktiviert das neue Profil erst nach ausdrücklicher
Auswahl und Neustart. Arbeitskopien vor der Sicherung ins DMS übernehmen.
S3-Snapshots, externe Archivdienste, Portalübermittlung und ein vollständiger
Desktopzugang über den HTTP-Server sind weiterhin getrennte Betriebsintegrationen.
Die neue NAS-Sicherung ersetzt keine native Windows-/NAS-Praxisabnahme.

## Modulgrenzen und Prüfung

`core-lib::verfahren` besitzt Fachverträge, Referenzprüfung, Profile, Suche und
Abgabestände. DMS, Berechnungen und PDF-Verarbeitung bleiben eigenständige
Verbraucher/Produzenten. Ein Verfahren besitzt eine revisionierte Fachakte;
Änderung und Audit werden atomar geschrieben. Veraltete Revisionen überschreiben
keine parallelen Bearbeitungen. Originale, genehmigte Stände und frühere Abgaben
werden durch neue Stände ergänzt. Dafür ist keine SQL-Schemamigration erforderlich.

Tauri und HTTP verwenden dieselben Core-Anwendungsfälle. Die neuen HTTP-Wege
liegen unter `/api/projekte/{id}/...`: `verfahrensprofil`,
`verfahren/{verfahren}/profil`, `verfahren/{verfahren}/abgabe`,
`verfahren/{verfahren}/abgaben/{abgabe}/{format}`, `erinnerungen`, `aktensuche`,
`suchtext`, `aktenexport`, `druckausgabe/unterlage`.
E-Mail-Anhänge: `POST /api/dokumente/{id}/email/{version}/anhaenge/{index}`.
Lesen und Schreiben respektieren Projektzugriffe; fachliche Freigaben bleiben
Projektleitung/Büroleitung vorbehalten. OCR läuft lokal im Desktopadapter.

Automatisierte Abnahme: echtes Musterverfahren mit drei Forderungspunkten,
Teilantwort, zurückgewiesenem Upload, Eingang, Fristverlängerung, Genehmigung mit
Version 2 und anschließendem Entwurf 3. Geprüft werden unveränderte ZIP-/PDF-/XBau-
Artefakte, Suchindex, Aktenexport, Projektgrenzen, Leserrechte, konkurrierende
Bearbeitung, Audit-Rollback und eine echte PostgreSQL-Wiederherstellung.
Der Browser prüft den Produktionsbuild mit Transportfixtures und echten Core-
Dokumenten; Tesseract wird separat mit einem tatsächlichen Bildbeleg geprüft.
