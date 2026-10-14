"""Compiled UI + real PDF.js, test IPC; actual folder/DMS contracts tested in Rust."""
from pathlib import Path
import base64, json, os
from playwright.sync_api import sync_playwright, expect
root = Path(__file__).resolve().parent
fixtures = Path(os.environ.get('BAUPROJEKT_PDF_ARTEFAKTE', '/workspace/review-main/pdf-ablage'))
wf = json.loads((fixtures/'workflow.json').read_text())
pdf = base64.b64encode((fixtures/'vorlage.pdf').read_bytes()).decode()
out = root.parents[2]/'docs/formularbibliothek'
out.mkdir(exist_ok=True)
custom = '''
const original = window.__TAURI_INTERNALS__.invoke;
window.libraryRoot = 'C:\\\\BauProjektIO\\\\Formulare';
window.libraryTemplates = [];
window.failConnect = true;
window.cancelImport = true;
const fields = [{name:'Vorname',beschriftung:'Vorname',art:'text',wert:null,optionen:[],schreibgeschuetzt:false,max_laenge:null}];
const preview = {pdf_base64:window.libraryPdf,seiten:[{nummer:1,breite_mm:210,hoehe_mm:297,rotation:0,platzierbar:true}],felder:fields};
window.__TAURI_INTERNALS__.invoke = async (cmd, args={}) => {
 const handled = ['formularbibliothek','formularordner_waehlen','bibliothek_importieren','bibliothek_vorschau','bibliothek_uebernehmen','formular_analysieren','pdf_vorschau','formular_ausfuellen','dokument_exportieren','plugin:dialog|open','plugin:dialog|save','plugin:dialog|message'];
 if (!handled.includes(cmd)) return original(cmd,args);
 window.calls.push({cmd,args:JSON.parse(JSON.stringify(args))});
 switch(cmd) {
 case 'formularbibliothek':return {root:window.libraryRoot,vorlagen:window.libraryTemplates};
 case 'formularordner_waehlen':if(window.failConnect)throw {art:'intern',meldung:'Ordner nicht erreichbar'};window.libraryRoot=args.pfad;return;
 case 'bibliothek_importieren':window.libraryTemplates.push({name:'Bauantrag.pdf',size:1234,sha256:'a'.repeat(64)});return window.libraryTemplates[0];
 case 'bibliothek_vorschau':return preview;
 case 'bibliothek_uebernehmen':return {...window.workflow.vorlage,project_id:args.projektId};
 case 'pdf_vorschau':return {...preview,dokument:window.workflow.vorlage};
 case 'formular_analysieren':return {dokument:window.workflow.vorlage,felder:fields,vorlage:null,vorschlag:{}};
 case 'formular_ausfuellen':return window.workflow.ausgefuellt;
 case 'dokument_exportieren':return;
 case 'plugin:dialog|open':return args.options.directory?'C:\\\\BauProjektIO\\\\NeueFormulare':window.cancelImport?null:'C:\\\\Downloads\\\\Bauantrag.pdf';
 case 'plugin:dialog|save':return 'C:\\\\BauProjektIO\\\\Daniel.pdf';
 case 'plugin:dialog|message':return 'Yes';
 }
};
'''
errors=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True,executable_path='/usr/bin/chromium')
 page=browser.new_page(viewport={'width':1440,'height':1000})
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(wf['projekt'])+';window.workflow='+json.dumps(wf)+';window.libraryPdf='+json.dumps(pdf)+';'+(root/'tauri-mock.js').read_text()+custom)
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1431'))
 page.locator('.aktennavigation').get_by_role('button',name='Formularbibliothek',exact=True).click()
 expect(page.get_by_text('Noch keine PDF-Vorlagen vorhanden.',exact=False)).to_be_visible()
 assert page.evaluate('window.calls.filter(c=>c.cmd==="projekt_oeffnen").length')==0
 page.get_by_role('button',name='PDF-Vorlage importieren …',exact=True).click()
 assert page.evaluate('window.calls.filter(c=>c.cmd==="bibliothek_importieren").length')==0
 page.evaluate('window.cancelImport=false')
 page.get_by_role('button',name='PDF-Vorlage importieren …',exact=True).click()
 page.get_by_role('button',name='Vorlage ansehen',exact=True).click()
 page.wait_for_function('document.querySelector(".pdf-seite canvas")?.width>0')
 expect(page.get_by_text('PDF-Seite wird geladen …',exact=True)).not_to_be_visible()
 expect(page.get_by_role('button',name='Im Projekt verwenden …',exact=True)).to_be_disabled()
 page.get_by_label('Projekt',exact=True).select_option(wf['projekt']['id'])
 page.screenshot(path=str(out/'01-bibliothek.png'))
 page.get_by_role('button',name='Im Projekt verwenden …',exact=True).click()
 page.get_by_label('Vorname',exact=True).fill('Daniel')
 page.get_by_role('button',name='Ausfüllen und ablegen',exact=True).click()
 page.wait_for_function('window.calls.some(c=>c.cmd==="dokument_exportieren")')
 use=page.evaluate('window.calls.find(c=>c.cmd==="bibliothek_uebernehmen").args')
 assert use['projektId']==wf['projekt']['id']
 assert use['vorlage']['sha256']=='a'*64
 filled=page.evaluate('window.calls.find(c=>c.cmd==="formular_ausfuellen").args')
 assert filled['werte']=={'Vorname':'Daniel'} and filled['basis']==1
 exported=page.evaluate('window.calls.find(c=>c.cmd==="dokument_exportieren").args')
 assert exported['version']==wf['ausgefuellt']['version']
 assert exported['dokumentId']==wf['ausgefuellt']['document_id']
 page.get_by_role('button',name='Formularordner auswählen …',exact=True).click()
 expect(page.get_by_text('Ordner nicht erreichbar',exact=False)).to_be_visible()
 expect(page.locator('dd').get_by_text('C:\\BauProjektIO\\Formulare',exact=True)).to_be_visible()
 page.evaluate('window.failConnect=false')
 page.get_by_role('button',name='Formularordner auswählen …',exact=True).click()
 expect(page.get_by_text('C:\\BauProjektIO\\NeueFormulare',exact=True)).to_be_visible()
 assert not errors, errors
 browser.close()
(out/'ui-pruefung.json').write_text(json.dumps({'checks':['Ohne geöffnetes Projekt verfügbar','Import-Abbruch unverändert','Reale PDF.js-Vorschau','Zielprojekt erforderlich','Projektkopie mit ausgewähltem Hash','Ausfüllen mit festem Dokumentstand','Export der ausgefüllten Projektdatei','Verbindungsfehler erhält alten Ordner','Erfolgreicher Ordnerwechsel'],'page_errors':errors},ensure_ascii=False,indent=2)+'\n')
print('Formularbibliothek: 9 UI-Prüfungen bestanden')
