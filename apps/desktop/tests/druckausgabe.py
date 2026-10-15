"""Produktions-UI und echte Core-PDFs: Auswahl, Einzelseiten, Druckbereich, Abbruch."""
from pathlib import Path
import base64,json,os
from playwright.sync_api import sync_playwright,expect
root=Path(__file__).resolve().parent
artefakte=Path(os.environ.get('BAUPROJEKT_DRUCK_ARTEFAKTE','/tmp/bauprojekt-druck'))
wf=json.loads((artefakte/'workflow.json').read_text())
out=Path(os.environ.get('BAUPROJEKT_DRUCK_UI_ARTEFAKTE',str(root.parents[2]/'docs/druckausgabe')));out.mkdir(parents=True,exist_ok=True)
for key,file in [('stapel','ui-stapel.pdf'),('einzel','ui-einzel.pdf'),('archiv','stapel.pdf')]:wf[key]=base64.b64encode((artefakte/file).read_bytes()).decode()
custom=r'''
const original=window.__TAURI_INTERNALS__.invoke,clone=v=>JSON.parse(JSON.stringify(v));
window.druckFail=false;window.druckDialoge=[];window.druckMode='stapel';window.saveTarget=null;
window.print=()=>{window.druckDialoge.push([...document.querySelectorAll('#bau-druckbereich .druckseite')].map(s=>({breite:s.style.width,hoehe:s.style.height,bild:s.querySelector('img')?.complete})));};
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 if(!['druckkatalog','druckausgabe_erstellen','druckausgabe_exportieren','plugin:dialog|save'].includes(cmd))return original(cmd,args);
 window.calls.push({cmd,args:clone(args)});
 if(cmd==='druckkatalog')return clone(window.druckWorkflow.katalog);
 if(cmd==='druckausgabe_erstellen'){
   if(window.druckFail)throw {art:'konflikt',meldung:'Projekt wurde geändert. Auswahl aktualisieren.'};
   return {pdf_base64:window.druckWorkflow[window.druckMode],seiten:window.druckMode==='einzel'?1:window.druckWorkflow.seiten,positionen:window.druckWorkflow.positionen};
 }
 if(cmd==='plugin:dialog|save')return window.saveTarget;
};
'''
errors=[]
with sync_playwright() as p:
    browser=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
    page=browser.new_page(viewport={'width':1680,'height':1100})
    page.on('pageerror',lambda e:errors.append(str(e)))
    page.add_init_script('window.fixture='+json.dumps(wf['projekt'])+';window.druckWorkflow='+json.dumps(wf)+';'+(root/'tauri-mock.js').read_text()+custom)
    # Separater Import derselben installierten PDF.js-Version nur zur Textprüfung.
    page.route('**/druck-pdf-test.mjs',lambda route:route.fulfill(path=str(root.parent/'node_modules/pdfjs-dist/build/pdf.mjs'),content_type='application/javascript'))
    page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1441'))
    page.locator('.projekttabelle').get_by_role('button',name=wf['projekt']['name'],exact=True).click()
    page.locator('.aktennavigation').get_by_role('button',name='Drucken / PDF-Ausgabe',exact=True).click()
    expect(page.get_by_role('heading',name='Drucken und PDF-Ausgabe')).to_be_visible()
    expect(page.get_by_role('checkbox',name='Lageplan.dwg · Version 1',exact=True)).to_be_disabled()
    page.get_by_role('button',name='Alle verfügbaren auswählen',exact=True).click()
    # Texte für den reproduzierten Core-Auftrag.
    page.get_by_role('textbox',name='Bezeichnung',exact=True).fill(wf['auftrag']['titel'])
    page.get_by_label('Absender',exact=True).fill(wf['auftrag']['absender'])
    page.get_by_label('Empfänger',exact=True).fill(wf['auftrag']['empfaenger'])
    page.get_by_label('Anschreiben',exact=True).fill(wf['auftrag']['anschreiben'])
    page.get_by_role('button',name='Druckausgabe erstellen',exact=False).click()
    expect(page.get_by_role('heading',name=f'Druckvorschau · {wf["seiten"]} Seiten')).to_be_visible(timeout=30000)
    page.wait_for_function("document.querySelector('canvas')?.width>0")
    page.get_by_role('button',name='PDF speichern …',exact=True).click()
    assert not page.evaluate("window.calls.some(c=>c.cmd==='druckausgabe_exportieren')")
    page.evaluate("window.saveTarget='C:/Export/Bauantrag.pdf'")
    page.get_by_role('button',name='PDF speichern …',exact=True).click()
    expect(page.get_by_role('status').filter(has_text='PDF gespeichert.')).to_be_visible()
    assert page.evaluate("window.calls.find(c=>c.cmd==='druckausgabe_exportieren').args.pdfBase64===window.druckWorkflow.stapel")
    page.get_by_role('button',name='Drucken …',exact=True).click()
    page.wait_for_function('window.druckDialoge.length===1',timeout=60000)
    printed=page.evaluate('window.druckDialoge[0]');assert len(printed)==wf['seiten'];assert all(s['bild'] for s in printed)
    page.emulate_media(media='print')
    assert page.locator('#bau-druckbereich').is_visible()
    assert not page.locator('.aktennavigation').is_visible()
    page.pdf(path=str(out/'systemdruck.pdf'),prefer_css_page_size=True,print_background=True)
    page.emulate_media(media='screen');page.evaluate("window.dispatchEvent(new Event('afterprint'))")
    expect(page.locator('#bau-druckbereich')).to_have_count(0)
    page.get_by_role('textbox',name='Bezeichnung',exact=True).fill('Geänderter Titel')
    expect(page.get_by_role('button',name='Drucken …',exact=True)).to_be_disabled()
    page.evaluate('window.druckFail=true')
    page.get_by_role('button',name='Druckausgabe erstellen',exact=False).click()
    expect(page.get_by_role('alert')).to_contain_text('Projekt wurde geändert')
    expect(page.get_by_role('textbox',name='Bezeichnung',exact=True)).to_have_value('Geänderter Titel')
    page.evaluate('window.druckFail=false;window.druckMode="einzel"')
    page.get_by_role('button',name='Auswahl leeren',exact=True).click()
    doc=next(e for e in wf['katalog']['eintraege'] if e['quelle']['art']=='dokument' and e['verfuegbar'])
    page.get_by_role('checkbox',name=doc['titel'],exact=True).check()
    page.get_by_role('checkbox',name='Anschreiben voranstellen',exact=True).uncheck()
    page.get_by_role('checkbox',name='Inhaltsverzeichnis mit Seitenzahlen',exact=True).uncheck()
    page.get_by_label('Seiten (leer = alle)',exact=True).fill('1')
    page.get_by_role('button',name='Druckausgabe erstellen (1)',exact=True).click()
    expect(page.get_by_role('heading',name='Druckvorschau · 1 Seiten')).to_be_visible()
    args=page.evaluate("window.calls.filter(c=>c.cmd==='druckausgabe_erstellen').at(-1).args.auftrag")
    assert args['quellen']==[{**doc['quelle'],'seiten':[1]}]
    assert not args['inhaltsverzeichnis'] and not args['mit_anschreiben']
    page.screenshot(path=str(out/'einzelausgabe.png'),full_page=True)
    # Direktzugang übernimmt genau die gewählte Berechnung ohne Vorblätter.
    page.locator('.aktennavigation').get_by_role('button',name='Baugrundstück',exact=True).click()
    page.get_by_role('button',name='Druckausgabe zusammenstellen',exact=True).click()
    dialog=page.get_by_role('dialog',name='Druckausgabe',exact=True)
    expect(dialog.get_by_role('checkbox',name='Baugrundstücksermittlung',exact=True)).to_be_checked()
    expect(dialog.get_by_role('checkbox',name='Anschreiben voranstellen',exact=True)).not_to_be_checked()
    expect(dialog.get_by_role('checkbox',name='Inhaltsverzeichnis mit Seitenzahlen',exact=True)).not_to_be_checked()
    dialog.get_by_role('button',name='Schließen',exact=True).click()
    # Archiviertes Original und alle Berichte wirklich auslesen, nicht nur Mockwerte prüfen.
    texts=page.evaluate('''async()=>{
      const pdfjs=await import('/druck-pdf-test.mjs');
      pdfjs.GlobalWorkerOptions.workerSrc=[...performance.getEntriesByType('resource')].find(e=>/pdf.worker.*mjs/.test(e.name)).name;
      const d=await pdfjs.getDocument({data:Uint8Array.from(atob(window.druckWorkflow.archiv),c=>c.charCodeAt(0)),isEvalSupported:false}).promise;
      const result=[];for(let n=1;n<=d.numPages;n++){const p=await d.getPage(n);result.push((await p.getTextContent()).items.map(i=>i.str??'').join(' '));}await d.destroy();return result;
    }''')
    assert 'Anschreiben' in texts[0] and 'Bauamt Musterstadt' in texts[0]
    assert 'Inhaltsverzeichnis' in texts[1] and 'Seite 3–4' in texts[1]
    assert 'Anlage Lageplan' in texts[2]
    assert 'Daniel Müller' in texts[3] and 'Musterstraße 7' in texts[3] and '12345 Musterstadt' in texts[3]
    assert not any('Neuere Eingabe' in t for t in texts)
    alltext=' '.join(texts)
    for t in ['GRZ I','GRZ II','GFZ','BMZ','Abstandsflächentiefe: 4 m','BauNVO: 2017','SHA-256','Nicht freigegeben']:assert t in alltext,t
    assert not page.evaluate("window.calls.some(c=>['dokument_importieren','formular_im_projekt_speichern','einreichung_erstellen'].includes(c.cmd))")
    assert not errors,errors
    browser.close()
print('Druckausgabe: Stapel, Einzelseiten, echte Feldwerte, Druckdialog, Drucklayout und Abbruch geprüft.')
