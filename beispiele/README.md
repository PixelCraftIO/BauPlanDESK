# Beispielprojekt „Neubau Einfamilienhaus Mustermann“ (NRW)

Ein vollständig durchgespieltes Projekt, das alle Arbeitsbereiche einmal
verwendet: freistehendes Einfamilienhaus mit Keller, ausgebautem Dachgeschoss und
Grenzgarage für Max Mustermann, Lindenweg 12, 59999 Musterstadt (fiktiv).
Stichtag aller Angaben ist der 21.10.2026.

| Datei | Inhalt |
|---|---|
| `Beispielprojekt-EFH-Mustermann.zip` | Sicherungsarchiv (Datenbank, DMS-Dateien, Formularvorlagen) zum Wiederherstellen |
| `Bauvorlagen-EFH-Mustermann.pdf` | Bauvorlagenpaket aus „Unterlagen → Drucken“ (220 Seiten, 36 Anlagen) |

## Laden

1. Einstellungen → Sicherung und Wiederherstellung → „Wiederherstellen“ und
   `Beispielprojekt-EFH-Mustermann.zip` wählen.
2. Ein neues Profil angeben, „Profil aktivieren“ und die App neu starten.
3. Das Rechnerkonto, das wiederherstellt, wird im neuen Profil Administrator.
   Das Projekt ist danach ohne weitere Freigaben bearbeitbar. Das bisherige
   Profil bleibt unverändert.

## Inhalt

- **Stammdaten und Planungsrecht:**
  - Büro „Planungsbüro Planer Architekten“, Entwurfsverfasserin Erika Planer (Kammer-Nr. A-12345).
  - Bauherr, Tragwerksplaner und zwei Nachbarn als Kunden.
  - Bebauungsplan Nr. 42 „Am Lindenweg“: WA, GRZ 0,4, GFZ 0,8, II Vollgeschosse.
  - Flurstücke 411–413 mit Grundbuch- und Eigentümerangaben.
- **Gebäude/Geschosse:**
  - Wohnhaus 11 × 9 m (KG, EG, DG, Satteldach 36°, Traufe +4,20, First +7,45 über ±0,00 = 72,64 m ü. NHN).
  - Garage 3,5 × 6 m.
  - Die Werte stammen aus dem PDF-Aufmaß eines Vektorplans (Grundrisse, Schnitt A‑A, Ausschnitte mit Bezugslinie).
- **Raumbücher** für KG, EG und DG.
- **Fachberechnungen und Nachweise:**
  - Wohnfläche (WoFlV) 154,47 m².
  - DIN 277: BGF 294,87 m², BRI 883,57 m³.
  - Vollgeschosse, Gebäudehöhen, Stellplätze, Freiflächen, Entwässerung (Rückhaltung) und Überbaubarkeit.
  - GRZ 0,19 (II: 0,29), GFZ 0,16, BMZ 0,46.
  - Alle Nachweise sind aktuell und freigegeben.
- **Abstandsflächen nach § 6 BauO NRW:**
  - Lageplan-DXF, Grundstücksgrenzen und Gebäudeumrisse.
  - Giebelanrechnung und Grenzgarage nach § 6 Abs. 8.
  - Nachweis mit PDF sowie JSON- und DXF-Export.
- **Verfahren:**
  - Unterlagenregister mit Bürocheckliste, Aufgaben, Verfahrensakte (vereinfachtes Verfahren, Aktenzeichen 63-BA-0815/2026).
  - Bauantrag mit Formularen (Musterformular und Büroformular).
  - XBau-Antrag (`baugenehmigung.antrag.0200`, schemageprüft) und XBau-Eingangsbestätigung.
  - Portalabgabe über Bauportal.NRW und externe Einreichung mit Beleg.
  - Nachforderung per E-Mail, Termine und Wiedervorlagen.
- **Ausgaben:**
  - Berechnungsheft und Abstandsflächennachweis.
  - Bauvorlagenpaket mit Deckblatt, Anschreiben, Inhaltsverzeichnis und Trennblättern.

Am Stichtag ist bewusst nur die Nachforderung der Bauaufsicht offen: der
Entwässerungsnachweis (Frist 05.11.2026). Die Projektübersicht zeigt sie mit den
zugehörigen Aufgaben.

## Neu erzeugen

Der Generator ist ein Integrationstest. Er bedient ausschließlich die
Fachschnittstellen des Cores und schlägt fehl, sobald eine Eintragung oder
Berechnung einen Befund liefert:

```sh
BAUPROJEKT_BEISPIEL_ZIEL=beispiele cargo test -p storage --test beispielprojekt -- --nocapture
```

Die Module liegen unter `crates/storage/tests/beispiel/`: `basis`, `fach`,
`abstand`, `verfahren` und `abschluss`. Jedes Modul lässt sich auch einzeln
prüfen, z. B. mit `--test beispiel_fach`. Zum Abschluss stellt der Test die
Sicherung in ein frisches Profil wieder her und prüft, dass alle Nachweise dort
aktuell sind.

## Offene Ausbauvorschläge

Diese Punkte sind Funktionslücken. Die Ergebnisse des Beispiels bleiben davon
unberührt.

- Raumbuch: keine Höhenzonen je Raum. Dachschrägen werden für die WoFlV als
  eigene Teilräume gezeichnet.
- Gebäude: keine Dachform und keine Dachneigung. Giebelhöhe und Neigung werden
  im Abstandsplan je Giebelwand von Hand eingegeben.
- Dokumentkategorien: keine Standardkategorien für Grundbuchauszug,
  Behördenschreiben, Bescheinigungen und Entwässerung.
- XBau: keine NRW-Codeliste der Verfahrensarten im Lieferumfang.
- XBau: die Anlagenart wird je Bauvorlage gesondert gewählt.
- XBau: keine Fachrichtung für Projektbeteiligte.
- E-Mail: kein direkter Weg aus dem E-Mail-Archiv in die Verfahrensakte.
