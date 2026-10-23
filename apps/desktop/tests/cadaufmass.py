"""DXF-Regression: verspätete Antworten und versionierte Projektmessungen."""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich

root=Path(__file__).resolve().parent
fixture=json.loads((root/'fixtures/projekt.json').read_text(encoding='utf-8'))
mock=r'''
const base=window.__TAURI_INTERNALS__.invoke;
const docs=['A','B'].map((n,i)=>({document_id:`b316da8b-cbfb-4ae6-931b-8765fb6454b${i+1}`,project_id:window.fixture.id,version:1,file_name:n+'.dxf',sha256:'0'.repeat(64),size_bytes:100,created_at:window.fixture.created_at,created_by:'Test',storage_key:n}));
window.cadSaved={};window.cadRequests=[];
const measure=(unit)=>({laenge_m:unit==='meter'?40:.04,flaeche_m2:unit==='meter'?100:.0001,selbstschneidend:false});
window.__TAURI_INTERNALS__.invoke=async(cmd,a={})=>{
 if(cmd==='dokumente_auflisten')return docs;
 if(cmd==='cad_plan'){
   window.cadRequests.push(a.dokumentId);
   const d=docs.find(d=>d.document_id===a.dokumentId);
   if(window.cadDelay&&d===docs[0])await new Promise(r=>setTimeout(r,500));
   return {dokument:d,plan:{einheit:d===docs[0]?'millimeter':'meter',ebenen:[{name:'Kontur',farbe:null,sichtbar:true}],linien:[{ebene:0,farbe:null,punkte:[[0,0],[10,0],[10,10],[0,10]],geschlossen:true}],texte:[],grenzen:{min:[0,0],max:[10,10]},hinweise:[]}};
 }
 if(cmd==='cadaufmass_laden')return window.cadSaved[a.dokument.dokument_id]??null;
 if(cmd==='cadaufmass_speichern')return window.cadSaved[a.aufmass.dokument.dokument_id]={projekt_id:window.fixture.id,modul:'cadaufmass',objekt:crypto.randomUUID(),revision:a.basis+1,schema_version:1,daten:structuredClone(a.aufmass)};
 if(cmd==='cadaufmass_messwerte')return a.aufmass.markierungen.map(()=>measure(a.aufmass.einheit));
 if(cmd==='aufmass')return measure(a.einheit);
 return base(cmd,a);
};
'''
errors=[]
with sync_playwright() as p:
    browser=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM',r'C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe'))
    page=browser.new_page(viewport={'width':1600,'height':1150})
    page.on('pageerror',lambda e:errors.append(str(e)))
    page.add_init_script('window.fixture='+json.dumps(fixture)+';'+(root/'tauri-mock.js').read_text(encoding='utf-8')+mock)
    page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1420'))
    page.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click()
    bereich(page,'Pläne / Aufmaß')
    scope=page.locator('.projekt-sitzung:visible')
    select=scope.get_by_label('Plan',exact=True)
    unit=scope.get_by_label('Einheit',exact=True)
    expect(unit).to_have_value('millimeter')
    ids=select.locator('option').evaluate_all('(o)=>o.map(x=>x.value)')
    select.select_option(ids[1]);expect(unit).to_have_value('meter')
    page.evaluate('window.cadDelay=true')
    select.select_option(ids[0])
    page.wait_for_function('window.cadRequests.length>=3')
    select.select_option(ids[1]);expect(unit).to_have_value('meter')
    page.wait_for_timeout(650)
    expect(unit).to_have_value('meter')
    scope.get_by_role('button',name='Auswählen',exact=True).click()
    canvas=scope.get_by_role('application')
    canvas.click(position={'x':canvas.bounding_box()['width']/2,'y':canvas.bounding_box()['height']/2})
    name=scope.locator('.messung').get_by_label('Bezeichnung',exact=True)
    expect(name).to_have_count(1)
    name.fill('Hallenfläche')
    scope.get_by_role('button',name='Projektaufmaß speichern',exact=True).click()
    expect(scope.get_by_text('Projektaufmaß gespeichert · Revision 1',exact=True)).to_be_visible()
    select.select_option(ids[0]);expect(unit).to_have_value('millimeter')
    select.select_option(ids[1]);expect(name).to_have_value('Hallenfläche')
    expect(scope.get_by_text('Gespeichertes Projektaufmaß · Revision 1',exact=True)).to_be_visible()
    assert not errors, errors
    browser.close()
print('DXF: verspätete Planantworten verworfen; benanntes Projektaufmaß gespeichert und wieder geladen.')
