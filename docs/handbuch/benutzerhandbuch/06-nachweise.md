# 6 Berechnungen und Nachweise

[← Unterlagen, Aufmaß, Gebäudemodell](05-ermittlung.md) · [Inhalt](README.md) · [Weiter: Formulare und Druck →](07-formulare-druck.md)

Register **Bereich 2 · Nachweis und Einreichung**, Gruppen *Grundstück und
Nutzung*, *Gebäude und Flächen* und *Weitere Nachweise*.

## Baugrundstück

![Baugrundstücksermittlung](bilder/nachweis-baugrundstueck.png)

Das Baugrundstück ist die Bezugsfläche aller Nutzungsmaße. Es wird aus den
gespeicherten Vorhabenflurstücken gebildet.

1. Oben steht die **Datengrundlage**: die gespeicherten Flurstücke mit Fläche
   und Grundbuchzuordnung (Änderungen dort, nicht hier).
2. **1 · Flächenabzüge und Zuordnungen:** je Flurstück nicht maßgebende Flächen
   abziehen (z. B. festgesetzte Verkehrs- oder Grünfläche) – von Hand oder mit
   „Abzugsflächen aus Aufmaß übernehmen“. Die rechtliche Grundlage ist zu
   ergänzen. Eine besondere Zuordnung mehrerer Flurstücke braucht eine belegte
   Grundlage.
3. **2 · Baugrundstücksermittlung:** „Baugrundstücke ermitteln“ bildet die
   Einheiten mit Eigentumsfläche, Abzügen und Bauland.
4. **Maßgebende Grundstücksfläche:** Zuschläge oder Gemeinschaftsanteile
   („Flächenansatz anlegen“), dann **Bezugsflächen berechnen** und **Eingaben
   speichern**.

Die Prüfung meldet hier auch Abweichungen zwischen gezeichneter und amtlicher
Flurstücksfläche sowie Doppelerfassungen von Hand- und Zeichnungswerten
([Kapitel 5](05-ermittlung.md#handwert-und-zeichnung-abgleichen)).

> „Ermittlung ist veraltet“ heißt: Flurstücke oder Zuordnungen haben sich
> geändert. Neu ermitteln und speichern. Abzüge nicht noch einmal in GRZ/GFZ
> abziehen.

## GRZ, GFZ, Geschossigkeit und BMZ

Die vier Nutzungsmaße arbeiten gleich: **Baugrundstück** oben wählen,
Eingaben prüfen, **Berechnen**, **Eingaben speichern**, **Nachweis
erstellen …**

![GRZ-Berechnung mit Bauteilen](bilder/nachweis-grz.png)

**GRZ (Grundflächenzahl)**

- „Bekannte Flächen aus Gebäude/Geschosse“: Gebäude, die dem Baugrundstück
  zugeordnet sind, kommen automatisch mit ihrer Grundfläche und Art
  („Verknüpfung lösen“ macht daraus eine Handzeile).
- „Bauteile für GRZ I und GRZ II“: weitere Flächen wie Zufahrt, Stellplatz,
  Terrasse, Garage, Nebenanlage, Tiefgarage. Der **Bauteiltyp** steuert die
  Anrechnung, nicht die Bezeichnung. „Bauteile aus Aufmaß übernehmen“ holt
  gezeichnete Flächen.
- Bei BauNVO 1990/2017 werden GRZ I (Hauptanlagen) und GRZ II (mit Flächen
  nach § 19 Abs. 4) getrennt ausgewiesen; ältere Fassungen rechnen anders.

![GRZ-Ergebnis](bilder/nachweis-grz-ergebnis.png)

Das Ergebnis vergleicht ermittelte und zulässige Werte mit Grundlage und
Abweichung („✓ Vorgabe eingehalten“ bzw. Überschreitung) und zeigt die Lage
im Baufenster, sofern im Aufmaß zugeordnet. Eine Überschreitung zeigt einen
möglichen Abweichungs- oder Befreiungsbedarf an; sie ist keine Befreiung
([Kapitel 8](08-verfahren.md#vorbereitung)).

**GFZ und Geschossigkeit** übernehmen die Geschosse aus dem Gebäudemodell.
Weitere Geschosse nur ergänzen, wenn sie dort fehlen. Bei Nichtvollgeschossen
die anrechenbaren Teilflächen getrennt angeben. Die Einstufung als
Vollgeschoss ändert man am Gebäude bzw. im verknüpften Vollgeschossnachweis.

**Bauvolumen / BMZ** braucht einen eigenen, nachvollziehbaren Volumenansatz in
m³. Ein DIN-277-Bruttorauminhalt ist nicht ohne Prüfung die Baumasse im
planungsrechtlichen Sinn.

Ohne gespeicherte Rechtsgrundlage lassen sich die Maße mit „Entwurf öffnen“
vorbereiten; „Entwurf auf diesem Gerät sichern“ hält ihn lokal fest.

## Fachberechnungen

Überbaubare Grundstücksfläche, Wohnflächenberechnung, DIN-277-Flächen,
Vollgeschossprüfung, Gebäudehöhen, Stellplatznachweis, Abstandsflächen,
Versiegelung / Freiflächen und Regenwasser / Rückhaltung folgen demselben
Aufbau:

![Wohnflächenberechnung](bilder/nachweis-wohnflaeche.png)

1. **Neu anlegen** oder einen Eintrag **öffnen**. Für getrennte Gebäude,
   Wohnungen oder Varianten getrennte Einträge führen.
2. Oben steht der Ablauf **Entwurf → Berechnen und prüfen → Nachweis →
   Freigabe**.
3. Reiter **Eingaben:** Bezeichnung, Baugrundstück, Gebäude und die
   Positionen. „Positionen als Tabelle bearbeiten“ wechselt in die
   Tabellenerfassung; Werte lassen sich aus Raumbuch oder Aufmaß übernehmen.
4. Reiter **Regeln / Grundlagen:** geltende Regeln mit Quelle, Fassung und
   Prüfvermerk. Mitgelieferte Beispielregeln vor Verwendung bestätigen oder
   ersetzen. Bestätigte Regelversionen (z. B. eine kommunale
   Stellplatzsatzung) lassen sich merken und als JSON weitergeben.
5. Reiter **Quellen / Übernahmen:** Dokumentversionen und Messwerte
   verknüpfen; bei geänderter Quelle „Aktuellen Aufmaßstand übernehmen“.
6. Reiter **Prüfbericht:** Ergebnis, offene Punkte, Hinweise. Offene Punkte
   klären – eine 0 ersetzt keinen unbekannten Wert.
7. Speichern, „Gespeicherte Grundlagen bestätigen …“ (mit Berechtigung) und
   **Nachweis erstellen …**.

Besonderheiten der einzelnen Nachweise:

| Nachweis | Worauf achten |
| --- | --- |
| Überbaubare Grundstücksfläche | Abstände zu Baugrenze/Baulinie mit Vorzeichen und Richtung; ergänzt die geometrische Baufensterprüfung |
| Wohnflächenberechnung | Räume „aus Raumbuch“ übernehmen; Raumart, Wohnung, Dachschrägen-Höhenzonen, Außenflächen-Anteil |
| DIN-277-Flächen | Teilflächen nach NUF/TF/VF/KGF; BRI über eigene äußere Volumenkörper |
| Vollgeschossprüfung | Landesrechtliche Definition (z. B. § 2 Abs. 6 BauO NRW); Ergebnis im Gebäude-Geschoss verknüpfen |
| Gebäudehöhen / Dachneigung | Einheitliches Höhensystem; absolute (m ü. NHN) und relative Höhen nicht mischen |
| Stellplatznachweis | Faktoren der örtlichen Satzung, Kfz und Fahrräder getrennt, Rundungsstufe |
| Abstandsflächen | Geführte Schritte siehe unten |
| Versiegelung / Freiflächen | Teilflächen vollständig bilanzieren, Faktoren mit Satzungsbezug |
| Regenwasser / Rückhaltung | Vereinfachte stationäre Bilanz; kein vollständiger Überflutungsnachweis |

![Stellplatznachweis](bilder/nachweis-stellplaetze.png)

### Abstandsflächen

![Abstandsflächen](bilder/nachweis-abstandsflaechen.png)

Der Abstandsflächennachweis führt mit „Vorheriger Schritt“ / „Nächster
Schritt“ durch die Reiter **Grundlagen → Gebäudeumriss → Wände / Bauteile →
Lagepläne / Grenzen → Sonderfälle / Sicherungen → Prüfbericht**.

1. Grundlagen: Gemeinde, konkrete LBO-Fassung, Baugebiet, Anwendbarkeit.
2. Gebäudeumriss mit Eckpunkten in Umlaufrichtung; „Seiten als Wände
   übernehmen“ erzeugt die Wände.
3. Je Wand Gelände- und Wandabschluss, Dachanteile und Bauteile; die
   Planskizze hebt die aktive Wand hervor. „Dachansatz einer anderen Wand
   verwenden“ kopiert nur die Dachwerte.
4. Grundstücksgrenzen und Nachbargebäude im selben Koordinatenbezug.
5. Sonderfälle und rechtliche Sicherungen (Baulast, Übernahme) mit Belegen.
6. Berechnen, Konflikte in Tabelle und Plan prüfen, Nachweis erstellen.

## Nachweisstände und Freigaben

![Nachweisstände](bilder/nachweis-nachweisstaende.png)

„Nachweisstände“ listet alle archivierten Nachweise mit Erstellungsdatum,
Vollständigkeit, **Aktualität** und **Freigabe**.

- **Aktualität prüfen** vergleicht jeden Stand mit den aktuellen Grundlagen.
  „Grundlagen geändert“ bedeutet: Der archivierte Inhalt bleibt, ist aber
  nicht mehr aktuell. „Berechnung öffnen“ führt zur Quelle, dort neu berechnen
  und einen neuen Nachweisstand anlegen.
- **Freigeben** ist eine Bürofreigabe durch die berechtigte Leitung. Archivieren
  und Freigeben sind getrennte Schritte; eine Freigabe ist keine Einreichung.
- „Archivstand wiederholen“ rechnet einen Stand mit seinen damaligen Eingaben
  und Regeln nach und meldet, ob dasselbe Ergebnis herauskommt. „Drucken /
  PDF“ öffnet die Druckausgabe; dort übernimmt „Als Projektunterlage ablegen“
  das PDF versioniert in die Projektdateien.
- „Grundlagen und Regeln ansehen“ zeigt, womit damals gerechnet wurde.
