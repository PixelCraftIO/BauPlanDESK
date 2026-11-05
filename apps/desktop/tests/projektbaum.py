"""Projektbaum mit Statussymbolen (Server auf http://127.0.0.1:1430).

Statusschema: Rot Problem · Blau nicht gespeichert · Orange veraltet · Gelb in
Arbeit · Grau zu erledigen · Grün geprüft · ohne Symbol leer bzw. nicht
benötigt. Zweige zeigen das dringendste Symbol, „Nur offene“ blendet
Geprüftes aus, „?“ öffnet die Legende, leere Bereiche lassen sich als nicht
benötigt kennzeichnen.
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
 staende=[
  {'bereich':'uebersicht','zustand':'in_arbeit','gruende':['Vorhaben beschreiben'],'kennzeichenbar':False},
  {'bereich':'rechtsgrundlagen','zustand':'geprueft','gruende':[],'kennzeichenbar':False},
  {'bereich':'flurstuecke','zustand':'zu_erledigen','gruende':['Mindestens ein Vorhabenflurstück erfassen'],'kennzeichenbar':False},
  {'bereich':'stellplaetze','zustand':'leer','gruende':[],'kennzeichenbar':True},
  {'bereich':'abstand','zustand':'problem','gruende':['Abstandsflächen Nord: Anforderung nicht eingehalten'],'kennzeichenbar':False},
  {'bereich':'freiflaechen','zustand':'veraltet','gruende':['Freiflächen: Grundlagen oder Eingaben geändert, Nachweis neu erstellen'],'kennzeichenbar':False},
 ]
 page.add_init_script('window.testBereichsstatus='+json.dumps(staende)+';window.fixture='+json.dumps(fixture)+';\n'+(root/'tauri-mock.js').read_text())
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1430'))
 page.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click()
 nav=gliederung(page)
 def symbol(name):return nav.locator('.navigationseintrag').filter(has=page.get_by_role('button',name=name,exact=True)).locator('.statussymbol')
 # Jeder Zustand mit eigenem Symbol und Grund; leer ohne Symbol.
 expect(symbol('Flurstücke und Grundbuch')).to_have_class(re.compile(r'\bgrau\b'))
 expect(symbol('Flurstücke und Grundbuch')).to_have_attribute('title','Zu erledigen:\nMindestens ein Vorhabenflurstück erfassen')
 expect(symbol('Stammdaten')).to_have_class(re.compile(r'\bgelb\b'))
 expect(symbol('Rechtsgrundlagen')).to_have_class(re.compile(r'\bgruen\b'))
 expect(symbol('Abstandsflächen')).to_have_class(re.compile(r'\brot\b'))
 expect(symbol('Abstandsflächen')).to_have_attribute('title','Problem:\nAbstandsflächen Nord: Anforderung nicht eingehalten')
 expect(symbol('Stellplatznachweis')).to_have_count(0)
 # Zweige sammeln das dringendste Symbol (Rot vor Orange vor Gelb vor Grau).
 expect(nav.get_by_role('button',name='Grundlagen',exact=True).locator('.statussymbol')).to_have_class(re.compile(r'\bgelb\b'))
 expect(nav.get_by_role('button',name='Ermittlung',exact=True).locator('.statussymbol')).to_have_class(re.compile(r'\bgelb\b'))
 expect(nav.get_by_role('button',name='Weitere Nachweise',exact=True).locator('.statussymbol')).to_have_class(re.compile(r'\brot\b'))
 # Legende erklärt alle Symbole und schließt mit Escape.
 nav.get_by_role('button',name='Legende der Statussymbole',exact=True).click()
 legende=page.get_by_role('dialog',name='Legende der Statussymbole')
 expect(legende).to_be_visible()
 for wort in ['Problem','Nicht gespeichert','Veraltet','In Arbeit','Zu erledigen','Geprüft','Ohne Zeichen']:
  expect(legende).to_contain_text(wort)
 expect(legende.locator('.statussymbol')).to_have_count(6)
 page.screenshot(path=str(out/'legende.png'))
 page.keyboard.press('Escape')
 expect(legende).to_have_count(0)
 page.screenshot(path=str(out/'projektbaum.png'))
 # Statuszeile im geöffneten Bereich nennt den Grund.
 bereich(page,'Flurstücke und Grundbuch')
 expect(page.locator('.projekt-sitzung:visible .bereichsstatus')).to_contain_text('Mindestens ein Vorhabenflurstück erfassen')
 # „Nur offene“ blendet Geprüftes aus und bleibt gemerkt.
 nav.get_by_role('button',name='Nur offene',exact=True).click()
 expect(nav.get_by_role('button',name='Rechtsgrundlagen',exact=True)).to_have_count(0)
 expect(nav.get_by_role('button',name='Flurstücke und Grundbuch',exact=True)).to_have_count(1)
 page.screenshot(path=str(out/'nur-offene.png'))
 nav.get_by_role('button',name='Nur offene',exact=True).click()
 expect(nav.get_by_role('button',name='Rechtsgrundlagen',exact=True)).to_have_count(1)
 # Das Aktionsmenü schließt bei Klick daneben und mit Escape.
 nav.get_by_role('button',name='Stellplatznachweis: Weitere Aktionen',exact=True).click()
 expect(page.get_by_role('menu',name='Stellplatznachweis: Aktionen')).to_be_visible()
 page.locator('.projekt-sitzung:visible main').click(position={'x':5,'y':5})
 expect(page.get_by_role('menu',name='Stellplatznachweis: Aktionen')).to_have_count(0)
 nav.get_by_role('button',name='Stellplatznachweis: Weitere Aktionen',exact=True).click()
 page.keyboard.press('Tab');page.keyboard.press('Escape')
 expect(page.get_by_role('menu',name='Stellplatznachweis: Aktionen')).to_have_count(0)
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
print('Projektbaum: Statusschema mit sechs Symbolen, Zweigsymbole, Legende, Statuszeile, Nur offene und nicht benötigt geprüft.')
