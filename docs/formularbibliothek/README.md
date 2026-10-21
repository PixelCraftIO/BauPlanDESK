# Zentrale PDF-Formularbibliothek

**Formularbibliothek** ist ein eigener Arbeitsbereich, der auch ohne geöffnetes
Projekt verfügbar ist. Der Formularordner ist von Projektdateien und von den
JSON-Dateien für automatische Feldzuordnungen getrennt.

## Ordner und Vorlagen

- Als Administrator unter **Einstellungen → Speicherorte → PDF-Formularvorlagen**
  **Formularordner auswählen …** verwenden, beispielsweise
  `C:\BauPlanIO\Formulare` oder eine eingebundene Freigabe
  `\\Firmenserver\BauPlanIO\Formulare`.
- Ohne eigene Auswahl werden im Einzelplatzmodus `Formularvorlagen` im
  App-Datenordner und bei einer NAS-/Dateisystem-Netzwerkkonfiguration `Formulare`
  unter deren Ablageroot verwendet. Bei S3 bleibt die Bibliothek ein separat
  wählbarer Dateisystemordner; sie wird nicht automatisch im Bucket abgelegt.
- Die Auswahl wird pro Arbeitsplatz in `formularbibliothek.json` im
  App-Konfigurationsordner gespeichert und nach Neustart wieder geladen.
  Für eine gemeinsame Bibliothek verbinden alle Arbeitsplätze denselben Ordner.
- Ein Ordnerwechsel verbindet den neuen Ordner. Bisherige Vorlagen bleiben am
  alten Ort; es findet keine automatische Verschiebung oder Kopie statt.
- **PDF-Vorlage importieren** kopiert die gewählte PDF in die Bibliothek.
  Vorhandene gleichnamige Dateien werden erhalten; die neue Datei bekommt dann
  eine eindeutige Ergänzung. Vorlagen können auch direkt in den Ordner gelegt
  werden; **Aktualisieren** liest sie erneut ein. Unterordner werden nicht durchsucht.

Die Bibliothek arbeitet mit regulären PDF-Dateien bis 100 MB. Import und
Projektübernahme erfordern die Rolle Bearbeitung oder höher. Dateisystemrechte
auf dem Server gelten zusätzlich. Nur Administratoren können den Ordner über
die Anwendung wechseln. Das gemeinsame Verzeichnis sollte ausschließlich
Formularvorlagen enthalten; eine nicht lesbare/zu große Datei wird als Fehler
angezeigt und nicht still ignoriert.

## Im Projekt verwenden

1. Projekt öffnen und **Unterlagen → Formulare** wählen.
2. Eine ausfüllbare Bibliotheksvorlage auswählen und die Maske ausfüllen.
3. **Im Projekt speichern** erzeugt genau ein ausgefülltes Projektdokument und
   legt es automatisch im eingestellten Projektordner ab. Es folgt kein Exportdialog.
4. Unter **Projektdateien → Formular bearbeiten** wird dasselbe Dokument als neue
   Version gespeichert.

Vor dem Speichern wird keine Projektkopie angelegt. Abbrechen erzeugt keine Datei.
Die gemeinsame Vorlage bleibt unverändert. Wurde sie nach ihrer Auswahl extern
geändert, wird der Vorgang mit einer Prüfsummenfehlermeldung abgelehnt.

PDFs ohne bearbeitbare Felder zuerst in der globalen Bibliothek über den
Feldeditor vorbereiten. Der Editor erzeugt eine zusätzliche ausfüllbare Vorlage
in der Bibliothek, unabhängig von Projekten. Eigene Projekt-PDFs lassen sich
über **Projektdateien → Weitere … → In Formularbibliothek ablegen** bereitstellen.
Bereits vorhandene Eingaben werden dabei kopiert; für gemeinsame Vorlagen eine
leere Fassung verwenden.

Die globale Bibliothek hat keine Projektzuordnung mehr. Die vollständige aktuelle
Bedienung einschließlich Screenshots ist unter
[Klare Projektstruktur](../klare-projektstruktur/README.md) dokumentiert.

## Prüfung

Die aktuelle UI-Suite `apps/desktop/tests/formularbibliothek.py` prüft Import-Abbruch,
projektunabhängigen Feldeditor, PDF.js-Darstellung, Textfelder und Checkboxen sowie
Fehler- und Abbruchverhalten. `klare-projektstruktur.py` ergänzt den Ausfüllablauf im
Projekt. Reale Vorlagen-/DMS-Verträge prüfen die Storage-Integrationstests
`formularbibliothek` und `projekt_formulare`.

[Aktueller Bibliothekseditor](../klare-projektstruktur/pdf/01-bibliothekseditor.png) ·
[Prüfprotokoll](../klare-projektstruktur/pdf/pruefung.json)
