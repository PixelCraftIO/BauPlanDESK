"""Besuchte verdeckte Fachbereiche pausieren Leseabfragen und holen beim Öffnen nach."""
from pathlib import Path
import json,os
from playwright.sync_api import sync_playwright,expect
from navigation_hilfe import bereich
root=Path(__file__).resolve().parent
fixture=json.loads((root/'fixtures/projekt.json').read_text());fixture['name']='Ansichtsabfragen'
custom=r'''
const base=window.__TAURI_INTERNALS__.invoke;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 if(cmd==='berechnungsvorlagen')return [];
 if(cmd==='fachobjekt_speichern'){
   window.calls.push({cmd,args:structuredClone(args)});
   return {projekt_id:args.projektId,modul:args.art,objekt:args.id,revision:args.basis+1,schema_version:1,daten:structuredClone(args.daten)};
 }
 return base(cmd,args);
};
'''
with sync_playwright() as p:
 b=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=b.new_page(viewport={'width':1440,'height':1000});errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(fixture)+';'+(root/'tauri-mock.js').read_text()+custom)
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1430'))
 page.locator('.projekttabelle').get_by_role('button',name='Ansichtsabfragen',exact=True).click()
 bereich(page,'Stellplatznachweis');page.wait_for_function("window.calls.some(c=>c.cmd==='nachweise')")
 bereich(page,'Wohnflächenberechnung');page.wait_for_function("window.calls.filter(c=>c.cmd==='nachweise').length>=2")
 bereich(page,'Aufgaben / Fristen')
 scope=page.locator('.projekt-sitzung:visible > .aktenrumpf > main > section:not([hidden])')
 scope.get_by_role('button',name='Neu anlegen',exact=True).click()
 vor=page.evaluate("window.calls.filter(c=>c.cmd==='nachweise').length")
 scope.get_by_label('Aufgabe',exact=True).fill('Unterlagen prüfen')
 scope.get_by_role('button',name='Speichern',exact=True).click()
 expect(scope.get_by_role('status').filter(has_text='Gespeichert.')).to_be_visible()
 page.evaluate('()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))')
 assert page.evaluate("window.calls.filter(c=>c.cmd==='nachweise').length")==vor,'Verdeckte Fachbereiche haben neu geladen'
 bereich(page,'Wohnflächenberechnung')
 page.wait_for_function("window.calls.filter(c=>c.cmd==='nachweise').length>"+str(vor))
 assert page.evaluate("window.calls.filter(c=>c.cmd==='nachweise').length")==vor+1
 assert not errors,errors
 print('Verdeckte Bereiche pausieren; beim Aktivieren wird der vorgemerkte Stand einmal geladen.')
 b.close()
