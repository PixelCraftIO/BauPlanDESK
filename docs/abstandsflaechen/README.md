# Projektbezogene Abstandsflächen

## Bedienung

Im Projekt **Berechnungen → Abstandsflächen** einen Abstandsplan anlegen.
Mehrere Pläne können Varianten oder Bauabschnitte abbilden. Der bisherige Rechner
bleibt unter **Einfache Tiefenvorprüfung** verfügbar; alte Nachweise bleiben lesbar.

1. **Grundlagen:** Gemeinde, konkrete Rechtsfassung und Anwendbarkeit, Baugebiet,
   Satzungs-/Planprüfung, Gelände-/Höhen- und Koordinatenbezug dokumentieren.
   Originalunterlagen mit ihrer konkreten Version zuordnen.
2. **Gebäudeumrisse:** Eckpunkte des Gebäudes in Umlaufrichtung erfassen, je
   Ecke optional die Geländehöhe, dazu eine Vorgabe der Wandoberkante (z. B.
   Traufe, auch aus Gebäude/Geschosse übernehmbar). **Seiten als Wände
   übernehmen** erzeugt je Seite eine Wand; Lage, Außenseite (aus der
   Umlaufrichtung), Gebäude, Grundstück und Eckhöhen führt danach der Umriss.
   Eckpunkte lassen sich aus einer geschlossenen DXF-Kontur übernehmen.
3. **Wände / Bauteile:** Je Gebäudeseite das **Höhenprofil** erfassen:
   beliebig viele Geländepunkte und Punkte der Wandoberkante, jeweils als
   Station in Metern ab Wandanfang. Zwei Oberkantenpunkte an derselben Station
   bilden einen Höhensprung und damit einen eigenen Wandteil; weitere
   Wandteilgrenzen können ausdrücklich angegeben werden. Geländepunkte lassen
   sich aus Höhenkoten oder Höhenpunkten einer DXF vorschlagen. Einzelne Wände
   ohne Umriss bleiben wie bisher mit Anfang/Ende und Außenseite möglich.
   Dach-/Giebelansatz, Tiefenfall und gegebenenfalls besonderen Wandfall wählen.
4. **Lageplan / Grenzen:** Rechtliche Grundstückseinheiten mit zugehörigen
   Flurstücken und Umrissen erfassen. Innenringe dienen tatsächlich nicht
   zugehörigen Grundstücksflächen. GRZ-/GFZ-Abzüge verändern diese Grenzen nicht.
   Eine Grundbuchdatei ist keine Pflicht.
5. **Sonderfälle / Sicherungen:** Andere Baukörper, belegte Sicherungsflächen
   und zulässige öffentliche Teilflächen eingeben. Besondere Wandansätze brauchen
   Rechtsgrundlage/Entscheidung, Geltungsumfang, Wirksamkeit, Beleg und Tiefe.
   Ein eigenes nachgewiesenes Abstandsflächenpolygon kann übernommen werden.
6. **Prüfbericht:** Vollständigkeit von Wänden, Geländeabschnitten, Ecken,
   Grenzen und Sonderfällen prüfen. Berechnen, Hinweise abarbeiten, speichern
   und anschließend gegebenenfalls erneut fachlich bestätigen.

Rechts- und Umfangsbestätigungen benötigen die Berechtigung der Projektleitung.
Planänderungen setzen beide Bestätigungen beim Speichern zurück. Danach erneut
bestätigen und speichern. Regelinhalt mit gleicher Kennung und Version darf nicht
still geändert werden. Versionierte eigene JSON-Regelpakete können geladen werden;
für weitere Länder ist ein passendes eigenes Paket erforderlich.

## Berechnung und fachliche Grenzen

Implementiert sind mehrere Gebäude/Wandabschnitte, getrennte Grundstückseinheiten,
konfigurierbare Tiefen- und Dachregeln, Grenzanlagenbedingungen und belegte
besondere Tiefen-/Flächenansätze. Der Kern erzeugt Konfliktpolygone und prüft
Grundstücksgrenzen einschließlich Innenringen, wirksame Zusatzflächen, andere
Baukörper sowie Überdeckungen zwischen Abstandsflächen.

### Wandhöhe aus Gelände- und Oberkantenprofil

Der Core interpoliert Gelände und Wandoberkante linear zwischen den Punkten und
bildet daraus die Wandhöhe H(s) = Oberkante − Gelände über die ganze Wandlänge.
Beide Profile müssen von 0 m bis zur Wandlänge reichen; eine Oberkante unter dem
Gelände bleibt offen. Wandteile entstehen an Höhensprüngen der Oberkante und an
ausdrücklichen Wandteilgrenzen. Wie aus dem Verlauf je Wandteil die maßgebliche
Wandhöhe wird, legt das Regelpaket fest (`wandhoehe`):

| Ansatz | Rechnung | Abstandsfläche |
| --- | --- | --- |
| `maximal` (Pakete ohne Angabe) | größte Wandhöhe je Wandteil | Rechteck je Wandteil |
| `mittel_kanten` | Mittel der Wandhöhen an den Gebäudekanten bzw. Wandteilgrenzen | Rechteck je Wandteil |
| `mittel_profil` | flächengemittelte Wandhöhe | Rechteck je Wandteil |
| `punktweise` | Wandhöhe an jeder Stelle | Polygon mit wechselnder Tiefe; Knick, wo die Mindesttiefe greift |

Weicht das Profil zwischen den Kanten um mehr als 0,25 m vom gleichmäßigen
Verlauf ab, nennt die Herleitung beim Kantenmittel einen Hinweis zur fachlichen
Prüfung der Wandteilung. Zur Wandhöhe kommt der Dachanteil: Dachflächen nach den
Neigungsschwellen, bei konfigurierter Gaubenregel Dächer mit Gauben/Dachaufbauten
über dem Wandanteil, Giebelflächen bei konfigurierter Giebelregel automatisch aus
Giebelhöhe und den Neigungen beider Dachseiten, sonst mit hergeleitetem Faktor.
Besondere Tiefenfälle (zu öffentlichen Flächen, Wohngebäude GK 1/2) brauchen eine
Begründung; sieht das Paket für das Gebiet keine abweichende Tiefe vor, bleibt
die Regeltiefe.

Überdeckungen gelten nur dann ohne gesonderte Prüfung als zulässig, wenn das
Regelpaket eine Winkelregel enthält und die Wände in einem größeren Winkel
zueinander stehen (0° = parallel gegenüber, 90° = Innenecke). Der Winkel steht
im Ergebnis und im Bericht.

Die Standardgeometrie besteht aus senkrecht zur Wand gerichteten Flächen.
Komplexe Eckzuschnitte und besondere Flächenumrisse benötigen einen belegten
besonderen Nachweis. Ungeklärte Überdeckungen verhindern eine vollständige Freigabe.
Andere Grundstückseinheiten decken Konflikte nicht automatisch.

Garage/Carport allein beweist keine Privilegierung: Nutzung und sämtliche
Anwendungsbedingungen müssen geprüft sein. Grenzkennungen und Längen müssen die
Anlagen ohne doppelte Längenzählung abbilden. Scheitert eine konfigurierte
Bedingung, bleibt der Regelabstand angesetzt und der Fall offen.

Öffentliche Flächen werden nicht pauschal vollständig oder automatisch bis zur
Mitte angerechnet. Nur den konkret rechtlich nutzbaren Teilumriss mit seiner
Grundlage erfassen. Nachbarzustimmung allein ersetzt keine erforderliche Sicherung.
Fehlende oder nicht wirksame Sicherungen werden nicht angerechnet.

Ohne Grundstücksgeometrie wird eine Tiefe ermittelt, aber kein positiver
projektbezogener Einhaltungsnachweis ausgegeben. Fehlende Angaben sind keine Null.

### Nordrhein-Westfalen (Paket `abstand-nw`, Version `2026-09-01`)

Abgebildet nach dem Wortlaut von § 6 BauO NRW 2018 in der ab 1. September 2026
geltenden Fassung (zuletzt geändert durch Art. 1 des Gesetzes vom 21. Juli 2026,
GV. NRW. S. 537):

| Regel | Fundstelle | Umsetzung |
| --- | --- | --- |
| Wandhöhe je Wandteil; bei geneigter Geländeoberfläche im Mittel aus den Wandhöhen an den Gebäudekanten bzw. Wandteilgrenzen | Abs. 4 Satz 3 bis 5 | `mittel_kanten` |
| Abgrabungen für Belichtung, Zugang, Zufahrt bleiben außer Betracht | Abs. 4 Satz 6 | Hinweis; im Geländeprofil berücksichtigen |
| Dächer > 70° voll, > 45° zu einem Drittel | Abs. 4 Satz 7 Nr. 1a, 2a | Dachregeln |
| Dächer mit Gauben/Dachaufbauten > ½ Wandlänge zu einem Drittel | Abs. 4 Satz 7 Nr. 2b | Gaubenregel |
| Giebelflächen voll, wenn beide Dachseiten > 70°, sonst zu einem Drittel | Abs. 4 Satz 7 Nr. 1b, 2c | Giebelregel |
| 0,4 H, mind. 3 m; GE/GI 0,2 H; MK 0,25 H | Abs. 5 Satz 1, 2 | Tiefenregeln |
| Zu öffentlichen Verkehrs-, Grün-, Wasserflächen in MK/MU 0,2 H | Abs. 5 Satz 3 | Tiefenfall |
| Wohngebäude GK 1/2, höchstens drei oberirdische Geschosse: 3 m | Abs. 5 Satz 5 | Tiefenfall |
| Keine unzulässige Überdeckung bei Wänden im Winkel > 75° | Abs. 3 Nr. 1 | Überdeckungsregel |
| Garagen, Nebengebäude bis 30 m³ ohne Aufenthaltsräume, mittlere Wandhöhe ≤ 3 m, 9 m je Grenze, 18 m gesamt | Abs. 8 Satz 1 Nr. 1, Satz 3 | Grenzanlagenregel |

Nicht automatisiert und gesondert zu prüfen: Satzungen (Abs. 5 Satz 6),
größere Tiefe zu angrenzenden anderen Baugebieten (Abs. 5 Satz 4), Vorbauten
und Dachüberstände (Abs. 6), Energieeinsparung und Solaranlagen an bestehenden
Gebäuden (Abs. 7), weitere Anlagen nach Abs. 8 Nr. 2 bis 6 in der Gesamtlänge,
Aufzüge (Abs. 9), Gestattungen (Abs. 10, 12), Bestandsgebäude (Abs. 11),
Abweichungen (Abs. 13) und Windenergieanlagen. Auch dieses Paket ersetzt keine
projektbezogene Prüfung von Rechtsfassung, Übergangsrecht und örtlichen Regeln.

Weitere Länder folgen als eigene Pakete mit derselben Struktur
(`wandhoehe`, `giebel`, `gauben`, `ueberdeckung`, Tiefenfälle).

### Bayern, Baden-Württemberg

Die mitgelieferten Pakete für Bayern und Baden-Württemberg sind
**ungeprüfte Pilotansätze**, keine amtlich oder unabhängig juristisch
freigegebenen Landespakete. Konkrete Gesetzesfassung, Übergangsrecht, örtliche
Satzung und alle Anwendungsbedingungen sind vor projektbezogener Bestätigung
anhand amtlicher Quellen fachlich zu prüfen. Die amtlichen Rechtsportale sind
in dieser Cloud-Umgebung nicht freigeschaltet; eine aktuelle vollständige Prüfung
aller Landesnormen konnte hier nicht erfolgen. Die allgemeinen Faktoren ersetzen
keine kommunalen oder besonderen Vorschriften.

Stand Baden-Württemberg (`pilot-2`): Besondere Wohngebiete gehören zur Gruppe
mit 0,2 H. Dachflächen über 45° werden zu einem Viertel, über 70° voll
angerechnet; Giebelflächen nach der LBO-Reform 2025 zu einem Viertel, als
Dachart „Giebel“ mit Faktor 0,25 und Grundlage. Die Mindesttiefe von 2 m bei
Wänden bis 5 m Breite ist ein Sonderansatz. Diese Werte stammen aus Sekundärquellen
und sind am amtlichen Text zu bestätigen.

Der Schnellrechner (`lbo-rules/*.rhai`) und dieser Katalog liefern für jedes
Baugebiet dieselben Faktoren und Mindesttiefen; der Test
`shipped_lbo_pakete_entsprechen_dem_katalog` erzwingt das. Maßgebend ist der
Katalog.

Der Kern rechnet mit ungerundeten Werten. Darstellung und PDF runden nur zur
Lesbarkeit. Numerische Berührung/Grenznähe bis 0,000001 m wird als Grenzfall
statt als automatische Einhaltung ausgegeben. Dies ist keine gesetzliche
Toleranz. Ungültige Geometrien werden abgewiesen.

## Geometrie übernehmen

Grundstücks-, Flächen- und Gebäudeumrisse über Eckpunkttabellen erfassen oder
aus einer im Projekt archivierten DXF-Version übernehmen. Quelle, Zeichnungseinheit und Kontur auswählen,
Lage/Koordinatenbezug prüfen und ausdrücklich bestätigen. Die Umrechnung in Meter
erfolgt im Core; der Quellbezug wird im Plan ergänzt. Angeboten werden höchstens
200 geschlossene Konturen mit jeweils 3 bis 2.000 Punkten.

**Geländehöhen aus DXF:** Im Höhenprofil einer Wand liest
**Geländehöhen aus DXF-Lageplan vorschlagen** Höhenkoten (Texte mit
Dezimalstellen wie „101,35“, „+101.35 m“, „GOK 99,8 NHN“) und Punkte mit Höhe
(POINT, Blockeinfügungen mit z ≠ 0 im Modellbereich) einer DXF-Version. Höhen
bis zum gewählten Abstand (höchstens 10 m) neben der Wandlinie werden auf die
Wand projiziert; nahe Stationen werden auf die wandnächste Höhe reduziert. Ebene
und Suchabstand wählt der Anwender; Quelle, Ebene und Abstand stehen je Punkt im
Profil. Koten sind in Metern beschriftet, Punkthöhen folgen der Zeichnungseinheit.

Bögen/Splines werden vom vorhandenen CAD-Adapter angenähert. Vermessungsqualität
und Näherungsfehler sind zu prüfen. Ein PDF ist keine automatisch kalibrierte
Vermessungsgrundlage; DWG-Dateiverwaltung ist kein DWG-Geometrieimport.

## Archivieren und Behördenausgabe

**Nachweisstand archivieren** speichert zuerst den aktuellen Projektentwurf.
Der Nachweis enthält Eingaben, Regelinhalt samt Hash, Quellversionen mit
Dateinamen und SHA-256, Ergebnisgeometrie, Rechenkernstand, Erfasser und Zeitpunkt.
Offene Stände können archiviert und entsprechend gekennzeichnet ausgegeben werden.
Eine Freigabe unter **Nachweisstände** setzt einen aktuellen, vollständigen und
konfliktfreien Stand voraus.

**Nachweis archivieren und PDF ausgeben …** öffnet die bestehende Druckausgabe.
Der Bericht enthält Rechts-/Prüfgrundlagen mit Wandhöhenansatz, Gebäudeumrisse
mit Eckhöhen, je Wand die Herleitung mit Wandteiltabelle (H an den Kanten,
maßgebliche Wandhöhe, H mit Dach, Tiefe) und Höhenprofil (Station, Gelände,
Oberkante, Wandhöhe, Tiefe, Quelle), Maße, Flächen, Konflikte, Überdeckungen mit
Winkel und Beurteilung sowie Originalquellen. Danach folgen die
**Wandabwicklungen** (je Blatt drei Wände, maßstäblich als Ansicht von außen mit
Geländelinie, Wandoberkante, Wandteilgrenzen und Höhenangaben) und als letztes
Blatt der gezeichnete Vektorplan. Alle Blätter tragen Anlage und „Blatt k von m“.
Wand-/Grundstücksnummern, Konflikte und grafischer Maßstab bleiben enthalten.
Mit **Als Projektunterlage ablegen** die fertige PDF direkt aus diesem Dialog
im Projekt speichern; wiederholtes Ablegen derselben Ausgabe erzeugt kein Duplikat.
Der Papiermaßstab gilt bei unveränderter PDF-Ausgabe mit 100 Prozent;
Druckertreiber dürfen die Seite dafür nicht automatisch skalieren.

Über **Nachweisstände → Drucken / PDF** einen früheren Stand ausgeben. Dabei
werden archivierte Ergebnisse verwendet. Über die zentrale **Drucken / PDF-Ausgabe**
kann die fertige Ausgabe als Projektunterlage abgelegt werden. Danach lässt sie
sich in einem unveränderlichen Abgabestand der Verfahrensakte zusammen mit
Lageplan, Formularen und Anschreiben einreichen. Bescheide referenzieren genau
diese Dokumentversion. Die Software übermittelt nicht selbst an Behörden.

Zusätzlich verfügbar: **JSON** als vollständiger archivierter Nachweis und
**DXF** mit Vektorgeometrie in Metern, Grundstücks-, Innenring-, Abstands- und
Konfliktlayern. DXF ist eine technische Ergänzung, keine rechtliche Freigabe.

Planänderungen verändern frühere Nachweise und ausgegebene PDFs nicht.
Neue Versionen verwendeter Quelldateien führen zur erneuten Aktualitätsprüfung.
Reine Änderungen an GRZ-Vorgaben machen einen unabhängigen Abstandsplan nicht
veraltet. Änderungen an ausgewählten Flurstücksfakten, maßgeblichen gemeinsamen
Planrechtsgrundlagen oder am Abstandsplan dagegen schon.

## Architektur und Prüfung

`core-lib::abstandsflaechen` besitzt Modelle, Validierung, Regelauswertung,
Geometrie, Berichtslesesicht und CAD-/Exportadapter. Regelpakete liegen in
`regelwerke/abstandsflaechen/pilot.json`. Die UI berechnet keine verbindlichen
Rechtsfaktoren oder Flächen nach. Tauri und HTTP verwenden dieselben Services.

Projektentwürfe verwenden revisionierte Fachstände (`abstandsflaechen`).
`NachweisEingabe::AbstandProjekt` erweitert den bisherigen Vertrag additiv.
Alte einfache Nachweise behalten ihren Vertrag und werden als Tiefenvorprüfung
beschrieben. Es ist keine SQL-Migration erforderlich.
Historische Wiederholung verwendet die archivierten Regeln und prüft die
Rechenkernkennung. Bei Kernänderungen ist diese Kennung zu erhöhen und historische
Wiederholung ausdrücklich zu behandeln. Originalausgaben bleiben lesbar.

**Version 2 des Rechenkerns (`abstand-projekt-2`):** Höhenprofile, Wandteile,
Umrisse, Wandhöhenansatz, Giebel-/Gauben-/Überdeckungsregeln und Tiefenfälle
sind additive, optionale Vertragsfelder; ohne sie werden sie nicht gespeichert,
gespeicherte Pläne und Plan-Hashes älterer Stände ändern sich also nicht. Wände
ohne Profil rechnen weiter mit `gelaende_a_m`/`abschluss_a_m` usw. als
Zweipunktprofil; mit dem im Plan gespeicherten älteren Regelpaket (ohne
`wandhoehe`) ergibt das dieselben Werte und Flächen wie Version 1. Nachweise mit
`abstand-projekt-1` wiederholt der Kern in einem Kompatibilitätsmodus
(`berechnen_stand`) mit identischem Ergebnis einschließlich Herleitungstexten;
das wurde gegen den vorherigen Kernstand abgeglichen. Neue Pläne erhalten das
NRW-Paket `2026-09-01`; dort gilt das Kantenmittel, sodass bei geneigtem Gelände
geringere Wandhöhen als beim bisherigen Maximalansatz entstehen können. Der
Wechsel geschieht nur, wenn ein Plan dieses Paket ausdrücklich verwendet.
Bestehende Pläne behalten ihr eingebettetes Paket. Das NRW-Skript der einfachen
Tiefenvorprüfung (`lbo-rules/nw.rhai`, Version `2026.2`) rechnet im Kerngebiet
jetzt mit 0,25 H; ältere Tiefenvorprüfungen werden dadurch als veraltet
erkannt und bleiben mit ihrem archivierten Skript wiederholbar.
Die Planskizzen- und Abwicklungsblätter verwenden WinAnsi-kodierten Text
(Umlaute statt Umschreibungen).

HTTP unter `/api/projekte/{id}`: `GET /abstandsregeln`, `POST /abstandsplan`,
`POST /abstandscad`, `POST /abstandscadhoehen` (DXF-Höhen), `POST
/abstandsumrisse` (Umrissabgleich), `POST /abstandsgelaende` (Geländevorschlag
für eine Wand), Fachobjekt-API mit Art `abstandsflaechen`, bestehende
Nachweis-/Freigabe-/Druck-API und `GET /abstandsnachweise/{nachweis}/{json|dxf}`.
Lesezugriffe bleiben projektgebunden; Entwürfe brauchen Schreibrechte.

Kontrollen umfassen unabhängig erwartete Tiefen/Flächen, Altpläne mit
unveränderter Geometrie, Kantenmittel mit Hinweis bei ungleichmäßigem Gelände,
Wandteile an Höhensprüngen, punktweise Tiefe mit Knick, flächengemittelte
Wandhöhe, unvollständige Profile, NRW-Giebel/Gauben, Tiefenfälle, Umrissabgleich
(Außenseite, Eckhöhen, Wiederholbarkeit, entfallene Seiten), Überdeckungswinkel,
Höhenkoten und den DXF-Geländevorschlag, Dachschwellen,
fehlende Daten, Grundstückstrennung, Sicherungen, Längensummen, Baukörper,
Überdeckungen, Innenringe, numerische Grenzfälle, Translation/Drehung, ungültige
Geometrie und belegte manuelle Ansätze. Der durchgehende SQLite-/DMS-Test prüft
Archivierung, Wiederholung, Freigabe, echte PDF-Seiten und Vektorplan,
JSON-/DXF-Export, Quellversionen, Entwurfsänderungen, Konflikte und Leserrechte.
Ein zweiter durchgehender Test erzeugt Wände aus einem Umriss mit Eckhöhen,
archiviert einen Nachweis mit Wandteilen, prüft Wiederholung, Abwicklungsblätter
und Blattzählung im PDF sowie den Geländevorschlag aus einer DXF.
Der HTTP-Test prüft Projektgrenzen und lesende Auswertung für Leser.
Playwright verwendet den Produktionsbuild, echte Core-PDFs und einen Transportmock.
Diese technischen Tests ersetzen weder externe rechtliche Regelprüfung noch
native Büro-/Behörden-Praxisabnahme.

- [Beispiel mit Gebäudeumriss, Höhenprofil und Wandabwicklungen (PDF)](beispiel-hoehenprofil.pdf)
- [Höhenprofil einer Umrisswand](screenshots/hoehenprofil.png)
- [Ergebnis mit Wandabwicklung](screenshots/wandabwicklung.png)
