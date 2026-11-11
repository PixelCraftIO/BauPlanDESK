# Beispielprojekt „Neubau Einfamilienhaus Mustermann“ (NRW)

Ein vollständig durchgespieltes Projekt, das alle Arbeitsbereiche einmal
verwendet: freistehendes Einfamilienhaus mit Keller, ausgebautem Dachgeschoss und
Grenzgarage für Max Mustermann, Lindenweg 12, 59999 Musterstadt (fiktiv).
Stichtag aller Angaben ist der 21.10.2026.

| Datei | Inhalt |
|---|---|
| `Beispielprojekt-EFH-Mustermann.zip` | Sicherungsarchiv (Datenbank, DMS-Dateien, Formularvorlagen); Grundlage der Handbuchbilder |
| `Bauvorlagen-EFH-Mustermann.pdf` | Bauvorlagenpaket aus „Unterlagen → Drucken“ (216 Seiten, 36 Anlagen) |

## Verwendung

Die Anwendung liefert das Beispielprojekt seit 11/2026 nicht mehr aus (der
Menüpunkt „Hilfe → Beispielprojekt öffnen“ ist entfallen). Das Archiv bleibt
die Grundlage der Bildschirmfotos im Benutzerhandbuch: `scripts/handbuch-bilder`
stellt es in ein eigenes Profil wieder her und nimmt die Szenen darin auf.

Von Hand laden: Einstellungen → Sicherung und Wiederherstellung → „In neues
Profil wiederherstellen“ mit diesem Archiv, dann „Profil aktivieren“ und die App
neu starten.

## Rechentest

`cargo test -p storage --test beispielarchiv` stellt genau das hier abgelegte
Archiv mit dem aktuellen Programmstand wieder her. Der Test prüft, dass alle
Nachweise aktuell sind (die heutige Berechnung kommt aus denselben Eingaben und
Regeln zum gespeicherten Ergebnis) und dass Formulare die Kennzahlen unten
erhalten (GRZ 0,19, GFZ 0,16, Grundfläche 119 m², Grundstück 620 m² …). Schlägt
er nach einer Regel- oder Berechnungsänderung fehl, das Archiv neu erzeugen
(siehe unten) und die geänderten Zahlen hier und im Test nachtragen.

## Inhalt

- **Stammdaten und Planungsrecht:**
  - Büro „Planungsbüro Planer Architekten“, Entwurfsverfasserin Erika Planer (Kammer-Nr. A-12345).
  - Bauherr, Tragwerksplaner und zwei Nachbarn als Kunden.
  - Bebauungsplan Nr. 42 „Am Lindenweg“: WA, GRZ 0,4, GFZ 0,8, II Vollgeschosse.
  - Flurstücke 411–413 mit Grundbuch- und Eigentümerangaben; alle drei sind im
    Lageplan LP-01 gezeichnet, die gezeichnete Fläche steht als Gegenprobe
    neben der Katasterfläche.
- **Gebäude/Geschosse:**
  - Wohnhaus 11 × 9 m (KG, EG, DG, Satteldach 36° als Dachform erfasst, Traufe +4,20, First +7,45 über ±0,00 = 72,64 m ü. NHN).
  - Garage 3,5 × 6 m.
  - Die Werte stammen aus dem PDF-Aufmaß eines Vektorplans (Grundrisse, Schnitt A‑A, Ausschnitte mit Bezugslinie).
- **Raumbücher** für KG, EG und DG. Die Dachschrägen (1,00–2,00 m lichte Höhe)
  sind im PDF-Aufmaß als Höhenzonen ihrer Räume gekennzeichnet; das Raumbuch DG
  hat 6 Räume mit 5 Höhenzonen.
- **Fachberechnungen und Nachweise:**
  - Wohnfläche (WoFlV) 154,47 m².
  - DIN 277: BGF 294,87 m², BRI 883,57 m³.
  - Vollgeschosse, Gebäudehöhen, Stellplätze, Freiflächen, Entwässerung (Rückhaltung) und Überbaubarkeit.
  - GRZ 0,19 (II: 0,29), GFZ 0,16, BMZ 0,46.
  - Alle Nachweise sind aktuell und freigegeben.
- **Abstandsflächen nach § 6 BauO NRW:**
  - Lageplan LP-01 (PDF, M 1:200) mit eigenem Aufmaß: Flurstücke, Umrisse von
    Wohnhaus und Garage und je Außenwand eine Linie „Abstandsfläche“ (A1–A8).
  - Wände und Grundstücksgrenzen des Abstandsplans sind aus diesen Linien und
    Flurstücken übernommen und bleiben an die Zeichnung gebunden; der
    Abstandsflächenplan entsteht auf dem Blatt des Lageplans.
  - Giebelanrechnung mit Dachhöhe und Neigung aus dem Gebäude, Grenzgarage nach § 6 Abs. 8.
  - Nachweis mit PDF sowie JSON- und DXF-Export.
- **Verfahren:**
  - Unterlagenregister mit Bürocheckliste, Aufgaben, Verfahrensakte (vereinfachtes Verfahren, Aktenzeichen 63-BA-0815/2026).
  - Bauantrag mit Formularen (Musterformular und Büroformular); 6 von 11
    Anlagenarten folgen der Dokumentkategorie.
  - XBau-Antrag (`baugenehmigung.antrag.0200`, schemageprüft) mit Fachrichtung
    des Tragwerksplaners und XBau-Eingangsbestätigung. Eine Verfahrensart
    enthält er nicht, weil für NRW keine Landesliste veröffentlicht ist.
  - Portalabgabe über Bauportal.NRW und externe Einreichung mit Beleg.
  - Nachforderung per E-Mail, aus dem E-Mail-Archiv in die Verfahrensakte
    übernommen; Termine und Wiedervorlagen.
  - Verfahrenswahl nach BauO NRW (Fassung ab 01.09.2026) und NRW-Bauvorlagenkatalog;
    Abweichungsabgleich ohne Unterschreitung und erfasste Unterschriften.
  - Lage der Akte mit voraussichtlicher Genehmigungsfiktion am 10.11.2026,
    Entwurf des Antwortschreibens auf die Nachforderung (PDF) und fachliche
    Auswertung der XBau-Eingangsbestätigung.
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
