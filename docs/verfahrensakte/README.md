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
   Ende als Frist und endet mit diesem Datum oder vorher mit einem eingegangenen
   Folgeschreiben der Behörde. Danach ist sie nicht „überfällig“, sondern die
   Übersicht wartet auf die Entscheidung. Art und Schreiben bleiben nach dem
   Speichern unverändert.
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
   Unterlagen einer externen Einreichung. Ein fehlendes Zustelldatum lässt sich
   nachtragen; ein erfasstes wird nur mit **begründeter Berichtigung** geändert
   und nicht entfernt, sonst ist der Bescheid unveränderlich. Eine Genehmigung
   oder Ablehnung wird zusammen mit dem Stand „Entschieden“ gespeichert.
   Nebenbestimmungen sind nach § 36 VwVfG typisiert. Nur **Auflage und
   Bedingung** sind Pflichten: Sie bleiben auch nach der Entscheidung bis zur
   Erfüllung mit Beleg in den Wiedervorlagen (Bauausführung). **Befristung,
   Widerrufs- und Auflagenvorbehalt** sind nicht erfüllbar: Mit Frist erinnern
   sie vor dem Ablauf und enden mit dem Ablaufdatum, ohne Frist werden sie nur
   geführt; eine Befristung braucht ihr Ablaufdatum. Ein Hinweis begründet keine
   Pflicht. Ein ablehnender Bescheid trägt nur Hinweise. Ältere Einträge gelten
   als Auflage. Wortlaut und Auslöser einer gespeicherten Nebenbestimmung
   bleiben unverändert; ein abweichender Wortlaut ist ein Folgebescheid. Ein
   späterer Entwurf ersetzt keine genehmigte Version.
   Folgebescheide ersetzen ausdrücklich benannte Bescheide; ein
   **Verlängerungsbescheid** nennt den verlängerten Bescheid und das neue Ende
   der Geltungsdauer (je Bescheid höchstens eine Verlängerung, weitere beziehen
   sich auf die letzte).
11. **Bauausführung:** Baubeginn und Fertigstellung im Register Verfahren
   erfassen (setzt eine Einreichung voraus, nicht in der Zukunft). Der Baubeginn
   beendet die Wiedervorlage der Geltungsdauer, die Fertigstellung die der
   Fertigstellungsfrist. Gespeicherte Daten ändern sich nur mit Berichtigung.
12. **Korrekturen / Verfahrenschronik:** Dokumentersatz und Änderungsgründe
    verknüpfen, fachliche Ereignisse und sämtliche gespeicherten Stände nachsehen.
    Die **Zeitleiste** zeigt jede Revision mit der vom Core abgeleiteten Art
    (Bearbeitungsstand, Abgabe, Übermittlung, Bescheid, Zustellung,
    Nebenbestimmung, Nachforderung, Anhörung/Zurückstellung, Bauausführung,
    Berichtigung …) und lässt sich danach filtern.

### Berichtigung erfasster Werte

Zustelldatum, Vollständigkeitsbestätigung, Rücknahme, Baubeginn und
Fertigstellung sind nach dem Speichern gesperrt. **… berichtigen** öffnet das Feld
und verlangt einen Grund. Die Oberfläche legt dafür ein neues Aktenereignis mit
`berichtigung: { gegenstand, grund }` an; der Core prüft, dass genau dieser Wert
sich ändert, trägt alten und neuen Wert ein und schreibt den Text
„Berichtigung …: alt → neu · Grund: …“. Eine Änderung ohne Berichtigung und eine
Berichtigung ohne Änderung werden abgewiesen.

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
<<<<<<< HEAD
| In Vorbereitung | Ausgangsstand; zurück nur, solange weder ein Stand versendet oder eingegangen ist noch eine externe Einreichung vorliegt |
=======
| In Vorbereitung | Ausgangsstand; zurück nur, solange keine Einreichung erfasst ist |
>>>>>>> d6827f5e993c875089dcc484db5b709b09a4196c
| Eingereicht, In behördlicher Prüfung, Ruhend | versendeter oder eingegangener Abgabestand oder externe Einreichung |
| Unvollständig | zusätzlich ein offener Nachforderungspunkt |
| Vollständig | zusätzlich die behördliche Bestätigung mit Datum und eingegangenem Schreiben |
| Entschieden | zusätzlich ein Bescheid mit Genehmigung, Teilgenehmigung oder Ablehnung (bei Freistellung und „Sonstiges“ jeder Bescheid) |
| Zurückgenommen | Rücknahmedatum und zugehöriges Schreiben |

Entschiedene und zurückgenommene Verfahren werden nicht wieder geöffnet; dafür
ein Folgeverfahren anlegen. Eine Rücknahme wird nur zusammen mit dem Stand
„Zurückgenommen“ gespeichert; eine zurückgenommene Akte erhält keine neuen
Bescheide, Nebenbestimmungen, Nachforderungen, Anhörungen, Zurückstellungen
oder Abgabestände mehr. Ist nach einer Rückweisung kein Stand mehr versendet,
darf das Verfahren in die Vorbereitung zurück.

Bleibt der Stand gleich, darf eine Speicherung einen bisher erfüllten Stand
nicht ungültig machen: Wird etwa die einzige Abgabe zurückgewiesen, der letzte
offene Forderungspunkt eines unvollständigen Verfahrens bestätigt oder die
Vollständigkeitsbestätigung entfernt, verlangt der Core, den Bearbeitungsstand
in derselben Speicherung anzupassen. Jeder Wechsel, die Vollständigkeitsbestätigung
und die Rücknahme erscheinen in der Verfahrenschronik. Ältere, schon
widersprüchliche Akten bleiben bearbeitbar.

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
  führt die Jahre je Landesbauordnung mit Quelle (Bayern und Rheinland-Pfalz 4,
  Brandenburg 6 ohne Verlängerung, Berlin 2 bis zum Baubeginn, sonst 3) und
  landesrechtliche Hinweise (Unterbrechung in Hamburg und Niedersachsen). Das
  Ende wird nicht auf einen Werktag verschoben. Während eines Widerspruchs oder
  einer Klage Dritter kann die Frist ruhen (z. B. OVG Hamburg 2 Bs 179/14); das
  nennt die Oberfläche, berechnet es aber nicht. Ein Verlängerungsbescheid
  ersetzt das Ende.
- **Fertigstellung:** in Berlin zusätzlich sechs Jahre nach Erteilung bis zur
  Fertigstellung (§ 73 Abs. 1 BauO Bln).
- **Vorbescheid und Freistellung:** ohne berechnete Frist. Beim Vorbescheid
  rät der Hinweis, sein Ablaufdatum als Befristung zu erfassen; die Freistellung weist auf
  die nicht berechnete Wartefrist nach Eingang bei der Gemeinde hin.

Die Werte werden bei jeder Anzeige neu berechnet und nicht in der Akte
gespeichert, damit eine korrigierte Regel sofort für alle Bescheide gilt. Jede
Berechnung nennt ihren `regelstand` (aktuell `2026-10-18`; Berlin wurde dabei
von 3 auf 2 Jahre berichtigt, die Werte für Sachsen sind nur über eine
Sekundärquelle bestätigt).

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

**Organisation → Suche / Wiedervorlagen** zeigt fällige Termine ab ihrem
Vorlauf. Wiedervorlagen, Projektübersicht und Kalenderexport lesen dieselbe
Ableitung (`core-lib::verfahren::termine`):

| Termin | Vorlauf | endet |
| --- | --- | --- |
| Forderungspunkt mit Frist | eigener | mit Bestätigung, Entfallen oder der Entscheidung über das Verfahren |
| Auflage / Bedingung mit Frist | eigener | mit Erfüllung (auch nach der Entscheidung) |
| Befristung / Vorbehalt mit Frist | eigener | mit dem Ablaufdatum, nie überfällig |
| Anhörung | eigener | mit Stellungnahme oder Entscheidung |
| Ende der Zurückstellung | eigener | mit dem Fristende, einem Folgeschreiben oder der Entscheidung |
| Rechtsbehelfsfrist | 14 Tage | mit ihrem Ablauf |
| Geltungsdauer | 90 Tage | mit erfasstem Baubeginn oder Verlängerungsbescheid; danach überfällig |
| Fertigstellungsfrist (Berlin) | 90 Tage | mit erfasster Fertigstellung |

Zurückgenommene Verfahren, ablehnende und ausdrücklich ersetzte Bescheide
erzeugen keine Termine. Der Projektarbeitsstand verweist ebenfalls auf offene
Punkte und Verfahrensfristen.

**Termine als Kalender (ICS) exportieren** schreibt alle laufenden Termine aller
Verfahren als Kalenderdatei nach RFC 5545: ganztägige Einträge, gefaltete Zeilen,
maskierte Texte, eine Erinnerung mit dem Vorlauf und eine stabile UID je Termin,
sodass ein erneuter Import Einträge aktualisiert statt verdoppelt.

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
`suchtext`, `aktenexport`, `termine.ics` (GET, Leserecht), `druckausgabe/unterlage`.

Das Projektprotokoll führt Verfahrensänderungen mit eigenen Aktionen
(`verfahren_angelegt`, `verfahrensstatus_geaendert`, `verfahren_abgabe`,
`verfahren_bescheid`, `verfahren_nachforderung`, `verfahren_schritt`,
`verfahren_ausfuehrung`, `verfahren_berichtigt`, sonst `verfahren_geaendert`),
abgeleitet aus dem Aktenereignis der Revision; bei mehreren Änderungen gilt die
gewichtigste, der Protokolltext nennt alle. Ältere Programmstände an derselben
Datenbank kennen diese Aktionen nicht und können das Protokoll erst nach dem
Update lesen.
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
