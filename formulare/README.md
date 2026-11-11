# Formularvorlagen

Jede `*.json`-Datei beschreibt, wie ein amtliches PDF-Formular mit
Projektdaten befüllt wird (siehe `crates/core-lib/src/formular/vorlage.rs`).

* `land`: Bundesland der Zuordnung (z. B. `"BY"`). Ohne `land` gilt sie
  bundesweit. Erkannt werden nur Zuordnungen für das Bundesland des Projekts
  oder bundesweite; bei gleich vielen Erkennungsfeldern gewinnt die
  landesspezifische, danach die kleinere `id` (nicht die Ladereihenfolge).
* `version`: wird mit `id` in jeder ausgefüllten Formularversion gespeichert
  (`formularinstanz.zuordnung`). Beim erneuten Bearbeiten bleibt diese
  Zuordnung maßgebend, solange sie geladen ist und passt.
* `erkennung`: Feldnamen, an denen das Formular erkannt wird.
* `zuordnung`: Formularfeld → Quelle. Eine Quelle ist ein Schlüssel aus dem
  Projektkontext oder, mit führendem `'`, ein fester Text.

Verfügbare Schlüssel (Zahlen mit Dezimalkomma, leere Angaben fehlen):

* **Projekt und Planung:** `projekt.name`, `projekt.bundesland`,
  `projekt.bundesland_name`, `vorhaben.bezeichnung`, `planung.gemeinde`,
  `planung.bebauungsplan`, `planung.bebauungsplan_nummer`,
  `planung.baugebiet` (z. B. `WA`), `planung.baugebiet_name`, `datum.heute`.
* **Baugrundstück** (das gewählte oder einzige): `grundstueck.bezeichnung`,
  `grundstueck.flaeche_m2` (Größe laut Grundbuch, Summe der Flurstücke),
  `grundstueck.bezugsflaeche_m2` (maßgebende Fläche nach Abzügen),
  `grundstueck.gemarkung`, `grundstueck.flur`, `grundstueck.flurstuecke`
  (z. B. `100/1, 100/2`), `grundstueck.flurstuecksverzeichnis`,
  `grundstueck.grundbuchbezirk`, `grundstueck.grundbuchblatt` und die
  Anschrift aus dem Bauantrag (`grundstueck.strasse` … `grundstueck.plz_ort`).
* **Kennzahlen** aus der aktuellen Nutzungsberechnung des Baugrundstücks:
  `kennzahlen.grz`, `kennzahlen.grz_gesamt`, `kennzahlen.gfz`,
  `kennzahlen.bmz`, `flaechen.grundflaeche_m2`,
  `flaechen.grundflaeche_nebenanlagen_m2`, `flaechen.geschossflaeche_m2`,
  `flaechen.baumasse_m3`, `gebaeude.vollgeschosse`.
* **Ergebnisse der Fachberechnungen** (aktuell gerechnet, nur wenn es je Art
  genau eine vollständige Berechnung im Projekt gibt; sonst nennt ein Hinweis
  den Grund): `wohnflaeche.gesamt_m2`, `wohnflaeche.wohneinheiten`,
  `din277.bgf_m2`, `din277.kgf_m2`, `din277.bri_m3`,
  `stellplaetze.kfz_bedarf`, `stellplaetze.fahrrad_bedarf`,
  `freiflaechen.versiegelt_m2`, `freiflaechen.gruen_m2`,
  `entwaesserung.abflusswirksame_flaeche_m2`,
  `entwaesserung.rueckhaltevolumen_m3` sowie je Höhenpunkt und Dach der
  Höhenberechnung `hoehen.hoehe_<name>` (m) und `hoehen.dachneigung_<name>`
  (Grad), z. B. `hoehen.hoehe_traufe`.
* **Personen** aus der Kundendatenbank, jeweils mit `.name` („Vorname
  Familienname“ bzw. Firma), `.vorname`, `.familienname`, `.firma`,
  `.name_vorname` („Familienname, Vorname“ bzw. Firma, wie amtliche Formulare
  es verlangen), `.strasse`, `.hausnummer`, `.strasse_hausnummer`, `.plz`,
  `.ort`, `.plz_ort`, `.anschrift` („Straße Hausnummer, PLZ Ort“),
  `.name_anschrift` (Name und Anschrift in zwei Zeilen für mehrzeilige
  Sammelfelder wie „Bauherrschaft“), `.name_anschrift_zeile` (dasselbe in
  einer Zeile), `.telefon`, `.email`:
  `bauherr.*` (der gewählte oder einzige Bauherr), `eigentuemer.*` (laut
  Flurstück, sonst die Grundstückseigentümer unter „Weitere Beteiligte“),
  `erbbauberechtigter.*`, `bevollmaechtigter.*`, `fachplaner.*`. Bei mehreren
  Personen einer Rolle steht nur `.name` mit allen Namen.
* **Entwurfsverfasser und Büro:** `entwurfsverfasser.name`, `.vorname`,
  `.familienname`, `.name_vorname`, `.name_buero` („Familienname, Vorname,
  Büro“), `.berufsbezeichnung`, `.mitgliedsnummer`, `.kammerland`,
  `.telefon` und `.email` (eigene Angabe der Person, sonst die des Büros),
  `.mobil`, die Büroanschrift als Geschäftsanschrift (`.strasse` …
  `.plz_ort`, `.anschrift`) sowie `.name_anschrift` (mit Büro, zwei Zeilen)
  und `.name_anschrift_zeile`; `buero.name`, `buero.telefon`, `buero.email`
  und die Büroanschrift (`buero.strasse` … `buero.plz_ort`, `buero.anschrift`,
  `buero.name_anschrift`). Die Personen stehen in den Büroangaben; der
  Bauantrag wählt die verantwortliche Person, die Eingabemaske kann je
  Formular eine andere Person des Büros wählen.
* **Verfahren** (das gewählte oder einzige): `verfahren.bezeichnung`,
  `verfahren.aktenzeichen`, `behoerde.name`. Ohne Verfahren kommt
  `behoerde.name` aus dem Bauantrag.

Das Tagesdatum gehört nicht zum Datenstand: Ein Formular gilt nach
Mitternacht nicht als geändert.

**Veraltete Werte:** Ausgefüllte Felder mit Projektdatenquelle werden mit den
aktuellen Projektdaten verglichen. Weicht ein Wert ab (z. B. nach geänderter
Bauherrenanschrift), meldet die Formularanalyse ihn unter `veraltet`; die
Eingabemaske markiert das Feld und bietet die Übernahme des aktuellen Werts an.
Wird trotzdem gespeichert, hält die Formularinstanz die Abweichungen unter
`veraltet` fest. Eine Abgabe mit einer solchen Formularversion erhält einen
Hinweis, wird aber nicht abgebrochen. Feste Texte (`'…`), `datum.heute` und
Felder ohne aktuellen Projektwert werden nicht verglichen; bewusst manuell
abweichende Werte bleiben als Abweichung sichtbar.

**Entfernte Quellen:** Wurde das gespeicherte Baugrundstück, der Bauherr, die
entwurfsverfassende Person oder das Verfahren entfernt, öffnet das Formular
ohne diesen Bezug und mit einem Hinweis; eine Ersatzquelle wird nicht still
gewählt.

**Gewählte Feldquellen:** In der Eingabemaske lässt sich je Textfeld eine
andere Projektdatenquelle wählen, etwa `bauherr.name_anschrift` statt
`bauherr.name` für ein zweizeiliges Sammelfeld „Bauherrschaft“. Die Wahl wird
mit der Formularversion gespeichert
(`formularinstanz.quellen.datenstand.bezug.feldquellen`), gilt beim Abgleich
veralteter Werte und beim erneuten Laden und verändert die JSON-Vorlage nicht.
Zweizeilige Werte werden nur in mehrzeilige Felder vorgeschlagen.

Die Feldnamen der amtlichen Formulare liest die App selbst aus
(Dokumente → „Formular ausfüllen …“ zeigt alle Felder). Mitgeliefert sind
`beispiel-bauantrag.json` (Beispiel) sowie für Nordrhein-Westfalen
`nw-bauantrag-anlage-i-2.json` (Bauantrag / Antrag auf Vorbescheid),
`nw-baubeschreibung-anlage-i-7.json` und
`nw-betriebsbeschreibung-anlage-i-8.json` (Anlagen zur VV BauPrüfVO; die
amtlichen PDFs liegen unter `beispiele/`). Für jedes weitere Landesformular
wird eine eigene Vorlage mit den echten Feldnamen angelegt.


Eigene PDF-Eingabefelder können auch ohne Bearbeitung dieser JSON-Dateien unter
**Dokumente → Eigene Felder / Vorlage** angelegt werden. Die Felder werden als
AcroForm in einer neuen PDF gespeichert; die bisherige Ausfüllfunktion erkennt
sie direkt. [Bedienung und Grenzen](../docs/pdf-ablage/README.md).

Die [zentrale PDF-Formularbibliothek](../docs/formularbibliothek/README.md)
verwendet einen eigenen wählbaren Ordner für die eigentlichen PDF-Dateien.
Dieser Ordner ist von den hier beschriebenen JSON-Feldzuordnungen getrennt.
