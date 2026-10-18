# Drucken und PDF-Ausgabe

[Strukturierte Behördenausgabe, Briefbogen und Beispiel-PDF](behoerdenausgabe/README.md)
beschreibt die tabellarischen Vorlagen und die einstellbare Gestaltung.

Im Projekt unter **Unterlagen → Drucken / PDF-Ausgabe** stehen gespeicherte
Projektdateien und Fachberichte gemeinsam zur Auswahl. **Alle verfügbaren
auswählen** stellt einen Stapel zusammen. Die Liste darunter erlaubt Umordnen,
bei PDFs zusätzlich die Auswahl einzelner Seiten (`1, 3-5`). Mit nur einer
Unterlage und ausgeschaltetem Anschreiben/Inhaltsverzeichnis entsteht eine
Einzelausgabe. Die Vorschau, die gespeicherte PDF und der Druck verwenden genau
dieselbe erzeugte Ausgabe; Änderungen an der Auswahl erfordern erneutes Erstellen.

Anschreiben mit Absender, Empfänger und mehrzeiligem Text sowie ein
Inhaltsverzeichnis mit tatsächlichen Start-/Endseiten sind optional. Der Absender
wird aus den Büroangaben vorbelegt. Anschreiben und Inhaltsverzeichnis können
mehrere Seiten umfassen. Formate und Ausrichtung der PDF-Quellseiten bleiben in
der PDF erhalten. Fotos (PNG/JPEG) werden mit korrigierter EXIF-Ausrichtung auf
einer passenden A4-Seite eingepasst. EML/MSG werden über das bestehende Mailmodul
als Klartext mit Kopf und Anhangsliste ausgegeben; Anhänge sind separat auszuwählen.

**Drucken …** öffnet den Systemdruckdialog mit allen Seiten. Der Dialog bestimmt
Drucker, Exemplare, Duplexdruck und die vom Gerät unterstützten Papierformate.
Die Druckdarstellung rastert jede Seite mit bis zu 144 dpi / maximal 6 MP; für
maßstabsgerechte große Pläne empfiehlt sich die gespeicherte PDF im PDF-Programm.
Word, Excel, DWG/DXF und andere nicht unterstützte Originalformate werden sichtbar
als nicht direkt druckbar markiert. Über **Projektdateien → Öffnen** können sie im
zugeordneten Programm gedruckt oder als PDF erzeugt und wieder importiert werden.
Originale können weiterhin einzeln mit exakter Version exportiert werden.

Die Formularmaske hat zusätzlich **Aktuelle Eingaben drucken …**. Das ist erst
möglich, wenn die Live-Vorschau die aktuellen Eingaben vollständig darstellt.
Dieser Druck speichert keine neue Formularversion. Für die spätere gemeinsame
Ausgabe das Formular zunächst im Projekt speichern.

## Berechnungen und Erweiterbarkeit

Baugrundstücksermittlung, gespeicherte Baugrenzenprüfung und Raumbücher besitzen
Fachadapter. GRZ, GFZ, Geschossigkeit, BMZ und Abstandsflächen nutzen gespeicherte
Nachweisstände inklusive Eingaben, archivierter Ergebnisse und Regelstände. Vor
der Ausgabe deshalb im Fachbereich **Nachweisstand speichern** verwenden oder dort **Speichern und drucken …** wählen.
Ungespeicherte Eingaben werden nicht unbemerkt als verbindliches Ergebnis gedruckt.
Veraltete, unvollständige und nicht freigegebene Stände werden im Bericht so
bezeichnet; eine vorhandene Freigabe wird mit Datum ausgegeben. Die Ausgabe
verleiht keine fachliche Freigabe und bestätigt keine Einreichung.

Der Core-Vertrag liegt in `core-lib::druckausgabe`: `Druckkatalog` liefert
verfügbare Quellen, `Druckauftrag` die Reihenfolge und optionale Vorblätter,
`Druckergebnis` die PDF sowie Seitenpositionen. `Druckquelle` referenziert exakte
Dokumentversionen, unveränderliche Nachweis-IDs, konkrete Raumbuchrevisionen oder
die mit `projekt_version` geschützten aktuellen Fachgrundlagen. Fachadapter liefern
typisierte Absätze und Tabellen mit Spaltenbreiten; die PDF-Engine übernimmt Unicode, Umbruch, Paginierung,
Formulardarstellung, Seitenauswahl und Zusammenführung. Neue Berechnungen ergänzen
ihren Adapter, ohne vorhandene Berechnungsregeln oder PDF-Formulare zu ändern.
Desktop- und HTTP-Adapter führen dieselbe Core-Operation aus.

HTTP: `GET /api/v1/projekte/{id}/druckkatalog`,
`POST /api/v1/projekte/{id}/druckausgabe` (JSON-Druckauftrag, Antwort PDF).
Beide benötigen Projektleserechte; der POST ist ausdrücklich eine lesende Ausgabe.
Projektfremde Quellen werden vor dem Laden abgelehnt, Daten werden hashgeprüft.

Die Ausgabe schreibt weder DMS-Versionen noch Kategorien, Einreichungen oder
Projektstände. PDF-Formularfelder werden anhand vorhandener Darstellungen als
Druckkopie eingefroren, sodass identische Feldnamen verschiedener Vorlagen keine
Werte vermischen. Gefüllte Felder ohne druckbare Darstellung führen zu einem
Fehler mit Handlungsanweisung; das erneute Befüllen mit der Formularfunktion
erzeugt die fehlende Darstellung (siehe [PDF-Ablage](pdf-ablage/README.md)). Druckkopien übernehmen keine Gültigkeit digitaler
Originalsignaturen. Grenzen: 100 Unterlagen, 300 Seiten, 64 MB je Quelldatei,
128 MB Gesamtquelldaten/Ausgabedatei, 25 MP je Foto. Fehler oder Abbruch beim
Speichern erhalten Auswahl und Vorschau. Es gibt keine Datenmigration.
