# PDF-Vorlagen, eigene Eingabefelder und Dateiablage

## Formulare verwenden

1. Projekt öffnen und **Unterlagen → Formulare** wählen.
2. Eine ausfüllbare Vorlage auswählen. Die Eingabemaske verwendet Textfelder,
   Checkboxen sowie unterstützte Options- und Auswahlfelder des PDFs.
3. Werte eingeben und **Im Projekt speichern** wählen. Das Ergebnis wird einmal
   in der Projektakte und automatisch im konfigurierten Projektordner gespeichert.
   Es gibt keinen zusätzlichen Speicherdialog und keine leere Projektkopie.
4. **Projektdateien → Formular bearbeiten** erzeugt eine neue Version desselben
   Dokuments. Ein manueller Export ist unter **Weitere …** verfügbar.

## Eigene Platzhalter auf einer PDF-Vorlage

PDF in der globalen **Formularbibliothek** importieren und dort den Feldeditor
öffnen. Textfelder oder Checkboxen hinzufügen, Namen und Beschriftung vergeben und
auf der PDF-Seite positionieren. Position und Größe sind auch in Millimetern ab
der linken oberen Ecke einstellbar.

Beispiele: `Vorname`, `<Vorname>` oder `< Platzhalter1 >`. Die äußeren Klammern
werden entfernt. Die spätere Maske enthält beispielsweise **Vorname: Daniel**.
Der Editor legt eine zusätzliche ausfüllbare Vorlage in der Bibliothek an; er
benötigt kein Projekt. Hintergrund und bestehende Felder bleiben erhalten.

Für verfügbare automatische Vorschläge kann ein Feld beispielsweise
`bauherr.vorname` oder `projekt.name` heißen. Solche Namen werden als echte
hierarchische PDF-Felder angelegt. Die Zuordnung bleibt beim bestehenden
Projektkontext-/Vorlagenmodul; der PDF-Parser kennt keine Grundstücksberechnung.
Eigene Textfelder verwenden eine eingebettete DejaVu-Sans-Schrift, sodass etwa
Umlaute, Ł und € auch in der exportierten Darstellung erhalten bleiben.

Auch Text- und Auswahlfelder fremder PDFs (amtliche Landesformulare) erhalten
beim Befüllen eine eigene Darstellung mit dieser Schrift. Feldgröße,
Schriftgröße (`/DA`, `0` = automatisch verkleinern), Textfarbe und Ausrichtung
(`/Q`) werden übernommen, mehrzeilige Felder umbrochen. Bereits befüllte
Felder ohne Darstellung (etwa aus einem anderen Programm) werden beim nächsten
Speichern nachgezogen, sodass Druckausgabe und Abgabe keine leeren Felder
zeigen. Ein Wert, der nicht ins Feld passt oder Zeichen außerhalb der Schrift
enthält, wird mit dem Feldnamen abgewiesen. `NeedAppearances` wird nur noch
gesetzt (wenn eine Darstellung nicht erzeugt werden kann), nie von `true` auf
`false` zurückgesetzt. Darstellungen und Schriftkopien früherer Befüllungen
werden beim Ersetzen entfernt.

Platzhalternamen bezeichnen PDF-Eingabefelder. Bereits gedruckte Zeichenfolgen
wie `<Vorname>` in einem PDF-Seiteninhalt oder einer gescannten Bildseite werden
nicht durch eine Textersetzung gesucht oder entfernt. Bei solchen Dokumenten
wird ein Feld an der gewünschten freien Stelle platziert.

## Speicherort und Projektordner

Unter **Einstellungen → Speicherorte → Verwaltete Dateiablage** zeigt die Anwendung den
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
Metadaten für Versionen und Prüfsummen. Eine lesbare Ausgabe erfolgt automatisch am eingestellten Projektordner.
Zusätzlich ist sie unter
**Projektdateien → Projektordner exportieren …** verfügbar, optional mit Dokumenthistorie:

```text
C:\BauPlanIO\
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

Die aktuellen Prüfungen und Aufnahmen sind unter
[Klare Projektstruktur](../klare-projektstruktur/README.md#prüfung) dokumentiert.
Der Einstieg `apps/desktop/tests/pdf-ablage.py` führt Bibliothekseditor und den
neuen Formularablauf im Projekt aus. `cargo test -p storage --test projekt_formulare`
prüft die neuen Core-/DMS-Verträge mit echten PDF-Dateien und SQLite.

Die hier vorhandenen älteren Aufnahmen und die Architektur-Notiz beschreiben den
früheren Ablauf mit abgeleiteten Dokumenten. Diese Core-APIs bleiben verfügbar;
die aktuelle Desktop-Oberfläche verwendet die oben beschriebenen Operationen.
