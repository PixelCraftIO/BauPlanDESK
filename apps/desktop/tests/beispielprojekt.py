"""Beispielprojekt: Menü „Hilfe → Beispielprojekt öffnen“ und Hinweis im Beispielprofil.
Start a built frontend on http://127.0.0.1:1430 (see README.md).
"""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
root=Path(__file__).resolve().parent
out=Path(os.environ.get('BAUPROJEKT_UI_ARTEFAKTE',str(root.parents[2]/'docs/ui-umsetzung')))
out.mkdir(parents=True,exist_ok=True)
fixture=json.loads((root/'fixtures/projekt.json').read_text())
url=os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1430')
errors=[]
def start(browser,status):
 page=browser.new_page(viewport={'width':1440,'height':900})
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(fixture)+';window.beispielStatus='+json.dumps(status)+';\n'+(root/'tauri-mock.js').read_text())
 page.goto(url)
 page.get_by_role('heading',name='Projektverzeichnis',exact=True).wait_for()
 return page
def aufrufe(page,cmd):
 return page.evaluate('c=>window.calls.filter(x=>x.cmd===c).map(x=>x.args)',cmd)
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 # Erstes Öffnen: eine frische Kopie.
 page=start(browser,{'aktiv':False,'kopie_vorhanden':False})
 expect(page.get_by_text('Sie arbeiten im')).to_have_count(0)
 page.evaluate('window.emitNative("menu","hilfe.beispielprojekt")')
 frage=page.get_by_role('dialog',name='Beispielprojekt')
 expect(frage).to_contain_text('Ihre Projekte bleiben unverändert')
 page.screenshot(path=str(out/'beispielprojekt-oeffnen.png'))
 frage.get_by_role('button',name='Beispielprojekt öffnen',exact=True).click()
 page.wait_for_function('window.calls.some(x=>x.cmd==="beispielprojekt_oeffnen")')
 assert aufrufe(page,'beispielprojekt_oeffnen')==[{'neu':True}]
 page.close()
 # Bearbeitete Kopie vorhanden: fortsetzen oder neu beginnen.
 page=start(browser,{'aktiv':False,'kopie_vorhanden':True})
 page.evaluate('window.emitNative("menu","hilfe.beispielprojekt")')
 frage=page.get_by_role('dialog',name='Beispielprojekt')
 frage.get_by_role('button',name='Bearbeitete Kopie öffnen',exact=True).click()
 page.wait_for_function('window.calls.some(x=>x.cmd==="beispielprojekt_oeffnen")')
 assert aufrufe(page,'beispielprojekt_oeffnen')==[{'neu':False}]
 page.close()
 # Im Beispielprofil: dauerhafter Hinweis mit Rückweg.
 page=start(browser,{'aktiv':True,'kopie_vorhanden':True})
 hinweis=page.get_by_role('status').filter(has_text='Sie arbeiten im Beispielprojekt')
 expect(hinweis).to_be_visible()
 page.screenshot(path=str(out/'beispielprojekt-aktiv.png'))
 hinweis.get_by_role('button',name='Zu meinen Projekten zurückkehren').click()
 page.wait_for_function('window.calls.some(x=>x.cmd==="beispielprojekt_verlassen")')
 page.close()
 browser.close()
assert not errors,errors
print('Beispielprojekt-UI: ok')
