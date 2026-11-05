"""Früheres Beispielprojekt: Menüpunkt entfallen, Rückweg aus dem Beispielprofil bleibt.
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
 # Der Menüpunkt ist entfallen: kein Dialog, kein Öffnen.
 page=start(browser,{'aktiv':False})
 expect(page.get_by_text('Sie arbeiten im')).to_have_count(0)
 page.evaluate('window.emitNative("menu","hilfe.beispielprojekt")')
 expect(page.get_by_role('dialog',name='Beispielprojekt')).to_have_count(0)
 assert aufrufe(page,'beispielprojekt_oeffnen')==[]
 page.close()
 # Wer beim Update noch im Beispielprofil arbeitet, behält den Rückweg.
 page=start(browser,{'aktiv':True})
 hinweis=page.get_by_role('status').filter(has_text='Sie arbeiten im Beispielprojekt')
 expect(hinweis).to_be_visible()
 page.screenshot(path=str(out/'beispielprojekt-aktiv.png'))
 hinweis.get_by_role('button',name='Zu meinen Projekten zurückkehren').click()
 page.wait_for_function('window.calls.some(x=>x.cmd==="beispielprojekt_verlassen")')
 page.close()
 browser.close()
assert not errors,errors
print('Beispielprojekt-UI: ok')
