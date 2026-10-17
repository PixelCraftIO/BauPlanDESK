"""UI regression suite. Start a built frontend on http://127.0.0.1:1430.
Requires Python Playwright and Chromium; test IPC is injected before app startup.
"""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich, register, gliederung
root=Path(__file__).resolve().parent
out=Path(os.environ.get('BAUPROJEKT_UI_ARTEFAKTE',str(root.parents[2]/'docs/ui-umsetzung')))
out.mkdir(parents=True,exist_ok=True)
fixture=json.loads((root/'fixtures/projekt.json').read_text());fixture['name']='Wohnbebauung Lindenweg'
errors=[];checks=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=browser.new_page(viewport={'width':1440,'height':1000})
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(fixture)+';\n'+(root/'tauri-mock.js').read_text())
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1430'))
 def active():return page.locator('.projekt-sitzung:visible')
 def view(name):bereich(page,name)
 def tab(name):page.locator('.tab-auswahl').filter(has_text=name).click()
 def close(name):page.get_by_role('button',name=name+' schließen',exact=True).click()
 def directory():bereich(page,'Projekte')
 def open_project(name):page.locator('.projekttabelle').get_by_role('button',name=name,exact=True).click()
 page.get_by_role('heading',name='Projektverzeichnis',exact=True).wait_for()
 page.screenshot(path=str(out/'01-projektverzeichnis.png'))
 page.get_by_placeholder('Projektname oder Bundesland').fill('kein treffer')
 assert page.locator('.projekttabelle').get_by_role('button',name='Öffnen',exact=True).count()==0
 page.get_by_placeholder('Projektname oder Bundesland').fill('')
 open_project('Wohnbebauung Lindenweg');view('Rechtsgrundlagen')
 active().locator('section[aria-label="Rechtsgrundlagen"]').get_by_label('Planbezeichnung',exact=True).fill('Entwurf A')
 directory();open_project('Umbau Bürogebäude');view('Rechtsgrundlagen')
 active().locator('section[aria-label="Rechtsgrundlagen"]').get_by_label('Planbezeichnung',exact=True).fill('Entwurf B')
 tab('Wohnbebauung Lindenweg');assert active().locator('section[aria-label="Rechtsgrundlagen"]').get_by_label('Planbezeichnung',exact=True).input_value()=='Entwurf A'
 tab('Umbau Bürogebäude');assert active().locator('section[aria-label="Rechtsgrundlagen"]').get_by_label('Planbezeichnung',exact=True).input_value()=='Entwurf B'
 assert page.locator('.entwurf-punkt').count()==2
 directory();open_project('Umbau Bürogebäude');assert page.locator('.projekttab').count()==2
 checks.append('Getrennte Rechtsgrundlagen-Entwürfe, Dirty-Markierung, kein doppeltes Öffnen und keine Öffnen-Aktion bei leerem Filter.')
 close('Wohnbebauung Lindenweg');page.keyboard.press('Escape');assert page.locator('.projekttab').count()==2
 close('Wohnbebauung Lindenweg');page.get_by_role('button',name='Verwerfen und schließen',exact=True).click()
 assert active().locator('section[aria-label="Rechtsgrundlagen"]').get_by_label('Planbezeichnung',exact=True).input_value()=='Entwurf B'
 directory();open_project('Wohnbebauung Lindenweg');view('Bauantrag / XBau')
 active().locator('.bauantrag').get_by_label('Bezeichnung',exact=True).fill('Antragsentwurf A')
 tab('Umbau Bürogebäude');close('Wohnbebauung Lindenweg')
 page.evaluate('window.failSave=true')
 page.get_by_role('button',name='Speichern und schließen',exact=True).click()
 page.wait_for_function('window.calls.some(c=>c.cmd==="bauantrag_speichern")')
 expect(page.get_by_role('heading',name='Projektakte schließen?',exact=True)).not_to_be_visible()
 tab('Wohnbebauung Lindenweg');assert active().locator('.bauantrag').get_by_label('Bezeichnung',exact=True).input_value()=='Antragsentwurf A'
 page.evaluate('window.failSave=false')
 close('Wohnbebauung Lindenweg');page.get_by_role('button',name='Speichern und schließen',exact=True).click()
 page.wait_for_function('window.savedApplications[window.fixture.id]?.antrag.bezeichnung==="Antragsentwurf A"')
 expect(page.locator('.projekttab')).to_have_count(1)
 assert active().locator('section[aria-label="Rechtsgrundlagen"]').get_by_label('Planbezeichnung',exact=True).input_value()=='Entwurf B'
 checks.append('Schließen von Hintergrundakten schützt Bauantrag; fehlgeschlagenes Speichern erhält Entwürfe; Speichern/Verwerfen verändern nur Zielprojekt.')
 # A delayed save must not alter the tab selected while it is in flight.
 directory();open_project('Wohnbebauung Lindenweg');view('Rechtsgrundlagen')
 active().locator('section[aria-label="Rechtsgrundlagen"]').get_by_label('Planbezeichnung',exact=True).fill('Gespeichert A')
 page.evaluate('window.saveDelay=450')
 page.evaluate('window.emitNative("menu","anwendung.speichern")')
 tab('Umbau Bürogebäude')
 page.wait_for_function('window.testProjects[0].planungsrecht.plan_bezeichnung==="Gespeichert A"')
 assert active().locator('section[aria-label="Rechtsgrundlagen"]').get_by_label('Planbezeichnung',exact=True).input_value()=='Entwurf B'
 page.evaluate('window.saveDelay=0')
 checks.append('Verzögertes Speichern nach Projektwechsel aktualisiert ausschließlich ursprüngliche Projektakte.')
 tab('Wohnbebauung Lindenweg');view('Flurstücke und Grundbuch')
 active().get_by_label('Flurstücknummer',exact=True).first.fill('100/9')
 view('Abstandsflächen');active().get_by_text('Einfache Tiefenvorprüfung (bisheriger Rechner)',exact=True).click();active().locator('section:visible input[type=number]').fill('11')
 tab('Umbau Bürogebäude');view('Abstandsflächen');active().get_by_text('Einfache Tiefenvorprüfung (bisheriger Rechner)',exact=True).click();assert active().locator('section:visible input[type=number]').input_value()=='6.5'
 tab('Wohnbebauung Lindenweg');assert active().locator('section:visible input[type=number]').input_value()=='11'
 view('Flurstücke und Grundbuch');assert active().get_by_label('Flurstücknummer',exact=True).first.input_value()=='100/9'
 checks.append('Flurstücke und Abstandsflächen-Eingaben bleiben unabhängig über Bereichs- und Projektwechsel erhalten.')
 for notice in page.locator('#app>.banner button').all(): notice.click()
 page.screenshot(path=str(out/'02-grundstuecke-projekttabs.png'))
 for name,file in [('Projektdateien','03-dokumente.png'),('Bauantrag / XBau','04-bauantrag.png'),('Stammdaten','05-stammblatt.png')]:
  view(name);page.screenshot(path=str(out/file))
 for width in [1024,768,600]:
  page.set_viewport_size({'width':width,'height':900});assert not page.evaluate('document.documentElement.scrollWidth>innerWidth')
 page.set_viewport_size({'width':1440,'height':1000});page.emulate_media(color_scheme='dark')
 page.screenshot(path=str(out/'06-dunkel.png'))
 # Closing the native window must check all retained sessions.
 page.evaluate('window.emitNative("tauri://close-requested")')
 page.get_by_role('heading',name='Anwendung schließen?',exact=True).wait_for()
 assert not page.evaluate('!!window.testDestroyed')
 page.evaluate('window.previousCloseDialog=document.querySelector("dialog[open]")')
 page.keyboard.press('Escape')
 expect(page.get_by_role('heading',name='Anwendung schließen?',exact=True)).not_to_be_visible()
 assert page.locator('.projekttab').count()==2
 page.evaluate('window.failSave=true;window.emitNative("tauri://close-requested")')
 expect(page.get_by_role('button',name='Alle speichern und beenden',exact=True)).to_be_visible()
 # Native close events are queued. A retired dialog must not dismiss its replacement.
 page.evaluate('window.previousCloseDialog.dispatchEvent(new Event("close"))')
 expect(page.get_by_role('button',name='Alle speichern und beenden',exact=True)).to_be_visible()
 page.get_by_role('button',name='Alle speichern und beenden',exact=True).click()
 expect(page.get_by_role('heading',name='Anwendung schließen?',exact=True)).not_to_be_visible()
 assert not page.evaluate('!!window.testDestroyed')
 assert page.locator('.projekttab').count()==2
 checks.append('Beenden berücksichtigt alle offenen Akten; verspätete close-Ereignisse geschlossener Dialoge beeinflussen den neuen Dialog nicht; Abbrechen und fehlgeschlagenes Speichern erhalten Fenster und Projektentwürfe.')
 page.evaluate('window.emitNative("menu","datei.importieren")')
 page.wait_for_function('window.calls.filter(c=>c.cmd==="plugin:dialog|open").length===1')
 page.evaluate('window.emitNative("menu","datei.xbau")')
 page.wait_for_function('window.calls.filter(c=>c.cmd==="plugin:dialog|save").length===1')
 checks.append('Import- und Export-Adapter lösen nur im aktiven Projekt genau eine Aktion aus.')
 page.evaluate('window.failSave=false')
 for name in ['Wohnbebauung Lindenweg','Umbau Bürogebäude']:
  close(name)
  if page.get_by_role('heading',name='Projektakte schließen?',exact=True).is_visible():
   page.get_by_role('button',name='Verwerfen und schließen',exact=True).click()
 expect(page.locator('.projekttab')).to_have_count(0)
 expect(page.get_by_role('heading',name='Projektverzeichnis',exact=True)).to_be_visible()
 open_project('Wohnbebauung Lindenweg');view('Rechtsgrundlagen')
 assert active().locator('section[aria-label="Rechtsgrundlagen"]').get_by_label('Planbezeichnung',exact=True).input_value()=='Gespeichert A'
 checks.append('Letzte Projektakte schließen führt zum Verzeichnis; erneutes Öffnen zeigt gespeicherte Daten.')
 assert not errors,errors
 checks.append('Schmale Fenster und dunkles Theme, keine Seitenüberläufe und keine JavaScript-Fehler.')
 reader=browser.new_page(viewport={'width':1024,'height':768})
 reader.on('pageerror',lambda e:errors.append(str(e)))
 reader.add_init_script('window.fixture='+json.dumps(fixture)+';\n'+(root/'tauri-mock.js').read_text()+';window.testUser.rolle="lesen";')
 reader.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1430'))
 reader.locator('.projekttabelle').get_by_role('button',name='Wohnbebauung Lindenweg',exact=True).click()
 bereich(reader,'Rechtsgrundlagen')
 assert reader.locator('.projekt-sitzung:visible section[aria-label="Rechtsgrundlagen"]').get_by_label('Planbezeichnung',exact=True).is_disabled()
 assert reader.locator('.projekt-sitzung:visible section[aria-label="Rechtsgrundlagen"]').get_by_role('button',name='Speichern',exact=True).is_disabled()
 assert reader.get_by_role('button',name='Neues Projekt',exact=True,include_hidden=True).is_disabled()
 assert not reader.evaluate('window.calls.some(c=>c.cmd==="projekt_aktualisieren")')
 checks.append('Leserrolle behält deaktivierte Schreibaktionen und sendet keine Projektänderungen.')
 assert not errors,errors
 browser.close()
(out/'pruefung.json').write_text(json.dumps({'transport':'Tauri IPC mock; real compiled application UI','checks':checks,'errors':errors},ensure_ascii=False,indent=2)+'\n')
print(json.dumps(checks,ensure_ascii=False,indent=2))
