"""Compiled UI/test IPC. Real DMS, ZIP, comparisons and persistence covered in Rust."""
from pathlib import Path
import json,os
from playwright.sync_api import sync_playwright,expect
from navigation_hilfe import bereich, register, gliederung
root=Path(__file__).resolve().parent
fixtures=Path(os.environ.get('BAUPROJEKT_EINREICHUNG_ARTEFAKTE','/workspace/review-main/einreichungen'))
wf=json.loads((fixtures/'workflow.json').read_text())
out=root.parents[2]/'docs/dateiverwaltung-einreichungen';out.mkdir(exist_ok=True)
custom='''
const original=window.__TAURI_INTERNALS__.invoke;
const wf=window.workflow;
window.serverDocs=[wf.plan4,wf.excel,wf.foto];window.copies=[];window.changed=false;window.failCommit=true;window.cancelSave=true;window.failExport=false;
window.submissionHistory=[{...structuredClone(wf.chronik[0]),stand:{...structuredClone(wf.chronik[0].stand),id:'46b9312a-0997-4483-91be-08d3b3f99053',titel:'Korrektur der Planunterlagen',dokumente:[wf.plan4]},einreichung:null,version:1},...wf.chronik];
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 const handled=['dokumente_auflisten','arbeitskopien','dokument_extern_oeffnen','arbeitskopie_oeffnen','arbeitskopie_uebernehmen','einreichungen','einreichungs_vergleich','einreichung_exportieren','plugin:dialog|save','plugin:dialog|message'];
 if(!handled.includes(cmd))return original(cmd,args);
 window.calls.push({cmd,args:JSON.parse(JSON.stringify(args))});
 switch(cmd){
 case 'dokumente_auflisten':return args.projektId===wf.projekt.id?window.serverDocs:[];
 case 'arbeitskopien':return window.copies.map(c=>({...c,geaendert:window.changed}));
 case 'dokument_extern_oeffnen':{const doc=window.serverDocs.find(d=>d.document_id===args.dokumentId);const c={stand:{id:'07d8bc45-7bd4-4234-bbe7-2a8f477c21dd',dokument:doc},pfad:'C:\\\\BauProjektIO\\\\Arbeitskopien\\\\Berechnung.xlsx',geaendert:false};window.copies.push(c);return c;}
 case 'arbeitskopie_oeffnen':return;
 case 'arbeitskopie_uebernehmen':if(window.failCommit)throw {art:'konflikt',meldung:'Arbeitskopie bleibt erhalten: inzwischen neuere Version'};window.changed=false;{const next={...window.copies[0].stand.dokument,version:2};window.copies[0].stand.dokument=next;window.serverDocs=window.serverDocs.map(d=>d.document_id===next.document_id?next:d);return next;}
 case 'einreichungen':return structuredClone(window.submissionHistory);
 case 'einreichungs_vergleich':return wf.vergleich.map(v=>{
   let row=v.aktuell.document_id===wf.excel.document_id?{...v,aktuell:window.serverDocs.find(d=>d.document_id===wf.excel.document_id),status:window.copies.length?'gleicher_inhalt':v.status}:v;
   const newlyConfirmed=window.submissionHistory.find(e=>e.stand.id==='46b9312a-0997-4483-91be-08d3b3f99053' && e.einreichung && e.stand.empfaenger===args.empfaenger);
   const doc=newlyConfirmed?.stand.dokumente.find(d=>d.document_id===row.aktuell.document_id);
   if(doc)row={...row,zuletzt_eingereicht:doc,paket_id:newlyConfirmed.stand.id,eingereicht_am:newlyConfirmed.einreichung.datum,status:'bereits_eingereicht'};
   return row;
 });
 case 'einreichung_exportieren':if(window.failExport)throw {art:'datei',meldung:'ZIP konnte nicht geschrieben werden; Paket bleibt gespeichert'};return;
 case 'plugin:dialog|save':return window.cancelSave?null:'C:\\\\BauProjektIO\\\\Bauantrag.zip';
 case 'plugin:dialog|message':return args.buttons?.OkCancelCustom?.[0]??'Yes';
 }
};
'''
errors=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=browser.new_page(viewport={'width':1600,'height':1100})
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(wf['projekt'])+';window.workflow='+json.dumps(wf)+';'+(root/'tauri-mock.js').read_text()+custom)
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1431'))
 page.locator('.projekttabelle').get_by_role('button',name=wf['projekt']['name'],exact=True).click()
 def view(n):bereich(page,n)
 view('Projektdateien')
 page.get_by_label('Dateien suchen',exact=True).fill('.xlsx')
 expect(page.locator('.ergebnistabelle:visible tbody tr')).to_have_count(1)
 page.get_by_role('button',name='Öffnen',exact=True).click()
 expect(page.get_by_text('Unverändert',exact=True)).to_be_visible()
 opened=page.evaluate('window.calls.find(c=>c.cmd==="dokument_extern_oeffnen").args')
 assert opened=={'dokumentId':wf['excel']['document_id'],'version':1}
 page.evaluate('window.changed=true')
 page.get_by_role('button',name='Änderungen prüfen',exact=True).click()
 expect(page.get_by_role('cell',name='Geändert',exact=True)).to_be_visible()
 page.get_by_role('button',name='Als neue Version übernehmen',exact=True).click()
 expect(page.get_by_role('alert').filter(has_text='Arbeitskopie bleibt erhalten: inzwischen neuere Version')).to_be_visible()
 expect(page.get_by_role('cell',name='Geändert',exact=True)).to_be_visible()
 page.get_by_role('alert').get_by_role('button',name='Schließen',exact=True).click()
 page.evaluate('window.failCommit=false')
 page.get_by_role('button',name='Als neue Version übernehmen',exact=True).click()
 expect(page.get_by_text('Berechnung.xlsx als Version 2 übernommen.',exact=True)).to_be_visible()
 page.get_by_label('Dateien suchen',exact=True).fill('')
 page.screenshot(path=str(out/'01-dateiverwaltung.png'))
 view('Einreichungen')
 expect(page.get_by_role('heading',name='Frühere Einreichungspakete',exact=True)).to_be_visible()
 expect(page.get_by_role('button',name='Paket festhalten und ZIP exportieren …',exact=True)).to_have_count(0)
 expect(page.get_by_text('Neues Einreichungspaket',exact=True)).to_have_count(0)
 legacy='Korrektur der Planunterlagen · Bauamt · Ohne bestätigte Einreichung · 1 Unterlagen'
 expect(page.get_by_text(legacy,exact=True)).to_be_visible()
 page.get_by_text(legacy,exact=True).click()
 page.evaluate('window.cancelSave=false;window.failExport=true')
 page.get_by_role('button',name='Dieses ZIP erneut exportieren …',exact=True).first.click()
 expect(page.get_by_role('alert').filter(has_text='ZIP konnte nicht geschrieben werden; Paket bleibt gespeichert')).to_be_visible()
 page.get_by_role('alert').get_by_role('button',name='Schließen',exact=True).click()
 page.evaluate('window.failExport=false')
 page.get_by_role('button',name='Dieses ZIP erneut exportieren …',exact=True).first.click()
 expect(page.get_by_text('Unverändertes Einreichungspaket exportiert.',exact=True)).to_be_visible()
 exported=page.evaluate('window.calls.filter(c=>c.cmd==="einreichung_exportieren").map(c=>c.args.id)')
 assert exported[-1]=='46b9312a-0997-4483-91be-08d3b3f99053',exported
 # Pakete werden nicht mehr bestätigt; die externe Einreichung gehört in die Verfahrensakte.
 expect(page.get_by_role('button',name='Einreichungsdatum erfassen …',exact=True)).to_have_count(0)
 expect(page.get_by_text('Externe Einreichung nachtragen',exact=False).first).to_be_visible()
 assert not page.evaluate('window.calls.some(c=>c.cmd==="einreichung_bestaetigen")')
 page.evaluate('document.querySelector(".aktenbereich").scrollTop=0')
 page.screenshot(path=str(out/'02-einreichungschronik.png'))
 page.get_by_role('button',name='Zur Abgabe in der Verfahrensakte …',exact=True).click()
 expect(page.locator('.projekt-sitzung:visible').get_by_role('heading',name='Verfahrensakte',exact=True)).to_be_visible()
 page.set_viewport_size({'width':980,'height':900})
 assert page.evaluate('document.documentElement.scrollWidth <= innerWidth'), 'horizontal page overflow'
 assert not errors,errors
 browser.close()
(out/'ui-pruefung.json').write_text(json.dumps({'checks':['Excel-Datei gezielt suchen','Öffnen mit konkreter Dokumentversion','Externe Arbeitskopie und Änderungsprüfung','Konflikt erhält Arbeitskopie','Übernahme als neue Version','Keine neuen Einreichungspakete mehr','Frühere Pakete bleiben sichtbar','Exportfehler erhält gespeichertes Paket','ZIP erneut exportieren','Keine Paketbestätigung mehr, Hinweis auf externe Einreichung','Weg zur Abgabe in der Verfahrensakte','Schmale Ansicht ohne Seitenüberlauf'],'page_errors':errors},ensure_ascii=False,indent=2)+'\n')
print('Dateiverwaltung und Einreichungen: 12 UI-Prüfungen bestanden')
