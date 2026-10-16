# Gebäude und Geschosse: bekannte Grundlagen

## Aufgabe und öffentlicher Vertrag

Das Modul `plattform::gebaeude` in `core-lib` besitzt die geometrische Herleitung. `Gebaeude::grundlagen(id)` liefert `GebaeudeGrundlagen` mit Gebäudeidentität, Rechenstand `gebaeudegrundlagen-1`, optionaler Grundfläche, Gebäudetyp, Geschossgrundlagen und offenen Angaben. Die reine Funktion verarbeitet auch ungespeicherte Entwürfe. Tauri `gebaeude_auswerten` und HTTP `POST /api/projekte/{id}/gebaeudegrundlagen` sind dünne Adapter. Der HTTP-Body enthält `id` und `gebaeude`.

Die optionale `geometrie` ergänzt `Gebaeude` und `GebaeudeGeschoss`. Fehlende oder nullwertige Optionen bedeuten unbekannt; die Zahl 0 bleibt 0. Alt-JSON wird ohne die neuen Felder wieder ausgegeben. Es gibt keine Datenbankmigration oder Neuberechnung beim Öffnen. Bestehende manuelle Flächenaufstellungen und Formularverbraucher behalten ihre bisherigen Felder und deren Bedeutung.

Flächen sind m² nach Außenmaßen bzw. als horizontale Gebäudeprojektion. Flächen und Abmessungen sind endlich und nicht negativ. Höhen sind Meter relativ zu ±0,00 und dürfen negativ sein. Nur `nullpunkt_m_nhn` ist eine absolute NHN-Höhe. Herkunftsangaben dokumentieren etwa Aufmaß und Vermessungsstand. Eine geometrische Geschossflächensumme ist keine anrechenbare GFZ-Fläche.

## Geschosse und Höhen

Ohne explizite Geschosse entsteht genau ein vorläufiges Einzelgeschoss mit der Gebäude-ID als stabiler Identität und der Gebäudegrundfläche als Geschossfläche. Es wird kein EG gespeichert. Sobald explizite Geschosse existieren, werden nur diese verwendet; ihre fehlenden Flächen werden nicht von der Gebäudegrundfläche übernommen. Die Summe bleibt unbekannt, sobald eine Geschossfläche fehlt. Frühere Einzelgeschossangaben bleiben für spätere Bearbeitung erhalten.

Beispiel: Wohnhaus01 mit 100 m² Grundfläche und KG/EG/OG/DG mit je 100 m² ergibt 100 m² Gebäudegrundfläche und 400 m² Geschossflächensumme. Eine Garage mit 20 m² ohne Geschosseintrag liefert eine Ebene mit 20 m². Daraus folgt keine Vollgeschosseinstufung.

OK Decke = OK Fertigfußboden + lichte Raumhöhe + Deckendicke. Die Deckendicke reicht von UK bis OK Decke. Eine direkt bekannte Deckenoberkante ist zulässig; liegen beide Ansätze vor, gilt 1 cm Toleranz. Der Geländeabstand ist OK Decke minus erfasste maßgebende mittlere Geländehöhe. Ob diese Messdefinition für das konkrete Vorhaben gilt, wird im Fachnachweis geprüft. Aus Fundamentsohle oder Geschosshöhe allein wird keine Deckenlage angenommen. Geschosshöhe bezeichnet OKFF bis OKFF und ersetzt keine Deckendicke.

Unterliegende Geschosse werden über IDs ausgewählt, niemals über Listenpositionen. Fremde und auf sich selbst verweisende Vergleichsgeschosse sind ungültig. Eine vollständige Höhenzone entsteht nur bei ausdrücklich bestätigter einheitlicher Raumhöhe über die gesamte Fläche. Für Dachschrägen werden eigene Höhenzonen im Nachweis erfasst.

## Verbraucher und Aktualität

- **GRZ:** `Bauteil.gebaeude_id` bindet Fläche und Typ an das Gebäude. Die bestehende BauNVO-Engine bestimmt die Anrechnung. Das Gebäude muss eindeutig der aktuellen Grundstückseinheit zugeordnet sein. Dieselbe Gebäudegrundfläche darf je Einheit nur einmal verknüpft sein. Überlagerungen und Doppelungen mit manuellen Positionen müssen zusätzlich geprüft werden; Schnittflächen lassen sich ohne Geometrie nicht bestimmen.
- **GFZ/Geschossigkeit:** `Geschoss.flaeche_aus_grundlagen=true` bindet die Außenmaßfläche an Gebäude/Geschoss. Die bestehende Einstufung bzw. ihr aktueller Quellnachweis bleibt maßgeblich. Die Oberfläche übernimmt explizite Geschosse erst mit bekannten Flächen und Einstufungen. Beim Einzelgeschoss wird die Einstufung ausdrücklich in der GFZ-Erfassung gewählt. `teilflaechen_geprueft=false` kennzeichnet unbekannte Aufenthalts-, Wand- und Erschließungsflächen und blockiert deren erforderliche Anrechnung bis zur Bestätigung. Alte Aufstellungen ohne dieses Feld behalten ihr Verhalten.
- **Vollgeschosse:** `FachPlan.gebaeudedaten_uebernehmen=true` befüllt Geschoss- und Höhenzonenlisten bei Prüfung/Speicherung aus dem Gebäude. Fehlende Flächen müssen zuerst ergänzt werden. Fehlende Höhen, Vergleichsflächen und Herkunftsangaben bleiben offene Kriterien. Änderungen der übernommenen Listen setzen Aufmaß-, Sonderfall- und Regelbestätigung zurück. Für eigene Höhenzonen lässt sich die Übernahme lösen. LBO-Kriterien stammen weiterhin aus dem ausdrücklich geprüften Regelkontext; es gibt keine neuen bundesweiten Rechtsannahmen.
- **Nachweise:** Nutzungsnachweise beziehen auch Gebäudequellen von GRZ-Bauteilen in ihren Fingerprint ein. Änderungen machen abhängige archivierte Nachweise veraltet. Fachnachweise verwenden den vorhandenen Gebäude-Fingerprint. Verspätete Entwurfsantworten der Geometrievorschau werden verworfen.

Der Rechenstand `vollgeschosse-2` lässt bei unvollständigen Höhenzonen die Einzelgeschosseinstufung und damit auch eine noch nicht vollständig bestimmte Gesamtzahl unbekannt, statt fehlende Fläche als negative Einstufung oder null Vollgeschosse auszugeben. Alte Nachweise bleiben lesbar; Wiederholungen mit abweichendem Rechenkernstand werden wie bisher ausdrücklich abgelehnt.

Verknüpfte Nutzungsflächen aktualisieren sich bei Prüfung und Nachweisarchivierung. Gespeicherte Entwürfe können frühere abgeleitete Zahlen enthalten; beim nächsten Prüfen gelten die Core-Quellen. Fehlende Quellgebäude/-geschosse und geänderte Grundstückszuordnungen werden als Fehler gemeldet. Das Lösen einer Flächenverknüpfung überführt den Zahlenwert in eine manuelle Eingabe.

## Validierung

Modultests prüfen Haus/Garage, optionale Werte, Alt-JSON-Roundtrips, signierte Höhen, widersprüchliche Deckenlagen und Vergleichsgeschosse. `storage/tests/gebaeudegrundlagen.rs` prüft Gebäude → GRZ/GFZ je Grundstück, Vollgeschossvorbefüllung, manipulierte Clientzahlen, Aktualität, doppelte Übernahmen und mehrdeutige Zuordnungen. Die bestehenden Fachberechnungstests sichern bisherige Verbraucher und archivierte Ergebnisse ab.

Ausgeführt: 172 Core-Modultests erfolgreich; die vier betroffenen Vollgeschoss-/Nichtvollgeschossfälle nach der letzten Anpassung erneut erfolgreich. Beide Storage-Integrationstests sowie der HTTP-Projektzugriffstest sind erfolgreich. Svelte-/TypeScript-Prüfung: keine Fehler oder Warnungen. Vite-Produktionsbuild und cargo check für Server und Tauri erfolgreich. Vite meldet einen Hinweis zur bestehenden Bundlegröße. Eine visuelle Browserprüfung war mangels verfügbarem steuerbaren Browser bzw. wegen fehlgeschlagener lokaler Grafikinitialisierung nicht möglich.
