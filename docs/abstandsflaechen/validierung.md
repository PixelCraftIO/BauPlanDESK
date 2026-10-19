# Technische Validierung

Ausgangsstand: Main `b1a2148` (einschließlich optionaler Grundbuchangaben).

| Prüfung | Ergebnis |
| --- | --- |
| Rust-Workspace einschließlich PostgreSQL und nativer Adapter | 238 Tests bestanden |
| Zusätzlicher durchgehender Abstandsflächen-/DMS-Test | bestanden, einschließlich Originalquellenwechsel und DXF-Übernahme |
| Clippy, Workspace/alle Targets, `-D warnings` | bestanden |
| Formatierung und generierte TypeScript-Verträge | geprüft, identisch |
| Svelte-Prüfung und Produktionsbuild | keine Fehler/Warnungen, Build bestanden |
| Browser: Projekttabs, Plattform, Druck, Verfahren, Abstandsflächen | fünf Skripte bestanden |
| Node-Regression der PDF-Livevorschau | vier Tests bestanden |
| npm-Lizenzen | 54 Pakete, keine Verstöße |

Die zwölf neuen fachlichen Kontrollfälle decken Tiefe/Fläche, Dachschwellen,
Giebel, unbekannte Angaben, Grundstückstrennung, Sicherungen, Grenzanlagenlängen,
Baukörperkonflikte, Überdeckungen, numerische Grenznähe, Innenringe, besondere
Flächenansätze sowie Translation/Drehung und ungültige Geometrie ab.
Die Polygon-Referenzfläche wird mit begründeter numerischer Vergleichsgenauigkeit
von 0,000001 m² geprüft; dies ist keine gesetzliche Flächentoleranz.

Der Integrationstest nutzt SQLite, echten lokalen DMS-Speicher, echte PDF-Seiten
und den DXF-Parser. Er weist nach: Archivierung und Freigabe funktionieren;
Nachrechnung verwendet den archivierten Regelstand; eigene Planänderungen und
neuere Originalquellen machen den alten Stand veraltet; archivierte Ergebnisse
und exportierte Bytes bleiben erhalten. Eine reine GRZ-Vorgabenänderung verändert
die Aktualität eines unabhängigen Abstandsplans nicht. Projektfremde Quellen,
veraltete Revisionen und Leser-Schreibzugriffe werden abgewiesen.

Der Browser verwendet das Produktionsbundle mit IPC-Testtransport und echten
Core-Berichten. Geprüft werden Wandänderung, Speicherfehler mit erhaltenem Entwurf,
Ergebnisanzeige, PDF-Text und Vektorseite sowie die direkte Übernahme derselben
PDF-Bytes als Projektunterlage. Bestehende Projekttab- und Druckabläufe bleiben
in der Regression erhalten. CI erzeugt die neuen Daten und führt den Test aus.

- [Fiktiver Beispielnachweis als PDF](beispiel-nachweis.pdf)
- [Ergebnisansicht](screenshots/pruefbericht.png)
- [PDF-Vektorplan in der Anwendung](screenshots/vektorplan-pdf.png)

Der Beispielnachweis ist ein synthetischer Kontrollfall. Seine Bestätigungen sind
Testdaten und keine juristische Freigabe des Pilotkatalogs.
Unabhängige Prüfung aktueller Landesvorschriften und Satzungen sowie eine native
Windows-/Büro-/Behörden-Praxisabnahme stehen weiterhin aus. Unterstützte automatisierte
und manuell belegte Fallgruppen sind in der [Bedienung](README.md) beschrieben.

## Ausbau Höhenprofil, Gebäudeumriss und NRW 2026

| Prüfung | Ergebnis |
| --- | --- |
| Kontrollfälle `core-lib::abstandsflaechen` | 28 Tests bestanden (u. a. Kantenmittel, Wandteile, punktweise Tiefe, Giebel/Gauben, Tiefenfälle, Umriss, Überdeckungswinkel, DXF-Höhen, Rechenstand 1) |
| Durchgehende SQLite-/DMS-Tests | 2 bestanden, davon einer mit Umriss, Profil, Nachweis, Wiederholung, Abwicklungsblättern und DXF-Geländevorschlag |
| Rechenstand-1-Kompatibilität | Ergebnisse des vorherigen Kerns für drei Altfälle byte-gleich reproduziert; Planserialisierung unverändert |
| Browser (Produktionsbuild) | Altplan ins Höhenprofil übernehmen, Umriss zu Wänden, gesperrte Umrissfelder, Wandabwicklung im Ergebnis |
| Sichtprüfung PDF | Wandteil- und Profiltabellen, Abwicklungsblätter mit Blattzählung, Umlaute in Plan- und Abwicklungsblättern |

