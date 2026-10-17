"""Projektbezogene Abstandsflächen im Produktionsbuild, echte Core-Berichte."""
from pathlib import Path
import os,json,base64
from playwright.sync_api import sync_playwright,expect
from navigation_hilfe import bereich, register, gliederung
root=Path(__file__).resolve().parent
art=Path(os.environ.get('BAUPROJEKT_ABSTAND_ARTEFAKTE','/tmp/bauprojekt-abstand'))
wf=json.loads((art/'workflow.json').read_text());wf['pdf']=base64.b64encode((art/'nachweis.pdf').read_bytes()).decode()
out=Path(os.environ.get('BAUPROJEKT_ABSTAND_UI_ARTEFAKTE',str(root.parents[2]/'docs/abstandsflaechen/screenshots')));out.mkdir(parents=True,exist_ok=True)
custom=r'''
{
const base=window.__TAURI_INTERNALS__.invoke,clone=v=>JSON.parse(JSON.stringify(v));
window.testUser=clone(window.abstand.benutzer);window.aPlan=clone(window.abstand.fachstand);window.aFail=false;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 if(['fachobjekte','fachobjekt_speichern'].includes(cmd)&&args.art!=='abstandsflaechen')return base(cmd,args);
 if(!['fachobjekte','fachobjekt_speichern','abstandsregeln','abstandsplan_pruefen','nachweis_erstellen','druckkatalog','druckausgabe_erstellen','druckausgabe_ablegen','dokumente_auflisten'].includes(cmd))return base(cmd,args);
 window.calls.push({cmd,args:clone(args)});
 switch(cmd){
 case 'abstandsregeln':return clone(window.abstand.katalog);
 case 'dokumente_auflisten':return [clone(window.abstand.quelle)];
 case 'fachobjekte':return [clone(window.aPlan)];
 case 'fachobjekt_speichern':{
  if(window.aFail)throw {art:'konflikt',meldung:'Abstandsplan inzwischen geändert. Ihr Entwurf bleibt erhalten.'};
  const daten=clone(args.daten);
  if(JSON.stringify(daten)!==JSON.stringify(window.aPlan.daten)){daten.recht.geprueft=false;daten.umfang_geprueft=false;}
  window.aPlan={...window.aPlan,revision:args.basis+1,daten};return clone(window.aPlan);
 }
 case 'abstandsplan_pruefen':return clone(args.plan.waende[0].abschluss_a_m===110?window.abstand.ergebnis_geaendert:window.abstand.ergebnis);
 case 'nachweis_erstellen':return clone(window.abstand.nachweis);
 case 'druckkatalog':return {projekt_version:window.fixture.version,eintraege:[{titel:window.abstand.plan.bezeichnung,quelle:{art:'nachweis',id:window.abstand.nachweis.id},verfuegbar:true,hinweis:null}]};
 case 'druckausgabe_ablegen':return {file_name:'Abstandsflächen.pdf'};
 case 'druckausgabe_erstellen':return {pdf_base64:window.abstand.pdf,seiten:window.abstand.druckseiten,positionen:[]};
 }
};
}
'''
errors=[]
with sync_playwright() as p:
 b=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=b.new_page(viewport={'width':1680,'height':1100},locale='de-DE');page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(wf['projekt'])+';window.abstand='+json.dumps(wf)+';'+(root/'tauri-mock.js').read_text()+custom)
 page.route('**/abstand-pdf-test.mjs',lambda route:route.fulfill(path=str(root.parent/'node_modules/pdfjs-dist/build/pdf.mjs'),content_type='application/javascript'))
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1444'))
 page.locator('.projekttabelle').get_by_role('button',name=wf['projekt']['name'],exact=True).click()
 bereich(page,'Abstandsflächen')
 section=page.locator('section').filter(has=page.get_by_role('heading',name='Abstandsflächen im Projekt',exact=True)).first
 section.get_by_role('button',name='Öffnen',exact=True).click()
 def tab(n):page.get_by_role('navigation',name='Abstandsflächenbereiche').get_by_role('button',name=n,exact=True).click()
 page.get_by_role('button',name='Abstandsflächen berechnen',exact=True).click()
 result=page.get_by_role('region',name='Ergebnis Abstandsflächen')
 expect(result.get_by_role('cell',name='3,2',exact=True)).to_be_visible()
 expect(result.get_by_role('cell',name='32',exact=True)).to_be_visible()
 expect(result.get_by_role('img',name='Abstandsflächenplan mit Grundstücksgrenzen und Konflikten')).to_be_visible()
 page.screenshot(path=str(out/'pruefbericht.png'))
 # Archivieren und der tatsächliche PDF-Bericht mit Vektorplan.
 page.get_by_role('button',name='Nachweis archivieren und PDF ausgeben …',exact=True).click()
 dialog=page.get_by_role('dialog',name='Druckausgabe',exact=True)
 dialog.get_by_role('button',name='Druckausgabe erstellen',exact=False).click()
 expect(dialog.get_by_role('heading',name=f'Druckvorschau · {wf["druckseiten"]} Seiten')).to_be_visible(timeout=30000)
 dialog.get_by_role('region',name='Druckvorschau').locator('select').select_option(str(wf['druckseiten']))
 page.wait_for_function("document.querySelector('dialog canvas,[role=dialog] canvas')?.width>0")
 expect(dialog.locator('.pdf-render-status')).to_have_text('')
 page.screenshot(path=str(out/'vektorplan-pdf.png'))
 texts=page.evaluate('''async()=>{const pdfjs=await import('/abstand-pdf-test.mjs');pdfjs.GlobalWorkerOptions.workerSrc=[...performance.getEntriesByType('resource')].find(e=>/pdf.worker.*mjs/.test(e.name)).name;const d=await pdfjs.getDocument({data:Uint8Array.from(atob(window.abstand.pdf),c=>c.charCodeAt(0)),isEvalSupported:false}).promise;const ts=[];for(let n=1;n<=d.numPages;n++){const p=await d.getPage(n);ts.push((await p.getTextContent()).items.map(i=>i.str??'').join(' '));}await d.destroy();return ts;}''')
 assert 'Vektorplanskizze' in texts[-1] and 'W1' in texts[-1]
 assert 'SHA-256' not in ' '.join(texts) and 'Südwand' in ' '.join(texts)
 assert 'Ergebnisübersicht der Wände' in ' '.join(texts)
 assert f'Seite {len(texts)} von {len(texts)}' in texts[-1]
 dialog.get_by_role('button',name='Als Projektunterlage ablegen',exact=True).click()
 expect(dialog.get_by_role('status').filter(has_text='Im Projekt abgelegt: Abstandsflächen.pdf.')).to_be_visible()
 assert page.evaluate("window.calls.find(c=>c.cmd==='druckausgabe_ablegen').args.pdfBase64===window.abstand.pdf")
 dialog.get_by_role('button',name='Schließen',exact=True).click()
 tab('Wände / Bauteile');page.get_by_role('button',name='Bearbeiten',exact=True).click()
 page.get_by_role('spinbutton',name='Wandabschluss Anfang (m)',exact=True).fill('110')
 page.evaluate('window.aFail=true');section.get_by_role('button',name='Speichern',exact=True).click()
 expect(section.get_by_role('alert')).to_contain_text('inzwischen geändert')
 expect(page.get_by_role('spinbutton',name='Wandabschluss Anfang (m)',exact=True)).to_have_value('110')
 page.evaluate('window.aFail=false');section.get_by_role('button',name='Speichern',exact=True).click()
 page.get_by_role('button',name='Abstandsflächen berechnen',exact=True).click()
 expect(result.get_by_role('cell',name='4',exact=True)).to_be_visible()
 assert not errors,errors
 b.close()
print('Abstandsflächen: Projektentwurf, Konflikterhalt, Wandänderung, echte PDF-Texte und Vektorplanskizze geprüft.')
