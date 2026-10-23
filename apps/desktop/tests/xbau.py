"""XBau-Oberfläche mit echten Core-Prüfdaten; nur Tauri-Transport simuliert."""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich, register, gliederung
root=Path(__file__).resolve().parent
art=Path(os.environ.get('BAUPROJEKT_XBAU_ARTEFAKTE','/tmp/bauprojekt-xbau'))
wf=json.loads((art/'workflow.json').read_text())
out=Path(os.environ.get('BAUPROJEKT_XBAU_UI_ARTEFAKTE','/tmp/bauprojekt-xbau-ui'));out.mkdir(parents=True,exist_ok=True)
custom=r'''
const base=window.__TAURI_INTERNALS__.invoke, clone=x=>JSON.parse(JSON.stringify(x));
window.testUser=clone(window.xbauData.benutzer);window.xbauFail=false;window.savedXbau=clone(window.xbauData.nachrichten);
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 cmd=({fachobjektseite:'fachobjekte',fachverlaufsseite:'fachverlauf',druckartefakt_erstellen:'druckausgabe_erstellen'})[cmd]??cmd;
 if(cmd==='xbau_nachrichtentypen')return clone(window.xbauData.typen);
 if(cmd==='xbau_nachrichten')return args.projektId===window.fixture.id?clone(window.savedXbau):[];
 if(cmd==='dokumente_auflisten'&&args.projektId===window.fixture.id)return clone(window.xbauData.dateien);
 if(cmd==='fachobjekte'&&args.art==='verfahren'&&args.projektId===window.fixture.id)return clone(window.xbauData.verfahren);
 if(cmd==='xbau_pruefen'){window.calls.push({cmd,args});if(window.xbauFail)throw {art:'eingabe',meldung:'XBau: Testfehler bei der Schemaprüfung'};return clone(window.xbauData.pruefung);}
 if(cmd==='xbau_aufnehmen'){window.calls.push({cmd,args});return clone(window.savedXbau[0]);}
 if(cmd==='xbau_in_verfahren'){window.calls.push({cmd,args});return clone(window.xbauData.verfahren[0]);}
 if(cmd==='xbau_beleg'){window.calls.push({cmd,args});const n=window.savedXbau.find(n=>n.id===args.id);n.nachricht.belege.push(clone(args.beleg));n.revision++;return clone(n);}
 return base(cmd,args);
};
'''
errors=[]
with sync_playwright() as pw:
 browser=pw.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=browser.new_page(viewport={'width':1600,'height':1100},locale='de-DE');page.on('pageerror',lambda e:errors.append(str(e)))
 init='window.fixture='+json.dumps(wf['projekt'])+';window.xbauData='+json.dumps(wf)+';'+(root/'tauri-mock.js').read_text()+custom
 page.add_init_script(init);page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1451'))
 page.locator('.projekttabelle').get_by_role('button',name=wf['projekt']['name'],exact=True).click()
 def view(n):bereich(page,n)
 def area():return page.locator('.projekt-sitzung:visible [data-xbau]')
 view('XBau-Nachrichten');expect(area().get_by_text('Behördenanschluss: Dateiaustausch',exact=True)).to_be_visible();expect(area()).to_contain_text('Testportal');expect(area()).to_contain_text(wf['nachrichten'][0]['id'])
 page.screenshot(path=str(out/'xbau-hell.png'))
 original=wf['nachrichten'][0]['nachricht']['original'];area().get_by_label('Original im Projekt',exact=True).select_option(f"{original['dokument_id']}:{original['version']}")
 area().get_by_role('button',name='Datei und Anlagen prüfen',exact=True).click();expect(area()).to_contain_text('libxml2/xmllint');expect(area().get_by_role('button',name='Im Nachrichtenjournal erfassen',exact=True)).to_be_disabled()
 area().get_by_label('Ich habe Projekt, Richtung und Beteiligte mit dem Original abgeglichen.',exact=True).check();area().get_by_role('button',name='Im Nachrichtenjournal erfassen',exact=True).click();expect(area()).to_contain_text('Dadurch wird kein Versand bestätigt.')
 area().get_by_label('Verfahrensakte',exact=True).select_option(wf['verfahren'][0]['objekt']);area().get_by_role('button',name='Als Kommunikation übernehmen',exact=True).click();expect(area()).to_contain_text('Als Kommunikation in der Verfahrensakte erfasst.')
 # Datum der Kommunikation ist der bestätigbare Zugang, vorbelegt mit dem Erfassungstag.
 uebernahme=page.evaluate('window.calls.find(c=>c.cmd==="xbau_in_verfahren").args')
 assert uebernahme['zugang'] and len(uebernahme['zugang'])==10,uebernahme
 # Bei Prüfungsfehlern bleibt keine alte erfolgreiche Vorschau stehen.
 page.evaluate('window.xbauFail=true');area().get_by_role('button',name='Datei und Anlagen prüfen',exact=True).click();expect(area().get_by_role('alert')).to_contain_text('Testfehler');expect(area().get_by_role('button',name='Im Nachrichtenjournal erfassen',exact=True)).to_have_count(0);page.evaluate('window.xbauFail=false')
 # Entwurf ist nach Bereichswechsel erhalten und projektgebunden.
 area().get_by_label('Nachricht',exact=True).select_option(wf['nachrichten'][0]['id']);area().get_by_label('Kanal / Portal',exact=True).fill('Musterportal');area().get_by_label('Übermittlungsreferenz',exact=True).fill('PORTAL-2026-42')
 view('Stammdaten');view('XBau-Nachrichten');expect(area().get_by_label('Kanal / Portal',exact=True)).to_have_value('Musterportal')
 area().get_by_label('Datum / Uhrzeit',exact=True).fill('2026-01-03T12:00');beleg=next(d for d in wf['dateien'] if d['file_name']=='Portalbeleg.txt');area().get_by_label('Belegdatei im Projekt',exact=True).select_option(f"{beleg['document_id']}:{beleg['version']}");area().get_by_role('button',name='Beleg speichern',exact=True).click();expect(area()).to_contain_text('Beleg unveränderlich erfasst.');expect(area().get_by_label('Kanal / Portal',exact=True)).to_have_value('')
 page.evaluate("document.documentElement.dataset.theme='dark';document.documentElement.style.colorScheme='dark'");area().get_by_role('heading',name='Nachrichtenjournal',exact=True).scroll_into_view_if_needed();page.screenshot(path=str(out/'xbau-dunkel.png'))
 for width in [900,600]:
  page.set_viewport_size({'width':width,'height':1000});assert page.evaluate('document.documentElement.scrollWidth<=innerWidth')
 # Ausgänge entstehen in der Verfahrensakte; das Postfach führt dorthin.
 area().get_by_role('button',name='Zur Abgabe in der Verfahrensakte …',exact=True).click();expect(page.locator('.projekt-sitzung:visible').get_by_role('heading',name='Verfahrensakte',exact=True)).to_be_visible()
 reader=browser.new_page(viewport={'width':1440,'height':1000});reader.on('pageerror',lambda e:errors.append(str(e)));reader.add_init_script(init+';window.testUser.rolle="lesen";');reader.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1451'));reader.locator('.projekttabelle').get_by_role('button',name=wf['projekt']['name'],exact=True).click();bereich(reader,'XBau-Nachrichten');expect(reader.get_by_role('button',name='Bauantrag als Ausgang vorbereiten',exact=True)).to_have_count(0);expect(reader.get_by_role('button',name='Zur Abgabe in der Verfahrensakte …',exact=True)).to_be_enabled();expect(reader.get_by_role('button',name='Beleg speichern',exact=True)).to_have_count(0)
 assert not errors,errors;browser.close()
print('XBau: echte Prüfdaten, Importbestätigung, Fehlerzustand, Verfahrensübergabe, Belegentwurf, Hell/Dunkel, Responsivität, Weg zur Abgabe und Leserechte geprüft.')
