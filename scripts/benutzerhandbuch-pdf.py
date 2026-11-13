"""Setzt das bebilderte Benutzerhandbuch (docs/handbuch/benutzerhandbuch/*.md)
zu einer druckfähigen PDF zusammen.

Voraussetzungen: pandoc und Playwright mit Chromium.
Aufruf: python3 scripts/benutzerhandbuch-pdf.py
"""
from pathlib import Path
import html as htmllib
import os
import re
import subprocess
import tempfile

from playwright.sync_api import sync_playwright

root = Path(__file__).resolve().parents[1]
quelle = root / "docs/handbuch/benutzerhandbuch"
pdf = quelle / "Benutzerhandbuch.pdf"
kapitel = sorted(quelle.glob("[0-9][0-9]-*.md"))
nummer = {k.name: k.name[:2] for k in kapitel}

readme = (quelle / "README.md").read_text()
stand = re.search(r"\*\*Stand:\*\* ([0-9.]+)", readme).group(1)


def verweise(md: str) -> str:
    """Links zwischen den Kapiteldateien auf Sprungmarken im Gesamtdokument umbiegen."""
    def ersetze(m):
        datei, anker = m.group(1), m.group(2)
        if datei == "README.md":
            return "](#inhalt)"
        if datei not in nummer:
            return m.group(0)
        return f"](#k{nummer[datei]}-{anker})" if anker else f"](#k{nummer[datei]})"
    return re.sub(r"\]\(([\w.-]+\.md)(?:#([^)]+))?\)", ersetze, md)


def umwandeln(md: str) -> str:
    return subprocess.run(["pandoc", "-f", "gfm", "-t", "html"], input=md,
                          capture_output=True, text=True, check=True).stdout


abschnitte = []
for k in kapitel:
    n = nummer[k.name]
    zeilen = k.read_text().splitlines()
    # Blätterzeile unter der Überschrift entfällt im Druck
    zeilen = [z for z in zeilen if not z.startswith("[←") and not z.startswith("[Inhalt")]
    teil = umwandeln(verweise("\n".join(zeilen)))
    teil = re.sub(r'id="(?!k\d\d)([^"]+)"', rf'id="k{n}-\1"', teil)
    teil = re.sub(r'href="#(?!k\d\d|inhalt)([^"]+)"', rf'href="#k{n}-\1"', teil)
    teil = teil.replace('src="bilder/', f'src="{(quelle / "bilder").as_uri()}/')
    abschnitte.append(f'<section class="kapitel" id="k{n}">{teil}</section>')

# Titel und Inhaltsverzeichnis aus der README
einleitung = readme.split("## Inhalt")[0].split("\n", 1)[1]
einleitung_html = umwandeln(verweise(einleitung))
inhalt = "".join(
    f'<li><a href="#k{nummer[k.name]}">{htmllib.escape(re.sub(r"^# ", "", k.read_text().splitlines()[0]))}</a></li>'
    for k in kapitel)

dokument = f"""<!doctype html><html lang="de"><head><meta charset="utf-8">
<title>BauPlanDESK – Benutzerhandbuch</title>
<style>
@page {{ size: A4; margin: 20mm 18mm 18mm 18mm; }}
body {{ font: 10pt/1.45 "DejaVu Sans", Arial, sans-serif; color: #1b2a35; }}
h1 {{ font-size: 20pt; color: #0b3d5c; border-bottom: 2px solid #0b3d5c; padding-bottom: 4px; }}
h2 {{ font-size: 14pt; color: #0b3d5c; margin-top: 18px; break-after: avoid; }}
h3 {{ font-size: 11.5pt; color: #0b3d5c; break-after: avoid; }}
a {{ color: #0b5c8a; text-decoration: none; }}
img {{ max-width: 100%; max-height: 210mm; border: 1px solid #b8c4cc; display: block; margin: 6px auto; break-inside: avoid; }}
p:has(> img) {{ break-inside: avoid; }}
table {{ border-collapse: collapse; width: 100%; margin: 8px 0; font-size: 9pt; }}
th, td {{ border: 1px solid #b8c4cc; padding: 3px 6px; vertical-align: top; text-align: left; }}
th {{ background: #e6edf2; }}
tr {{ break-inside: avoid; }}
code {{ font-size: 9pt; background: #eef2f5; padding: 0 2px; }}
blockquote {{ border-left: 3px solid #0b5c8a; margin: 8px 0; padding: 2px 10px; background: #f3f7fa; }}
ul.task-list {{ list-style: none; padding-left: 4px; }}
section.kapitel {{ break-before: page; }}
.titel {{ height: 240mm; display: flex; flex-direction: column; justify-content: center; }}
.titel h1 {{ font-size: 30pt; border: 0; margin: 0; }}
.titel .unter {{ font-size: 14pt; color: #425563; margin-top: 8px; }}
.inhalt {{ break-before: page; }}
.inhalt ul {{ font-size: 12pt; line-height: 2; }}
</style></head><body>
<div class="titel"><h1>BauPlanDESK</h1><div class="unter">Benutzerhandbuch · Stand {stand}</div></div>
<section class="inhalt" id="inhalt"><h1>Inhalt</h1><ul style="list-style:none;padding-left:0">{inhalt}</ul>{einleitung_html}</section>
{''.join(abschnitte)}
</body></html>"""

# Jeder interne Link muss ein Ziel haben
ziele = set(re.findall(r'id="([^"]+)"', dokument))
fehlend = sorted({h for h in re.findall(r'href="#([^"]+)"', dokument)} - ziele)
assert not fehlend, f"Links ohne Ziel: {fehlend}"

with tempfile.TemporaryDirectory() as tmp:
    datei = Path(tmp) / "handbuch.html"
    datei.write_text(dokument)
    with sync_playwright() as p:
        pfad = os.environ.get("BAUPROJEKT_CHROMIUM")
        browser = p.chromium.launch(**({"executable_path": pfad} if pfad else {}))
        page = browser.new_page(locale="de-DE")
        page.goto(datei.as_uri(), wait_until="load")
        kaputt = page.evaluate("[...document.images].filter(i => !i.naturalWidth).map(i => i.src)")
        assert not kaputt, f"Bilder fehlen: {kaputt}"
        page.pdf(path=str(pdf), prefer_css_page_size=True, print_background=True,
                 outline=True, tagged=True, display_header_footer=True,
                 header_template='<div style="font:8px Arial;width:100%;margin:0 18mm;color:#425563">BauPlanDESK · Benutzerhandbuch</div>',
                 footer_template=f'<div style="font:8px Arial;width:100%;margin:0 18mm;color:#425563;display:flex;justify-content:space-between"><span>Stand: {stand}</span><span>Seite <span class="pageNumber"></span> von <span class="totalPages"></span></span></div>')
        browser.close()
print(f"PDF mit {len(kapitel)} Kapiteln erstellt: {pdf}")
