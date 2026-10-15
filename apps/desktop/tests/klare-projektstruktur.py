"""Project-first forms and hierarchical navigation in the compiled real UI."""
from pathlib import Path
import json,base64,os
from playwright.sync_api import sync_playwright,expect
root=Path(__file__).resolve().parent
pdfroot=Path(os.environ.get('BAUPROJEKT_PDF_ARTEFAKTE','/workspace/review-main/pdf-ablage'))
wf=json.loads((pdfroot/'workflow.json').read_text())
fixture=json.loads((root/'fixtures/projekt.json').read_text());fixture['name']='Wohnhaus Lindenweg'
fixture['planungsrecht'].update({'bau_nvo':None,'rechtsgrundlage':None,'bebauungsplan_art':'ungeklaert','baugrundstuecke':None,'berechnung':None})
pdf=base64.b64encode((pdfroot/'vorlage.pdf').read_bytes()).decode()
out=root.parents[2]/'docs/klare-projektstruktur';out.mkdir(exist_ok=True)
custom='''
const original=window.__TAURI_INTERNALS__.invoke;
const fields=[{name:'Vorname',beschriftung:'Vorname',art:'text',wert:null,optionen:[],schreibgeschuetzt:false,max_laenge:null},{name:'Zustimmung',beschriftung:'Zustimmung',art:'ankreuzfeld',wert:null,optionen:['Yes'],schreibgeschuetzt:false,max_laenge:null}];
const seiten=[{nummer:1,breite_mm:210,hoehe_mm:297,rotation:0,platzierbar:true}];
const source={name:'Bauantrag.pdf',sha256:'a'.repeat(64),size:1234};
window.projectDocs=[];window.failForm=true;window.mirrorFailure=false;window.formRoot='C:\\\\BauProjektIO\\\\Formulare';window.projectRoot='C:\\\\BauProjektIO\\\\Projekte';window.cancelFolder=true;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 const handled=['formularbibliothek','bibliothek_formular','bibliothek_formular_speichern','formular_im_projekt_speichern','dokumente_auflisten','formular_analysieren','pdf_vorschau','projektordner_status','projektordner_waehlen','formularordner_waehlen','baugrundstuecke_ermitteln','plugin:dialog|save','plugin:dialog|open','plugin:dialog|message'];
 if(!handled.includes(cmd))return original(cmd,args);
 window.calls.push({cmd,args:JSON.parse(JSON.stringify(args))});
 switch(cmd){
 case 'formularbibliothek':return {root:window.formRoot,vorlagen:[source]};
 case 'projektordner_status':return window.projectRoot;
 case 'projektordner_waehlen':window.projectRoot=args.pfad;return;
 case 'formularordner_waehlen':window.formRoot=args.pfad;return;
 case 'bibliothek_formular':return {schema:{felder:fields,vorlage:null,vorschlag:{}},pdf_base64:window.sourcePdf,seiten};
 case 'bibliothek_formular_speichern':if(window.failForm)throw {art:'datei',meldung:'Speichern fehlgeschlagen; Eingaben bleiben erhalten'};{const d={...window.pdfWorkflow.ausgefuellt,project_id:args.projektId,file_name:'Bauantrag.pdf',projektordner_pfad:window.projectRoot+'\\\\Wohnhaus\\\\Dokumente\\\\Bauantrag.pdf',ablage_hinweis:window.mirrorFailure?'Im Projekt gespeichert; Ordnerausgabe nicht erreichbar':null};window.projectDocs.push(d);return structuredClone(d);}
 case 'dokumente_auflisten':return structuredClone(window.projectDocs.filter(d=>d.project_id===args.projektId));
 case 'formular_analysieren':return {dokument:window.projectDocs.at(-1),felder:fields,vorlage:null,vorschlag:{}};
 case 'pdf_vorschau':return {dokument:window.projectDocs.at(-1),pdf_base64:window.sourcePdf,seiten,felder:fields};
 case 'formular_im_projekt_speichern':{const d={...window.projectDocs.at(-1),version:args.basis+1};window.projectDocs.push(d);return structuredClone(d);}
 case 'baugrundstuecke_ermitteln':return args.plan.flurstuecke.length?[{schluessel:'grundbuch-unit',bezeichnung:'Flurstück einzeln angesetzt',flurstueck_ids:args.plan.flurstuecke.map(f=>f.id),eigentumsflaeche_m2:900,abzuege_m2:0,bauland_m2:900}]:[];
 case 'plugin:dialog|save':throw {art:'test',meldung:'Unexpected save dialog'};
 case 'plugin:dialog|open':return window.cancelFolder?null:'C:\\\\BauProjektIO\\\\NeueProjekte';
 case 'plugin:dialog|message':return args.buttons?.OkCancelCustom?.[0]??'Yes';
 }
};
'''
errors=[];checks=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=browser.new_page(viewport={'width':1440,'height':1000},color_scheme='light')
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(fixture)+';window.pdfWorkflow='+json.dumps(wf)+';window.sourcePdf='+json.dumps(pdf)+';'+(root/'tauri-mock.js').read_text()+custom)
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1431'))
 page.locator('.projekttabelle').get_by_role('button',name='Wohnhaus Lindenweg',exact=True).click()
 nav=page.get_by_role('complementary',name='Arbeitsbereiche')
 def view(n):nav.get_by_role('button',name=n,exact=True).click()
 def section():return page.locator('.projekt-sitzung:visible > main > section:not([hidden])')
 assert page.get_by_role('navigation',name='Projektbereiche').count()==0
 assert page.get_by_role('navigation',name='Werkzeugleiste').count()==0
 expect(nav.get_by_role('button',name='GRZ',exact=True)).to_be_disabled()
 expect(nav.get_by_role('button',name='Bebauungsplanfestsetzungen',exact=True)).to_have_count(0)
 checks.append('Eine Sidebar, keine doppelte Projekt- oder Werkzeugnavigation')
 view('Rechtsgrundlagen')
 section().get_by_label('Bebauungsplan',exact=True).select_option('qualifiziert')
 section().get_by_label('BauNVO',exact=True).select_option('1977')
 section().get_by_label('Planbezeichnung',exact=True).fill('Lindenweg Nord')
 section().get_by_role('button',name='Speichern',exact=True).click()
 expect(nav.get_by_role('button',name='GRZ',exact=True)).to_be_enabled()
 expect(nav.get_by_role('button',name='Bebauungsplanfestsetzungen',exact=True)).to_be_visible()
 assert not section().get_by_label('GFZ',exact=True).count()
 view('Bebauungsplanfestsetzungen')
 section().get_by_label('GFZ',exact=True).fill('0.8')
 section().get_by_role('button',name='Speichern',exact=True).click()
 checks.append('Rechtsgrundlagen schalten Bereiche frei; Festsetzungen separat')
 view('Flurstücke und Grundbuch')
 section().get_by_role('button',name='Flurstück hinzufügen',exact=True).click()
 for label,value in [('Gemarkung','Linden'),('Flur','2'),('Flurstücknummer','123/4'),('Flurstücksfläche','900')]:section().get_by_label(label,exact=True).fill(value)
 page.evaluate('window.failSave=true')
 section().get_by_role('button',name='Grundstücksinformationen speichern',exact=True).click()
 expect(section().get_by_role('alert')).to_contain_text('Speichern fehlgeschlagen')
 expect(section().get_by_label('Flurstücknummer',exact=True)).to_have_value('123/4')
 page.evaluate('window.failSave=false')
 section().get_by_role('button',name='Grundstücksinformationen speichern',exact=True).click()
 expect(section().get_by_text('Ungespeicherte Änderungen',exact=True)).not_to_be_visible()
 assert page.evaluate('window.testProjects[0].planungsrecht.grundstuecksinformationen.flurstuecke[0].laufende_nummer') is None
 assert not page.evaluate('window.testProjects[0].planungsrecht.baugrundstuecksermittlung')
 for width in [1024,768,600]:
  page.set_viewport_size({'width':width,'height':900});assert not page.evaluate('document.documentElement.scrollWidth>innerWidth')
 page.set_viewport_size({'width':1440,'height':1000})
 page.screenshot(path=str(out/'00-flurstuecksinformationen.png'))
 view('Baugrundstück')
 expect(section().get_by_role('button',name='Flurstück hinzufügen',exact=True)).to_have_count(0)
 expect(section().get_by_label('Flurstücknummer',exact=True)).to_have_count(0)
 section().get_by_role('button',name='Baugrundstücke ermitteln',exact=True).click()
 expect(section().get_by_role('alert')).to_have_count(0)
 assert all(f['abweichende_zuordnung'] is None and not f['grundbuchblatt'] for f in page.evaluate("window.calls.filter(c=>c.cmd==='baugrundstuecke_ermitteln').at(-1).args.plan.flurstuecke"))
 expect(section().get_by_role('cell',name='900 m²',exact=True)).to_have_count(4)
 page.screenshot(path=str(out/'01-baugrundstueck.png'))
 section().get_by_role('button',name='Eingaben speichern',exact=True).click()
 page.wait_for_function('window.testProjects[0].planungsrecht.baugrundstuecksermittlung?.ansaetze.length===1')
 assert 'flurstuecke' not in page.evaluate('window.testProjects[0].planungsrecht.baugrundstuecksermittlung')
 assert page.evaluate('window.testProjects[0].planungsrecht.grundstuecksinformationen.flurstuecke[0].flaeche_m2')==900

 view('GRZ')
 section().get_by_role('button',name='Bauteil hinzufügen',exact=True).click()
 section().get_by_label('Bezeichnung',exact=True).fill('Wohnhaus')
 section().get_by_label('Grundfläche (m²)',exact=True).fill('150')
 view('GFZ')
 expect(section().get_by_role('button',name='Bauteil hinzufügen',exact=True)).to_have_count(0)
 section().get_by_role('button',name='Geschoss hinzufügen',exact=True).click()
 section().get_by_label('Bezeichnung',exact=True).fill('Erdgeschoss')
 view('GRZ');expect(section().get_by_label('Bezeichnung',exact=True)).to_have_value('Wohnhaus')
 view('Flurstücke und Grundbuch');expect(section().get_by_label('Flurstücknummer',exact=True)).to_have_value('123/4')
 checks.append('Flurstücksfakten ohne Grundbuch speichern; getrennte Ermittlung ohne Grundbuchpflicht; gemeinsame Eingaben für GRZ/GFZ')
 view('Formulare')
 section().get_by_role('button',name='Ausfüllen',exact=True).click()
 dialog=page.get_by_role('dialog')
 dialog.get_by_label('Vorname',exact=True).fill('Daniel')
 dialog.get_by_label('Zustimmung',exact=True).check()
 expect(page.get_by_text('PDF-Seite wird geladen …',exact=True)).not_to_be_visible()
 page.screenshot(path=str(out/'02-formular-im-projekt.png'))
 assert not page.evaluate('window.projectDocs.length')
 dialog.get_by_role('button',name='Im Projekt speichern',exact=True).click()
 expect(dialog.get_by_text('Speichern fehlgeschlagen; Eingaben bleiben erhalten',exact=True)).to_be_visible()
 expect(dialog.get_by_label('Vorname',exact=True)).to_have_value('Daniel')
 page.evaluate('window.failForm=false')
 dialog.get_by_role('button',name='Im Projekt speichern',exact=True).click()
 expect(page.get_by_role('dialog')).to_have_count(0)
 assert page.evaluate('window.projectDocs.length')==1
 assert page.evaluate('window.calls.filter(c=>c.cmd==="plugin:dialog|save").length')==0
 assert page.evaluate('window.calls.find(c=>c.cmd==="bibliothek_formular_speichern").args.projektId')==fixture['id']
 view('Projektdateien')
 expect(section().get_by_role('table',name='Projektdateien',exact=True).locator('tbody tr')).to_have_count(1)
 section().get_by_role('button',name='Formular bearbeiten',exact=True).click()
 page.get_by_role('dialog').get_by_label('Vorname',exact=True).fill('Daniel korrigiert')
 page.get_by_role('dialog').get_by_role('button',name='Im Projekt speichern',exact=True).click()
 expect(page.get_by_role('dialog')).to_have_count(0)
 expect(section().get_by_role('table',name='Projektdateien',exact=True).locator('tbody tr')).to_have_count(1)
 assert page.evaluate('window.projectDocs.at(-1).version')==2
 checks.append('Projektvorlage ohne leere Zwischenkopie; erneutes Ausfüllen erhält Dokumentidentität; kein Exportdialog')
 view('Flurstücke und Grundbuch')
 section().locator('details > summary').first.click()
 auszug=page.evaluate('window.projectDocs.at(-1)')
 section().get_by_label('Grundbuchauszug',exact=True).select_option(auszug['document_id']+':2')
 section().get_by_role('button',name='Grundstücksinformationen speichern',exact=True).click()
 page.wait_for_function('window.testProjects[0].planungsrecht.grundstuecksinformationen.flurstuecke[0].grundbuchauszug?.version===2')
 checks.append('Neu hinzugefügte Projektdateien erscheinen beim erneuten Öffnen der Auszugswahl; Bezug speichert die konkrete Version')

 nav.get_by_role('button',name='Einstellungen',exact=True).click()
 page.get_by_label('Farbschema',exact=True).select_option('dark')
 assert page.evaluate('document.documentElement.dataset.theme')=='dark'
 page.emulate_media(color_scheme='dark')
 page.get_by_label('Farbschema',exact=True).select_option('light')
 assert page.evaluate('document.documentElement.dataset.theme')=='light'
 page.get_by_label('Farbschema',exact=True).select_option('system')
 assert page.evaluate('document.documentElement.dataset.theme')=='dark'
 page.emulate_media(color_scheme='light')
 page.wait_for_function('document.documentElement.dataset.theme==="light"')
 page.get_by_role('button',name='Projektordner auswählen …',exact=True).click()
 assert page.evaluate('window.calls.filter(c=>c.cmd==="projektordner_waehlen").length')==0
 page.evaluate('window.cancelFolder=false')
 page.get_by_role('button',name='Projektordner auswählen …',exact=True).click()
 expect(page.get_by_text('C:\\BauProjektIO\\NeueProjekte',exact=True)).to_be_visible()
 page.screenshot(path=str(out/'03-einstellungen.png'))
 checks.append('Hell/Dunkel/System und Ordnerauswahl mit Abbruch in Einstellungen')
 page.set_viewport_size({'width':1000,'height':800})
 assert page.evaluate('document.documentElement.scrollWidth <= innerWidth')
 assert not errors,errors
 browser.close()
(out/'ui-pruefung.json').write_text(json.dumps({'checks':checks,'page_errors':errors},ensure_ascii=False,indent=2)+'\n')
print(json.dumps(checks,ensure_ascii=False,indent=2))
