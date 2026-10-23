"""Acht Fachmasken, Entwurfsrettung, native Adapter und echte Nachweis-PDFs."""
from pathlib import Path
import json,os,base64,re
from playwright.sync_api import sync_playwright,expect
from navigation_hilfe import bereich, register, gliederung
root=Path(__file__).resolve().parent
art=Path(os.environ.get('BAUPROJEKT_FACH_ARTEFAKTE','/tmp/bauprojekt-fach'))
wf=json.loads((art/'workflow.json').read_text());wf['pdf']=base64.b64encode((art/'nachweise.pdf').read_bytes()).decode();wf['einzel']=base64.b64encode((art/'einzel.pdf').read_bytes()).decode()
out=Path(os.environ.get('BAUPROJEKT_FACH_UI_ARTEFAKTE','/tmp/bauprojekt-fach-ui'));out.mkdir(parents=True,exist_ok=True)
custom=r'''
{
 const original=window.__TAURI_INTERNALS__.invoke,clone=v=>JSON.parse(JSON.stringify(v));
 window.testUser=clone(window.fach.benutzer);window.fachFail=false;
 window.fachDaten=Object.fromEntries(Object.entries(window.fach.staende).map(([k,v])=>[k,clone(v.fachstand)]));
 window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 cmd=({fachobjektseite:'fachobjekte',fachverlaufsseite:'fachverlauf',druckartefakt_erstellen:'druckausgabe_erstellen'})[cmd]??cmd;
  const list=['berechnungsvorlagen','fachberechnung_pruefen','fachobjekte','fachobjekt_speichern','grundstueckskontext','nachweise','nachweis_erstellen','dokumente_auflisten','druckkatalog','druckausgabe_erstellen'];
  if(!list.includes(cmd))return original(cmd,args);
  if(['fachobjekte','fachobjekt_speichern'].includes(cmd)&&!window.fach.staende[args.art]){
   if(cmd==='fachobjekte'&&['gebaeude','raumbuch'].includes(args.art))return [];
   return original(cmd,args);
  }
  window.calls.push({cmd,args:clone(args)});
  if(cmd==='berechnungsvorlagen')return clone(window.fach.vorlagen);
  if(cmd==='grundstueckskontext')return clone(window.fach.grundstuecke);
  if(cmd==='dokumente_auflisten')return clone(window.fach.dateien);
  if(cmd==='nachweise')return Object.values(window.fach.staende).map(s=>({stand:clone(s.nachweis),aktuell:true,freigabe:null}));
  if(cmd==='fachobjekte')return [clone(window.fachDaten[args.art])];
  if(cmd==='fachobjekt_speichern'){
   if(window.fachFail)throw {art:'konflikt',meldung:'Berechnungsstand inzwischen geändert. Ihr Entwurf bleibt erhalten.'};
   const alt=window.fachDaten[args.art],p=clone(args.daten),v=clone(p);v.regelkontext.anwendbarkeit_bestaetigt=alt.daten.regelkontext.anwendbarkeit_bestaetigt;
   if(JSON.stringify(v)!==JSON.stringify(alt.daten))p.regelkontext.anwendbarkeit_bestaetigt=false;
   window.fachDaten[args.art]={...alt,revision:args.basis+1,daten:p};return clone(window.fachDaten[args.art]);
  }
  if(cmd==='fachberechnung_pruefen'){
   const s=window.fach.staende[args.plan.modul];
   return {plan:clone(args.plan),ergebnis:clone(args.plan.modul==='wohnflaechen'&&args.plan.eingaben.positionen[0].aufmass_m2===120?window.fach.ergebnis120:s.ergebnis)};
  }
  if(cmd==='nachweis_erstellen')return clone(window.fach.staende[args.eingabe.plan.modul].nachweis);
  if(cmd==='druckkatalog')return clone(window.fach.katalog);
  if(cmd==='druckausgabe_erstellen')return {pdf_base64:window.fach.einzel,seiten:window.fach.einzel_seiten,positionen:[]};
 };
}
'''
errors=[]
with sync_playwright() as p:
 b=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=b.new_page(viewport={'width':1680,'height':1100},locale='de-DE');page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(wf['projekt'])+';window.fach='+json.dumps(wf)+';'+(root/'tauri-mock.js').read_text()+custom)
 page.route('**/fach-pdf-test.mjs',lambda route:route.fulfill(path=str(root.parent/'node_modules/pdfjs-dist/build/pdf.mjs'),content_type='application/javascript'))
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1446'))
 page.locator('.projekttabelle').get_by_role('button',name=wf['projekt']['name'],exact=True).click()
 names={'ueberbaubarkeit':'Überbaubare Grundstücksfläche','stellplaetze':'Stellplatznachweis','wohnflaechen':'Wohnflächenberechnung','din277':'DIN-277-Flächen / Rauminhalte','vollgeschosse':'Vollgeschossprüfung','hoehen':'Gebäudehöhen / Dachneigung','freiflaechen':'Versiegelung / Freiflächen','entwaesserung':'Regenwasser / Rückhaltung'}
 def offen(art,neu=False):
  bereich(page,names[art])
  titel=next(v['titel'] for v in wf['vorlagen'] if v['plan']['modul']==art)
  section=page.locator('section').filter(has=page.get_by_role('heading',name=titel,exact=True)).first
  if neu:section.get_by_role('button',name='Öffnen',exact=True).click()
  return section
 for art in names:
  s=offen(art,True)
  s.get_by_role('button',name='Berechnung prüfen',exact=True).click()
  expect(s.get_by_role('region',name='Berechnungsergebnis')).to_be_visible()
  expect(s.get_by_role('region',name='Berechnungsergebnis')).to_contain_text('Vollständig ermittelt')
  s.get_by_role('navigation',name='Berechnungsbereiche').get_by_role('button',name='Prüfbericht',exact=True).click()
  expect(s.get_by_role('heading',name=wf['staende'][art]['ergebnis']['tabellen'][0]['titel'],exact=True)).to_be_visible()
  if art in ['wohnflaechen','stellplaetze']:page.screenshot(path=str(out/f'{art}.png'),full_page=True)
 # Wiederholtes Archivieren verwendet dieselbe Vorgangskennung; keine doppelten Nachweise.
 s=offen('stellplaetze')
 for _ in range(2):
  s.get_by_role('button',name='Nachweis archivieren und PDF ausgeben …',exact=True).click()
  dialog=page.get_by_role('dialog',name='Druckausgabe',exact=True)
  expect(dialog).to_be_visible()
  if _==0:
   dialog.get_by_role('button',name='Druckausgabe erstellen',exact=False).click()
   expect(dialog.get_by_role('heading',name=f'Druckvorschau · {wf["einzel_seiten"]} Seiten')).to_be_visible(timeout=30000)
   page.wait_for_function("document.querySelector('dialog canvas,[role=dialog] canvas')?.width>0")
   page.screenshot(path=str(out/'stellplaetze-pdf.png'))
  dialog.get_by_role('button',name='Schließen',exact=True).click()
 ids=page.evaluate("window.calls.filter(c=>c.cmd==='nachweis_erstellen').map(c=>c.args.id)");assert len(ids)==2 and ids[0]==ids[1]
 # Ungespeicherte Eingabe bleibt bei Bereichswechsel und Speicherkonflikt bestehen.
 s=offen('wohnflaechen');s.get_by_role('navigation',name='Berechnungsbereiche').get_by_role('button',name='Eingaben',exact=True).click()
 row=s.get_by_role('heading',name='Nutzungen / Teilflächen',exact=True).locator('../..').locator('details').first
 row.locator('summary').click();row.get_by_label('Direkte Aufmaßfläche (m²)',exact=True).fill('120')
 offen('hoehen');s=offen('wohnflaechen');expect(row.get_by_label('Direkte Aufmaßfläche (m²)',exact=True)).to_have_value('120')
 page.evaluate('window.fachFail=true');s.get_by_role('button',name='Speichern',exact=True).click();expect(s.get_by_role('alert')).to_contain_text('inzwischen geändert');expect(row.get_by_label('Direkte Aufmaßfläche (m²)',exact=True)).to_have_value('120')
 page.evaluate('window.fachFail=false');s.get_by_role('button',name='Speichern',exact=True).click();s.get_by_role('button',name='Berechnung prüfen',exact=True).click();expect(s.get_by_role('region',name='Berechnungsergebnis')).to_contain_text('Offene Punkte')
 texts=page.evaluate('''async()=>{const pdfjs=await import('/fach-pdf-test.mjs');pdfjs.GlobalWorkerOptions.workerSrc=[...performance.getEntriesByType('resource')].find(e=>/pdf.worker.*mjs/.test(e.name)).name;const d=await pdfjs.getDocument({data:Uint8Array.from(atob(window.fach.pdf),c=>c.charCodeAt(0)),isEvalSupported:false}).promise;const result=[];for(let n=1;n<=d.numPages;n++){const p=await d.getPage(n);result.push((await p.getTextContent()).items.map(i=>i.str??'').join(' '));}await d.destroy();return result;}''')
 alltext=' '.join(texts)
 for word in ['Wohnflächenberechnung','Stellplatznachweis','Vollgeschossprüfung','Regenwasser','Angeboten','Grünanteil','Wohnfläche gesamt','Brutto-Grundfläche','Baulinien, Baugrenzen und Vorbauten']:assert word in alltext,word
 assert not re.search(r'\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b',alltext)
 assert 'SHA-256' not in alltext
 for i,t in enumerate(texts):assert f'Seite {i+1} von {len(texts)}' in t
 assert not errors,errors
 b.close()
print('Fachberechnungen: acht Projektmasken, Ergebnisse, idempotentes Archivieren, Konflikterhalt, Quellen und echte tabellarische PDFs geprüft.')
