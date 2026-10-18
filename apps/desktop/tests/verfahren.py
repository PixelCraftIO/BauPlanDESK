"""Produktionsoberfläche: Verfahrensakte, Nachforderungen, Genehmigung und Suche."""
from pathlib import Path
import os,json,base64,zipfile
from playwright.sync_api import sync_playwright,expect
from navigation_hilfe import bereich, register, gliederung
root=Path(__file__).resolve().parent
art=Path(os.environ.get('BAUPROJEKT_VERFAHREN_ARTEFAKTE','/tmp/bauprojekt-verfahren'))
wf=json.loads((art/'workflow.json').read_text())
out=Path(os.environ.get('BAUPROJEKT_VERFAHREN_UI_ARTEFAKTE',str(root.parents[2]/'docs/verfahrensakte/screenshots')));out.mkdir(parents=True,exist_ok=True)
with zipfile.ZipFile(art/'akte.zip') as z:
 wf['pdfs']={f'{d["document_id"]}:{d["version"]}':base64.b64encode(z.read(f'Dokumente/{d["document_id"]}/v{d["version"]}-{d["file_name"]}')).decode() for d in wf['dateien']}
custom=r'''
{
window.testUser=window.wf.benutzer;
const original=window.__TAURI_INTERNALS__.invoke,copy=v=>JSON.parse(JSON.stringify(v));
window.procedures=[copy(window.wf.verfahren)];window.procedureFail=false;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 if(cmd==='fachobjekte'&&args.art!=='verfahren')return original(cmd,args);
 if(cmd==='fachobjekt_speichern'&&args.art!=='verfahren')return original(cmd,args);
 if(!['fachobjekte','fachobjekt_speichern','verfahren_erinnerungen','verfahren_bescheidfristen','verfahren_suche','dokumente_auflisten','dokument_historie','benutzer_auflisten','verfahrensprofil','suchtext_speichern','pdf_vorschau','verfahren_aktenexport','verfahren_export'].includes(cmd))return original(cmd,args);
 window.calls.push({cmd,args:copy(args)});
 switch(cmd){
 case 'fachobjekte':return copy(window.procedures);
 case 'fachobjekt_speichern':{
  if(window.procedureFail)throw {art:'konflikt',meldung:'Verfahrensakte inzwischen geändert. Eingaben bleiben erhalten.'};
  const f={projekt_id:args.projektId,modul:'verfahren',objekt:args.id,revision:args.basis+1,schema_version:1,daten:copy(args.daten)};
  window.procedures=[...window.procedures.filter(p=>p.objekt!==args.id),f];return copy(f);
 }
 case 'verfahren_erinnerungen':return copy(window.wf.erinnerungen);
 case 'verfahren_bescheidfristen':return copy(window.wf.fristen);
 case 'verfahren_suche':return copy(window.wf.treffer);
 case 'dokumente_auflisten':return copy(window.wf.dateien);
 case 'dokument_historie':return copy(window.wf.dateien.filter(d=>d.document_id===args.dokumentId));
 case 'benutzer_auflisten':return [copy(window.wf.benutzer)];
 case 'verfahrensprofil':return copy(window.wf.akte.profil.katalog);
 case 'pdf_vorschau':return {pdf_base64:window.wf.pdfs[args.dokumentId+':'+args.version],seiten:[{nummer:1,breite_mm:210,hoehe_mm:297}]};
 }
};
}
'''
errors=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=browser.new_page(viewport={'width':1680,'height':1100},locale='de-DE');page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(wf['projekt'])+';window.wf='+json.dumps(wf)+';'+(root/'tauri-mock.js').read_text()+custom)
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1444'))
 page.locator('.projekttabelle').get_by_role('button',name=wf['projekt']['name'],exact=True).click()
 def view(name):bereich(page,name)
 def tab(name):page.get_by_role('navigation',name='Verfahrensbereiche').get_by_role('button',name=name,exact=True).click()
 view('Verfahrensakte');page.get_by_role('button',name='Öffnen',exact=True).click()
 expect(page.get_by_role('textbox',name='Aktenzeichen',exact=True)).to_have_value('BA-2026-42')
 # Status und behördliche Vollständigkeitsbestätigung stammen aus der Akte, nicht aus freien Texten.
 expect(page.get_by_role('combobox',name='Bearbeitungsstand',exact=True).locator('option:checked')).to_have_text('Entschieden')
 expect(page.get_by_role('textbox',name='Vollständigkeit bestätigt am',exact=True)).to_have_value(wf['verfahren']['daten']['vollstaendigkeit']['datum'])
 tab('Nachforderungen');expect(page.get_by_text('0 von 3 Punkten abgeschlossen',exact=True)).to_be_visible()
 expect(page.get_by_role('textbox',name='Forderung',exact=True)).to_have_count(3)
 # Wortlaut gespeicherter Forderungspunkte ist schreibgeschützt.
 expect(page.get_by_role('textbox',name='Forderung',exact=True).first).not_to_be_editable()
 # Eine neue Antwort lässt andere Forderungspunkte unverändert; Speicherfehler erhält sie.
 antwort=page.get_by_role('textbox',name='Antwort / Begründung',exact=True).nth(2)
 antwort.fill('Ergänzender Nachweis wird vorbereitet')
 page.evaluate('window.procedureFail=true');page.get_by_role('button',name='Speichern',exact=True).click()
 expect(page.get_by_role('alert')).to_contain_text('inzwischen geändert')
 expect(antwort).to_have_value('Ergänzender Nachweis wird vorbereitet')
 page.evaluate('window.procedureFail=false');page.get_by_role('button',name='Speichern',exact=True).click()
 expect(page.get_by_role('status').filter(has_text='Gespeichert.')).to_be_visible()
 tab('Bescheide / Auflagen');expect(page.get_by_role('heading',name='Bescheide und genehmigte Unterlagen')).to_be_visible()
 expect(page.get_by_role('listitem').filter(has_text='Lageplan.pdf · Version 2')).to_be_visible()
 expect(page.get_by_role('textbox',name='Nebenbestimmung',exact=True).first).to_have_value('Nachweis vor Baubeginn')
 expect(page.get_by_role('textbox',name='Nebenbestimmung',exact=True).first).not_to_be_editable()
 # Nebenbestimmungen sind typisiert; ein Hinweis kennt keine Erfüllung.
 expect(page.get_by_role('combobox',name='Art',exact=True).first.locator('option:checked')).to_have_text('Bedingung')
 expect(page.get_by_text('Ein Hinweis begründet keine Pflicht',exact=False)).to_be_visible()
 # Fristen berechnet der Core aus Zustellung und Bundesland.
 ende=wf['fristen'][0]['rechtsbehelfsfrist']['ende']
 expect(page.get_by_role('region',name='Berechnete Fristen')).to_contain_text('Rechtsbehelfsfrist: bis '+ende)
 expect(page.get_by_role('region',name='Berechnete Fristen')).to_contain_text('Geltungsdauer der Genehmigung')
 # Ein gespeicherter Bescheid mit Zustelldatum bleibt unveränderlich.
 expect(page.get_by_text('Zustelldatum',exact=True).locator('input')).to_be_disabled()
 page.screenshot(path=str(out/'genehmigter-stand.png'))
 tab('Anhörung / Zurückstellung');expect(page.get_by_role('heading',name='Anhörung und Zurückstellung')).to_be_visible()
 expect(page.get_by_role('button',name='Verfahrensschritt erfassen',exact=True)).to_be_disabled()
 tab('Abgabestände');expect(page.locator('summary').filter(has_text=wf['akte']['abgaben'][0]['titel']+' · eingegangen')).to_be_visible();expect(page.get_by_role('button',name='PDF exportieren',exact=True)).to_be_visible()
 expect(page.get_by_role('cell',name='eingegangen',exact=True)).to_be_visible()
 # Altakten erhalten die Einreichung außerhalb der App über einen eigenen Weg.
 expect(page.get_by_role('heading',name='Externe Einreichungen',exact=True)).to_be_visible()
 expect(page.get_by_role('button',name='Externe Einreichung nachtragen …',exact=True)).to_be_visible()
 # Frühere Versandereignisse werden in der UI vor versehentlicher Bearbeitung geschützt.
 page.get_by_text('Ereignis bearbeiten / ansehen',exact=False).first.click()
 expect(page.get_by_role('textbox',name='Notiz / Begründung',exact=True).first).to_be_disabled()
 tab('Verfahrenschronik');expect(page.get_by_role('heading',name='Verfahrenschronik',exact=True)).to_be_visible()
 view('Suche / Wiedervorlagen');expect(page.get_by_role('cell',name='Nachweis vor Baubeginn',exact=False)).to_be_visible()
 page.get_by_role('textbox',name='Suchbegriff',exact=True).fill('Nachforderung');page.get_by_role('button',name='Akte durchsuchen',exact=True).click()
 expect(page.get_by_role('cell',name='Dokumentinhalt',exact=True)).to_be_visible()
 d=next(d for d in wf['dateien'] if d['file_name']=='Lageplan.pdf' and d['version']==2)
 page.get_by_role('combobox',name='Dokumentstand',exact=True).select_option(f'{d["document_id"]}:2')
 page.get_by_role('button',name='Text aus PDF / E-Mail indexieren',exact=True).click()
 page.wait_for_function("window.calls.some(c=>c.cmd==='suchtext_speichern')")
 text=page.evaluate("window.calls.find(c=>c.cmd==='suchtext_speichern').args.text")
 assert 'Lageplan v2 korrigiert' in text,text
 expect(page.get_by_role('status').filter(has_text='Dokumentinhalt indexiert.')).to_be_visible()
 page.screenshot(path=str(out/'suche-wiedervorlagen.png'))
 assert not errors,errors
 browser.close()
print('Verfahrensakte: Teilantwort, Konflikterhalt, genehmigte Version, Versandhistorie, Wiedervorlagen und echte PDF-Textextraktion geprüft.')
