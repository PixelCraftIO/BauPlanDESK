"""Workflow-Übergänge mit kontrolliertem IPC; native Berechnungen bleiben separat.

Benötigt die Fach-/Verfahrensfixtures der bestehenden Storage-Integrationstests.
"""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich, register, verfahrensbereich

root=Path(__file__).resolve().parent
repo=root.parents[2]
fach=json.loads((Path(os.environ.get('BAUPROJEKT_FACH_ARTEFAKTE','/tmp/bauprojekt-fach'))/'workflow.json').read_text())
verfahren=json.loads((Path(os.environ.get('BAUPROJEKT_VERFAHREN_ARTEFAKTE','/tmp/bauprojekt-verfahren'))/'workflow.json').read_text())
fixture=json.loads((root/'fixtures/projekt.json').read_text())
fixture['name']='Workflowprüfung'
fixture['bundesland']='NW'
fixture['planungsrecht'].update(rechtsgrundlage='par34',bau_nvo=None,bebauungsplan_art='kein')
fixture['planungsrecht']['baugrundstuecke']['berechnungen']=[]
fixture['planungsrecht']['baugrundstuecke']['flurstuecke']=fixture['planungsrecht']['baugrundstuecke']['flurstuecke'][:1]
rules=[json.loads((repo/'regelwerke/abstandsflaechen'/f).read_text())['regeln'] for f in ('nw/2026-09-01.json','by/pilot.json','bw/pilot.json')]
out=Path(os.environ.get('BAUPROJEKT_WORKFLOW_UI_ARTEFAKTE','/tmp/bauplan-workflow-ui'));out.mkdir(parents=True,exist_ok=True)
custom=r'''
const base=window.__TAURI_INTERNALS__.invoke,clone=x=>JSON.parse(JSON.stringify(x));
window.auditSave=[];window.auditFail=false;
const wohn=clone(window.fach.staende.wohnflaechen.fachstand);
wohn.projekt_id=window.fixture.id;wohn.daten.regelkontext.anwendbarkeit_bestaetigt=false;
const rp=wohn.daten.eingaben.positionen[0];rp.id='raum1';rp.quelle='Raumbuch Test';
wohn.daten.eingaben.positionen.push({...clone(rp),id:'raum3',raum:'Entfallener Raum'});
const ver=clone(window.verfahren.verfahren);ver.projekt_id=window.fixture.id;
const doks=clone(window.verfahren.dateien).map(d=>({...d,project_id:window.fixture.id}));
window.auditDaten={wohnflaechen:wohn,verfahren:ver,raumbuch:{...clone(wohn),objekt:'22222222-2222-4222-8222-222222222222',modul:'raumbuch',daten:{bezeichnung:'Raumbuch Test',gebaeude_id:null,raeume:[]}}};
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 cmd=({fachobjektseite:'fachobjekte',fachverlaufsseite:'fachverlauf'})[cmd]??cmd;
 if(cmd==='plugin:dialog|open' && window.auditImportPath)return window.auditImportPath;
 if(cmd==='abstandsregeln')return clone(window.rules);
 if(cmd==='berechnungsvorlagen')return clone(window.fach.vorlagen);
 if(cmd==='formularbibliothek')return {root:'/tmp/Formulare',vorlagen:[]};
 if(cmd==='grundstueckskontext')return clone(window.fach.grundstuecke);
 if(cmd==='dokumente_auflisten')return clone(doks);
 if(cmd==='benutzer_auflisten')return [clone(window.testUser)];
 if(cmd==='verfahren_optionen')return {lage:'Prüfung',optionen:[]};
 if(cmd==='verfahren_planunterlagen'||cmd==='verfahrenskataloge')return [];
 if(cmd==='verfahrensprofil')return clone(window.verfahren.akte.profil.katalog);
 if(cmd==='fachobjekte')return window.auditDaten[args.art]?[clone(window.auditDaten[args.art])]:[];
 if(cmd==='fachobjekt_lesen')return Object.values(window.auditDaten).find(d=>d.objekt===args.id)??null;
 if(cmd==='fachobjekt_speichern'){
  window.auditSave.push(clone(args));
  if(window.auditFail)throw {art:'konflikt',meldung:'Parallel geändert; Entwurf bleibt erhalten.'};
  const alt=window.auditDaten[args.art],daten=clone(args.daten);
  if(args.art==='wohnflaechen'&&alt){const v=clone(daten);v.regelkontext.anwendbarkeit_bestaetigt=alt.daten.regelkontext.anwendbarkeit_bestaetigt;if(JSON.stringify(v)!==JSON.stringify(alt.daten))daten.regelkontext.anwendbarkeit_bestaetigt=false;}
  const f={projekt_id:args.projektId,objekt:args.id,modul:args.art,revision:args.basis+1,schema_version:1,daten};window.auditDaten[args.art]=f;return clone(f);
 }
 if(cmd==='berechnung_aus_raumbuch'){
  const p=clone(window.auditDaten.wohnflaechen.daten),r=p.eingaben.positionen[0];
  p.eingaben.positionen=[{...clone(r),aufmass_m2:123},{...clone(r),id:'raum2',raum:'Neuer Raum',aufmass_m2:20}];return p;
 }
 if(cmd==='fachberechnung_pruefen'){window.calls.push({cmd,args:clone(args)});return {plan:clone(args.plan),ergebnis:clone(window.fach.staende.wohnflaechen.ergebnis)};}
 if(cmd==='baugrundstuecke_ermitteln')return args.plan.flurstuecke.map(f=>({schluessel:f.id,bezeichnung:f.nummer,flurstueck_ids:[f.id],eigentumsflaeche_m2:f.flaeche_m2,abzuege_m2:0,bauland_m2:f.flaeche_m2}));
 if(cmd==='fachobjekte_raumbuch')return [];
 if(cmd==='nachweise')return [];
 if(cmd==='fachobjekte_unused')return [];
 return base(cmd,args);
};
'''
errors=[]
with sync_playwright() as pw:
 b=pw.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 def start():
  page=b.new_page(viewport={'width':1440,'height':1000},locale='de-DE');page.on('pageerror',lambda e:errors.append(str(e)))
  page.add_init_script('window.fixture='+json.dumps(fixture)+';window.fach='+json.dumps(fach)+';window.verfahren='+json.dumps(verfahren)+';window.rules='+json.dumps(rules)+';'+(root/'tauri-mock.js').read_text()+custom)
  page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1430'))
  page.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click()
  return page
 page=start();a=page.locator('.projekt-sitzung:visible')
 register(page,'Nachweis und Einreichung');nav=a.locator('.aktennavigation')
 expect(nav.get_by_role('button',name='GRZ',exact=True)).to_be_disabled()
 nav.get_by_role('button',name='GRZ',exact=True).hover(force=True)
 tip=page.get_by_role('tooltip').filter(has_text='Berechnungsmaßstab')
 expect(tip).to_be_visible();assert tip.bounding_box()['x']+tip.bounding_box()['width']<=1440
 info=nav.get_by_role('button',name='GRZ: Voraussetzungen anzeigen',exact=True)
 for theme,scale in [('light',1),('dark',1.5)]:
  page.evaluate('([t,s])=>{document.documentElement.dataset.theme=t;document.documentElement.style.colorScheme=t;document.documentElement.style.zoom=String(s)}',[theme,scale])
  info.click();expect(tip).to_be_visible()
  box=tip.bounding_box();assert box['x']>=0 and box['x']+box['width']<=1441,(theme,scale,box)
  page.screenshot(path=str(out/f'00-sperrgrund-{theme}.png'))
 page.evaluate("document.documentElement.style.zoom='1';document.documentElement.dataset.theme='light';document.documentElement.style.colorScheme='light'")
 info.focus();info.press('Escape');expect(tip).not_to_be_visible()
 nav.get_by_role('button',name='GRZ: Entwurf öffnen',exact=True).click()
 expect(a.get_by_role('heading',name='GRZ-Berechnung',exact=True)).to_be_visible()
 a.get_by_role('button',name='Bauteil hinzufügen',exact=True).click()
 a.locator('.flaechenformular').get_by_label('Bezeichnung',exact=True).fill('Haus vorläufig')
 a.get_by_label('Grundfläche (m²)',exact=True).fill('120')
 expect(a.get_by_role('button',name='Berechnen',exact=True)).to_be_disabled()
 a.get_by_role('button',name='Entwurf auf diesem Gerät sichern',exact=True).click()
 expect(a.get_by_role('status').filter(has_text='Entwurf auf diesem Gerät gesichert')).to_be_visible()
 assert not page.evaluate("window.calls.some(c=>c.cmd==='nachweis_erstellen'||c.cmd==='projekt_baugrundstuecke_pruefen')")
 page.screenshot(path=str(out/'01-grz-entwurf.png'))
 page.reload();page.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click();register(page,'Nachweis und Einreichung')
 a=page.locator('.projekt-sitzung:visible');a.locator('.aktennavigation').get_by_role('button',name='GRZ: Entwurf öffnen',exact=True).click()
 a.get_by_role('button',name='Lokalen Entwurf wiederherstellen',exact=True).click()
 expect(a.locator('.flaechenformular').get_by_label('Bezeichnung',exact=True)).to_have_value('Haus vorläufig')
 page.close()

 page=start();a=page.locator('.projekt-sitzung:visible');bereich(page,'Wohnflächenberechnung')
 a.get_by_role('button',name='Öffnen',exact=True).click()
 row=a.locator('details.positionszeile').first;row.locator('summary').click();row.get_by_label('Direkte Aufmaßfläche (m²)',exact=True).fill('110')
 a.get_by_role('button',name='Gespeicherte Grundlagen bestätigen …',exact=True).click()
 page.wait_for_function('window.auditSave.length===2')
 assert page.evaluate('window.auditSave[1].daten.regelkontext.anwendbarkeit_bestaetigt')
 assert page.evaluate('window.auditDaten.wohnflaechen.daten.regelkontext.anwendbarkeit_bestaetigt')
 a.get_by_label('Positionen als Tabelle bearbeiten',exact=True).check()
 expect(a.locator('.positionsliste table').first).to_be_visible();expect(a.locator('.positionsliste table').get_by_label('Direkte Aufmaßfläche (m²)',exact=True).first).to_have_value('110')
 a.get_by_label('Positionen als Tabelle bearbeiten',exact=True).uncheck()
 a.get_by_text('Raumfakten aus dem Raumbuch übernehmen',exact=True).click()
 a.get_by_label('Raumbuch',exact=True).select_option('22222222-2222-4222-8222-222222222222')
 a.get_by_role('button',name='Raumfakten übernehmen / aktualisieren',exact=True).click()
 dlg=page.get_by_role('dialog',name='Raumfakten vor Übernahme vergleichen',exact=True)
 expect(dlg).to_be_visible();expect(dlg).to_contain_text('Entfällt');expect(dlg).to_contain_text('Neuer Raum')
 assert a.locator('details.positionszeile').count()==2
 dlg.get_by_role('button',name='Vergleich bestätigen und Entwurf übernehmen',exact=True).click()
 expect(a.locator('details.positionszeile').filter(has_text='Neuer Raum')).to_have_count(1)
 a.get_by_role('button',name='Letzte Raumübernahme rückgängig machen',exact=True).click()
 expect(a.locator('details.positionszeile').filter(has_text='Entfallener Raum')).to_have_count(1)
 page.screenshot(path=str(out/'02-raumvergleich.png'));page.close()

 page=start();a=page.locator('.projekt-sitzung:visible');bereich(page,'Verfahrensakte');a.get_by_role('button',name='Öffnen',exact=True).click()
 verfahrensbereich(page,'Unterlagen und Arbeitsaufträge')
 expect(a.get_by_role('region',name='Verfahrensbezogener Arbeitsplatz')).to_be_visible()
 verfahrensbereich(page,'Anforderungen')
 req=a.locator('details[data-uebersicht-eintrag]').first
 if req.get_attribute('open') is None:req.locator('summary').click()
 req.get_by_role('button',name='Datei importieren oder auswählen …',exact=True).click()
 expect(a.get_by_text('Unterlage für:',exact=False)).to_be_visible()
 a.get_by_role('button',name='Für Anforderung auswählen',exact=True).first.click()
 expect(a.get_by_role('button',name='Version dieser Anforderung zuordnen',exact=True)).to_be_visible()
 a.get_by_role('button',name='Version dieser Anforderung zuordnen',exact=True).click()
 expect(a.get_by_role('button',name='Version dieser Anforderung zuordnen',exact=True)).to_have_count(0)
 expect(req.get_by_role('checkbox',name='Fachlich freigegeben (Projektleitung)',exact=True)).not_to_be_checked()
 page.screenshot(path=str(out/'03-anforderung.png'))
 bereich(page,'Projektdateien')
 page.evaluate("const ds=window.verfahren.dateien;const d=ds.find(d=>ds.filter(x=>x.file_name===d.file_name).length===1);if(!d)throw Error('Fixture braucht eindeutigen Dateinamen');window.auditImportPath='/tmp/'+d.file_name")
 a.get_by_role('button',name='Dateien importieren …',exact=True).click()
 entscheidung=page.get_by_role('dialog',name='Datei oder neue Version',exact=True)
 expect(entscheidung).to_be_visible();entscheidung.get_by_role('button',name='Abbrechen',exact=True).click()
 expect(entscheidung).to_have_count(0)
 assert not page.evaluate("window.calls.some(c=>c.cmd==='dokument_importieren')")
 page.close()
 page=start();a=page.locator('.projekt-sitzung:visible');bereich(page,'Abstandsflächen')
 a.get_by_role('button',name='Neu anlegen',exact=True).click()
 expect(a.get_by_role('combobox',name='Baugebiet',exact=True)).to_have_value('')
 page.screenshot(path=str(out/'04-abstand-grundlagen.png'))
 a.get_by_role('navigation',name='Abstandsflächenbereiche').get_by_role('button',name='Wände / Bauteile',exact=True).click()
 a.get_by_role('button',name='Wand hinzufügen',exact=True).click()
 a.get_by_role('spinbutton',name='Ende X (m)',exact=True).fill('10')
 a.get_by_role('spinbutton',name='Ende Y (m)',exact=True).fill('10')
 expect(a.get_by_role('button',name='W1: Wand 1 auswählen',exact=True)).to_be_visible()
 a.get_by_role('button',name='W1: Wand 1 auswählen',exact=True).focus()
 page.keyboard.press('Enter')
 page.screenshot(path=str(out/'05-abstand-wand.png'))
 for theme in ['light','dark']:
  page.evaluate('(t)=>{document.documentElement.dataset.theme=t;document.documentElement.style.colorScheme=t}',theme)
  for scale in [1,1.5]:
   page.evaluate('(s)=>document.documentElement.style.zoom=String(s)',scale)
   assert page.evaluate('document.documentElement.scrollWidth<=document.documentElement.clientWidth+1'),(theme,scale)
   page.screenshot(path=str(out/f'06-abstand-{theme}-{scale}.png'))
 page.close()
 assert not errors,errors
 b.close()
print('Workflow-UX: erreichbarer Sperrgrund, §34-Entwurf/IDB-Rettung ohne Nachweis, Regelbestätigung nach Speicherung, Tabellenerfassung, Raumvergleich/Rückgängig und explizite Dokumentzuordnung geprüft.')
