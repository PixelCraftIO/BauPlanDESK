# IFC-Modelle

BauPlanIO liest IFC-Modelle (ISO 16739) als Unterlage eines Projekts, prüft sie
auf die Angaben eines Bauantrags, lässt sie nacharbeiten und wertet sie für
Flächen und Höhen aus. Konzept und Abwägung: Machbarkeitsprüfung vom 28.10.2026.

## Grundsätze

- Die IFC-Datei bleibt unverändert als Dokumentversion im DMS (`.ifc` im
  STEP-Format oder `.ifczip`; IFC-XML wird abgewiesen).
- Eine **Modellakte** (Fachobjekt `ifcmodell`, `core_lib::ifc::Modellakte`)
  verweist auf diese Dokumentversion.
- Der **Modellauszug** (`core_lib::ifc::Modellauszug`) wird bei Bedarf aus der
  Datei berechnet und nie gespeichert. Die letzten vier gelesenen Modelle hält
  der Core im Speicher (Schlüssel: Dokument und Version, beide unveränderlich).
- Rechtliche Anrechnungen (Vollgeschoss, GRZ/GFZ, Abstandsflächen) bleiben bei
  ihren Fachmodulen. Das Modell liefert Flächen, Höhen und Herkunft.

## Lesen (`core_lib::ifc::step`, `auszug`, `geometrie`)

Der STEP-Leser kennt kein Schema und liest IFC2X3, IFC4 und IFC4X3 gleich.
Unveränderte Instanzen schreibt er beim Export zeichengleich zurück.

Der Auszug enthält:

| Teil | Quelle in der Datei |
| --- | --- |
| Längeneinheit, Flächen- und Volumeneinheit | `IfcProject.UnitsInContext` (`IfcSIUnit` mit Vorsatz, `IfcConversionBasedUnit`) |
| Georeferenz | `IfcMapConversion` + `IfcProjectedCRS`; bei IFC2X3 `IfcSite.RefLatitude/RefLongitude` |
| Grundstücke, Gebäude, Geschosse | `IfcSite`, `IfcBuilding`, `IfcBuildingStorey` (Höhe aus `Elevation`, sonst aus der Platzierung) |
| Elemente | alle Produkte außer Öffnungen, Raster, Beschriftungen; Räume (`IfcSpace`) |
| Geschoss eines Elements | `IfcRelContainedInSpatialStructure`, `IfcRelAggregates`, `IfcRelNests` (über Dach → Dachplatte und Raum → Geschoss) |
| Eigenschaften und Mengen | `IfcRelDefinesByProperties` (Psets, `IfcElementQuantity`); Typ-Eigenschaften über `IfcRelDefinesByType`, Elementwerte gehen vor |

Alle Zahlen sind in SI-Einheiten (m, m², m³), Geometrie in Metern im
Weltsystem des Modells (x, y waagerecht, z nach oben).

### Geometrie

| Darstellung | Auswertung |
| --- | --- |
| `IfcExtrudedAreaSolid` senkrecht mit Rechteck-, Hohlrechteck-, Kreis-, freiem Profil (`IfcPolyline`, `IfcIndexedPolyCurve` mit Bögen, `IfcCompositeCurve`, getrimmte Kreise) | exakt (Prisma) |
| schräge oder gedrehte Extrusion, `IfcExtrudedAreaSolidTapered` | Hülle aus Projektion und Höhenbereich |
| `IfcFacetedBrep`, Shell-/Face-Modelle, `IfcPolygonalFaceSet`, `IfcTriangulatedFaceSet`, `IfcBlock` | Hülle; Volumen aus den Flächen |
| `IfcMappedItem` | mit Abbildungsoperator |
| `IfcBooleanClippingResult`, `IfcBooleanResult` | Vereinigung exakt, Differenz über den ersten Operanden (Näherung, gekennzeichnet) |
| übrige (Rotationskörper, NURBS, I-/L-/T-Profile …) | nicht ausgewertet, im Bericht unter „offene Darstellungen“ |

### Modellbericht

`Modellauszug.bericht` zählt Elemente je Klasse, Elemente mit und ohne
Geometrie, Näherungen, Räume mit Flächenmenge und offene Darstellungsarten.
Hinweise nennen fehlende Einheiten, fehlende Geschosse und doppelte GlobalIds.

## Modellansicht

Im Aufmaß öffnet **IFC-Modell** einen eigenen Tab: Modellakte aus einer
IFC-Datei der Projektdateien anlegen oder wählen, links die Struktur
(Gebäude → Geschoss → Klasse → Element), in der Mitte Grundriss oder 3D,
rechts Eigenschaften und Modellbericht. Der Tab merkt sich die Modellakte.

- **Grundriss** (`ifc::grundriss`, Befehl `ifc_grundriss`): der Core schneidet
  alle Körperteile, deren Höhenbereich die Ebene Geschosshöhe + Schnitthöhe
  (Standard 1,0 m) enthält; Hüllen gehen mit ihrer Projektion ein und sind als
  solche gekennzeichnet. Räume kommen aus ihrem Geschoss. Der Umfang ist der
  des ganzen Modells, damit alle Geschosse deckungsgleich liegen.
- **3D** (`apps/desktop/src/lib/ifc/Ansicht3d.svelte`): web-ifc 0.0.78
  (MPL-2.0, unverändert, Quelle https://github.com/ThatOpen/engine_web-ifc) und
  Three.js (MIT), erst beim Öffnen der Ansicht geladen; `web-ifc.wasm` kopiert
  `scripts/pdf-assets.mjs` nach `public/web-ifc/`. Nur Darstellung und
  Auswahl über die GlobalId; alle Zahlen rechnet der Core. Modelle ohne
  Darstellungsstil werden nach Klasse eingefärbt.
- **Hintergrund einer eigenen Zeichnung**: Ein Geschoss eines IFC-Modells kann
  wie eine DXF als Hintergrund dienen (`Planhintergrund.geschoss`,
  `ifc::als_plan`, Befehl `ifc_grundriss_plan`): eine Zeichnungsebene je
  Klasse und „Räume“, Meter, Fang an allen Kanten. Der Core prüft beim
  Speichern, dass das Geschoss im Modell steht; PDF- und DXF-Hintergründe
  tragen kein Geschoss.

## Prüfung (IDS)

`CoreServices::ifc_pruefen` (Befehl `ifc_pruefen`, Reiter „Prüfung“) prüft
den Auszug der Modellakte gegen eine Information Delivery Specification
(IDS 1.0) und die eingebauten Regeln.

- Mitgeliefert: `regelwerke/ifc/bauantrag.ids` (Gebäude und Geschosse mit
  Namen, Räume mit Nutzung, Wände mit `Pset_WallCommon.IsExternal`, Platten
  mit Art). Ein eigenes IDS des Büros (`.ids` in den Projektdateien) kann
  stattdessen gewählt werden.
- Ausgewertet werden die Facetten `entity` (mit `predefinedType`;
  `…StandardCase` zählt zur Grundklasse), `attribute` (Name, LongName,
  ObjectType, PredefinedType, GlobalId) und `property` (Eigenschaften und
  Mengen) mit `simpleValue`, Aufzählung und Zahlengrenzen, die
  Kardinalitäten `required`/`prohibited` und `minOccurs`/`maxOccurs`.
  Muster, `classification`, `material` und `partOf` erscheinen unter „Nicht
  geprüft“, Spezifikationen für ein anderes Schema ebenso.
- Stufe aus dem Kennzeichen der Spezifikation: `W-…` Warnung, `H-…` Hinweis,
  sonst Fehler.
- Eingebaute Regeln: fehlende Einheiten und doppelte GlobalIds (Fehler),
  keine Geschosse (Fehler), Elemente ohne Geschoss, unklassifizierte Bauteile
  (Proxy), Elemente ohne auswertbare Geometrie (Aggregate mit Teilen zählen
  nicht), Geschosse ohne Räume, Raummengen (`NetFloorArea`/`GrossFloorArea`),
  die mehr als 0,5 m² und 2 % von der Geometrie abweichen (Warnungen), keine
  Georeferenz, Näherungen, offene Darstellungen, Räume ohne Namen und
  Geschosshöhen außerhalb 2–6 m (Hinweise).
- Jeder Befund nennt die Elemente (im Grundriss und in 3D hervorhebbar) und
  die passende Nacharbeit (klassifizieren, Geschoss zuordnen, Eigenschaft
  setzen, Raum bilden, benennen).

## Nacharbeit

Importierte Modelle brauchen fast immer Nacharbeit. Sie steht als Liste
`Modellakte.nacharbeit` (`Modellaenderung`) in der Modellakte, gebunden an
die GlobalId, und wird in dieser Reihenfolge auf den gelesenen Auszug
angewendet (`ifc::nacharbeit::anwenden`, `CoreServices::ifc_akte_auszug`).
Die IFC-Datei bleibt unverändert; Prüfung, Übernahme, Auswertung und Export
sehen das nachgearbeitete Modell.

| Änderung | Wirkung |
| --- | --- |
| `klassifizieren` | Klasse und Art (`PredefinedType`), z. B. Proxy → Wand, Platte → Dach; Zielklassen `nacharbeit::KLASSEN` |
| `geschoss_zuordnen` | Element einem Geschoss (und dessen Gebäude) zuordnen |
| `eigenschaft_setzen` | Eigenschaft mit Typ Text, Zahl oder Ja/Nein setzen oder ersetzen |
| `benennen` | Name und Langname (Räume: Nummer und Nutzung), auch Geschosse und Gebäude |
| `raum_bilden` | neuer Raum (neue GlobalId) aus einem Umriss in Modellkoordinaten, Höhe bis zur nächsten Geschossebene oder angegeben |
| `ausblenden` | Element aus Prüfung, Übernahme und Auswertung nehmen |

**Räume aus Wänden** (`ifc::raumvorschlaege`, Befehl
`ifc_raumvorschlaege`): Die Wände, Stützen und Vorhangfassaden im
Grundriss-Schnitt werden vereinigt; jede geschlossene Fläche über 1 m² ist ein
Vorschlag. Türen zählen als geschlossen, weil Öffnungen nicht abgezogen
werden. Vorschläge, die ein vorhandener Raum schon abdeckt, sind markiert.

Beim Speichern prüft der Core jede Änderung (GlobalId-Format, Zielklasse,
Geschoss, Umriss ≥ 0,1 m², Textlängen). Änderungen für Elemente, die nach
einem Reimport fehlen, bleiben gespeichert, wirken nicht und erscheinen als
Hinweis im Modellbericht. Im Reiter „Prüfung“ öffnet die Aktion eines Befunds
die Nacharbeit mit den betroffenen Elementen.

## Auswertung

`CoreServices::ifc_auswerten` (Befehl `ifc_auswerten`, Reiter „Auswertung“)
rechnet aus der Geometrie des Modells mit Nacharbeit (`ifc::auswerten`):

| Wert | Ermittlung |
| --- | --- |
| Gebäudegrundfläche | Projektion der Außenkanten (Wände, Stützen, Vorhangfassaden, Decken ohne Dach) aller Geschosse ab dem Erdgeschoss; im Grundriss blau gezeigt |
| BGF je Geschoss | Fläche der Außenkante des Geschosses, ohne Innenhöfe |
| BRI je Geschoss | BGF × Höhe: bis zur nächsten Geschossebene, im obersten Geschoss bis zur Oberkante der Bauteile, im untersten ab Unterkante Bodenplatte |
| NRF | Summe der Raumgeometrie je Geschoss und nach Nutzung (Langname); die `NetFloorArea` der Datei steht daneben |
| Höhen | über der Ebene des Erdgeschosses: Oberkante Flachdach bzw. First und Traufe (Näherung: tiefste Unterkante der Dachbauteile), höchster Punkt aller Bauteile, Bezug in m ü. NHN bei Georeferenz |
| GRZ, GFZ | rechnerisch: Gebäudegrundfläche bzw. BGF der oberirdischen Geschosse durch die Bezugsfläche des gewählten (oder einzigen) Baugrundstücks |

GRZ und GFZ der Auswertung sind Orientierungswerte. Vollgeschosse,
Anrechnungen nach § 19 Abs. 4 BauNVO und Abstandsflächen rechnen weiter die
Fachmodule; dafür das Modell über „Übernahme“ in Gebäude und Zeichnungen
übernehmen.

## Reimport

Kommt eine neue Fassung vom Planer (neue Dokumentversion derselben Datei oder
eine andere IFC-Datei), gleicht `CoreServices::ifc_abgleich` (Befehl
`ifc_abgleich`, Reiter „Modellbericht“) sie mit dem bisherigen Modell der Akte
ab, beide ohne Nacharbeit und über die GlobalId (`ifc::abgleichen`):

- neue, entfernte und geänderte Elemente (Klasse, Art, Name, Geschoss,
  Grundfläche, Ober-/Unterkante, Eigenschaften), Zahl der unveränderten,
- Geschosse, die hinzukommen, entfallen oder ihre Höhe ändern,
- der Stand jeder Nacharbeit: *wirksam* (wirkt weiter), *verwaist* (Element
  fehlt) oder *überholt* (die neue Fassung hat den Zustand schon selbst, z. B.
  der Proxy ist jetzt eine Wand; ein gebildeter Raum ist überholt, wenn ein
  Raum der neuen Fassung ihn abdeckt).

`CoreServices::ifc_reimport` (Befehl `ifc_reimport`) stellt die Akte auf die
neue Datei um. Die Nacharbeit wandert mit; überholte und verwaiste Änderungen
können dabei gestrichen werden. Die Revision der Akte muss stimmen, dieselbe
Version wird abgelehnt. Eine vorhandene Übernahme bleibt bestehen und wird beim
nächsten Übernehmen aktualisiert (Markierungs-IDs bleiben). Steht im DMS eine
neuere Version der Datei, zeigt die Modellansicht „Neue Version … vorhanden“.

## Übernahme ins Projekt

`CoreServices::ifc_uebernehmen` (Befehl `ifc_uebernehmen`, Reiter
„Übernahme“ der Modellansicht) legt aus dem Auszug der Modellakte (mit
Nacharbeit) an oder aktualisiert:

- ein **Gebäude** mit je einem Geschoss je IFC-Geschoss (gleichnamige
  Geschosse eines vorhandenen Gebäudes werden wiederverwendet; die
  Geschosshöhe aus den Geschossebenen wird nur ergänzt, nie überschrieben),
- je Geschoss eine **eigene Zeichnung** in Metern mit dem IFC-Grundriss als
  Hintergrund (Lage `RAND_M` = 2 m) und den Markierungen *Raum* je `IfcSpace`
  (Name „Nummer Langname“), *Geschoss-Außenkante* (Vereinigung von Wänden,
  Stützen und Decken ohne Innenhöfe) und im Geschoss nächst ±0 die
  *Gebäudegrundfläche* (Außenkanten aller Geschosse darüber),
- auf Wunsch je Geschoss ein **Raumbuch** über die vorhandene Übernahme aus
  dem Aufmaß; die lichte Höhe kommt aus `FinishCeilingHeight`, `Height` oder
  `ClearHeight`, wenn der Raum noch keine hat.

Die Flächen rechnet das Aufmaß aus der Geometrie, nicht aus den Mengen der
Datei (die Mengen zeigt die Prüfung als Abweichung). Die Modellakte merkt
sich die Übernahme (`Modelluebernahme`: Gebäude, Geschosse, Zeichnungen,
Raumbücher und je Markierung die Herkunft). Wiederholen ersetzt nur die
übernommenen Markierungen und behält ihre IDs, damit Bindungen an
Berechnungen gültig bleiben; selbst gezeichnete Markierungen bleiben.

## Export (IFC schreiben)

`CoreServices::ifc_exportieren` (Befehl `ifc_exportieren`, Reiter „Export“)
schreibt eine IFC-Datei, liest sie zur **Selbstkontrolle** wieder ein
(`ifc::kontrolle`, `ifc::entwurf_kontrolle`) und legt sie in den
Projektdateien ab; gibt es den Dateinamen schon, als neue Version.

**Nacharbeit zurückschreiben** (`Exportwunsch::Nacharbeit`,
`ifc::nacharbeit_schreiben`): Die Datei der Modellakte wird im Schema der
Quelle neu geschrieben (`<Name>-nachgearbeitet.ifc`). Nur betroffene
Instanzen ändern sich, alle anderen bleiben zeichengleich
(`StepDatei::neu_schreiben`).

| Änderung | in der Datei |
| --- | --- |
| `klassifizieren` | Entität ersetzt (gleiche ID, Attribute der Zielklasse, `PredefinedType`); fehlt die Klasse im Schema (Schornstein, Sonnenschutz in IFC2X3), unklassifiziertes Bauteil mit Objekttyp und Hinweis |
| `geschoss_zuordnen` | aus der bisherigen Enthaltensein- bzw. Aggregationsbeziehung genommen, in die des Geschosses aufgenommen |
| `eigenschaft_setzen` | neuer Einzelwert im Typ des bisherigen Werts (Längen in Dateieinheiten); ein mit anderen Elementen geteilter Satz wird für das Element kopiert |
| `benennen` | `Name`, bei Räumen, Geschossen und Gebäuden `LongName` |
| `raum_bilden` | `IfcSpace` als Extrusion des Umrisses mit `Qto_SpaceBaseQuantities` (NetFloorArea, Height), dem Geschoss aggregiert |
| `ausblenden` | Element entfernt, aus allen Beziehungen genommen; Teile eines entfernten Aggregats rücken in dessen Geschoss |

**Eigenes IFC4-Modell** (`Exportwunsch::Entwurf`, `ifc::entwurf_schreiben`):
Aus einem Gebäude und je Geschoss einer eigenen Zeichnung (Planaufmaß ohne
Unterlage, Meter) entsteht `<Gebäude>-Modell.ifc` mit Projekt, Grundstück,
Gebäude (Nullpunkt in m ü. NHN), Geschossen (Ebene aus `fussboden_m` oder
aufsummierten Geschosshöhen, sonst 2,75 m), `IfcWall` je gezeichneter Wand
(Höhe Geschosshöhe − Deckendicke; `Pset_WallCommon.IsExternal`, wenn die Achse
an der Geschoss-Außenkante liegt), `IfcSlab` aus der Außenkante (oberstes
Geschoss als Dach) und `IfcSpace` je Raummarkierung („0.01 Wohnen“ → Nummer und
Nutzung, mit Basismengen). Die GlobalIds werden aus den IDs von Gebäude,
Geschossen, Wänden und Markierungen abgeleitet (UUID v5) und bleiben bei jedem
Export gleich, damit Empfänger neu exportierte Fassungen über die GlobalId
abgleichen können. Die Datei lässt sich direkt wieder als Modellakte öffnen.

## Testdaten

`core_lib::ifc::testdaten::efh_ifc4()` erzeugt ein vollständig bekanntes
zweigeschossiges Einfamilienhaus (Millimeter, Georeferenz, Proxy, Flächenmodell,
absichtlich abweichende Raummengen). `efh_ifc4_v2()` ist die zweite Fassung für den Reimport (Proxy als Wand,
Schornstein entfernt, Raum umbenannt). `wand_ifc2x3()` ist ein minimales
IFC2X3-Modell. Tests: `crates/core-lib/src/ifc/tests.rs`,
`crates/storage/tests/ifc.rs`, UI: `apps/desktop/tests/ifc.py` mit der Fixture
`apps/desktop/tests/fixtures/ifc-efh.json`, erzeugt mit
`cargo run -p core-lib --features testdaten --example ifc_fixture`.
