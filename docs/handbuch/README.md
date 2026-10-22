# Bedienhandbuch und integrierte Hilfe

Für Architekten, Entwurfsverfasser und Planer:

- [Bedienhandbuch als Markdown](Bedienhandbuch.md)
- [Eigenständige HTML-Ausgabe](Bedienhandbuch.html): lokal im Browser öffnen; ohne Internet, Skripte oder externe Schriften nutzbar und druckbar.
- [PDF-Ausgabe](Bedienhandbuch.pdf): mit verlinktem Inhaltsverzeichnis,
  Lesezeichen, strukturierten Texten und Seitenzählung.

Die Inhalte wurden gegen Main `c1e1e80` einschließlich der anschließenden Arbeitsplatzverbesserungen vom 22.10.2026 und die dortigen
Bedienmasken, Modulregister und dokumentierten Datenwege abgeglichen. Sie
beschreiben auch die in diesem Branch ergänzten Hilfezugänge. Fachliche Grenzen,
ungeprüfte Regelbeispiele und der manuelle Portaltransport sind ausdrücklich
beschrieben. Das Handbuch ist keine rechtliche Prüfung der mitgelieferten Regeln.

## Gemeinsame Inhaltsquelle

`apps/desktop/src/lib/hilfe/handbuch.json` enthält Kapitel, Schritte, Tabellen,
Suchbegriffe, Modulzuordnungen und Querverweise. Die Svelte-Hilfe stellt diese
strukturiert und ohne HTML-Inhalteingabe dar. Eine Änderung erfolgt an dieser
Quelle; anschließend die lesbaren Ausgaben regenerieren:

```sh
npm run handbuch:export --prefix apps/desktop
BAUPROJEKT_CHROMIUM=/usr/bin/chromium python3 scripts/handbuch-pdf.py
npm run handbuch:check --prefix apps/desktop
node --experimental-strip-types --test apps/desktop/tests/handbuch.test.mjs
```

Node 22 mit Type-Stripping oder neuer verwenden. Die Frontend-CI prüft
Querverweise, Modulabdeckung, Suchverhalten, HTML-Escaping und Übereinstimmung
der Markdown-/HTML-Ausgaben. `pdf-stand.json` bindet die PDF über Prüfsummen an
die HTML-Quelle; bei einer Inhaltsänderung wird damit auch eine vergessene
PDF-Aktualisierung erkannt. Für neue Module ist eine passende Kapitelzuordnung
zu ergänzen; die Hilfe öffnet bei unbekanntem Kontext den Einstieg.

Die PDF wird nach inhaltlichen Änderungen aus der generierten HTML-Datei neu
erstellt. Das Skript `scripts/handbuch-pdf.py` verwendet Python Playwright und
Chromium. Es prüft sämtliche Kapitel vor Ausgabe und ergänzt einen Briefkopf
sowie Stand und Seitenzählung. Aufruf aus dem Repository:

```sh
BAUPROJEKT_CHROMIUM=/usr/bin/chromium python3 scripts/handbuch-pdf.py
```

## Modulgrenze und Prüfung

Die Hilfe ist ein lesendes Frontend-Modul und verwendet keine Fach- oder
Speicher-API. Das Anzeigen, Suchen, Herunterladen und Drucken verändert keine
Projektdaten. Projekteditoren bleiben beim Aufruf montiert; Links zu
Projektbereichen verwenden die bestehende Navigation mit deren Voraussetzungen.
Der Handbuchdruck ist von der Projekt-/PDF-Druckausgabe getrennt und enthält
ausschließlich mitgelieferte, HTML-escaped Texte. Die Benutzerrolle begrenzt
weiterhin die eigentlichen Bearbeitungsaktionen.

`apps/desktop/tests/handbuch-ui.py` prüft den Zugriff ohne/mit Projekt,
Kontexthilfe, Suche, Entwurfserhalt, Download sowie den isolierten Volltextdruck
in Hell/Dunkel und bei schmaler Ansicht. Die Fachberechnungen selbst sind
unverändert; deren Rechtsanwendung wird durch diesen Hilfe-Test nicht geprüft.

## Geprüfter Stand dieses Ausbaus

- 52 Kapitel; alle registrierten Projektbereiche mit Kontexthilfe abgedeckt.
- 63-seitige PDF auf Kapitelvollständigkeit, leere Seiten, Briefkopf und
  Seitenzählung geprüft; Stichproben der Darstellung einschließlich GRZ-Kapitel.
- Typprüfung ohne Fehler/Warnungen und Produktionsbuild erfolgreich.
- Zehn Node-Tests einschließlich vorhandener Release-/Feldtests unter Node 22
  erfolgreich; 55 npm-Pakete ohne Lizenzverstoß.
- Browserprüfung von Einstieg, Kontexthilfe, Suche, Download, Entwurfserhalt,
  Druck aller Kapitel ohne Projektdaten sowie Hell/Dunkel und 600–1440 px.
- Bestehende Projekttab- und Eingabefeld-/Navigationsregressionen erfolgreich.
- Rust-Formatprüfung erfolgreich. Der neue native Menüeintrag verwendet den
  vorhandenen Menüadapter; ein nativer Windows-/macOS-Installertest wurde hier
  nicht ausgeführt.

Ansichten der integrierten Hilfe:
[Einstieg im hellen Modus](screenshots/einstieg-hell.png) ·
[Nachforderungsworkflow im dunklen Modus](screenshots/nachforderung-dunkel.png).
