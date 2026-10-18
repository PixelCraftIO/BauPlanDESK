# Benutzeranmeldung mit Anmeldename und Kennwort

Bis Version 0.1 meldete die Desktop-App den Benutzer des Betriebssystems an
(`whoami`). Im Netzwerkbetrieb war damit der Rechnername bzw. das Windows-Konto
die Identität; wer an einem anderen Arbeitsplatz saß, arbeitete unter fremdem
Namen. Jetzt hat jeder Benutzer einen Anmeldenamen und ein eigenes Kennwort und
kann an jedem Arbeitsplatz arbeiten.

## Betriebsarten

| Betriebsart | Anmeldung |
| --- | --- |
| Netzwerk (PostgreSQL, `"modus": "netzwerk"`) | Anmeldedialog beim Start, Anmeldename + Kennwort. Menü *Anwendung → Abmelden / Benutzer wechseln …* |
| Einzelplatz (SQLite) | Unverändert automatisch über das Benutzerkonto des Rechners, ohne Kennwort, kein Abmelden |

Einzelplatz bleibt bewusst ohne Kennwort: Es gibt dort genau einen Arbeitsplatz,
und die SQLite-Datei liegt ungeschützt im Profil des angemeldeten
Betriebssystem-Benutzers. Ein Kennwort würde keinen zusätzlichen Schutz bringen,
aber das Risiko schaffen, sich aus den eigenen Projekten auszusperren.

## Regeln (crates/core-lib)

- **Anmeldename** eindeutig, Groß-/Kleinschreibung egal (`Ben` = `ben`;
  Domänenpräfix wie bisher entfernt).
- **Kennwort** mindestens 10, höchstens 128 Zeichen, keine Steuerzeichen, nicht
  gleich dem Anmeldenamen, mindestens drei verschiedene Zeichen. Bewusst keine
  Pflicht zu Sonderzeichen (Länge schützt besser).
- **Speicherung** nur als Argon2id-Hash (19 MiB, 2 Durchläufe, 1 Spur) im
  PHC-Format mit zufälligem Salz; die Parameter stehen im Hash, ältere Hashes
  bleiben nach einer späteren Anhebung prüfbar. Bibliothek: `argon2` (RustCrypto,
  MIT/Apache-2.0).
- **Fehlversuche:** Vier Fehleingaben in Folge bleiben ohne Folgen, ab der
  fünften ist die Anmeldung 30 Sekunden gesperrt, jede weitere Fehleingabe
  verdoppelt die Wartezeit bis höchstens 15 Minuten. Während der Sperre wird kein
  Kennwort geprüft. Gezählt wird atomar in der Datenbank, auch bei gleichzeitigen
  Versuchen an mehreren Arbeitsplätzen. Eine erfolgreiche Anmeldung setzt den
  Zähler zurück; ein Administrator hebt eine Sperre auf, indem er ein neues
  Erstkennwort vergibt.
- **Meldungen:** Unbekannter Anmeldename und falsches Kennwort ergeben dieselbe
  Meldung; ein unbekannter Name wird mit gleichem Rechenaufwand geprüft. Dass ein
  Konto deaktiviert ist, erfährt nur, wer das richtige Kennwort kennt. Dass für
  einen vorhandenen Namen noch kein Kennwort vergeben ist, wird dagegen genannt,
  damit Kollegen im Übergang wissen, dass sie ein Erstkennwort brauchen.
- **Erstkennwort:** Ein Administrator vergibt oder setzt es in der
  Benutzerverwaltung zurück. Bei der nächsten Anmeldung muss der Benutzer es
  durch ein eigenes ersetzen; dazu gibt er das Erstkennwort noch einmal ein. So
  kann eine ältere, noch offene Sitzung nach einem Zurücksetzen kein eigenes
  Kennwort mehr setzen.
- **Kennwortpflicht:** Im Netzwerkbetrieb (`CoreServices::mit_kennwortpflicht`)
  lehnt jede Rechteprüfung im Core Konten ab, die noch kein eigenes Kennwort
  haben (Erstkennwort, Übergang, Bestand ohne Kennwort). Ausgenommen ist nur das
  technische Systemkonto, das sich nicht anmelden kann. Ohne Anmeldung arbeitet
  der Arbeitsplatz mit `Benutzer::nicht_angemeldet()`, den jede Prüfung ablehnt.

Die Abläufe stehen in `crates/core-lib/src/anmeldung.rs`, die reinen Regeln in
`crates/core-lib/src/kennwort.rs`. Desktop-Adapter:
`apps/desktop/src-tauri/src/anmeldung.rs`, Oberfläche: `src/Start.svelte`,
`src/lib/Anmeldung.svelte`, `src/lib/KennwortAendern.svelte`,
`src/lib/Benutzerverwaltung.svelte`.

## Übergang für bestehende Büros

Bestehende Benutzer haben nach dem Update kein Kennwort. Damit sich ein Büro
nicht aussperrt:

1. Solange **kein aktiver Administrator ein Kennwort** (Erst- oder eigenes) hat,
   bietet der Anmeldedialog einem Administrator einmalig die Anmeldung über das
   Benutzerkonto des Rechners an („Als *name* anmelden und Kennwort festlegen“).
   Er muss sofort ein eigenes Kennwort setzen; vorher sind keine Änderungen
   möglich.
2. Danach vergibt er in *Einstellungen → Benutzer und Rechte* Erstkennwörter für
   die Kollegen und gibt sie persönlich weiter.
3. Sobald ein Administrator ein Kennwort hat, ist die Anmeldung über das
   Rechnerkonto für alle geschlossen. Benutzer ohne Kennwort können sich dann
   nicht anmelden, bis sie ein Erstkennwort erhalten.

Auf einer **leeren Datenbank** legt dieselbe Übergangsanmeldung wie bisher den
ersten Benutzer als Administrator an. Weitere Benutzer legt der Administrator mit
Erstkennwort an; die automatische Anlage mit Rolle *Lesen* bei der ersten
Anmeldung gibt es im Netzwerkbetrieb nicht mehr.

## Protokoll

Anmeldung (`angemeldet`), Fehlversuch (`anmeldung_fehlgeschlagen`, auch für
unbekannte Namen unter dem Systemkonto) und Kennwortänderung bzw. vergebenes
Erstkennwort (`kennwort_geaendert`) werden über das vorhandene Protokoll
gespeichert. Die Einträge gehören zu keinem Projekt und erscheinen deshalb nicht
im Projektprotokoll der Oberfläche; ältere Programmversionen lesen nur
Projektprotokolle und stoßen so nicht auf die neuen Aktionen.

## Was die Kennwortanmeldung nicht leistet

Die Arbeitsplätze verbinden sich weiterhin **direkt mit der Datenbank**; die
Zugangsdaten stehen in der `config.json` jedes Arbeitsplatzes. Rollen, Rechte,
Projektmitgliedschaften und die Kennwortanmeldung gelten deshalb **gegenüber der
Anwendung**, nicht gegenüber jemandem, der diese Zugangsdaten direkt mit einem
Datenbankwerkzeug verwendet. Wer das ausschließen muss, braucht einen
Anwendungsserver zwischen Arbeitsplätzen und Datenbank (der Projektserver in
`crates/server` mit Token-Zugängen ist dafür der Ansatz) und darf die
Datenbank-Zugangsdaten nicht auf die Arbeitsplätze verteilen. Die Kennwörter
selbst liegen nur als Argon2id-Hash in der Datenbank.

## Datenbank und Kompatibilität

- Migration `0012_benutzer_kennwort.sql` (SQLite und PostgreSQL gleich) ergänzt
  die Tabelle `benutzer` um `kennwort_hash`, `kennwort_aendern`,
  `kennwort_geaendert_am`, `fehlversuche`, `gesperrt_bis`, `letzte_anmeldung`.
  Bestehende Zeilen bleiben unverändert und haben kein Kennwort.
- `Benutzer` hat die neuen Felder `kennwort` (`keines` | `erstkennwort` |
  `gesetzt`), `anmeldesperre_bis` und `letzte_anmeldung`, alle mit
  `serde(default)`. Der Hash verlässt die Datenbankschicht nur über
  `BenutzerRepository::kennwortdaten` und wird nie an die Oberfläche gegeben.
- Ältere App-Versionen starten an einer migrierten Datenbank nicht mehr (die
  Migrationsprüfung meldet die ihnen unbekannte Migration 12); sie können sich
  also nicht an der Kennwortanmeldung vorbei über das Rechnerkonto anmelden. Alle
  Arbeitsplätze eines Büros sind gemeinsam zu aktualisieren.
- Der Projektserver (`crates/server`) meldet weiterhin über Token-Zugänge an;
  `CoreServices::anmelden` (Rechnerkonto ohne Kennwort) bleibt für Einzelplatz und
  die Einrichtung von Serverzugängen bestehen.
