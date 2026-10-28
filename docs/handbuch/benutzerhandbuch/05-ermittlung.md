# 5 Unterlagen, Aufmaß und Gebäudemodell

[← Projekt und Grundlagen](04-projekt-grundlagen.md) · [Inhalt](README.md) · [Weiter: Berechnungen und Nachweise →](06-nachweise.md)

Register **Bereich 1 · Ermittlung**, Gruppen *Unterlagen* und *Gebäudemodell*.
Hier entstehen die Werte, mit denen später gerechnet wird.

## Projektdateien

![Projektdateien](bilder/ermittlung-projektdateien.png)

Alle Unterlagen des Projekts (PDF, DXF/DWG, Office, Fotos, E-Mails) liegen
versioniert in der verwalteten Ablage.

- **Importieren:** „Kategorie für neue Importe“ wählen, dann „Dateien
  importieren …“. Bei einer gleichnamigen Datei fragt die App, ob sie eine neue
  Version des vorhandenen Dokuments oder ein eigenes Dokument werden soll.
- **Ordnen:** Dateisuche, Kategoriefilter (Hauptkategorie schließt
  Unterkategorien ein, „Ohne Kategorie“ zum Nachpflegen) und Sortierung.
  Mehrere Dateien auswählen und „Kategorie zuordnen“.
- **Kategorie ändern:** „Ändern“ in der Zeile. Das erzeugt keine neue
  Dateiversion.
- **Öffnen / extern bearbeiten:** „Öffnen“ erstellt eine Arbeitskopie und
  startet das zugeordnete Programm (Excel, CAD …). Die Änderung wird unter
  „Externe Arbeitskopien“ geprüft und ausdrücklich als neue Version übernommen.
- **Weitere …:** Versionen und Historie, Dokumentangaben, „Im Aufmaß öffnen“
  (PDF/DXF), „Formular bearbeiten“ (ausgefüllte Formulare), „E-Mail lesen“
  (EML/MSG mit Übernahme von Anhängen).
- **Projektordner exportieren …** schreibt eine lesbare Kopie („Dokumente“,
  „Pläne“), auf Wunsch mit Dokumenthistorie.
- **Sperre:** Wer eine Datei extern bearbeitet, sperrt sie für andere.

DWG-Dateien werden als Datei verwaltet, aber nicht dargestellt. Für das Aufmaß
im CAD-Programm als DXF exportieren.

## Aufmaß und Zeicheneditor

Im Aufmaß werden Flächen, Strecken und Höhen auf Plänen gemessen und den
Zielen im Projekt zugeordnet (Flurstück, Abzugsfläche, Gebäude, Geschoss,
Raum, Wand, Fachberechnung).

![Aufmaß in der Projektakte](bilder/ermittlung-aufmass.png)

### Aufmaß anlegen

- Aus den Projektdateien: bei einer PDF oder DXF „Im Aufmaß öffnen“.
- Oder im Bereich „Aufmaß“: **Neu anlegen**, Unterlage, Dokumentversion,
  Seite (PDF) und Bezeichnung wählen.
- **Neue Zeichnung** legt ein leeres Blatt in Metern an (siehe unten).

Geöffnete Aufmaße und Zeichnungen erscheinen als **Tabs** über der
Zeichenfläche („Aufmaßliste“ führt zur Übersicht). Das Symbol rechts daneben
öffnet das Aufmaß **in einem eigenen Fenster**, z. B. auf dem zweiten
Bildschirm; die Akte bleibt im Hauptfenster bedienbar.

### Arbeitsplatz des Editors

![Zeicheneditor im Vollbild](bilder/ermittlung-aufmass-vollbild.png)

| Bereich | Inhalt |
| --- | --- |
| Kopfleiste | Speichern, Schließen, Rückgängig/Wiederholen, „Aufmaßblatt PDF …“, „Messwerte CSV …“, Maße anzeigen, Vollbild (F11), Kurzbefehle (F1) |
| Werkzeugleiste links | Auswählen (V), Verschieben (M), Polygon (P), Rechteck (R), Kreis (K), Strecke / Dach (S), Wand (W), Maßkette (L), Text (X), Höhenpunkt (H), Bezugslinie setzen, Ausschnitt anlegen, Zwei Punkte kalibrieren, Kontrollstrecke messen, Duplizieren, Markierung entfernen |
| Ansichtsleiste | Zoom, Einpassen, 1:1, Auf Auswahl, Zoomfenster (Z), drehen, **Punktfang** und Fangoptionen |
| Paletten rechts | **Aufgaben** (was noch gezeichnet bzw. übernommen werden muss), **Objekte** (Ausschnitte, Zeichenebenen, Markierungen), **Eigenschaften** (der Auswahl), **Werkzeugkiste** (Vorlagen) |
| Fußzeile | Kurzhilfe zu Maus und Tastatur |

Alle Werkzeuge erscheinen als Symbol; der Name und das Tastenkürzel stehen im
Tooltip. Die vollständige Tastenbelegung: [Kapitel 10](10-anhang.md#zeicheneditor).

### Maßstab und Ausschnitte

1. **PDF:** Blattmaßstab 1:n setzen oder mit „Zwei Punkte kalibrieren“ eine
   bekannte Strecke anklicken und ihre Länge eingeben. **DXF:** Die
   Zeichnungseinheit kommt aus der Datei und kann korrigiert werden;
   DXF-Ebenen lassen sich ein- und ausblenden.
2. Mit **Kontrollstrecke messen** eine bekannte Länge nachmessen.
3. Bei Sammelblättern (Grundrisse, Schnitt, Ansicht auf einem Blatt) je Teil
   einen **Ausschnitt** mit eigenem Maßstab anlegen. Für Höhen im Schnitt eine
   **Bezugslinie** mit Höhe und Höhensystem (z. B. m ü. NHN) setzen.

![Objekte: Ausschnitte, Zeichenebenen, Markierungen](bilder/ermittlung-aufmass-objekte.png)

Die Palette **Objekte** listet die Ausschnitte („Öffnen“ zeigt nur diesen
Bereich), die **Zeichenebenen** (Sichtbarkeit „An“, „Sperre“, aktive Ebene)
und alle **Markierungen** mit Suche und Filtern nach Bedeutung und Gebäude.

### Zeichnen und zuordnen

Der schnellste Weg führt über die Palette **Aufgaben**: Sie listet alle Ziele
im Projekt, die einen Messwert brauchen – Flurstücke, Abzugsflächen,
GRZ-Bauteile, Gebäude und Geschosse, Abstandswände, Positionen der
Fachberechnungen und Räume der Raumbücher.

1. Beim gewünschten Ziel **Zeichnen** wählen. Werkzeug, Bedeutung und
   Zuordnung sind damit voreingestellt.
2. Auf dem Plan die Punkte setzen. Nach dem ersten Punkt öffnet eine Ziffer die
   **Maßeingabe** (Länge, Winkel). Enter schließt die Form, Escape bricht ab.
3. Aufmaß **speichern**.
4. Beim Ziel **Übernehmen** wählen (oder in der Zielmaske „Aus Aufmaß“). Der
   Wert wird eingesetzt, das Zielfeld ist danach mit der Markierung verknüpft
   und schreibgeschützt. „Verknüpfte Werte gesammelt prüfen …“ zeigt alle
   Übernahmen mit altem und neuem Wert.

Alternativ eine Vorlage aus der **Werkzeugkiste** anklicken (z. B. „DIN 277
NUF“, „Versiegelte Fläche“, „Gebäudegrundfläche“). Sie stellt Werkzeug,
Bedeutung, Namen und Zeichenebene ein. Freie Markierungen erhalten ihre
**Bedeutung** und Zuordnung in der Palette **Eigenschaften**.

| Zu zeichnen | Bedeutung | Zuordnung |
| --- | --- | --- |
| Flurstücksgrenze aus Flurkarte/Lageplan | Flurstück | Flurstück |
| Abzugsfläche (Grünfläche, Verkehrsfläche, außerhalb Bauland …) | passende Abzugsbedeutung | Flurstück |
| Zufahrt, Stellplatz, Garage, Nebenanlage, Terrasse, unterirdische Anlage | Bauteilbedeutung | Baugrundstück |
| Gebäudegrundfläche, Geschossfläche | Gebäudegrundfläche / Geschoss – Außenkante | Gebäude, Geschoss |
| Raum | DIN 277 NUF/TF/VF bzw. Wohnfläche | Raumbuch |
| Höhe (Traufe, First, Gelände) | Höhenpunkt bzw. Strecke / Dach | Gebäude |

### Lage im Baufenster

Für die Prüfung nach § 23 BauNVO im Lageplan je genau eine Markierung
**Baufenster** und **Vorhabenumriss** zeichnen (bei mehreren Baugrundstücken
dem Umriss das Baugrundstück zuordnen) und unter „Lage im Baufenster“ dem
Projekt zuordnen. Der GRZ-Nachweis zeigt dann die Fläche außerhalb des
Baufensters.

### Handwert und Zeichnung abgleichen

Oft wird ein Wert erst von Hand eingetragen und später gezeichnet. BauPlanIO
vermeidet dabei Doppelerfassungen:

- **Ein Feld hat genau eine Quelle.** Die Übernahme ersetzt den Handwert;
  „Verknüpfung lösen“ macht ihn wieder zur Handangabe und behält den Wert.
- **Abzugsflächen, GRZ-Bauteile, Räume:** Gibt es schon eine passende
  Handzeile (gleiche Art am selben Flurstück/Baugrundstück bzw. gleicher
  Raumname), fragt die Übernahme: *verknüpfen* (Art, Grundlage und
  Bezeichnung bleiben) oder *neue Zeile*. Die Aufgaben-Palette kennzeichnet von Hand
  erfasste Werte mit „manuell erfasst“ bzw. „Handwert ohne Zeichnung, passende
  Markierung vorhanden“.
- **Doppelerfassung:** Stehen eine gezeichnete und eine fast gleich große
  Handzeile (höchstens 1 m² oder 2 % Unterschied) nebeneinander, meldet die
  Baugrundstücksprüfung einen Hinweis mit „Handzeile entfernen“.
- **Veraltete Verknüpfung:** Ändert sich eine gezeichnete Kontur, der Maßstab
  oder die Dokumentversion, melden Zielmaske und Editor dies schon beim
  Öffnen. Im Ziel „Aktuellen Aufmaßstand übernehmen“, speichern und neu
  prüfen.
- **Löschen:** Vor dem Entfernen einer verknüpften Markierung nennt die App
  die betroffenen Werte.
- **Nachvollziehbarkeit:** Die Chronik vermerkt jede neue, aktualisierte oder
  gelöste Aufmaßverknüpfung mit Zielfeld, altem und neuem Wert.

### Eigene Zeichnung in Metern

1. **Neue Zeichnung** wählen. Das Blatt rechnet in Metern (Meterraster),
   Maßstab und Kalibrierung entfallen.
2. Unter „Blatt und Einrichtung → Hintergründe“ eine PDF-Seite oder DXF als
   Hintergrund hinzufügen und einpassen: Maßstab bzw. Einheit, „Über Strecke
   einpassen“, „Verschieben“, „Um Punkt drehen“, Deckkraft und Sichtbarkeit.
   Der Punktfang greift auf die Vektoren des Hintergrunds.
3. Mit **Wand** (W), **Maßkette** (L) und **Text** (X) zeichnen. Diese Elemente
   dienen der Darstellung und liefern keine Messwerte.
4. Flächen, Strecken und Höhen als Markierungen zeichnen und wie oben zuordnen.
   Der PDF-Export gibt die Zeichnung im Maßstab 1:100 aus.

## Messwerte

![Messwerte und Verwendung](bilder/ermittlung-messwerte.png)

„Messwerte“ beantwortet, welche Fläche, Länge oder Höhe aus welchem Aufmaß in
welche Berechnung eingeht. Oben stehen die Zähler (Markierungen, davon
verwendet, mit älterem Aufmaßstand, offene Aufgaben). Filter nach Aufmaß,
Status und Suchtext. **Im Editor zeigen** springt zur Markierung.

## Unterlagenregister

![Unterlagenregister](bilder/ermittlung-unterlagenregister.png)

Das Unterlagenregister führt die Bauvorlagen des Projekts als Liste mit Stand
(z. B. „Dokument fehlt“, „Freigegeben“). Jeder Eintrag verbindet eine Unterlage
mit einer konkreten Dokumentversion und ihrer Prüfung. „Bürocheckliste
vorbereiten“ lädt die mitgelieferte Standard-Checkliste als Vorlage für neue
Einträge; „Eigene Checkliste laden“ liest eine Büro-Checkliste als JSON-Datei.

## Gebäude und Geschosse

![Gebäude mit Grundfläche und Geschossen](bilder/ermittlung-gebaeude.png)

Das Gebäudemodell ist die gemeinsame Quelle für GRZ, GFZ, Geschossigkeit,
Vollgeschosse, Höhen und den Bauantrag.

1. **Neu anlegen**, Bezeichnung und **Gebäudetyp** (Hauptgebäude/Wohnhaus,
   Garage, Nebenanlage …) erfassen.
2. Die **Flurstücke** zuordnen, auf denen das Gebäude steht. Daraus ergibt sich
   das Baugrundstück.
3. **Gebäudegrundfläche** (horizontale Projektion) eingeben oder aus dem
   Aufmaß übernehmen.
4. **Geschosse** mit Fläche nach Außenmaßen und Einstufung (Vollgeschoss ja/nein)
   ergänzen. „Vollgeschossstatus dieses Geschosses prüfen …“ legt einen
   Prüfauftrag in der Vollgeschossprüfung an; das Ergebnis wird danach mit
   „Aus Vollgeschossnachweis übernehmen“ verknüpft.
5. Gelände-, Trauf- und Firsthöhen mit Höhensystem ergänzen und speichern.

Vollständig erfasste Gebäude fließen automatisch in GRZ und GFZ ein. Dort nicht
noch einmal von Hand erfassen.

## Flächen- und Raumbuch

![Raumbuch Erdgeschoss](bilder/ermittlung-raumbuch.png)

Das Raumbuch sammelt die Räume je Gebäude und Geschoss. Es ist Quelle für
Wohnflächenberechnung und DIN-277-Nachweis.

1. **Neu anlegen**, Gebäude und Geschoss wählen.
2. Räume mit Nummer, Bezeichnung, Nutzung, Fläche und lichter Höhe erfassen.
   Bei Dachschrägen die Höhenzonen getrennt angeben.
3. Oder „Räume aus Aufmaß übernehmen“: gezeichnete Raumkonturen werden zu
   Räumen; vorhandene Handräume mit gleicher Nummer/Bezeichnung werden
   verknüpft statt verdoppelt.
4. Speichern. Nach einer Planänderung zuerst das Aufmaß, dann das Raumbuch und
   danach die Wohnfläche/DIN 277 aktualisieren.
