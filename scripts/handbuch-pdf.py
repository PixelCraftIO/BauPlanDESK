"""Druckfähiges Handbuch aus der gemeinsamen, generierten Offline-HTML-Ausgabe."""
from pathlib import Path
import json
import hashlib
import os
from playwright.sync_api import sync_playwright

root = Path(__file__).resolve().parents[1]
inhalt = json.loads((root / "apps/desktop/src/lib/hilfe/handbuch.json").read_text())
html = root / "docs/handbuch/Bedienhandbuch.html"
pdf = root / "docs/handbuch/Bedienhandbuch.pdf"
with sync_playwright() as p:
    browser = p.chromium.launch(headless=True, executable_path=os.environ.get("BAUPROJEKT_CHROMIUM", "/usr/bin/chromium"))
    page = browser.new_page(locale="de-DE")
    page.set_content(html.read_text(), wait_until="load")
    assert page.locator("section.kapitel").count() == len(inhalt["kapitel"])
    for k in inhalt["kapitel"]:
        assert page.locator(f"section#{k['id']} > h2").inner_text().endswith(k["titel"])
    page.pdf(path=str(pdf), prefer_css_page_size=True, print_background=True,
             outline=True, tagged=True,
             display_header_footer=True,
             header_template='<div style="font:8px Arial;width:100%;margin:0 18mm;color:#425563">BauPlanDESK · Bedienhandbuch für Architekten und Planer</div>',
             footer_template=f'<div style="font:8px Arial;width:100%;margin:0 18mm;color:#425563;display:flex;justify-content:space-between"><span>Stand: {inhalt["stand"]}</span><span>Seite <span class="pageNumber"></span> von <span class="totalPages"></span></span></div>')
    browser.close()
print(f"PDF mit {len(inhalt['kapitel'])} Kapiteln erstellt: {pdf}")
nachweis = {"stand": inhalt["stand"], "html_sha256": hashlib.sha256(html.read_bytes()).hexdigest(),
            "pdf_sha256": hashlib.sha256(pdf.read_bytes()).hexdigest()}
(pdf.parent / "pdf-stand.json").write_text(json.dumps(nachweis, indent=2) + "\n")
