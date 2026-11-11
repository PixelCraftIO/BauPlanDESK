# Datenwege vom Aufmaß zu Berechnungen, Nachweisen und Formularen

Stand: Prüfung aller Übernahmewege, Oktober 2026. Ergänzt
[planaufmass.md](planaufmass.md) um die Bedienreihenfolge, die Lade- und
Aktualisierungszeitpunkte und die festgestellten offenen Punkte.

## Grundprinzip

```text
PDF (Projektdateien, versioniert)
  ↓  Unterlagen → Aufmaß: zeichnen, Maßstab, Bedeutung, Grundstück/Gebäude/Geschoss, speichern
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
den Entwurf geschrieben und beim Prüfen/Speichern vom Core neu ermittelt.
Ändern sich danach die gebundene Markierung (Kontur, Bezeichnung, Bedeutung,
Zuordnung), Maßstab, Höhenbezug oder die PDF-Version, gilt die Verknüpfung als
veraltet; der Verbraucher blockiert Speichern bzw. Nachweis, bis **Aktuellen
Aufmaßstand übernehmen** gewählt und gespeichert wurde. Dadurch ändert sich kein
gespeicherter Wert unbemerkt. Ebenen, Sichtbarkeit und weitere Markierungen
dürfen sich ändern, ohne dass etwas veraltet.

## Wege im Einzelnen

### 1. Zeichnen (Unterlagen → Aufmaß)

1. PDF unter Projektdateien importieren, **Neu anlegen**, Unterlage und Seite wählen.
2. Blattmaßstab setzen oder zwei Punkte kalibrieren.
3. Bei mehreren Zeichnungen auf dem Blatt je Zeichnung **Ausschnitt anlegen**
   (Rahmen ziehen), Art wählen, bei Bedarf eigenen Maßstab setzen. Für
   Schnitte/Ansichten **Bezugslinie setzen** (±0,00-Linie anklicken, Höhe und
   System angeben; Gebäudenullpunkt als Vorschlag).
4. Ausschnitt öffnen (Doppelklick oder „Öffnen“). Unter **Aufgaben** steht,
   was Gebäude und Wände noch brauchen; „Zeichnen“ wählt das Werkzeug und
   belegt Bedeutung, Gebäude, Geschoss und Namen vor. Ohne Aufgabe: zeichnen,
   **Bedeutung** und Zuordnung von Hand wählen.
5. Unter **Aufgaben → Übernehmen** die Markierung in das Gebäude bzw. die
   Wand übernehmen; ein ungespeichertes Aufmaß wird dabei zuerst gespeichert,
   danach der Verbraucher. Die Vorschau nennt alten und neuen Wert und die
   Quelle mit Aufmaß, Markierung und Aufmaßstand; das
   Seitenpanel nennt je Bedeutung den sonstigen Übernahmeort.
6. Für Fachberechnungen weiterhin unter „Quellen / Übernahmen“ verknüpfen.
7. Soll eine Kontur zwei Bedeutungen haben (z. B. Gebäudegrundfläche *und*
   EG-Außenkante), in den Eigenschaften **Weitere Bedeutung für diese
   Kontur …** wählen. Die weitere Bedeutung ist eine eigene Markierung mit
   eigener Zuordnung, deren Geometrie der Kontur folgt (`kontur_von`); im Plan
   steht die Kontur nur einmal. „Von der Kontur lösen“ macht daraus eine
   unabhängige Kopie. Wird die Kontur gelöscht, bleibt die erste weitere
   Bedeutung als eigenständige Kontur stehen.
8. Räume misst allein das Raumbuch. Wohnflächen und DIN 277 mit
   Raumbuchquelle bekommen für ihre Raumzeilen keine eigene Messaufgabe; sie
   folgen dem Raumbuch (Hinweis „Räume aktualisieren“ in der Berechnung oder
   automatisch bei „Alle veralteten Nachweise erneuern“). Eigene Angaben der
   Berechnung (Wohnung, Raumart, Geschoss, Flächenart) bleiben je Raum
   erhalten; bei gleichen Flächen und Höhen bleibt auch die Bestätigung.
9. Neuer Planstand: Das Aufmaß wechselt die Unterlage und behält seine
   Kennungen (`planstandWechseln` in `uebertragung.ts`). Die Ausrichtung der
   beiden Blätter kommt aus zwei Passpunkten (`ausrichtung.ts`) oder von
   Hand; Konturen, Rahmen, Maßstäbe und Höhenbezüge werden umgerechnet. Die
   Verbraucher sehen danach nur einen neuen Aufmaßstand und übernehmen ihn
   auf dem üblichen Weg.
10. Gegenproben (`planaufmass/gegenprobe.rs`): Die Projektübersicht vergleicht
    doppelt Gezeichnetes über alle Aufmaße (Flurstück, Gebäudegrundfläche,
    Geschoss-Außenkante, Grundfläche gegen Außenkante Erdgeschoss, First und
    Traufe) und meldet Abweichungen über der Toleranz als Hinweis.
11. Pläne verbinden: Über die Passpunkte des Planvergleichs lassen sich
    Markierungen eines anderen Aufmaßes in den geöffneten Plan holen
    (`markierungenHolen`), etwa Flurstücke der Flurkarte in den Lageplan. Der
    Abstandsplan bleibt bei genau einem Lageplan als Koordinatenbezug.

Bedienung: Strg+Rad oder Pinch zoomt um den Mauszeiger, Z zieht ein
Zoomfenster, Leertaste oder mittlere Maustaste verschiebt, ↶/↷ dreht die
Ansicht, „Seiten“ zeigt Miniaturen, F11 schaltet Vollbild, Esc verlässt
Werkzeug, Ausschnitt bzw. Vollbild. Werkzeuge über V, M, P, R, K, S, H; eine
Ziffer nach dem ersten Punkt öffnet die Maßeingabe (Länge;Winkel bzw.
Breite;Höhe in m); Entf, Strg+Z/Y, Strg+D, Pfeiltasten (Markierung
verschieben), Bild ↑/↓ (Ausschnitt wechseln).

### 2. Gebäude / Geschosse

| Ziel | Bedeutung der Markierung | Wo übernehmen |
| --- | --- | --- |
| Gebäudegrundfläche | Gebäudegrundfläche – horizontale Projektion | Feld *Gebäudegrundfläche* → **Aus Aufmaß** |
| Geschossfläche | Geschoss – Außenkante + Geschosszuordnung | im Geschoss → **Aus Aufmaß** |
| Höhen (NHN, relativ) | Höhenpunkt mit Höhenbezug „m ü. NHN“ bzw. Strecke | **Höhen und alle Verknüpfungen aus Aufmaß** |

Reihenfolge: Gebäude anlegen und speichern → Geschosse anlegen und speichern →
im Aufmaß zuordnen und speichern → im Gebäude übernehmen → Gebäude speichern.
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

Gebäude und Geschoss wählen → **Räume aus Aufmaß übernehmen** → Aufmaß →
**Planräume anlegen / aktualisieren** → speichern. Wohnfläche/DIN 277 übernehmen
die Räume über **aus Raumbuch** als Kopie mit gebundener Raumbuchrevision.
Eine erneute Übernahme gleicht über die Raum-ID ab: Flächen und Höhen kommen
aus dem Raumbuch, Raumart, Wohnung, Geschoss und Klassifikation bleiben erhalten,
manuelle Positionen ebenso. Entfallene Räume und deren PDF-Verknüpfungen werden
entfernt.

### 6. Abstandsflächen

Manuelle Wand anlegen → **Grundlagen → Aufmaß verknüpfen**. Beim Entfernen
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

## Umgesetzte Verbesserungen

Erste Stufe:

* Direkte Flächenübernahme an Gebäudegrundfläche und Geschossfläche mit
  Erklärung, warum eine vorhandene Markierung nicht passt.
* Duplizieren von Markierungen; Hinweise je Bedeutung statt des pauschalen
  Verweises auf „Quellen / Übernahmen“; unbenannte Geschosse werden als
  „Geschoss n“ angezeigt (auch in den Zielfeldern des Cores).
* Markierungsauswahl in Fachberechnungen, Gebäuden und Abstandsflächen
  entspricht der Core-Prüfung (Bedeutung, Geometrie, Höhensystem); eine leere
  Auswahl wird erklärt.
* „Neu ermitteln“ im Veraltet-Hinweis auf GRZ/GFZ-Seiten.
* Keine verwaisten Verknüpfungen nach Wandentfernung und Raumbuchimport.

Zweite Stufe (Arbeitsablauf im Büro):

* **Gestalterische Änderungen veralten nichts mehr.** Ebenen ein-/ausblenden,
  sperren oder weitere Markierungen zeichnen lässt bestehende Verknüpfungen und
  Nachweise gültig. Nur Änderungen an der gebundenen Markierung, am Maßstab,
  am Höhenbezug oder an der Unterlage verlangen „Aktuellen Aufmaßstand
  übernehmen“.
* **Eine fehlerhafte Markierung blockiert nicht das ganze Aufmaß** für andere
  Verbraucher; Meldungen nennen Markierung, Gebäude bzw. Raum und den nächsten
  Schritt. Die Projektübersicht meldet ungültige Zuordnungen am Aufmaß selbst.
* **Projektübersicht springt an die richtige Stelle:** Berechnungen öffnen
  „Quellen / Übernahmen“, Gebäude und Raumbuch den PDF-Abschnitt.
* **Raumbuch-Aktualisierung erhält Zuordnungen:** Wohnfläche/DIN 277 gleichen
  über die Raum-ID ab; Raumart, Wohnung, Geschoss und Klassifikation bleiben,
  manuelle Positionen bleiben stehen, die Meldung nennt neu/aktualisiert/entfallen.
* **Gebäudeübernahme vs. PDF-Verknüpfung** (Vollgeschosse/Höhen): Hinweis vor
  dem Verknüpfen und ein Knopf zum Beenden der Gebäudeübernahme.
* **Nachweisstand speichern** speichert ungespeicherte Eingaben zuerst und
  berechnet neu, damit Nachweis, Formulare und XBau denselben Stand haben.
* **Einzelgeschoss:** Die Einstufung aus Gebäude/Geschosse gilt auch in der GFZ.
* **Geschoss entfernen** warnt, wenn Markierungen auf das Geschoss verweisen.
* **Abstandsflächen:** Herkunft aller Bindungen einer Wand mit Namen im Druck;
  das Höhenprofil sperrt nur die verknüpfte Liste (Gelände bzw. Oberkante).
* **Aktualität der Listen:** Gebäude (Baugrundstück, Vollgeschossnachweise),
  Bauantrag (Gebäudeanzahl) und Abstandsflächen (Gebäude, Dokumente) laden
  nach Änderungen anderer Bereiche neu; Formulare weisen auf geänderte
  Projektdaten hin, ohne Eingaben zu überschreiben.
* **Nachweisdruck** führt die aus dem Aufmaß übernommenen Messwerte mit
  Aufmaß, Markierung, Unterlage und Stand auf.

## Offene Punkte

1. **Veraltete Gebäudeaufmaße blockieren GRZ/GFZ aller Baugrundstücke** und das
   Speichern der Grundstückseingaben. Die Meldung nennt jetzt das Gebäude, und
   gestalterische Änderungen lösen den Fall nicht mehr aus. Eine Einbindung,
   die nur das betroffene Baugrundstück sperrt, erfordert je Grundstück
   getrennte Fehler im Vertrag der Gebäudeeinbindung.
2. **Formulare** erhalten weiterhin nur GRZ/GFZ/Geschossflächen/Vollgeschosse;
   Wohnfläche, BGF/BRI, Versiegelung, Entwässerung und Höhen fehlen. Dafür ist
   ein Ergebnisvertrag der Fachberechnungen für den Formularkontext nötig.
3. BMZ übernimmt keine Gebäudedaten; Entwässerung kann Dachflächen mehrerer
   Gebäude nicht in einer Berechnung bündeln.
4. Raumbuch: Es gibt keine Sammelaktualisierung über mehrere Aufmaße; jedes
   Aufmaß wird einzeln erneut übernommen.
5. Weitere Ausbaupunkte des Aufmaßes (Flurstücksflächen, Lagegeometrie,
   Ansichten/Schnitte) siehe [planaufmass.md](planaufmass.md).

## Kompatibilität

Keine Datenmigration und keine Änderung der Datentypen oder generierten
TypeScript-Verträge. Regeländerungen, jeweils in
[planaufmass.md](planaufmass.md) bzw.
[datenfluss-gebaeude-grundstuecke.md](datenfluss-gebaeude-grundstuecke.md)
beschrieben:

* Aktualität über den Bindungsstand statt über die Objektrevision. Archivierte
  Fachnachweise mit PDF-Verknüpfungen gelten einmalig als neu zu prüfen.
* Abstandspläne mit PDF-Bindungen erhalten eine neue Herkunftsangabe und sind
  einmal mit „Aktuellen Aufmaßstand übernehmen“ zu aktualisieren.
* Einzelgeschoss-Einstufung in der GFZ folgt dem Gebäude.

Die Übernahmeprüfung im Core bleibt maßgebend; die Oberfläche filtert nur vor.
