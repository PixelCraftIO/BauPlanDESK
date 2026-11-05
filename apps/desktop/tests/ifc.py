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
 page.wait_for_function("window.ifcAufrufe.some(a=>a.cmd==='ifc_akte_grundriss'&&a.args.schnitthoeheM===1.5)")
 expect(scope.get_by_text('Schnitt 1,5 m über Geschossebene')).to_be_visible()
 # Modellbericht.
 details.get_by_role('tab',name='Modellbericht').click()
 bericht=details.get_by_label('Modellbericht')
 expect(bericht).to_contain_text('EPSG:25832')
 expect(bericht).to_contain_text(f"{ifc['auszug']['bericht']['elemente']} (")
 expect(details.get_by_label('Klassen')).to_contain_text('Unklassifiziertes Bauteil')
 # Prüfung: mitgeliefertes IDS, Befund mit Elementen; eigenes IDS aus den Projektdateien.
 details.get_by_role('tab',name='Prüfung').click()
 pruef=details.get_by_role('region',name='Modellprüfung')
 expect(pruef.get_by_role('status',name='Prüfergebnis')).to_contain_text('0 Fehler, 1 Warnungen, 0 Hinweise')
 befund=pruef.get_by_role('list',name='Befunde').locator('li.warnung')
 expect(befund).to_contain_text('Unklassifizierte Bauteile')
 befund.get_by_role('button',name='Elemente').click()
 proxy=next(e for e in ifc['auszug']['elemente'] if e['klasse']=='IFCBUILDINGELEMENTPROXY')
 befund.get_by_role('button',name=proxy['name']).click()
 details.get_by_role('tab',name='Eigenschaften').click()
 expect(details.get_by_label('Element')).to_contain_text(proxy['global_id'])
 details.get_by_role('tab',name='Prüfung').click()
 pruef.get_by_label('Regelwerk der Prüfung').select_option(label='buero.ids · V1')
 expect(pruef.get_by_role('status',name='Prüfergebnis')).to_contain_text('Büro: 1 Fehler')
 assert page.evaluate("window.ifcAufrufe.filter(a=>a.cmd==='ifc_pruefen').at(-1).args.ids.dokument_id")=='d416da8b-cbfb-4ae6-931b-8765fb6454d1'
 page.screenshot(path=str(out/'ifc-pruefung.png'))
 # Nacharbeit aus dem Befund: Proxy als Wand klassifizieren, Raum aus Wänden bilden, zurücknehmen.
 pruef.get_by_label('Regelwerk der Prüfung').select_option(index=0)
 befund=pruef.get_by_role('list',name='Befunde').locator('li.warnung')
 befund.get_by_role('button',name='Klassifizieren …').click()
 nach=details.get_by_role('region',name='Nacharbeit')
 expect(nach).to_contain_text(f"Auswahl: {proxy['name']}")
 nach.get_by_label('Neue Klasse').select_option('IFCWALL')
 nach.get_by_role('button',name='Klassifizieren',exact=True).click()
 aenderungen=nach.get_by_role('list',name='Änderungen')
 expect(aenderungen).to_contain_text(f"{proxy['name']}: Klasse Wand")
 akte=page.evaluate('window.ifcAkten[0]')
 assert akte['daten']['nacharbeit']==[{'art':'klassifizieren','global_id':proxy['global_id'],'klasse':'IFCWALL','vordefiniert':None}], akte
 nach.get_by_role('button',name='Hervorhebung aufheben').click()
 mitte.get_by_label('Geschoss',exact=True).select_option(eg['global_id'])
 nach.get_by_role('button',name='Geschlossene Flächen suchen').click()
 vorschl=nach.get_by_role('list',name='Raumvorschläge')
 expect(vorschl.locator('li')).to_have_count(len(ifc['raumvorschlaege'][eg['global_id']]))
 expect(grundriss.locator('path.zusatz')).to_have_count(1)
 vorschl.get_by_label('Name des Raums 1').fill('0.03 Bad')
 vorschl.get_by_role('button',name='Raum bilden').click()
 expect(aenderungen.locator('li')).to_have_count(2)
 neu=page.evaluate('window.ifcAkten[0].daten.nacharbeit[1]')
 assert neu['art']=='raum_bilden' and neu['name']=='0.03' and neu['langname']=='Bad' and neu['geschoss']==eg['global_id'] and len(neu['umriss'])>=4, neu
 page.screenshot(path=str(out/'ifc-nacharbeit.png'))
 aenderungen.locator('li').first.get_by_role('button',name='Zurücknehmen').click()
 expect(aenderungen.locator('li')).to_have_count(1)
 assert page.evaluate('window.ifcAkten[0].daten.nacharbeit.length')==1
 # Geometrie nacharbeiten: Wandhöhe ändern, neuen Umriss im Grundriss zeichnen.
 innen=next(e for e in ifc['auszug']['elemente'] if e['name']=='Innenwand' and e['geschoss']==eg['global_id'])
 struktur.locator('details[open] details').filter(has=page.locator('summary',has_text='Wand')).first.locator('summary').click()
 struktur.get_by_role('button',name='Innenwand',exact=True).first.click()
 details.get_by_role('tab',name='Nacharbeit').click()
 expect(nach).to_contain_text('Auswahl: Innenwand')
 expect(nach.get_by_label('Oberkante')).to_have_value(str(innen['geometrie']['oben_m']))
 nach.get_by_label('Oberkante').fill('2')
 nach.get_by_role('button',name='Höhe ändern').click()
 expect(aenderungen.locator('li')).to_have_count(2)
 expect(aenderungen).to_contain_text('Innenwand: Oberkante 2 m')
 h=page.evaluate('window.ifcAkten[0].daten.nacharbeit[1]')
 assert h=={'art':'hoehe_setzen','global_id':innen['global_id'],'unten_m':None,'oben_m':2}, h
 nach.get_by_role('button',name='Neuen Umriss zeichnen').click()
 svg=grundriss
 box=svg.bounding_box()
 for fx,fy in [(.45,.3),(.55,.3),(.55,.7)]:
     page.mouse.click(box['x']+box['width']*fx,box['y']+box['height']*fy)
 expect(nach.get_by_role('button',name='Umriss übernehmen (3 Punkte)')).to_be_enabled()
 expect(grundriss.locator('path.zusatz')).to_have_count(1)
 nach.get_by_role('button',name='Umriss übernehmen (3 Punkte)').click()
 expect(aenderungen.locator('li')).to_have_count(3)
 u=page.evaluate('window.ifcAkten[0].daten.nacharbeit[2]')
 assert u['art']=='umriss_setzen' and u['global_id']==innen['global_id'] and len(u['umriss'])==3, u
 expect(nach.get_by_role('button',name='Neuen Umriss zeichnen')).to_be_visible()
 page.screenshot(path=str(out/'ifc-geometrie.png'))
 for _ in range(2):
     aenderungen.locator('li').last.get_by_role('button',name='Zurücknehmen').click()
 expect(aenderungen.locator('li')).to_have_count(1)
 assert page.evaluate("window.ifcAufrufe.filter(a=>a.cmd==='ifc_akte_auszug').length")>=4
 # Reimport: neue Version V2 wird angeboten, abgeglichen und übernommen.
 struktur.get_by_role('button',name='Neue Version V2 vorhanden: abgleichen').click()
 re=details.get_by_role('region',name='Neue Modellversion')
 expect(re.get_by_label('Neue Modellversion')).to_have_value(page.evaluate('window.ifcDok.document_id')+'|2')
 re.get_by_role('button',name='Abgleichen').click()
 ab=ifc['abgleich_v2']
 expect(re.get_by_role('status',name='Abgleich')).to_contain_text(f"{len(ab['neu'])} neu, {len(ab['entfernt'])} entfernt, {len(ab['geaendert'])} geändert")
 expect(re.get_by_role('status',name='Abgleich')).to_contain_text('Nacharbeit: 1 wirkt weiter')
 re.get_by_role('button',name='Auf diese Version umstellen').click()
 expect(re.get_by_text('Modellakte auf die neue Version umgestellt',exact=False)).to_be_visible()
 assert page.evaluate('window.ifcAkten[0].daten.dokument.version')==2
 w=page.evaluate("window.ifcAufrufe.find(a=>a.cmd==='ifc_reimport').args.wunsch")
 assert w['ueberholte_entfernen'] is True and w['verwaiste_entfernen'] is False, w
 expect(struktur.get_by_role('button',name='Neue Version V2 vorhanden: abgleichen')).to_have_count(0)
 expect(struktur).to_contain_text('V2')
 # Auswertung: Kennwerte, Geschosse, Nutzungen, Höhen; Gebäudegrundfläche im Grundriss.
 details.get_by_role('tab',name='Auswertung').click()
 aus=details.get_by_role('region',name='Auswertung')
 kw=aus.get_by_label('Kennwerte')
 expect(kw).to_contain_text('Gebäudegrundfläche80 m²')
 expect(kw).to_contain_text('GRZ (rechnerisch)0,2')
 expect(kw).to_contain_text('GFZ (rechnerisch)0,4')
 expect(aus.get_by_label('Geschosse').locator('tbody tr')).to_have_count(2)
 expect(aus.get_by_label('Nutzungen')).to_contain_text('Wohnen')
 expect(aus.get_by_label('Höhen')).to_contain_text('Oberkante Flachdach5,6 m')
 expect(mitte.locator('path.zusatz')).to_have_count(1)
 assert page.evaluate("window.ifcAufrufe.some(a=>a.cmd==='ifc_auswerten' && a.args.grundstueckId===null)")
 page.screenshot(path=str(out/'ifc-auswertung.png'))
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
 # Übernahme ins Projekt: neues Gebäude, Zeichnungen je Geschoss, Raumbücher.
 details.get_by_role('tab',name='Übernahme').click()
 ueb=details.get_by_role('region',name='Übernahme ins Projekt')
 expect(ueb.get_by_label('Zielgebäude')).to_have_value('')
 rev=page.evaluate('window.ifcAkten[0].revision')
 ueb.get_by_role('button',name='In Projekt übernehmen').click()
 ergebnis=ueb.get_by_role('status',name='Übernahmeergebnis')
 expect(ergebnis).to_contain_text('2 Zeichnung(en), 2 Raumbuch/Raumbücher')
 aufruf=page.evaluate("window.ifcAufrufe.find(a=>a.cmd==='ifc_uebernehmen').args.wunsch")
 assert aufruf['gebaeude_id'] is None and aufruf['raumbuch'] is True and aufruf['revision']==rev, aufruf
 expect(ueb.get_by_role('button',name='Übernahme aktualisieren')).to_be_visible()
 expect(ueb).to_contain_text('Zuletzt übernommen')
 page.screenshot(path=str(out/'ifc-uebernahme.png'))
 ergebnis.get_by_role('button',name=f"Entwurf Architekt · {eg['name']}").click()
 expect(scope.get_by_role('tab',name=f"Entwurf Architekt · {eg['name']}")).to_have_attribute('aria-selected','true')
 page.wait_for_function(f"window.ifcAufrufe.some(a=>a.cmd==='ifc_grundriss_plan'&&a.args.geschoss==={json.dumps(eg['global_id'])})")
 # Zweite Übernahme nutzt die neue Revision der Modellakte.
 scope.get_by_role('tab',name='Entwurf Architekt',exact=True).click()
 ueb.get_by_role('button',name='Übernahme aktualisieren').click()
 page.wait_for_function("window.ifcAufrufe.filter(a=>a.cmd==='ifc_uebernehmen').length===2")
 expect(ueb.get_by_role('alert')).to_have_count(0)
 assert page.evaluate("window.ifcAufrufe.filter(a=>a.cmd==='ifc_uebernehmen')[1].args.wunsch.revision")==rev+1
 # Export: Datei mit Nacharbeit schreiben, eigenes IFC4-Modell aus den Zeichnungen.
 details.get_by_role('tab',name='Export').click()
 ex=details.get_by_role('region',name='Export')
 ex.get_by_role('button',name='IFC-Datei schreiben').click()
 bericht=ex.get_by_role('status',name='Exportergebnis').first
 expect(bericht).to_contain_text('EFH-Mustermann-nachgearbeitet.ifc')
 expect(bericht).to_contain_text('Selbstkontrolle bestanden')
 aufruf=page.evaluate("window.ifcAufrufe.find(a=>a.cmd==='ifc_exportieren').args.wunsch")
 assert aufruf=={'art':'nacharbeit','akte_id':page.evaluate('window.ifcAkten[0].objekt')}, aufruf
 bericht.get_by_role('button',name='Dem Bauantrag beifügen').click()
 expect(bericht.get_by_role('status').filter(has_text='BIM-Gebäudemodell')).to_be_visible()
 ex.get_by_role('button',name='Modelldatei dem Bauantrag beifügen').click()
 expect(ex.get_by_text('Liegt dem Bauantrag schon bei.')).to_be_visible()
 aufrufe=page.evaluate("window.ifcAufrufe.filter(a=>a.cmd==='ifc_dem_bauantrag_beifuegen').map(a=>a.args)")
 assert [a['dokumentId'] for a in aufrufe]==['e416da8b-cbfb-4ae6-931b-8765fb6454e1',page.evaluate('window.ifcDok.document_id')], aufrufe
 ent=ex.get_by_role('region',name='Eigenes IFC4-Modell')
 expect(ent.get_by_label('Gebäude für das Modell')).not_to_have_value('')
 zeichnung=ent.get_by_label('Zeichnung für EG')
 expect(zeichnung).to_have_value(page.evaluate("window.planSaves.find(z=>z.daten.bezeichnung.endsWith('EG')).objekt"))
 ent.get_by_role('button',name='IFC4-Modell schreiben').click()
 eb=ent.get_by_role('status',name='Exportergebnis')
 expect(eb).to_contain_text('Lagerhalle-Modell.ifc')
 expect(eb).to_contain_text('keine Geschosshöhe')
 w=page.evaluate("window.ifcAufrufe.filter(a=>a.cmd==='ifc_exportieren')[1].args.wunsch")
 assert w['art']=='entwurf' and len(w['geschosse'])==1 and w['geschosse'][0]['zeichnung_id']==zeichnung.input_value(), w
 page.screenshot(path=str(out/'ifc-export.png'))
 n=len(page.evaluate('window.ifcAkten'))
 eb.get_by_role('button',name='Als Modell öffnen').click()
 page.wait_for_function(f"window.ifcAkten.length==={n+1}")
 neu=page.evaluate("window.ifcAkten.find(a=>a.daten.dokument.dokument_id==='f416da8b-cbfb-4ae6-931b-8765fb6454f1')")
 assert neu and neu['daten']['bezeichnung']=='Lagerhalle-Modell', neu
 expect(scope.get_by_role('complementary',name='Modellstruktur')).to_contain_text('Lagerhalle-Modell')
 # IFC-Geschoss als Hintergrund einer eigenen Zeichnung.
 scope.get_by_role('button',name='Neue Zeichnung',exact=True).click()
 tab=scope.locator('section[aria-label="Neue Zeichnung"]:visible')
 tab.get_by_role('tab',name='Eigenschaften',exact=True).click()
 expect(tab.locator('section.palette[aria-label="Blatt und Einrichtung"]')).to_be_visible()
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
