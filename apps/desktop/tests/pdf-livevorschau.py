"""Live-Vorschau: echte Core-PDFs zeichnen; IPC kontrolliert Reihenfolge und DMS-Schreibgrenzen."""
from pathlib import Path
import base64,json,os
from playwright.sync_api import sync_playwright,expect
root=Path(__file__).resolve().parent
fixtures=Path(os.environ.get('BAUPROJEKT_PDF_LIVE_ARTEFAKTE','/workspace/review-main/pdf-live'))
wf=json.loads((fixtures/'workflow.json').read_text())
pdfs={n:base64.b64encode((fixtures/(n+'.pdf')).read_bytes()).decode() for n in ['original','daniel','anna']}
fixture=json.loads((root/'fixtures/projekt.json').read_text());fixture['id']=wf['dokument']['project_id'];fixture['name']='PDF-Liveprojekt'
out=Path(os.environ.get('BAUPROJEKT_PDF_LIVE_UI_ARTEFAKTE',str(root.parents[2]/'docs/pdf-livevorschau')));out.mkdir(parents=True,exist_ok=True)
custom=r'''
const original=window.__TAURI_INTERNALS__.invoke;
const clone=v=>JSON.parse(JSON.stringify(v));
window.previewCalls=[];window.docs=[];window.savedValues={};window.holdNextPreview=false;window.failPreview=false;
const vorlage={name:'Liveformular.pdf',sha256:window.wf.dokument.sha256,size:window.wf.dokument.size_bytes,felder:3};
const defaults={Vorname:'',Anschrift:'',Zustimmung:false};
const merged=(pdf,werte)=>({...defaults,...(pdf===window.pdfs.daniel?window.wf.daniel:pdf===window.pdfs.anna?window.wf.anna:{}),...werte});
const checkpoint=(werte)=>werte.Vorname==='Anna'&&werte.Anschrift===window.wf.anna.Anschrift&&!werte.Zustimmung?window.pdfs.anna:werte.Vorname==='Daniel'&&werte.Anschrift===window.wf.daniel.Anschrift&&werte.Zustimmung?window.pdfs.daniel:window.pdfs.original;
const fields=()=>window.wf.felder.map(f=>({...f,wert:window.savedValues[f.name]??f.wert}));
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 const handled=['formularbibliothek','bibliothek_formular','formular_livevorschau','bibliothek_formular_speichern','formular_im_projekt_speichern','dokumente_auflisten','formular_analysieren','pdf_vorschau'];
 if(!handled.includes(cmd))return original(cmd,args);
 window.calls.push({cmd,args:clone(args)});
 switch(cmd){
 case 'formularbibliothek':return {root:'/Formulare',vorlagen:[vorlage]};
 case 'bibliothek_formular':return {schema:{felder:window.wf.felder,vorlage:null,vorschlag:{}},pdf_base64:window.pdfs.original,seiten:window.wf.seiten};
 case 'formular_livevorschau':{
   const w=merged(args.pdfBase64,args.werte);window.previewCalls.push(clone(args));
   const result=checkpoint(w);
   if(window.holdNextPreview){window.holdNextPreview=false;await new Promise(r=>window.releasePreview=()=>r());}
   if(window.failPreview)throw {art:'pdf',meldung:'Test: Vorschau konnte nicht erzeugt werden'};
   return result;
 }
 case 'bibliothek_formular_speichern':window.savedValues=clone(args.werte);{const d={...window.wf.dokument,version:1};window.docs=[d];return d;}
 case 'formular_im_projekt_speichern':window.savedValues={...window.savedValues,...args.werte};{const d={...window.wf.dokument,version:args.basis+1};window.docs=[d];return d;}
 case 'dokumente_auflisten':return clone(window.docs);
 case 'formular_analysieren':return {dokument:window.docs[0],felder:fields(),vorlage:null,vorschlag:{}};
 case 'pdf_vorschau':return {dokument:window.docs[0],felder:fields(),seiten:window.wf.seiten,pdf_base64:checkpoint(window.savedValues)};
 }
};
'''
errors=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=browser.new_page(viewport={'width':1440,'height':1100})
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(fixture)+';window.wf='+json.dumps(wf)+';window.pdfs='+json.dumps(pdfs)+';'+(root/'tauri-mock.js').read_text()+custom)
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1430'))
 page.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click()
 def view(name):page.locator('.aktennavigation').get_by_role('button',name=name,exact=True).click()
 def dialog():return page.get_by_role('dialog')
 def canvas():return dialog().locator('canvas')
 def aktuell():expect(dialog().get_by_text('Vorschau mit Ihren aktuellen Eingaben.',exact=True)).to_be_visible()
 def bitmap():return canvas().evaluate('(c)=>c.toDataURL()')
 def setzen(typ):
  d=wf[typ];dialog().get_by_label('Vorname',exact=True).fill(d['Vorname']);dialog().get_by_label('Anschrift',exact=True).fill(d['Anschrift']);dialog().get_by_label('Zustimmung',exact=True).set_checked(d['Zustimmung'])
 view('Formulare');page.locator('.projekt-sitzung:visible').get_by_role('button',name='Ausfüllen',exact=True).click()
 aktuell();expect(dialog().get_by_label('Anschrift',exact=True)).to_have_js_property('tagName','TEXTAREA')
 dialog().get_by_label('Seite',exact=True).select_option('2');expect(dialog().get_by_text('PDF-Seite wird geladen …',exact=True)).not_to_be_visible()
 leer=bitmap();setzen('daniel');page.wait_for_function('window.previewCalls.at(-1)?.werte.Anschrift===window.wf.daniel.Anschrift&&window.previewCalls.at(-1)?.werte.Zustimmung===true');aktuell()
 assert bitmap()!=leer
 expect(dialog().get_by_label('Seite',exact=True)).to_have_value('2')
 assert not page.evaluate('window.docs.length')
 assert not page.evaluate('window.calls.some(c=>["bibliothek_formular_speichern","formular_im_projekt_speichern","plugin:dialog|save"].includes(c.cmd))')
 page.evaluate("window.print=()=>window.formularDruckseiten=document.querySelectorAll('#bau-druckbereich .druckseite img').length")
 dialog().get_by_role('button',name='Aktuelle Eingaben drucken …',exact=True).click()
 page.wait_for_function('window.formularDruckseiten===2')
 page.evaluate("window.dispatchEvent(new Event('afterprint'))")
 assert not page.evaluate('window.docs.length')
 expect(dialog().get_by_label('Anschrift',exact=True)).to_have_value(wf['daniel']['Anschrift'])
 aktuell()
 page.screenshot(path=str(out/'01-adresse-live.png'))
 # Eine alte laufende Anfrage darf den danach eingegebenen Stand nicht anzeigen.
 page.evaluate('window.holdNextPreview=true')
 dialog().get_by_label('Vorname',exact=True).fill('Anna')
 page.wait_for_function('typeof window.releasePreview==="function"')
 n=page.evaluate('window.previewCalls.length');setzen('anna');page.wait_for_timeout(220)
 assert page.evaluate('window.previewCalls.length')==n
 page.evaluate('window.releasePreview()')
 page.wait_for_function('window.previewCalls.length>'+str(n)+'&&window.previewCalls.at(-1).werte.Anschrift===window.wf.anna.Anschrift')
 aktuell();anna=bitmap();assert anna!=leer
 assert page.evaluate('window.previewCalls.every(c=>c.pdfBase64===window.pdfs.original)')
 # Vorschaufehler erhalten die Eingaben; eine spätere gültige Eingabe aktualisiert wieder.
 page.evaluate('window.failPreview=true');dialog().get_by_label('Vorname',exact=True).fill('Daniel')
 expect(dialog().get_by_text('Test: Vorschau konnte nicht erzeugt werden',exact=False)).to_be_visible()
 expect(dialog().get_by_label('Anschrift',exact=True)).to_have_value(wf['anna']['Anschrift'])
 page.evaluate('window.failPreview=false');setzen('daniel');aktuell()
 dialog().get_by_role('button',name='Im Projekt speichern',exact=True).click();expect(dialog()).to_have_count(0)
 assert page.evaluate('window.docs.length')==1
 assert page.evaluate('window.savedValues.Anschrift')==wf['daniel']['Anschrift']
 view('Projektdateien');page.locator('.projekt-sitzung:visible').get_by_role('button',name='Formular bearbeiten',exact=True).click();aktuell()
 dialog().get_by_label('Seite',exact=True).select_option('2');setzen('anna')
 page.wait_for_function('window.previewCalls.at(-1)?.pdfBase64===window.pdfs.daniel&&window.previewCalls.at(-1)?.werte.Anschrift===window.wf.anna.Anschrift&&window.previewCalls.at(-1)?.werte.Zustimmung===false')
 aktuell();assert bitmap()==anna
 dialog().get_by_role('button',name='Im Projekt speichern',exact=True).click();expect(dialog()).to_have_count(0)
 assert page.evaluate('window.docs.length')==1 and page.evaluate('window.docs[0].version')==2
 assert page.evaluate('window.savedValues.Anschrift')==wf['anna']['Anschrift']
 assert not errors,errors
 browser.close()
(out/'pruefung.json').write_text(json.dumps({'checks':['Mehrzeilige Maske aus PDF-Merkmal','Echte PDF-Pixel aktualisiert und gewählte Seite erhalten','Keine DMS-Version durch Vorschau und direkten Druck','Veraltete Antwort verworfen und höchstens eine Anfrage gleichzeitig','Vorschaufehler und anschließende Erholung','Bibliotheks- und bestehendes Projektformular mit demselben PDF-Weg'], 'errors':errors},ensure_ascii=False,indent=2)+'\n')
print('PDF-Livevorschau: 6 Prüfgruppen mit echten PDF-Renderings erfolgreich')
