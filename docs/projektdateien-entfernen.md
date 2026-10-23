# Projektdateien entfernen und im Explorer anzeigen

`CoreServices::dokument_entfernen(id, basis, benutzer)` entfernt alle Versionen
einer Dokumentidentitaet aus den aktiven Projektlisten. Bearbeitungsrecht und
DokumentAblegen sind erforderlich. Gesperrte Dokumente werden abgewiesen;
eine abweichende aktuelle Version erzeugt einen Versionskonflikt.

Die additive Tabelle `entfernte_dokumente` markiert die Entfernung. Bestehende
Versionen, Inhalte, Formular- und Einreichungsreferenzen bleiben erhalten.
`latest`, `history` und konkrete Versionsabrufe bleiben lesbar;
`active_for_project` und `latest_for_project` liefern nur aktive Dokumente.
`list_for_project` erhaelt den vollstaendigen Bestand fuer historische Auswertungen.
Dies ist keine physische Datenbereinigung. Bereits exportierte Dateien und
Arbeitskopien bleiben erhalten. Neue Importe erhalten eine neue Identitaet.
SQLite und PostgreSQL verwenden denselben Vertrag; vorhandene Projekte erhalten
bei der Migration keine Entfernungsmarkierung.

Desktop: `dokument_entfernen`; Server: POST
`/dokumente/{id}/entfernen/{basis}`. Die Desktop-Oberflaeche bestaetigt die
Entfernung und informiert ueber erhaltene Versionsbezuege und Ausgaben.

`dokument_im_explorer(id, version)` verwendet ausschliesslich die serverseitig
ermittelte Projektordnerausgabe, keine frei uebergebenen Shell-Pfade. Falls noetig,
wird die Ausgabe erneut erstellt (mit den bestehenden Ausgaberechten). Windows
markiert die konkrete Datei im Explorer; andere Systeme oeffnen ihren Ordner.
Ausgabefehler werden sichtbar gemeldet.
