"""Eigene Zeichnung in Metern: anlegen, Wand und Maßkette zeichnen, Rechteck messen,
PDF als Hintergrund einpassen, speichern ohne Unterlage, wieder öffnen."""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich
from plan_fixture import pdf_fixture, custom
root=Path(__file__).resolve().parent
fixture=json.loads((root/'fixtures/projekt.json').read_text(encoding='utf-8'))
out=Path(os.environ.get('BAUPROJEKT_ZEICHNUNG_ARTEFAKTE','/tmp/bauplan-zeichnung'));out.mkdir(parents=True,exist_ok=True)
errors=[]
with sync_playwright() as pw:
 browser=pw.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=browser.new_page(viewport={'width':1600,'height':1000},locale='de-DE')
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.on('console',lambda m:errors.append(m.text) if m.type=='error' and 'favicon' not in m.text and '404' not in m.text else None)
 page.add_init_script('window.fixture='+json.dumps(fixture)+';window.planPdf='+json.dumps(pdf_fixture())+';'+(root/'tauri-mock.js').read_text(encoding='utf-8')+custom)
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1430'))
 page.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click()
 bereich(page,'Aufmaß')
 scope=page.locator('.projekt-sitzung:visible')
 scope.get_by_role('button',name='Neue Zeichnung',exact=True).click()
 tab=scope.locator('section[aria-label="Neue Zeichnung"]:visible')
 svg=scope.get_by_role('application');expect(svg).to_be_visible()
 expect(scope.get_by_role('tab',name='Neue Zeichnung')).to_have_attribute('aria-selected','true')
 expect(tab.get_by_label('Arbeitsbereich')).to_contain_text('Eigene Zeichnung (Meter)')
 expect(tab.get_by_role('status').filter(has_text='Eigene Zeichnung in Metern')).to_be_visible()
 # Kalibrieren ist gesperrt; Wand, Maßkette, Text sind frei.
 expect(tab.get_by_role('button',name='Zwei Punkte kalibrieren').first).to_be_disabled()
 for name in ['Wand','Maßkette','Text']:expect(tab.get_by_role('button',name=name,exact=True)).to_be_enabled()
 def click(x,y):
  q=svg.evaluate("""async(svg,p)=>{let q=new DOMPoint(p[0],p[1]).matrixTransform(svg.getScreenCTM());
    const s=svg.closest('.plan-scroll'),r=s.getBoundingClientRect();s.scrollLeft+=q.x-r.left-r.width/2;s.scrollTop+=q.y-r.top-r.height/2;
    await new Promise(requestAnimationFrame);q=new DOMPoint(p[0],p[1]).matrixTransform(svg.getScreenCTM());return {x:q.x,y:q.y};}""",[x,y])
  page.mouse.click(q['x'],q['y'])
 tab.get_by_label('Punktfang',exact=True).uncheck()
 # Wand 10 m lang, Maßkette darüber, Rechteck 12,5 × 8 m als Markierung (Mock liefert 50 m²).
 tab.get_by_role('button',name='Wand',exact=True).click();click(5,5);click(15,5)
 expect(tab.get_by_role('group',name='Zeichenelement')).to_contain_text('Wand')
 expect(tab.get_by_role('group',name='Zeichenelement')).to_contain_text('10 m')
 tab.get_by_role('button',name='Maßkette',exact=True).click();click(5,12);click(15,12)
 expect(svg.locator('.element .masstext')).to_contain_text('10 m')
 tab.get_by_role('button',name='Text',exact=True).click();click(6,8)
 tab.get_by_role('group',name='Zeichenelement').get_by_label('Text',exact=True).fill('Wohnen')
 expect(svg.locator('.element .freitext')).to_have_text('Wohnen')
 tab.get_by_role('button',name='Rechteck',exact=True).click();click(5,15);click(17.5,23)
 expect(tab.get_by_label('Bezeichnung',exact=True).first).to_have_value('Markierung 1')
 expect(tab.locator('.messwerte')).to_contain_text('50 m²')
 # Werkzeugkiste am Polygon: ein Klick wählt Polygon, Bedeutung NUF, Namen und legt die Ebene „DIN 277“ an.
 tab.get_by_role('button',name='Vorlagen für Polygon',exact=True).click()
 tab.get_by_role('menu',name='Vorlagen Polygon').get_by_role('menuitem',name='DIN 277 NUF').first.click()
 expect(tab.get_by_role('menu')).to_have_count(0)
 expect(tab.get_by_role('status').filter(has_text='Vorlage DIN 277 NUF')).to_be_visible()
 expect(tab.get_by_role('button',name='Polygon',exact=True)).to_have_attribute('aria-pressed','true')
 click(20,5);click(26,5);click(26,9);click(20,9);svg.press('Enter')
 expect(tab.get_by_label('Bezeichnung',exact=True).first).to_have_value('DIN 277 NUF 1')
 expect(tab.get_by_label('Bedeutung',exact=True)).to_have_value('nuf')
 expect(tab.get_by_label('Zeichenebene',exact=True)).to_have_value(tab.get_by_label('Zeichenebene',exact=True).locator('option',has_text='DIN 277').evaluate('o=>o.value'))
 expect(tab.get_by_role('status').filter(has_text='Vorlage DIN 277 NUF')).to_have_count(0)
 # Linie „Abstandsfläche“: Namen A1, A2 … für den Abstandsflächennachweis.
 for n,y in enumerate([11,13],start=1):
  tab.get_by_role('button',name='Vorlagen für Strecke / Dach',exact=True).click()
  tab.get_by_role('menu',name='Vorlagen Strecke / Dach').get_by_role('menuitem',name='Abstandsfläche').first.click()
  click(20,y);click(26,y)
  expect(tab.get_by_label('Bezeichnung',exact=True).first).to_have_value(f'A{n}')
  expect(tab.get_by_label('Bedeutung',exact=True)).to_have_value('abstandsflaeche')
 # Versatz als Werkzeug: Auswahl bleibt, Abstand in der Optionsleiste, danach rückgängig.
 tab.get_by_role('button',name='Versatz',exact=True).click()
 expect(tab.get_by_role('button',name='Versatz',exact=True)).to_have_attribute('aria-pressed','true')
 tab.get_by_label('Abstand der Parallele',exact=True).fill('1')
 tab.get_by_role('button',name='Parallele anlegen',exact=True).click()
 expect(tab.get_by_text('Versatz: 1 Kopie(n) angelegt',exact=False)).to_be_visible()
 svg.press('Control+z')
 # Escape schließt zuerst nur die Fangoptionen; das Werkzeug bleibt aktiv.
 tab.get_by_role('button',name='Fangoptionen ▾',exact=True).click()
 expect(tab.get_by_role('group',name='Fangoptionen')).to_be_visible()
 page.keyboard.press('Escape')
 expect(tab.get_by_role('group',name='Fangoptionen')).to_have_count(0)
 expect(tab.get_by_role('button',name='Versatz',exact=True)).to_have_attribute('aria-pressed','true')
 # Menü „Ausgabe“ schließt bei Klick daneben.
 tab.get_by_role('button',name='Ausgabe ▾',exact=True).click()
 expect(tab.get_by_role('menu',name='Ausgabe')).to_be_visible()
 tab.get_by_role('tab',name='Objekte',exact=True).click()
 expect(tab.get_by_role('menu',name='Ausgabe')).to_have_count(0)
 svg.press('Escape')
 expect(tab.get_by_role('button',name='Auswählen',exact=True)).to_have_attribute('aria-pressed','true')
 # Hintergrund: PDF-Seite 1 im Maßstab 1:100, über Strecke einpassen, verschieben.
 tab.get_by_role('tab',name='Eigenschaften',exact=True).click()
 # Nach dem Rückgängigmachen ist nichts mehr gewählt: Eigenschaften zeigt direkt das Blatt.
 expect(tab.locator('section.palette[aria-label="Blatt und Einrichtung"]')).to_be_visible()
 tab.get_by_label('Hintergrund-Unterlage',exact=True).select_option(label='Grundrisse.pdf · V1')
 tab.get_by_role('button',name='Hintergrund hinzufügen',exact=True).click()
 expect(svg.locator('.hintergrund image')).to_have_count(1)
 hg=tab.locator('fieldset.hintergrund')
 expect(hg).to_contain_text('Grundrisse.pdf · Seite 1')
 expect(hg.get_by_label('Maßstab 1 :',exact=True)).to_have_value('100')
 hg.get_by_role('button',name='Verschieben',exact=True).click();click(2,30);click(4,32)
 expect(hg.get_by_label('Lage X (m)',exact=True)).to_have_value('2')
 expect(hg.get_by_label('Lage Y (m)',exact=True)).to_have_value('2')
 # Strecke ab der Blattecke des Hintergrunds: der Ankerpunkt bleibt liegen, der Maßstab halbiert sich.
 hg.get_by_label('Strecke (m)',exact=True).fill('20')
 hg.get_by_role('button',name='Über Strecke einpassen',exact=True).click()
 expect(tab.get_by_role('status').filter(has_text='bekannten Strecke (20 m)')).to_be_visible()
 click(2,2);click(12,2)
 expect(hg.get_by_label('Maßstab 1 :',exact=True)).to_have_value('200')
 expect(hg.get_by_label('Lage X (m)',exact=True)).to_have_value('2')
 page.wait_for_timeout(300);page.screenshot(path=str(out/'zeichnung.png'))
 # Speichern ohne Unterlage; Diagnose lief mit dem Zeichnungsvertrag.
 tab.get_by_role('button',name='Speichern',exact=True).click()
 expect(scope.get_by_label('Speicherstatus',exact=True).first).to_contain_text('Gespeichert')
 saved=page.evaluate('window.planSaved.daten')
 assert 'dokument' not in saved,saved
 assert saved['kalibrierung']=={'art':'einheit','einheit':'meter'},saved['kalibrierung']
 assert [e['art'] for e in saved['elemente']]==['wand','masskette','text'],saved['elemente']
 assert abs(saved['elemente'][0]['dicke_m']-0.24)<1e-9
 assert saved['elemente'][2]['inhalt']=='Wohnen'
 assert len(saved['hintergruende'])==1 and saved['hintergruende'][0]['seite']==1
 assert abs(saved['hintergruende'][0]['meter_pro_punkt']-200*0.0254/72)<1e-9,saved['hintergruende'][0]
 g=saved['markierungen'][0]['geometrie']
 assert g['art']=='rechteck' and all(abs(a-b)<1e-6 for a,b in zip(g['von']+g['bis'],[5,15,17.5,23])),g
 assert saved['markierungen'][1]['bezeichnung']=='DIN 277 NUF 1' and saved['markierungen'][1]['bedeutung']=='nuf',saved['markierungen'][1]
 assert any(e['bezeichnung']=='DIN 277' for e in saved['ebenen']),saved['ebenen']
 # Wieder öffnen aus der Liste: Tab heißt wie die Zeichnung, Hintergrund und Elemente sind da.
 tab.get_by_label('Bezeichnung des Aufmaßes',exact=True).fill('Skizze EG')
 tab=scope.locator('section[aria-label="Skizze EG"]:visible')
 tab.get_by_role('button',name='Speichern',exact=True).click()
 expect(scope.get_by_label('Speicherstatus',exact=True).first).to_contain_text('Gespeichert')
 scope.get_by_role('button',name='Skizze EG schließen',exact=True).click()
 scope.get_by_role('tab',name='Aufmaßliste',exact=True).click()
 liste=scope.locator('section[aria-label="Aufmaßliste"]:visible')
 liste.get_by_role('button',name='Liste aktualisieren',exact=True).click()
 liste.get_by_role('row',name='Skizze EG').get_by_role('button',name='Öffnen',exact=True).click()
 expect(scope.get_by_role('tab',name='Skizze EG')).to_have_attribute('aria-selected','true')
 tab=scope.locator('section[aria-label="Skizze EG"]:visible')
 expect(tab.locator('.element')).to_have_count(3)
 expect(tab.locator('.hintergrund image')).to_have_count(1)
 expect(tab.get_by_label('Arbeitsbereich')).to_contain_text('Eigene Zeichnung (Meter)')
 # Ein Aufmaß auf einer Unterlage kennt weiterhin Seite und Maßstab, aber keine Hintergründe.
 scope.get_by_role('tab',name='Aufmaßliste',exact=True).click()
 scope.get_by_role('button',name='Neu anlegen',exact=True).click()
 tab=scope.locator('section[aria-label="Neues Aufmaß"]:visible')
 expect(tab.get_by_label('Unterlage',exact=True)).to_be_visible()
 expect(tab.get_by_role('button',name='Hintergrund hinzufügen')).to_have_count(0)
 expect(tab.get_by_role('button',name='Maßstab setzen',exact=True)).to_be_visible()
 browser.close()
assert not errors,errors
print('zeichnung ok')
