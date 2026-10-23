# Illustriertes Bedienhandbuch und integrierte Hilfe

Für Architekten, Entwurfsverfasser und Planer:

- [PDF herunterladen](Bedienhandbuch.pdf): 52 Kapitel, 133 Seiten und 35 Screenshots mit Erläuterungen; anklickbares Inhaltsverzeichnis mit tatsächlichen Kapitelstartseiten, Lesezeichen, Kopf-/Fußzeile und Seitenzählung.
- [Eigenständige HTML-Ausgabe](Bedienhandbuch.html): Bilder eingebettet, ohne Internet, Skripte oder externe Schriften lesbar und druckbar.
- [Markdown-Ausgabe](Bedienhandbuch.md): mit relativen Bildverweisen innerhalb dieses Repositorys. Für eine einzelne weiterzugebende Datei PDF oder HTML verwenden.

Beschrieben wird der Arbeitsstand `e564fb1` mit den Workflowverbesserungen vom
23.10.2026. Die Anleitung erklärt Einrichtung, Projektanlage, Rechtsgrundlagen,
Flurstücke, Baugrundstücke, Gebäude, sämtliche Berechnungsbereiche, PDF-Formulare,
Dateien und Versionen, Druckausgaben, Verfahrensakten, Erstabgabe, Nachforderungen,
Bescheide, Ausführung und Sicherung. Ergänzt wurden detaillierte Schritte,
Feld-/Kontrolltabellen, Fehlerbehandlung und ein durchgängiges Zahlenbeispiel.

Die Bilder zeigen verschiedene **fiktive Beispielprojekte** in der tatsächlichen
Produktionsoberfläche mit kontrollierten IPC-Testadaptern. Sie wurden nicht als
Oberflächenentwürfe generiert. 29 Motive wurden für diese Ausgabe aufgenommen;
sechs stammen aus den unmittelbar vorhergehenden Prüfungen desselben Arbeitsstands.
Die sichtbaren Daten und Regeln sind keine fachlich geprüfte Musterplanung.
Native Betriebssystemdialoge, reale Behördenverbindungen und ein automatischer
Portalversand werden durch die Aufnahmen nicht nachgewiesen. Der unterstützte
manuelle Portalablauf ist im Handbuch beschrieben.

## Gemeinsame Inhaltsquelle

`apps/desktop/src/lib/hilfe/handbuch.json` enthält Kapitel, Schritte, Tabellen,
Suchbegriffe, Modulzuordnungen, Querverweise und optionale Abbildungen mit
Alternativtext und Bildunterschrift. Die Svelte-Hilfe verwendet dieselben Inhalte.
Bilder liegen unter `apps/desktop/public/handbuch/`; `aufnahmen.json` nennt
Herkunft und Prüfsumme jeder verwendeten Datei. Sie werden bei der Anzeige der
Hilfe erst bei Bedarf geladen. Der isolierte Handbuchdruck wartet auf vollständig
geladene Bilder und meldet Ladefehler, statt unvollständige Unterlagen zu drucken.

Änderungen an der Inhaltsquelle bzw. an Bildern anschließend exportieren:

```sh
npm run handbuch:export --prefix apps/desktop
BAUPROJEKT_CHROMIUM=/usr/bin/chromium python3 scripts/handbuch-pdf.py
npm run handbuch:check --prefix apps/desktop
node --experimental-strip-types --test apps/desktop/tests/handbuch.test.mjs
```

Node 22 mit Type-Stripping oder neuer verwenden. Der HTML-Export bettet nur
mitgelieferte PNG-Dateien mit geprüften Dateinamen ein; Pfadnavigation wird
abgelehnt. Das PDF-Skript benötigt Python Playwright, Chromium und PyMuPDF
(`fitz`). Es lädt keine externen Ressourcen und prüft Kapitel, Bilder,
Lesezeichen, Inhaltslinks und Seitenzählung. Die Kapitelstartseiten werden aus
dem erzeugten PDF ermittelt und bis zu einem stabilen Seitenverzeichnis
übernommen. `pdf-stand.json` bindet die PDF mit Prüfsummen an die HTML-Quelle und
nennt Seitenzahl, Bildzahl und tatsächliche Kapitelstartseiten.

## Neue Bildschirmaufnahmen erstellen

Die mitgelieferten Bilder genügen zum Build und zum Lesen; für neue Aufnahmen
werden zusätzlich die Testartefakte der vorhandenen Storage-Integrationsprüfungen
benötigt. Die jeweiligen Tests exportieren diese bei gesetzter Umgebungsvariable:

| Testziel (`crates/storage/tests/`) | Variable für Artefaktordner |
| --- | --- |
| `fachberechnungen` | `BAUPROJEKT_FACH_ARTEFAKTE` |
| `druckausgabe` | `BAUPROJEKT_DRUCK_ARTEFAKTE` |
| `verfahren` | `BAUPROJEKT_VERFAHREN_ARTEFAKTE` |
| `abstandsflaechen` | `BAUPROJEKT_ABSTAND_ARTEFAKTE` |
| `pdf_livevorschau` | `BAUPROJEKT_PDF_LIVE_ARTEFAKTE` |

Für Fach- und Druckaufnahmen müssen auch die PDF-Artefakte des jeweiligen
Workflows vorliegen. Die Skriptvoreinstellungen entsprechen den hier geprüften
Artefaktordnern unter `/tmp`; auf einem anderen Arbeitsplatz die Variablen setzen.
Die genauen nativen Testnamen und Voraussetzungen vor dem Erzeugen anhand der
jeweiligen Testdatei prüfen. Ein vollständiger Rust-Neubuild ist für die reine
Handbucherstellung nicht nötig.

Produktionsbuild erstellen, über einen lokalen HTTP-Server bereitstellen und mit
`BAUPROJEKT_UI_URL` aufrufen. Während der Aufnahmen den Build nicht neu erzeugen,
weil dabei die geladenen Moduldateien ausgetauscht würden.

```sh
BAUPROJEKT_UI_URL=http://127.0.0.1:1465 \
BAUPROJEKT_HANDBUCH_AUFNAHMEN=/tmp/bauplan-handbuch-aufnahmen \
python3 scripts/handbuch-screenshots.py
```

Das Skript verwendet die kontrollierten Adapter der vorhandenen UI-Prüfungen.
Es bricht bei sichtbaren Fehlern oder unbekannten Befehlen ab. Die Abbildung der
Dokumentenverwaltung wird nach dem grundlegenden Leerszenario durch den gefüllten
Verfahrensstand ersetzt; es bleiben 29 unterschiedliche Motive. Die sechs weiteren
Motive werden über die in `aufnahmen.json` genannten vorhandenen Browserprüfungen
erzeugt. Ausgewählte Bilder anschließend visuell kontrollieren, nach
`apps/desktop/public/handbuch/` übernehmen und die Bildprüfsummen aktualisieren.
Danach HTML/PDF neu erzeugen; kein blindes Ersetzen der veröffentlichten Bilder.

## Geprüfter Stand

- Typprüfung: keine Fehler und keine Warnungen; Produktionsbuild erfolgreich.
- Sechs Node-Handbuchtests: Kapitel, Modulabdeckung, Suche, interne Links,
  HTML-Escaping, aktuelle Ausgaben/PDF-Prüfsummen sowie alle 35 Abbildungen geprüft.
- Browserprüfung: Hilfe ohne/mit Projekt, Kontexthilfe, Suche, Markdown-Download,
  Erhalt offener Projektentwürfe, Hell/Dunkel, 600–1440 px und isolierter Druck.
- PDF-Prüfung: sämtliche Kapitel, 35 vollständig geladene Bilder, stabile
  Kapitelstartseiten, interne Links, Seitenzählung und keine inhaltsleeren Seiten.
  Titelblatt, Inhaltsverzeichnis und Seiten mit Berechnungen, Abstandsflächen und
  Formularvorschau zusätzlich visuell kontrolliert.

Die Hilfe bleibt ein lesendes Modul ohne Fach-/Speicher-API. Änderungen betreffen
Inhalte, Abbildungen und deren Ausgabe, keine Fachberechnungen, Datenbankverträge
oder Projektversionen. Vorhandene Bürofreigaben und rechtliche Prüfpflichten
werden durch das Handbuch nicht ersetzt.
