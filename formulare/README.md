# Formularvorlagen

Jede `*.json`-Datei beschreibt, wie ein amtliches PDF-Formular mit
Projektdaten befüllt wird (siehe `crates/core-lib/src/formular/vorlage.rs`).

* `erkennung`: Feldnamen, an denen das Formular erkannt wird.
* `zuordnung`: Formularfeld → Quelle. Eine Quelle ist ein Schlüssel aus dem
  Projektkontext oder, mit führendem `'`, ein fester Text.

Verfügbare Schlüssel: `projekt.name`, `projekt.bundesland`,
`projekt.bundesland_name`, `grundstueck.flaeche_m2`, `datum.heute`.

Aus dem ersten Bauherrn des Projekts (Kundendatenbank): `bauherr.name`,
`bauherr.vorname`, `bauherr.familienname`, `bauherr.firma`,
`bauherr.strasse`, `bauherr.hausnummer`, `bauherr.strasse_hausnummer`,
`bauherr.plz`, `bauherr.ort`, `bauherr.plz_ort`, `bauherr.telefon`,
`bauherr.email`.

Die Feldnamen der amtlichen Formulare liest die App selbst aus
(Dokumente → „Formular ausfüllen …“ zeigt alle Felder). Die enthaltene
Vorlage ist ein **Beispiel**; für jedes Landesformular wird eine eigene
Vorlage mit den echten Feldnamen angelegt.


Eigene PDF-Eingabefelder können auch ohne Bearbeitung dieser JSON-Dateien unter
**Dokumente → Eigene Felder / Vorlage** angelegt werden. Die Felder werden als
AcroForm in einer neuen PDF gespeichert; die bisherige Ausfüllfunktion erkennt
sie direkt. [Bedienung und Grenzen](../docs/pdf-ablage/README.md).
