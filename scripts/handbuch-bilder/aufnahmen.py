"""Erzeugt die Bildschirmfotos des Benutzerhandbuchs aus der echten Anwendung.

Ablauf: Xvfb starten, Brücke (bruecke.py) für apps/desktop/dist bereitstellen,
Debug-Binary starten, über „Hilfe → Beispielprojekt öffnen“ eine frische Kopie
von „Neubau Einfamilienhaus Mustermann“ laden und die Szenen unten der Reihe
nach aufnehmen. Ergebnis: docs/handbuch/benutzerhandbuch/bilder/<name>.png.

Voraussetzungen (Linux): Xvfb, ImageMagick (import), WebKitGTK-Laufzeit,
`npm run build --prefix apps/desktop` und
`CARGO_PROFILE_DEV_DEBUG=0 cargo build -p bauprojektio-desktop`.

Aufrufe:
  python3 scripts/handbuch-bilder/aufnahmen.py                # alles neu
  python3 scripts/handbuch-bilder/aufnahmen.py grz abstand    # nur diese Szenen
  python3 scripts/handbuch-bilder/aufnahmen.py --laufend grz  # App läuft schon

Umgebungsvariablen: BAUPROJEKT_APP_BENUTZER (Systemkonto, unter dem die App
läuft, z. B. „mmeier“ mit vollem Namen „Maria Meier“; sonst der aktuelle
Benutzer), BAUPROJEKT_APP_HOME (Profilverzeichnis, Standard: Home des Kontos).
"""

import glob
import json
import os
import shutil
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

WURZEL = Path(__file__).resolve().parents[2]
HIER = Path(__file__).resolve().parent
ZIEL = WURZEL / "docs/handbuch/benutzerhandbuch/bilder"
DISPLAY = os.environ.get("BAUPROJEKT_DISPLAY", ":77")
HELFER = (HIER / "helfer.js").read_text()
BENUTZER = os.environ.get("BAUPROJEKT_APP_BENUTZER")


def js(code, timeout=90, fenster="main"):
    daten = json.dumps({"js": HELFER + "\n" + code, "label": fenster, "timeout": timeout}).encode()
    antwort = urllib.request.urlopen(urllib.request.Request("http://127.0.0.1:1421", daten), timeout=timeout + 10)
    d = json.loads(antwort.read())
    if not d.get("ok"):
        raise RuntimeError(d.get("v"))
    return d.get("v")


def foto(name, ausschnitt=None):
    ZIEL.mkdir(parents=True, exist_ok=True)
    befehl = ["import", "-display", DISPLAY, "-window", "root"]
    if ausschnitt:
        befehl += ["-crop", ausschnitt, "+repage"]
    datei = ZIEL / f"{name}.png"
    subprocess.run(befehl + [str(datei)], check=True)
    # Verkleinert die Dateien auf etwa ein Drittel, Text bleibt scharf.
    if shutil.which("pngquant"):
        subprocess.run(["pngquant", "--quality", "80-95", "--strip", "--force", "--ext", ".png", str(datei)])


def warte_auf_app(sekunden=60):
    ende = time.time() + sekunden
    while time.time() < ende:
        try:
            js("return document.readyState", timeout=3)
            return
        except Exception:
            time.sleep(1)
    raise RuntimeError("App meldet sich nicht")


def home():
    if os.environ.get("BAUPROJEKT_APP_HOME"):
        return Path(os.environ["BAUPROJEKT_APP_HOME"])
    return Path(os.path.expanduser(f"~{BENUTZER}" if BENUTZER else "~"))


def app_starten(frisch):
    subprocess.run(["pkill", "-x", "bauprojektio"])
    time.sleep(1)
    h = home()
    if frisch:
        shutil.rmtree(h / ".local/share/io.pixelcraft.bauprojektio", ignore_errors=True)
        shutil.rmtree(h / ".config/io.pixelcraft.bauprojektio", ignore_errors=True)
    umgebung = ["env", f"DISPLAY={DISPLAY}", f"HOME={h}", "LANG=de_DE.UTF-8", "TZ=Europe/Berlin"]
    vorne = ["sudo", "-u", BENUTZER] if BENUTZER else []
    log = open("/tmp/handbuch-app.log", "w")
    subprocess.Popen(vorne + umgebung + [str(WURZEL / "target/debug/bauprojektio")], cwd=WURZEL, stdout=log, stderr=log)
    warte_auf_app()


def umgebung_starten():
    if subprocess.run(["pgrep", "-f", f"Xvfb {DISPLAY}"], capture_output=True).returncode:
        subprocess.Popen(["Xvfb", DISPLAY, "-screen", "0", "1440x900x24"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(1)
    try:
        urllib.request.urlopen("http://127.0.0.1:1420/", timeout=2)
    except Exception:
        subprocess.Popen([sys.executable, str(HIER / "bruecke.py"), str(WURZEL / "apps/desktop/dist")])
        time.sleep(1)


def beispielprojekt_oeffnen():
    """Frische Kopie des Beispielprojekts; die App startet dabei neu."""
    js('await window.__TAURI_INTERNALS__.invoke("plugin:event|emit",{event:"menu",payload:"hilfe.beispielprojekt"}); await sleep(900); return 1')
    foto("arbeitsplatz-beispielprojekt-oeffnen")
    try:
        js('klick("Beispielprojekt öffnen","[role=dialog] button"); return 1', timeout=5)
    except Exception:
        pass  # Fenster wird während der Antwort neu gestartet
    time.sleep(8)
    warte_auf_app()
    # Das Beispielprofil bringt keine Formularvorlagen mit; die amtlichen
    # Beispiel-PDFs aus formulare/beispiele dienen als Bibliothek.
    for ordner in glob.glob(str(home() / ".local/share/io.pixelcraft.bauprojektio/Beispielprojekt/*/Formularvorlagen")):
        for pdf in (WURZEL / "formulare/beispiele").glob("*.pdf"):
            shutil.copy(pdf, ordner)
            if BENUTZER:
                shutil.chown(Path(ordner) / pdf.name, BENUTZER)
    js('await klick("Öffnen","main button, button"); await sleep(2500); return 1')


# Jede Szene: Name der Bilddatei und JavaScript, das die Ansicht herstellt.
# Szenen laufen in dieser Reihenfolge; spätere setzen keinen bestimmten
# Zustand voraus außer der geöffneten Projektakte.
SZENEN = [
    # Arbeitsplatz
    ("arbeitsplatz-projektverzeichnis", 'await arbeitsplatz("Projekte")'),
    ("arbeitsplatz-neues-projekt", 'await arbeitsplatz("Projekte"); await klick("Neues Projekt")'),
    ("arbeitsplatz-kunden", 'await arbeitsplatz("Kunden / Bauherren"); await klick("Max Mustermann")'),
    ("arbeitsplatz-formularbibliothek", 'await arbeitsplatz("Formularbibliothek")'),
    ("einstellungen-speicherorte", 'await arbeitsplatz("Einstellungen"); await zeige("Darstellung","h2,h3")'),
    ("einstellungen-buero", 'await arbeitsplatz("Einstellungen"); await imAbschnitt("Büroangaben","Bearbeiten"); await sleep(800)'),
    ("einstellungen-benutzer", 'await arbeitsplatz("Einstellungen"); await imAbschnitt("Benutzer und Rechte","Öffnen"); await sleep(800)'),
    ("einstellungen-briefbogen", 'await arbeitsplatz("Einstellungen"); await aufklappen("Briefbogen und Druckvorlagen"); await zeige("Briefbogen und Druckvorlagen","summary")'),
    ("arbeitsplatz-hilfe", 'await arbeitsplatz("Hilfe / Handbuch")'),
    # Übersicht
    ("akte-projektuebersicht", 'await register("Übersicht"); await bereich("Projektübersicht")'),
    ("akte-arbeitscheck", 'await register("Übersicht"); await bereich("Projektübersicht"); await aufklappen("Arbeitscheck vor der Abgabe"); await zeige("Arbeitscheck vor der Abgabe","summary")'),
    ("akte-aufgaben", 'await register("Übersicht"); await bereich("Aufgaben / Fristen"); await eintrag("bei der Bauaufsicht nachreichen")'),
    ("akte-chronik", 'await register("Übersicht"); await bereich("Chronik")'),
    # Bereich 1 – Ermittlung
    ("ermittlung-stammdaten", 'await register("Ermittlung"); await bereich("Stammdaten")'),
    ("ermittlung-rechtsgrundlagen", 'await register("Ermittlung"); await bereich("Rechtsgrundlagen")'),
    ("ermittlung-festsetzungen", 'await register("Ermittlung"); await bereich("Planfestsetzungen")'),
    ("ermittlung-flurstuecke", 'await register("Ermittlung"); await bereich("Flurstücke")'),
    ("ermittlung-projektdateien", 'await register("Ermittlung"); await bereich("Projektdateien")'),
    ("ermittlung-aufmass", 'await register("Ermittlung"); await bereich("Aufmaß"); if(!finde("Rückgängig")) await eintrag()'),
    ("ermittlung-aufmass-vollbild", 'await register("Ermittlung"); await bereich("Aufmaß"); if(!finde("Rückgängig")) await eintrag(); await klick("Vollbild"); await sleep(1500)'),
    ("ermittlung-aufmass-objekte", 'await klick("Objekte","[role=tab],button"); await sleep(800)'),
    ("ermittlung-messwerte", 'await taste("Escape"); await register("Ermittlung"); await bereich("Messwerte")'),
    ("ermittlung-unterlagenregister", 'await register("Ermittlung"); await bereich("Unterlagenregister")'),
    ("ermittlung-gebaeude", 'await register("Ermittlung"); await bereich("Gebäude / Geschosse"); await eintrag("Wohnhaus")'),
    ("ermittlung-raumbuch", 'await register("Ermittlung"); await bereich("Flächen- und Raumbuch"); await eintrag("Erdgeschoss")'),
    # Bereich 2 – Nachweis und Einreichung
    ("nachweis-baugrundstueck", 'await register("Nachweis und Einreichung"); await bereich("Baugrundstück"); await zeige("2 · Baugrundstücksermittlung")'),
    ("nachweis-grz", 'await register("Nachweis und Einreichung"); await bereich("GRZ")'),
    ("nachweis-grz-ergebnis", 'await register("Nachweis und Einreichung"); await bereich("GRZ"); await klick("Berechnen"); let t; for(let i=0;i<30&&!t;i++){await sleep(500); t=[...haupt().querySelectorAll("table")].filter(sichtbar).pop()} if(!t) throw new Error("kein Ergebnis"); t.scrollIntoView({block:"center"})'),
    ("nachweis-wohnflaeche", 'await register("Nachweis und Einreichung"); await bereich("Wohnflächenberechnung"); await eintrag()'),
    ("nachweis-stellplaetze", 'await register("Nachweis und Einreichung"); await bereich("Stellplatznachweis"); await eintrag()'),
    ("nachweis-abstandsflaechen", 'await register("Nachweis und Einreichung"); await bereich("Abstandsflächen"); await eintrag()'),
    ("nachweis-nachweisstaende", 'await register("Nachweis und Einreichung"); await bereich("Nachweisstände")'),
    ("nachweis-formulare", 'await register("Nachweis und Einreichung"); await bereich("Formulare")'),
    ("nachweis-druckausgabe", 'await register("Nachweis und Einreichung"); await bereich("Drucken / PDF-Ausgabe")'),
    # Verfahren
    ("verfahren-akte", 'await register("Nachweis und Einreichung"); await bereich("Verfahrensakte"); if(!finde("Vorbereitung")) await eintrag(); await klick("Vorbereitung"); await reiter("Verfahren")'),
    ("verfahren-anforderungen", 'await klick("Vorbereitung"); await reiter("Anforderungen")'),
    ("verfahren-kommunikation", 'await klick("Laufendes Verfahren"); await reiter("Kommunikation")'),
    ("verfahren-nachforderungen", 'await klick("Laufendes Verfahren"); await reiter("Nachforderungen")'),
    ("verfahren-abgabestaende", 'await klick("Laufendes Verfahren"); await reiter("Abgabestände")'),
    ("verfahren-bescheide", 'await klick("Entscheidung und Ausführung"); await reiter("Bescheide / Auflagen")'),
    ("verfahren-bauausfuehrung", 'await klick("Entscheidung und Ausführung"); await reiter("Bauausführung")'),
    ("verfahren-einreichen", 'await register("Nachweis und Einreichung"); await bereich("Einreichen / Nachreichen"); const s=haupt().querySelector("select"); if(s){s.selectedIndex=1; s.dispatchEvent(new Event("change",{bubbles:true})); await sleep(2500)}'),
    ("verfahren-bauantrag", 'await register("Nachweis und Einreichung"); await bereich("Bauantrag / XBau")'),
    ("verfahren-portal", 'await register("Nachweis und Einreichung"); await bereich("Einreichen / Portal")'),
    ("verfahren-xbau", 'await register("Nachweis und Einreichung"); await bereich("XBau-Nachrichten")'),
    ("verfahren-aktensuche", 'await register("Nachweis und Einreichung"); await bereich("Suche / Wiedervorlagen")'),
    # IFC-Modell (Kapitel 5): die Testmodelle aus ifc_vorbereiten().
    ("ifc-modell-waehlen", 'await register("Ermittlung"); await bereich("Aufmaß"); await klick("IFC-Modell","button"); await sleep(1500); await waehle("IFC-Datei","EFH-Mustermann.ifc"); await tippe("Bezeichnung des Modells","Entwurf Architekt")'),
    ("ifc-modell-struktur", 'await klick("Modell öffnen"); await sleep(4000); const st=document.querySelector("[aria-label=Modellstruktur]"); const w=[...st.querySelectorAll("summary")].find(s=>txt(s).startsWith("Wand")); w.click(); await sleep(400); w.parentElement.querySelector("button.blatt").click(); await sleep(1200)'),
    ("ifc-modell-3d", 'await ifcModell(); await klick("3D","[role=radio]"); await sleep(8000)'),
    ("ifc-modell-bericht", 'await ifcModell(); await klick("Grundriss","[role=radio]"); await ifcReiter("Modellbericht")'),
    ("ifc-modell-pruefung", 'await ifcModell(); await ifcReiter("Prüfung"); await sleep(1500)'),
    ("ifc-modell-nacharbeit", 'await ifcModell(); await ifcReiter("Prüfung"); await sleep(1500); await klick("Klassifizieren","[aria-label=Details] button"); await sleep(1500)'),
    ("ifc-modell-auswertung", 'await ifcModell(); await ifcReiter("Auswertung"); await sleep(1500)'),
    ("ifc-modell-uebernahme", 'await ifcModell(); await ifcReiter("Übernahme"); await klick("In Projekt übernehmen"); await sleep(6000)'),
    ("ifc-modell-export", 'await ifcModell(); await ifcReiter("Export"); await klick("Modelldatei dem Bauantrag beifügen"); await sleep(2500); await waehle("Gebäude für das Modell","(Entwurf Architekt)"); await sleep(1000); await klick("IFC4-Modell schreiben"); await sleep(5000); ifcDetailsEnde()'),
    ("ifc-modell-abgleich", 'await ifcModell(); await klick("Entwurf Architekt schließen","button"); await sleep(800); const p=(await window.__TAURI_INTERNALS__.invoke("projekte_auflisten")).find(x=>x.name==="Neubau Einfamilienhaus Mustermann"); const d=(await window.__TAURI_INTERNALS__.invoke("dokumente_auflisten",{projektId:p.id})).find(x=>x.file_name==="EFH-Mustermann.ifc"); await window.__TAURI_INTERNALS__.invoke("dokument_importieren",{projektId:p.id,dokumentId:d.document_id,basis:d.version,kategorie:null,pfad:IFC_ORDNER+"/v2/EFH-Mustermann.ifc"}); await klick("IFC-Modell","button"); await sleep(1500); await klick("Entwurf Architekt","section[aria-label^=IFC-Modell] button"); await sleep(4000); await klick("Neue Version","button"); await sleep(1500); await klick("Abgleichen","[aria-label=Details] button"); await sleep(3000); await aufklappen("Geändert"); ifcDetailsEnde()'),
    # Zeicheneditor (Webseite /zeicheneditor, Kapitel 5): EP-01 und die eigene
    # Zeichnung „Entwurf Erdgeschoss“ aus zeichnung_vorbereiten().
    ("editor-aufgaben", 'await aufmassTab("EP-01"); await vollbildAn(); await klick("Aufgaben","[role=tab],button"); await waehleText("Alle Ziele", "Wohnfläche"); await klick("Einpassen"); await sleep(800); await zoomAufText("Grundriss EG", 2.0)'),
    ("editor-eigenschaften", 'await planKlick(640, 440); await klick("Eigenschaften","[role=tab],button"); await sleep(800); [...document.querySelectorAll("label")].filter(sichtbar).find(x=>txt(x).startsWith("Bezeichnung")).scrollIntoView({block:"start"}); await sleep(500)'),
    ("editor-werkzeugkiste", 'await taste("Escape"); await klick("Vorlagen für Polygon","button"); await sleep(600)'),
    ("editor-zeichnung", 'await taste("Escape"); await aufmassTab("Entwurf Erdgeschoss"); await vollbildAn(); await klick("Einpassen"); await sleep(800); await zoomAufText("Grundriss Erdgeschoss", 2.0); await klick("Objekte","[role=tab],button"); await sleep(800); [...document.querySelectorAll(".baum button[data-element=wand]")].filter(sichtbar)[2].click(); await sleep(1500)'),
    ("editor-konstruieren", 'const o=[...document.querySelectorAll("label")].filter(sichtbar).find(l=>txt(l)==="Ortho").querySelector("input"); if(!o.checked) o.click(); const w=[...document.querySelectorAll("select")].filter(sichtbar).find(x=>[...x.options].some(o=>o.text==="frei")); w.value="15"; w.dispatchEvent(new Event("change",{bubbles:true})); await klick("Versatz","button"); await sleep(400); await klick("Fangoptionen"); await sleep(600)'),
    ("editor-darstellung", 'await klick("Fangoptionen"); await taste("Escape"); await klick("Auswählen","button"); await klick("Objekte","[role=tab],button"); await sleep(800); await klick("Wände",".baum .name"); await sleep(800)'),
    ("editor-planlayout", 'await klick("Ausgabe ▾"); await sleep(400); await klick("Planblatt PDF (maßstäblich) …","[role=menuitem]"); await sleep(1200); const i=[...document.querySelectorAll(".werkzeugfenster input")].find(x=>x.maxLength===300); i.value="Grundriss Erdgeschoss"; i.dispatchEvent(new Event("input",{bubbles:true})); await sleep(400)'),
    # Zuletzt: Das Verlassen des Formulareditors fragt nativ nach und würde
    # spätere Aufnahmen verdecken.
    ("nachweis-formular-ausfuellen", 'if(finde("Vollbild beenden")) await klick("Vollbild beenden"); await register("Nachweis und Einreichung"); await bereich("Formulare"); await klick("Ausfüllen"); await sleep(8000)'),
]


IFC_ORDNER = "/tmp/handbuch-ifc"


def ifc_vorbereiten():
    """Legt das IFC-Testmodell (erste Fassung) in die Projektdateien des
    Beispielprojekts; die zweite Fassung liegt für den Abgleich bereit."""
    subprocess.run(["cargo", "run", "-q", "-p", "core-lib", "--features", "testdaten", "--example", "ifc_dateien", "--", IFC_ORDNER], cwd=WURZEL, check=True)
    (Path(IFC_ORDNER) / "v2").mkdir(exist_ok=True)
    shutil.move(f"{IFC_ORDNER}/EFH-Mustermann-V2.ifc", f"{IFC_ORDNER}/v2/EFH-Mustermann.ifc")
    subprocess.run(["chmod", "-R", "a+rX", IFC_ORDNER], check=True)
    js(f'const p=(await window.__TAURI_INTERNALS__.invoke("projekte_auflisten")).find(x=>x.name==="Neubau Einfamilienhaus Mustermann"); await window.__TAURI_INTERNALS__.invoke("dokument_importieren",{{projektId:p.id,dokumentId:null,basis:null,kategorie:null,pfad:"{IFC_ORDNER}/EFH-Mustermann.ifc"}}); return 1')


def zeichnung_vorbereiten():
    """Legt die eigene Zeichnung „Entwurf Erdgeschoss“ an (helfer.js,
    beispielzeichnung). Muss vor der ersten Aufmaßszene laufen, weil die
    Aufmaßliste danach zwischengespeichert ist."""
    js("await beispielzeichnung(); return 1")


def main(argumente):
    laufend = "--laufend" in argumente
    auswahl = [a for a in argumente if not a.startswith("--")]
    if not laufend:
        umgebung_starten()
        app_starten(frisch=True)
        beispielprojekt_oeffnen()
        zeichnung_vorbereiten()
        if not auswahl or any(a.startswith("ifc-") for a in auswahl):
            ifc_vorbereiten()
    for name, code in SZENEN:
        if auswahl and name not in auswahl:
            continue
        try:
            js(f"const IFC_ORDNER={json.dumps(IFC_ORDNER)}; await dialogeSchliessen(); " + code + "; aufraeumen(); await sleep(700); return 1")
            foto(name)
            print("ok   ", name)
        except Exception as fehler:
            print("FEHLT", name, str(fehler).splitlines()[0][:200])


if __name__ == "__main__":
    main(sys.argv[1:])
