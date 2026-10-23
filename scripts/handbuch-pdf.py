"""Illustriertes Offline-Handbuch mit geprüftem Inhaltsverzeichnis und Kapitelstartseiten."""
from pathlib import Path
import json
import hashlib
import os
import re
import fitz
from playwright.sync_api import sync_playwright

root = Path(__file__).resolve().parents[1]
inhalt = json.loads((root / "apps/desktop/src/lib/hilfe/handbuch.json").read_text())
html = root / "docs/handbuch/Bedienhandbuch.html"
pdf = root / "docs/handbuch/Bedienhandbuch.pdf"
bilder = [a['abbildung'] for k in inhalt['kapitel'] for a in k['abschnitte'] if 'abbildung' in a]
with sync_playwright() as p:
    browser = p.chromium.launch(headless=True, executable_path=os.environ.get("BAUPROJEKT_CHROMIUM", "/usr/bin/chromium"))
    page = browser.new_page(locale="de-DE")
    # Keine externen Ressourcen: auch Bilder sind in der HTML-Ausgabe eingebettet.
    page.route('http://**/*', lambda route: route.abort())
    page.route('https://**/*', lambda route: route.abort())
    page.set_content(html.read_text(), wait_until="load")
    assert page.locator("section.kapitel").count() == len(inhalt["kapitel"])
    for k in inhalt["kapitel"]:
        assert page.locator(f"section#{k['id']} > h2").inner_text().endswith(k["titel"])
    assert page.locator('figure img').count() == len(bilder)
    assert page.locator('figure img').evaluate_all('(imgs)=>imgs.every(i=>i.complete&&i.naturalWidth>0&&i.alt)')
    options = dict(prefer_css_page_size=True, print_background=True, outline=True, tagged=True,
             display_header_footer=True,
             header_template='<div style="font:8px Arial;width:100%;margin:0 18mm;color:#425563">BauPlanIO · Bedienhandbuch für Architekten und Planer</div>',
             footer_template=f'<div style="font:8px Arial;width:100%;margin:0 18mm;color:#425563;display:flex;justify-content:space-between"><span>Stand: {inhalt["stand"]}</span><span>Seite <span class="pageNumber"></span> von <span class="totalPages"></span></span></div>')
    previous = None
    for attempt in range(3):
        page.pdf(path=str(pdf), **options)
        with fitz.open(pdf) as doc:
            # Ausschließlich echte Kapitelüberschriften; Querverweise/TOC nicht als Start zählen.
            bookmarks = {re.sub(r'\s+', '', title): num for level, title, num in doc.get_toc() if re.match(r'^\d{2}\. ',title)}
            starts = {k['id']: bookmarks[re.sub(r'\s+', '', f'{i:02d}. {k["titel"]}')] for i,k in enumerate(inhalt['kapitel'],1)}
        if starts == previous:
            break
        page.evaluate('(starts)=>{for(const e of document.querySelectorAll("[data-kapitel]"))e.textContent=starts[e.dataset.kapitel]}', starts)
        previous = starts
    else:
        raise AssertionError('Kapitelstartseiten nicht stabil; PDF-Layout prüfen.')
    browser.close()
with fitz.open(pdf) as doc:
    pages = len(doc)
    for i, pg in enumerate(doc):
        text = pg.get_text()
        assert f'Seite {i+1} von {pages}' in text, ('Seitenzählung',i+1)
        assert len(text.strip()) > 100, ('Inhaltsleere Seite',i+1)
    assert sum(len(pg.get_images()) for pg in doc) == len(bilder)
    assert all(any(link.get('page') == num-1 for pg in doc for link in pg.get_links()) for num in starts.values())
print(f"PDF mit {len(inhalt['kapitel'])} Kapiteln, {pages} Seiten und {len(bilder)} Abbildungen erstellt: {pdf}")
nachweis = {"stand": inhalt["stand"], "html_sha256": hashlib.sha256(html.read_bytes()).hexdigest(),
            "pdf_sha256": hashlib.sha256(pdf.read_bytes()).hexdigest(),
            "seiten": pages, "abbildungen": len(bilder), "kapitelseiten": starts}
(pdf.parent / "pdf-stand.json").write_text(json.dumps(nachweis, indent=2) + "\n")
