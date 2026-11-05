# CI-Korrektur der Rust-Abhängigkeiten

Die Abhängigkeitsprüfung meldete für `rustls 0.23.45` die Advisories
RUSTSEC-2026-0346 bis RUSTSEC-2026-0349. Das Workspace-Lockfile verwendet
deshalb die korrigierte Version `0.23.46`.

`ttf-parser 0.25.1` ist laut RUSTSEC-2026-0192 nicht mehr gepflegt. Die direkte
Abhängigkeit im Core wurde durch `skrifa 0.48` mit dem Feature `std` ersetzt.
Die bestehende `cargo-deny`-Konfiguration bleibt unverändert; es werden keine
Advisories ausgenommen.

Der Austausch betrifft ausschließlich den internen Adapter
`crates/core-lib/src/formular/schrift.rs`. PDF-Feldverträge, Unicode-Zuordnung,
eingebettete DejaVuSans, Zeichenbreiten und Textpositionierung bleiben erhalten.
Desktop, Server und DMS verwenden weiterhin dieselben Core-Schnittstellen.
Bestehende Projekte, Formularvorlagen und PDF-Dateien benötigen keine Migration.
Fehlende Zeichen oder Zeichenbreiten werden als Fehler zurückgegeben.

Die Regressionstests prüfen die erzeugten PDF-Schriftressourcen und
Darstellungsstreams einschließlich Umlauten, Eurozeichen, UTF-16-Surrogatpaar,
Zeichenbreiten, Grundlinie, automatischer Verkleinerung und Fehlerfällen.
Die vorhandenen Formular-Integrationstests prüfen die Übergabe an das DMS.

Relevante Prüfungen vom Repository-Stamm:

```powershell
cargo fmt --all --check
cargo clippy -p core-lib --all-targets --locked -- -D warnings
cargo test -p core-lib --lib --locked
cargo test -p storage --test formular_workflow --test formularbibliothek --test projekt_formulare --locked
cargo deny --all-features check
```

Lokal unter Windows am 15.10.2026 geprüft: 124 Core-Modultests und vier
Formular-/DMS-Integrationstests erfolgreich; Formatprüfung und Clippy ohne Fehler.
`cargo-deny 0.20.2` meldet mit allen Features:
`advisories ok, bans ok, licenses ok, sources ok`.

## npm-Hinweise der Desktop-Oberfläche (05.11.2026)

`npm audit` in `apps/desktop` meldet drei Hinweise niedriger Stufe, alle aus
derselben Quelle: `deepmerge` (GHSA-gcqx-w48m-w2h4, Prototype Poisoning in
`mergeObject()`), eingezogen über `@sveltejs/vite-plugin-svelte` und dessen
Inspector. Beide sind reine Entwicklungswerkzeuge (Vite-Plugin beim Bauen bzw.
im Dev-Server); sie gelangen nicht in das ausgelieferte Programm und verarbeiten
keine Projektdaten. Eine korrigierte Version gibt es noch nicht („No fix
available“). Bewertung: kein Handlungsbedarf, bei der nächsten
Svelte-/Vite-Aktualisierung erneut prüfen. `npm audit fix --force` nicht
verwenden, es würde das Svelte-Plugin auf eine inkompatible Version setzen.
