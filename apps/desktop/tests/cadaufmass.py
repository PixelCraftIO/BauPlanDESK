"""DXF-Regression im Aufmaß-Editor: Einheit aus dem DXF-Kopf und verspätete Antworten."""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich

root=Path(__file__).resolve().parent
fixture=json.loads((root/'fixtures/projekt.json').read_text(encoding='utf-8'))
mock=r'''
const base=window.__TAURI_INTERNALS__.invoke;
const docs=['A','B'].map((n,i)=>({document_id:`b316da8b-cbfb-4ae6-931b-8765fb6454b${i+1}`,project_id:window.fixture.id,version:1,file_name:n+'.dxf',sha256:'0'.repeat(64),size_bytes:100,created_at:window.fixture.created_at,created_by:'Test',storage_key:n}));
window.cadRequests=[];window.cadDelay=false;
window.__TAURI_INTERNALS__.invoke=async(cmd,a={})=>{
 if(cmd==='dokumente_auflisten')return docs;
 if(cmd==='cad_plan'){
   window.cadRequests.push(a.dokumentId);
   const d=docs.find(d=>d.document_id===a.dokumentId);
   if(window.cadDelay&&d===docs[0])await new Promise(r=>setTimeout(r,500));
   return {dokument:d,plan:{einheit:d===docs[0]?'millimeter':'meter',ebenen:[{name:'Kontur '+(d===docs[0]?'A':'B'),farbe:null,sichtbar:true}],linien:[{ebene:0,farbe:null,punkte:[[0,0],[10,0],[10,10],[0,10]],geschlossen:true}],texte:[],grenzen:{min:[0,0],max:[10,10]},hinweise:[]}};
 }
 return base(cmd,a);
};
'''
A,B='b316da8b-cbfb-4ae6-931b-8765fb6454b1|1','b316da8b-cbfb-4ae6-931b-8765fb6454b2|1'
errors=[]
with sync_playwright() as p:
    browser=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM',r'C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe'))
    page=browser.new_page(viewport={'width':1600,'height':1150})
    page.on('pageerror',lambda e:errors.append(str(e)))
    page.add_init_script('window.fixture='+json.dumps(fixture)+';'+(root/'tauri-mock.js').read_text(encoding='utf-8')+mock)
    page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1420'))
    page.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click()
    bereich(page,'Aufmaß')
    scope=page.locator('.projekt-sitzung:visible')
    scope.get_by_role('button',name='Neu anlegen',exact=True).click()
    quelle=scope.get_by_label('Unterlage',exact=True)
    unit=scope.get_by_label('Zeichnungseinheit',exact=True)
    # Ein neuer Entwurf übernimmt die im DXF-Kopf genannte Einheit.
    quelle.select_option(A);expect(unit).to_have_value('millimeter')
    quelle.select_option(B);expect(unit).to_have_value('meter')
    # Eine verspätete Antwort für das verlassene Blatt überschreibt nichts.
    page.evaluate('window.cadDelay=true')
    quelle.select_option(A)
    page.wait_for_function('window.cadRequests.filter(x=>x.endsWith("b1")).length>=2')
    quelle.select_option(B);expect(unit).to_have_value('meter')
    page.wait_for_timeout(650)
    expect(unit).to_have_value('meter')
    expect(quelle).to_have_value(B)
    expect(scope.get_by_text('Kontur B',exact=False).first).to_be_visible();expect(scope.get_by_text('Kontur A',exact=False)).to_have_count(0)
    expect(scope.get_by_role('application')).to_be_visible()
    assert not errors,errors
    browser.close()
print('DXF-Aufmaß: Einheit aus dem DXF-Kopf, verspätete Antworten beim Blattwechsel ignoriert.')
