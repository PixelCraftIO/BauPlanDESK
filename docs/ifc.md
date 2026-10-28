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

## Testdaten

`core_lib::ifc::testdaten::efh_ifc4()` erzeugt ein vollständig bekanntes
zweigeschossiges Einfamilienhaus (Millimeter, Georeferenz, Proxy, Flächenmodell,
absichtlich abweichende Raummengen). `wand_ifc2x3()` ist ein minimales
IFC2X3-Modell. Tests: `crates/core-lib/src/ifc/tests.rs`,
`crates/storage/tests/ifc.rs`.
