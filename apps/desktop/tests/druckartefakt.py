"""Binäre Druckvorschau: dasselbe Artefakt für Anzeige, Export und DMS, plus Abbruch."""
from pathlib import Path
import base64,json,os
from playwright.sync_api import sync_playwright,expect
from navigation_hilfe import bereich
root=Path(__file__).resolve().parent
art=Path(os.environ.get('BAUPROJEKT_DRUCK_ARTEFAKTE','/tmp/bauprojekt-druck'))
wf=json.loads((art/'workflow.json').read_text())
wf['pdf']=base64.b64encode((art/'ui-einzel.pdf').read_bytes()).decode()
custom=r'''
const base=window.__TAURI_INTERNALS__.invoke, clone=x=>JSON.parse(JSON.stringify(x));
window.abbruchTest=false;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 if(cmd==='druckkatalog')return clone(window.workflow.katalog);
 if(cmd==='plugin:dialog|save')return '/tmp/Bauantrag.pdf';
 if(!cmd.startsWith('druckartefakt_'))return base(cmd,args);
 window.calls.push({cmd,args:clone(args)});
 if(cmd==='druckartefakt_erstellen'){
   if(window.abbruchTest)return new Promise((resolve,reject)=>{window.offenerDruck=reject});
   return {artefakt:'22222222-2222-4222-8222-222222222222',seiten:1,positionen:[]};
 }
 if(cmd==='druckartefakt_lesen')return Uint8Array.from(atob(window.workflow.pdf),c=>c.charCodeAt(0)).buffer;
 if(cmd==='druckartefakt_ablegen')return {file_name:'Druckkopie.pdf'};
 if(cmd==='druckartefakt_abbrechen'){window.offenerDruck?.(Error('Druckauftrag abgebrochen'));window.offenerDruck=null;}
};
'''
with sync_playwright() as p:
 b=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=b.new_page(viewport={'width':1440,'height':1000});errors=[];page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(wf['projekt'])+';window.workflow='+json.dumps(wf)+';'+(root/'tauri-mock.js').read_text()+custom)
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1430'))
 page.locator('.projekttabelle').get_by_role('button',name=wf['projekt']['name'],exact=True).click()
 bereich(page,'Drucken / PDF-Ausgabe')
 page.get_by_role('button',name='Alle verfügbaren auswählen',exact=True).click()
 page.get_by_role('button',name='Druckausgabe erstellen',exact=False).click()
 expect(page.get_by_role('heading',name='Druckvorschau · 1 Seiten',exact=True)).to_be_visible()
 page.wait_for_function("document.querySelector('canvas')?.width>0")
 page.get_by_role('button',name='PDF speichern …',exact=True).click()
 expect(page.get_by_role('status').filter(has_text='PDF gespeichert.')).to_be_visible()
 page.get_by_role('button',name='Als Projektunterlage ablegen',exact=True).click()
 expect(page.get_by_role('status').filter(has_text='Im Projekt abgelegt: Druckkopie.pdf.')).to_be_visible()
 calls=page.evaluate("window.calls.filter(c=>['druckartefakt_lesen','druckartefakt_exportieren','druckartefakt_ablegen'].includes(c.cmd))")
 assert len(calls)==3 and len({c['args']['artefakt'] for c in calls})==1
 assert all('pdfBase64' not in c['args'] for c in calls)
 page.evaluate('window.abbruchTest=true')
 page.get_by_role('button',name='Druckausgabe erstellen',exact=False).click()
 page.wait_for_function('!!window.offenerDruck')
 page.get_by_role('button',name='Druckauftrag abbrechen',exact=True).click()
 expect(page.get_by_role('button',name='Druckausgabe erstellen',exact=False)).to_be_enabled()
 assert page.evaluate("window.calls.some(c=>c.cmd==='druckartefakt_entfernen')")
 assert not page.get_by_role('heading',name='Druckvorschau · 1 Seiten',exact=True).count()
 assert not errors,errors
 print('Binäre Vorschau, identische Export-/DMS-Quelle, Freigabe und kooperativer Abbruch geprüft.')
 b.close()
