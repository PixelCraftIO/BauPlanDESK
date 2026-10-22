"""Offline-Hilfe, Kontexte, Entwurfserhalt, Download und isolierter Gesamtdruck."""
from pathlib import Path
import json
import os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich

root = Path(__file__).resolve().parent
repo = root.parents[2]
inhalt = json.loads((root.parent / "src/lib/hilfe/handbuch.json").read_text())
fixture = json.loads((root / "fixtures/projekt.json").read_text())
fixture["name"] = "Handbuch-Testprojekt"
out = Path(os.environ.get("BAUPROJEKT_HANDBUCH_UI_ARTEFAKTE", "/tmp/bauplan-handbuch-ui"))
out.mkdir(parents=True, exist_ok=True)
errors = []
with sync_playwright() as p:
    browser = p.chromium.launch(headless=True, executable_path=os.environ.get("BAUPROJEKT_CHROMIUM", "/usr/bin/chromium"))
    page = browser.new_page(viewport={"width": 1440, "height": 1000}, locale="de-DE")
    page.on("pageerror", lambda e: errors.append(str(e)))
    page.add_init_script("window.fixture=" + json.dumps(fixture) + ";" + (root / "tauri-mock.js").read_text())
    page.goto(os.environ.get("BAUPROJEKT_UI_URL", "http://127.0.0.1:1460"))
    page.locator(".app-werkzeuge").get_by_role("button", name="Hilfe / Handbuch", exact=True).click()
    help = page.get_by_role("main", name="Bedienhandbuch", exact=True)
    expect(help.get_by_role("heading", name="Mit BauPlanIO arbeiten", exact=True)).to_be_visible()
    expect(help.get_by_role("button", name="Zurück zum Projekt")).to_have_count(0)
    page.screenshot(path=str(out / "hilfe-start-light.png"))
    search = help.get_by_label("Handbuch durchsuchen", exact=True)
    search.fill("Scan-PDF")
    expect(help.get_by_role("heading", name="Aktensuche, OCR und Wiedervorlagen", exact=True)).to_be_visible()
    search.fill("xyz-keine-treffer-123")
    expect(help.get_by_role("heading", name="Keine passenden Kapitel", exact=True)).to_be_visible()
    help.get_by_role("button", name="Suche zurücksetzen", exact=True).click()
    with page.expect_download() as event:
        help.get_by_role("button", name="Handbuch herunterladen (.md)", exact=True).click()
    download = event.value
    download.save_as(out / download.suggested_filename)
    assert (out / download.suggested_filename).read_text() == (repo / "docs/handbuch/Bedienhandbuch.md").read_text()
    page.locator(".app-werkzeuge").get_by_role("button", name="Projekte", exact=True).click()
    page.locator(".projekttabelle").get_by_role("button", name=fixture["name"], exact=True).click()
    bereich(page, "Rechtsgrundlagen")
    plan = page.locator(".projekt-sitzung:visible").get_by_label("Planbezeichnung", exact=True)
    plan.fill("Erhaltener Planentwurf")
    page.locator(".projekt-sitzung:visible").get_by_role("button", name="Hilfe zu diesem Bereich", exact=True).click()
    expect(help.get_by_role("heading", name="Rechtsgrundlagen des Vorhabens", exact=True)).to_be_visible()
    # Strg+S in der Hilfe darf keinen ausgeblendeten Projektentwurf speichern.
    page.evaluate('window.emitNative("menu", "anwendung.speichern")')
    expect(page.get_by_role("status").filter(has_text="In dieser Ansicht gibt es nichts zu speichern")).to_be_visible()
    page.get_by_role("button", name="OK", exact=True).click()
    assert not page.evaluate("window.calls.some(c => c.cmd === 'projekt_bereich_speichern')")
    help.get_by_role("button", name="Zurück zum Projekt", exact=True).click()
    expect(plan).to_have_value("Erhaltener Planentwurf")
    page.evaluate('window.emitNative("menu", "hilfe.handbuch")')
    expect(help.get_by_role("heading", name="Rechtsgrundlagen des Vorhabens", exact=True)).to_be_visible()
    help.get_by_role("button", name="Rechtsgrundlagen öffnen", exact=True).click()
    expect(plan).to_have_value("Erhaltener Planentwurf")
    bereich(page, "Abstandsflächen")
    page.locator(".projekt-sitzung:visible").get_by_role("button", name="Hilfe zu diesem Bereich", exact=True).click()
    expect(help.get_by_role("heading", name="Abstandsflächen nachweisen", exact=True)).to_be_visible()
    for theme in ["light", "dark"]:
        page.evaluate("theme => document.documentElement.dataset.theme = theme", theme)
        search.fill("Nachforderung Nachreichung")
        help.get_by_role("navigation", name="Inhaltsverzeichnis", exact=True).get_by_role("button", name="Workflow: Nachforderung und Nachreichung", exact=True).click()
        assert help.locator("article ol").first.evaluate("e => getComputedStyle(e).listStyleType") == "decimal"
        assert help.locator("article ol li").first.evaluate("e => getComputedStyle(e).display") == "list-item"
        page.screenshot(path=str(out / f"nachforderung-{theme}.png"))
        for width in [1024, 800, 600]:
            page.set_viewport_size({"width": width, "height": 950})
            assert page.evaluate("document.documentElement.scrollWidth <= innerWidth"), (theme, width)
            if width == 600:
                page.screenshot(path=str(out / f"hilfe-600-{theme}.png"))
        page.set_viewport_size({"width": 1440, "height": 1000})
    # Der Gesamtdruck enthält trotz aktiver Suche alle Kapitel und keine Projektdaten.
    page.evaluate("window.print = () => { window.handbuchPrint = true; }")
    help.get_by_role("button", name="Gesamtes Handbuch drucken", exact=True).click()
    assert page.evaluate("window.handbuchPrint")
    druck = page.locator("#bau-handbuch-druck")
    expect(druck.locator("section.kapitel")).to_have_count(len(inhalt["kapitel"]))
    expect(druck).not_to_contain_text(fixture["name"])
    page.emulate_media(media="print")
    expect(page.locator(".app-werkzeuge")).not_to_be_visible()
    expect(druck).to_be_visible()
    page.pdf(path=str(out / "handbuch-app-druck.pdf"), prefer_css_page_size=True, print_background=True)
    page.emulate_media(media="screen")
    page.evaluate("window.dispatchEvent(new Event('afterprint'))")
    expect(druck).to_have_count(0)
    expect(page.locator(".app-werkzeuge")).to_be_visible()
    assert not page.evaluate("window.calls.some(c => /speichern|loeschen|importieren|ablegen/.test(c.cmd))")
    assert not errors, errors
    browser.close()
print(f"Handbuch: {len(inhalt['kapitel'])} Kapitel, Kontext, Suche, Download, Entwurfserhalt, Hell/Dunkel, schmale Ansichten und isolierter Druck geprüft.")
