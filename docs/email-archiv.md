# Archivierte E-Mails und erweiterte Dokumentkategorien

In **Projektdateien → E-Mail lesen** lassen sich archivierte `.eml`- und
`.msg`-Dateien direkt ansehen. Die Ansicht zeigt Betreff, Absender, An/Cc/Bcc,
Datum, Nachrichten-ID, Nachrichtentext und Anhangsangaben. Über die Dateiversion
kann ein älterer Originalstand ausgewählt werden. „Im Mailprogramm öffnen“ nutzt
den vorhandenen Ablauf zum Öffnen einer Arbeitskopie im zugeordneten Programm.

Das Lesen verändert weder Originaldatei, Dokumentversion, Kategorie noch Audit.
Die Ansicht lädt immer die ausdrücklich ausgewählte Dokumentidentität und
Version. Verspätete Antworten ersetzen keine neuere Auswahl; nach dem Schließen
werden sie ignoriert. Fehler lassen das Original weiterhin zum externen Öffnen
und Exportieren verfügbar.

## Unterstützte Inhalte und Grenzen

- **EML:** RFC-/MIME-Kopfdaten, kodierte Betreffzeilen und Anzeigenamen,
  Adressgruppen, UTF-8 und weitere Zeichensätze, Base64 und Quoted-Printable,
  Text-/HTML-Nachrichten und Anhangsangaben einschließlich eingebetteter E-Mails.
- **MSG:** Outlook-CFB-Container, Unicode-/ANSI-MAPI-Textfelder, SMTP-Adressen
  und vorhandene Ersatzadressen, Empfängertypen, Sende-/Empfangsdatum als FILETIME,
  Klartext oder HTML sowie Dateinamen, Typ und Größe von Anhängen.
- Nachrichtentexte erscheinen ausschließlich als Text. HTML, Skripte, externe
  Bilder, eingebettete Objekte und Anhänge werden nicht ausgeführt oder geladen.
  Die Originalnachricht enthält weiterhin alle eingebetteten Dateien.
- Ausschließlich als RTF gespeicherter MSG-Text sowie verschlüsselte/geschützte
  Inhalte werden nicht entschlüsselt oder in der Anwendung als Nachrichtentext
  dargestellt. Die Ansicht zeigt fehlenden Text und den Weg zum Mailprogramm an.
- Vorschau bis 32 MB Quelldatei, 2 MB Nachrichtentext, 500 Anhänge beziehungsweise
  Adressen je Empfängergruppe. MSG-Textstreams sind auf 8 MB begrenzt, einzelne Kopfdaten auf 64 KB
  und gesammelte Empfänger-/Anhangsangaben auf 2 MB. Die Containerstruktur ist auf 10.000 Einträge begrenzt. Größere Archive können weiterhin über
  die bestehende Dateiverwaltung verwaltet und extern geöffnet werden.

## Unabhängiger Modulvertrag

`core-lib::email::lesen(dateiname, bytes)` liefert `EmailInhalt` mit den Typen
`MailAdresse` und `MailAnhang`. Der MIME-Parser und die MAPI-/CFB-Interpretation
bleiben intern. Verbraucher erhalten keine Parserobjekte, Containerpfade oder
Dateisystemzugriffe. `CoreServices::archivierte_email` lädt die konkrete,
hashgeprüfte DMS-Version und ergänzt deren Dokumentmetadaten.

Tauri `email_laden` und HTTP `GET /dokumente/{id}/email/{version}` verwenden
denselben Service. HTTP-Aufrufe unterliegen der bestehenden Projektzugriffsprüfung.
Das Lesen ist auch mit Leserechten möglich. TypeScript-Inhaltstypen werden aus
Rust generiert. Es gibt keinen neuen Speicher-/Migrationsablauf und keine
Abhängigkeit des Kategorienmoduls vom E-Mail-Parser.

Neue direkte Bibliotheken: `mail-parser` (Apache-2.0/MIT), `cfb` (MIT) und
`encoding_rs` ((Apache-2.0/MIT) und BSD-3-Clause). Der Lockfile-Stand ist enthalten;
die vorhandene Lizenz-CI prüft diese Abhängigkeiten mit.

## Ergänzungen an Kategorien

**Dokumentangaben:** Kategorie direkt im vorhandenen Dialog bearbeiten und mit
„Speichern“ sichern. Die Kategorie gehört zum Dokument, weitere Angaben zum
konkreten Dokumentstand. Beide besitzen eigene Commit-Grenzen. Bei kombinierten
Änderungen wird zuerst die Kategorie gespeichert. Falls anschließend die weiteren
Angaben scheitern, bleibt „Kategorie gespeichert“ sichtbar und deren erneute
Speicherung wird beim Wiederholen ausgelassen. Fehlgeschlagene Eingaben bleiben
erhalten. Eine alleinige Kategorieänderung erzeugt keine leeren Zusatzangaben.

**Übersicht:** Die Auswahlliste zeigt die Dokumentanzahl je Kategorie. Bei einer
Hauptkategorie werden ihre Unterkategorien mitgezählt. Unterschiedliche
Dateiversionen zählen als ein Dokument. Schnellfilter zeigen alle Unterlagen
oder die noch nicht zugeordneten Unterlagen. Die Anzahlen beziehen sich auf die
gesamte Projektakte, unabhängig von der Dateisuche.

**Einreichungsvergleich:** Aktuelle und zuletzt eingereichte Kategorie stehen
nebeneinander. Ein Hinweis unterscheidet Änderungen an Kategorie-ID, Name oder
Hauptkategorie. Der Vergleich von Dateiversion und Dateiinhalt bleibt unabhängig
davon. Ein ausdrücklich erfasstes „Ohne Kategorie“ unterscheidet sich von einer
fehlenden historischen Angabe älterer Pakete.

## Prüfungen

Unit-Tests prüfen MIME-Kodierungen, HTML-Text, Adressen, Anhänge und MSG-Felder
einschließlich reservierter Bytes, Unicode, ANSI, separater HTML-Codepage,
beschädigter Daten und RTF-Hinweis. Storage-Tests prüfen Originalerhalt,
Versionswahl, unveränderte Historie, Fehler und Kategorienwechsel bei gleicher
eingereichter Datei. HTTP-Tests prüfen Lesedaten und verweigerte Fremdzugriffe.

`apps/desktop/tests/emails.py` verwendet tatsächlich von Core gelesene Inhalte
mit ersetztem Tauri-Transport. Er prüft EML/MSG-Anzeige, Versionswechsel,
veraltete Antworten, escaped Text, fehlende externe Bildabrufe, Fehlererholung,
Kategorieänderungen in den Dokumentangaben, Kategorieanzahlen und historischen
Vergleich. Testdaten entstehen aus `storage/tests/emails.rs` mit gesetztem
`BAUPROJEKT_EMAIL_ARTEFAKTE`. Die neuen Abläufe sind in die UI-CI aufgenommen.
