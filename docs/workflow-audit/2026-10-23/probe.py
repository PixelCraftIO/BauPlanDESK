"""Explorative UI-Belege; kein Test der nativen Berechnungs-/Portaladapter.

Vom Repository aus starten: python docs/workflow-audit/2026-10-23/probe.py
Benötigt Produktionsserver (BAUPROJEKT_UI_URL), Python Playwright und Chromium.
"""
from pathlib import Path
import json
import os
import sys
from playwright.sync_api import sync_playwright, expect

here = Path(__file__).resolve().parent
repo = here.parents[2]
tests = repo / "apps/desktop/tests"
sys.path.insert(0, str(tests))
from navigation_hilfe import bereich, register

fixture = json.loads((tests / "fixtures/projekt.json").read_text())
fixture["name"] = "UX-Prüfung: Einfamilienhaus"
fixture["bundesland"] = "NW"
fixture["planungsrecht"]["rechtsgrundlage"] = "par34"
fixture["planungsrecht"]["bau_nvo"] = None
fixture["planungsrecht"]["bebauungsplan_art"] = "kein"
for f in fixture["planungsrecht"]["baugrundstuecke"]["flurstuecke"]:
    f.update(grundbuchbezirk="", grundbuchblatt="", laufende_nummer="")
rules = json.loads((repo / "regelwerke/abstandsflaechen/pilot.json").read_text())
custom = """
const original = window.__TAURI_INTERNALS__.invoke;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 if(cmd==='abstandsregeln')return window.auditRules;
 if(cmd==='formularbibliothek')return {root:'/tmp/Formulare',vorlagen:[]};
 if(cmd==='gebaeude_auswerten')return {grundflaeche_m2:null,geschossflaechensumme_m2:null,geschosse:[],offen:[]};
 return original(cmd,args);
};
"""
out = Path(os.environ.get("BAUPROJEKT_AUDIT_ARTEFAKTE", str(here / "belege")))
out.mkdir(parents=True, exist_ok=True)
result = {"basis": "e80717f39fe82f05d21705a0e70a5f94662d26d2",
          "modus": "Produktions-UI mit Test-IPC, keine native End-to-End-Prüfung",
          "viewport": {"width": 1366, "height": 900}, "seiten": [], "pruefungen": []}
with sync_playwright() as pw:
    browser = pw.chromium.launch(executable_path=os.environ.get("BAUPROJEKT_CHROMIUM", "/usr/bin/chromium"))
    page = browser.new_page(viewport=result["viewport"], locale="de-DE")
    errors = []
    page.on("pageerror", lambda e: errors.append(str(e)))
    page.add_init_script("window.fixture=" + json.dumps(fixture) + ";window.auditRules=" + json.dumps(rules) + ";" + (tests / "tauri-mock.js").read_text() + custom)
    page.goto(os.environ.get("BAUPROJEKT_UI_URL", "http://127.0.0.1:1465"))
    page.locator(".projekttabelle").get_by_role("button", name=fixture["name"], exact=True).click()
    active = page.locator(".projekt-sitzung:visible")

    def capture(name, slug):
        page.screenshot(path=str(out / (slug + ".png")))
        result["seiten"].append({"bereich": name, "beleg": slug + ".png",
            "sichtbare_eingaben": active.locator("input:visible,select:visible,textarea:visible").count(),
            "sichtbare_schaltflaechen": active.locator("button:visible").count(),
            "seitenueberlauf": page.evaluate("document.documentElement.scrollWidth > innerWidth"),
            "scrollhoehe": page.evaluate("document.documentElement.scrollHeight"),
            "meldungen": active.locator("[role=alert]:visible").all_text_contents()})

    register(page, "Berechnungen")
    for name in ["GRZ", "GFZ", "Geschossigkeit", "Bauvolumen / BMZ"]:
        button = active.locator(".aktennavigation").get_by_role("button", name=name, exact=True)
        expect(button).to_be_disabled()
        assert button.get_attribute("title") == "Zuerst Rechtsgrundlage und BauNVO festlegen"
    capture("§34 ohne BauNVO: gesperrte Nutzungsmodule", "01-par34-navigation")
    result["pruefungen"].append("§34 ohne BauNVO sperrt vier Nutzungsmodule; Grund nur als title am deaktivierten Button")
    bereich(page, "Baugrundstück")
    expect(active.get_by_text("Optional – Flurstück wird einzeln angesetzt", exact=True).first).to_be_visible()
    capture("Baugrundstück ohne Grundbuch", "02-baugrundstueck")
    result["pruefungen"].append("Kein Grundbuch-Pflichtfehler in der UI; Core-Gruppierung hier nicht ausgeführt")
    bereich(page, "Gebäude / Geschosse")
    active.get_by_role("button", name="Neu anlegen", exact=True).click()
    active.get_by_label("Gebäudebezeichnung", exact=True).fill("Wohnhaus Entwurf")
    capture("Gebäude bearbeiten", "03-gebaeude")
    bereich(page, "Abstandsflächen")
    active.get_by_role("button", name="Neu anlegen", exact=True).click()
    capture("Abstandsflächen Grundlagen", "04-abstand-grundlagen")
    active.get_by_role("navigation", name="Abstandsflächenbereiche").get_by_role("button", name="Wände / Bauteile", exact=True).click()
    capture("Abstandsflächen Wände", "05-abstand-waende")
    bereich(page, "Formulare")
    capture("Leere Formularbibliothek im Projekt", "06-formulare-leer")
    bereich(page, "Einreichen / Nachreichen")
    expect(active.get_by_role("button", name="Portalübergabe vorbereiten", exact=True)).to_be_disabled()
    expect(active.get_by_role("button", name="Formulare ausfüllen", exact=True)).to_be_enabled()
    capture("Abgabe ohne Verfahrensakte", "07-einreichen")
    result["pruefungen"].append("Ohne Verfahrensakte bleibt Formularvorbereitung zugänglich; Portalübergabe gesperrt")
    bereich(page, "Gebäude / Geschosse")
    expect(active.get_by_label("Gebäudebezeichnung", exact=True)).to_have_value("Wohnhaus Entwurf")
    result["pruefungen"].append("Gebäudeentwurf bleibt nach Wechsel über Abstandsflächen, Formulare und Einreichen erhalten")
    result["javascript_fehler"] = errors
    assert not errors, errors
    browser.close()
(out / "probe.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
print(json.dumps(result, ensure_ascii=False, indent=2))
