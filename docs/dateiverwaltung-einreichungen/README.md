# Projektdateien und Einreichungsstände

## Alle Projektdateien verwalten

Die Dokumentenablage nimmt beliebige Dateien auf, darunter DWG, Excel, Word,
Fotos und PDF. Die Dateisuche filtert nach Namen oder Endung (`.xlsx` usw.).
Alle Dateien verwenden dieselbe Dokumentidentität, Versionshistorie und
unveränderliche DMS-Ablage. Die Originalanwendungen müssen auf dem Arbeitsplatz
installiert und dem jeweiligen Dateityp zugeordnet sein.

**Im zugeordneten Programm öffnen** erzeugt eine lokale Arbeitskopie mit der
angezeigten Basisversion und startet die Dateizuordnung des Betriebssystems:
Windows ShellExecuteW, macOS `open`, Linux `xdg-open`. Der DMS-Dateispeicher wird
nicht direkt im externen Programm geöffnet. Ausführbare Programme und Skripte
können gespeichert/exportiert, aber nicht über diese Aktion gestartet werden.

Nach dem Speichern in Excel, CAD oder einem anderen Programm:

1. **Änderungen prüfen** aktualisiert die Liste der externen Arbeitskopien.
2. **Als neue Version übernehmen** speichert den aktuellen Inhalt in derselben
   Dokumenthistorie. Eine unveränderte Arbeitskopie erzeugt keine neue Version.
3. Die Basis der Arbeitskopie wird erst nach dem DMS-Commit aktualisiert; die
   lokale Datei bleibt erhalten. Weitere Bearbeitung kann erneut übernommen werden.

Arbeitskopien und ihre Basisinformationen bleiben unter `Arbeitskopien` im
App-Datenordner dieses Geräts über Neustarts erhalten. Die Liste ist projektbezogen.
Bei Konflikten oder fremden Dokumentsperren bleiben die Arbeitskopie und die
bereits gespeicherten Versionen erhalten. Ein Startfehler des externen Programms
löscht die erstellte Kopie ebenfalls nicht. Direktes Öffnen/Übernehmen ist bis
512 MB je Arbeitsdatei vorgesehen. Es gibt keine automatische Synchronisierung,
keinen Dateibeobachter und keinen automatischen Import beim Schließen von Excel.

Im Büro verbleiben die verbindlichen Dateiversionen auf der konfigurierten
NAS-/S3-/Serverablage; die lokale Arbeitskopie wird erst durch die ausdrückliche
Übernahme als neue Version für andere Nutzer verfügbar. CAD-Dateien können
zusätzliche externe Referenzen benötigen: Einzeldateien werden nicht automatisch
als XRef-/Verknüpfungsverbund erkannt oder gebündelt.

## Einreichungen und Nachreichungen

Neue Einreichungen und Nachreichungen entstehen als **Abgabestand der
Verfahrensakte** (siehe [Verfahrensakte](../verfahrensakte/README.md)). Er friert
Unterlagen, Behörde, Aktenzeichen, XBau-Nachricht und Druck gemeinsam ein. Der
XBau-Export im Bauantrag, „Ausgang/Nachreichung vorbereiten“ im XBau-Postfach und
das Erstellen neuer Einreichungspakete entfallen; diese Stellen führen zur
Verfahrensakte.

Der Projektbereich **Einreichungen** zeigt die früher erstellten
Einreichungspakete unverändert:

- Die Chronik nennt je Paket die festgehaltenen Unterlagen, Versionen, Prüfsummen
  und die Kategorie zum Zeitpunkt des Pakets.
- **Dieses ZIP erneut exportieren** liefert dieselben Bytes wie beim Erstellen.
- **Einreichungsdatum erfassen** bestätigt ein noch unbestätigtes Paket einmalig
  mit Datum, Benutzer, Bestätigungszeit und Notiz. Export allein bestätigt keine
  Einreichung.

Das ZIP enthält die Dateiversionen unter
`Unterlagen/<Dokument-ID>/v<Version>-<Dateiname>` sowie `Einreichungsstand.json` mit
Dokumentidentitäten, Dateinamen, Versionen und Prüfsummen. Die Bestätigung wird
separat in der Chronik gespeichert und verändert das ZIP nicht.

Die Projektübersicht vergleicht weiterhin den Behördenstand bestätigter Pakete
je Empfänger mit den aktuellen Dokumentversionen. Der Stand wird über alle
bestätigten Pakete eines Empfängers zusammengestellt; das tatsächliche
Einreichungsdatum bestimmt die Reihenfolge, bei gleichem Datum die
Bestätigungszeit, danach die Paket-ID.

Das Bestätigen erfordert Bearbeitungsrechte und eine gültige Lizenz. Eine
begonnene Bestätigung muss vor dem Schließen abgeschlossen oder abgebrochen
werden.

## Architektur und Migration

- `core-lib::dateiverwaltung`: typisierter `Arbeitsstand` mit konkreter DMS-Basis,
  unterstützte Dateitypen für den Betriebssystemaufruf.
- `storage::arbeitskopie`: lokale bearbeitbare Kopien und atomare Basisaktualisierung.
  Die Desktop-Commands starten Programme und verwenden den bestehenden
  `CoreServices::store_document`-Vertrag für neue Versionen.
- `core-lib::einreichung`: Paketvalidierung, Inhaltsstand, ZIP-Erzeugung und
  Empfängervergleich. Das eigene Service-Modul koordiniert Rechte, Dokumente,
  unveränderliche ZIP-Ablage, Repository und Änderungsprotokoll.
- `EinreichungRepository`: eigenes Persistenzinterface; SQLite und PostgreSQL
  implementieren denselben Vertrag mit atomarer Basisversionsprüfung bei Bestätigung.
- Migration `0007_einreichungen.sql` ergänzt jeweils eine Tabelle und einen Index.
  Bestehende Projekte, Dokumentversionen und Formularvorlagen bleiben unverändert.
- Neue Core-/Desktop-Aufrufe ergänzen bestehende Verträge. HTTP:
  `GET /api/projekte/{id}/einreichungen` (neue Pakete nur noch über die
  Verfahrensakte; das Erstellen ist im Core auf Testdaten beschränkt),
  `GET /api/projekte/{id}/einreichungsvergleich?empfaenger=…`,
  `POST /api/einreichungen/{id}/bestaetigen` (`basis`, `datum`, `notiz`),
  `GET /api/einreichungen/{id}/zip`.

Pakete werden in der bestehenden Storage-Ablage unter dem jeweiligen Projekt
archiviert, unabhängig von den veränderlichen Arbeitskopien und von ZIP-Exporten
auf dem Gerät. Datenbank und Dateispeicher müssen gemeinsam gesichert werden.
Dieser ursprüngliche Dateiverwaltungsstand verwendete noch ein technisches Serverkonto. Die [Plattformumsetzung](../plattform-umsetzung/README.md) ersetzt es durch verifizierte Benutzerzugänge und Projektmitgliedschaften.

## Nachweise

- Rust-Integration mit realer SQLite-Datenbank und Dateispeicher: Arbeitskopien,
  Versionskonflikt, feste ZIP-Inhalte, initiale Einreichung, Teilnachreichung,
  gleiche Inhalte, historische Daten, Empfängertrennung und Neustart.
- PostgreSQL-Test mit dedizierter Testdatenbank: Migration, Persistenz und
  gleichzeitige Bestätigung – genau ein erfolgreicher Commit.
- HTTP-Integration: kein Erstellen neuer Pakete, ZIP laden, Bestätigung und
  Konflikt, Versionsvergleich nach Korrektur, unverändertes ursprüngliches ZIP.
- Kompilierte UI mit Test-IPC: Dateiöffnung, externe Änderung, Konflikt,
  frühere Pakete, Exportfehler, Chronik, Datumsbestätigung und Weg zur Abgabe. Die UI
  verwendet Daten aus der realen Rust-Integration. Excel/AutoCAD unter Windows
  werden in dieser Linux-Testumgebung nicht tatsächlich gestartet.

[UI-Prüfprotokoll](ui-pruefung.json)

![Dateiverwaltung mit externer Arbeitskopie](01-dateiverwaltung.png)
![Einreichungsstände und Versionschronik](02-einreichungschronik.png)
