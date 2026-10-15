# Fachliche Kategorien für Projektunterlagen

Dateiname, Dateiformat, Ordner und fachliche Kategorie sind getrennte Angaben.
Jedes Dokument besitzt höchstens eine Hauptzuordnung, etwa „Baubeschreibung“,
„Lageplan“, „Anschreiben“, „E-Mail“ oder „Berechnung / GRZ-Berechnung“. DWG-, PDF-, Office-
und Bilddateien werden gleich behandelt. Ohne Zuordnung bleibt eine Unterlage
ausdrücklich „Ohne Kategorie“; Dateinamen werden nicht automatisch interpretiert.

## Bedienung

- **Projektdateien:** Kategorie beim Import vorwählen, an einer Datei ändern
  oder mehrere Unterlagen auswählen und gemeinsam zuordnen. Filtern nach einer
  Hauptkategorie umfasst deren Unterkategorien. Die Sortierung kann Kategorien
  als Gruppen anzeigen. „Dateiliste aktualisieren“ lädt auch den Katalog neu.
- **Formularbibliothek:** Standardkategorie einer Vorlage festlegen. Ausgefüllte
  Formulare erhalten sie atomar beim Anlegen im Projekt. Inhaltsgleiche Vorlagen
  teilen die Vorgabe anhand ihres SHA-256-Inhaltsstands. Eine bearbeitete
  Vorlagenkopie übernimmt die vorhandene Vorgabe, sofern das Ziel noch keine hat.
  Das Veröffentlichen eines Projektdokuments als Vorlage übernimmt dessen Kategorie.
- **Einstellungen → Dokumentkategorien:** Projektleitung und Administration
  können eigene Haupt-/Unterkategorien hinzufügen, Namen ändern und Kategorien
  archivieren. Es gibt zwei Hierarchieebenen. Bestehende Kennungen werden nicht
  gelöscht. Unterkategorien müssen vor ihrer Hauptkategorie archiviert werden.
- **Einreichungen:** Aktuelle Kategorien helfen bei der Auswahl. Die Chronik
  zeigt die zum Paket gehörenden Kategorien. ZIP und JSON-Manifest enthalten
  diese historischen Bezeichnungen; spätere Änderungen wirken sich nicht aus.

Archivierte E-Mails können als `.eml` oder `.msg` importiert und der Kategorie
„E-Mail“ zugeordnet werden. „Öffnen“ verwendet das auf dem Gerät zugeordnete
Mailprogramm; ein Mailimportdienst oder das Versenden von E-Mails ist dafür nicht
erforderlich. Auch als PDF archivierte Nachrichten können diese Kategorie erhalten.
Neue Standardkategorien ergänzen bereits gespeicherte Bürokataloge beim Lesen;
individuelle Namen und Archivierungen bleiben erhalten.

Eine Kategorieänderung erzeugt keine neue Dateiversion. Neue Versionen,
Arbeitskopien und Wiederherstellungen behalten die Zuordnung des Dokuments.
Das Ändern einer Vorlagenvorgabe kategorisiert bereits erstellte Dokumente nicht
nachträglich um. Archivierte Zuordnungen bleiben lesbar und können durch eine
aktive Kategorie ersetzt oder entfernt werden.

## Öffentlicher Vertrag und Speicherung

`core-lib::dokumentkategorien` besitzt Katalogregeln, Vorlagenvorgaben,
Dokumentzuordnungen und historische Kategoriesnapshots. Tauri und HTTP delegieren
an dieselben Services. TypeScript-Typen entstehen über `examples/vertraege.rs`.

- `KategorienKatalog`: Revision und Kategorien mit stabiler ID, Name,
  optionaler Eltern-ID und Aktivstatus. Der Katalog gilt für die gemeinsam
  verwendete Datenbank und damit auch für ein Büro mit mehreren Arbeitsplätzen.
- `KategorieZuordnung`: eigene Revision und optionale Kategorie-ID. `null`
  entfernt ausdrücklich eine Zuordnung; es bedeutet keine vererbte Vorgabe.
- `DokumentKategorie`: Dokumentidentität plus Zuordnung. Dateiversion und
  Metadatenrevision sind voneinander unabhängig.
- `KategorieStand`: ID, Name und damaliger Name der Hauptkategorie. Pakete
  speichern einen Snapshot je ausgewähltem Dokument, einschließlich `null`.
- Erzeuger von Berechnungsunterlagen können
  `CoreServices::store_document_kategorisiert` mit stabilen IDs wie `grz` oder
  `gfz` verwenden. Eine neue PDF-Berichtserzeugung für Berechnungen ist hierfür
  nicht erforderlich; bestehende Berechnungsabläufe werden nicht verändert.

Die vorhandenen Tabellen `einstellungen`, `dokumentkontexte`, `fachstaende` und
`fachhistorie` werden verwendet. Der installationsweite Katalog und die
Vorlagenvorgaben benutzen einen atomaren Vergleich des alten Einstellungswerts
in SQLite und PostgreSQL. Dokumentzuordnungen verwenden Fachrevisionen samt
Audit in einer Transaktion. Initiale Zuordnungen werden im Dokument-Commit
gespeichert. Gleichzeitige Änderungen ergeben einen Versionskonflikt.

Sammelzuordnungen sind einzelne, protokollierte Dokumentaktionen. Scheitert eine
Zuordnung, bleiben noch nicht gespeicherte Dokumente ausgewählt. Die Oberfläche
nennt die Zahl erfolgreicher Zuordnungen und lädt aktuelle Revisionen nach.
Fremde Dokumentensperren, Benutzerrechte und Lizenzzustand bleiben wirksam.

HTTP: `GET/PUT /dokumentkategorien`,
`GET /projekte/{id}/dokumentkategorien`,
`PUT /dokumente/{id}/kategorie`,
`GET/PUT /vorlagen/{sha256}/kategorie`, jeweils unter dem vorhandenen API-Präfix.
Projektbezogene Aufrufe durchlaufen die vorhandene Zugriffsprüfung.

## Kompatibilität und Prüfung

Es ist keine SQL-Migration nötig. Bestandsdokumente ohne Zuordnung erscheinen
als „Ohne Kategorie“. Ältere Einreichungspakete bleiben unverändert und zeigen
„Damals nicht erfasst“, wenn historische Kategorien fehlen.

Die Tests decken Katalogvalidierung, konkurrierende Speicherung in beiden
Datenbanken, Rechte und Sperren, Dateiversionen, Vorlagenvererbung, explizites
Entfernen, ZIP-Manifest, unveränderte Pakete und Neustart ab. HTTP-Tests prüfen
dieselben Verträge einschließlich unzulässiger Projektzugriffe. Der produktive
Browser-Test nutzt reale Core-Vertragsdaten mit ersetztem Tauri-Transport.
Seine Daten werden durch `storage/tests/dokumentkategorien.rs` bei gesetztem
`BAUPROJEKT_KATEGORIEN_ARTEFAKTE` erzeugt. Die Prüfungen laufen auch in CI.

Die E-Mail-Inhaltsansicht sowie Kategorieänderungen in den Dokumentangaben,
Kategorieanzahlen und der historische Vergleich sind in
[email-archiv.md](email-archiv.md) beschrieben.
