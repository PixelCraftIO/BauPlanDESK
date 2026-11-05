"""Projektbezogene Abstandsflächen im Produktionsbuild, echte Core-Berichte."""
from pathlib import Path
import os,json,base64,hashlib,re
from playwright.sync_api import sync_playwright,expect
from navigation_hilfe import bereich, register, gliederung
root=Path(__file__).resolve().parent
art=Path(os.environ.get('BAUPROJEKT_ABSTAND_ARTEFAKTE','/tmp/bauprojekt-abstand'))
wf=json.loads((art/'workflow.json').read_text(encoding='utf-8'));wf['pdf']=base64.b64encode((art/'nachweis.pdf').read_bytes()).decode()
# Echter Core-Abgleich eines Gebäudeumrisses aus dem Storage-Test.
wf['umriss']=json.loads((art/'umriss.json').read_text(encoding='utf-8'))
# Echte Übernahme der Linien „Abstandsfläche“ aus dem Lageplan (storage-Test planaufmass).
wf['linien']=json.loads((art/'linien.json').read_text(encoding='utf-8'))
wf['ansichten']=json.loads((art/'ansichten.json').read_text(encoding='utf-8'))
wf['hoehen']=json.loads((art/'hoehen.json').read_text(encoding='utf-8'))
# Regeldateien wie im Rechenkern: Inhalt plus Dateiname und SHA-256 des Dateitexts.
rw=root.parents[2]/'regelwerke/abstandsflaechen'
werke=[{'datei':str(f.relative_to(rw)),'pruefsumme':hashlib.sha256(f.read_bytes()).hexdigest(),**json.loads(f.read_text(encoding='utf-8'))} for f in sorted(rw.glob('*/*.json'))]
out=Path(os.environ.get('BAUPROJEKT_ABSTAND_UI_ARTEFAKTE',str(root.parents[2]/'docs/abstandsflaechen/screenshots')));out.mkdir(parents=True,exist_ok=True)
custom=r'''
{
const base=window.__TAURI_INTERNALS__.invoke,clone=v=>JSON.parse(JSON.stringify(v));
window.testUser=clone(window.abstand.benutzer);window.aPlan=clone(window.abstand.fachstand);window.aFail=false;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 cmd=({fachobjektseite:'fachobjekte',fachverlaufsseite:'fachverlauf',druckartefakt_erstellen:'druckausgabe_erstellen'})[cmd]??cmd;
 if(cmd==='fachobjekte'&&args.art==='planaufmass')return [window.abstand.linien.aufmass,window.abstand.ansichten.aufmass,window.abstand.hoehen.aufmass].map(l=>({daten:clone(l.daten),modul:'planaufmass',objekt:l.objekt,projekt_id:window.abstand.fachstand.projekt_id,revision:l.revision,schema_version:1}));
 if(cmd==='abstandslinien_uebernehmen'){window.calls.push({cmd,args:clone(args)});const q=[window.abstand.ansichten,window.abstand.hoehen].find(x=>x.aufmass.objekt===args.aufmass.aufmass_id)??window.abstand.linien;return clone(q.uebernahme);}
 if(['fachobjekte','fachobjekt_speichern'].includes(cmd)&&args.art!=='abstandsflaechen')return base(cmd,args);
 if(!['fachobjekte','fachobjekt_speichern','abstandsregeln','abstandsplan_pruefen','nachweis_erstellen','druckkatalog','druckausgabe_erstellen','druckausgabe_ablegen','dokumente_auflisten','abstandsumrisse_einbinden'].includes(cmd))return base(cmd,args);
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
 case 'abstandsplan_pruefen':if(args.plan.umrisse?.length)return clone(window.abstand.umriss.ergebnis);return clone((args.plan.waende[0].oberkante??[])[0]?.hoehe_m===110?window.abstand.ergebnis_geaendert:window.abstand.ergebnis);
 case 'abstandsumrisse_einbinden':return clone(window.abstand.umriss.umriss_abgleich);
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
 page.add_init_script('window.fixture='+json.dumps(wf['projekt'])+';window.abstand='+json.dumps(wf)+';window.abstandRegelwerke='+json.dumps(werke)+';'+(root/'tauri-mock.js').read_text()+custom)
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
 page.get_by_role('button',name='Nachweis erstellen …',exact=True).click()
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
 # Altplan mit Einzelwerten: ausdrücklich ins Höhenprofil übernehmen.
 page.get_by_role('button',name='In Höhenprofil übernehmen',exact=True).click()
 expect(page.get_by_role('spinbutton',name='Geländeoberfläche Station 2',exact=True)).to_have_value('10')
 page.get_by_role('spinbutton',name='Wandoberkante Höhe 1',exact=True).fill('110')
 page.evaluate('window.aFail=true');section.get_by_role('button',name='Speichern',exact=True).click()
 expect(section.get_by_role('alert')).to_contain_text('inzwischen geändert')
 expect(page.get_by_role('spinbutton',name='Wandoberkante Höhe 1',exact=True)).to_have_value('110')
 page.evaluate('window.aFail=false');section.get_by_role('button',name='Speichern',exact=True).click()
 page.get_by_role('button',name='Abstandsflächen berechnen',exact=True).click()
 # NRW: im Mittel gemessene Wandhöhe (10 m + 8 m) / 2 = 9 m, Tiefe 0,4 × 9 m = 3,6 m.
 expect(result.get_by_role('cell',name='3,6',exact=True)).to_be_visible()
 # Gebäudeumriss: Seiten werden im Core zu Wänden, Lage ist dort gesperrt.
 tab('Gebäudeumrisse')
 page.get_by_role('button',name='Gebäudeumriss hinzufügen',exact=True).click()
 for _ in range(3):page.get_by_role('button',name='Ecke hinzufügen',exact=True).click()
 page.get_by_role('button',name='Seiten als Wände übernehmen',exact=True).click()
 expect(page.get_by_role('spinbutton',name='U1 Ecke 3 Gelände',exact=True)).to_have_value('101.2')
 tab('Wände / Bauteile')
 expect(page.get_by_role('cell',name='Wohnhaus – Seite 1',exact=False).first).to_contain_text('aus Umriss')
 page.get_by_role('button',name='Bearbeiten',exact=True).first.click()
 expect(page.get_by_role('spinbutton',name='Anfang X (m)',exact=True)).not_to_be_editable()
 expect(page.get_by_role('textbox',name='Wandbezeichnung',exact=True)).to_be_editable()
 expect(page.get_by_text('aus Umriss',exact=True).first).to_be_visible()
 page.screenshot(path=str(out/'hoehenprofil.png'),full_page=True)
 page.get_by_role('button',name='Abstandsflächen berechnen',exact=True).click()
 result.get_by_text('Wohnhaus – Seite 1 – Herleitung und offene Punkte',exact=True).click()
 expect(result.get_by_role('img',name='Wandabwicklung Wohnhaus – Seite 1',exact=True)).to_be_visible()
 expect(result.get_by_text('im Mittel gemessene Wandhöhe aus den Kanten (7 m + 6,8 m) / 2 = 6,9 m',exact=False)).to_be_visible()
 page.screenshot(path=str(out/'wandabwicklung.png'),full_page=True)
 # Linien „Abstandsfläche“ aus dem Lageplan: A1/A2 als Wände, Lage gebunden.
 tab('Wände / Bauteile')
 li=page.locator('section.linien')
 expect(li.get_by_role('heading',name='Abstandsflächen aus Lageplan und Ansichten')).to_be_visible()
 li.get_by_role('combobox',name='Aufmaß').select_option(wf['linien']['aufmass']['objekt'])
 expect(li.get_by_role('combobox',name='Aufmaß')).to_contain_text('2 im Lageplan')
 li.get_by_role('button',name='Linien übernehmen',exact=True).click()
 expect(li.get_by_role('status')).to_contain_text('Linien übernommen (2 im Lageplan)')
 call=page.evaluate("window.calls.find(c=>c.cmd==='abstandslinien_uebernehmen').args")
 assert call['aufmass']['aufmass_id']==wf['linien']['aufmass']['objekt'],call['aufmass']
 a1=page.get_by_role('row').filter(has_text='A1')
 expect(a1).to_contain_text('aus Lageplan')
 a1.get_by_role('button',name='Bearbeiten',exact=True).click()
 expect(page.get_by_role('spinbutton',name='Anfang X (m)',exact=True)).not_to_be_editable()
 expect(page.get_by_role('spinbutton',name='Anfang X (m)',exact=True)).to_have_value('10')
 expect(page.get_by_text('Lage und Länge aus der Linie „A1“ im Lageplan',exact=False)).to_be_visible()
 page.screenshot(path=str(out/'linien-lageplan.png'),full_page=True)
 tab('Lageplan / Grenzen')
 expect(page.get_by_text('Grenze aus dem Lageplan (4 Eckpunkte)',exact=False)).to_be_visible()
 # Linien in einer Ansicht: A1 erhält den Wandbezug, A3 entsteht mit der Breite aus der Ansicht.
 tab('Wände / Bauteile')
 li.get_by_role('combobox',name='Aufmaß').select_option(wf['ansichten']['aufmass']['objekt'])
 expect(li.get_by_text('Ansichten: A1, A3',exact=False)).to_be_visible()
 li.get_by_role('button',name='Linien übernehmen',exact=True).click()
 expect(li.get_by_role('status')).to_contain_text('2 in Ansichten')
 expect(li.get_by_role('listitem').filter(has_text='A3: Breite aus „Ansicht Nord“')).to_be_visible()
 a3=page.get_by_role('row').filter(has_text='A3')
 expect(a3).to_contain_text('Breite aus Ansicht')
 a3.get_by_role('button',name='Bearbeiten',exact=True).click()
 expect(page.get_by_text('Wandbezug in der Ansicht: Breite 8 m',exact=False)).to_be_visible()
 expect(page.get_by_role('spinbutton',name='Ende X (m)',exact=True)).not_to_be_editable()
 expect(page.get_by_role('combobox',name='Grundstückseinheit',exact=True)).to_be_disabled()
 page.get_by_role('spinbutton',name='Abstand zur Grundstücksgrenze (m, Angabe des Planers)',exact=True).fill('4')
 expect(page.get_by_role('button',name='In der Ansicht zeigen',exact=True)).to_be_visible()
 page.screenshot(path=str(out/'linien-ansicht.png'),full_page=True)
 # Höhen aus der Ansicht: Gelände, Traufe/Attika, First, Neigung, Wandteilgrenze, Gaube.
 li.get_by_role('combobox',name='Aufmaß').select_option(wf['hoehen']['aufmass']['objekt'])
 li.get_by_role('button',name='Linien übernehmen',exact=True).click()
 expect(li.get_by_role('status')).to_contain_text('Linien übernommen')
 page.get_by_role('row').filter(has_text='A1').get_by_role('button',name='Bearbeiten',exact=True).click()
 expect(page.get_by_text('Aus der Ansicht abgegriffen: Gelände, Wandoberkante',exact=False)).to_be_visible()
 expect(page.get_by_role('spinbutton',name='Geländeoberfläche Höhe 2',exact=True)).to_have_value('100.5')
 expect(page.get_by_role('spinbutton',name='Geländeoberfläche Höhe 2',exact=True)).to_be_disabled()
 expect(page.get_by_role('spinbutton',name='Wandoberkante Höhe 3',exact=True)).to_have_value('108')
 expect(page.get_by_role('textbox',name='Zusätzliche Wandteilgrenzen (Stationen in m, mit Semikolon getrennt)',exact=True)).to_have_value('3')
 expect(page.get_by_role('textbox',name='Gelände-/Höhenherleitung und Planquelle',exact=True)).to_have_value(re.compile('Abgrabung „G3“ bei 2 m \\(99 m\\) nicht maßgebend'))
 page.get_by_text('Aus der Ansicht abgegriffen',exact=False).scroll_into_view_if_needed()
 page.screenshot(path=str(out/'hoehen-ansicht.png'),full_page=True)
 expect(page.get_by_role('button',name='Abstandsflächenplan auf dem Lageplan (PDF)',exact=True)).to_be_attached()
 # Fassungswahl: Stichtag heute schlägt die geltende NRW-Fassung vor; Wechsel nur mit Vergleich.
 tab('Grundlagen')
 fw=page.locator('section.fassung')
 expect(fw.get_by_role('heading',name='Rechtsgrundlage: Landesbauordnung und Fassung')).to_be_visible()
 expect(fw.get_by_role('combobox',name='Fassung')).to_contain_text('gilt am Stichtag')
 fw.get_by_role('button',name='Fassung übernehmen',exact=True).click()
 expect(fw.get_by_text('Regelwerk fachlich nicht freigegeben',exact=True)).to_be_visible()
 expect(fw.get_by_text('GV. NRW. S. 537',exact=False)).to_be_visible()
 expect(fw.get_by_role('heading',name='Nicht automatisch geprüfte Tatbestände',exact=False)).to_be_visible()
 fw.get_by_role('checkbox').first.check()
 fw.get_by_role('textbox',name='Vermerk zu Abs. 1',exact=True).fill('Gebäude, keine Sonderanlage')
 page.screenshot(path=str(out/'fassungswahl.png'),full_page=True)
 fw.get_by_role('combobox',name='Fassung').select_option('nw/2024-01-01.json')
 fw.get_by_role('button',name='Fassung wechseln (mit Vergleich)',exact=True).click()
 expect(fw.get_by_role('table').filter(has_text='Tiefe je Wand')).to_be_visible()
 fw.get_by_role('button',name='Wechsel bestätigen',exact=True).click()
 expect(fw.get_by_role('textbox',name='Begründung',exact=False)).to_be_visible()
 assert not errors,errors
 b.close()
print('Abstandsflächen: Projektentwurf, Konflikterhalt, Höhenprofil, Gebäudeumriss, Wandabwicklung, echte PDF-Texte und Vektorplanskizze geprüft.')
