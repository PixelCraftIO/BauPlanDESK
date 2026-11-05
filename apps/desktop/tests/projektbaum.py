"""Projektbaum mit Statussymbolen (Server auf http://127.0.0.1:1430).

Rot: ungespeichert · Gelb: noch zu erledigen · Grün: erledigt · ohne Symbol:
leer und nicht benötigt. Zweige zeigen das dringendste Symbol, „Nur offene“
blendet Erledigtes aus, leere Bereiche lassen sich als nicht benötigt
kennzeichnen.
"""
from pathlib import Path
import json, os, re
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich, gliederung
root=Path(__file__).resolve().parent
out=Path(os.environ.get('BAUPROJEKT_UI_ARTEFAKTE','/tmp/bauprojekt-projektbaum'));out.mkdir(parents=True,exist_ok=True)
fixture=json.loads((root/'fixtures/projekt.json').read_text());fixture['name']='Wohnhaus Lindenweg 5'
errors=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=browser.new_page(viewport={'width':1440,'height':1000},locale='de-DE')
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(fixture)+';\n'+(root/'tauri-mock.js').read_text())
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1430'))
 page.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click()
 nav=gliederung(page)
 def symbol(name):return nav.locator('.navigationseintrag').filter(has=page.get_by_role('button',name=name,exact=True)).locator('.statussymbol')
 # Gelb mit Grund, Grün, leer ohne Symbol.
 expect(symbol('Flurstücke und Grundbuch')).to_have_class(re.compile(r'\bgelb\b'))
 expect(symbol('Flurstücke und Grundbuch')).to_have_attribute('title','Noch zu erledigen:\nMindestens ein Vorhabenflurstück erfassen')
 expect(symbol('Rechtsgrundlagen')).to_have_class(re.compile(r'\bgruen\b'))
 expect(symbol('Stellplatznachweis')).to_have_count(0)
 # Zweige sammeln das dringendste Symbol.
 expect(nav.get_by_role('button',name='Grundlagen',exact=True).locator('.statussymbol')).to_have_class(re.compile(r'\bgelb\b'))
 expect(nav.get_by_role('button',name='Ermittlung',exact=True).locator('.statussymbol')).to_have_class(re.compile(r'\bgelb\b'))
 page.screenshot(path=str(out/'projektbaum.png'))
 # Statuszeile im geöffneten Bereich nennt den Grund.
 bereich(page,'Flurstücke und Grundbuch')
 expect(page.locator('.projekt-sitzung:visible .bereichsstatus')).to_contain_text('Mindestens ein Vorhabenflurstück erfassen')
 # „Nur offene“ blendet Erledigtes aus und bleibt gemerkt.
 nav.get_by_role('button',name='Nur offene',exact=True).click()
 expect(nav.get_by_role('button',name='Rechtsgrundlagen',exact=True)).to_have_count(0)
 expect(nav.get_by_role('button',name='Flurstücke und Grundbuch',exact=True)).to_have_count(1)
 page.screenshot(path=str(out/'nur-offene.png'))
 nav.get_by_role('button',name='Nur offene',exact=True).click()
 expect(nav.get_by_role('button',name='Rechtsgrundlagen',exact=True)).to_have_count(1)
 # Leeren Bereich als nicht benötigt kennzeichnen und zurücknehmen.
 nav.get_by_role('button',name='Stellplatznachweis: Weitere Aktionen',exact=True).click()
 page.get_by_role('menuitem',name='Als nicht benötigt kennzeichnen',exact=True).click()
 assert page.evaluate("window.calls.some(c=>c.cmd==='bereich_benoetigt_setzen'&&c.args.bereich==='stellplaetze'&&c.args.benoetigt===false)")
 expect(nav.locator('.navigationseintrag.nichtbenoetigt')).to_have_count(1)
 nav.get_by_role('button',name='Stellplatznachweis: Weitere Aktionen',exact=True).click()
 page.get_by_role('menuitem',name='Wieder als benötigt führen',exact=True).click()
 expect(nav.locator('.navigationseintrag.nichtbenoetigt')).to_have_count(0)
 # Pflichtbereich mit Daten bietet die Kennzeichnung nicht an.
 expect(nav.get_by_role('button',name='Rechtsgrundlagen: Weitere Aktionen',exact=True)).to_have_count(0)
 browser.close()
assert not errors,errors
print('Projektbaum: Symbole gelb/grün/ohne, Zweigsymbole, Statuszeile, Nur offene und nicht benötigt geprüft.')
