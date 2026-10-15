# PDF-Livevorschau und mehrzeilige Textfelder

Beim Ausfüllen eines Formulars aktualisiert sich die PDF-Vorschau nach einer
kurzen Eingabepause von 180 ms automatisch. Text, Auswahlfelder und Checkboxen
werden mit derselben PDF-Füllfunktion verarbeitet wie beim Speichern. Die
ausgewählte Vorschauseite bleibt erhalten.

Die Vorschau erzeugt keine Datei, Dokumentversion oder Projekteingabe. Erst
„Speichern“ verwendet den bestehenden Projekt-/Dokumentablauf. Veraltete
Antworten können neuere Eingaben nicht überschreiben. Während einer neuen
Berechnung bleibt die letzte Vorschau sichtbar; Fehler erhalten die Eingaben.

## Mehrzeilige Felder

- Offizielle PDF-Textfelder mit dem PDF-Multiline-Flag erhalten eine mehrzeilige
  Eingabemaske. Auch vom übergeordneten Feld geerbte Flags werden berücksichtigt.
- Bei eigenen Formularfeldern kann im Vorlageneditor „Mehrzeiliges Textfeld“
  aktiviert werden. Breite und Höhe bestimmen den verfügbaren Platz.
- Zeilenumbrüche bleiben im Feldwert erhalten. Die Darstellung eigener Felder
  unterstützt zusätzlich Wortumbruch und verkleinert die Schrift bei Bedarf bis
  auf 4 pt. Reicht der Platz nicht aus, erscheint eine Fehlermeldung.
- Einzeilig definierte Felder bleiben einzeilig; Checkboxen haben keine
  Mehrzeilenoption.

## Modulgrenzen und Kompatibilität

Die Erkennung und PDF-Ausgabe liegen in `core-lib::formular`. Der neue
Tauri-Befehl `formular_livevorschau` ist ein zustandsloser Adapter ohne
Speicherzugriff. Die Oberfläche bündelt schnelle Eingaben und erlaubt höchstens
eine laufende Vorschauerzeugung sowie den neuesten ausstehenden Stand.

`Formularfeld` und `EigenesPdfFeld` erhalten das additive Merkmal `mehrzeilig`.
Bei älteren gespeicherten Vorlagen fehlt es und wird als `false` gelesen.
Eine Datenbankmigration ist nicht erforderlich. Die ursprünglichen Vorlagen
werden durch die Vorschauerzeugung nicht verändert.

## Prüfungen

Rust-Tests prüfen geerbte PDF-Flags, Adresszeilen, Textumbruch, Platzmangel und
das unveränderte Original. Ein Storage-Integrationstest vergleicht Vorschau und
gespeichertes Ergebnis und prüft die Dokumentversionierung. Vier Node-Tests
prüfen die zeitliche Bündelung, alte Antworten, Abbruch und Fehlerbehandlung.

Der Browser-Test `apps/desktop/tests/pdf-livevorschau.py` verwendet echte,
von Core erzeugte PDF-Dateien und den produktiven PDF.js-Renderer. Er prüft
sichtbare Änderungen, Seitenwechsel, schnelle Eingaben, Fehlerbehandlung und
das einmalige Speichern beziehungsweise Anlegen einer neuen Dokumentversion.
Die Testdaten entstehen durch `crates/storage/tests/pdf_livevorschau.rs` mit
gesetztem `BAUPROJEKT_PDF_LIVE_ARTEFAKTE`. Der Browser-Test ersetzt nur den
Tauri-Transport; die PDF-Inhalte stammen aus der tatsächlichen Füllfunktion.
