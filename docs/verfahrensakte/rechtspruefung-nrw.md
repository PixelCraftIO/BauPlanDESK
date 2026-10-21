# Rechtsprüfung NRW: Prüfprotokoll

Prüfung der Rechtsangaben aus dem Genehmigungsverfahren
([genehmigungsverfahren.md](genehmigungsverfahren.md)) gegen die amtlichen
Texte. Abrufdatum aller Quellen: **21.10.2026**. Grundlage: `main` nach PR #59.

Dieses Protokoll ist **keine fachliche Abnahme**. `geprueft` bleibt in
`workflow-regeln/landesrecht.json` für beide Fassungen `false`. Die Abnahme
durch das Büro ist am Ende beschrieben.

## Ergebnis in Kürze

- **Bundesrecht** (BauGB, BauNVO, VwVfG, VwGO, ZPO): Alle geprüften Fundstellen
  im Code stimmen mit dem Text überein. Es musste nichts korrigiert werden.
  Offen ist, ob in NRW statt des VwVfG des Bundes das VwVfG NRW zu zitieren ist
  (siehe unten).
- **Landesrecht NRW** (BauO NRW in beiden Fassungen, BauPrüfVO): **nicht
  geprüft**. recht.nrw.de war erreichbar, lieferte aber nur eine Platzhalterseite.
  Kein Wert wurde geändert, keiner bestätigt.
- **XBau-Codelisten:** Abweichungsart 2.1 und Erleichterung 1.0 sind jetzt
  mitgeliefert. Für NRW gibt es keine Landesliste der Verfahrensarten.
- **XBau-Version:** Verbindlich ist derzeit XBau **2.6**. Die mitgelieferte
  Version 2.7 ist im XRepository ein **Entwurf**. Nur berichtet, nicht migriert.

## Zugriff auf die Quellen

| Domain | Ergebnis am 21.10.2026 |
|---|---|
| recht.nrw.de | HTTP 200 nur für `/`, Inhalt „test-neu.recht.nrw.de – Dieser Server ist in Vorbereitung! Ihre Webadministration NRW“. Jede geprüfte Unterseite gibt HTTP 404, auch die URLs aus der Websuche: `/lrgv/gesetz/01092026-landesbauordnung-2018-bauo-nrw-2018/`, `/lrgv/gesetz/01012024-…`, `/lrgv/rechtsverordnung/26112024-verordnung-ueber-bautechnische-pruefungen-baupruefvo-1/`, die PDF unter `/system/files/pdf/…`, `/lmi/owa/br_bes_text?…`, `/gvnrw/2018-s421/`. `www.recht.nrw.de` sperrt die Umgebung. |
| www.gesetze-im-internet.de | erreichbar, Einzelnormen abgerufen |
| www.xrepository.de | erreichbar, API `api/codeliste`, `api/version_codeliste`, `api/version_standard`, Genericode-Download |
| www.xleitstelle.de | leitet auf `xleitstelle.de` weiter; nach Freigabe erreichbar (`/xbau`, `/node/199`, `/node/223`). `/xbau/releases` gibt HTTP 403. |

## Landesrecht NRW (BauO NRW, BauPrüfVO)

Alle Punkte sind **offen**, weil der Gesetzestext nicht abrufbar war. Die
Fundstellen in `landesrecht.json` stammen weiter aus Suchergebnissen und
Fachveröffentlichungen. `quellen` je Fassung nennt die amtlichen URLs mit
`abgerufen: null` und einem Vermerk. Die Verfahrenswahl zeigt dazu einen Hinweis.

| Regel | Fundstelle im Code | Code-Stand | Prüfung | Offen |
|---|---|---|---|---|
| Fassungswechsel, Übergang | `landesrecht.json` `gilt_bis` 2026-08-31 / `gilt_ab` 2026-09-01; Stichtag Antragseingang | unverändert | nicht möglich | Inkrafttreten des Gesetzes vom 21.07.2026 (GV. NRW. S. 537) und Übergangsregel für Anträge, die vor dem 01.09.2026 eingehen |
| Verfahrensfreie Vorhaben | § 62 BauO NRW | unverändert | nicht möglich | Katalog in beiden Fassungen |
| Genehmigungsfreistellung | § 63 BauO NRW; alt: Wohngebäude, GK ≤ 3, B-Plan, keine Ausnahme/Befreiung, Erschließung, Monatsfrist der Gemeinde; neu: Gebäudeklasse `null` | unverändert | nicht möglich | Bedingungen, Gebäudeklassen und Monatsfrist; neuer Anwendungsbereich ab 01.09.2026 |
| Vereinfachtes / reguläres Verfahren | § 64 / § 65, große Sonderbauten § 50 BauO NRW | unverändert | nicht möglich | Abgrenzung und Katalog großer Sonderbauten |
| Genehmigungsfiktion | neu: § 74 BauO NRW, 3 Monate regulär, 6 Wochen vereinfacht, ab Vollständigkeit (`optionen.rs` `fiktion`) | unverändert | nicht möglich | Paragraf, Fristlängen, Fristbeginn, Hemmung oder Verlängerung |
| Bauvorlageberechtigung | § 67 BauO NRW (`landesrecht.json`, `nrw-bauvorlagen.json`) | unverändert | nicht möglich | Nummer nach der Änderung |
| Abweichungen | § 69 BauO NRW, § 69 Abs. 1 (`abweichungen.rs:44`, `:426`; `optionen.rs:124`) | unverändert | nicht möglich | Textform, Begründungspflicht |
| Unterschriften | § 70 Abs. 3 BauO NRW (`landesrecht.json`, `mod.rs:605`) | unverändert | nicht möglich | Rollen je Bauvorlage |
| Rücknahmefiktion | § 71 BauO NRW (`eingang.rs:241`, `termine.rs:179–181`, `schreiben.rs:189`) | unverändert | nicht möglich | Absatz und Voraussetzungen |
| Baubeginn, Absteckung | § 74 BauO NRW (`ausfuehrung.rs:211`, `:254`; `optionen.rs:293`) | unverändert | nicht möglich | **Widerspruch im Regelpaket:** § 74 steht zugleich für die neue Genehmigungsfiktion. Einer der beiden Verweise ist nach der Änderung vermutlich falsch. |
| Geltungsdauer | § 75 Abs. 1 BauO NRW, 3 Jahre (`fristen.rs:118`; `ausfuehrung.rs:230`) | unverändert | nicht möglich | Dauer und Verlängerung in der neuen Fassung |
| Vorbescheid | § 77 BauO NRW, im Code schon „(prüfen)“ (`optionen.rs:184`) | unverändert | nicht möglich | Nummer |
| Wartefrist Freistellung | § 63 BauO NRW (`ausfuehrung.rs:197`) | unverändert | nicht möglich | Frist |
| Abstandsflächen (Testdaten) | § 6 Abs. 5 BauO NRW (`xbau/antrag.rs:1261`, `xbau/folge.rs:413`) | Beispielwert in Tests | nicht nötig | – |

**BauPrüfVO NRW** (`nrw-bauvorlagen.json`): §§ 3, 4, 5, 8, 9, 10, 13, 15 und 16
konnten nicht gegen den Text geprüft werden. Offen sind insbesondere § 3
(amtlicher Lageplan), § 10 (Liegenschaftskarte, Betriebsbeschreibung) und § 13
(Erklärung zum Brandschutz bei Freistellung). Quell-URL laut Suche:
https://recht.nrw.de/lrgv/rechtsverordnung/26112024-verordnung-ueber-bautechnische-pruefungen-baupruefvo-1/
(HTTP 404).

## Bundesrecht

Quelle jeweils `https://www.gesetze-im-internet.de/<gesetz>/__<nr>.html`
(`bbaug`, `baunvo`, `vwvfg`, `vwgo`, `zpo`), abgerufen am 21.10.2026.

| Regel | Fundstelle im Code | Text (Kern) | Ergebnis |
|---|---|---|---|
| Ausnahme | § 31 Abs. 1 BauGB (`abweichungen.rs:49`) | Ausnahmen, die der B-Plan nach Art und Umfang vorsieht | stimmt |
| Befreiung | § 31 Abs. 2 BauGB (`abweichungen.rs:50`) | Grundzüge nicht berührt, Nr. 1–3, nachbarliche Interessen | stimmt; Abs. 3 (Wohnungsbau) ist im Code nicht eigens erfasst |
| Zurückstellung | § 15 BauGB (`mod.rs:381`) | bis zu zwölf Monate auf Antrag der Gemeinde | stimmt; Code setzt keine Dauer, die Frist kommt aus dem Schreiben |
| Einvernehmen | § 36 Abs. 2 Satz 2 BauGB (`termine.rs:268`, `mod.rs:383`) | gilt als erteilt, wenn nicht binnen zwei Monaten verweigert | stimmt |
| Drittanfechtung | § 212a BauGB (`optionen.rs:279`, `ausfuehrung.rs:271`) | Abs. 1: keine aufschiebende Wirkung | stimmt |
| Maß der Nutzung | §§ 16 Abs. 2 Nr. 3, 19, 19 Abs. 2, 19 Abs. 4, 20, 20 Abs. 2, 21, 23 BauNVO (`abweichungen.rs:181–204`) | wie bezeichnet | stimmt |
| Anhörung | § 28 VwVfG (`optionen.rs:110`, `eingang.rs:320`, `mod.rs:379`) | Gelegenheit zur Äußerung vor belastendem VA | stimmt (Bund) |
| Nebenbestimmungen | § 36 VwVfG (`ausfuehrung.rs:326`, `mod.rs:279`) | Befristung, Bedingung, Widerrufsvorbehalt, Auflage | stimmt (Bund) |
| Bekanntgabe | § 41 Abs. 2 VwVfG (`fristen.rs:224`), §§ 41, 43 (`optionen.rs:267`) | vierter Tag nach Aufgabe zur Post bzw. Absendung | stimmt (Bund) |
| Fiktionsbescheinigung | § 42a Abs. 3 VwVfG (`optionen.rs:225`) | Eintritt auf Verlangen schriftlich bescheinigen | stimmt (Bund); § 42a Abs. 2: drei Monate ab vollständigen Unterlagen, einmal verlängerbar, soweit nichts anderes bestimmt |
| Rechtsbehelfsfrist | §§ 70, 74 VwGO (`fristen.rs:216`, `optionen.rs:163`, `termine.rs:311`) | ein Monat nach Bekanntgabe bzw. Zustellung | stimmt |
| Fristende | § 57 Abs. 2 VwGO, § 222 Abs. 2 ZPO (`fristen.rs:217`) | Sonnabend, Sonntag, Feiertag → nächster Werktag | stimmt |
| Fehlende Belehrung | § 58 Abs. 2 VwGO (`fristen.rs:220`, `optionen.rs:159`) | ein Jahr | stimmt |
| Untätigkeitsklage | § 75 VwGO (`optionen.rs:247`) | nicht vor Ablauf von drei Monaten seit Antrag | stimmt |
| Vollziehung bei Drittwiderspruch | § 80a VwGO (`optionen.rs:283`) | Anträge auf Anordnung oder Aussetzung | stimmt |

**Offen (VwVfG NRW):** § 1 Abs. 3 VwVfG (Bund) gilt nicht, soweit ein Land
die Verwaltungstätigkeit seiner Behörden selbst regelt. Für Bauaufsichtsbehörden
in NRW gilt damit das VwVfG NRW. Der Code zitiert das VwVfG ohne Landeszusatz.
Ob die Paragrafen (§§ 28, 36, 41, 42a, 43) im VwVfG NRW gleich lauten, ließ
sich ohne recht.nrw.de nicht prüfen. Deshalb ist nichts geändert.

## XBau-Codelisten

| Liste | XRepository (21.10.2026) | Im Programm |
|---|---|---|
| `urn:xoev-de:xbau:codeliste:abweichungart` | gültige Version **2.1**, Endfassung, eingestellt 17.12.2024. Codes: 1 Befreiung (§ 31 Abs. 2 und 3 BauGB), 2 Ausnahme (Bebauungsplan, § 31 Abs. 1 BauGB), 3 Abweichung (§ 67 Abs. 1 MBO), 4 Abweichung (§ 67 Abs. 2 S. 2 MBO), 5 Ausnahme (Veränderungssperre, § 14 Abs. 2 BauGB) | **neu mitgeliefert:** `codelisten/xbau-abweichungart_2.1.xml`, unverändert aus `api/xrepository/urn:xoev-de:xbau:codeliste:abweichungart_2.1:technischerBestandteilGenericode` |
| `urn:xoev-de:xbau:codeliste:erleichterung.gegenstand` | gültige Version **1.0**, Endfassung; 24 Codes (Ziffern 1–23 § 51 MBO, 24 Sonstiges) | **neu mitgeliefert:** `codelisten/xbau-erleichterung.gegenstand_1.0.xml`, unverändert |
| `urn:xoev-de:xbau:nw:codeliste:verfahrensart` | HTTP 404. Unter demselben Schema vorhanden: BE, BW, BY, HE, HH | weiterhin keine Liste; die Nachricht lässt die optionale Verfahrensart weg |
| `urn:xoev-de:xbau:codeliste:anlagen` | „gültige Version“ laut API 2.0; 3.0 Endfassung (14.07.2025); 4.0 Entwurf (07.10.2026) | 4.0 (passt zu XBau 2.7) |
| `urn:xoev-de:xbau:codeliste:xbau-mimetypes` | gültige Version 1, Endfassung; 2026.03.26 nicht veröffentlicht (ausgesondert) | 1 |

Zuordnung im Code (`verfahren::abweichungen::xbau_code`):

- Befreiung → Code 1.
- Ausnahme → Code 2 bei § 31 BauGB, Code 5 bei § 14 BauGB in der Rechtsgrundlage.
- Abweichung nach Landesrecht → kein automatischer Code. Die Liste kennt nur
  die MBO-Fälle 3 und 4. Ob § 69 BauO NRW dem § 67 Abs. 1 MBO entspricht, ist
  ohne Landestext nicht geprüft. Das Büro wählt im Feld „XBau-Code“; die Abgabe
  nennt dazu die Codes 3 und 4.
- Erleichterung → Code wählen (Ziffern nach § 51 MBO, nicht nach Landesrecht).

## XBau-Version

- XRepository `urn:xoev-de:bmk:standard:xbau`: Versionen bis 2.7.
  **2.6**: Endfassung, eingestellt 20.06.2025. **2.7**: Status `ENTWURF`,
  eingestellt 05.08.2026.
- XLeitstelle, „Neue Gültigkeitszeiträume von XBau-Versionen“ (28.05.2025,
  https://xleitstelle.de/node/199): XBau 2.4.0.1 gilt bis 31.07.2026, danach
  XBau 2.6. Neue Versionen werden Ende Dezember endgültig festgelegt und gelten
  ab Anfang Mai des Folgejahres.
- Die Sitzung des Expertengremiums vom 21./22.04.2026 behandelte die
  „Releaseplanung XBau 2.7“ (https://xleitstelle.de/xbau/news/xbausitzung_02_26).

**Folge:** Maßgeblich für den Austausch mit Bauaufsichten ist derzeit XBau 2.6.
Die App erzeugt und prüft Nachrichten gegen den Entwurf 2.7
(`crates/core-lib/xbau/2.7`). Laut XRepository ersetzt 2.7 unter anderem die
verfahrensspezifischen Nachrichten zur formellen Prüfung durch eine generische
Kernmodul-Nachricht. Ob Bauaufsichten mit 2.6 Nachrichten nach 2.7
annehmen, ist nicht geprüft. Nicht migriert; eine Entscheidung (2.6
zusätzlich unterstützen oder bis zur Endfassung 2.7 warten) steht aus.

## Was geändert wurde

- Codelisten Abweichungsart 2.1 und Erleichterung 1.0 mitgeliefert.
- Code für Ausnahmen nach der Rechtsgrundlage eindeutig; Hinweis der Abgabe
  nennt die möglichen Codes.
- `landesrecht.json`: je Fassung `quellen` (amtliche URL, `abgerufen`,
  `vermerk`). Keine Frist, kein Wert, keine Fundstelle geändert.

## Vorschlag für die fachliche Abnahme

1. Sobald recht.nrw.de wieder Texte liefert, die offenen Punkte dieses
   Protokolls Zeile für Zeile prüfen (oder die Prüfung erneut beauftragen).
   Für jede Quelle `abgerufen` setzen und den Vermerk ersetzen.
2. Korrekturen als eigene Regeländerung mit neuer `version` des Regelpakets.
3. Eine verantwortliche Person des Büros prüft Fassungswechsel, Fiktion,
   Freistellung und Geltungsdauer und setzt danach `geprueft: true` je Fassung.
   Name und Datum der Abnahme gehören in dieses Protokoll.
