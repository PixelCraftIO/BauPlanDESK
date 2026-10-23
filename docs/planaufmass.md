# PDF-Aufmaß und verknüpfte Berechnungseingaben

## Umfang dieses Ausbaus

Unter **Unterlagen → PDF-Aufmaß** lassen sich PDF-Dateien aus dem Projekt-DMS
als unveränderte Zeichenunterlagen verwenden. Ein Aufmaß gehört zu genau einer
Dokumentversion und einer Seite. Mehrere Aufmaße können dieselbe Seite verwenden,
beispielsweise Lageplan, Ansicht und Schnitt mit unterschiedlichen Maßstäben auf
einem Sammelblatt.

Enthalten sind:

- Maßstab 1:n oder Kalibrierung einer bekannten Strecke über zwei Punkte,
  je Blatt oder je Ausschnitt.
- Ausschnitte: gespeicherte Sichten auf Teile eines Blattes (Grundriss,
  Lageplan, Schnitt, Ansicht, Detail) mit eigenem Maßstab und, bei Schnitten,
  Ansichten und Details, eigener Bezugslinie für Höhen. Blatt- und
  Ausschnittmodus, Rahmen ziehen und bearbeiten, Abdunkeln des Restblattes.
- Arbeitsfläche über die volle Fensterhöhe, Vollbild (F11), Zoom um den
  Mauszeiger (Strg+Rad, Pinch), Zoomfenster (Z), Verschieben mit Leertaste
  oder mittlerer Maustaste, Ansichtsdrehung in 90°-Schritten, Seitenminiaturen,
  scharfes Nachrastern beim Zoomen, Live-Maße am Cursor, Höhenhilfslinie und
  Höhenvorschau am Höhenpunkt, Maßeingabe per Tastatur (Ziffer tippen: Länge
  in m, optional Winkel; beim Rechteck Breite;Höhe), Beschriftung der
  Markierungen mit Core-Messwerten, Tastaturkürzel (V, M, P, R, K, S, H, Z,
  Entf, Strg+Z/Y/D, Pfeiltasten, Bild ↑/↓, Esc).
- Aufmaß-Aufgaben: Der Core listet je gespeichertem Gebäude und je manueller
  Abstandswand die Zielfelder mit benötigter Bedeutung und Geometrie, erkennt
  verknüpfte Ziele und passende freie Markierungen. „Zeichnen“ belegt Werkzeug,
  Bedeutung, Gebäude, Geschoss und Namen vor; „Übernehmen“ verknüpft die
  Markierung über die bestehenden Übernahmedienste und speichert den
  Verbraucher.
- Polygon, Rechteck, Kreis, Strecke und Höhenpunkt; Eckpunkte verschieben,
  Zeichenaktionen rückgängig machen und wiederholen.
- Benannte Zeichenebenen mit Sichtbarkeit und Bearbeitungssperre.
- Fachliche Kennzeichnung, Grundstücks-, Gebäude- und Geschosszuordnung.
- Endpunkt-, Mittelpunkt- und Kantenfang für extrahierte gerade PDF-Vektoren.
  Rasterpläne bleiben manuell bearbeitbar.
- Flächen, Umfänge, Längen, horizontale Strecken, Höhendifferenzen, absolute
  Höhen bei bekanntem Bezug und Neigungen zur Seitenhorizontalen.
- Versionierte Speicherung, lokale Entwurfssicherung und Konfliktbehandlung
  über die vorhandene Fachobjekt-Schnittstelle.
- Verknüpfung von Messwerten mit vorhandenen Positionen der Fachberechnungen.
- Räume aus Planmarkierungen im Raumbuch anlegen und aktualisieren.
- Gebäudegrundfläche und Geschossflächen in die Gebäudedaten übernehmen;
  Gebäude-/Geschosshöhen mit eindeutigem Höhenbezug wiederverwenden.
- Absolute Gelände-/Wandabschlusshöhen sowie Dachhöhen und -neigungen mit
  vorhandenen manuellen Wänden der Abstandsflächenberechnung verknüpfen.
- Projektübersicht mit offenen Aufmaßangaben und offenen Angaben gespeicherter
  Fachberechnungen, auch vor dem ersten archivierten Nachweis.

Dies ist die erste integrierte Ausbaustufe des umfassenderen grafischen
Projektmodells. Fachliche Kennzeichnung allein erzeugt noch keinen Datensatz
in einem anderen Modul.

## Bedienung und unterstützte Übernahmen

Bedienreihenfolge, Lade-/Aktualisierungszeitpunkte und offene Punkte aller
Übernahmewege: [planaufmass-datenwege.md](planaufmass-datenwege.md).

1. PDF unter **Projektdateien** importieren.
2. **PDF-Aufmaß → Neu anlegen**, Unterlage, Seite und Bezeichnung auswählen.
3. Maßstab setzen oder eine bekannte Strecke kalibrieren. Ein gedruckter
   Maßstab gilt nur, wenn die PDF unverändert im angegebenen Papiermaß vorliegt.
   Liegen mehrere Zeichnungen auf dem Blatt, je Zeichnung mit **Ausschnitt
   anlegen** einen Rahmen ziehen, Art wählen und bei Bedarf einen eigenen
   Maßstab setzen (Schnitte sind oft 1:50 neben einem 1:100-Grundriss).
   Für Schnitte, Ansichten und Details **Bezugslinie setzen**: die ±0,00-Linie
   (oder eine andere Linie bekannter Höhe) anklicken und Höhe sowie
   Höhensystem angeben; der Gebäudenullpunkt aus Gebäude/Geschosse wird als
   Vorschlag angeboten. Höhen sind der lotrechte Abstand zu dieser Linie, der
   Schnitt darf daher auch gedreht auf dem Blatt liegen.
4. Ausschnitt öffnen (Doppelklick auf den Rahmen oder „Öffnen“) und darin auf
   einer sichtbaren, ungesperrten Ebene zeichnen; neue Markierungen gehören
   zum geöffneten Ausschnitt. Am einfachsten über **Aufgaben** im Seitenpanel:
   „Zeichnen“ an einer offenen Zeile setzt Werkzeug, Bedeutung und Zuordnung;
   sonst Bedeutung und Zuordnung von Hand setzen. Ohne Ausschnitte gilt wie
   bisher der blattweite Höhenbezug (Bezugspunkt, Seitenvertikale).
5. Aufmaß speichern. Gebäude- und Wandziele direkt unter **Aufgaben →
   Übernehmen** verknüpfen (das Gebäude bzw. der Abstandsplan wird dabei
   gespeichert). Für Fachberechnungen eine passende Position anlegen und unter
   **Quellen / Übernahmen → PDF-Aufmaß** das Zielfeld verknüpfen.
6. Die eingesetzten Werte prüfen und die Berechnung speichern. Neue Maskenstände
   werden mit **Aktuellen Aufmaßstand übernehmen** bewusst übernommen.

| Verbraucher | Unterstützte Eingangswerte |
| --- | --- |
| DIN 277 | Rohflächen der Positionen, äußere Volumengrundflächen und Höhen; NUF/TF/VF/KGF aus der Markierung |
| Wohnflächen | Rohflächen, lichte Höhendifferenzen, Abzugsflächen |
| Freiflächen / Versiegelung | Rohflächen der Bilanzpositionen |
| Entwässerung | Rohflächen der Einzugspositionen |
| Vollgeschosse | Geschoss-/Vergleichsflächen, Höhenzonenflächen und lichte Höhen |
| Gebäudehöhen | Absolute Höhenpunkte und horizontale Dachstrecken |
| Raumbuch | Raumflächen; neue Räume aus Raum-/Aufenthaltsraum-/NUF-/TF-/VF-Markierungen |
| Abstandsflächen | Gelände und Wandabschluss an beiden Wandenden, Dachhöhe und Dachneigungen |
| Gebäude / Geschosse | Horizontale Gebäudeprojektion, Geschossflächen nach Außenmaßen, NHN-Nullpunkt, Gelände-/Trauf-/First-/Fundamenthöhen, Fußboden-/Deckenlage und Höhenabmessungen |

In **Gebäude / Geschosse** bieten die Felder *Gebäudegrundfläche* und
*Geschossfläche nach Außenmaßen* direkt **Aus PDF-Aufmaß** an. Angeboten werden
nur Markierungen dieses Gebäudes mit passender Bedeutung (und beim Geschoss mit
passender Geschosszuordnung); findet sich keine, nennt das Feld die nicht
passenden Markierungen und den fehlenden Schritt. Höhen und die Liste aller
Verknüpfungen stehen unter **Höhen und alle Verknüpfungen aus PDF-Aufmaß**.
Gebäude und Geschosse zuerst speichern, damit Markierungen ihre stabilen IDs
wählen können. Soll dieselbe Kontur Gebäudegrundfläche und Geschossfläche
liefern, wird die Markierung im PDF-Aufmaß mit **Duplizieren** kopiert und die
Kopie als *Geschoss – Außenkante* gekennzeichnet. Das Seitenpanel des
PDF-Aufmaßes nennt je Bedeutung die Stelle, an der übernommen wird. Die Bedeutung **Gebäudegrundfläche – horizontale Projektion** ist vom
**Geschoss – Außenkante** getrennt; das EG ist nicht automatisch der gesamte
Gebäudeumgriff. Die bestehende Gebäudeeinbindung versorgt GRZ/GFZ je zugeordnetem
Baugrundstück. Die rechtliche Geschosseinstufung bleibt ausdrücklich offen.

Gebäudehöhen stammen aus Punkten im System `m ü. NHN` (auch `NHN` wird
akzeptiert). Relative Gebäude-/Geschosskoten werden im Core durch Abzug des
Gebäudenullpunkts gebildet. Ein verknüpfter Nullpunkt wird zuerst aufgelöst.
Lichte Höhe, Geschosshöhe und Deckendicke stammen aus gemessenen
Höhendifferenzen. Die fachliche Auswahl des maßgebenden Geländes bleibt beim
Bearbeiter; ein einzelner Punkt bildet kein automatisch gemitteltes Gelände.

Im **Raumbuch → Räume aus PDF-Aufmaß übernehmen** werden Markierungen des
gewählten Gebäudes und Geschosses übernommen. Die Markierungs-ID dient als
stabile Raum-ID. Erneute Übernahmen aktualisieren die Fläche, erhalten aber
Raumnummer, Bezeichnung, Nutzung und Raumhöhe. Änderungen der Klassifikation
sind deshalb im bestehenden Raum bewusst nachzuführen. Ein gelöschter Planraum
wird nicht still im Raumbuch gelöscht; die fehlende Quelle muss geklärt werden.

**Höhenzonen:** Eine Raumfläche lässt sich in den Eigenschaften als „Höhenzone
von Raum …“ kennzeichnen (`Planmarkierung.hoehenzone_von`), z. B. der Streifen
einer Dachschräge zwischen den Linien 1 m und 2 m lichter Höhe. Die Raum-
markierung umfasst dabei die ganze Raumfläche. Die Übernahme legt die Zone
mit Planbindung `raeume.<raum>.hoehenzonen.<zone>.flaeche_m2` im Raum an;
eine früher als eigener Raum übernommene Fläche wird zur Zone und behält ihre
Höhe als kleinste lichte Höhe. Die Wohnflächenübernahme bildet je Zone eine
Position `Raum_Zone` mit der kleinsten lichten Höhe; DIN 277 zählt weiter eine
Position je Raum. Der Bindungsstand enthält `hoehenzone_von` nur, wenn es
gesetzt ist; bestehende Bindungen bleiben aktuell.

Unter **Abstandsflächen → Grundlagen → PDF-Aufmaß verknüpfen** werden bestehende
manuelle Wände als Ziel angeboten. Absolute Höhen benötigen dasselbe
Höhensystem. Bereits vorhandene Gelände-/Oberkantenprofile müssen ausdrücklich
entfernt werden, bevor Einzelhöhen eingesetzt werden. Umrisswände erhalten
weiterhin ihre Geometrie und Eckhöhen vom Gebäudeumriss. Bei abweichenden
Grundstücks-IDs wird die Flurstückszuordnung geprüft.

Die Zuordnung verwendet **Tabelle.Zeilen-ID.Feld**, keine Listenposition.
Eine Quelle darf mehrere Berechnungen versorgen. Mehrere konkurrierende
Quellen für dasselbe Zielfeld werden zurückgewiesen. Grundstück und Gebäude
müssen zwischen Markierung und Zielberechnung übereinstimmen. Ausnahme:
grundstücksbezogene Bilanzen (Freiflächen, Entwässerung, ohne Gebäude) dürfen
Markierungen eines Gebäudes binden, das laut seinen Flurstücken eindeutig auf
ihrer Grundstückseinheit steht (z. B. die Gebäudegrundfläche als Dachfläche).
Das Höhensystem einer Höhenbindung (Gebäude, Gebäudehöhen, Wände) ist das der
Markierung (`Planaufmass::hoehensystem`): die Bezugslinie ihres Ausschnitts,
ohne Ausschnitt der blattweite Höhenbezug – wie bei der Messung selbst.
Die automatische Quellangabe einer Zielzeile nennt Dateiname, Plannummer/Index
(aus den Dokumentdaten), Version, Seite, Aufmaß, Markierung und Aufmaßrevision,
aber keine technischen IDs; mehrere Bindungen derselben Zeile werden als
„Fläche: … | Höhe: …“ zusammengeführt. Aufmaß-Aufgaben schlagen eine Markierung
je Gebäude höchstens einmal vor (nicht für bereits gebundene) und berücksichtigen
bei Höhenmaßen Plausibilität (Deckendicke < 0,60 m) und Bezeichnung.
Eine aktive Gebäudeübernahme muss vor der direkten PDF-Verknüpfung deaktiviert
werden, damit nicht zwei Lieferanten dieselben Eingaben überschreiben.
Die DIN-277-Übernahme benötigt eine explizite NUF/TF/VF/KGF-Kennzeichnung.
Rohaufmaß wird nicht als bereits angerechnete Wohnfläche oder GFZ-Fläche
ausgegeben. Rechtliche Faktoren, Vollständigkeit und Freigaben bleiben beim
jeweiligen Verbraucher.

## Öffentlicher Vertrag und Modulgrenze

`crates/core-lib/src/planaufmass` besitzt Kalibrierung, geometrische Validierung,
Messwerte und die Übernahmeprüfung. Es verwendet keine UI- oder
Datenbankbibliothek. Die PDF-Darstellung und Vektorinterpretation sind im
Frontend-Adapter `planaufmass/pdfGeometry.ts` und `PdfPlan.svelte` gekapselt.
Die Bedienoberfläche berechnet keine weitergegebenen Fachwerte.

- `Planaufmass`: Dokumentbezug, einsbasierte Seite, optionale Kalibrierung/
  Höhenreferenz, Ebenen, identifizierbare Markierungen und optionale
  `ausschnitte`.
- `Planausschnitt`: UUID, Bezeichnung, `art` (`grundriss`, `lageplan`,
  `schnitt`, `ansicht`, `detail`), Rahmen `von`/`bis` in Seitenpunkten,
  optionale eigene `kalibrierung` und optionale `bezugslinie`
  (`Planbezugslinie`: zwei Punkte, `hoehe_m`, `system`; nur für Schnitt,
  Ansicht, Detail). Ohne eigenen Maßstab gilt der Maßstab der Seite.
- `Planmarkierung`: UUID, Ebene, Geometrie, Bedeutung, optionale
  Grundstücks-, Gebäude- und Geschoss-IDs sowie optionale `ausschnitt_id`
  (Ausschnitt, in dem gezeichnet wurde; nicht nur die Lage).
- `Planmesswert`: Messgrößen in m, m² und Grad, jeweils `null`, solange diese
  Größe nicht bestimmbar oder für die Geometrie nicht vorhanden ist.
- `Planbindung`: Aufmaß-ID, ausdrücklich übernommene Revision, Markierungs-ID,
  Messgröße und stabiles Zielfeld.
- `Planaufmass::messen`: reine Geometrieprüfung und Ermittlung.
- `CoreServices::planaufmass_pruefen`: projektbezogene Quellen-/Identitätsprüfung.
- `CoreServices::planaufmass_einsetzen`: geprüfte Entwurfsübernahme ohne Speicherung.
- `CoreServices::planaufmass_raumbuch`: Planräume im Raumbuch anlegen/aktualisieren.
- `CoreServices::planaufmass_abstand`: Höhen und Neigungen in Wandfelder einsetzen.
- `CoreServices::planaufmass_gebaeude`: Flächen und Höhen in Gebäudedaten einsetzen.
- `planziele`: Positivliste fachlich unterstützter Zielfelder.
- `abstandsziele`: entsprechende Positivliste für manuelle Wände.
- `gebaeudeziele`: entsprechende Positivliste für Gebäude und explizite Geschosse.
- `CoreServices::planaufmass_aufgaben`: `Planaufgabe` je Zielfeld der
  gespeicherten Gebäude und manuellen Wände (Verbraucher, Objekt, Gruppe,
  Zielfeld, Messgröße, geforderte `Planbedeutung`, `Aufgabengeometrie`
  `flaeche`/`hoehenpunkt`/`strecke`, Zuordnungen, `wert_vorhanden`,
  `gebunden`, passende freie `markierung`, Hinweis). Reine Auskunft; die
  Übernahme läuft über `planaufmass_gebaeude`/`planaufmass_abstand` und das
  Speichern des Verbrauchers.
- `Planbedeutung` um `nullpunkt`, `fussboden`, `decke`, `fundamentsohle` und
  `hoehenmass` erweitert, damit Gebäudehöhen eine eigene Bedeutung tragen.
  Bestehende Markierungen sind unberührt; die Übernahmeprüfung der Höhen
  verlangt weiterhin Höhenpunkt und Höhensystem, nicht eine bestimmte
  Bedeutung.

Koordinaten sind physische PDF-Seitenpunkte (72 Punkte = 1 Zoll) der sichtbaren,
gedrehten Seite ab links oben; y wächst nach unten. Der PDF.js-Viewport
berücksichtigt CropBox, Rotation und UserUnit. Zoom, Vollbild und
Bildschirmauflösung beeinflussen gespeicherte Punkte nicht. Kreise bleiben
echte Kreise und werden im Core analytisch berechnet. Ohne Ausschnitt ist die
absolute Höhe `Bezugshöhe + (Bezug.y − Punkt.y) × MeterProPunkt`. Innerhalb
eines Ausschnitts mit Bezugslinie ist sie `Linienhöhe + lotrechter Abstand zur
Linie × MeterProPunkt`; „oben“ ist die Seite der Linie mit kleinerem y.
Strecken liefern Höhendifferenz und Horizontale ebenfalls bezogen auf die
Bezugslinie. Ein Ausschnitt ohne Bezugslinie liefert keine absoluten Höhen
(`offen`), ein Grundriss/Lageplan nie; der blattweite Höhenbezug gilt nicht
innerhalb eines Ausschnitts. Markierungen außerhalb ihres Rahmens werden als
`offen` gemeldet, nicht verschoben. Live-Maße am Cursor sind Vorschauen des
Frontends; Beschriftungen und Panelwerte stammen aus `planaufmass_pruefen`.

Tauri stellt `planaufmass_pruefen`, `planaufmass_ziele` und
`planaufmass_einsetzen` bereit. HTTP bietet dieselben Operationen unter
`POST /projekte/{id}/planaufmass`, `POST /berechnung/planaufmass/ziele`
und `POST /projekte/{id}/planwerte`.
Für Raumbuch und Wände kommen die Tauri-Commands `planaufmass_raumbuch`,
`planaufmass_abstand`, `planaufmass_abstandsziele` sowie HTTP
`POST /projekte/{id}/planraumbuch`, `POST /projekte/{id}/planabstand`
und `POST /berechnung/planaufmass/abstandsziele` hinzu.
Gebäude verwenden `planaufmass_gebaeude` / `planaufmass_gebaeudeziele`
bzw. `POST /projekte/{id}/plangebaeude` (Payload: `id`, `gebaeude`) und
`POST /berechnung/planaufmass/gebaeudeziele`. Aufgaben liefert
`planaufmass_aufgaben` bzw. `POST /projekte/{id}/planaufgaben` (Payload:
das Aufmaß).
Speichern erfolgt über die vorhandenen Fachobjekt-Endpunkte mit
`ObjektArt::Planaufmass`. Die tatsächliche PDF-Seite wird an der Speichergrenze
über den bestehenden PDF-Vertrag geprüft.

## Kompatibilität, Herkunft und Aktualität

Keine Datenbankmigration und keine automatische Neuberechnung beim Öffnen.
Bestehende PDF- und DXF-Funktionen bleiben erhalten. `ObjektArt` erhält die
neue Variante `planaufmass`; generierte TypeScript-Verträge werden mitgeführt.
`FachPlan.planbindungen`, `Raumbuch.planbindungen`, `Gebaeude.planbindungen`
und `AbstandsPlan.planbindungen`
sind optional und werden ohne Inhalt nicht serialisiert.
Alte Projekte und Vorlagen bleiben lesbar.

Rechenstand: `planaufmass-1`; Fachdatenschema: 1. Messwerte werden aus Geometrie
und Kalibrierung neu ermittelt, nicht aus vom Frontend gelieferten Ergebnissen.
Quelltexte enthalten Dokumentversion, Seite, Aufmaß- und Markierungsbezeichnung
sowie die übernommene Maskenrevision. Die Verknüpfung selbst trägt stabile IDs.

Die Übernahme weist fremde Projekte, fehlende Markierungen, inkompatible Größen,
andere Einheiten, geänderte gebundene Markierungen und neuere PDF-Versionen zurück.
Maßgebend ist der **Bindungsstand** (`bindungsstand`): Unterlage und Version,
Seite, Kalibrierung, Höhenbezug, Aufmaßbezeichnung sowie Kontur, Bezeichnung,
Bedeutung und Zuordnung der gebundenen Markierung. Ist das Aufmaß seit der
Bindung neu gespeichert, vergleicht der Core diesen Stand der gebundenen mit der
aktuellen Revision (`fachverlauf`). Ebenen, Sichtbarkeit, Sperren und andere
Markierungen lassen eine Bindung gültig; nur die gebundene Markierung wird
geprüft, sodass eine andere, nicht mehr passende Markierung desselben Aufmaßes
keine Übernahme blockiert. Berechnungs-Fingerprints enthalten je Bindung deren
Bindungsstand und die neueste PDF-Version. Ein alter Nachweis bleibt unverändert
archiviert, wird aber bei geänderter Quelle als veraltet erkannt.

*Regeländerung:* Archivierte Fachnachweise mit PDF-Verknüpfungen aus dem
vorherigen Stand erhalten dadurch einmalig einen neuen Fingerprint und gelten
als neu zu prüfen. Wandherkunft (`hoehenquelle`) führt nun alle Bindungen einer
Wand mit Aufmaß- und Markierungsbezeichnung statt nur der letzten mit IDs; bereits
gespeicherte Abstandspläne mit PDF-Bindungen sind einmal mit „Aktuellen
Aufmaßstand übernehmen“ zu aktualisieren. Fehlermeldungen nennen Gebäude, Raum
bzw. Markierung. Der Nachweisdruck führt die übernommenen Messwerte mit Aufmaß,
Markierung, Unterlage und Stand auf.
Diese Prüfung gilt auch für indirekte Verbraucher des Raumbuchs sowie
Wandnachweise. Aktuelle Raumbuchberichte verwenden keine veralteten Planflächen.
Die Projektübersicht führt veraltete Raumquellen und offene Wandangaben auf.
Gebäudeauswertung und Gebäudeeinbindung prüfen ebenfalls den Quellstand;
zugeordnete GRZ-/GFZ-Nachweise und Berechnungen mit Gebäudeübernahme werden bei
veralteten Gebäudeaufmaßen ungültig. Betroffene Gebäude sind in der Übersicht
verlinkt. Rein manuelle Gebäude ohne Planbindungen verhalten sich unverändert.

Miniaturen wechseln nur die betrachtete Seite. Eine andere Arbeitsseite oder
PDF erhält nach bewusster Entscheidung eine eigene Maske. Die Übertragung über
den bestätigten Planvergleich erzeugt neue IDs und verlangt eine neue
Kalibrierung; die ursprüngliche Maske und ihre Bindungen bleiben erhalten.
`ausschnitte`, `ausschnitt_id` und Ausschnitt-Ebenenbezüge sind optional;
Aufmaße ohne Ausschnitte rechnen unverändert. Der Bindungsstand enthält den
Ausschnitt der gebundenen Markierung, sodass eine geänderte Bezugslinie nur die
daraus übernommenen Werte veraltet. Alte Dokumentversionen werden
niemals überschrieben. Ebenensperren sind eine Bedienhilfe, keine zusätzliche
Berechtigungsgrenze.

## Verbleibende Ausbaupunkte

Für das beschriebene Gesamtziel fehlen noch eigenständige Verbraucheradapter:

- Flurstücks- und private Grünflächen direkt in Grundstücksinformationen und
  Baugrundstücksermittlung übernehmen; Identität und rechtliche Flächenabzüge
  ausdrücklich erhalten.
- Vorschläge für Ausschnitte, Maßstab und Höhenkoten aus dem PDF-Text,
  Lotfang und eine Werkzeugkiste mit Vorlagen;
  Aufgaben für Fachberechnungen (Positionen) und Raumbuch; siehe
  [planaufmass-editor-analyse.md](planaufmass-editor-analyse.md).
- Geschoss-Außenkonturen zusätzlich als transformierte Lagegeometrie für
  Gebäudeumrisse und Wandkorrespondenzen bereitstellen. Die Flächenübernahme
  in Gebäude/Geschosse ist vorhanden; räumliche Planausrichtung noch nicht.
- Wände in Ansichten mit Lageplanwänden korrespondieren lassen; Gelände- und
  Oberkantenprofile mit Stationsbezug in die Abstandsflächen übernehmen.
  Eine Ansichtslänge darf nicht still eine Lageplankoordinate ersetzen.
- Fachliche Transformation zwischen mehreren Plänen; gedrehte lokale
  Höhenbezüge und manueller Versionsvergleich sind vorhanden.
- Automatische Prüfung überlappender Bilanzflächen; Polygonlöcher und
  Flächenverschneidung sind vorhanden.
- CAD-artige Tangenten-/Kreiszentrumserkennung, vollständige
  Auswertung von Clipping und optionalen PDF-Inhaltsebenen. Kurven liefern
  derzeit Endpunkte, keine vorgetäuschten geraden Kanten. Fangziele sind am
  sichtbaren Plan zu kontrollieren; Alt deaktiviert den Fang temporär.
- Übernahme durch Auswahl einer Bedeutung allein, automatische Positionsanlage
  außerhalb des Raumbuchs. Annotierte Plananlagen können als PDF ausgegeben werden.

Die zentrale Übersicht kennt gespeicherte Berechnungen und Anforderungen.
Sie behauptet keine Vollständigkeit für noch nicht angelegte Nachweise oder
nicht erfasste projektspezifische Pflichten.

## Prüfungen

- Core-Modultests: Kalibrierung, Kreis-/Polygonfläche, Orientierung, Höhen,
  ungültige Geometrie, Nullwerte, Serialisierung und Altvorlagen; Ausschnitte
  mit eigener Bezugslinie je Schnitt, eigenem Maßstab, gedrehter Bezugslinie,
  fehlenden Bezügen, Rahmenprüfung und Serialisierung ohne Inhalt.
- Storage-Integration: reale PDF-/Maskenspeicherung, DIN-277-Übernahme,
  Versionskonflikt, Projekttrennung, falsche Ziel-ID, falsche Messgröße,
  geänderte Maske/PDF, Originalschutz und zentrale offene Punkte.
- Raum-/Wandintegration: Raumidentität bei erneuter Übernahme, erhaltene
  Raumnummern und Höhen, Höhensystemprüfung, indirekte Wohnflächenverbraucher,
  veraltete Wandnachweise und unveränderte Wiederholung archivierter Ergebnisse.
- Gebäudeverbraucher: separate Projektion und Geschossfläche, NHN-Umrechnung,
  GRZ-Einbindung sowie Sperre veralteter Gebäude- und Höhenübernahmen.
- JavaScript: PDF-Transformationen/Form-XObjects, Kurven ohne falsche Sehnen,
  Fangtoleranz, End-/Mittel-/Kantenfang und Umfangsbegrenzung.
- Browser-Test `apps/desktop/tests/planaufmass.py`: echte PDF-Darstellung mit
  kontrolliertem IPC-Testtransport; Zeichnen, Ebenen, Sperren, Rückgängig,
  Kalibrierung, Höhenbezug, Zoomkoordinaten, Strg+Rad-Zoom, Vollbild,
  Aufgaben (Vorbelegung und Übernahme in ein Gebäude), Maßeingabe,
  Zoomfenster, Drehung mit Seitenkoordinaten, Ausschnitt mit Bezugslinie und
  Höhenpunkt, Tastenkürzel, Speichern und Seitenwechsel über Miniaturen.
- Storage-Integration: Aufgaben mit gebundenen Zielen, vorgeschlagener freier
  Markierung und offener Bedeutung für Gebäude und Wand.
  Fachwerte werden durch die Core-Tests geprüft.

PDF-Adapterreferenz: [PDF.js API](https://mozilla.github.io/pdf.js/api/draft/api.js.html).
Maßgebend für den implementierten Adapter ist die im Projekt gebundene
`pdfjs-dist`-Version 4.10.38.
