# Sachlicher Stil der Oberfläche

Stand: 27.10.2026. Ersetzt die Optik des „Behördenstils“ aus `docs/ui-umsetzung`
durch ein Erscheinungsbild, wie es Planer von CAD- und AVA-Programmen kennen.
Die Aufteilung der Masken, die Register und die Gliederung bleiben unverändert.

## Regeln

- Dunkle Werkzeugleiste (`.app-werkzeuge`) oben im Fenster, darunter helle,
  flache Arbeitsflächen. Nur die Leiste ist dunkel; ein dunkles Theme für die
  ganze Anwendung bleibt wie bisher über `data-theme="dark"` möglich.
- Neutrale Grautöne, Ecken von höchstens 2 px, keine Schatten auf Flächen.
  Paletten und Masken werden durch Linien und graue Kopfleisten getrennt.
- Ein gedecktes Stahlblau (`--accent`) ist die einzige Akzentfarbe: Auswahl,
  aktives Register, Hauptknopf. Zustände (Warnung, Fehler, in Ordnung)
  behalten ihre eigenen Farben.
- Tabellen zeigen Zeilenlinien und einen grauen Kopf in Großbuchstaben statt
  eines Gitters; die überfahrene Zeile hebt sich ab.
- Projekttabs und Register sind flache Reiter; der aktive Reiter ist weiß mit
  einer Akzentlinie oben.
- Symbolknöpfe zeigen nur das Symbol. Name und Kürzel erscheinen als Tooltip
  und stehen als `aria-label` bzw. `aria-keyshortcuts` bereit
  (`Symbolknopf.svelte`, Klasse `.symbolknopf`).
- Unverändert: Pflichtfelder gelb hinterlegt, schreibgeschützte Felder
  gestrichelt, Schriftgröße 13 px und die dichte Anordnung der Fachmasken.

## Editoren

Der Aufmaß-Editor (`Planaufmass.svelte`, `PdfPlan.svelte`) folgt dem Schnitt
eines CAD-Arbeitsplatzes und ist Vorbild für weitere Zeichenbereiche:

- `Fachliste` im Modus `kompakt`: Bei geöffnetem Eintrag verschwinden Titel,
  Listenknöpfe und Pflichthinweis; es bleibt eine schmale Speicherleiste mit
  Status, Speichern, Schließen und den Symbolknöpfen des Editors (Snippet
  `werkzeuge`).
- Senkrechte Werkzeugleiste links, Gruppen durch Linien getrennt, nur Symbole
  (`WerkzeugIcon.svelte`, 16 × 16, Strich in `currentColor`). Ein gesperrtes
  Werkzeug nennt im Tooltip den Grund.
- Zeichenfläche in der Mitte mit Pfadzeile oben (Unterlage › Blatt ›
  Ausschnitt), Ansichtsleiste (Zoom, Drehen, Seiten, Fang) und einer Zeile
  Statusmeldung unten; Hintergrund `--fenster`.
- Rechts Paletten als `<details class="palette">` mit grauer Kopfzeile in
  Großbuchstaben, 320 px breit; unter 1100 px rutschen sie unter die Fläche.
- Im Vollbild trägt der Editor eine eigene Kopfleiste mit denselben Knöpfen.
- Zeichnungs-Tabs über dem Editor (`role=tablist`): flache Reiter wie die
  Projekttabs, aktiver Tab weiß mit Akzentlinie, ungespeichert mit eckigem
  Kennzeichen, Schließen als Symbolknopf.

## Umsetzung

Alle Werte stehen als Variablen in `apps/desktop/src/app.css` im Abschnitt
„Sachlicher Stil“; er überschreibt die früheren Werte am Ende der Datei.
Komponenten verwenden ausschließlich diese Variablen. Die Bildschirmfotos
unter `docs/ui-umsetzung` und `docs/handbuch/screenshots` stammen aus den
UI-Tests `projekttabs.py` und `handbuch-ui.py` und wurden mit diesem Stand
neu erzeugt.
