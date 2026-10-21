# PDF-Zeicheneditor: Analyse und Zielbild für den Büroeinsatz

Stand: Oktober 2026, nach PR #57. Betrachtet werden `PdfPlan.svelte`,
`Planaufmass.svelte`, `pdfGeometry.ts` und der Core-Vertrag `Planaufmass`.
Ziel: Ein Architekt kann aus einem Plansatz (Grundrisse, Schnitte, Ansichten,
oft mehrere auf einem Blatt) alle Werte abgreifen, die Berechnungen und
Nachweise brauchen, ohne Umwege und ohne Rechnen von Hand.

## 1. Befund: Was heute bremst

### Arbeitsfläche

- Der Editor liegt als Formularteil in der Fachliste: Kopfzeile, Projektkopf,
  Register, Listentabelle, Quellzeile, Kalibrierung, Werkzeugleiste und erst
  dann die Zeichenfläche mit fester Höhe (`65vh`, Seitenpanel 300 px). Auf
  einem 1080p-Monitor bleiben rund 500 px Höhe für den Plan; bei 1440p etwa
  die Hälfte des Bildschirms. Die Seite scrollt zusätzlich außen, sodass
  Werkzeuge und Plan auseinanderlaufen (sichtbar im zweiten Bildschirmfoto:
  Werkzeugleiste abgeschnitten).
- Zoom nur über `−`/`+`/`Einpassen` in 25-%-Schritten, immer um den
  Fensterursprung, nicht um den Mauszeiger. Kein Mausrad-Zoom, kein
  Pinch, kein Zoomfenster, kein 1:1.
- Verschieben nur mit mittlerer Maustaste oder dem Werkzeug „Verschieben“;
  keine Leertaste, kein Scrollen mit Rad in beide Richtungen.
- Die Seite wird einmal als PNG mit maximal 2200 px Kantenlänge gerastert.
  Ab etwa 150 % Zoom wird ein A1/A0-Plan unscharf; genau dort, wo man Ecken
  trifft, sieht man Pixel.
- Keine Tastaturkürzel für Werkzeuge, kein `Entf`, kein Ziehen des gesamten
  Objekts, keine Pfeiltasten zum Feinverschieben, keine Mehrfachauswahl.
- Keine Seitenübersicht (Miniaturen), keine Drehung der Ansicht.

### Messen

- Beim Zeichnen wird keine Länge oder Fläche angezeigt; erst nach dem
  Abschließen steht der Wert rechts im Panel. Markierungen tragen im Plan nur
  ihren Namen, keine Bemaßung.
- Fang nur an Endpunkten, Mittelpunkten und Kanten extrahierter PDF-Vektoren;
  kein Fang an eigenen Markierungen, keine Schnittpunkte, kein Orthogonalmodus
  (Shift), kein Lot.
- Ein Aufmaß hat genau **eine** Kalibrierung und **einen** Höhenbezug je Seite.
  Absolute Höhe = `Bezugshöhe + (Bezug.y − Punkt.y) × MeterProPunkt`, gemessen
  entlang der Seitenvertikalen. Liegen Grundriss, vier Schnitte und ein
  weiterer Schnitt auf einem Blatt (wie im Beispielplan), gilt der eine
  Nullpunkt für alle Schnitte, obwohl jeder Schnitt seine eigene ±0,00-Linie an
  anderer Stelle hat. Der Nullpunkt stimmt dann nur für einen Schnitt; alle
  anderen liefern falsche Höhen, ohne dass der Editor das merkt.
- Im Grundriss ist der Höhenpunkt sinnlos, wird aber angeboten.
- Der Workaround „je Schnitt ein eigenes Aufmaß derselben Seite“ ist zulässig
  (mehrere Aufmaße dürfen dieselbe Seite verwenden), aber mühsam: man wechselt
  ständig den Datensatz, sieht die anderen Markierungen nicht und verliert den
  Zusammenhang eines Blattes.

### Zuordnen

- Jede Markierung entsteht als „Markierung n · Noch nicht zugeordnet“. Bedeutung,
  Baugrundstück, Gebäude und Geschoss werden danach einzeln gesetzt. Für ein
  Gebäude mit drei Geschossen, Traufe, First, Gelände und Nullpunkt sind das
  rund 30 Klicks in Auswahllisten, bevor eine Übernahme möglich ist.
- Der Editor weiß nicht, was das Projekt noch braucht. Welche Felder eines
  Gebäudes, einer Wand oder einer Berechnung offen sind, erfährt man erst im
  jeweiligen Verbraucher.

## 2. Wie große Anbieter das lösen

Verglichen wurden Bluebeam Revu (Nemetschek), Adobe Acrobat Messwerkzeuge,
Autodesk Build/PlanGrid, Fieldwire, PlanRadar sowie die PDF/DWG-Unterlegung
in Archicad, Allplan und Revit. Die Muster sind über alle Produkte sehr ähnlich:

| Thema | Übliche Lösung |
| --- | --- |
| Arbeitsfläche | Die Zeichenfläche ist das Fenster. Alle Paletten sind andockbar und einklappbar; Vollbild mit `F11`; Eigenschaften rechts, Werkzeuge links oder oben, Seiten als Miniaturen. |
| Navigation | Mausrad zoomt um den Mauszeiger (Acrobat/Bluebeam: `Strg`+Rad, Web-Viewer meist Rad direkt), Rad scrollt, `Shift`+Rad horizontal, Leertaste oder mittlere Taste verschiebt, Doppelklick auf Rad = Einpassen, Zoomfenster, Zoom auf Markierung. Pinch auf Touchpad/Tablet. |
| Darstellung | Rendern in der aktuellen Zoomstufe (gekachelt oder nach Zoomende neu), nie ein festes Rasterbild. |
| Maßstab | **Je Ausschnitt** („Viewport“ in Revu seit Version 20): Auf einer Seite werden Rechtecke definiert, jedes mit eigenem Maßstab und eigener Ausrichtung. Markierungen übernehmen Maßstab des Ausschnitts, in dem sie liegen. Kalibrieren über zwei Punkte einer bekannten Maßkette, Vorschlag aus dem Schriftfeld („1:100“). |
| Höhen in Schnitten | CAD-Logik: Jede Ansicht/jeder Schnitt hat eine Bezugslinie (±0,00 oder OK FFB). Höhen sind senkrechte Abstände zu dieser Linie, umgerechnet mit dem Maßstab des Ausschnitts, plus Höhe der Bezugslinie (relativ oder absolut, z. B. „±0,00 = 72,64 m ü. NHN“). Mehrere Schnitte auf einem Blatt haben mehrere Bezugslinien. |
| Messen | Länge/Fläche/Winkel werden während des Zeichnens am Cursor angezeigt, nach Abschluss als Beschriftung am Objekt; Flächen mit Abzugsflächen (Löcher). |
| Fang | Endpunkt, Mittelpunkt, Schnittpunkt, Lot, Kante, eigene Markierungen, Orthogonal mit `Shift`, temporäres Aus mit `Alt`. |
| Vorlagen | „Tool Chest“/Werkzeugkiste: vorbelegte Messwerkzeuge mit Bedeutung, Ebene, Farbe und Namen (z. B. „Geschossfläche EG“). Ein Klick wählt Werkzeug und Zuordnung zugleich. |
| Mengen | Markierungen landen in einer Liste mit Gruppen und Summen; Kopplung an Kalkulation oder Formulare über eindeutige Kennungen, nicht über Listenpositionen. |
| Tastatur | Ein-Buchstaben-Kürzel für Werkzeuge, `Esc`, `Enter`, `Entf`, `Strg+Z/Y`, `Strg+D` (Duplizieren), Pfeiltasten zum Feinverschieben. |

Das Muster „Ausschnitt mit eigenem Maßstab und eigener Bezugslinie“ ist die
Antwort auf die Frage, wie Grundriss und mehrere Schnitte auf einem Blatt
zusammengehen. Ein einzelner Nullpunkt je Seite reicht nicht und wird von
keinem der Produkte so umgesetzt.

## 3. Zielbild für BauPlanIO

### 3.1 Planausschnitte (Vertragsänderung, Kern des Ausbaus)

Neuer Typ im Core-Vertrag `Planaufmass`:

```text
Planausschnitt
  id              UUID
  bezeichnung     "Grundriss EG", "Schnitt B-B"
  art             grundriss | schnitt | ansicht | lageplan | detail
  bereich         Rechteck in Seitenpunkten
  kalibrierung    optional; sonst gilt die Seitenkalibrierung
  drehung_grad    optional; 0 = Seitenvertikale ist lotrecht
  hoehenbezug     optional, nur schnitt/ansicht:
                    linie (zwei Punkte, i. d. R. waagerecht), hoehe_m, system
```

- `Planmarkierung` erhält optional `ausschnitt_id`. Beim Zeichnen wird sie aus
  der Lage des ersten Punktes bestimmt und angezeigt; sie bleibt änderbar.
- `Planaufmass::messen` verwendet Maßstab und Höhenbezug des Ausschnitts, sonst
  die der Seite. Ohne Ausschnitte verhält sich alles wie heute: **keine
  Datenmigration, alte Aufmaße bleiben gültig**.
- Höhen werden senkrecht zur Bezugslinie gemessen, nicht zur Seitenvertikalen.
  Damit sind auch gedreht platzierte Schnitte korrekt. Ein Höhenpunkt im
  Grundriss ist ungültig (`offen`: „Ausschnitt ohne Höhenbezug“); das Werkzeug
  ist dort ausgegraut.
- `bindungsstand` nimmt den Ausschnitt der gebundenen Markierung mit auf
  (Maßstab, Bezugslinie, Drehung), damit Nachweise veralten, wenn der
  Bezug eines Schnitts geändert wird, aber nicht bei Änderungen an einem
  anderen Ausschnitt.
- Die Projektübersicht meldet Ausschnitte ohne Maßstab und Schnitte ohne
  Höhenbezug.

Bedienung am Beispielplan:

1. „Ausschnitt anlegen“: Rechteck um den Grundriss ziehen, Art *Grundriss*,
   Name vorschlagen aus nächstem PDF-Text („GRUNDRISS OBERGESCHOSS“).
2. Dasselbe für jeden Schnitt. Maßstab: Blattmaßstab übernehmen oder je
   Ausschnitt kalibrieren (Schnitte sind oft 1:50 neben einem 1:100-Grundriss).
3. Je Schnitt „Bezugslinie setzen“: die ±0,00-Linie anklicken (Fang auf
   waagerechte Vektorkante), Höhe eintippen oder aus dem Text daneben
   übernehmen („+0,00 = 72,64 NHN“ steht im Plan). Der Gebäudenullpunkt aus
   Gebäude/Geschosse wird als Vorschlag angeboten.
4. Höhenpunkte (Traufe, First, Gelände, OK Decke) in jedem Schnitt setzen. Die
   Höhe ergibt sich aus dem Abstand zur Bezugslinie **dieses** Schnitts; wo der
   Schnitt auf dem Blatt liegt, spielt keine Rolle mehr.
5. Strecken innerhalb eines Schnitts liefern Höhendifferenzen (lichte Höhe,
   Deckendicke), wie heute.

Ergänzend ein Werkzeug **Höhenlinie**: eine waagerechte Kante anklicken
(Traufe, Attika, OK Decke) statt eines Punktes. Das vermeidet das Zielen auf
einen Pixel und passt zu Zeichnungen, in denen Höhen als Linien vorliegen.

### 3.1a Ausschnitte verwalten und darin arbeiten

Ein Ausschnitt ist eine **gespeicherte Sicht auf dieselbe Seite**, keine
herausgeschnittene PDF. Die Original-PDF bleibt unverändert und einmalig im
DMS, alle Markierungen bleiben Seitenpunkte, der Fang an PDF-Vektoren und die
Versionsprüfung funktionieren wie heute. Eine physische Kopie je Schnitt würde
Dokumente vervielfachen, Koordinaten zwischen Kopie und Blatt auseinanderlaufen
lassen und bei einer neuen PDF-Version jede Kopie einzeln ungültig machen.

**Verwaltung** im Seitenpanel als eigener Abschnitt oberhalb der Ebenen:

| Spalte | Inhalt |
| --- | --- |
| Name, Art | „Schnitt B-B“, Schnitt |
| Maßstab | 1:50, „vom Blatt“ oder „fehlt“ |
| Höhenbezug | „±0,00 = 72,64 m ü. NHN“ oder „fehlt“ (nur Schnitt/Ansicht) |
| Markierungen | Anzahl, davon ohne Bedeutung |
| Aktionen | Öffnen, Rahmen bearbeiten, Duplizieren, Entfernen |

Dazu ein **Blattmodus** und ein **Ausschnittmodus**:

- *Blatt*: Die ganze Seite mit allen Ausschnittrahmen und ihren Namen. Hier
  werden Rahmen gezogen, verschoben und benannt. Klick in einen Rahmen oder
  auf die Zeile öffnet den Ausschnitt.
- *Ausschnitt*: Die Ansicht zoomt auf den Rahmen und dunkelt den Rest des
  Blattes ab (nicht ausblenden: Maßketten und Beschriftungen am Rand bleiben
  lesbar). Es gelten Maßstab, Drehung und Bezugslinie des Ausschnitts; nur
  seine Markierungen werden angezeigt, andere grau. Neue Markierungen gehören
  automatisch zu diesem Ausschnitt. Brotkrume „Blatt 1 › Schnitt B-B“, `Esc`
  oder Klick auf „Blatt“ führt zurück, `Bild ↑/↓` wechselt zum nächsten
  Ausschnitt.
- Eine Zeichenebene je Ausschnitt wird automatisch angelegt („Schnitt B-B“),
  damit die Ebenenliste nicht von Hand gepflegt werden muss.

**Ausschnitte vorschlagen**: Aus dem PDF-Text lassen sich Titel wie
„SCHNITT B-B“ oder „GRUNDRISS ERDGESCHOSS“ erkennen; aus den extrahierten
Vektoren ergeben sich zusammenhängende Zeichnungsbereiche mit Leerraum
dazwischen. Daraus entstehen Rahmenvorschläge mit Namen und Art, die der
Bearbeiter übernimmt, verschiebt oder verwirft. Gerasterte Scans erhalten keine
Vorschläge; dort zieht man die Rahmen von Hand.

**Regeln und Randfälle**:

- Ein Blatt ohne Ausschnitte verhält sich wie heute. Ausschnitte sind
  optional; für ein Blatt mit nur einem Grundriss lohnt sich keiner.
- Die Zugehörigkeit einer Markierung ist der Ausschnitt, der beim Zeichnen
  aktiv war, nicht nur die Lage. Überlappende Rahmen sind dadurch zulässig
  (ein Detail innerhalb eines Grundrisses). Liegt eine Markierung außerhalb
  ihres Rahmens, meldet der Core dies als offenen Punkt, verschiebt sie aber
  nicht still.
- Rahmen bearbeiten verschiebt keine Markierungen; Koordinaten sind
  Seitenpunkte. Ausschnitt entfernen fragt nach, wenn Markierungen daran
  hängen; sie bleiben als „ohne Ausschnitt“ erhalten und rechnen mit den
  Blattwerten, Höhenpunkte werden dann „offen“.
- Ohne eigenen Maßstab gilt der Blattmaßstab; ein Schnitt ohne Bezugslinie
  erlaubt Strecken (Differenzen), aber keine absoluten Höhen.
- Neue PDF-Version: Rahmen bleiben als Vorschlag erhalten, Maßstab und
  Bezugslinien werden wie heute zurückgesetzt und müssen bestätigt werden.
- Die Verbraucher zeigen die Herkunft als „Aufmaß › Ausschnitt › Markierung“;
  `bindungsstand` enthält den Ausschnitt, sodass eine geänderte Bezugslinie
  in Schnitt B-B nur die daraus übernommenen Werte veraltet.
- Ein Aufmaß bleibt an eine Seite gebunden. Mehrseitige Plansätze erhalten je
  Seite ein Aufmaß mit eigenen Ausschnitten; die Seitenminiaturen (3.2)
  verbinden sie. Bestehende Aufmaße, die als Behelf „je Schnitt eines“ angelegt
  wurden, bleiben gültig; eine Zusammenführung ist eine bewusste Entscheidung,
  keine automatische Migration.

Gegenüber dem heutigen Behelf (mehrere Aufmaße derselben Seite) gewinnt man:
ein Datensatz je Blatt, alle Markierungen im Zusammenhang, je Schnitt eigener
Maßstab und Nullpunkt, und ein Arbeitsbereich, der nur den relevanten Teil des
Plans groß zeigt.

### 3.2 Arbeitsfläche und Navigation (nur Oberfläche)

- **Vollbildmodus** für den Editor (`F11` oder Knopf): Projektkopf, Register
  und Liste werden ausgeblendet, die Zeichenfläche füllt das Fenster. Die
  Werkzeugleiste bleibt oben, das Eigenschaftenpanel rechts einklappbar
  (Standard 320 px, per Griff verschiebbar). Zurück mit `Esc` oder Knopf.
- Ohne Vollbild: Zeichenfläche nimmt die gesamte verbleibende Höhe
  (`flex: 1` statt `65vh`); Quellzeile, Kalibrierung und Höhenbezug wandern in
  ein einklappbares „Einrichtung“-Panel, das nach dem Setzen zusammenklappt.
- **Zoom um den Mauszeiger** mit `Strg`+Rad und Pinch (Touchpad liefert
  `ctrlKey`+`wheel`); Rad allein scrollt, `Shift`+Rad horizontal. Stufenlos,
  Bereich 10 % bis 800 %. Zusätzlich Zoomfenster (Rechteck aufziehen), „Auf
  Auswahl“, „1:1“ (72 dpi) und „Breite einpassen“.
- **Verschieben** mit Leertaste+Ziehen, mittlerer Taste, Touch mit zwei
  Fingern. Im Werkzeug „Auswählen“ zieht Linksklick auf leerer Fläche ein
  Auswahlfenster, auf einer Markierung verschiebt es die ganze Markierung.
- **Scharfes Rendern**: Seite bei jedem Zoomende neu rastern (PDF.js mit
  `scale = zoom × devicePixelRatio`, auf 4096 px gedeckelt) und nur den
  sichtbaren Bereich zeichnen (Canvas-Kacheln). Während des Zoomens wird das
  vorhandene Bild skaliert, danach ausgetauscht.
- **Seitenleiste** mit Miniaturen aller Seiten; Klick wechselt die Seite und
  legt bei Bedarf ein weiteres Aufmaß derselben Unterlage an.
- **Drehen** der Ansicht in 90°-Schritten für quer gescannte Blätter (reine
  Anzeige; gespeicherte Koordinaten bleiben Seitenpunkte).

### 3.3 Zeichnen und Messen

- Live-Maß am Cursor: Länge des aktuellen Segments, Gesamtlänge, Fläche des
  Polygons, Winkel zur Horizontalen; nach Abschluss Beschriftung am Objekt
  (ein-/ausblendbar).
- Fang erweitern: Schnittpunkte zweier Kanten, Lot auf Kante, eigene
  Markierungen, Bezugslinien; Orthogonal- und 45°-Raste mit `Shift`;
  Fangsymbole unterscheiden (Quadrat Endpunkt, Dreieck Mittelpunkt, Kreuz
  Schnittpunkt), wie in CAD üblich.
- Polygon mit Löchern (Lichthof, Treppenauge): „Abzug zeichnen“ im gewählten
  Polygon. Das ist bereits als offener Punkt in `planaufmass.md` geführt.
- Tastatur: `V` Auswählen, `P` Polygon, `R` Rechteck, `K` Kreis, `S` Strecke,
  `H` Höhenpunkt, `L` Höhenlinie, `Esc`, `Enter`, `Rücktaste` letzter Punkt,
  `Entf` Markierung, `Strg+Z/Y`, `Strg+D` Duplizieren, Pfeiltasten verschieben
  die Auswahl um einen Punkt (mit `Shift` zehn).
- Koordinaten- und Maßeingabe: nach dem ersten Punkt eine Länge und einen
  Winkel tippen (`5,20 Tab 90 Enter`), um ohne Zielen weiterzuzeichnen.

### 3.4 Zuordnen durch Aufgaben statt durch Auswahllisten

Der größte Zeitgewinn liegt nicht im Zeichnen, sondern im Wegfall der
nachträglichen Zuordnung. Vorschlag: ein Panel **Aufmaß-Aufgaben**, das vom
Core gespeist wird.

- Der Core liefert je Gebäude die offenen Zielfelder (es gibt bereits
  `gebaeudeziele`, `abstandsziele`, `planziele`): Gebäudegrundfläche,
  Geschossfläche EG/OG/…, Gebäudenullpunkt, Gelände, Traufe, First,
  Wandabschluss je manueller Wand, Räume je Geschoss. Jede Zeile kennt
  Bedeutung, Geometrieart und Zuordnung (Gebäude, Geschoss, Grundstück, Wand).
- Klick auf eine Zeile wählt das passende Werkzeug und belegt Bedeutung,
  Gebäude, Geschoss, Grundstück und den Namen vor („Lagerhalle · OG ·
  Geschossfläche“). Die Zeichenebene folgt dem Ausschnitt (eine Ebene je
  Ausschnitt wird automatisch angelegt).
- Nach dem Speichern bietet das Panel **„Übernehmen“** je Zeile: Das ist die
  bereits dokumentierte, noch offene „Übernahme durch Auswahl einer
  Bedeutung“. Technisch ein Core-Service, der aus Aufmaß-ID, Markierung und
  Zielfeld eine `Planbindung` bildet und den Verbraucher wie heute
  (`planaufmass_gebaeude`, `planaufmass_abstand`, `planaufmass_einsetzen`)
  einsetzt. Die fachliche Prüfung bleibt vollständig im Core.
- Erledigte Zeilen werden abgehakt; die Projektübersicht zeigt dieselbe Liste.

Zusätzlich eine **Werkzeugkiste** mit Vorlagen (Bedeutung + Ebene + Farbe +
Namensschema), z. B. „DIN 277 NUF“, „Versiegelte Fläche“, „Flurstück“, für
Fälle ohne Gebäudebezug.

### 3.5 Werte aus dem Plan lesen

PDF.js liefert den Seitentext mit Positionen (`getTextContent`). Damit lassen
sich ohne OCR:

- Maßstab aus dem Schriftfeld vorschlagen („M 1:100“),
- Höhenkoten neben einem angeklickten Punkt vorschlagen („+5,56“, „72,64“),
- Namen für Ausschnitte vorschlagen („SCHNITT B-B“),
- Maßketten als Kalibrierhilfe anbieten (Klick auf „12,34“ und beide Enden
  der Maßlinie werden vorgeschlagen).

Es bleiben Vorschläge, die der Bearbeiter bestätigt; gerasterte Scans liefern
nichts und werden wie heute manuell bedient. Die Textextraktion gehört in den
Frontend-Adapter (`pdfGeometry.ts`), nicht in den Core.

## 4. Umsetzungsstufen

**Stand nach Umsetzung:** Stufe A und Stufe B sind umgesetzt (Ausschnitte mit
eigenem Maßstab und Bezugslinie, Blatt-/Ausschnittmodus, Vollbild, Zoom um den
Mauszeiger, Pinch, Leertaste-Pan, scharfes Nachrastern, Live-Maße, Beschriftung
aus Core-Werten, Kürzel, Markierung verschieben, Entf, Pfeiltasten, Shift-Raste,
Fang an eigenen Markierungen und Bezugslinien). Abweichungen zum Entwurf: Statt
`drehung_grad` bestimmt die Richtung der Bezugslinie die Höhenrichtung; eine
Ebene je Ausschnitt wird beim Anlegen erzeugt und mit dem Ausschnitt umbenannt.
Inzwischen ebenfalls umgesetzt: Seitenminiaturen, Ansichtsdrehung in
90°-Schritten (Koordinaten bleiben Seitenpunkte), Zoomfenster, Maßeingabe per
Tastatur, Höhenhilfslinie mit Höhenvorschau am Höhenpunkt (ersetzt das
geplante eigene Werkzeug „Höhenlinie“: der Fang an Kanten plus Hilfslinie
deckt den Fall ab) sowie **Stufe C**: Aufmaß-Aufgaben aus `planaufmass_aufgaben`
mit Vorbelegung beim Zeichnen und Übernahme per Klick für Gebäude und manuelle
Abstandswände. Offen bleiben aus C: Aufgaben für Positionen der
Fachberechnungen und des Raumbuchs (dort müssen die Positionen erst entstehen)
sowie die Werkzeugkiste und Textvorschläge aus Stufe D.



| Stufe | Inhalt | Vertrag | Aufwand |
| --- | --- | --- | --- |
| A | Vollbild, volle Höhe, Zoom um Mauszeiger, Rad/Pinch, Leertaste-Pan, scharfes Rendern, Live-Maße, Kürzel, Objekt verschieben, `Entf` | keine | 3–5 Tage |
| B | Planausschnitte mit eigenem Maßstab, Drehung und Bezugslinie; Höhenlinie; Messung im Core; Übersicht meldet fehlende Bezüge | `Planaufmass.ausschnitte`, `Planmarkierung.ausschnitt_id`, `bindungsstand` | 5–8 Tage |
| C | Aufmaß-Aufgaben aus Zielfeldern, Vorbelegung beim Zeichnen, Übernahme per Klick, Ebene je Ausschnitt | neuer Core-Service „Planaufgaben“ und „Planbindung anlegen“ | 5–8 Tage |
| D | Textvorschläge (Maßstab, Koten, Namen), erweiterter Fang, Polygonlöcher, Seitenminiaturen, Werkzeugkiste | Polygonlöcher ändern `Plangeometrie` | 5–10 Tage |

Stufe A ist sofort möglich und ändert keinen Vertrag. Stufe B löst das
fachliche Problem mehrerer Schnitte je Blatt und sollte vor Stufe C kommen,
weil die Aufgabenliste Höhenziele nur dann sinnvoll anbieten kann, wenn der
Editor weiß, in welchem Schnitt gezeichnet wird.

Bis Stufe B gilt als Zwischenlösung: je Schnitt ein eigenes Aufmaß derselben
Seite mit eigenem Höhenbezug anlegen („Schnitt B-B“, „Schnitt C-C“). Das ist
heute bereits korrekt, nur unbequem.

## 5. Regeln, die bleiben

- Der Core misst; die Oberfläche zeigt Vorschauen. Live-Maße am Cursor sind
  Vorschau und werden beim Speichern vom Core neu ermittelt.
- Koordinaten bleiben Seitenpunkte der gedrehten Seite; Zoom, Ansichtdrehung
  und Bildschirmauflösung verändern gespeicherte Daten nicht.
- Ein Aufmaß gehört zu genau einer Dokumentversion. Ausschnitte und
  Bezugslinien werden mit der Maske versioniert; ein Wechsel der PDF-Version
  entfernt sie wie heute Maßstab und Höhenbezug.
- Textvorschläge aus der PDF sind Bedienhilfe, nie Fachwert ohne Bestätigung.
- Kompatibilität: Aufmaße ohne Ausschnitte rechnen unverändert; neue Felder
  sind optional und werden ohne Inhalt nicht serialisiert.

## 6. Prüfung

- Core: Messung je Ausschnitt (Maßstab, Drehung, Bezugslinie), Höhenpunkt
  außerhalb eines Schnitts, Markierung über Ausschnittgrenze, Altdaten ohne
  Ausschnitte, `bindungsstand` mit Ausschnitt.
- Browser (`tests/planaufmass.py` erweitern): Rad-Zoom um den Cursor,
  Leertaste-Pan, Vollbild und zurück, Kürzel, Live-Maß, Ausschnitt anlegen,
  Bezugslinie setzen, Höhenpunkt in zwei Schnitten mit unterschiedlichen
  Bezugslinien, Aufgabenzeile wählt Werkzeug und Zuordnung vor.
- Storage: Übernahme einer Höhe aus Schnitt B-B in `traufe_m`, Veralten nur
  bei Änderung des eigenen Ausschnitts.
