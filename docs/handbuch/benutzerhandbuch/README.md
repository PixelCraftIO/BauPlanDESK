# BauPlanIO – Benutzerhandbuch

Bebildertes Handbuch für Architektinnen, Architekten, Entwurfsverfasser und
die Projektorganisation im Büro. Es erklärt die Oberfläche, alle Funktionen
und die Standardabläufe vom ersten Projekt bis zur Bauausführung.

Alle Bildschirmfotos stammen aus der echten Anwendung mit dem mitgelieferten
Beispielprojekt **„Neubau Einfamilienhaus Mustermann“** (Nordrhein-Westfalen).
Sie können jeden Schritt dort nachvollziehen: **Hilfe → Beispielprojekt öffnen**
(siehe [Kapitel 1](01-einstieg.md#beispielprojekt-öffnen)).

**Stand:** 28.10.2026 · Programmstand `main` f5c4e8d (inkl. neuem Zeicheneditor,
Abgleich von Hand- und Zeichnungswerten und IFC-Modellen).

## Inhalt

| Nr. | Kapitel | Worum es geht |
| --- | --- | --- |
| 1 | [Einstieg](01-einstieg.md) | Was BauPlanIO leistet, Begriffe, Beispielprojekt |
| 2 | [Oberfläche und Bedienung](02-oberflaeche.md) | Arbeitsplatz, Projekttabs, Projektbaum, Speichern, Status |
| 3 | [Büro einrichten](03-einrichtung.md) | Einstellungen, Büroangaben, Benutzer, Kunden, Formularbibliothek |
| 4 | [Projekt anlegen und Grundlagen](04-projekt-grundlagen.md) | Neues Projekt, Stammdaten, Rechtsgrundlagen, B-Plan, Flurstücke |
| 5 | [Unterlagen, Aufmaß und Gebäudemodell](05-ermittlung.md) | Projektdateien, Zeicheneditor, IFC-Modelle (BIM), Messwerte, Gebäude, Raumbuch |
| 6 | [Berechnungen und Nachweise](06-nachweise.md) | Baugrundstück, GRZ/GFZ/BMZ, Fachberechnungen, Nachweisstände |
| 7 | [Formulare und Druckausgabe](07-formulare-druck.md) | PDF-Formulare ausfüllen, Berechnungsheft, Behördenausgabe |
| 8 | [Verfahren und Einreichung](08-verfahren.md) | Verfahrensakte, Abgabestände, Portal, XBau, Bescheid, Bauausführung |
| 9 | [Standard-Workflows](09-workflows.md) | Schritt-für-Schritt-Abläufe für die typischen Aufgaben |
| 10 | [Tastatur, Fragen und Begriffe](10-anhang.md) | Kurzbefehle, häufige Meldungen, Glossar, Abschlusscheckliste |

Das ganze Handbuch als eine Datei zum Drucken oder Weitergeben:
[Benutzerhandbuch.pdf](Benutzerhandbuch.pdf).

Wer zum ersten Mal mit BauPlanIO arbeitet, liest Kapitel 1 und 2 und folgt
dann dem Workflow [„Vom Grundstück zur Erstabgabe“](09-workflows.md#workflow-1-vom-grundstück-zur-erstabgabe).

> Die fachliche Verantwortung bleibt beim Planungsteam. Mitgelieferte
> Regelpakete und Beispielwerte sind keine geprüften Satzungen oder
> Rechtsauskünfte; Rechtsangaben sind bis zur fachlichen Abnahme durch das
> Büro als ungeprüft gekennzeichnet.

## Pflege dieses Handbuchs

Das Handbuch ist so aufgebaut, dass es nicht neu geschrieben, sondern nur
fortgeschrieben wird:

- **Ein Kapitel je Arbeitsbereich.** Eine neue Funktion wird in dem Kapitel
  ergänzt, zu dessen Bereich sie in der Projektakte gehört (Gliederung wie in
  der App: Übersicht, Bereich 1 Ermittlung, Bereich 2 Nachweis und Einreichung).
  Ein neuer Projektbereich bekommt einen eigenen Abschnitt `## <Titel wie in der
  Navigation>`.
- **Bilder neu erzeugen statt nachbearbeiten.** Alle Bilder entstehen mit
  `scripts/handbuch-bilder/aufnahmen.py` aus der laufenden App und dem
  Beispielprojekt (Anleitung in [scripts/handbuch-bilder/README.md](../../../scripts/handbuch-bilder/README.md)).
  Für eine neue Ansicht eine Szene in `SZENEN` ergänzen; der Dateiname ist der
  Bildname in `bilder/`. Nach einer Oberflächenänderung genügt ein erneuter Lauf,
  einzelne Szenen lassen sich gezielt wiederholen.
- **Bezeichnungen aus der Oberfläche übernehmen.** Knöpfe und Bereiche stehen
  in „Anführungszeichen“ genau so, wie sie in der App heißen. Wird ein Knopf
  umbenannt, nach dem alten Text im Ordner suchen und ersetzen.
- **Web-Handbuch:** Die Webseite (`web/`, bauplanio.de/handbuch) zeigt genau
  diese Dateien. Kapitel heißen `NN-name.md`, Bilder liegen in `bilder/`;
  `npm test` in `web/` prüft, dass alle Verweise und Bilder auflösbar sind.
- **Änderungen unten im Verlauf eintragen** (Datum, Programmstand, was neu ist).
- **PDF neu erzeugen.** Nach jeder Änderung `python3 scripts/benutzerhandbuch-pdf.py`
  ausführen (braucht pandoc und Playwright mit Chromium; ein anderes Chromium
  über `BAUPROJEKT_CHROMIUM=<Pfad>`). Das Skript setzt Titel, Inhalt und alle
  Kapitel mit Bildern zu `Benutzerhandbuch.pdf` zusammen und bricht ab, wenn
  ein Verweis oder Bild ins Leere zeigt.

Daneben gibt es die in die App eingebaute Hilfe (Taste F1, Quelle
`apps/desktop/src/lib/hilfe/handbuch.json`, Ausgaben unter
[docs/handbuch/](../README.md)). Sie ist ein kurzes Nachschlagewerk ohne Bilder
und wird beim Ergänzen neuer Module ebenfalls gepflegt.

## Änderungsverlauf

| Datum | Programmstand | Änderung |
| --- | --- | --- |
| 28.10.2026 | main 3c66d55 | Erstfassung mit 48 Bildschirmfotos aus dem Beispielprojekt. Berücksichtigt den neuen Zeicheneditor (Zeichnungs-Tabs, eigenes Fenster, eigene Zeichnungen, Werkzeugkiste), die Gliederung der Projektakte in zwei Bereiche und den Abgleich von Hand- und Zeichnungswerten. |
| 28.10.2026 | main 09df005 | PDF-Ausgabe `Benutzerhandbuch.pdf` (59 Seiten) und Skript `scripts/benutzerhandbuch-pdf.py`. |
| 28.10.2026 | main f5c4e8d | IFC-Modelle (BIM): neuer Abschnitt in Kapitel 5 mit 10 Bildschirmfotos (Modell öffnen, Ansicht, Prüfung, Nacharbeit, Auswertung, Übernahme, Abgleich, Export), Workflow 6, Hinweise in Kapitel 1 und 8, fünf neue Begriffe. Die Bildautomatik legt dafür das IFC-Testmodell in das Beispielprojekt (Szenen `ifc-modell-…`). |
| 04.11.2026 | main 6fb8eac | Alle Bilder auf den aktuellen Programmstand neu aufgenommen. Sieben neue Bilder zum Zeicheneditor in Kapitel 5 (Aufgaben, Werkzeugkiste, Eigenschaften und Verwendung, eigene Zeichnung, Konstruieren, Darstellung, Planlayout); die Bildautomatik legt dafür die eigene Zeichnung „Entwurf Erdgeschoss“ im Beispielprojekt an (Szenen `editor-…`). |
