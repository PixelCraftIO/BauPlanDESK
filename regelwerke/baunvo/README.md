# BauNVO-Regelwerke

Jede JSON-Datei beschreibt eine Fassung. Rust-Rechenkern und Svelte-Oberfläche
lesen dieselben Dateien. Es gibt keine zusätzliche Bauteil-Anrechnungstabelle
im Frontend. Die Dateien werden beim Build eingebettet; Änderungen erfordern
Versionserhöhung und einen neuen Build von Backend und Oberfläche.

* `id`, `version`, `fassung`: Zuordnung und nachvollziehbare Ergebnisquelle.
* `bauteile`: Bezeichnung und automatische GRZ-Zuordnung jeder Kategorie.
  `grz_ii` bezeichnet die zusätzliche Fläche; der Gesamtwert enthält GRZ I.
* `grz_ii`, `grz_ii_faktor`, `grz_ii_kappung`: getrennter Nachweis und Regelgrenze.
* `nichtvollgeschoss_faktor`: Standardanrechnung der Aufenthaltsräume mit
  Umfassungswänden und zugehöriger Erschließung.
* `nichtvollgeschoss_planabweichung`: ausdrückliche Planfestsetzung darf den
  Standard für Nichtvollgeschosse ersetzen.
* `gemeinschaftsanteile`, `gemeinschaftsanteile_rechtsgrundlage`: Möglichkeit
  der Grundstückszurechnung; 1962 enthält keine vorkonfigurierte §-21a-Regel.
* `quellen`: rechtliche Bezugspunkte des Regelansatzes.

Ein Nutzer gibt Bauteiltyp und Fläche ein. Die Anrechnung wird angezeigt,
aber nicht im normalen Formular ausgewählt. Für Geschosse erscheinen nur die
nach dem effektiven Regelwerk benötigten Flächenfelder. Besondere
Nichtvollgeschoss-Festsetzungen werden einmal in den Projekt-Rechtsgrundlagen
konfiguriert; der Backend-Rechenkern wendet sie unabhängig vom Client an.
Gespeicherte ältere Sonderansätze bleiben erhalten und sind erkennbar.

Grundstücksformel:

`maßgebende Fläche = eigene Basisfläche − nicht maßgebende Flächen + Summe(Anlagenfläche × zugeordneter Anteil / 100)`

Zuschläge benötigen eine benannte Gemeinschaftsanlage außerhalb des eigenen
Baugrundstücks im selben Plangebiet und eine dokumentierte Zuordnung im
Bebauungsplan. Eine Eigentumsquote allein ersetzt keine Planfestsetzung.
Basis und Zuschläge bleiben getrennt, damit Speichern und erneutes Berechnen
nicht mehrfach zuschlagen. GRZ, GFZ und BMZ sowie die Umrechnung absoluter
Grundflächenlimits verwenden die berechnete Bezugsfläche. Die Zurechnung
verändert weder die Eigentumsfläche noch automatisch das geometrische Baufenster.

Die Dateien enthalten die implementierten Regelansätze. Sonderregelungen,
insbesondere weitere §-21a-Fälle, und die Vollgeschosseinstufung nach Landesrecht
sind nicht automatisch aus Plantext oder Geometrie erschlossen. Eine unabhängige
juristische Freigabe der historischen Regelwerke steht aus.
