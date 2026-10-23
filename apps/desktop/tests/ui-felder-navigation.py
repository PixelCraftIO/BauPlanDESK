"""Bedienverträge der UI mit Test-IPC und echten Formularkonfigurationen."""
from pathlib import Path
import json,os
from playwright.sync_api import sync_playwright,expect
from navigation_hilfe import bereich as bereich_oeffnen, register, gliederung
root=Path(__file__).resolve().parent;repo=root.parents[2]
out=Path(os.environ.get('BAUPROJEKT_UI_FELDER_ARTEFAKTE','/tmp/bauprojekt-ui-felder'));out.mkdir(parents=True,exist_ok=True)
fixture=json.loads((root/'fixtures/projekt.json').read_text());fixture['name']='UI-Prüfung Musterstraße';fixture['planungsrecht']['bau_nvo']='2017'
vorlagen=[json.loads(p.read_text()) for p in (repo/'regelwerke/fachberechnungen').glob('*.json')]
for v in vorlagen:
 for f in v['felder']+[f for t in v['tabellen'] for f in t['felder']]:
  for k,val in {'optionen':[],'readonly':False,'gruppe':'','referenz':None,'groesse':None}.items():f.setdefault(k,val)
custom=r'''
// Diese Suite prüft den tatsächlichen kompakten Standard, nicht die gespeicherte
// offene Navigation der älteren Fachablauf-Suiten.
for (const p of window.testProjects)localStorage.removeItem(`bauprojekt.navigation.v1.${p.id}`);
const base=window.__TAURI_INTERNALS__.invoke;
window.savedFach={};window.fachSaveFail=false;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 cmd=({fachobjektseite:'fachobjekte',fachverlaufsseite:'fachverlauf',druckartefakt_erstellen:'druckausgabe_erstellen'})[cmd]??cmd;
 if(cmd==='berechnungsvorlagen')return window.vorlagen;
 if(cmd==='formularbibliothek')return {root:'/tmp/Formulare',vorlagen:[]};
 if(cmd==='projektordner_status')return '/tmp/Projekte';
 if(cmd==='portalprofile'||cmd==='xbau_nachrichten'||cmd==='xbau_nachrichtentypen'||cmd==='benutzer_auflisten')return [];
 if(cmd==='verfahrensprofil')return window.verfahrensprofil;
 if(cmd==='gebaeude_auswerten')return {grundflaeche_m2:null,geschossflaechensumme_m2:null,geschosse:[],offen:[]};
 if(cmd==='fachobjekte')return Object.values(window.savedFach).filter(f=>f.projekt_id===args.projektId&&f.modul===args.art);
 if(cmd==='fachobjekt_speichern'){
  window.calls.push({cmd,args:JSON.parse(JSON.stringify(args))});
  if(window.fachSaveFail)throw {art:'konflikt',meldung:'Test: Parallel geändert'};
  const f={projekt_id:args.projektId,objekt:args.id,modul:args.art,revision:args.basis+1,schema_version:1,daten:JSON.parse(JSON.stringify(args.daten))};window.savedFach[args.id]=f;return f;
 }
 if(cmd==='fachberechnung_pruefen'){window.calls.push({cmd,args});throw Error('Keine Core-Rechnung in diesem UI-Test vorgesehen');}
 return base(cmd,args);
};
'''
init='window.fixture='+json.dumps(fixture)+';window.vorlagen='+json.dumps(vorlagen)+';window.verfahrensprofil='+(repo/'workflow-regeln/verfahrensprofil.json').read_text()+';'+(root/'tauri-mock.js').read_text()+custom
errors=[]
with sync_playwright() as pw:
 browser=pw.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=browser.new_page(viewport={'width':1440,'height':1000},locale='de-DE');page.on('pageerror',lambda e:errors.append(str(e)));page.add_init_script(init)
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1460'))
 def open_project(name=fixture['name']):page.locator('.projekttabelle').get_by_role('button',name=name,exact=True).click()
 def active():return page.locator('.projekt-sitzung:visible')
 def view(name):bereich_oeffnen(page,name)
 def wc():return active().locator('[data-berechnungsmodul="wohnflaechen"]')
 open_project();expect(gliederung(page).get_by_role('button',name='Projektübersicht',exact=True)).to_have_attribute('aria-current','page')
 # Die Gliederung zeigt nur das geöffnete Register.
 expect(active().locator('.register').get_by_role('tab',name='Übersicht',exact=True)).to_have_attribute('aria-selected','true')
 expect(gliederung(page).get_by_role('button',name='Wohnflächenberechnung',exact=True)).to_have_count(0)
 view('Wohnflächenberechnung');wc().get_by_role('button',name='Neu anlegen',exact=True).click()
 expect(wc().get_by_text('Eintragsliste anzeigen',exact=True)).to_be_visible()
 assert page.locator('.projekt-sitzung:visible .projektkopf').bounding_box()['height']<90
 row=wc().locator('details.positionszeile').first;row.locator('summary').click()
 field=row.get_by_label('Direkte Aufmaßfläche (m²)',exact=True)
 expect(field).to_be_visible();expect(row.get_by_label('Länge / Höhe Trapez (m)',exact=True)).to_have_count(0)
 expect(row.get_by_label('Wohnung',exact=True)).to_have_attribute('type','text')
 check=wc().get_by_label('Aufmaß einschließlich Abzügen vollständig geprüft',exact=True)
 pos=check.bounding_box();text=check.locator('..').locator('span').bounding_box();assert abs(pos['y']-text['y'])<10 and text['x']-pos['x']<40
 field.fill('');field.press_sequentially('-2.5');expect(field).to_have_value('-2.5')
 field.fill('12');field.fill('');expect(field).to_have_value('');expect(field).to_have_attribute('aria-invalid','true')
 wc().get_by_role('button',name='Speichern',exact=True).click();expect(wc()).to_contain_text('Bitte die Eingaben prüfen.')
 wc().locator('.fehlerzusammenfassung button').filter(has_text='Direkte Aufmaßfläche').first.click();expect(field).to_be_focused()
 assert page.evaluate('window.calls.filter(c=>c.cmd==="fachobjekt_speichern").length')==0
 page.evaluate('window.emitNative("menu","anwendung.speichern")')
 assert page.evaluate('window.calls.filter(c=>c.cmd==="fachobjekt_speichern").length')==0
 wc().get_by_role('button',name='Berechnung prüfen',exact=True).click()
 assert page.evaluate('window.calls.filter(c=>c.cmd==="fachberechnung_pruefen").length')==0
 wc().get_by_role('button',name='Nachweis archivieren und PDF ausgeben …',exact=True).click()
 expect(page.get_by_role('dialog',name='Druckausgabe',exact=True)).to_have_count(0)
 assert page.evaluate('window.calls.filter(c=>["nachweis_erstellen","fachobjekt_speichern"].includes(c.cmd)).length')==0
 # Bereichswechsel und Entwurfswiederherstellung behalten die geleerte Zahl.
 view('Rechtsgrundlagen');view('Wohnflächenberechnung');expect(field).to_have_value('')
 page.reload();open_project();view('Wohnflächenberechnung');wc().get_by_role('button',name='Entwurf wiederherstellen',exact=True).click()
 row=wc().locator('details.positionszeile').first;row.locator('summary').click();field=row.get_by_label('Direkte Aufmaßfläche (m²)',exact=True);expect(field).to_have_value('')
 field.fill('0');expect(field).not_to_have_attribute('aria-invalid','true')
 wc().get_by_label('Auch derzeit nicht verwendete Geometriewerte anzeigen',exact=True).uncheck()
 row.get_by_label('Aufmaßform',exact=True).select_option('rechteck');row.get_by_label('Länge / Höhe Trapez (m)',exact=True).fill('8');row.get_by_label('Breite (m)',exact=True).fill('4')
 row.get_by_label('Aufmaßform',exact=True).select_option('aufmass');expect(row.get_by_label('Länge / Höhe Trapez (m)',exact=True)).to_have_count(0);expect(field).to_have_value('0')
 wc().get_by_role('button',name='Speichern',exact=True).click();expect(wc().get_by_text('Gespeichert.',exact=True)).to_be_visible()
 saved=page.evaluate('window.calls.filter(c=>c.cmd==="fachobjekt_speichern").at(-1).args.daten.eingaben.positionen[0]');assert saved['laenge_m']==8 and saved['breite_m']==4 and saved['aufmass_m2']==0
 field=row.get_by_label('Direkte Aufmaßfläche (m²)',exact=True);field.fill('45');page.evaluate('window.fachSaveFail=true');wc().get_by_role('button',name='Speichern',exact=True).click();expect(wc()).to_contain_text('Parallel geändert');expect(field).to_have_value('45');expect(wc().get_by_text('Gespeichert.',exact=True)).to_have_count(0)
 page.evaluate('window.fachSaveFail=false');field.fill('46');wc().get_by_role('button',name='Speichern',exact=True).click()
 # Jedes Register merkt sich je Projekt den zuletzt geöffneten Bereich.
 view('Pläne / Aufmaß');register(page,'Übersicht');register(page,'Unterlagen')
 expect(gliederung(page).get_by_role('button',name='Pläne / Aufmaß',exact=True)).to_have_attribute('aria-current','page')
 view('Projekte');open_project('Umbau Bürogebäude');register(page,'Unterlagen')
 expect(gliederung(page).get_by_role('button',name='Projektdateien',exact=True)).to_have_attribute('aria-current','page')
 page.locator('.tab-auswahl').filter(has_text=fixture['name']).click();expect(gliederung(page).get_by_role('button',name='Pläne / Aufmaß',exact=True)).to_have_attribute('aria-current','page')
 view('Bebauungsplanfestsetzungen');expect(active().get_by_role('textbox',name='Planbezeichnung',exact=True)).to_have_count(0)
 active().get_by_role('button',name='Rechtsgrundlagen bearbeiten',exact=True).click();expect(active().get_by_label('Planbezeichnung',exact=True)).to_be_visible()
 view('Projektdateien');page.screenshot(path=str(out/'dateifilter-light.png'));search=active().get_by_label('Dateien suchen',exact=True);label=active().get_by_label('Kategorie filtern',exact=True);assert label.locator('..').bounding_box()['height']>label.bounding_box()['height']+10
 view('Einreichen / Nachreichen');page.screenshot(path=str(out/'einreichen-light.png'));expect(active()).to_contain_text('5 · Eingang und Rückmeldungen dokumentieren');expect(active().get_by_role('button',name='Portalübergabe vorbereiten',exact=True)).to_be_disabled()
 active().get_by_role('button',name='Formulare ausfüllen',exact=True).click();expect(active().get_by_role('heading',name='Formulare',exact=True)).to_be_visible()
 view('Wohnflächenberechnung');row=wc().locator('details.positionszeile').first
 if row.get_attribute('open') is None:row.locator('summary').click()
 field=row.get_by_label('Direkte Aufmaßfläche (m²)',exact=True);field.scroll_into_view_if_needed()
 for theme in ['light','dark']:
  page.evaluate('(t)=>{document.documentElement.dataset.theme=t;document.documentElement.style.colorScheme=t}',theme);page.screenshot(path=str(out/f'wohnflaeche-{theme}.png'))
 for width in [1024,800,600]:
  page.set_viewport_size({'width':width,'height':850});assert page.evaluate('document.documentElement.scrollWidth<=innerWidth'),width
  assert page.locator('.aktennavigation button, .register button').evaluate_all('(es)=>es.filter(e=>e.getClientRects().length).every(e=>e.scrollWidth<=e.clientWidth+1)'),width
 page.set_viewport_size({'width':1440,'height':1000});page.evaluate("document.documentElement.dataset.theme='light';document.documentElement.style.colorScheme='light'")
 view('Gebäude / Geschosse');active().get_by_role('button',name='Neu anlegen',exact=True).click();active().get_by_role('button',name='Speichern',exact=True).click()
 geb=active().get_by_label('Gebäudebezeichnung',exact=True);expect(geb).to_have_attribute('aria-invalid','true');geb.fill('Garage');expect(geb).not_to_have_attribute('aria-invalid','true')
 page.screenshot(path=str(out/'gebaeude-light.png'))
 active().get_by_role('button',name='Speichern',exact=True).click();expect(active().locator('main > section:not([hidden])').get_by_text('Gespeichert.',exact=True)).to_be_visible()
 # Alle weiteren Berechnungsmasken können ihre konfigurierten Eingaben weiter speichern.
 page.set_viewport_size({'width':1440,'height':1000})
 for modul,titel in [('stellplaetze','Stellplatznachweis'),('din277','DIN-277-Flächen / Rauminhalte'),('vollgeschosse','Vollgeschossprüfung'),('hoehen','Gebäudehöhen / Dachneigung'),('freiflaechen','Versiegelung / Freiflächen'),('entwaesserung','Regenwasser / Rückhaltung'),('ueberbaubarkeit','Überbaubare Grundstücksfläche')]:
  view(titel);bereich=active().locator(f'[data-berechnungsmodul="{modul}"]');bereich.get_by_role('button',name='Neu anlegen',exact=True).click();bereich.get_by_role('button',name='Speichern',exact=True).click();expect(bereich.get_by_text('Gespeichert.',exact=True)).to_be_visible()
  assert page.evaluate('window.calls.filter(c=>c.cmd==="fachobjekt_speichern").at(-1).args.art')==modul
 reader=browser.new_page(viewport={'width':1440,'height':1000});reader.on('pageerror',lambda e:errors.append(str(e)));reader.add_init_script(init+';window.testUser.rolle="lesen";');reader.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1460'));reader.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click();bereich_oeffnen(reader,'Bebauungsplanfestsetzungen')
 expect(reader.locator('.projekt-sitzung:visible')).to_contain_text('Nur Leserecht');expect(reader.locator('.projekt-sitzung:visible').get_by_label('GRZ – Hauptanlagen',exact=True)).to_be_disabled()
 reader.screenshot(path=str(out/'lesemodus-light.png'))
 assert reader.locator('.projekt-sitzung:visible').get_by_label('GRZ – Hauptanlagen',exact=True).evaluate('(e)=>getComputedStyle(e).borderStyle')=='dashed'
 assert not errors,errors
 browser.close()
print('UI: Leerwert/Null, Speicher- und Rechensperre, Entwurfsrettung, Geometriewechsel, Konflikt, Checkboxen, Leserechte, Projekt-Navigation, Einreichungsführung und Hell/Dunkel geprüft.')
