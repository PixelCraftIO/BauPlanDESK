# 7 Formulare und Druckausgabe

[← Berechnungen und Nachweise](06-nachweise.md) · [Inhalt](README.md) · [Weiter: Verfahren und Einreichung →](08-verfahren.md)

Register **Bereich 2 · Nachweis und Einreichung → Formulare und Ausgabe**.

## PDF-Formulare ausfüllen

![Formulare im Projekt](bilder/nachweis-formulare.png)

Der Bereich „Formulare“ zeigt oben die **ausgefüllten Formulare dieses
Projekts** und darunter die Vorlagen der Formularbibliothek
([Kapitel 3](03-einrichtung.md#formularbibliothek)). „Vorlagen hinzufügen /
Eingabefelder einrichten …“ wechselt in die Bibliothek. Fehlen dort die
mitgelieferten Vordrucke für das Bundesland des Projekts, bietet ein Hinweis
an, sie zu übernehmen.

1. Vorlage suchen oder nach Kategorie filtern und **Ausfüllen** wählen.
2. Die App erkennt bekannte Formulare (z. B. „Baubeschreibung (Anlage I/7 zur
   VV BauPrüfVO NRW)“) und übernimmt passende Projektdaten.

![Formular ausfüllen mit Projektdaten](bilder/nachweis-formular-ausfuellen.png)

3. Unter **Quellen für Formularvorschläge** Baugrundstück, Bauherr,
   Entwurfsverfasser/in und Verfahren wählen, wenn es mehrere gibt.
   „Projektdaten neu laden“ holt geänderte Werte; manuell geänderte Felder
   bleiben erhalten.
4. Rechts die Felder ergänzen. Unter **Quelle** kann je Textfeld ein anderer
   Projektwert gewählt werden (z. B. Name und Anschrift der Bauherrschaft
   gesammelt oder „Familienname, Vorname“). „Nur leere Felder anzeigen“ hilft
   beim Vervollständigen.
5. Links die **Vorschau** der Originalseite prüfen (Seite wechseln).
   „Aktuelle Eingaben drucken …“ druckt den momentanen Stand.
6. **Im Projekt speichern.** Das ausgefüllte PDF wird Projektdatei mit
   Kategorie und erscheint im Projektordner.

### Ausgefüllte Formulare wiederfinden und aktuell halten

Die Tabelle **„Ausgefüllte Formulare dieses Projekts“** nennt je Formular
Datei und Vorlage, den Stand (Version und Datum) und den **Abgleich
mit den Projektdaten**:

| Abgleich | Bedeutung | Schaltfläche |
| --- | --- | --- |
| Aktuell | Alle aus Projektdaten übernommenen Felder stimmen noch | „Bearbeiten …“ |
| n Felder weichen ab | Projektdaten wurden seit dem Ausfüllen geändert; die Feldnamen stehen im Hinweis | „Aktualisieren …“ |
| Nicht prüfbar | Die Vorlage fehlt in der Bibliothek oder das PDF lässt sich nicht lesen | „Bearbeiten …“ |

„Bearbeiten …“ und „Aktualisieren …“ öffnen das Formular mit den gespeicherten
Eingaben. Beim Speichern entsteht eine neue Version desselben Dokuments;
frühere Versionen und bereits eingereichte Abgabestände bleiben unverändert.

### Berechnungsergebnisse als Quelle

Neben Stammdaten, Baugrundstück und Verfahren stehen unter **Quelle** auch
Ergebnisse der Fachberechnungen bereit:

| Gruppe | Werte |
| --- | --- |
| Wohnfläche | Gesamtwohnfläche, Zahl der Wohneinheiten |
| DIN 277 | Brutto-Grundfläche, Konstruktions-Grundfläche, Brutto-Rauminhalt |
| Stellplätze | Bedarf Kfz und Fahrrad |
| Freiflächen | versiegelte Fläche, Grünfläche |
| Entwässerung | abflusswirksame Fläche, Rückhaltevolumen |
| Höhen | Gebäudehöhen und Dachneigung |

Ein Wert steht nur bereit, wenn es zu der Berechnungsart **genau eine
vollständige Berechnung** gibt. Bei mehreren oder unvollständigen
Berechnungen nennt der Dialog den Grund, statt einen Wert zu raten.

## Drucken und PDF-Ausgabe

![Drucken und PDF-Ausgabe](bilder/nachweis-druckausgabe.png)

Hier entstehen Berechnungsheft und Behördenausgabe aus gespeicherten
Formularen, Dateien und Nachweisständen.

1. Unter **Unterlagen zusammenstellen** die gewünschten Einträge ankreuzen.
   Die Reihenfolge der Auswahl ist die Reihenfolge im Paket. Die Hinweise
   zeigen, ob ein Nachweis aktuell, vollständig und freigegeben ist.
   - „Berechnungsheft zusammenstellen“ wählt die empfohlenen Nachweise.
   - „Alle verfügbaren auswählen“ / „Auswahl leeren“.
2. Unter **Briefbogen und Ausgabeangaben** Bezeichnung, Empfänger,
   Aktenzeichen, Datum und Anschreiben für diese Ausgabe anpassen (Vorgabe
   aus den Einstellungen), Inhaltsverzeichnis wählen.
3. **Druckausgabe erstellen** und alle Seiten in der Vorschau prüfen
   (Vollständigkeit, Lesbarkeit, Planmaßstab, richtige Version).
4. Als PDF speichern, mit **Als Projektunterlage ablegen** versioniert sichern
   oder **Drucken …** für den Systemdruck.

Originalformulare und Pläne behalten Gestaltung und Maßstab und erhalten keinen
Briefkopf. Nicht druckbare Dateien (DWG, Excel) müssen vorher als PDF
abgelegt werden. Aus einem Abgabestand der Verfahrensakte lassen sich genau die
eingereichten Versionen in die Druckausgabe übernehmen („Versionen aus
Abgabestand auswählen“).

> Ungespeicherte Eingaben zuerst im Fachbereich speichern; GRZ, GFZ,
> Geschossigkeit, BMZ und Abstandsflächen vorher als Nachweisstand sichern.

## Rohbauwert, Gebühr und Baustatistik

„Formulare und Ausgabe → Rohbauwert, Gebühr, Baustatistik“ schätzt die
Baugenehmigungsgebühr aus dem Brutto-Rauminhalt der DIN-277-Berechnung, dem
Rohbauwert je m³ und dem Gebührensatz des Landes. Eigene Werte überschreiben
die abgeleiteten sichtbar. Für NRW sind die Werte ungeprüfte Schätzwerte; vor
der Weitergabe mit dem aktuellen Runderlass und dem Gebührentarif vergleichen.

Die Tabelle zur Baustatistik sammelt die Angaben für den Erhebungsbogen mit
Quelle je Angabe. Beides lässt sich als PDF im Briefbogen ausgeben.
