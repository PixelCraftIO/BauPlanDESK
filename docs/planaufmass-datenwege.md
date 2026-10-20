# Datenwege vom PDF-Aufmaß zu Berechnungen, Nachweisen und Formularen

Stand: Prüfung aller Übernahmewege, Oktober 2026. Ergänzt
[planaufmass.md](planaufmass.md) um die Bedienreihenfolge, die Lade- und
Aktualisierungszeitpunkte und die festgestellten offenen Punkte.

## Grundprinzip

```text
PDF (Projektdateien, versioniert)
  ↓  Unterlagen → PDF-Aufmaß: zeichnen, Maßstab, Bedeutung, Grundstück/Gebäude/Geschoss, speichern
Planaufmaß (Fachobjekt, Revision R n)
  ↓  Planbindung = Aufmaß-ID + Revision + Markierungs-ID + Zielfeld (stabile IDs)
  ├─ Gebäude / Geschosse ──→ Gebäudeeinbindung ──→ GRZ / GFZ / Geschossigkeit je Baugrundstück
  │                                               └→ Formulare, XBau-Flächen, Bauantrag
  ├─ Fachberechnungen (DIN 277, Wohnfläche, Freiflächen, Entwässerung, Vollgeschosse, Höhen)
  ├─ Raumbuch ──→ Wohnfläche / DIN 277 (Import „aus Raumbuch“)
  └─ Abstandsflächen (manuelle Wände)
                     ↓
          Nachweisstände (archiviert) → Druck
```

Eine Verknüpfung ist **kein Live-Wert**: Der Messwert wird beim Verknüpfen in
den Entwurf geschrieben und beim Prüfen/Speichern vom Core aus genau der
gebundenen Aufmaßrevision neu ermittelt. Wird das Aufmaß erneut gespeichert
oder die PDF in neuer Version abgelegt, gilt die Verknüpfung als veraltet; der
Verbraucher blockiert Speichern bzw. Nachweis, bis **Aktuellen Aufmaßstand
übernehmen** gewählt und gespeichert wurde. Dadurch ändert sich kein
gespeicherter Wert unbemerkt.

## Wege im Einzelnen

### 1. Zeichnen (Unterlagen → PDF-Aufmaß)

1. PDF unter Projektdateien importieren, **Neu anlegen**, Unterlage und Seite wählen.
2. Maßstab setzen oder zwei Punkte kalibrieren.
3. Fläche zeichnen, **Bedeutung** wählen. Sie entscheidet, wohin der Wert gehen kann.
4. Baugrundstück, Gebäude und ggf. Geschoss zuordnen. Angeboten werden nur
   **gespeicherte** Gebäude und Geschosse; die Listen laden bei jeder
   Speicherung eines Fachobjekts neu.
5. Aufmaß speichern. Das Seitenpanel nennt je Bedeutung den nächsten Schritt.
6. Soll eine Kontur zwei Bedeutungen haben (z. B. Gebäudegrundfläche *und*
   EG-Außenkante), **Duplizieren** und die Kopie umkennzeichnen.

### 2. Gebäude / Geschosse

| Ziel | Bedeutung der Markierung | Wo übernehmen |
| --- | --- | --- |
| Gebäudegrundfläche | Gebäudegrundfläche – horizontale Projektion | Feld *Gebäudegrundfläche* → **Aus PDF-Aufmaß** |
| Geschossfläche | Geschoss – Außenkante + Geschosszuordnung | im Geschoss → **Aus PDF-Aufmaß** |
| Höhen (NHN, relativ) | Höhenpunkt mit Höhenbezug „m ü. NHN“ bzw. Strecke | **Höhen und alle Verknüpfungen aus PDF-Aufmaß** |

Reihenfolge: Gebäude anlegen und speichern → Geschosse anlegen und speichern →
im PDF-Aufmaß zuordnen und speichern → im Gebäude übernehmen → Gebäude speichern.
Ohne einzelne Geschosse dient die Gebäudegrundfläche als Fläche des
vorläufigen Einzelgeschosses; dann ist keine Geschossmarkierung nötig.

### 3. Gebäude → GRZ / GFZ / Geschossigkeit

* Voraussetzungen: Gebäude steht auf Flurstücken (Grundlagen), die Flurstücke
  sind gespeichert, die Baugrundstücksermittlung ist aktuell, Gebäudegrundfläche
  **und** Gebäudetyp sind bekannt (GRZ), Geschossflächen und Einstufungen sind
  bekannt (GFZ).
* Übernahme: automatisch beim Öffnen von GRZ/GFZ, nach Speichern eines Gebäudes
  (Ereignis „fachobjekte“) sowie bei **Berechnen** und **Eingaben speichern**.
  Die eingesetzten Bauteile/Geschosse erscheinen als ungespeicherte Änderung.
* „Ermittlung ist veraltet“: tritt nach jeder Projektspeicherung auf, solange
  die Seite offen ist. Der Hinweis enthält jetzt **Neu ermitteln**; bisher half
  nur „Berechnen“.

### 4. Fachberechnungen

Position unter **Eingaben** anlegen → Reiter **Quellen / Übernahmen** → Zielfeld
→ Markierung → **Verknüpfen und Messwert einsetzen** → Prüfen/Speichern.
Baugrundstück und Gebäude der Berechnung müssen mit der Markierung
übereinstimmen. DIN 277 verlangt NUF/TF/VF/KGF, absolute Höhen einen Höhenpunkt
im selben Höhensystem, Höhendifferenzen eine Strecke. Die Auswahl bietet nur
noch solche Markierungen an und erklärt eine leere Liste. Bei Vollgeschossen und
Höhen muss die Gebäudeübernahme vorher abgeschaltet werden.

### 5. Raumbuch

Gebäude und Geschoss wählen → **Räume aus PDF-Aufmaß übernehmen** → Aufmaß →
**Planräume anlegen / aktualisieren** → speichern. Wohnfläche/DIN 277 übernehmen
die Räume über **aus Raumbuch** als Kopie mit gebundener Raumbuchrevision.
Ein erneuter Raumbuchimport entfernt jetzt auch die PDF-Verknüpfungen der
ersetzten Positionen, statt beim nächsten Prüfen mit „Zielposition fehlt“
abzubrechen.

### 6. Abstandsflächen

Manuelle Wand anlegen → **Grundlagen → PDF-Aufmaß verknüpfen**. Beim Entfernen
einer Wand entfallen jetzt auch ihre Verknüpfungen.

### 7. Nachweise, Druck, Formulare

* Nachweisstände sind archivierte Kopien. Ihr Fingerprint enthält Aufmaß,
  Revision und neueste PDF-Version; jede Änderung macht sie „veraltet“
  (Druck mit Hinweis), der archivierte Inhalt bleibt unverändert.
* Formulare und XBau lesen GRZ/GFZ/Geschossflächen/Vollgeschosse live über
  `projekt_mit_gebaeuden`. Formulare laden beim Öffnen oder über
  **Projektdaten neu laden**. Ergebnisse der übrigen Fachberechnungen
  (Wohnfläche, BGF/BRI, Versiegelung, Entwässerung, Höhen) erreichen Formulare
  derzeit nicht.

## In diesem Stand behoben

* Direkte Flächenübernahme an Gebäudegrundfläche und Geschossfläche mit
  Erklärung, warum eine vorhandene Markierung nicht passt.
* Duplizieren von Markierungen; Hinweise je Bedeutung statt des pauschalen
  Verweises auf „Quellen / Übernahmen“; unbenannte Geschosse werden als
  „Geschoss n“ angezeigt (auch in den Zielfeldern des Cores).
* Markierungsauswahl in Fachberechnungen und Gebäuden entspricht der
  Core-Prüfung (Bedeutung, Geometrie, Höhensystem); leere Auswahl wird erklärt.
* „Neu ermitteln“ im Veraltet-Hinweis auf GRZ/GFZ-Seiten.
* Verwaiste Verknüpfungen nach Wandentfernung und Raumbuchimport.

## Offene Punkte (nach Priorität)

1. **Ein veraltetes Gebäudeaufmaß blockiert alle Baugrundstücke.** Die
   Gebäudeeinbindung bricht mit einem harten Fehler ab (auch beim Speichern
   reiner Grundstücksabzüge) und nennt das Gebäude nicht. Vorschlag: pro
   Gebäude als Einbindungshinweis melden und nur dieses Gebäude auslassen, wie
   in [datenfluss-gebaeude-grundstuecke.md](datenfluss-gebaeude-grundstuecke.md)
   beschrieben. *Regeländerung im Core.*
2. **Jede Aufmaßspeicherung macht alle Verknüpfungen veraltet**, auch
   Ebenen-Sichtbarkeit oder Umbenennungen. Vorschlag: Aktualität je Markierung
   über einen Geometrie-/Kalibrierungs-Fingerprint statt der Objektrevision;
   im Editor anzeigen, welche Verbraucher eine Markierung nutzen.
   *Vertragsänderung `Planbindung`.*
3. **Eine ungültige Markierung legt das ganze Aufmaß lahm** (gelöschtes
   Geschoss/Gebäude, umgruppiertes Grundstück); die Meldung nennt die
   Markierung nicht. Vorschlag: Markierung benennen und in der
   Projektübersicht über `planaufmass_pruefen` melden; beim Löschen eines
   Geschosses auf verweisende Markierungen hinweisen.
4. **Raumbuch-Aktualisierung in Wohnfläche/DIN 277** ersetzt die Positionen
   und verliert Raumart, Wohnung und Klassifikation. Vorschlag: Abgleich über
   Raum-IDs statt Ersatz.
5. **Vollgeschosse/Höhen** schalten bei genau einem Gebäude die
   Gebäudeübernahme automatisch ein; eine PDF-Verknüpfung scheitert dann mit
   „konkurrierende Eingabequellen“ ohne vorherigen Hinweis.
6. **Abstandsflächen:** `hoehenquelle` hält nur die letzte Verknüpfung und
   druckt IDs statt Bezeichnungen; das Höhenprofil sperrt bei einer Verknüpfung
   den ganzen Bereich; Markierungsauswahl prüft Höhensystem und Bedeutung nicht.
7. **Aktualisierung von Listen:** Gebäude (Baugrundstücksanzeige,
   Vollgeschossnachweise), Bauantrag (Gebäudeanzahl) und Formulare abonnieren
   Änderungen anderer Bereiche nicht; Werte erscheinen erst nach Neuladen.
8. **Projektübersicht-Links** öffnen den Datensatz, aber nicht den Reiter bzw.
   Abschnitt mit der Verknüpfung.
9. **Einzelgeschoss:** Die Einstufung in GFZ ist eine Kopie und kann der
   Einstufung im Gebäude widersprechen.
10. **Nachweisstand speichern** ist auch mit ungespeicherten Eingaben möglich;
    der archivierte Nachweis kann dann vom gespeicherten Stand abweichen, den
    Formulare und XBau verwenden.
11. BMZ übernimmt keine Gebäudedaten; Entwässerung kann Dachflächen mehrerer
    Gebäude nicht in einer Berechnung bündeln; Druck führt Planbindungen nicht
    als Quellen auf.

## Kompatibilität

Keine Datenmigration, keine Vertragsänderung. Geändert sind nur Bedienung,
Vorauswahl und der Anzeigetitel von Zielfeldern unbenannter Geschosse
(`gebaeudeziele`); die Zielfeld-IDs bleiben gleich. Die Übernahmeprüfung im
Core bleibt maßgebend.
