# Fachlicher Ausbau der Bauantragsberechnungen

Umgesetzt sind acht zusätzliche eigenständige Fachmodule einschließlich
Projektmasken, versionierter Regeln, Quellübernahmen, Nachweisarchiv und PDF.
[Bedienung, Modulverträge, fachliche Grenzen und Prüfungen](fachberechnungen/README.md)
beschreibt den aktuellen Stand. Die folgenden Abschnitte halten die fachlichen
Ausbauziele fest; spezielle kommunale Sonderfälle, vollständige Hydraulik und
automatische CAD-Baulinienableitung bleiben zusätzliche Erweiterungen.

## Priorität 1: Stellplatznachweis

Ein eigenständiges Fachmodul liest typisierte Nutzungsdaten und stellt einen
Nachweisbedarf sowie die Gegenüberstellung der erforderlichen und angebotenen
Stellplätze bereit. Eingaben, Satzungsregelwerk, Berechnung, Archivierung und
Druckadapter bleiben getrennt. Das Modul definiert keine pauschale bundesweite
Pflicht und unterstellt nicht, dass eine Stellplatzpflicht überall besteht.

Ein versioniertes Regelpaket muss Geltungsgebiet, Rechts-/Satzungsfassung,
Wirksamkeit und Quelle sowie mögliche projektbezogene Ausnahmen festhalten.
Eine Auswahl der Kommune allein genügt nicht, wenn mehrere Satzungsstände oder
Zonen gelten. Örtliche Satzungen und Landesrecht sind im Zusammenhang zu prüfen.
Es gibt keinen stillen Standardfaktor bei ungeklärter Grundlage.

Eine Regel beschreibt mindestens:

- Nutzung, zum Beispiel Wohnungen, Büro, Verkauf, Gastronomie oder Beherbergung;
  Gebäudetyp, Größenklassen und gegebenenfalls Gebiet/Zone als Auswahlbedingungen.
- Eindeutige Bezugsgröße: Wohneinheiten, Wohnfläche, Nutzfläche, Verkaufsfläche,
  Sitzplätze oder Betten. „m²“ allein ist keine fachlich eindeutige Bezugsgröße.
- Faktor und Bezugsmenge, zum Beispiel fiktiv 0,75 je Wohneinheit oder 1 je 50 m²
  einer ausdrücklich benannten Flächenart; Mindestwerte und Staffelgrenzen.
- Rundungsverfahren und Rundungsstufe: je Nutzung, Gebäude, Grundstück oder erst
  Gesamtbedarf. Bedingte Regeln werden auf Eindeutigkeit geprüft.
- Besucheranteile und deren Einrechnung: im Gesamtfaktor enthalten oder zusätzlich.
- Fahrradabstellplätze als eigener Bedarf. Barrierefreie Stellplätze als
  Teilmenge/zusätzliche Anforderung entsprechend der konkreten Regel; keine
  unbeabsichtigte doppelte Addition.
- Nur ausdrücklich zulässige Reduktionen, etwa ÖPNV-/Mobilitätskonzept oder
  andere satzungsgemäße Voraussetzungen, mit Begründung und Belegen.

**Fiktiver Kontrollfall:** 4 Wohneinheiten × 0,75 = 3 Stellplätze; bei einer
anderen Satzung mit 1,5 je Wohneinheit ergeben sich 6. Das sind Beispielwerte,
keine kommunalen Vorgaben. Ein Flächensatz braucht eine Bezugsmenge: „1 je 50 m²
Verkaufsfläche“ ist etwas anderes als „1 je 50 m² Wohnfläche“.

Gemischte Nutzungen werden getrennt aufgestellt. Nutzungsänderungen benötigen
gegebenenfalls eine nachvollziehbare Alt-/Neubedarfsdifferenz. Zeitversetzte
Mehrfachnutzung, gesicherte Stellplätze auf anderen Grundstücken, Ablösung und
Ausnahmen dürfen nur auf dokumentierter Grundlage angerechnet werden. Eine
Ablösung wird nicht als tatsächlich vorhandener Stellplatz ausgewiesen.

Der Bericht enthält Nutzungsposition, Bezugswert/Einheit, Quelle der Fläche,
Satzungsregel, Ansatz, ungerundeten Bedarf, Rundung, Zuschläge/Reduktionen,
Endbedarf, Besucher-/Sonderanteile und die Deckung mit Lageplan-/Belegbezug.
Je Gebäudetyp oder Grundstück notwendige Trennungen bleiben erhalten.
Automatische Übergaben zum DMS, zum Bauantrag und zu Formularen erfolgen über
Ergebnisverträge. Einheiten oder Anforderungen ohne klare Grundlage bleiben offen.

## Priorität 2: Wohnflächen und DIN-277-Flächen

WoFlV und DIN 277 benötigen getrennte Berechnungsmethoden und Ergebnisbegriffe.
Raumgeometrien können gemeinsam genutzt werden; ihre fachliche Anrechnung darf
nicht vermischt werden. Auch GFZ-Geschossfläche ist ein eigenständiges Ergebnis.

Für Wohnflächen werden je Wohnung/Raum die Grundflächen, Höhenzonen, Abzüge,
Bauteile und Außenflächen erfasst. Schrägen, Treppen, Nischen, Balkone, Loggien
und Terrassen benötigen die jeweils anwendbare Behandlung und eine dokumentierte
Begründung für variable Ansätze. Ein Balkonanrechnungsfaktor darf nicht als
beliebiger unkommentierter Wert verwendet werden. Die maßgebliche Methode und
Fassung werden festgehalten.

Ausgabe: Aufmaß-/Teilflächenansätze, Anrechnungsfaktoren, Raum-, Wohnungs- und
Gebäudesummen. Wohnfläche, Nutzungsfläche und Geschossfläche werden in Formularen
nur passend zur jeweiligen fachlichen Bedeutung übernommen. Flächenänderungen
machen abhängige Stellplatz- und Formularnachweise gezielt prüfbedürftig.

## Weitere sinnvolle Nachweise

| Modul | Prüfgegenstand / Grundlage |
| --- | --- |
| Gebäudehöhen und Dachgeometrie | Trauf-/First-/Gebäudehöhe, Dachneigung, Höhenbezug und örtliche Planfestsetzungen |
| Vollgeschossprüfung je Geschoss | Landesrechtliche Definition, Flächen-/Höhenanteile, Geländebezug; mehr als eine manuell gesetzte Vollgeschossmarkierung |
| Überbaubare Grundstücksfläche | Bestehende Baugrenzenprüfung um Baulinien, zulässige Vorbauten und gesondert dokumentierte Abweichungen erweitern |
| Versiegelung und Freiflächen | Getrennt von GRZ: tatsächliche Versiegelung, Begrünung, Pflanz-/Grünfestsetzungen und gegebenenfalls Spiel-/Gemeinschaftsflächen |
| Regenwasser / Rückhaltung | Entwässerungsflächen, Abflussansätze und projektspezifische technische bzw. örtliche Vorgaben; eigenständiger technischer Nachweis |
| Fahrrad- und besondere Stellplätze | Satzungs-/landesrechtliche Bedarfe und tatsächlich angebotene Qualitäten getrennt ausweisen |

Nicht jeder Nachweis folgt aus dem Bebauungsplan. Zu jedem Modul müssen die
maßgebliche Rechts-/Satzungs-/Berechnungsmethode, Geltung und fachliche Freigabe
klar benannt werden. Die Anforderungsliste des Bauantrags entscheidet, welche
Nachweise im konkreten Verfahren benötigt werden.
