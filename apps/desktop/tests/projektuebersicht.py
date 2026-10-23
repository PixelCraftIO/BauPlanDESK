"""Zentrale Übersicht: echte Core-Auswertung, gezielte Navigation, Entwürfe und Projektgrenzen."""
from pathlib import Path
import os,json
from playwright.sync_api import sync_playwright,expect
from navigation_hilfe import bereich, register, gliederung
root=Path(__file__).resolve().parent
art=Path(os.environ.get('BAUPROJEKT_UEBERSICHT_ARTEFAKTE','/tmp/bauprojekt-uebersicht'))
wf=json.loads((art/'workflow.json').read_text())
out=Path(os.environ.get('BAUPROJEKT_UEBERSICHT_UI_ARTEFAKTE',str(root.parents[2]/'docs/projektuebersicht/screenshots')));out.mkdir(parents=True,exist_ok=True)
custom=r'''
{
const base=window.__TAURI_INTERNALS__.invoke,clone=x=>JSON.parse(JSON.stringify(x));
window.testUser=clone(window.dashboard.benutzer);window.overview=clone(window.dashboard.uebersicht);window.overviewFail=false;window.overviewDelay=false;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 cmd=({fachobjektseite:'fachobjekte',fachverlaufsseite:'fachverlauf',druckartefakt_erstellen:'druckausgabe_erstellen'})[cmd]??cmd;
 if(cmd==='projektuebersicht'&&args.projektId===window.fixture.id){
  window.calls.push({cmd,args:clone(args)});
  if(window.overviewFail)throw {art:'datenbank',meldung:'Die Übersicht ist vorübergehend nicht erreichbar.'};
  const result=clone(window.overview);
  if(window.overviewDelay){window.overviewDelay=false;await new Promise(r=>window.overviewResolve=r);}
  return result;
 }
 if(cmd==='fachobjekte'&&args.projektId===window.fixture.id){
  if(['aufgaben','unterlagen','verfahren'].includes(args.art))return clone(window.dashboard[args.art]);
 }
 if(cmd==='nachweise'&&args.projektId===window.fixture.id)return clone(window.dashboard.nachweise);
 if(cmd==='dokumente_auflisten'&&args.projektId===window.fixture.id)return clone(window.dashboard.dateien);
 if(cmd==='benutzer_auflisten')return [clone(window.dashboard.benutzer),clone(window.dashboard.leser)];
 return base(cmd,args);
};
}
'''
errors=[]
with sync_playwright() as p:
 b=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=b.new_page(viewport={'width':1600,'height':1100},locale='de-DE');page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(wf['projekt'])+';window.dashboard='+json.dumps(wf)+';'+(root/'tauri-mock.js').read_text()+custom)
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1450'))
 page.locator('.projekttabelle').get_by_role('button',name=wf['projekt']['name'],exact=True).click()
 def active():return page.locator('.projekt-sitzung:visible')
 def overview():return active().get_by_role('region',name='Zentrale Projektübersicht',exact=True)
 def view(n):bereich(page,n)
 expect(overview().get_by_role('heading',name='Projektübersicht',exact=True)).to_be_visible()
 expect(overview().get_by_role('button',name='Arbeitsschritt öffnen',exact=True)).to_be_visible()
 expect(overview().get_by_role('button',name='Offene Aufgaben 2 Einträge anzeigen →',exact=True)).to_be_visible()
 expect(overview().get_by_role('region',name='Einreichungsstand')).to_contain_text('Einreichung bestätigt')
 expect(overview().get_by_role('region',name='Dokumentierter Behördenstand')).to_contain_text('Inhalt geändert')
 page.screenshot(path=str(out/'uebersicht-hell.png'))
 page.evaluate("document.documentElement.dataset.theme='dark';document.documentElement.style.colorScheme='dark'");page.screenshot(path=str(out/'uebersicht-dunkel.png'));overview().get_by_role('region',name='Dokumentierter Behördenstand').scroll_into_view_if_needed();page.screenshot(path=str(out/'behoerdenstand-dunkel.png'))
 for width in [900,600]:
  page.set_viewport_size({'width':width,'height':1000});assert page.evaluate('document.documentElement.scrollWidth<=innerWidth');
  if width==900:page.screenshot(path=str(out/'uebersicht-900.png'))
 page.set_viewport_size({'width':1600,'height':1100})
 # Priorisierte nächste Aktion öffnet das konkrete Aufgabenobjekt.
 overview().get_by_role('button',name='Arbeitsschritt öffnen',exact=True).click();expect(active().get_by_label('Aufgabe',exact=True)).to_have_value('Lageplan beim Vermesser anfordern')
 active().get_by_label('Aufgabe',exact=True).fill('Ungespeicherter Aufgabenentwurf')
 view('Projektübersicht');overview().get_by_role('button',name='Bauherrnfreigabe einholen öffnen',exact=True).click()
 expect(active().get_by_role('status').filter(has_text='Ein anderer Entwurf')).to_be_visible();expect(active().get_by_label('Aufgabe',exact=True)).to_have_value('Ungespeicherter Aufgabenentwurf');active().get_by_role('button',name='Entwurf verwerfen',exact=True).click()
 # Der Sprung lädt den aktuellen Editorstand und verwendet keine alte Listenrevision.
 page.evaluate("window.dashboard.aufgaben[1].daten.titel='Aktualisierte Bauherrnfreigabe';window.dashboard.aufgaben[1].revision+=1")
 view('Projektübersicht');overview().get_by_role('button',name='Bauherrnfreigabe einholen öffnen',exact=True).click();expect(active().get_by_label('Aufgabe',exact=True)).to_have_value('Aktualisierte Bauherrnfreigabe');active().get_by_role('button',name='Schließen',exact=True).click()
 # Nachforderung: konkrete Akte und passendes Register, nicht nur eine leere Modulliste.
 view('Projektübersicht');overview().get_by_role('button',name='Wohnflächenberechnung ergänzen öffnen',exact=True).click();expect(active().get_by_label('Forderung',exact=True)).to_have_value('Wohnflächenberechnung ergänzen');expect(active().get_by_role('navigation',name='Verfahrensbereiche').get_by_role('button',name='Nachforderungen',exact=True)).to_have_attribute('aria-pressed','true')
 view('Projektübersicht');overview().get_by_role('button',name='Nachforderungen offen 1 Einträge anzeigen →',exact=True).click();expect(overview().get_by_role('region',name='Handlungsbedarf').get_by_role('listitem')).to_have_count(1);overview().get_by_role('button',name='Zusatzfilter aufheben',exact=True).click()
 overview().get_by_label('Bereich',exact=True).select_option('');overview().get_by_label('Suchen',exact=True).fill('Lageplan');expect(overview().get_by_role('region',name='Handlungsbedarf').get_by_role('listitem').first).to_contain_text('Lageplan');overview().get_by_label('Suchen',exact=True).fill('')
 # Aktualisierung nach fachlicher Änderung und ausdrücklicher Fehlerzustand ohne falsche Nullwerte.
 page.evaluate('window.overview=window.dashboard.nach_erledigung');overview().get_by_role('button',name='Übersicht aktualisieren',exact=True).click();expect(overview().get_by_role('button',name='Offene Aufgaben 1 Einträge anzeigen →',exact=True)).to_be_visible()
 page.evaluate('window.overviewFail=true');overview().get_by_role('button',name='Übersicht aktualisieren',exact=True).click();expect(overview().get_by_role('alert')).to_contain_text('letzte erfolgreich geladene Stand');expect(overview().get_by_role('button',name='Offene Aufgaben 1 Einträge anzeigen →',exact=True)).to_be_visible();page.evaluate('window.overviewFail=false')
 # Verspätete Antwort der Hintergrundakte verändert nicht die sichtbare zweite Projektakte.
 page.evaluate('window.overviewDelay=true');overview().get_by_role('button',name='Übersicht aktualisieren',exact=True).click();page.wait_for_function('typeof window.overviewResolve==="function"');view('Projekte');page.locator('.projekttabelle').get_by_role('button',name='Umbau Bürogebäude',exact=True).click();page.evaluate('window.overviewResolve()');expect(overview().get_by_role('button',name='Offene Aufgaben 0 Einträge anzeigen →',exact=True)).to_be_visible();expect(overview()).not_to_contain_text('Lageplan beim Vermesser')
 # Leserechte gelten auch nach einem gezielten Sprung in einen Editor.
 reader=b.new_page(viewport={'width':1440,'height':1000},locale='de-DE');reader.on('pageerror',lambda e:errors.append(str(e)))
 reader.add_init_script('window.fixture='+json.dumps(wf['projekt'])+';window.dashboard='+json.dumps(wf)+';'+(root/'tauri-mock.js').read_text()+custom+';window.testUser.rolle="lesen";')
 reader.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1450'));reader.locator('.projekttabelle').get_by_role('button',name=wf['projekt']['name'],exact=True).click();reader.get_by_role('button',name='Arbeitsschritt öffnen',exact=True).click();expect(reader.get_by_label('Aufgabe',exact=True)).to_be_disabled();expect(reader.get_by_role('button',name='Speichern',exact=True)).to_be_disabled();assert not reader.evaluate("window.calls.some(c=>c.cmd==='fachobjekt_speichern')")
 failed=b.new_page(viewport={'width':1440,'height':1000});failed.add_init_script('window.fixture='+json.dumps(wf['projekt'])+';window.dashboard='+json.dumps(wf)+';'+(root/'tauri-mock.js').read_text()+custom+';window.overviewFail=true;');failed.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1450'));failed.locator('.projekttabelle').get_by_role('button',name=wf['projekt']['name'],exact=True).click();expect(failed.get_by_role('alert').filter(has_text='Es liegt kein auswertbarer Stand vor')).to_be_visible();expect(failed.locator('.kennzahlen')).to_have_count(0)
 assert not errors,errors;b.close()
print('Projektübersicht: Core-Zahlen, Hell/Dunkel, gezielte Objekt-/Registersprünge, Entwurfschutz, Filter, Refreshfehler und verspätete Projektantwort geprüft.')
