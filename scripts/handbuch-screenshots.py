"""29 Bildschirmaufnahmen der echten Produktionsoberfläche mit fiktiven Testdaten.

Benötigt laufenden Desktop-Webbuild und die bestehenden Storage-Testartefakte.
Siehe docs/handbuch/README.md. Keine produktiven Daten/Behördenverbindungen.
Die Testadapter werden unverändert aus den bestehenden UI-Prüfungen gelesen.
"""
from pathlib import Path
import ast,json,os,sys,base64
from playwright.sync_api import sync_playwright,expect
repo=Path(__file__).resolve().parents[1]; tests=repo/'apps/desktop/tests';sys.path.insert(0,str(tests))
from navigation_hilfe import bereich,register,verfahrensbereich
out=Path(os.environ.get('BAUPROJEKT_HANDBUCH_AUFNAHMEN','/tmp/bauplan-handbuch-aufnahmen'));out.mkdir(parents=True,exist_ok=True)
manifest=[]
def adapter(name):
 t=ast.parse((tests/name).read_text())
 return next(ast.literal_eval(n.value) for n in t.body if isinstance(n,ast.Assign) and any(isinstance(a,ast.Name) and a.id=='custom' for a in n.targets))
def shot(page,name):
 page.wait_for_timeout(250)
 text=page.locator('body').inner_text()
 assert 'Unmocked command' not in text,(name,text[text.index('Unmocked command'):][:150] if 'Unmocked command' in text else '')
 assert not page.locator('[role=alert]:visible').count(),(name,page.locator('[role=alert]:visible').all_text_contents())
 page.screenshot(path=str(out/(name+'.png')))
 manifest.append(name);print(name,flush=True)
base=(tests/'tauri-mock.js').read_text()+"\n{const previous=window.__TAURI_INTERNALS__.invoke;window.__TAURI_INTERNALS__.invoke=async(c,a={})=>{if(c==='benutzer_auflisten')return [window.testUser];return previous(c,a)}}"
url=os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1465')
with sync_playwright() as p:
 b=p.chromium.launch(executable_path='/usr/bin/chromium')
 def start(fixture,extra='',values={}):
  page=b.new_page(viewport={'width':1440,'height':960},locale='de-DE',color_scheme='light')
  page.add_init_script('window.fixture='+json.dumps(fixture)+';'+''.join('window.'+k+'='+json.dumps(v)+';' for k,v in values.items())+base+extra)
  page.goto(url);page.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click();return page
 fixture=json.loads((tests/'fixtures/projekt.json').read_text());fixture['name']='Wohnhaus Lindenweg – Beispiel'
 page=start(fixture)
 page.locator('.app-werkzeuge').get_by_role('button',name='Projekte',exact=True).click();shot(page,'projekte')
 page.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click()
 for title,name in [('Stammdaten','stammdaten'),('Rechtsgrundlagen','rechtsgrundlagen'),('Bebauungsplanfestsetzungen','festsetzungen'),('Flurstücke und Grundbuch','flurstuecke'),('Gebäude / Geschosse','gebaeude'),('Projektdateien','dateien')]:
  bereich(page,title);shot(page,name)
 page.close()
 fach=json.loads((Path(os.environ.get('BAUPROJEKT_FACH_ARTEFAKTE','/tmp/bauplan-optimierung-fach'))/'workflow.json').read_text())
 page=start(fach['projekt'],adapter('fachberechnungen.py'),{'fach':fach})
 for art,title in [('stellplaetze','Stellplatznachweis'),('wohnflaechen','Wohnflächenberechnung'),('din277','DIN-277-Flächen / Rauminhalte'),('vollgeschosse','Vollgeschossprüfung'),('ueberbaubarkeit','Überbaubare Grundstücksfläche'),('freiflaechen','Versiegelung / Freiflächen'),('entwaesserung','Regenwasser / Rückhaltung')]:
  bereich(page,title);page.locator('.projekt-sitzung:visible').get_by_role('button',name='Öffnen',exact=True).click()
  if art in ['stellplaetze','wohnflaechen']:
   if art=='wohnflaechen':
    row=page.locator('details.positionszeile').first;row.locator('summary').click();row.get_by_label('Direkte Aufmaßfläche (m²)',exact=True).scroll_into_view_if_needed()
   shot(page,art+'-eingaben')
  page.get_by_role('button',name='Berechnung prüfen',exact=True).click();page.get_by_role('navigation',name='Berechnungsbereiche').get_by_role('button',name='Prüfbericht',exact=True).click()
  shot(page,art)
 page.close()
 wf=json.loads((Path(os.environ.get('BAUPROJEKT_VERFAHREN_ARTEFAKTE','/tmp/bauplan-workflow-neu/verfahren'))/'workflow.json').read_text())
 extra=adapter('verfahren.py')+'''\n{const old=window.__TAURI_INTERNALS__.invoke;window.__TAURI_INTERNALS__.invoke=async(c,a={})=>{if(c==='verfahren_optionen')return {lage:'Beispiel',optionen:[]};if(c==='verfahrenskataloge')return [];return old(c,a)}}'''
 page=start(wf['projekt'],extra,{'wf':wf});bereich(page,'Verfahrensakte');page.get_by_role('button',name='Öffnen',exact=True).click()
 for title,name in [('Unterlagen und Arbeitsaufträge','arbeitsauftraege'),('Nachforderungen','nachforderungen'),('Abgabestände','abgabestaende'),('Kommunikation','kommunikation'),('Bescheide / Auflagen','bescheide'),('Verfahrenschronik','chronik')]:
  verfahrensbereich(page,title);shot(page,name)
 bereich(page,'Projektdateien');shot(page,'dateien')
 page.close()
 wf=json.loads((Path(os.environ.get('BAUPROJEKT_ABSTAND_ARTEFAKTE','/tmp/bauplan-workflow-neu/abstand'))/'workflow.json').read_text());wf['umriss']=json.loads((Path(os.environ.get('BAUPROJEKT_ABSTAND_ARTEFAKTE','/tmp/bauplan-workflow-neu/abstand'))/'umriss.json').read_text())
 page=start(wf['projekt'],adapter('abstandsflaechen.py'),{'abstand':wf});bereich(page,'Abstandsflächen');page.get_by_role('button',name='Öffnen',exact=True).click()
 for title,name in [('Grundlagen','abstand-grundlagen'),('Wände / Bauteile','abstand-waende')]:
  page.get_by_role('navigation',name='Abstandsflächenbereiche').get_by_role('button',name=title,exact=True).click()
  if title=='Wände / Bauteile':page.get_by_role('button',name='Bearbeiten',exact=True).first.click()
  shot(page,name)
 page.get_by_role('button',name='Abstandsflächen berechnen',exact=True).click();shot(page,'abstand-ergebnis')
 page.close();b.close()

with sync_playwright() as p:
 b=p.chromium.launch(executable_path='/usr/bin/chromium')
 def start(fixture,extra='',values={}):
  page=b.new_page(viewport={'width':1440,'height':1000},locale='de-DE',color_scheme='light')
  page.add_init_script('window.fixture='+json.dumps(fixture)+';'+''.join('window.'+k+'='+json.dumps(v)+';' for k,v in values.items())+base+extra)
  page.goto(url);page.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click();return page
 wf=json.loads((Path(os.environ.get('BAUPROJEKT_PDF_LIVE_ARTEFAKTE','/tmp/bauplan-handbuch-live'))/'workflow.json').read_text())
 pdfs={n:base64.b64encode((Path(os.environ.get('BAUPROJEKT_PDF_LIVE_ARTEFAKTE','/tmp/bauplan-handbuch-live'))/(n+'.pdf')).read_bytes()).decode() for n in ['original','daniel','anna']}
 fixture=json.loads((tests/'fixtures/projekt.json').read_text());fixture['id']=wf['dokument']['project_id'];fixture['name']='Formularbeispiel Lindenweg'
 page=start(fixture,adapter('pdf-livevorschau.py'),{'wf':wf,'pdfs':pdfs})
 bereich(page,'Formulare');page.locator('.projekt-sitzung:visible').get_by_role('button',name='Ausfüllen',exact=True).click()
 d=page.get_by_role('dialog');d.get_by_label('Seite',exact=True).select_option('2')
 for field in ['Vorname','Anschrift']:d.get_by_label(field,exact=True).fill(wf['daniel'][field])
 d.get_by_label('Zustimmung',exact=True).check()
 expect(d.get_by_text('Vorschau mit Ihren aktuellen Eingaben.',exact=True)).to_be_visible()
 page.wait_for_function("window.previewCalls.at(-1)?.werte.Anschrift===window.wf.daniel.Anschrift&&window.previewCalls.at(-1)?.werte.Zustimmung===true")
 page.wait_for_timeout(500);shot(page,'formular-live');d.get_by_role('button',name='Abbrechen',exact=True).click()
 bereich(page,'Formularbibliothek');shot(page,'formularbibliothek');page.close()
 wf=json.loads((Path(os.environ.get('BAUPROJEKT_DRUCK_ARTEFAKTE','/tmp/bauplan-optimierung-druck'))/'workflow.json').read_text());wf['pdf']=base64.b64encode((Path(os.environ.get('BAUPROJEKT_DRUCK_ARTEFAKTE','/tmp/bauplan-optimierung-druck'))/'ui-einzel.pdf').read_bytes()).decode()
 page=start(wf['projekt'],adapter('druckartefakt.py'),{'workflow':wf})
 bereich(page,'Drucken / PDF-Ausgabe');page.get_by_role('button',name='Alle verfügbaren auswählen',exact=True).click();shot(page,'druckauswahl')
 page.get_by_role('button',name='Druckausgabe erstellen',exact=False).click()
 expect(page.get_by_role('heading',name='Druckvorschau · 1 Seiten',exact=True)).to_be_visible()
 page.wait_for_function("document.querySelector('canvas')?.width>0")
 shot(page,'druckvorschau');page.close()
 b.close()
