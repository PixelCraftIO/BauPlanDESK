# PDF-Vorlagen, eigene Eingabefelder und Dateiablage

## Formulare verwenden

1. Projekt öffnen und unter **Dokumente → Dokumente importieren** die offizielle
   oder eigene PDF-Datei hinzufügen.
2. Bei vorhandenen PDF-Formularfeldern **Formular ausfüllen** wählen. Textfelder,
   Checkboxen, Options- und Auswahlfelder werden als Eingabemaske angeboten.
   Die gespeicherte PDF-Seite wird daneben als Vorschau angezeigt.
3. Werte eingeben und **Ausfüllen und ablegen** wählen. Das Ergebnis wird als
   eigenes Dokument gespeichert; anschließend öffnet sich die Dateiauswahl für
   den PDF-Export. Ein abgebrochener Export löscht das bereits abgelegte Ergebnis
   nicht. Das importierte Original bleibt unverändert.

## Eigene Platzhalter auf einer PDF-Vorlage

**Eigene Felder / Vorlage** öffnet den Vorlageneditor auch für PDFs ohne
Formularfelder. Textfelder oder Checkboxen hinzufügen, Namen und Beschriftung
vergeben, dann durch Anklicken der PDF-Seite positionieren. Position und Größe
lassen sich auch in Millimetern ab der linken oberen Ecke eingeben.

Beispiele: `Vorname`, `<Vorname>` oder `< Platzhalter1 >`. Die äußeren Klammern
werden beim Erstellen entfernt. Die resultierende Maske kann dann etwa
**Vorname: Daniel** oder **Platzhalter1: Irgendein Text** enthalten.

**Vorlage erstellen** legt eine neue PDF mit standardkonformen AcroForm-Feldern
ab. Ihre Felder werden danach von derselben Ausfüllmaske erkannt wie native
Formularfelder. Hintergrund, Seiteninhalt und bestehende Felder bleiben erhalten.
Die eigene Vorlage bleibt als Dokument gespeichert und kann erneut ausgefüllt,
exportiert oder über **In Formularbibliothek ablegen** für andere Projekte
bereitgestellt werden. Die [zentrale Formularbibliothek](../formularbibliothek/README.md)
hat einen eigenen, projektunabhängigen Speicherort.

Für verfügbare automatische Vorschläge kann ein Feld beispielsweise
`bauherr.vorname` oder `projekt.name` heißen. Solche Namen werden als echte
hierarchische PDF-Felder angelegt. Die Zuordnung bleibt beim bestehenden
Projektkontext-/Vorlagenmodul; der PDF-Parser kennt keine Grundstücksberechnung.
Eigene Textfelder verwenden eine eingebettete DejaVu-Sans-Schrift, sodass etwa
Umlaute, Ł und € auch in der exportierten Darstellung erhalten bleiben.

Platzhalternamen bezeichnen PDF-Eingabefelder. Bereits gedruckte Zeichenfolgen
wie `<Vorname>` in einem PDF-Seiteninhalt oder einer gescannten Bildseite werden
nicht durch eine Textersetzung gesucht oder entfernt. Bei solchen Dokumenten
wird ein Feld an der gewünschten freien Stelle platziert.

## Speicherort und Projektordner

Unter **Verwaltung → Dateiablage und Projektordner** zeigt die Anwendung den
aktuellen Dateispeicher und den Ort der Projektdatenbank an. Im Einzelplatzmodus
kann ein Administrator über **Speicherort auswählen** einen anderen lokalen
Ordner oder ein eingebundenes Laufwerk bestimmen.

Der Wechsel kopiert alle verwalteten Dateiversionen mit Hash-Prüfung. Erst nach
vollständiger Kopie und erfolgreichem Speichern der Einstellung wird umgeschaltet.
Die bisherige Ablage wird nicht gelöscht. Eine andere bestehende DMS-Ablage wird
nicht unbemerkt mit der eigenen vermischt. Die Auswahl wird arbeitsplatzbezogen
in `dateiablage.json` im Konfigurationsverzeichnis gespeichert und beim Neustart
wieder verwendet. Die Projektdatenbank bleibt an ihrem bisherigen Ort; für eine
vollständige Sicherung sind Datenbank und Dateispeicher zusammen zu berücksichtigen.

Der verwaltete DMS-Speicher verwendet weiterhin unveränderliche Schlüssel und
Metadaten für Versionen und Prüfsummen. Eine lesbare Ausgabe ist zusätzlich unter
**Dokumente → Projektordner exportieren** verfügbar, optional mit Dokumenthistorie:

```text
C:\BauProjektIO\
  Projektname--Projekt-ID\
    Dokumente\
      Dokument-ID\v1-Formular.pdf
    Pläne\
      Dokument-ID\v2-Lageplan.dxf
    Projektstand-Export-ID.json
```

Aktuell werden DXF-/DWG-Dateien unter `Pläne` und andere Dokumente einschließlich
PDFs unter `Dokumente` ausgegeben. Projekt-ID, Dokument-ID und Version verhindern
Namenskollisionen. Ordner werden angelegt, sobald entsprechende Dateien ausgegeben
werden. Wiederholte Exporte verwenden unveränderte Dateien erneut; außerhalb der
Anwendung geänderte Exportdateien werden nicht überschrieben. Exportkopien sind
keine zweite schreibende Datenquelle: Änderungen daran werden nur durch einen
bewussten Dokumentimport als neue Version in der Anwendung übernommen.

## Gemeinsame Nutzung im Büro

Für mehrere Arbeitsplätze gibt es bereits den Netzwerkmodus mit gemeinsamer
PostgreSQL-Datenbank und NAS-/Serverablage, alternativ S3. Alle Clients benötigen
dieselbe fachliche Datenquelle und dieselbe konfigurierte Dateiablage. Ein
Serverordner allein synchronisiert keine Projekte und ersetzt keine gemeinsame
Datenbank. Eine SQLite-Datei auf einer Freigabe ist kein Mehrbenutzer-Setup.

Die bestehende Büro-Konfiguration wird zentral in `config.json` eingerichtet,
beispielsweise mit `modus: netzwerk`, `database_url` und einem NAS-Storage. Zugangsdaten
werden nicht in der neuen Oberfläche angezeigt. Im Netzwerkmodus ist der individuelle
Ablagewechsel absichtlich gesperrt: Sonst würden Clients dieselben Dokumentversionen
in unterschiedlichen Ordnern suchen. Bestehende Rechte, Sperren, Optimistic Locking
und Cache-Hash-Prüfungen bleiben wirksam. Ein grafischer Server-Einrichtungsassistent
und die Bereitstellung eines Firmenservers sind nicht Teil dieser Erweiterung.

## Modulverträge und Grenzen

[Architektur und öffentliche Übergaben](architektur.md) dokumentiert die Trennung
zwischen PDF-Editor, Ausfüllfunktion, DMS und Ablageadaptern.

Eigene Felder unterstützen derzeit einzeiligen Text und Checkboxen, maximal 200
neue Felder je Erstellung. Doppelte Namen und Positionen außerhalb des sichtbaren
Seitenbereichs werden abgewiesen. Neue Felder benötigen Seiten ohne gespeicherte
Rotation und mit normalem PDF-Maßstab; vorhandene Formularfelder auf gedrehten Seiten
können weiterhin verwendet werden. Zu lange Texte werden nicht still abgeschnitten.
Nicht von der eingebetteten Schrift unterstützte Zeichen ergeben eine Fehlermeldung.
Verschlüsselte PDFs, XFA-Bearbeitung und Änderungen an bereits digital signierten
PDFs werden nicht unterstützt; dafür ist eine passende Standard-/unsignierte Vorlage
nötig. Leere Signaturfelder bleiben dagegen in der Vorlage erhalten.

Projektordner-Exporte legen neue Dateien exklusiv an. Wo das Dateisystem es
unterstützt, werden vollständige Dateien atomar veröffentlicht; auf Laufwerken
oder Freigaben ohne Hardlinks wird ohne Überschreiben geschrieben. Bei einem
Schreibfehler werden neu angefangene Dateien entfernt. Ein Projektstand-Manifest
wird erst nach erfolgreicher Ausgabe der gewählten Dateien erzeugt.

## Prüfung und Aufnahmen

- Svelte-Check, Produktionsbuild, Desktop-/Server-Build und Clippy mit
  `-D warnings` erfolgreich; npm-Lizenzprüfung ohne Verstöße.
- Modulprüfungen: Feldnamen, Checkboxen, Unicode-Darstellung, Seitenmaße und CropBox,
  ungültige Positionen, rotierte Seiten, XFA und Signaturbehandlung.
- SQLite-/Dateisystem-Integration: importiertes Original → eigene Vorlage →
  automatische Analyse → ausgefülltes Dokument → Projektordner → Ablagewechsel →
  erneut lesbare Dokumentversion. Originalbytes und PDF-Seiteninhalt bleiben erhalten.
- UI-Tests mit echter PDF.js-Vorschau und kontrolliertem Tauri-Testtransport:
  Felddefinition, Maskenerzeugung, Abbrechen, Speicherfehler, versionsgebundene
  Übergabe, Export sowie Auswahl und Fehler beim Ablagewechsel. Bestehende
  Projekttab-/Leserrechte-Regressionen weiterhin erfolgreich.
- Native Desktop-Probe in eigener Testinstallation ohne externe Netzwerkverbindung:
  PDF importiert, Textfeld positioniert und als Vorlage gespeichert, Vorname=Daniel
  eingegeben, abgeleitete PDF tatsächlich exportiert und den Feldwert wieder ausgelesen.

[Vorlageneditor](01-vorlageneditor.png) · [Maske](02-eingabemaske.png) ·
[Ablageeinstellungen](04-ablageeinstellungen.png) ·
[Native Vorschau](06-native-vorlageneditor.png) ·
[UI-Prüfprotokoll](ui-pruefung.json)
