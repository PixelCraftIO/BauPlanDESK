# Bildschirmfotos für das Benutzerhandbuch

Erzeugt die Bilder unter `docs/handbuch/benutzerhandbuch/bilder/` aus der
echten Anwendung mit dem mitgelieferten Beispielprojekt
„Neubau Einfamilienhaus Mustermann“.

## Funktionsweise

- `bruecke.py` liefert `apps/desktop/dist` auf Port 1420 aus. Das ist die
  `devUrl`, die ein Debug-Build der Tauri-App lädt. In `index.html` wird ein
  kleines Fernsteuerskript eingefügt; über Port 1421 nimmt die Brücke
  JavaScript entgegen, führt es im App-Fenster aus und gibt das Ergebnis
  zurück. Die Anwendung selbst wird nicht verändert, alle Tauri-Befehle laufen
  gegen das echte Backend.
- `helfer.js` enthält Bedienhilfen (Register/Bereich öffnen, Eintrag öffnen,
  Reiter wählen, Abschnitt aufklappen, Dialoge schließen).
- `aufnahmen.py` startet Xvfb, Brücke und App, öffnet über „Hilfe →
  Beispielprojekt öffnen“ eine frische Kopie, legt die amtlichen Beispiel-PDFs
  aus `formulare/beispiele` in die Formularbibliothek und nimmt die Szenen aus
  `SZENEN` mit ImageMagick (`import`) auf. Ist `pngquant` installiert, werden
  die Bilder verlustarm verkleinert.

## Voraussetzungen (Linux)

```sh
sudo apt-get install xvfb imagemagick pngquant libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev
npm ci --prefix apps/desktop && npm run build --prefix apps/desktop
CARGO_PROFILE_DEV_DEBUG=0 cargo build -p bauprojektio-desktop
```

Damit in den Bildern ein lesbarer Benutzername steht, ein eigenes Systemkonto
verwenden, z. B. `sudo useradd -m -c "Maria Meier" mmeier`, und die App mit
`BAUPROJEKT_APP_BENUTZER=mmeier` darunter starten lassen (das Skript ruft dann
`sudo -u mmeier` auf und nutzt dessen Home als Profil).

## Aufruf

```sh
# Alle Bilder neu (löscht das App-Profil des Kontos und öffnet das Beispiel frisch)
BAUPROJEKT_APP_BENUTZER=mmeier python3 scripts/handbuch-bilder/aufnahmen.py

# Nur einzelne Szenen, App und Beispielprojekt laufen bereits
python3 scripts/handbuch-bilder/aufnahmen.py --laufend nachweis-grz verfahren-portal
```

Fehlschläge werden als `FEHLT <szene> <Meldung>` gemeldet; die übrigen Bilder
entstehen trotzdem.

Für die Szenen `ifc-modell-…` schreibt das Skript vorher mit
`cargo run -p core-lib --features testdaten --example ifc_dateien` das
IFC-Testmodell nach `/tmp/handbuch-ifc/` und legt die erste Fassung in die
Projektdateien des Beispielprojekts; die Szene `ifc-modell-abgleich` legt die
zweite Fassung als neue Version ab. Die IFC-Szenen laufen in ihrer Reihenfolge
(Übernahme vor Export, Export vor Abgleich) und deshalb nur zusammen.

Für die Szenen `editor-…` legt das Skript direkt nach dem Öffnen des
Beispielprojekts die eigene Zeichnung „Entwurf Erdgeschoss“ an (`beispielzeichnung()`
in `helfer.js`: Wände mit Fenstern und Türen, Maßketten, Raumstempel aus dem
Raumbuch EG, Planzeichen, ein Abbruch-Schuppen). Das muss vor der ersten
Aufmaßszene geschehen, weil die App die Aufmaßliste danach zwischenspeichert.
Die Bilder `editor-…` erscheinen auch auf der Webseite (`web/src/lib/zeicheneditor.ts`).

## Neue Szene ergänzen

1. In `SZENEN` eine Zeile `("<bildname>", '<JavaScript>')` einfügen. Das
   JavaScript stellt die Ansicht her, z. B.
   `await register("Nachweis und Einreichung"); await bereich("GRZ")`.
2. Mit `--laufend <bildname>` ausprobieren und das Bild ansehen.
3. Das Bild im passenden Kapitel unter `docs/handbuch/benutzerhandbuch/`
   einbinden: `![Beschreibung](bilder/<bildname>.png)`.

Hinweise: Szenen, die einen Formulareditor offen lassen, gehören ans Ende
(beim Verlassen erscheint eine native Rückfrage, die spätere Bilder verdeckt).
Die gelbe Leiste des Beispielprofils wird vor jedem Foto ausgeblendet.
