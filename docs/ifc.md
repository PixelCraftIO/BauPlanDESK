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

## Testdaten

`core_lib::ifc::testdaten::efh_ifc4()` erzeugt ein vollständig bekanntes
zweigeschossiges Einfamilienhaus (Millimeter, Georeferenz, Proxy, Flächenmodell,
absichtlich abweichende Raummengen). `wand_ifc2x3()` ist ein minimales
IFC2X3-Modell. Tests: `crates/core-lib/src/ifc/tests.rs`,
`crates/storage/tests/ifc.rs`, UI: `apps/desktop/tests/ifc.py` mit der Fixture
`apps/desktop/tests/fixtures/ifc-efh.json`, erzeugt mit
`cargo run -p core-lib --features testdaten --example ifc_fixture`.
