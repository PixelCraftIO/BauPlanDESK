"""Compiled UI/test IPC. Real DMS, ZIP, comparisons and persistence covered in Rust."""
from pathlib import Path
import json,os
from playwright.sync_api import sync_playwright,expect
root=Path(__file__).resolve().parent
fixtures=Path(os.environ.get('BAUPROJEKT_EINREICHUNG_ARTEFAKTE','/workspace/review-main/einreichungen'))
wf=json.loads((fixtures/'workflow.json').read_text())
out=root.parents[2]/'docs/dateiverwaltung-einreichungen';out.mkdir(exist_ok=True)
custom='''
const original=window.__TAURI_INTERNALS__.invoke;
const wf=window.workflow;
window.serverDocs=[wf.plan4,wf.excel,wf.foto];window.copies=[];window.changed=false;window.failCommit=true;window.cancelSave=true;window.failExport=false;
window.submissionHistory=wf.chronik;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 const handled=['dokumente_auflisten','arbeitskopien','dokument_extern_oeffnen','arbeitskopie_oeffnen','arbeitskopie_uebernehmen','einreichungen','einreichungs_vergleich','einreichung_erstellen','einreichung_exportieren','einreichung_bestaetigen','plugin:dialog|save','plugin:dialog|message'];
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
 case 'einreichung_erstellen':{const e={stand:{id:'46b9312a-0997-4483-91be-08d3b3f99053',projekt_id:args.projektId,titel:args.paket.titel,empfaenger:args.paket.empfaenger,erstellt_am:new Date().toISOString(),erstellt_von:'Anna Weber',dokumente:args.paket.dokumente.map(x=>window.serverDocs.find(d=>d.document_id===x.dokument_id))},zip_key:'paket.zip',zip_hash:'a'.repeat(64),size:100,version:1,einreichung:null};window.submissionHistory.unshift(e);return structuredClone(e);}
 case 'einreichung_exportieren':if(window.failExport)throw {art:'datei',meldung:'ZIP konnte nicht geschrieben werden; Paket bleibt gespeichert'};return;
 case 'einreichung_bestaetigen':{const e=window.submissionHistory.find(e=>e.stand.id===args.id);e.einreichung={datum:args.datum,notiz:args.notiz,bestaetigt_am:new Date().toISOString(),bestaetigt_von:'Anna Weber'};e.version=2;return structuredClone(e);}
 case 'plugin:dialog|save':return window.cancelSave?null:'C:\\\\BauProjektIO\\\\Bauantrag.zip';
 case 'plugin:dialog|message':return args.buttons?.OkCancelCustom?.[0]??'Yes';
 }
};
'''
errors=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True,executable_path='/usr/bin/chromium')
 page=browser.new_page(viewport={'width':1600,'height':1100})
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(wf['projekt'])+';window.workflow='+json.dumps(wf)+';'+(root/'tauri-mock.js').read_text()+custom)
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1431'))
 page.locator('.projekttabelle').get_by_role('button',name=wf['projekt']['name'],exact=True).click()
 def view(n):page.locator('.aktennavigation').get_by_role('button',name=n,exact=True).click()
 view('Dokumente')
 page.get_by_label('Dateien suchen',exact=True).fill('.xlsx')
 expect(page.locator('.ergebnistabelle:visible tbody tr')).to_have_count(1)
 page.get_by_role('button',name='Im zugeordneten Programm öffnen',exact=True).click()
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
 expect(page.get_by_text('Seit Einreichung geändert',exact=True)).to_be_visible()
 expect(page.get_by_text('Diese Version bereits eingereicht',exact=True)).to_have_count(1)
 expect(page.get_by_text('Neue Version, gleicher Dateiinhalt',exact=True)).to_be_visible()
 page.get_by_role('textbox',name='Bezeichnung',exact=True).fill('Korrektur der Planunterlagen')
 page.get_by_role('button',name='Neue und geänderte auswählen',exact=True).click()
 expect(page.get_by_role('checkbox',name='Plan.dwg auswählen',exact=True)).to_be_checked()
 expect(page.get_by_role('checkbox',name='Berechnung.xlsx auswählen',exact=True)).not_to_be_checked()
 page.get_by_role('button',name='Paket festhalten und ZIP exportieren …',exact=True).click()
 assert page.evaluate('window.calls.filter(c=>c.cmd==="einreichung_erstellen").length')==0
 # Draft remains in its project after switching to another tab.
 view('Stammblatt');view('Einreichungen')
 expect(page.get_by_role('textbox',name='Bezeichnung',exact=True)).to_have_value('Korrektur der Planunterlagen')
 page.evaluate('window.cancelSave=false;window.failExport=true')
 page.get_by_role('button',name='Paket festhalten und ZIP exportieren …',exact=True).click()
 expect(page.get_by_role('alert').filter(has_text='ZIP konnte nicht geschrieben werden; Paket bleibt gespeichert')).to_be_visible()
 package=page.evaluate('window.calls.find(c=>c.cmd==="einreichung_erstellen").args')
 assert package['projektId']==wf['projekt']['id']
 assert package['paket']['dokumente']==[{'dokument_id':wf['plan4']['document_id'],'version':4}]
 expect(page.get_by_text('Korrektur der Planunterlagen · Bauamt · Vorbereitet, noch nicht eingereicht · 1 Unterlagen',exact=True)).to_be_visible()
 page.get_by_text('Korrektur der Planunterlagen · Bauamt · Vorbereitet, noch nicht eingereicht · 1 Unterlagen',exact=True).click()
 page.get_by_role('alert').get_by_role('button',name='Schließen',exact=True).click()
 page.evaluate('window.failExport=false')
 page.get_by_role('button',name='Dieses ZIP erneut exportieren …',exact=True).first.click()
 page.get_by_role('button',name='Einreichungsdatum erfassen …',exact=True).click()
 page.get_by_label('Einreichungsdatum',exact=True).fill('2026-10-01')
 page.get_by_label('Übermittlungsweg / Aktenzeichen / Notiz',exact=True).fill('Bauportal, AZ 123, korrigierter Lageplan')
 page.get_by_role('button',name='Einreichung bestätigen',exact=True).click()
 expect(page.get_by_text('Korrektur der Planunterlagen · Bauamt · Eingereicht am 2026-10-01 · 1 Unterlagen',exact=True)).to_be_visible()
 confirmation=page.evaluate('window.calls.find(c=>c.cmd==="einreichung_bestaetigen").args')
 assert confirmation['basis']==1 and confirmation['datum']=='2026-10-01'
 expect(page.get_by_role('cell',name='v4 · 2026-10-01',exact=True)).to_be_visible()
 page.evaluate('document.querySelector(".aktenbereich").scrollTop=0')
 page.screenshot(path=str(out/'02-einreichungschronik.png'))
 page.get_by_label('Empfänger',exact=True).fill('Andere Behörde')
 expect(page.get_by_role('button',name='Paket festhalten und ZIP exportieren …',exact=True)).to_be_disabled()
 page.set_viewport_size({'width':980,'height':900})
 assert page.evaluate('document.documentElement.scrollWidth <= innerWidth'), 'horizontal page overflow'
 assert not errors,errors
 browser.close()
(out/'ui-pruefung.json').write_text(json.dumps({'checks':['Excel-Datei gezielt suchen','Öffnen mit konkreter Dokumentversion','Externe Arbeitskopie und Änderungsprüfung','Konflikt erhält Arbeitskopie','Übernahme als neue Version','Vergleich eingereichter und geänderter Dateien','Neue/geänderte Auswahl','ZIP-Abbruch erhält Entwurf','Entwurf bei Bereichswechsel','Auswahl mit festem Versionstand','Exportfehler erhält gespeichertes Paket','ZIP erneut exportieren','Einreichungsdatum/Notiz ausdrücklich bestätigen','Empfängerwechsel sperrt veralteten Vergleich','Schmale Ansicht ohne Seitenüberlauf'],'page_errors':errors},ensure_ascii=False,indent=2)+'\n')
print('Dateiverwaltung und Einreichungen: 15 UI-Prüfungen bestanden')
