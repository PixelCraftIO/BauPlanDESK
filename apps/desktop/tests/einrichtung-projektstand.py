"""Ersteinrichtung, mitgelieferte Vordrucke, ausgefüllte Formulare und Projektstand."""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich
root=Path(__file__).resolve().parent
fixture=json.loads((root/'fixtures/projekt.json').read_text())
custom=r'''
{
const base=window.__TAURI_INTERNALS__.invoke,clone=x=>JSON.parse(JSON.stringify(x));
window.aufrufe=[];
window.buero={name:'',anschrift:{strasse:'',hausnummer:'',plz:'',ort:''},kennung:'',telefon:'',email:'',entwurfsverfasser:[]};
window.werk=[{name:'NRW Bauantrag (Anlage I-2).pdf',titel:'Bauantrag (Anlage I/2)',land:'NW',vorhanden:false},{name:'NRW Baubeschreibung (Anlage I-7).pdf',titel:'Baubeschreibung (Anlage I/7)',land:'NW',vorhanden:false}];
window.vorlagen=[];
window.stand={projekt_id:window.fixture.id,stufe:'eingereicht',bezeichnung:'Eingereicht',seit:'2026-09-15',verfahrensstufe:'eingereicht'};
window.ausgefuellt=[
 {dokument_id:'11111111-1111-4111-8111-111111111111',version:2,datei:'Bauantrag.pdf',vorlage:'NRW Bauantrag (Anlage I-2).pdf',erstellt:'2026-09-10T09:00:00Z',erstellt_von:'Test',abweichungen:['Bauherr Name','Flurstück']},
 {dokument_id:'22222222-2222-4222-8222-222222222222',version:1,datei:'Baubeschreibung.pdf',vorlage:'NRW Baubeschreibung (Anlage I-7).pdf',erstellt:'2026-09-11T09:00:00Z',erstellt_von:'Test',abweichungen:[]},
 {dokument_id:'33333333-3333-4333-8333-333333333333',version:1,datei:'Alt.pdf',vorlage:'Entfernte Vorlage.pdf',erstellt:'2026-09-12T09:00:00Z',erstellt_von:'Test',abweichungen:null}];
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 window.aufrufe.push({cmd,args:clone(args??{})});
 if(cmd==='buero')return clone(window.buero);
 if(cmd==='formularbibliothek')return {root:'/tmp/Formulare',vorlagen:clone(window.vorlagen),werksvorlagen:clone(window.werk)};
 if(cmd==='werksvorlagen_uebernehmen'){
  window.werk.forEach(w=>w.vorhanden=true);
  window.vorlagen=window.werk.map(w=>({name:w.name,sha256:'a'.repeat(64),size:1000,felder:12}));
  return {root:'/tmp/Formulare',vorlagen:clone(window.vorlagen),werksvorlagen:clone(window.werk)};
 }
 if(cmd==='projektstaende')return [clone(window.stand)];
 if(cmd==='projektstand')return clone(window.stand);
 if(cmd==='projekt_abschliessen'){
  window.stand=args.abgeschlossen?{...window.stand,stufe:'abgeschlossen',bezeichnung:'Abgeschlossen',seit:null}:{...window.stand,stufe:'eingereicht',bezeichnung:'Eingereicht',seit:'2026-09-15'};
  return clone(window.stand);
 }
 if(cmd==='projekt_formulare')return clone(window.ausgefuellt);
 return base(cmd,args);
};
}
'''
errors=[]
with sync_playwright() as pw:
 browser=pw.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=browser.new_page(viewport={'width':1440,'height':1000},locale='de-DE')
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(fixture)+';'+(root/'tauri-mock.js').read_text()+custom)
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1465'))
 # Ersteinrichtung: drei offene Schritte, Vordrucke mit einem Klick.
 kasten=page.get_by_role('region',name='Einrichtung abschließen',exact=True)
 expect(kasten).to_contain_text('Noch 3 von 3 Schritten offen')
 expect(kasten.get_by_role('button',name='Büroangaben öffnen',exact=True)).to_have_count(2)
 kasten.get_by_role('button',name='Mitgelieferte Vordrucke übernehmen',exact=True).click()
 expect(kasten).to_contain_text('Noch 2 von 3 Schritten offen')
 expect(kasten).to_contain_text('2 Vorlagen stehen in der Formularbibliothek')
 # Vollständig eingerichtet: Der Kasten verschwindet nach dem nächsten Laden.
 page.evaluate("window.buero.name='Büro Test';window.buero.anschrift.ort='Köln';window.buero.entwurfsverfasser=[{id:'e1',vorname:'Anna',familienname:'Weber',berufsbezeichnung:'Architektin',kammer:'AKNW',mitgliedsnummer:'A 1',telefon:'',email:''}]")
 bereich(page,'Formularbibliothek');bereich(page,'Projekte')
 # Projektverzeichnis: Stand je Projekt und Filter.
 zeile=page.locator('.projekttabelle tbody tr').filter(has_text=fixture['name'])
 expect(zeile.locator('td').nth(1)).to_have_text('Eingereicht · 15.09.2026',use_inner_text=True)
 page.get_by_label('Projektstand filtern',exact=True).select_option('abgeschlossen')
 expect(page.locator('.projekttabelle tbody tr').filter(has_text=fixture['name'])).to_have_count(0)
 page.get_by_label('Projektstand filtern',exact=True).select_option('laufend')
 expect(zeile).to_have_count(1)
 # Projektstand in der Akte, Abschluss und Rücknahme.
 zeile.get_by_role('button',name=fixture['name'],exact=True).click()
 expect(kasten).to_have_count(0)
 akte=page.locator('.projekt-sitzung:visible')
 leiste=akte.get_by_role('region',name='Projektstand',exact=True)
 expect(leiste.locator('li[aria-current="step"]')).to_contain_text('Eingereicht · seit 15.09.2026')
 leiste.get_by_role('button',name='Projekt abschließen …',exact=True).click()
 expect(leiste.locator('li[aria-current="step"]')).to_contain_text('Abgeschlossen')
 leiste.get_by_role('button',name='Projekt wieder öffnen',exact=True).click()
 expect(leiste.locator('li[aria-current="step"]')).to_contain_text('Eingereicht')
 assert page.evaluate('window.aufrufe.filter(a=>a.cmd==="projekt_abschliessen").map(a=>a.args.abgeschlossen)')==[True,False]
 # Ausgefüllte Formulare mit Abgleich.
 bereich(page,'Formulare')
 tabelle=akte.locator('table').filter(has_text='Abgleich mit den Projektdaten')
 expect(tabelle.locator('tbody tr')).to_have_count(3)
 z=tabelle.locator('tbody tr')
 expect(z.nth(0)).to_contain_text('2 Felder weichen ab');expect(z.nth(0).get_by_role('button',name='Aktualisieren …',exact=True)).to_be_visible()
 expect(z.nth(1)).to_contain_text('aktuell');expect(z.nth(1).get_by_role('button',name='Bearbeiten …',exact=True)).to_be_visible()
 expect(z.nth(2)).to_contain_text('nicht prüfbar')
 assert not errors,errors
 browser.close()
print('Einrichtung und Projektstand: Ersteinrichtung, Vordrucke, Standspalte, Filter, Abschluss und Formularabgleich geprüft.')
