"""IFC-Modell im Aufmaß: Modellakte anlegen, Struktur und Eigenschaften,
Grundriss-Schnitt je Geschoss, 3D-Ansicht (web-ifc), Modellbericht und ein
Geschoss als Hintergrund einer eigenen Zeichnung."""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich
from ifc_fixture import init_script, ifc_daten
root=Path(__file__).resolve().parent
fixture=json.loads((root/'fixtures/projekt.json').read_text(encoding='utf-8'))
out=Path(os.environ.get('BAUPROJEKT_IFC_ARTEFAKTE','/tmp/bauplan-ifc'));out.mkdir(parents=True,exist_ok=True)
ifc=ifc_daten()
eg,og=ifc['geschosse'][0],ifc['geschosse'][1]
errors=[]
with sync_playwright() as pw:
 browser=pw.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'),args=['--use-angle=swiftshader','--enable-unsafe-swiftshader'])
 page=browser.new_page(viewport={'width':1600,'height':1000},locale='de-DE')
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.on('console',lambda m:errors.append(m.text) if m.type=='error' and 'favicon' not in m.text and '404' not in m.text else None)
 page.add_init_script(init_script(fixture))
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1430'))
 page.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click()
 bereich(page,'Aufmaß')
 scope=page.locator('.projekt-sitzung:visible')
 scope.get_by_role('button',name='IFC-Modell',exact=True).click()
 expect(scope.get_by_role('tab',name='IFC-Modell')).to_have_attribute('aria-selected','true')
 wahl=scope.get_by_role('region',name='IFC-Modell wählen')
 wahl.get_by_label('IFC-Datei').select_option(label=f"{ifc['datei_name']} · V1")
 wahl.get_by_label('Bezeichnung des Modells').fill('Entwurf Architekt')
 wahl.get_by_role('button',name='Modell öffnen').click()
 expect(scope.get_by_role('tab',name='Entwurf Architekt')).to_be_visible()
 assert page.evaluate('window.ifcAkten.length')==1
 assert page.evaluate('window.ifcAkten[0].daten.dokument.dokument_id')==page.evaluate('window.ifcDok.document_id')
 struktur=scope.get_by_role('complementary',name='Modellstruktur')
 expect(struktur).to_contain_text('IFC4')
 expect(struktur).to_contain_text(eg['name'])
 # Grundriss-Schnitt des EG: Wände und Räume klickbar.
 grundriss=scope.get_by_role('application',name='Grundriss-Schnitt')
 expect(grundriss).to_be_visible()
 raum=next(e for e in ifc['auszug']['elemente'] if e['klasse']=='IFCSPACE' and e['geschoss']==eg['global_id'])
 grundriss.get_by_role('button',name=f"Raum {raum['name']}").dispatch_event('pointerup')
 details=scope.get_by_role('complementary',name='Details')
 expect(details.get_by_label('Element')).to_contain_text(raum['global_id'])
 expect(details).to_contain_text('NetFloorArea')
 page.screenshot(path=str(out/'ifc-grundriss.png'))
 # Geschoss wechseln und Schnitthöhe setzen: Core schneidet neu.
 mitte=scope.get_by_role('region',name='Modellansicht')
 mitte.get_by_label('Geschoss',exact=True).select_option(og['global_id'])
 mitte.get_by_label('Schnitthöhe').fill('1.5')
 page.wait_for_function("window.ifcAufrufe.some(a=>a.cmd==='ifc_grundriss'&&a.args.schnitthoeheM===1.5)")
 expect(scope.get_by_text('Schnitt 1,5 m über Geschossebene')).to_be_visible()
 # Modellbericht.
 details.get_by_role('tab',name='Modellbericht').click()
 bericht=details.get_by_label('Modellbericht')
 expect(bericht).to_contain_text('EPSG:25832')
 expect(bericht).to_contain_text(f"{ifc['auszug']['bericht']['elemente']} (")
 expect(details.get_by_label('Klassen')).to_contain_text('Unklassifiziertes Bauteil')
 # 3D: web-ifc lädt die Datei, Klick wählt ein Element.
 mitte.get_by_role('radio',name='3D').click()
 leinwand=scope.get_by_role('img',name='3D-Ansicht des Modells')
 expect(leinwand).to_be_visible(timeout=30000)
 expect(scope.locator('.ansicht3d .flaeche')).to_have_attribute('data-elemente','15')
 assert page.evaluate("window.ifcAufrufe.some(a=>a.cmd==='ifc_datei')")
 expect(scope.get_by_role('button',name='Draufsicht')).to_be_enabled()
 scope.get_by_role('button',name='Draufsicht').click()
 box=leinwand.bounding_box()
 page.mouse.click(box['x']+box['width']/2,box['y']+box['height']/2)
 expect(details.get_by_role('tab',name='Eigenschaften')).to_have_attribute('aria-selected','true')
 expect(details.get_by_label('Element')).to_be_visible()
 page.screenshot(path=str(out/'ifc-3d.png'))
 # Tab merkt sich die Modellakte.
 tabs=page.evaluate("Object.entries(localStorage).filter(([k])=>k.startsWith('bauprojekt.aufmasstabs.v1.')).map(([,v])=>JSON.parse(v))[0].tabs")
 assert any(t['start'].get('ifc') and t['start'].get('modell')==page.evaluate('window.ifcAkten[0].objekt') for t in tabs), tabs
 # IFC-Geschoss als Hintergrund einer eigenen Zeichnung.
 scope.get_by_role('button',name='Neue Zeichnung',exact=True).click()
 tab=scope.locator('section[aria-label="Neue Zeichnung"]:visible')
 blatt=tab.locator('details.palette').filter(has_text='Blatt und Einrichtung')
 if not blatt.evaluate('d=>d.open'):blatt.locator('summary').click()
 tab.get_by_label('Hintergrund-Unterlage').select_option(label=f"{ifc['datei_name']} · V1")
 expect(tab.get_by_label('Geschoss des IFC-Modells')).to_have_value(eg['global_id'])
 tab.get_by_label('Geschoss des IFC-Modells').select_option(og['global_id'])
 tab.get_by_role('button',name='Hintergrund hinzufügen').click()
 expect(tab.get_by_text('IFC-Grundriss eingefügt',exact=False)).to_be_visible()
 page.wait_for_function(f"window.ifcAufrufe.some(a=>a.cmd==='ifc_grundriss_plan'&&a.args.geschoss==={json.dumps(og['global_id'])})")
 expect(tab.locator('fieldset.hintergrund legend')).to_contain_text(f"{ifc['datei_name']} · {og['name']}")
 expect(tab.get_by_text('IFC-Modell in Metern')).to_be_visible()
 page.screenshot(path=str(out/'ifc-hintergrund.png'))
 assert not errors, errors
 browser.close()
print('IFC: Modellakte angelegt, Grundriss je Geschoss, 3D-Auswahl, Modellbericht, Geschoss als Zeichnungshintergrund.')
