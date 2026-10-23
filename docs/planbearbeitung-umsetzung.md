# Umsetzung der Planbearbeitung

Auftrag: sämtliche Verbesserungen aus `analyse-planbearbeitung-2026-10-23.md`,
einschließlich der Ideen für die tägliche Bedienung, umsetzen.

## Arbeitspakete

- [x] Vollständiges Undo/Redo und transaktionales Ziehen/Abbrechen
- [x] Sichere Blattwechsel und bewusster Planversionsvergleich/Übertragung
- [x] Einzelprüfung pro Markierung und passende Übernahmekandidaten
- [x] Einheitliche Vorschauachsen und getrennte Maßeingabe
- [x] Anfragebindung für DXF und dauerhafte Projektaufmaße
- [x] Gemeinsame Bedienregeln, schneller konfigurierbarer Punktfang
- [x] Kontrollstrecke und nachvollziehbare Kalibrierung
- [x] Punkte bearbeiten, Mehrfachauswahl, numerische Maße und Ausrichten
- [x] Aussparungen, Teilen/Vereinigen und gedrehte Rechtecke
- [x] Stabile Ausschnitt-/Ebenenbeziehungen und bewusste Duplikate
- [x] Übernahmevorschau, Quellenprüfung und Sammelaktualisierung
- [x] Seitenpanels, Suche/Filter, Ansichtssicherung und Vollbildstatus
- [x] Lesemodus, Tastatur, Fokus, Touch und Kurzbefehle
- [x] Raster-/URL-Lebenszyklus, Miniaturen und große Objektlisten
- [x] Aufmaß-PDF und CSV mit dokumentierten Exportverträgen
- [x] Bessere Meldungen und erneute Prüfung bei Quellenkonflikten
- [x] Kompatibilitäts-, Core-, Integrations- und UI-Tests, Dokumentation

Vorhandene uncommittete Änderungen zur Dateiverwaltung und Quellenprüfung
gehören zur vorangegangenen Arbeit und bleiben erhalten.

## Ergebnis und Bedienung

Umgesetzt sind vollständiges Undo/Redo, sichere Blattwechsel, bestätigter
Versionsvergleich mit Übertragung, Einzelprüfung je Kontur, lokale Vorschauachsen,
numerische Maße und Kontrollstrecke. Benannte DXF-Messungen werden je
Dokumentversion gespeichert. Verspätete Lade-/Messantworten werden verworfen.
Ungespeicherte Messungen bleiben auch bei neu eintreffenden Planversionen geschützt.

Der gemeinsame räumliche Punktfang bietet Endpunkte, Mittelpunkte, Kanten und
Schnittpunkte. Punkte, Kanten und Gruppen lassen sich bearbeiten, verschieben,
ausrichten, duplizieren und löschen. Flächen können vereinigt, geschnitten,
abgezogen und geteilt werden; die zweite Hilfskontur bleibt bewusst erhalten
und muss für Summen fachlich eingeordnet werden. Kreise werden hierfür mit
512 Segmenten angenähert. Gedrehte Rechtecke werden echte Polygone.

Miniaturen wechseln nur die Ansicht. Neue Arbeitsseiten erhalten eigene Masken.
Übertragung erzeugt neue IDs und verlangt neue Kalibrierung; ursprüngliche
Bindungen bleiben erhalten. Ausschnitte besitzen stabile optionale Ebenenbezüge.
Duplizieren bietet Rahmen allein oder einschließlich neu zugeordneter Konturen.

Übernahmevorschauen zeigen Ziel, alten/neuen Wert und Quelle. Einsetzen im
Entwurf und Speichern sind getrennt. Sammelaktualisierung meldet Teilerfolge
und Fehler einzeln. Ein ausgewähltes Baugrundstück kann unabhängig geprüft und
gespeichert werden; globale Zuordnungsregeln und Versionssperren bleiben wirksam.

Aufgaben-/Objekt-/Eigenschaftenpanels bieten Suche und Filter, gespeicherte
Ansichten und Standardzuordnungen. Vollbild zeigt Speicherstatus und Aktion.
F1 zeigt Kurzbefehle: V wählt, M verschiebt, S misst, P zeichnet; Leertaste
verschiebt vorübergehend. Strg+Z/Strg+Umschalt+Z steuern Undo/Redo. Escape,
Fokusverlust und Pointer-Abbruch verwerfen laufende Ziehaktionen.

PDF-Aufmaßblatt und CSV sowie DXF-CSV lesen explizite gespeicherte Revisionen.
Vorheriges Speichern wird ausdrücklich bestätigt; Originaldateien bleiben erhalten.

## Verträge und Kompatibilität

Fachlogik liegt in core-lib; Tauri und Server nutzen dieselben Services.
`planaufmass_diagnose` liefert je Markierung Messwert oder Fehler; globale
Quellenfehler bleiben Request-Fehler. Flächenoperationen erzeugen Geometrien,
keine automatische rechtliche Anrechnung. `Planexportwunsch` enthält Aufmaß-ID,
Revision, Format und Ebenenfilter. PDF verwendet den Vertrag `Plananlage`;
die Planseite behält ihre physische Größe, Legende und Quellen folgen als Anlage.
CSV nutzt UTF-8 BOM, Semikolon, Dezimalkomma und Schutz gegen Tabellenformeln.
`CadProjektaufmass` nutzt den bestehenden Fachobjekt-/Revisionsvertrag.

Alte einfache Geometrien bleiben lesbar, fehlende Ausschnitt-Ebenen zulässig.
Präsentationsänderungen allein ändern keine fachlichen Bindungen. Die neuen
Enum-Varianten `Flaeche` und `Cadaufmass` erfordern die neue Anwendungsversion;
ältere Programme können diese neuen Daten nicht lesen. Keine automatische
Umrechnung alter Projekte und keine neue rechtliche Anrechnungsregel.
Dateientfernung nutzt gesonderte SQLite-/PostgreSQL-Migrationen und erhält Historie.

## Prüfungen und Grenzen

Core-, JavaScript-, Storage- und zwei Browserregressionen decken Fachwerte,
Quellenkonflikte, Geometrien, Exporte, Einzelgrundstücke und Bedienung ab.
Erfolgreich: 323 Core-Modultests, 20 betroffene Storage-Integrationstests,
13 JavaScript-Tests, beide Browserabläufe, Svelte-Prüfung ohne Fehler/Warnungen,
Desktop-/Server-Kompilierung und Produktionsbuild der Oberfläche.
Die strenge Clippy-Prüfung aller Targets von Core, Storage, Server und Desktop
ist mit `-D warnings` ebenfalls erfolgreich; generierte Plattformtypen stimmen überein.
Das zweitseitige Test-PDF wurde rasterisiert und visuell geprüft.
Browser nutzen echte PDF.js-/Canvas-Darstellung mit kontrolliertem IPC-Testtransport.
Geprüft werden Undo/Redo, Einzelfehler, numerische Eingabe, Mehrfachauswahl,
Ausschnittbezüge, Abbruch, Blattwechsel, Übernahmevorschau, Vergleich/Übertragung,
Vollbild, Lesemodus und DXF-Lade-/Speichervorgänge. Die neuen UI-Tests sind in CI.
Der PDF-Testexport wird zusätzlich rasterisiert und visuell kontrolliert.

Bedingt vorgeschlagene spätere OCR/Texterkennung, Tangenten-/Kurvenfang, affine
Scanentzerrung und ein anderer Binärtransport/Overlay-Renderer sind nicht
Bestandteil dieser Implementierung. Fang verarbeitet gerade Kanten und begrenzt
Schnittpunktsuche in sehr dichten Bereichen. Ein repräsentativer Leistungstest
mit großen realen Plänen sowie manuelle Touch-/Windows-WebView2-Prüfung stehen aus.
Das Desktop-Update ist noch nicht gebaut oder installiert.
