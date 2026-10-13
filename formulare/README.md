# Formularvorlagen

Jede `*.json`-Datei beschreibt, wie ein amtliches PDF-Formular mit
Projektdaten befüllt wird (siehe `crates/core-lib/src/formular/vorlage.rs`).

* `erkennung`: Feldnamen, an denen das Formular erkannt wird.
* `zuordnung`: Formularfeld → Quelle. Eine Quelle ist ein Schlüssel aus dem
  Projektkontext oder, mit führendem `'`, ein fester Text.

Verfügbare Schlüssel: `projekt.name`, `projekt.bundesland`,
`projekt.bundesland_name`, `grundstueck.flaeche_m2`, `datum.heute`.

Die Feldnamen der amtlichen Formulare liest die App selbst aus
(Dokumente → „Formular ausfüllen …“ zeigt alle Felder). Die enthaltene
Vorlage ist ein **Beispiel**; für jedes Landesformular wird eine eigene
Vorlage mit den echten Feldnamen angelegt.
