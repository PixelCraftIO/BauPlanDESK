# Formularvorlagen

Jede `*.json`-Datei beschreibt, wie ein amtliches PDF-Formular mit
Projektdaten befüllt wird (siehe `crates/core-lib/src/formular/vorlage.rs`).

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
* **Personen** aus der Kundendatenbank, jeweils mit `.name`, `.vorname`,
  `.familienname`, `.firma`, `.strasse`, `.hausnummer`,
  `.strasse_hausnummer`, `.plz`, `.ort`, `.plz_ort`, `.telefon`, `.email`:
  `bauherr.*` (der gewählte oder einzige Bauherr), `eigentuemer.*` (laut
  Flurstück, sonst die Grundstückseigentümer unter „Weitere Beteiligte“),
  `erbbauberechtigter.*`, `bevollmaechtigter.*`, `fachplaner.*`. Bei mehreren
  Personen einer Rolle steht nur `.name` mit allen Namen.
* **Entwurfsverfasser und Büro:** `entwurfsverfasser.name`, `.vorname`,
  `.familienname`, `.berufsbezeichnung`, `.mitgliedsnummer`, `.kammerland`;
  `buero.name` und die Büroanschrift (`buero.strasse` … `buero.plz_ort`).
* **Verfahren** (das gewählte oder einzige): `verfahren.bezeichnung`,
  `verfahren.aktenzeichen`, `behoerde.name`. Ohne Verfahren kommt
  `behoerde.name` aus dem Bauantrag.

Das Tagesdatum gehört nicht zum Datenstand: Ein Formular gilt nach
Mitternacht nicht als geändert.

Die Feldnamen der amtlichen Formulare liest die App selbst aus
(Dokumente → „Formular ausfüllen …“ zeigt alle Felder). Die enthaltene
Vorlage ist ein **Beispiel**; für jedes Landesformular wird eine eigene
Vorlage mit den echten Feldnamen angelegt.


Eigene PDF-Eingabefelder können auch ohne Bearbeitung dieser JSON-Dateien unter
**Dokumente → Eigene Felder / Vorlage** angelegt werden. Die Felder werden als
AcroForm in einer neuen PDF gespeichert; die bisherige Ausfüllfunktion erkennt
sie direkt. [Bedienung und Grenzen](../docs/pdf-ablage/README.md).

Die [zentrale PDF-Formularbibliothek](../docs/formularbibliothek/README.md)
verwendet einen eigenen wählbaren Ordner für die eigentlichen PDF-Dateien.
Dieser Ordner ist von den hier beschriebenen JSON-Feldzuordnungen getrennt.
