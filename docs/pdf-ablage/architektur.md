# Verträge der PDF- und Ablageerweiterung

Die Erweiterung folgt [den Entwicklungsregeln](../entwicklungsregeln.md). Es gibt
keine neue Berechnungslogik im Frontend und keine Änderung am Datenbankschema.

| Liefermodul | Eingaben | Ergebnis / Verbraucher |
| --- | --- | --- |
| `core-lib::formular::editor` | PDF-Bytes und `EigenesPdfFeld`: Name, Typ, Seite ab 1, Rechteck in mm, Schriftgröße in pt | Standard-PDF mit zusätzlichen AcroForm-Feldern; bestehender Feldleser und Ausfüllmodul |
| `core-lib::formular::pdf` | PDF und typisierte `Feldwert`-Zuordnung | Neues ausgefülltes PDF; `CoreServices` legt es als neues DMS-Dokument ab |
| `core-lib::formular::schrift` | Internes Dokument, Text und Feldgeometrie | Eingebettete Schrift und Darstellungsobjekte; außerhalb des PDF-Moduls nicht sichtbar |
| `CoreServices` | Dokument-ID, erwartete Version, Definitionen bzw. Werte und Benutzer | Berechtigungsprüfung, neue Dokumentreferenz und Herkunft im Protokoll; Desktop und HTTP-Server |
| `core-lib::projektablage` | Projekt und ausdrücklich gewählter Dokumentstand | `ProjektordnerPlan` mit Identitäten, Versionen, Prüfsummen und sicheren relativen Pfaden; Dateisystemadapter |
| `storage::WechselbareAblage` | Absoluter neuer Root und Konfigurations-Commit | Geprüfter Wechsel bei unverändertem `Storage`-Vertrag; alle Core-Verbraucher arbeiten weiter |
| `storage::projekt_export` | Zielroot, relativer Pfad, Bytes und erwarteter Hash | Unveränderliche Exportdatei; keine Änderung an DMS-Versionen |

Die PDF-Vorschau ist ein UI-Adapter. PDF.js zeichnet Seiten und optionale
Definitionsmarkierungen. Sie entscheidet nicht, ob eine Definition fachlich oder
geometrisch gültig ist; diese Validierung erfolgt im Core. Schriftparser und
Darstellung bleiben interne Details und können unabhängig ersetzt werden.

Eigene Vorlagen werden im PDF selbst gespeichert. Damit benötigen sie kein
neues, nur diesem Client bekanntes Zusatzformat in der Datenbank. Eine Vorlage
kann mit üblichen PDF-Programmen verwendet und erneut importiert werden. Äußere
Platzhalterklammern werden normalisiert; hierarchische Namen erhalten echte
Eltern-/Kindfelder statt ungültiger Punkte in einem partiellen PDF-Feldnamen.

Die bisherige `formular_ausfuellen`-Core-Methode bleibt als kompatible Hülle
vorhanden. `formular_ausfuellen_version` ergänzt eine optionale Basisversion. Die
Desktop-Maske übergibt den analysierten Dokumentstand; beim HTTP-Aufruf kann
`?basis=<version>` hinzugefügt werden, ohne die bestehende Werte-JSON zu ändern.
Eine fremde neue Version überschreibt weder den Entwurf noch das Original.

Neue öffentliche Übergaben:

- Vorlage erstellen: `formular_vorlage_erstellen`, im Server
  `POST /api/dokumente/{id}/vorlage` mit `basis` und `felder`.
- Versionierte Vorschau: Desktop `pdf_vorschau` liefert Dokumentreferenz,
  PDF-Transportdaten, Seitenbeschreibung und Feldschema.
- Backendunabhängiger Projektordnerstand: `CoreServices::projektordner_plan`,
  im Server `GET /api/projekte/{id}/projektordner?historie=true|false`.
- Desktop-Ablageadapter: `ablage_status`, `ablage_wechseln`,
  `projektordner_exportieren`. Lokale Pfadauswahl und Dateioperationen verbleiben
  im Infrastrukturadapter; die Auswahl des Dokumentstands liegt im Core.

Beim lokalen Ablagewechsel hält der Storage-Adapter eine exklusive Sperre über
Kopie, Prüfsummenprüfung, Konfigurations-Commit und Umschaltung. Dateioperationen
anderer Features warten auf diese Grenze. Schlägt ein Schritt fehl, bleibt die
bisherige Ablage aktiv. Die Datenbank und alle bisherigen Dokument-IDs und
Storage-Schlüssel bleiben unverändert. Frühere Ablageverzeichnisse werden nicht
entfernt. Der Netzwerkmodus verwendet weiter seine gemeinsame Storage-/Cache-
Konfiguration und wird nicht pro Client umgestellt.

Exporte sind eine bewusst getrennte Lesesicht, kein alternatives Repository.
Geänderte Exportkopien dürfen die Anwendung nicht still beeinflussen. So lässt
sich später etwa eine andere Ordnerdarstellung oder eine serverseitige Ausgabe
entwickeln, ohne PDF-Parser oder Berechnungen anzupassen.

PDF.js (Apache-2.0) und ttf-parser (MIT/Apache-2.0) sind gekapselte Abhängigkeiten.
Die unveränderte eingebettete DejaVu-Sans-Schrift enthält ihren Lizenzhinweis
unter `crates/core-lib/assets/DejaVuSans-LICENSE.txt`. Vorschau-Fonts und CMaps
werden bei `predev`/`prebuild` aus der festgeschriebenen npm-Abhängigkeit kopiert
und mit der Anwendung ausgeliefert; die Vorschau benötigt keine externen URLs.
