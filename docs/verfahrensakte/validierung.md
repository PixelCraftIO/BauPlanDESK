# Validierung

Ausgeführt auf dem Implementierungsbranch, ausgehend von Main `45791e9`.

| Prüfung | Ergebnis |
|---|---|
| `cargo test --workspace --locked` | 224 Tests bestanden, einschließlich echter PostgreSQL-Tests |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | bestanden |
| `cargo fmt --all --check` | bestanden |
| Generierte TypeScript-Verträge gegen Rust-Generator | identisch |
| `npm run check` | keine Fehler oder Warnungen |
| `npm run build` | bestanden |
| Produktionsbuild: elf Playwright-Skripte | bestanden |
| PDF-Livevorschau: Node-Tests | vier Tests bestanden |
| npm-Lizenzprüfung | 54 Pakete, keine Verstöße |

Der Musterfall prüft Nachforderung mit drei Punkten, Teilantwort,
Fristverlängerung, zurückgewiesenen Upload, erfolgreichen Eingang,
Genehmigung von Lageplan v2, offene Nebenbestimmung und späteren Entwurf v3.
ZIP und PDF bleiben unverändert. Ein eigener XBau-Test weist nach, dass spätere
Büro- und Planänderungen das bereits gespeicherte XBau-Paket nicht verändern.
Projektgrenzen, Leserrechte, konkurrierende Bearbeitung, unveränderliche Originale,
Vorgängerzyklen und Audit-Rollback sind als Fehlerfälle geprüft.

Die PostgreSQL-Sicherung wurde mit PostgreSQL 16 und passenden Clientprogrammen
in getrennten kurzlebigen Datenbanken erstellt und tatsächlich wiederhergestellt.
Der Test weist nach, dass nachträgliche Quelldatenänderungen nicht im Backup
erscheinen und befüllte Zieldatenbanken abgewiesen werden.
`BAUPROJEKT_TEST_BUERO_SICHERUNG` aktiviert diesen Test ausdrücklich; CI setzt die
Variable und installiert die Clients. Tesseract hat einen echten Bildbeleg erkannt;
CI aktiviert diesen Test über `BAUPROJEKT_TEST_OCR` und installiert `deu`/`eng`.

Die Browserprüfungen verwenden das Produktionsbundle, einen IPC-Testtransport
und echte Core-Dokumente. Sie ersetzen keine native Windows-/NAS-Praxisabnahme.
Sie prüfen unter anderem Entwurfserhalt bei Konflikten, PDF-Textextraktion,
Versandberichtigungen, Nebenbestimmungen, E-Mail-Anhangsübernahme und wiederholtes
Ablegen derselben Druckausgabe mit derselben Vorgangskennung.

Screenshots:

- [Bescheid und genehmigter Stand](screenshots/genehmigter-stand.png)
- [Suche und Wiedervorlagen](screenshots/suche-wiedervorlagen.png)

Nicht Gegenstand einer technischen Testabnahme sind amtliche Freigabe von
Landeskatalogen, rechtliche Vollständigkeitszertifizierung, S3-Betreibersnapshots
oder tatsächliche Übermittlung über behördliche Portale. Grenzen und benötigte
Betriebsvoraussetzungen stehen in der [Bedienungsdokumentation](README.md).
