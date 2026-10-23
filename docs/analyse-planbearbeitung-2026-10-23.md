# Analyse der Planbearbeitung und des PDF-Aufmaßes

Stand: 23.10.2026. Grundlage: Quellcode, vorhandene Verträge und Tests.
Die Vorschläge sind noch nicht implementiert. Die Korrektur der
Nachweis-Quellenprüfung ist davon getrennt und bereits umgesetzt.

## 1. Wiederkehrende Meldung zur Nachweisprüfung

**Belegte Ursache:** Mit der Erweiterung zum Entfernen von Projektdateien
unterscheiden sich vollständiger Versionsbestand und aktive Dateiliste.
Die Lesesitzung las über `list_for_project` den vollständigen Bestand,
verglich ihn abschließend aber mit `latest_for_project`, das entfernte
Dateien ausblendet. Ein unverändert vorhandenes, entferntes Dokument genügte
damit für die Meldung „Quellen während der Nachweisprüfung geändert“.
Erneutes Aktualisieren konnte diesen Unterschied nicht beheben.

Das ist eine Regression im Zusammenspiel mit der vorherigen Lösch-Erweiterung,
kein Hinweis auf fehlerhafte Benutzereingaben.

**Korrektur:** Die Lesesitzung hält vollständige Versionen und aktive
Dokumentidentitäten getrennt fest. Die Abschlussprüfung vergleicht den
vollständigen Versionsbestand mit demselben Bestand und die aktiven
Identitäten separat. Aktive Abfragen innerhalb der Sitzung liefern nur aktive
Dokumente; historische Versionsabrufe behalten den vollständigen Bestand.
Dokumentversionen werden unabhängig von der Reihenfolge auf den neuesten
Stand je Identität zusammengefasst.

Keine Datenmigration, Änderung gespeicherter Nachweise oder Abschaltung der
Konkurrenzprüfung. Tatsächliche Änderungen an Projektversion, Dokumenten
oder gelesenen Fachmodulen bleiben ein Grund zum Abbruch.

Quellen: [Lesesitzung](../crates/core-lib/src/lesesitzung.rs),
[Regressionstests](../crates/storage/tests/nachweis_quellen.rs).

Weitere Verbesserung: Bei echten Konflikten die betroffene Quelle mitloggen
und eine Aktion „Erneut prüfen“ anbieten. Ein begrenzter Wiederholungsversuch
wäre nur für diese lesende Auswertung sinnvoll; Schreibvorgänge und fachliche
Validierungsfehler dürfen dadurch nicht automatisch wiederholt werden.

## 2. Was bereits vorhanden ist

| Bereich | Heutiger Stand |
| --- | --- |
| PDF-Unterlagen | Konkrete DMS-Version und Seite; Original wird nicht überschrieben |
| Zeichnen | Polygon, Rechteck, Kreis, Strecke, Höhenpunkt; Auswahl, Verschieben und Duplizieren |
| Orientierung | Zoom, Zoomfenster, Einpassen, Ansichtsdrehung, Vollbild, Seitenminiaturen |
| Präzision | Maßstab oder Referenzstrecke; Endpunkt-, Mittelpunkt- und Kantenfang; Shift-Winkelraster; numerische Maßeingabe |
| Organisation | Ebenen mit Sichtbarkeit/Sperre; Ausschnitte mit eigenem Maßstab und Höhenbezug |
| Fachliche Zuordnung | Bedeutung, Baugrundstück, Gebäude, Geschoss, Raum-Höhenzonen |
| Übernahmen | Gebäude/Geschosse, Raumbuch, Fachberechnungen und manuelle Abstandswände über typisierte Bindungen |
| Aktualität | Bindungsstand der betreffenden Markierung; eine reine Ebenenänderung veraltet die fachliche Bindung nicht |
| Speicherung | Revisionen, lokale Entwurfssicherung, Wiederherstellung und Versionskonflikte |
| DXF | Eigener Canvas-Viewer, Ebenen, Punktfang, Längen/Flächen, Auswahl geschlossener Konturen, Baufenster/Vorhaben |

Die Trennung von unverändertem Quelldokument, Zeichenmaske und Core-Messwerten
ist eine gute Grundlage. Neue Funktionen sollten diese Verträge erweitern.
Eine zweite Fachberechnung im Frontend oder ein pauschaler Neubau wäre
dafür nicht erforderlich.

## 3. Belegte Schwachstellen mit hoher Priorität

### P1 – Rückgängig stellt keinen vollständigen Bearbeitungsstand her

[Planaufmass.svelte:174](../apps/desktop/src/lib/planaufmass/Planaufmass.svelte#L174)
speichert nur `markierungen`. Maßstäbe, Höhenbezüge, Ebenen, Ausschnitte und
deren Eigenschaften fehlen in Undo/Redo. Viele Eigenschaften werden direkt
gebunden und erzeugen keinen eigenen Rückgängig-Schritt.

Besonders problematisch: Beim Entfernen eines Ausschnitts wird die
Markierungsliste gesichert, der Ausschnitt selbst jedoch nicht. Rückgängig
kann danach Markierungen mit einer wiederhergestellten `ausschnitt_id`
erzeugen, zu der kein Ausschnitt mehr existiert.

**Vorschlag:** Zusammengehörige Änderungen als eine Bearbeitungsaktion führen:
Geometrie plus Zuordnung plus betroffene Ausschnitt-/Ebenendaten. Ziehen ist
eine Aktion vom Beginn bis zum Loslassen. Auch Kalibrierung, Bezugslinie,
Eigenschaften und Ausschnittentfernung müssen vollständig rückgängig sein.
Strg+Umschalt+Z zusätzlich als Wiederholen unterstützen.

**Abnahme:** Ausschnitt samt Bezugslinie entfernen und rückgängig machen;
anschließend stimmen Referenzen, Messwerte und Speicherfähigkeit wieder.

### P1 – Seiten-/Unterlagenwechsel versetzt vorhandene Konturen

[Planaufmass.svelte:219](../apps/desktop/src/lib/planaufmass/Planaufmass.svelte#L219),
[Planaufmass.svelte:263](../apps/desktop/src/lib/planaufmass/Planaufmass.svelte#L263):
Der Wechsel verändert Dokument/Seite und setzt Maßstäbe und Höhenbezüge zurück,
lässt aber vorhandene Markierungen und Ausschnitte an denselben Koordinaten
stehen. Der Hinweis verlangt eine Kontrolle; eine Bestätigung vor dem Wechsel
fehlt. Nach erneuter Kalibrierung können so alte Konturen auf einem anderen
Blatt wieder Messwerte liefern. Das aktuelle UI-Testskript erwartet dieses
Verhalten ausdrücklich; es wäre daher eine bewusste Verhaltensänderung.

**Vorschlag:** Zwischen „anderes Blatt öffnen“ und „Aufmaß auf neue Unterlage
übertragen“ unterscheiden. Anderes Blatt öffnet ein eigenes Aufmaß.
Übertragung erhält den alten Stand, zeigt alte/neue Unterlage überlagert und
verlangt die Bestätigung von Ausrichtung, Maßstab und Konturen. Bestehende
Nachweisbezüge bleiben am bisherigen Stand.

### P1 – Abgebrochene Zeigeraktion kann Änderungen übernehmen

[PdfPlan.svelte:365](../apps/desktop/src/lib/planaufmass/PdfPlan.svelte#L365),
[PdfPlan.svelte:465](../apps/desktop/src/lib/planaufmass/PdfPlan.svelte#L465):
`pointercancel` verwendet denselben Handler wie reguläres Loslassen.
Dieser übernimmt bearbeitete Geometrie. Ausschnitt-Rahmen werden bereits bei
jeder Zeigerbewegung in den Entwurf geschrieben; dafür fehlt ein Undo-Schritt.

**Vorschlag:** Vorschau, Übernehmen und Abbrechen trennen. Bei Escape,
`pointercancel`, Fokusverlust oder Wechsel in eine Zoomgeste den Anfangsstand
wiederherstellen. Rahmenänderungen erst zum Abschluss als eine Aktion speichern.

### P1 – Ein ungültiger Eintrag lässt andere Messwerte verschwinden

[Planaufmass.svelte:80](../apps/desktop/src/lib/planaufmass/Planaufmass.svelte#L80),
[quellen.ts:9](../apps/desktop/src/lib/planaufmass/quellen.ts#L9),
[service.rs:157](../crates/core-lib/src/planaufmass/service.rs#L157):
Die Editor-Prüfung und das Laden von Übernahmekandidaten prüfen das ganze
Aufmaß. Bei einem Fehler wird die gesamte Messwertliste geleert.
Ein ungültiges Polygon oder ein nicht mehr vorhandenes Geschoss kann somit
auch gültige Markierungen im Editor und in Auswahllisten ohne Messwert lassen.
Der Core löst bestehende Bindungen bereits gezielter einzeln auf; diese
Verbesserung erreicht die allgemeinen Kandidatenlisten noch nicht vollständig.

**Vorschlag:** Einen additiven Prüfvertrag mit globalen Fehlern und Ergebnissen
je Markierungs-ID anbieten. Gültige Messwerte weiter anzeigen; fehlerhafte
Markierungen sichtbar kennzeichnen und direkt auswählbar machen. Bestehende
strenge Speicher-/Übernahmeprüfungen erhalten, bis ihre Verbraucher ausdrücklich
auf den erweiterten Vertrag umgestellt sind.

### P1 – Höhenvorschau und endgültige Messung können unterschiedliche Achsen verwenden

[PdfPlan.svelte:406](../apps/desktop/src/lib/planaufmass/PdfPlan.svelte#L406):
Die Streckenvorschau berechnet Δh und Horizontale aus Seiten-x/y. Der Core
verwendet bei Schnitten mit Bezugslinie deren lokale Achsen. Bei einer schrägen
Bezugslinie kann deshalb die laufende Vorschau andere Teilmaße zeigen als das
endgültige Ergebnis. Höhenpunkte besitzen bereits eine eigene bezugslinienbezogene
Vorschau; bei Strecken fehlt die entsprechende Berücksichtigung.

**Vorschlag:** Anzeigen derselben Bezugsebene und eindeutiger Vorschau-Status.
Gemeinsame Koordinatentransformation für die Anzeige, verbindliche Messung
weiterhin im Core. Abnahmetest mit einer 30° geneigten Bezugslinie und einem
bekannten Höhen-/Horizontalmaß.

### P1 – Maßeingabe befindet sich in einem weiteren Formular

[PdfPlan.svelte:504](../apps/desktop/src/lib/planaufmass/PdfPlan.svelte#L504)
legt ein `form` an; der übergeordnete
[Fachliste.svelte:236](../apps/desktop/src/lib/Fachliste.svelte#L236)
ist bereits ein Formular. Verschachtelte Formulare sind eine ungültige
HTML-Struktur. Der innere Submit verhindert zwar die Standardaktion, stoppt
aber nicht ausdrücklich das Weiterreichen zum äußeren Speicherhandler.
Die konkrete Wirkung im WebView muss interaktiv reproduziert werden.

**Vorschlag:** Die Maßeingabe als eigenes Eingabefeld/Popover ohne verschachteltes
Formular umsetzen. Enter übernimmt ausschließlich das Maß und bringt den Fokus
zurück auf die Zeichenfläche. Speichern bleibt eine getrennte Aktion.

### P1 – DXF-Ladevorgänge haben keinen Schutz gegen verspätete Antworten

[Plaene.svelte:240](../apps/desktop/src/lib/Plaene.svelte#L240):
`planLaden` schreibt das erhaltene Ergebnis ohne Anfragekennung in den Zustand.
Bei schnellem Wechsel A → B kann eine langsamere A-Antwort B wieder ersetzen.
Auch abgeschlossene Messungen und die Neuberechnung nach Einheitenwechsel
besitzen keinen entsprechenden Plan-/Einheitenvergleich.

**Vorschlag:** Projekt, Dokumentversion, Einheit und Anfragegeneration beim Start
festhalten. Eine Antwort darf nur den dazugehörigen Bearbeitungsstand verändern.
Die vorhandenen Schutzmuster aus dem PDF-Editor dafür wiederverwenden.

## 4. Weitere konkrete Verbesserungen

### P2 – Einheitliche Bedienung für PDF und DXF

PDF verwendet V für Auswahl und M für Verschieben; DXF verwendet V für
Verschieben und W für Flächenauswahl. Im PDF zoomt Strg+Rad, im DXF schon das
normale Rad. DXF-Messungen werden beim Planladen geleert und sind nicht wie
PDF-Markierungen versioniert gespeichert.

**Vorschlag:** Gemeinsame Werkzeugnamen, Kürzel, Zoomregeln und Statusleiste.
Einmalige Messungen ausdrücklich als temporär kennzeichnen; eine Aktion
„Als Projektaufmaß speichern“ anbieten. Den etablierten PDF-Fachvertrag und
den DXF-Import über Adapter verbinden, statt beide Renderer sofort zu ersetzen.

### P2 – Fang gezielt auswählbar und bei großen Plänen schneller machen

[pdfGeometry.ts:72](../apps/desktop/src/lib/planaufmass/pdfGeometry.ts#L72)
durchläuft beim Fang Punkte und Segmente linear. Zusätzlich kopiert
[PdfPlan.svelte:229](../apps/desktop/src/lib/planaufmass/PdfPlan.svelte#L229)
bei Zeigerbewegungen die Punktliste. Der Adapter lässt bis zu ungefähr
100.000 Geometrieelemente zu. Das ist ein erkennbares Skalierungsrisiko;
eine konkrete Geschwindigkeitsmessung mit großen Kundenplänen steht aus.

**Vorschlag:** Räumlicher Index, vorberechnete Mittelpunkte und Suche nur in der
Bildschirmtoleranz. Fangarten einzeln schalten: Endpunkt, Mittelpunkt, Kante,
Schnittpunkt, eigene Kontur. Eigengeometrie auch mit Kanten anbieten.
Shift-Winkelbindung sollte nach dem Fang geprüft werden, damit ein naher
Vektorpunkt die gewünschte Richtung nicht wieder aufhebt. Rasterpläne klar
als solche anzeigen; OCR/Vektorisierung wären gesonderte, bestätigungspflichtige
Erfassungshilfen und keine automatisch verlässlichen Fachquellen.

### P2 – Kalibrierung überprüfbar machen

Aktuell gibt es einen Maßstab oder eine Referenzstrecke. Ergänzen würde ich
sichtbare Referenzpunkte, eine unabhängige Kontrollstrecke sowie eine Anzeige
der relativen Abweichung. Eine Flächenmessung vergrößert Maßstabsfehler
quadratisch; bei verzerrten Scans ist eine einzige Strecke daher nur begrenzt
aussagekräftig. Zunächst warnen und die Unterlage prüfen, bevor eine affine
Entzerrung als eigener versionierter Vorgang eingeführt wird.

Sonderfälle wie CropBox, PDF-eigene Rotation und UserUnit gehören in eine
gezielte Testmatrix. Der Formularadapter berücksichtigt solche Metadaten;
der Planpfad muss seine Koordinatenkonvention durchgängig belegen.
Hier wird ausdrücklich kein bereits reproduzierter Skalierungsfehler behauptet.

### P2 – Geometrie bearbeiten statt neu zeichnen

Derzeit lassen sich vorhandene Stützpunkte ziehen, aber zusätzliche
Polygonpunkte, das Entfernen einzelner Ecken, Mehrfachauswahl und gemeinsame
Verschiebung sind nicht als Werkzeuge vorhanden.

**Vorschlag:** Zuerst Punkt einfügen/entfernen, Mehrfachauswahl, Ausrichtung und
numerisch bearbeitbare Kantenlänge/Winkel. Danach Aussparungen, Flächenabzug,
Teilen/Vereinigen und gedrehte Rechtecke. Innenringe und boolesche Operationen
benötigen eine kompatible Erweiterung des Core-Geometrievertrags; negative
Flächen oder übereinandergelegte Polygone dürfen Aussparungen nicht nur vortäuschen.

### P2 – Ausschnitte und Ebenen über Identitäten verbinden

[Planaufmass.svelte:233](../apps/desktop/src/lib/planaufmass/Planaufmass.svelte#L233)
findet die passende Zeichenebene anhand ihrer Bezeichnung. Umbenennung und
gleiche Namen können diese Zuordnung mehrdeutig machen. Ausschnittduplikate
kopieren Rahmen/Metadaten, aber nicht automatisch zugehörige Markierungen oder
eine eigene Ebene; diese Bedeutung von „Duplizieren“ sollte klarer sein.

**Vorschlag:** Optionale stabile Ebenenreferenz am Ausschnitt mit kompatiblem
Fallback für Altbestände. Getrennte Aktionen „Nur Rahmen kopieren“ und
„Rahmen mit Markierungen kopieren“. Neue IDs vergeben, fachliche Bindungen
nicht unbemerkt auf die Kopie umstellen.

### P2 – Übernahmen verständlich und einheitlich machen

„Aufgaben → Übernehmen“ speichert das Zielobjekt direkt; die Flächenübernahme
im Gebäudefeld setzt dagegen erst einen Entwurfswert ein. Beide Wege sind
vorhanden und sinnvoll, sollten aber ihre Speicherwirkung sichtbar benennen.

**Vorschlag:** Vor der Übernahme Ziel, alten/neuen Wert, Einheit und betroffene
Bindung anzeigen. „Im Entwurf einsetzen“ und „Übernehmen und speichern“
unterscheiden. Bei geänderten Quellen eine Liste der tatsächlich betroffenen
Felder mit Sammelprüfung anbieten. Gebäude-/Geschosszuordnung bereits beim
Zeichnen vorbelegen und den Aufgabenablauf zum primären Einstieg machen.
Veraltete Gebäudequellen möglichst nur für die betroffene Grundstückseinheit
blockieren; der bestehende Dokumentationsstand nennt hier bereits eine breitere
Blockierung als offenen Punkt.

### P2 – Arbeitsfläche entlasten und Zustand behalten

Das Seitenpanel kombiniert Aufgaben, Ausschnitte, Ebenen, Markierungen und
Eigenschaften in einer langen Spalte. Auf kleineren Fenstern wird es unter
die Zeichenfläche verschoben.

**Vorschlag:** Oben Unterlage/Version/Seite und Speichern; schmale Werkzeugleiste;
umschaltbare Seitenpanels „Aufgaben“, „Objekte“, „Eigenschaften“. Suchfeld und
Filter nach Gebäude, Geschoss, Bedeutung und Fehlerstatus. Ansicht, Auswahl,
Zoom und offenes Panel je Projekt/Aufmaß merken. Vollbild sollte sichtbaren
Speicherstatus und einen dauerhaft erreichbaren Rückweg enthalten.

### P2 – Betrachtung auch mit Leserecht vollständig bedienbar machen

Das PDF-Aufmaß verwendet `Fachliste` ohne `lesendBedienbar`. Deren äußeres
deaktiviertes Fieldset sperrt mit den Eingaben auch enthaltene
Zoom-/Seiten-/Ausschnittbuttons. Die SVG hat eigene Eingabeguards; daraus
entsteht eine uneinheitliche Betrachtungsbedienung.

**Vorschlag:** Navigation und Betrachtung auch ohne Bearbeitungsrecht freigeben;
Schreibaktionen gezielt sperren. Dazu sichtbarer Fokus, größere Trefferflächen
für Eckpunkte, numerische Alternativen zum Ziehen und ein Kurzbefehle-Dialog.
Im DXF-Viewer braucht das Canvas eine benannte, fokussierbare Bedienfläche;
globale Kürzel ersetzen keine vollständige Tastaturbedienung.

### P2 – Laden, Rasterbilder und lange Listen robuster machen

Das PDF wird als Base64 geladen; Seiten und Vektorgeometrie werden im Client
aufbereitet. Nachrastern erhöht die Schärfe, aber alle Markierungen werden als
SVG mit Beschriftungen ausgegeben. Seitenminiaturen werden nur für die ersten
60 Seiten gerendert. Ein verspätetes Nachraster-Ergebnis wird bei Seitenwechsel
ignoriert, seine neu erzeugte Objekt-URL dabei aber nicht freigegeben
([PdfPlan.svelte:116](../apps/desktop/src/lib/planaufmass/PdfPlan.svelte#L116)).

**Vorschlag:** Objekt-URLs in allen Abbruchpfaden freigeben, Miniaturen im
sichtbaren Bereich nachladen und Markierungs-/Messwertzugriffe über Maps
organisieren. Zeichnen mit vielen Objekten messen, unsichtbare Beschriftungen
auslassen und erst bei belegtem Bedarf einen anderen Overlay-Renderer einsetzen.
Dateitransport später über eine autorisierte Binär-/Dateiquelle entlasten.

### P3 – Austausch und nachvollziehbare Ausgabe

Die geprüfte Zeichenoberfläche besitzt keine direkte Ausgabe des PDF-Blatts
mit der kompletten sichtbaren Zeichenmaske. Vorhandene Fachnachweisdrucke
führen bereits Quellenangaben auf; das ersetzt keine Plananlage mit Konturen.

**Vorschlag:** „Aufmaßblatt als PDF“ mit Originalplan, gewählten Ebenen,
Konturbezeichnungen, Maßstab, Referenzstrecke, Legende, Unterlagenversion und
Bearbeitungsstand. Separater CSV-Export für Messwerte und Quellen. Die Ausgabe
erzeugt ein neues Dokument, überschreibt keine Unterlage und verwendet eine
definierte Export-Schnittstelle des Aufmaßmoduls.

## 5. Empfohlene Reihenfolge

| Stufe | Inhalt | Abnahmekriterium |
| --- | --- | --- |
| 1: Verlässlichkeit | Vollständiges Undo, sichere Blattwechsel, Cancel-Verhalten, Anfragegenerationen, getrennte Maßeingabe | Keine falschen Zuordnungen und keine unerwartet übernommenen Änderungen in gezielten UI-Tests |
| 2: Messqualität | Einzelfehler je Markierung, konsistente Vorschauachsen, Kalibrierkontrolle, schneller/steuerbarer Fang | Eine fehlerhafte Kontur beeinträchtigt andere Konturen nicht; Vorschau und Core stimmen im Test überein |
| 3: Büroablauf | Einheitliche Werkzeuge, Aufgabenführung, Übernahmevorschau, Filter, Lesemodus, persistente DXF-Aufmaße | Vom markierten Planbereich zum geprüften Zielfeld ohne unklare Speicherwirkung |
| 4: Ausbau | Punktbearbeitung, Mehrfachauswahl, Aussparungen, Vergleich von Planversionen, Plananlagenexport | Neue Geometrien und Exporte haben dokumentierte Verträge und bleiben mit Altprojekten kompatibel |

Ich würde mit Stufe 1 beginnen. Zusätzliche Zeichenwerkzeuge lohnen sich erst
richtig, wenn Rückgängig, Quellenwechsel und Abbruch zuverlässig sind.

## 6. Prüfungen und verbleibende Grenzen

In dieser Analyse erfolgreich ausgeführt:

- 2 neue Storage-Integrationstests: kein dauerhafter Konflikt nach Entfernung;
  echte neue Version bzw. Entfernung während der Auswertung wird weiterhin
  erkannt; nach abgeschlossener Änderung ist eine neue Prüfung möglich.
- 13 Core-Modultests des PDF-Aufmaßes, unter anderem Kalibrierung, Geometrie,
  geneigte Bezugslinie, Höhen, Altbestände und Bindungsaktualität.
- 6 JavaScript-Tests für PDF-Geometrieextraktion, Transformation, Punktfang,
  Vorschau und Verschiebung.
- 4 bestehende Storage-Integrationstests für Aufmaß, DIN 277, Raumbuch,
  Wandhöhen, Ausschnitte, Aufgaben und Wohnflächen-Höhenzonen.
- 8 bestehende Plattform-Integrationstests, einschließlich Nachweisarchiv,
  Regelaktualität, Dokumentversionen und Berechtigungen.
- 1 Lesesitzungs-Modultest für wiederholte und parallele Quellenabfragen.

Die Rust-Formatierung und der Diff wurden ebenfalls geprüft.

Das bestehende Playwright-UI-Skript wurde inhaltlich geprüft, hier aber nicht
ausgeführt: Im verfügbaren Python fehlt das Paket `playwright`. Deshalb sind
die UI-Befunde als Codeanalyse zu verstehen; konkrete Reproduktionsfälle
stehen jeweils dabei. Eine interaktive Prüfung unter Windows-WebView2,
Touchbedienung und ein Leistungstest mit großen realen Plänen stehen aus.
Es wurden keine produktiven Projektdateien oder Nachweise verändert.

Als ergänzende Prüfliste für Fokus, Tastatur, Formulare und Rückmeldungen wurden
die [Web Interface Guidelines](https://raw.githubusercontent.com/vercel-labs/web-interface-guidelines/main/command.md)
verwendet. Die fachlichen und architektonischen Befunde stammen aus dem lokalen
Code und den [Entwicklungsregeln](entwicklungsregeln.md).
