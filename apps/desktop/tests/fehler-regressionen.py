"""Regressionen: Neuanlage, Konfliktfehler, Lizenzrolle, Mitgliedschaften und reine Anzeige.
Läuft gegen den Produktionsbuild mit dem vorhandenen Tauri-Testtransport.
"""
import json
import os
from pathlib import Path
from playwright.sync_api import sync_playwright, expect

root = Path(__file__).resolve().parent
fixture = json.loads((root / 'fixtures/projekt.json').read_text(encoding='utf-8'))
fixture['planungsrecht']['bau_nvo'] = '1990'  # gespeicherte Aufstellungen sind noch 2017
custom = r'''
const base = window.__TAURI_INTERNALS__.invoke;
window.customer = { id:'kunde',typ:'person',kategorie:'privat',kundennummer:'',vorname:'Ada',familienname:'Bauherr',firma:'',ansprechpartner:'',anschrift:{strasse:'',hausnummer:'',postleitzahl:'',ort:'',staat:'DE'},telefon:'',email:'',notizen:'',version:1,created_at:window.fixture.created_at,updated_at:window.fixture.updated_at };
window.members = [];
window.__TAURI_INTERNALS__.invoke = async (cmd,args={}) => {
  if(cmd==='kunden_auflisten') {
    if(window.customerFailure) throw {art:'datenbank',meldung:'Kundenliste nicht erreichbar'};
    return [structuredClone(window.customer)];
  }
  if(cmd==='projekt_anlegen') {
    window.calls.push({cmd,args});
    const p={...structuredClone(window.fixture),...args.projekt,id:'aaf2bbea-56ee-444a-82c2-0a79092cc613'};
    window.testProjects.push(p);return p;
  }
  if(cmd==='bauherren_setzen') throw {art:'datenbank',meldung:'Zuordnung fehlgeschlagen'};
  if(cmd==='projekt_aktualisieren' && window.conflict) {
    window.reloadFailure=true;throw {art:'konflikt',meldung:'Versionskonflikt'};
  }
  if(cmd==='projekt_laden' && window.reloadFailure) throw {art:'datenbank',meldung:'Neuladen fehlgeschlagen'};
  if(cmd==='benutzer_auflisten') return [window.testUser,{...window.testUser,id:'kollege',anzeigename:'Kollege',rolle:'bearbeitung'}];
  if(cmd==='projektmitglieder') return structuredClone(window.members);
  if(cmd==='projektmitglied_setzen') {
    window.calls.push({cmd,args});
    if(window.memberConflict)throw {art:'konflikt',meldung:'Mitgliedschaft inzwischen geändert'};
    const m={projekt_id:args.projektId,modul:'mitgliedschaften',objekt:args.mitglied.benutzer_id,schema_version:1,revision:args.basis+1,daten:args.mitglied};
    window.members=[m];return structuredClone(m);
  }
  const result=await base(cmd,args);
  if(cmd==='lizenz_status' && window.readLicense)result.schreiben_erlaubt=false;
  return result;
};
'''
errors = []
with sync_playwright() as p:
    executable = os.environ.get('BAUPROJEKT_CHROMIUM')
    browser = p.chromium.launch(headless=True, **({'executable_path': executable} if executable else {}))

    def page(extra=''):
        tab = browser.new_page(viewport={'width':1440,'height':1050}, timezone_id='Europe/Berlin')
        tab.on('pageerror', lambda e: errors.append(str(e)))
        tab.add_init_script('window.fixture='+json.dumps(fixture)+';'+(root/'tauri-mock.js').read_text(encoding='utf-8')+custom+extra)
        tab.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1430'))
        tab.get_by_role('heading',name='Projektverzeichnis',exact=True).wait_for()
        return tab

    tab=page()
    tab.get_by_role('button',name='Neues Projekt',exact=True).click()
    dialog=tab.get_by_role('dialog')
    dialog.get_by_label('Projektname',exact=True).fill('Einmal angelegt')
    dialog.get_by_label('Bauherr').select_option('kunde')
    dialog.get_by_role('button',name='Anlegen',exact=True).click()
    expect(dialog).to_have_count(0)
    expect(tab.locator('.tab-auswahl')).to_contain_text('Einmal angelegt')
    expect(tab.get_by_role('alert').filter(has_text='Bauherr konnte nicht zugeordnet')).to_be_visible()
    assert tab.evaluate("window.calls.filter(c=>c.cmd==='projekt_anlegen').length") == 1
    tab.close()

    tab=page('window.customerFailure=true;')
    tab.get_by_role('button',name='Neues Projekt',exact=True).click()
    expect(tab.get_by_role('dialog').get_by_role('alert')).to_contain_text('Kundenliste nicht erreichbar')
    expect(tab.get_by_role('dialog').get_by_role('button',name='Anlegen',exact=True)).to_be_enabled()
    tab.close()

    tab=page('window.readLicense=true;')
    expect(tab.get_by_role('button',name='Anna Weber (Lesen)',exact=True)).to_be_visible()
    tab.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click()
    tab.locator('.aktennavigation').get_by_role('button',name='Baugrundstück',exact=True).click()
    expect(tab.locator('.entwurf-punkt')).to_have_count(0)
    tab.get_by_role('button',name=fixture['name']+' schließen',exact=True).click()
    expect(tab.get_by_role('dialog')).to_have_count(0)
    expect(tab.locator('.projekttab')).to_have_count(0)
    tab.close()

    tab=page()
    tab.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click()
    tab.locator('.aktennavigation').get_by_role('button',name='Stammdaten',exact=True).click()
    tab.get_by_role('button',name='Bearbeiten …',exact=True).click()
    tab.get_by_role('dialog').get_by_label('Projektname',exact=True).fill('Entwurf erhalten')
    tab.evaluate('window.conflict=true')
    tab.get_by_role('dialog').get_by_role('button',name='Speichern',exact=True).click()
    expect(tab.get_by_role('alert').filter(has_text='Neuladen fehlgeschlagen')).to_be_visible()
    expect(tab.get_by_role('dialog').get_by_label('Projektname',exact=True)).to_have_value('Entwurf erhalten')
    tab.get_by_role('dialog').get_by_role('button',name='Abbrechen',exact=True).click()

    tab.get_by_role('button',name='Projektmitglieder …',exact=True).click()
    recht=tab.get_by_label('Projektberechtigung für Kollege',exact=True)
    recht.select_option('lesen')
    tab.get_by_role('dialog').get_by_role('button',name='Berechtigung für Kollege speichern',exact=True).click()
    expect(tab.get_by_role('dialog').get_by_role('status')).to_contain_text('gespeichert')
    assert tab.evaluate("window.calls.find(c=>c.cmd==='projektmitglied_setzen').args.mitglied.recht") == 'lesen'
    tab.evaluate('window.memberConflict=true')
    recht.select_option('bearbeiten')
    tab.get_by_role('dialog').get_by_role('button',name='Berechtigung für Kollege speichern',exact=True).click()
    expect(tab.get_by_role('dialog').get_by_role('alert')).to_contain_text('inzwischen geändert')
    expect(recht).to_have_value('bearbeiten')
    # Im Konflikt bleibt die Auswahl erhalten; zum Schließen gezielt zurücksetzen.
    recht.select_option('lesen')
    tab.get_by_role('dialog').get_by_role('button',name='Schließen',exact=True).click()
    tab.evaluate("window.emitNative('menu','bearbeiten.kunden')")
    expect(tab.get_by_role('heading',name='Kunden / Bauherren',exact=True)).to_be_visible()
    tab.evaluate("window.emitNative('menu','bearbeiten.kunden')")
    expect(tab.get_by_role('dialog')).to_have_count(0)
    expect(tab.get_by_role('heading',name='Kunden / Bauherren',exact=True)).to_have_count(1)
    assert not errors, errors
    browser.close()
print('UI-Regressionen: einmalige Neuanlage, Folgefehler, Konflikt, Lizenzrolle, Mitglieder und reine Anzeige erfolgreich.')
