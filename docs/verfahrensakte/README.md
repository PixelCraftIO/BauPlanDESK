# Verfahrensakte und Büroabläufe

## Bedienung

Im Projekt unter **Organisation → Verfahrensakte** ein Verfahren anlegen.
Ein Projekt kann mehrere Anträge, Vorbescheide oder Änderungsverfahren enthalten;
Vorgängerverfahren und Behörden besitzen feste Identitäten.

1. **Verfahren:** Art, Behörde, Aktenzeichen, Bearbeitung und Zuständigkeit festlegen.
   Ein Vorbescheid erhält hier seine Fragen, eine Freistellung die Angabe, ob sie
   bei abschlägiger Entscheidung als Bauantrag weitergeführt werden soll.
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
6. **Anhörung / Zurückstellung:** Eine Anhörung vor belastender Entscheidung
   (§ 28 VwVfG) oder eine Zurückstellung nach § 15 BauGB an das eingegangene
   Schreiben binden. Die Anhörung erhält die Frist zur Stellungnahme und wird mit
   der ausgehenden Stellungnahme abgeschlossen; die Zurückstellung braucht ihr
   Ende als Frist. Art und Schreiben bleiben nach dem Speichern unverändert.
7. **Abgabestände:** Exakte Dokumentversionen auswählen und verbindlich festhalten.
   ZIP, optional XBau-ZIP und Druck-PDF werden einmal erzeugt und gespeichert.
   Die XBau-Nachricht folgt der Verfahrensart (0200, 0210 oder 0600, siehe
   [XBau](../xbau/README.md#nachricht-nach-verfahrensart)); Teilbaugenehmigung
   und „Sonstiges“ haben noch keine XBau-Ausgabe. Berechnungen können zuvor unter **Drucken / PDF-Ausgabe** erstellt und mit
   **Als Projektunterlage ablegen** ohne erneute Berechnung gespeichert werden.
   Empfänger und Aktenzeichen kommen aus der Verfahrensakte und werden mit dem
   Stand eingefroren: Druck-PDF und XBau nennen beide, XBau verlangt die
   Behördenkennung (DVDV) der Akte. Weicht die Bauaufsicht im Bauantrag ab, nennt
   der Stand das als Hinweis; „Behörde aus dem Bauantrag übernehmen“ füllt eine
   leere Kennung der Akte. Unter **XBau-Nachricht** wird eine Nachreichung auf den
   eigenen Antrag oder eine Befundliste bezogen; fehlt deren Aktenzeichen, gilt das
   der Akte, ein abweichendes wird abgewiesen. **Im XBau-Journal erfassen**
   übernimmt das eingefrorene XBau-ZIP als ausgehende Nachricht, auch mehrfach
   nur einmal.
8. **Übermittlung:** Versand, Eingang oder Zurückweisung einzeln mit Datum,
   Übermittlungsweg, Referenz und Beleg erfassen. Export bedeutet keinen Versand.
   Das Datum liegt nicht nach heute und nicht vor dem Festhalten des
   Abgabestands; eine frühere Einreichung wird als externe Einreichung
   nachgetragen.
   Bestätigter Eingang benötigt einen Dokumentbeleg. Frühere Versandereignisse
   bleiben unveränderlich; begründete Berichtigungen sind eigene Ereignisse.
9. **Antwort zuordnen:** Ein eingereichter Forderungspunkt braucht einen tatsächlich
   versandten/eingegangenen Abgabestand, der seine Antwortunterlagen enthält.
   Eine behördliche Bestätigung benötigt eine weitere eingehende Kommunikation.
   Wortlaut und Grundlage eines gespeicherten Forderungspunkts bleiben
   unverändert; änderbar sind Bearbeitungsstand, Zuständigkeit, Frist, Antwort
   und Unterlagen. Ein abweichender Wortlaut ist ein neuer Punkt, der bisherige
   wird begründet als entfallen geführt.
10. **Bescheide / Auflagen:** Originalbescheid, Zustellung und ausdrücklich
   genehmigte Dokumentversionen festhalten. Ein Bescheid setzt einen versendeten
   oder eingegangenen Abgabestand oder eine externe Einreichung voraus;
   genehmigte Unterlagen stammen aus solchen Abgabeständen oder aus den
   Unterlagen einer externen Einreichung. Ein fehlendes Zustelldatum lässt sich einmal
   nachtragen, danach ist der Bescheid unveränderlich. Nebenbestimmungen sind
   nach § 36 VwVfG typisiert (Auflage, Bedingung, Befristung, Widerrufs- und
   Auflagenvorbehalt) und werden mit Auslöser, Zuständigkeit und Nachweis
   überwacht; eine Befristung braucht ihr Ablaufdatum. Ein Hinweis begründet
   keine Pflicht und wird weder erfüllt noch wiedervorgelegt. Ältere Einträge
   gelten als Auflage. Wortlaut und Auslöser einer gespeicherten
   Nebenbestimmung bleiben unverändert; ein abweichender Wortlaut ist ein
   Folgebescheid. Ein späterer Entwurf ersetzt keine genehmigte Version.
   Folgebescheide ersetzen ausdrücklich benannte Bescheide.
11. **Korrekturen / Verfahrenschronik:** Dokumentersatz und Änderungsgründe
    verknüpfen, fachliche Ereignisse und sämtliche gespeicherten Stände nachsehen.

## Externe Einreichung

Für Anträge, die auf Papier, persönlich, per Fax oder im Portal ohne Abgabestand
der App eingereicht wurden (vor allem Altakten), im Register **Abgabestände**
„Externe Einreichung nachtragen …“ wählen: Tag der Einreichung, Weg (Post,
Portal, persönlich, E-Mail, Fax, sonstiger Weg mit Bemerkung), optional Beleg und
Bemerkung sowie optional die eingereichten Dokumentversionen der Ablage. Der Core
(`verfahren::externe_einreichung`, Befehl `verfahren_externe_einreichung`,
Server `POST /api/projekte/{id}/verfahren/{verfahren}/externe-einreichung`) setzt
Erfassungszeit und Benutzer; die allgemeine Speicherung der Akte nimmt keine
neuen Einträge an.

- Die Einreichung zählt wie ein versendeter Abgabestand als eingereicht
  (Statuswechsel, Bescheid). Sie friert keine Projektdaten ein, erzeugt keine
  Ausgabedateien und ist in der Übersicht als „Extern eingereicht“ gekennzeichnet.
- Als genehmigte Unterlagen eines Bescheids kommen nur die dort genannten
  Dokumentversionen in Frage; ohne sie ist nur eine Ablehnung oder sonstige
  Entscheidung erfassbar.
- Gespeicherte Einträge sind unveränderlich. Eine begründete Berichtigung ist ein
  Folgeeintrag und ersetzt den berichtigten Eintrag; jeder Eintrag wird höchstens
  einmal berichtigt.
- Antworten auf Forderungspunkte laufen weiter über Abgabestände.

## Datumsangaben

Kalendertage fachlicher Vorgänge gelten als deutsches Ortsdatum
(`kalender::heute`), auch für Aktenereignisse, Anschreiben der Druckausgabe,
Berichtsdaten, XBau-Anlagendatum, Formularwert `datum.heute` und Lizenzablauf.
Versand, Eingang, Zurückweisung, Kommunikation, Bescheid, Zustellung,
Vollständigkeit, Rücknahme, externe Einreichung und Chronikereignisse dürfen
nicht nach heute liegen; geprüft werden neue oder geänderte Angaben, ältere
Akten bleiben bearbeitbar. Fristen dürfen in der Zukunft liegen. Ein künftig
datierter Versand zählt nicht als Einreichung.

## Bearbeitungsstand

Der Core prüft jeden Wechsel des Bearbeitungsstands; die Oberfläche bietet nur
die Auswahl an.

| Stand | Voraussetzung |
| --- | --- |
| In Vorbereitung | Ausgangsstand; nach der Einreichung kein Rückweg |
| Eingereicht, In behördlicher Prüfung, Ruhend | versendeter oder eingegangener Abgabestand oder externe Einreichung |
| Unvollständig | zusätzlich ein offener Nachforderungspunkt |
| Vollständig | zusätzlich die behördliche Bestätigung mit Datum und eingegangenem Schreiben |
| Entschieden | zusätzlich ein Bescheid |
| Zurückgenommen | Rücknahmedatum und zugehöriges Schreiben |

Entschiedene und zurückgenommene Verfahren werden nicht wieder geöffnet; dafür
ein Folgeverfahren anlegen. Jeder Wechsel, die Vollständigkeitsbestätigung und die
Rücknahme erscheinen in der Verfahrenschronik. Bestehende Akten bleiben
bearbeitbar: Die Voraussetzungen gelten erst beim nächsten Statuswechsel.

## Berechnete Fristen

Zu jedem Bescheid mit Zustelldatum berechnet der Core nach dem Bundesland des
Projekts:

- **Rechtsbehelfsfrist:** ein Monat ab Zustellung (§§ 70, 74 VwGO), Ende nach
  §§ 187, 188 BGB am Tag gleicher Zahl oder am Monatsletzten. Fällt es auf einen
  Samstag, Sonntag oder landesweiten Feiertag, endet die Frist am nächsten
  Werktag (§ 57 Abs. 2 VwGO, § 222 Abs. 2 ZPO). Örtliche Feiertage, eine
  fehlerhafte Rechtsbehelfsbelehrung (dann ein Jahr) und Fristen der Nachbarn
  sind nicht berücksichtigt; die Oberfläche nennt das.
- **Geltungsdauer:** nur für Bauantrag, Änderungsantrag und Teilbaugenehmigung
  mit (teilweiser) Genehmigung. Die Tabelle in `crates/core-lib/src/verfahren/fristen.rs`
  führt die Jahre je Landesbauordnung (Bayern und Rheinland-Pfalz 4,
  Brandenburg 6, sonst 3). Unterbrechung und Verlängerung bleiben eine Prüfung
  nach Landesrecht.

Die Werte werden bei jeder Anzeige neu berechnet und nicht in der Akte
gespeichert, damit eine korrigierte Regel sofort für alle Bescheide gilt.

## Planindex und Einreichungsstatus

`crates/core-lib/src/verfahren/planindex.rs` (`ableiten`) leitet je Dokument aus
der gespeicherten Akte ab:

- **Planindex:** Jede Version, die erstmals in einem versendeten oder
  eingegangenen Abgabestand enthalten war, erhält den nächsten Buchstaben
  (A, B, …, Z, AA). Reihenfolge: Datum des maßgebenden Versandereignisses, dann
  Erstellung des Abgabestands. Vorbereitete, zurückgewiesene und ungeklärte Stände
  vergeben keinen Index.
- **Einreichungsstatus:** Entwurf (nicht eingereicht), Eingereicht (mit dem ersten
  versendeten Stand), Nachgereicht (später), Genehmigt (genehmigte Unterlage eines
  nicht ersetzten Bescheids) und Ersetzt (später eingereichte Version oder
  eingereichte Fassung aus einer Dokumentbeziehung; eine genehmigte Fassung bleibt
  genehmigt, bis der Nachfolger genehmigt ist). Eine neuere, nicht eingereichte
  DMS-Version wird als offene Version genannt.

Das Register „Abgaben“ zeigt die Tabelle für den gespeicherten Stand
(`GET /api/projekte/{id}/verfahren/{verfahren}/planunterlagen`, Desktop-Befehl
`verfahren_planunterlagen`). Beim Festhalten eines Abgabestands nennt das
Inhaltsverzeichnis der Druckausgabe (Anlagenverzeichnis) „Version · Index“; der
dort ausgewiesene Index wird im Abgabedokument als `planindex` eingefroren. Weicht
er später von der Ableitung ab (etwa wenn ein anderer Stand zuerst versendet
wurde), meldet die Ableitung das als Hinweis. Die XBau-Anlage enthält weiter nur
die DMS-Versionsnummer; ein Feld für den Planindex gehört zum XBau-Inhalt.

Der bisherige Unterlagenstatus der Projektunterlagen (Entwurf, Prüfung,
Freigegeben) bleibt unverändert als manuelle Bürofreigabe erhalten; er steuert
weiter die interne Freigabe vor der Einreichung. Der Revisionsindex der
Dokumentdaten bleibt die Kennung des Planverfassers.

## Suche und Wiedervorlagen

**Organisation → Suche / Wiedervorlagen** zeigt fällige Forderungspunkte,
Nebenbestimmungen, offene Anhörungen und das Ende einer Zurückstellung,
einschließlich eingereichter, noch nicht bestätigter Punkte.
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
