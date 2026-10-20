# PDF-Aufmaß und verknüpfte Berechnungseingaben

## Umfang dieses Ausbaus

Unter **Unterlagen → PDF-Aufmaß** lassen sich PDF-Dateien aus dem Projekt-DMS
als unveränderte Zeichenunterlagen verwenden. Ein Aufmaß gehört zu genau einer
Dokumentversion und einer Seite. Mehrere Aufmaße können dieselbe Seite verwenden,
beispielsweise Lageplan, Ansicht und Schnitt mit unterschiedlichen Maßstäben auf
einem Sammelblatt.

Enthalten sind:

- Maßstab 1:n oder Kalibrierung einer bekannten Strecke über zwei Punkte.
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

1. PDF unter **Projektdateien** importieren.
2. **PDF-Aufmaß → Neu anlegen**, Unterlage, Seite und Bezeichnung auswählen.
3. Maßstab setzen oder eine bekannte Strecke kalibrieren. Ein gedruckter
   Maßstab gilt nur, wenn die PDF unverändert im angegebenen Papiermaß vorliegt.
4. Auf einer sichtbaren, ungesperrten Ebene zeichnen, Bedeutung und Einheiten
   zuordnen. Absolute Höhen benötigen zusätzlich einen bekannten Bezugspunkt
   und ein benanntes Höhensystem. Ansichten/Schnitte müssen lotrecht zur
   dargestellten Seite ausgerichtet sein.
5. Aufmaß speichern. In der Fachberechnung eine passende Position anlegen
   und unter **Quellen / Übernahmen → PDF-Aufmaß** das Zielfeld verknüpfen.
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

Unter **Gebäude / Geschosse → Flächen und Höhen aus PDF-Aufmaß verknüpfen**
werden die gezeichneten Außenflächen mit vorhandenen Gebäuden und Geschossen
verbunden. Diese zuerst speichern, damit Markierungen ihre stabilen IDs wählen
können. Die Bedeutung **Gebäudegrundfläche – horizontale Projektion** ist vom
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

Unter **Abstandsflächen → Grundlagen → PDF-Aufmaß verknüpfen** werden bestehende
manuelle Wände als Ziel angeboten. Absolute Höhen benötigen dasselbe
Höhensystem. Bereits vorhandene Gelände-/Oberkantenprofile müssen ausdrücklich
entfernt werden, bevor Einzelhöhen eingesetzt werden. Umrisswände erhalten
weiterhin ihre Geometrie und Eckhöhen vom Gebäudeumriss. Bei abweichenden
Grundstücks-IDs wird die Flurstückszuordnung geprüft.

Die Zuordnung verwendet **Tabelle.Zeilen-ID.Feld**, keine Listenposition.
Eine Quelle darf mehrere Berechnungen versorgen. Mehrere konkurrierende
Quellen für dasselbe Zielfeld werden zurückgewiesen. Grundstück und Gebäude
müssen zwischen Markierung und Zielberechnung übereinstimmen.
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
  Höhenreferenz, Ebenen und identifizierbare Markierungen.
- `Planmarkierung`: UUID, Ebene, Geometrie, Bedeutung sowie optionale
  Grundstücks-, Gebäude- und Geschoss-IDs.
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

Koordinaten sind physische PDF-Seitenpunkte (72 Punkte = 1 Zoll) der sichtbaren,
gedrehten Seite ab links oben; y wächst nach unten. Der PDF.js-Viewport
berücksichtigt CropBox, Rotation und UserUnit. Zoom und Bildschirmauflösung
beeinflussen gespeicherte Punkte nicht. Kreise bleiben echte Kreise und werden
im Core analytisch berechnet. Absolute Höhe ist
`Bezugshöhe + (Bezug.y − Punkt.y) × MeterProPunkt`.

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
`POST /berechnung/planaufmass/gebaeudeziele`.
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
andere Einheiten, neue Maskenrevisionen und neuere PDF-Versionen zurück.
Berechnungs-Fingerprints berücksichtigen die aktuelle Maske und neueste
PDF-Version. Ein alter Nachweis bleibt unverändert archiviert, wird aber
als veraltet erkannt. Derzeit macht auch eine rein gestalterische Änderung
einer referenzierten Maske die Übernahme prüfbedürftig.
Diese Prüfung gilt auch für indirekte Verbraucher des Raumbuchs sowie
Wandnachweise. Aktuelle Raumbuchberichte verwenden keine veralteten Planflächen.
Die Projektübersicht führt veraltete Raumquellen und offene Wandangaben auf.
Gebäudeauswertung und Gebäudeeinbindung prüfen ebenfalls den Quellstand;
zugeordnete GRZ-/GFZ-Nachweise und Berechnungen mit Gebäudeübernahme werden bei
veralteten Gebäudeaufmaßen ungültig. Betroffene Gebäude sind in der Übersicht
verlinkt. Rein manuelle Gebäude ohne Planbindungen verhalten sich unverändert.

Beim Wechsel der PDF oder Seite bleiben Markierungen zur manuellen Anpassung
erhalten; Maßstab und Höhenbezug werden entfernt. Alte Dokumentversionen werden
niemals überschrieben. Ebenensperren sind eine Bedienhilfe, keine zusätzliche
Berechtigungsgrenze.

## Verbleibende Ausbaupunkte

Für das beschriebene Gesamtziel fehlen noch eigenständige Verbraucheradapter:

- Flurstücks- und private Grünflächen direkt in Grundstücksinformationen und
  Baugrundstücksermittlung übernehmen; Identität und rechtliche Flächenabzüge
  ausdrücklich erhalten.
- Geschoss-Außenkonturen zusätzlich als transformierte Lagegeometrie für
  Gebäudeumrisse und Wandkorrespondenzen bereitstellen. Die Flächenübernahme
  in Gebäude/Geschosse ist vorhanden; räumliche Planausrichtung noch nicht.
- Wände in Ansichten mit Lageplanwänden korrespondieren lassen; Gelände- und
  Oberkantenprofile mit Stationsbezug in die Abstandsflächen übernehmen.
  Eine Ansichtslänge darf nicht still eine Lageplankoordinate ersetzen.
- Höhenbezüge einschließlich gedrehter Ansichten und Transformation zwischen
  mehreren Plänen; derzeit sind Seitenhorizontalen/-vertikalen maßgebend.
- Polygonlöcher, Flächenverschneidung und Prüfung überlappender Bilanzflächen.
- CAD-artige Schnittpunkt-/Tangenten-/Kreiszentrumserkennung, vollständige
  Auswertung von Clipping und optionalen PDF-Inhaltsebenen. Kurven liefern
  derzeit Endpunkte, keine vorgetäuschten geraden Kanten. Fangziele sind am
  sichtbaren Plan zu kontrollieren; Alt deaktiviert den Fang temporär.
- Übernahme durch Auswahl einer Bedeutung allein, automatische Positionsanlage
  außerhalb des Raumbuchs sowie separate Ausgabe der annotierten Pläne.

Die zentrale Übersicht kennt gespeicherte Berechnungen und Anforderungen.
Sie behauptet keine Vollständigkeit für noch nicht angelegte Nachweise oder
nicht erfasste projektspezifische Pflichten.

## Prüfungen

- Core-Modultests: Kalibrierung, Kreis-/Polygonfläche, Orientierung, Höhen,
  ungültige Geometrie, Nullwerte, Serialisierung und Altvorlagen.
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
  Kalibrierung, Höhenbezug, Zoomkoordinaten, Speichern und Seitenwechsel.
  Fachwerte werden durch die Core-Tests geprüft.

PDF-Adapterreferenz: [PDF.js API](https://mozilla.github.io/pdf.js/api/draft/api.js.html).
Maßgebend für den implementierten Adapter ist die im Projekt gebundene
`pdfjs-dist`-Version 4.10.38.
