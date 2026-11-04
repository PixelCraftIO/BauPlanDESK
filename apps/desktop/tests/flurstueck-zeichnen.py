"""Flurstücksfläche im Zeicheneditor ermitteln: Knopf in den Flurstücken, Unterlage
wählen, Grenze mit vorbelegter Bedeutung zeichnen, Fläche übernehmen und zurück.
Läuft gegen den Produktionsbuild mit dem vorhandenen Tauri-Testtransport."""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich
from plan_fixture import pdf_fixture, custom
root = Path(__file__).resolve().parent
fixture = json.loads((root / 'fixtures/projekt.json').read_text(encoding='utf-8'))
# Aufgabe und Übernahme für Flurstück „a“ (Muenchen 100/1); die Fläche setzt wie im Core die Übernahme ein.
flur = r"""
const vorher=window.__TAURI_INTERNALS__.invoke;
window.flurAufrufe=[];
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 if(cmd==="planaufmass_aufgaben"){
   const m=args.plan.markierungen.find(m=>m.bedeutung==="flurstueck"&&m.flurstueck_id==="a");
   return [{verbraucher:"flurstueck",objekt:"a",objekt_name:"Muenchen 100/1",gruppe:"Grundstück",ziel:"flurstuecke.a.flaeche_m2",titel:"Flurstück Muenchen 100/1 · Fläche",groesse:"flaeche",bedeutung:"flurstueck",geometrie:"flaeche",gebaeude_id:null,geschoss_id:null,grundstueck_id:null,flurstueck_id:"a",wert_vorhanden:true,gebunden:null,markierung:m?.id??null,hinweis:null}];
 }
 if(cmd==="planaufmass_diagnose")return {markierungen:args.plan.markierungen.map(m=>({id:m.id,wert:{id:m.id,laenge_m:null,flaeche_m2:m.bedeutung==="flurstueck"?298.4:null,horizontale_m:null,hoehe_m:null,neigung_grad:null,offen:[]},fehler:null}))};
 if(cmd==="planaufmass_flurstuecke"){
   window.flurAufrufe.push(structuredClone(args.infos));
   const infos=structuredClone(args.infos);
   for(const f of infos.flurstuecke)if(f.planbindung)f.planflaeche_m2=298.4;
   return infos;
 }
 return vorher(cmd,args);
};
"""
errors = []
with sync_playwright() as pw:
    browser = pw.chromium.launch(headless=True, executable_path=os.environ.get('BAUPROJEKT_CHROMIUM', '/usr/bin/chromium'))
    page = browser.new_page(viewport={'width': 1600, 'height': 1000}, locale='de-DE')
    page.on('pageerror', lambda e: errors.append(str(e)))
    page.add_init_script('window.fixture=' + json.dumps(fixture) + ';window.planPdf=' + json.dumps(pdf_fixture()) + ';' + (root / 'tauri-mock.js').read_text(encoding='utf-8') + custom + flur)
    page.goto(os.environ.get('BAUPROJEKT_UI_URL', 'http://127.0.0.1:1430'))
    page.locator('.projekttabelle').get_by_role('button', name=fixture['name'], exact=True).click()
    bereich(page, 'Flurstücke und Grundbuch')
    scope = page.locator('.projekt-sitzung:visible')
    knopf = scope.get_by_role('button', name='Im Zeicheneditor ermitteln', exact=True)
    expect(knopf).to_have_count(3)
    knopf.first.click()
    # Zwei Projektdateien ohne Aufmaß: Auswahl der Unterlage.
    wahl = scope.get_by_label('Flurstücksfläche ermitteln')
    expect(wahl).to_contain_text('Flurstück Muenchen 100/1')
    wahl.get_by_label('Zeichnen auf').select_option(label='Grundrisse.pdf')
    wahl.get_by_role('button', name='Öffnen', exact=True).click()
    expect(wahl).to_have_count(0)
    auftrag = scope.get_by_label('Flurstücksauftrag')
    expect(auftrag).to_contain_text('Grenze als Polygon zeichnen')
    expect(auftrag).to_contain_text('Zwei Punkte kalibrieren')
    expect(auftrag.get_by_role('button', name='Fläche übernehmen und zurück')).to_be_disabled()
    svg = scope.get_by_role('application'); expect(svg).to_be_visible()
    def click(x, y):
        q = svg.evaluate("""async(svg,p)=>{let q=new DOMPoint(p[0],p[1]).matrixTransform(svg.getScreenCTM());
          const s=svg.closest('.plan-scroll'),r=s.getBoundingClientRect();s.scrollLeft+=q.x-r.left-r.width/2;s.scrollTop+=q.y-r.top-r.height/2;
          await new Promise(requestAnimationFrame);q=new DOMPoint(p[0],p[1]).matrixTransform(svg.getScreenCTM());return {x:q.x,y:q.y};}""", [x, y])
        page.mouse.click(q['x'], q['y'])
    auftrag.get_by_role('button', name='Grenze zeichnen').click()
    expect(scope.get_by_role('button', name='Polygon', exact=True)).to_have_attribute('aria-pressed', 'true')
    scope.get_by_label('Punktfang', exact=True).uncheck()
    click(100, 100); click(300, 100); click(300, 250); click(100, 250); svg.press('Enter')
    expect(scope.get_by_label('Bedeutung', exact=True)).to_have_value('flurstueck')
    expect(auftrag).to_contain_text('298,4 m²')
    auftrag.get_by_role('button', name='Fläche übernehmen und zurück').click()
    vorschau = page.get_by_role('dialog')
    expect(vorschau).to_contain_text('Flurstück Muenchen 100/1 · Fläche')
    expect(vorschau).to_contain_text('298,4')
    vorschau.get_by_role('button', name='Übernehmen und speichern').click()
    # Zurück in den Flurstücken mit gespeicherter Verknüpfung.
    planflaeche = scope.get_by_label('Flurstücksgrenze aus Aufmaß').first
    expect(planflaeche).to_contain_text('gezeichnet 298,4 m²')
    expect(scope.get_by_role('button', name='Im Zeicheneditor bearbeiten', exact=True)).to_have_count(1)
    gespeichert = page.evaluate('window.testProjects[0].planungsrecht.grundstuecksinformationen.flurstuecke.find(f=>f.id==="a")')
    assert gespeichert['planbindung']['ziel'] == 'flurstuecke.a.flaeche_m2', gespeichert
    assert gespeichert['planflaeche_m2'] == 298.4 and gespeichert['flaeche_m2'] == 300, gespeichert
    assert page.evaluate('window.planSaved?.daten.markierungen.some(m=>m.bedeutung==="flurstueck"&&m.flurstueck_id==="a")')
    # Neues Flurstück ohne Fläche: Der Knopf speichert ohne Rückfrage mit
    # „Fläche wird im Aufmaß ermittelt“ und öffnet den Zeicheneditor.
    scope.get_by_role('button', name='Flurstück hinzufügen').click()
    zeile = scope.locator('.eingabetabelle tbody tr').last
    zeile.get_by_label('Gemarkung', exact=True).fill('Neudorf')
    zeile.get_by_label('Flur', exact=True).fill('2')
    zeile.get_by_label('Zähler', exact=True).fill('7')
    zeile.get_by_label('Flurstücksfläche').fill('')
    zeile.get_by_role('button', name='Im Zeicheneditor ermitteln', exact=True).click()
    expect(page.get_by_role('dialog')).to_have_count(0)
    expect(scope.get_by_label('Flurstücksfläche ermitteln')).to_contain_text('Flurstück Neudorf 7')
    neu = page.evaluate('window.testProjects[0].planungsrecht.grundstuecksinformationen.flurstuecke.find(f=>f.gemarkung==="Neudorf")')
    assert neu and neu.get('flaeche_offen') is True and neu['flaeche_m2'] == 0, neu
    assert not errors, errors
    browser.close()
print('Flurstücksfläche: Knopf, Unterlage, Grenze zeichnen, Übernahme gespeichert, zurück in den Flurstücken.')
