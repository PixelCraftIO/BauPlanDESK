# Planungsrecht und Grundstücksnachweise

Projekte speichern Rechtsgrundlagen nach §§ 30/34/35 BauGB, Planart,
Planbezeichnung/Rechtskraft, Vorhaben und strukturierte Festsetzungen.

Die Flächenformulare lesen gemeinsam mit dem Rust-Rechenkern versionierte
BauNVO-Konfigurationen für 1962/1968/1977/1990/2017. Bauteiltyp und Fläche
bestimmen automatisch die GRZ-Anrechnung; Voll-/Nichtvollgeschosse liefern
die GFZ-Fläche. Besondere Nichtvollgeschoss-Festsetzungen können einmal im
Projekt hinterlegt werden. Die JSON-Regelwerke sind im Build eingebettet;
Konfigurationsänderungen benötigen Versionsänderung und Neubuild.

Flurstücke werden nach Grundbuchbezirk, Blatt und laufender Nummer gruppiert.
Abzüge wie festgesetzte private Grünflächen verkleinern die maßgebende
Bezugsfläche. Durch den Plan zugeordnete Gemeinschaftsanteile können sie
vergrößern. Eigentumsflächen, Abzüge und Zuschläge bleiben getrennt gespeichert.
Jede Grundstückseinheit hat eigene Bauteile, Geschosse, Baumasse und Nachweise;
eine Überschreitung wird nicht mit einer anderen Grundstückseinheit verrechnet.
Bei besonderen rechtlichen Verhältnissen kann eine belegte Zuordnung erfasst werden.

In „Pläne“ lassen sich geschlossene Polygone als Baufenster und Vorhaben
zuordnen. Die Differenzfläche wird rot dargestellt und in m² und Prozent
berechnet. Die Zuordnung ist an DMS-Dokument und Version gebunden.

## Screenshots aus der nativen Anwendung

Alle dargestellten Projekte, Personen und Planfestsetzungen sind Testdaten.

| Bereich | Screenshot |
| --- | --- |
| Strukturierte Planfestsetzungen | [Festsetzungen](screenshots/festsetzungen.png) |
| CAD-Differenzfläche außerhalb des Baufensters | [Baugrenzen](screenshots/baugrenzen.png) |
| Zugeordnete Gemeinschaftsfläche | [Grundstücksbezugsfläche](screenshots/gemeinschaftsflaeche.png) |
| Kataster, Grundbuch und Grünflächenabzüge | [Flurstücksformular](screenshots/flurstuecke.png) |
| Eigener Nachweis für eine Grundstückseinheit | [GRZ-/GFZ-Überschreitung](screenshots/einzelnachweis.png) |

## Validierung

141 Workspace-Rust-Tests einschließlich SQLite und PostgreSQL bestanden.
Rust-Format, Clippy mit `-D warnings`, Svelte-Prüfung und Frontend-/Desktop-/Server-Build
wurden geprüft. HTTP und native Oberfläche wurden mit gespeicherten Festsetzungen,
Flächenaufstellungen und CAD-Polygonen getestet. Nach geändertem Grünflächenabzug
verschwinden alte Ergebnisse; Speichern und erneutes Öffnen erhalten drei
Flurstücke und zwei getrennte Grundstücksnachweise.

## Fachliche Grenzen

* Die historischen BauNVO-Regelansätze benötigen unabhängige juristische Freigabe.
  Vollgeschosse werden vom Anwender nach dem maßgebenden Landesrecht eingestuft.
* Grundbuchliche Gruppierung ist eine nachvollziehbare Datenzuordnung und ersetzt
  keine Prüfung besonderer rechtlicher Grundstücksverhältnisse.
* Überlagerte Bauteil-/Abzugsflächen sind vor Eingabe zu bereinigen. Die Lage
  außerhalb eines Baufensters allein verkleinert nicht die Grundstücksbezugsfläche.
* Die CAD-Flächendifferenz umfasst den Polygonumgriff. Der angezeigte maximale
  Eckpunktabstand ist kein allgemeiner maximaler Überstand über alle Kantenpunkte.
* Geometrische Baugrenzen und Abstandsflächen bleiben separate Prüfbereiche;
  die Mehrgrundstücksnachweise betreffen Maß der Nutzung und quantitative Flächenlimits.

Bestehende Projekte erhalten über Migration `0006` eine leere Planungsrechts-
Konfiguration. Ältere API-Payloads bleiben lesbar; Updates ohne `planungsrecht`
erhalten die vorhandenen Angaben. Alle Projektänderungen verwenden die bestehende
Rechteprüfung und Optimistic Locking.
