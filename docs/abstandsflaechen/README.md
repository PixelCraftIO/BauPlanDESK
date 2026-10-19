# Projektbezogene Abstandsflächen

## Bedienung

Im Projekt **Berechnungen → Abstandsflächen** einen Abstandsplan anlegen.
Mehrere Pläne können Varianten oder Bauabschnitte abbilden. Der bisherige Rechner
bleibt unter **Einfache Tiefenvorprüfung** verfügbar; alte Nachweise bleiben lesbar.

1. **Grundlagen:** Gemeinde, konkrete Rechtsfassung und Anwendbarkeit, Baugebiet,
   Satzungs-/Planprüfung, Gelände-/Höhen- und Koordinatenbezug dokumentieren.
   Originalunterlagen mit ihrer konkreten Version zuordnen.
2. **Wände / Bauteile:** Gebäude und Wandabschnitte mit Anfang/Ende in Metern,
   Außenseite, Geländehöhen und Wandabschluss an beiden Enden erfassen.
   Dach-/Giebelansatz und gegebenenfalls besonderen Wandfall wählen.
3. **Lageplan / Grenzen:** Rechtliche Grundstückseinheiten mit zugehörigen
   Flurstücken und Umrissen erfassen. Innenringe dienen tatsächlich nicht
   zugehörigen Grundstücksflächen. GRZ-/GFZ-Abzüge verändern diese Grenzen nicht.
   Eine Grundbuchdatei ist keine Pflicht.
4. **Sonderfälle / Sicherungen:** Andere Baukörper, belegte Sicherungsflächen
   und zulässige öffentliche Teilflächen eingeben. Besondere Wandansätze brauchen
   Rechtsgrundlage/Entscheidung, Geltungsumfang, Wirksamkeit, Beleg und Tiefe.
   Ein eigenes nachgewiesenes Abstandsflächenpolygon kann übernommen werden.
5. **Prüfbericht:** Vollständigkeit von Wänden, Geländeabschnitten, Ecken,
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

Der Wandansatz lautet konservativ:
`max(Wandabschluss A − Gelände A, Wandabschluss B − Gelände B)`.
Zwischenhochpunkte, gestufte Fassaden und nichtlineare Verläufe durch weitere
Abschnitte erfassen. Ein gesetzlicher besonderer Mittelungsansatz wird nicht
ungeprüft angenommen. Dachflächen werden nach den konfigurierten Neigungsschwellen
angesetzt; Giebel und besondere Dachansätze brauchen einen ausdrücklich
hergeleiteten Faktor mit Quelle.

Die Standardgeometrie besteht aus senkrecht zur Wand gerichteten Rechtecken.
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

Die mitgelieferten Pakete für NRW, Bayern und Baden-Württemberg sind
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

Umrisse über Eckpunkttabellen erfassen oder aus einer im Projekt archivierten
DXF-Version übernehmen. Quelle, Zeichnungseinheit und Kontur auswählen,
Lage/Koordinatenbezug prüfen und ausdrücklich bestätigen. Die Umrechnung in Meter
erfolgt im Core; der Quellbezug wird im Plan ergänzt. Angeboten werden höchstens
200 geschlossene Konturen mit jeweils 3 bis 2.000 Punkten.

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
Der Bericht enthält Rechts-/Prüfgrundlagen, Wandherleitungen, Maße, Flächen,
Konflikte, Originalquellen und einen zusätzlichen gezeichneten Vektorplan.
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

HTTP unter `/api/projekte/{id}`: `GET /abstandsregeln`, `POST /abstandsplan`,
`POST /abstandscad`, Fachobjekt-API mit Art `abstandsflaechen`, bestehende
Nachweis-/Freigabe-/Druck-API und `GET /abstandsnachweise/{nachweis}/{json|dxf}`.
Lesezugriffe bleiben projektgebunden; Entwürfe brauchen Schreibrechte.

Kontrollen umfassen unabhängig erwartete Tiefen/Flächen, Dachschwellen,
fehlende Daten, Grundstückstrennung, Sicherungen, Längensummen, Baukörper,
Überdeckungen, Innenringe, numerische Grenzfälle, Translation/Drehung, ungültige
Geometrie und belegte manuelle Ansätze. Der durchgehende SQLite-/DMS-Test prüft
Archivierung, Wiederholung, Freigabe, echte PDF-Seiten und Vektorplan,
JSON-/DXF-Export, Quellversionen, Entwurfsänderungen, Konflikte und Leserrechte.
Der HTTP-Test prüft Projektgrenzen und lesende Auswertung für Leser.
Playwright verwendet den Produktionsbuild, echte Core-PDFs und einen Transportmock.
Diese technischen Tests ersetzen weder externe rechtliche Regelprüfung noch
native Büro-/Behörden-Praxisabnahme.
