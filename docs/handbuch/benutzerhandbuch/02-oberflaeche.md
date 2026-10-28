# 2 Oberfläche und Bedienung

[← Einstieg](01-einstieg.md) · [Inhalt](README.md) · [Weiter: Büro einrichten →](03-einrichtung.md)

## Arbeitsplatz und Projektverzeichnis

Nach dem Start zeigt BauPlanIO das **Projektverzeichnis**.

![Projektverzeichnis](bilder/arbeitsplatz-projektverzeichnis.png)

Die dunkle Leiste oben ist der **Arbeitsplatz**. Sie bleibt in jeder Ansicht
gleich:

| Eintrag | Inhalt |
| --- | --- |
| Projekte | Projektverzeichnis mit Suche, Sortierung, „Öffnen“ und „Löschen“ |
| Kunden / Bauherren | Kundenstamm, der Projekten als Bauherrschaft zugeordnet wird ([Kapitel 3](03-einrichtung.md#kunden-und-bauherren)) |
| Formularbibliothek | Projektunabhängige PDF-Vorlagen ([Kapitel 3](03-einrichtung.md#formularbibliothek)) |
| Einstellungen | Darstellung, Speicherorte, Sicherung, Büroangaben, Benutzer, Lizenz, Updates |
| Hilfe / Handbuch | Eingebaute Kurzhilfe mit Suche (auch mit F1) |

Rechts steht der angemeldete Benutzer. Die **Statusleiste** unten nennt die
aktuelle Ansicht, Benutzer und Rolle, Lizenzstand, Ablageart und
Programmversion.

### Menüleiste

| Menü | Befehle |
| --- | --- |
| Anwendung | Neues Projekt … (Strg+N), Projekt öffnen … (Strg+O), Zuletzt geöffnet, Projekt schließen (Strg+W), Projekt speichern (Strg+S), Einstellungen …, Abmelden / Benutzer wechseln … (Netzwerkbetrieb), Beenden (Strg+Q) |
| Bearbeiten | Rückgängig (Strg+Z), Wiederholen (Strg+Umschalt+Z), Ausschneiden, Kopieren, Einfügen, Alles auswählen |
| Fenster | Vergrößern / Verkleinern / Originalgröße (Strg+= / Strg+- / Strg+0), Vollbild |
| Hilfe | Bedienhandbuch (F1), Beispielprojekt öffnen, Nach Updates suchen …, Über BauPlanIO |

## Projekttabs

Ein geöffnetes Projekt erscheint als **Projekttab** unter dem Arbeitsplatz.
Mehrere Projekte können parallel offen sein; Eingaben bleiben beim Wechsel
zwischen Tabs und zum Arbeitsplatz erhalten. Ein Punkt am Tab zeigt
ungespeicherte Änderungen. Das × schließt nur die Arbeitsakte – Löschen ist
ein eigener Vorgang im Projektverzeichnis.

## Aufbau der Projektakte

![Projektakte mit Aktenkopf, Registern und Gliederung](bilder/akte-projektuebersicht.png)

1. **Aktenkopf** mit Bundesland, Bauherr, Baugrundstück, Rechtsgrundlage und
   Stand. „Projektdetails einklappen“ schafft Platz, „Hilfe zu diesem Bereich“
   öffnet das passende Hilfekapitel.
2. **Drei Register:**
   - **Übersicht** – Projektübersicht, Aufgaben / Fristen, Chronik.
   - **Bereich 1 · Ermittlung** – alles, was Werte erzeugt: Grundlagen,
     Unterlagen, Aufmaß, Messwerte, Gebäudemodell.
   - **Bereich 2 · Nachweis und Einreichung** – alles, was Werte verarbeitet
     und ausgibt: Berechnungen, Formulare, Druck, Verfahren und
     Einreichungswerkzeuge.
3. **Gliederung links** mit den Bereichen des gewählten Registers.
   - **Bereich suchen** (Strg+K) findet Bereiche und Aktionen, z. B. „GRZ“,
     „Formulare“, „Drucken“ oder „Aktuellen Bereich speichern“.
   - Der **Stern** neben einem Eintrag merkt ihn als Favorit; „Zuletzt geöffnet“
     listet die letzten fünf Bereiche.
   - Ein ausgegrauter Eintrag ist noch gesperrt. Beim Darüberfahren nennt er
     die fehlenden Schritte (z. B. fehlende Rechtsgrundlage für GRZ).
4. **Arbeitsfläche** rechts mit dem gewählten Bereich.

Die vollständige Gliederung:

| Register | Gruppe | Bereiche |
| --- | --- | --- |
| Übersicht | – | Projektübersicht, Aufgaben / Fristen, Chronik |
| Ermittlung | Grundlagen | Stammdaten, Rechtsgrundlagen, Planfestsetzungen, Flurstücke |
| | Unterlagen | Projektdateien, Aufmaß, Messwerte, Unterlagenregister |
| | Gebäudemodell | Gebäude / Geschosse, Flächen- und Raumbuch |
| Nachweis und Einreichung | Grundstück und Nutzung | Baugrundstück, GRZ, GFZ, Geschossigkeit, Bauvolumen / BMZ, Überbaubare Grundstücksfläche |
| | Gebäude und Flächen | Wohnflächenberechnung, DIN-277-Flächen, Vollgeschossprüfung, Gebäudehöhen |
| | Weitere Nachweise | Stellplatznachweis, Abstandsflächen, Versiegelung / Freiflächen, Regenwasser / Rückhaltung, Nachweisstände |
| | Formulare und Ausgabe | Formulare, Drucken / PDF-Ausgabe |
| | Verfahren | Verfahrensakte, Einreichen / Nachreichen, Suche / Wiedervorlagen |
| | Einreichungswerkzeuge | Bauantrag / XBau, Einreichen / Portal, XBau-Nachrichten, Einreichungen |

## Listen und Eintragsformulare

Viele Bereiche (Gebäude, Raumbücher, Fachberechnungen, Verfahrensakte,
Aufgaben) führen mehrere Einträge. Sie sehen zuerst eine **Eintragsliste** mit
„Neu anlegen“ und „Liste aktualisieren“. „Öffnen“ lädt einen Eintrag ins
Formular darunter, „Verlauf“ zeigt seine gespeicherten Versionen. Während Sie
bearbeiten, lässt sich die Liste mit „Eintragsliste anzeigen“ einklappen.

## Eingaben, Speichern und Entwürfe

- Felder mit **\*** sind Pflichtfelder (gelb hinterlegt). Eine leere Zahl ist
  nicht dasselbe wie die ausdrückliche Angabe 0.
- **Gestrichelt umrandete** Felder sind schreibgeschützt, weil der Wert aus
  einer Quelle kommt (z. B. aus dem Aufmaß oder dem Gebäudemodell). Geändert
  wird er an der Quelle.
- **Speichern** sichert den Bereich. Berechnen, Archivieren, Freigeben und
  Einreichen sind jeweils eigene Schritte mit eigener Bedeutung.
- Ungespeicherte Änderungen bleiben beim Wechsel erhalten. Vor dem Schließen
  fragt die App nach. Nach einem Absturz wird ein lokaler Entwurf zur
  Wiederherstellung angeboten.
- Bei gleichzeitiger Bearbeitung durch eine Kollegin meldet die App einen
  **Versionskonflikt**. Ihre Eingaben bleiben erhalten; stimmen Sie sich ab,
  bevor Sie den aktuellen Stand laden.
- Ein Eingabefehler öffnet den betroffenen Abschnitt und setzt den Fokus in
  das Feld.

### Bearbeitungsstände

Statusanzeigen kombinieren Symbol und Text. „Was bedeuten die
Bearbeitungsstände?“ klappt die Erklärung auf.

| Symbol | Stand | Bedeutung |
| --- | --- | --- |
| ✎ | Ungespeicherte Änderungen | Noch nicht Teil des gespeicherten Projektstands |
| ▣ | Gespeichert | Eingaben gespeichert; Berechnung und Freigabe stehen aus |
| ≙ | Berechnet | Ergebnis passt zu den angezeigten Eingaben |
| ▣ | Archiviert | Unveränderlicher Nachweisstand |
| ↻ | Grundlagen geändert | Neuen Nachweis erstellen, der Archivstand bleibt |
| ! | Zu prüfen | Offene Angaben oder fachliche Prüfung nötig |
| ✓ | Fachlich freigegeben | Bürofreigabe – keine Einreichung oder Genehmigung |
| ↗ | Exportiert | Dateien bereitgestellt; Versand und Eingang separat |

### Datenherkunft und nächste Schritte

Oben in vielen Bereichen steht die aufklappbare Leiste **„Datenherkunft und
nächste Schritte“**. Sie zeigt, aus welchen Bereichen die Werte stammen, ob
dort noch ungespeicherte Entwürfe liegen („Nur diese Quelle speichern“), und
führt mit einem Rückweg wieder zurück zur aufrufenden Berechnung.

## Eingebaute Hilfe

„Hilfe / Handbuch“ oder F1 öffnet die eingebaute Kurzhilfe mit Volltextsuche,
Download als Markdown und Gesamtdruck. „Hilfe zu diesem Bereich“ im Aktenkopf
springt direkt zum passenden Kapitel. Ein Wechsel zur Hilfe schließt keine
Akte; „Zurück zum Projekt“ führt zurück.

![Eingebaute Hilfe](bilder/arbeitsplatz-hilfe.png)
