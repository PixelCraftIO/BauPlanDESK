"""Template editing stays in the library; filling starts in the project."""
from pathlib import Path
import json,base64,os
from playwright.sync_api import sync_playwright,expect
root=Path(__file__).resolve().parent
fixtures=Path(os.environ.get('BAUPROJEKT_PDF_ARTEFAKTE','/workspace/review-main/pdf-ablage'))
wf=json.loads((fixtures/'workflow.json').read_text())
pdfs={name:base64.b64encode((fixtures/(name+'.pdf')).read_bytes()).decode() for name in ['original','vorlage']}
out=Path(os.environ.get('BAUPROJEKT_PDF_UI_ARTEFAKTE',str(root.parents[2]/'docs/klare-projektstruktur/pdf')));out.mkdir(parents=True,exist_ok=True)
custom='''
const original=window.__TAURI_INTERNALS__.invoke;
window.sources=[];window.cancelImport=true;window.failFields=true;window.discard=false;
const fields=[{name:'Vorname',beschriftung:'Vorname',art:'text',wert:null,optionen:[],schreibgeschuetzt:false,max_laenge:null}];
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 const handled=['formularbibliothek','bibliothek_importieren','bibliothek_vorschau','bibliothek_felder_erstellen','plugin:dialog|open','plugin:dialog|message'];
 if(!handled.includes(cmd))return original(cmd,args);
 window.calls.push({cmd,args:JSON.parse(JSON.stringify(args))});
 switch(cmd){
 case 'formularbibliothek':return {root:'C:\\\\BauProjektIO\\\\Formulare',vorlagen:structuredClone(window.sources)};
 case 'bibliothek_importieren':{const v={name:'Eigenes Formular.pdf',sha256:'a'.repeat(64),size:1000,felder:0};window.sources.push(v);return v;}
 case 'bibliothek_vorschau':return {pdf_base64:args.vorlage.felder?window.pdfBytes.vorlage:window.pdfBytes.original,seiten:[{nummer:1,breite_mm:210,hoehe_mm:297,rotation:0,platzierbar:true}],felder:args.vorlage.felder?fields:[]};
 case 'bibliothek_felder_erstellen':if(window.failFields)throw {art:'datei',meldung:'Felder konnten nicht gespeichert werden'};{const v={name:'Eigenes Formular (ausfüllbar).pdf',sha256:'b'.repeat(64),size:2000,felder:2};window.sources.push(v);return v;}
 case 'plugin:dialog|open':return window.cancelImport?null:'C:\\\\Downloads\\\\Eigenes Formular.pdf';
 case 'plugin:dialog|message':return window.discard?(args.buttons?.OkCancelCustom?.[0]??'Yes'):'Weiter bearbeiten';
 }
};
'''
errors=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=browser.new_page(viewport={'width':1440,'height':1000})
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(wf['projekt'])+';window.pdfBytes='+json.dumps(pdfs)+';'+(root/'tauri-mock.js').read_text()+custom)
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1432'))
 page.get_by_role('complementary',name='Arbeitsbereiche').get_by_role('button',name='Formularbibliothek',exact=True).click()
 expect(page.get_by_role('heading',name='Formularbibliothek',exact=True)).to_be_visible()
 assert page.get_by_role('button',name='Im Projekt verwenden …',exact=True).count()==0
 page.get_by_role('button',name='PDF-Vorlage importieren …',exact=True).click()
 assert page.evaluate('window.sources.length')==0
 page.evaluate('window.cancelImport=false')
 page.get_by_role('button',name='PDF-Vorlage importieren …',exact=True).click()
 page.get_by_role('button',name='Felder hinzufügen …',exact=True).click()
 dialog=page.get_by_role('dialog')
 dialog.get_by_role('button',name='Textfeld hinzufügen',exact=True).click()
 dialog.get_by_label('Feldname / Platzhalter',exact=True).fill('<Vorname>')
 dialog.get_by_role('button',name='Checkbox hinzufügen',exact=True).click()
 dialog.get_by_label('Feldname / Platzhalter',exact=True).fill('Zustimmung')
 expect(dialog.get_by_text('PDF-Seite wird geladen …',exact=True)).not_to_be_visible()
 page.keyboard.press('Escape')
 expect(dialog).to_be_visible()
 dialog.get_by_role('button',name='Vorlage erstellen',exact=True).click()
 expect(dialog.get_by_text('Felder konnten nicht gespeichert werden',exact=True)).to_be_visible()
 expect(dialog.get_by_label('Feldname / Platzhalter',exact=True)).to_have_value('Zustimmung')
 page.screenshot(path=str(out/'01-bibliothekseditor.png'))
 page.evaluate('window.failFields=false')
 dialog.get_by_role('button',name='Vorlage erstellen',exact=True).click()
 expect(page.get_by_role('dialog')).to_have_count(0)
 assert page.evaluate('window.sources.length')==2
 assert page.evaluate('window.calls.filter(c=>["bibliothek_uebernehmen","formular_vorlage_erstellen","projekt_oeffnen"].includes(c.cmd)).length')==0
 assert not errors,errors
 browser.close()
(out/'pruefung.json').write_text(json.dumps({'checks':['Bibliothek unabhängig vom Projekt','Importabbruch ohne Änderung','Felder direkt an Bibliotheksquelle definieren','PDF.js rendert Original','Escape und Speicherfehler erhalten Definitionen','Neue ausfüllbare Vorlage ohne Projektdokument'], 'errors':errors},ensure_ascii=False,indent=2)+'\n')
print('Bibliothekseditor: 6 Prüfungen bestanden')
