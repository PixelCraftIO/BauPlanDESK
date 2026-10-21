"""Updater UI; native Antworten simuliert, keine Installation im Browser."""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich, register, gliederung
root=Path(__file__).resolve().parent
fixture=json.loads((root/'fixtures/projekt.json').read_text())
custom=r'''
const base=window.__TAURI_INTERNALS__.invoke;
window.updateConfigured=true;window.updateAvailable=null;window.updateFail=false;window.installFail=true;window.installDelay=500;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 if(cmd==='formularbibliothek')return {root:'/tmp/Formulare',vorlagen:[]};
 if(cmd==='projektordner_status')return '/tmp/Projekte';
 if(cmd==='update_status')return {eingerichtet:window.updateConfigured,quelle:'https://updates.test/latest.json',hinweis:window.updateConfigured?'Signierte Updates eingerichtet':'Updates sind für diesen Build noch nicht eingerichtet.'};
 if(cmd==='update_pruefen'){window.calls.push({cmd,args});if(window.updateFail)throw {art:'update',meldung:'Test: Updatequelle nicht erreichbar'};return window.updateAvailable;}
 if(cmd==='update_installieren'){
  window.calls.push({cmd,args});await window.emitNative('update-fortschritt',{geladen:1048576,gesamt:2097152,phase:'download'});
  await new Promise(r=>setTimeout(r,window.installDelay));
  if(window.installFail)throw {art:'update',meldung:'Test: Signatur ungültig – Installation abgelehnt'};
  await window.emitNative('update-fortschritt',{geladen:0,gesamt:null,phase:'installation'});return;
 }
 return base(cmd,args);
};
'''
errors=[]
with sync_playwright() as p:
 browser=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=browser.new_page(viewport={'width':1440,'height':1000});page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(fixture)+';'+(root/'tauri-mock.js').read_text()+custom)
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1452'))
 bereich(page,'Einstellungen')
 button=page.get_by_role('button',name='Updates prüfen',exact=True)
 button.click();expect(page.get_by_text('BauPlanIO ist auf dem neuesten Stand.',exact=True)).to_be_visible()
 page.evaluate('window.updateConfigured=false');button.click();expect(page.get_by_role('status')).to_contain_text('noch nicht eingerichtet')
 page.evaluate('window.updateConfigured=true;window.updateFail=true');button.click();expect(page.get_by_role('status')).to_contain_text('nicht erreichbar')
 page.evaluate('window.updateFail=false;window.updateAvailable={version:"0.1.1",notizen:"Neue Funktionen"}')
 button.click();install=page.get_by_role('button',name='Update installieren und neu starten',exact=True);expect(install).to_be_visible()
 page.get_by_text('Änderungen anzeigen',exact=True).click();expect(page.get_by_text('Neue Funktionen',exact=True)).to_be_visible()
 bereich(page,'Projekte')
 page.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click()
 bereich(page,'Rechtsgrundlagen')
 page.locator('.projekt-sitzung:visible section[aria-label="Rechtsgrundlagen"]').get_by_label('Planbezeichnung',exact=True).fill('Nicht verlieren')
 bereich(page,'Einstellungen')
 install.click();expect(page.get_by_role('status').filter(has_text='Vor der Aktualisierung')).to_contain_text('alle Projekt- und Kundeneingaben speichern')
 assert page.evaluate('window.calls.filter(c=>c.cmd==="update_installieren").length')==0
 page.locator('.tab-auswahl').filter(has_text=fixture['name']).click()
 expect(page.locator('.projekt-sitzung:visible section[aria-label="Rechtsgrundlagen"]').get_by_label('Planbezeichnung',exact=True)).to_have_value('Nicht verlieren')
 page.get_by_role('button',name=fixture['name']+' schließen',exact=True).click()
 page.get_by_role('button',name='Verwerfen und schließen',exact=True).click()
 bereich(page,'Einstellungen')
 install.click();expect(page.get_by_role('progressbar',name='Update herunterladen')).to_be_visible()
 expect(page.locator('.arbeitsrahmen')).to_have_attribute('inert','')
 page.evaluate('window.emitNative("tauri://close-requested")')
 assert not page.evaluate('!!window.testDestroyed')
 expect(page.locator('.fehler[role=alert]')).to_contain_text('Signatur ungültig')
 expect(install).to_be_enabled();assert page.evaluate('window.calls.filter(c=>c.cmd==="update_installieren")[0].args.version')=='0.1.1'
 page.evaluate('window.installFail=false');install.click();expect(install).to_be_enabled()
 assert page.evaluate('window.calls.filter(c=>c.cmd==="update_installieren").length')==2
 assert not errors,errors
 browser.close()
print('Updater UI: Konfiguration, kein Update, Offlinefehler, Releasehinweise, Entwurfschutz, Fortschritt, Fenstersperre und Wiederholung geprüft. Native Installation separat mit signierten Builds testen.')
