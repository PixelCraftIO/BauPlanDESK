"""Chronik: fachliche Einträge vorn, technische Ordnerausgaben ausgeblendet."""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich
root=Path(__file__).resolve().parent
fixture=json.loads((root/'fixtures/projekt.json').read_text())
def eintrag(n,aktion,details):
 return {'id':n,'zeit':f'2026-10-2{n}T08:00:00Z','benutzer_id':'u','benutzer_name':'Erika Planer','project_id':fixture['id'],'document_id':None,'aktion':aktion,'details':details}
protokoll=[eintrag(1,'dokument_importiert','Lageplan.pdf'),eintrag(2,'ausgabeauftrag_geaendert','Dokumente/Lageplan.pdf: in den Projektordner geschrieben, Versuch 1'),eintrag(3,'ausgabeauftrag_geaendert','Dokumente/Schnitt.pdf: Ausgabe fehlgeschlagen, Versuch 2 – Zugriff verweigert')]
errors=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=browser.new_page(viewport={'width':1440,'height':1000})
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(fixture)+';window.testProtokoll='+json.dumps(protokoll)+';\n'+(root/'tauri-mock.js').read_text())
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1430'))
 page.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click()
 bereich(page,'Chronik')
 akte=page.locator('.projekt-sitzung:visible')
 expect(akte.get_by_role('heading',name='Chronik',exact=True)).to_be_visible()
 chronik=akte.locator('section').filter(has=page.get_by_role('heading',name='Chronik',exact=True));tabelle=chronik.locator('table.ergebnistabelle')
 expect(tabelle.locator('tbody tr')).to_have_count(2)
 expect(tabelle).to_contain_text('Lageplan.pdf')
 # Eine fehlgeschlagene Ausgabe bleibt sichtbar, erfolgreiche sind technisch.
 expect(tabelle).to_contain_text('Ausgabe fehlgeschlagen')
 expect(tabelle).not_to_contain_text('in den Projektordner geschrieben')
 schalter=chronik.get_by_label('Technische Einträge anzeigen',exact=False)
 expect(chronik.get_by_text('(1 ausgeblendet: Ordnerausgaben)',exact=False)).to_be_visible()
 schalter.check();expect(tabelle.locator('tbody tr')).to_have_count(3)
 assert not errors,errors
 browser.close()
print('Chronik: technische Ordnerausgaben ausgeblendet, Fehler sichtbar, Schalter zeigt alle.')
