"""Anmeldedialog im Netzwerkbetrieb: Fehlversuch, Erstkennwort ersetzen, Übergang und Abmelden.
Läuft gegen den Produktionsbuild mit dem vorhandenen Tauri-Testtransport; die
Anmelderegeln selbst prüfen die Rust-Tests (crates/storage/tests/anmeldung.rs).
"""
import json
import os
from pathlib import Path
from playwright.sync_api import sync_playwright, expect

root = Path(__file__).resolve().parent
fixture = json.loads((root / 'fixtures/projekt.json').read_text(encoding='utf-8'))
custom = r'''
const base = window.__TAURI_INTERNALS__.invoke;
const offen = (optionen) => ({kennwortanmeldung:true,benutzer:null,kennwort_aendern:null,bisheriges_kennwort_noetig:false,optionen,kennwort_mindestlaenge:10});
window.testAnmeldung = offen({uebergang_moeglich:window.uebergang??false,rechnerkonto:'chefin',erster_administrator:false});
window.__TAURI_INTERNALS__.invoke = async (cmd,args={}) => {
  if(cmd==='anmelden') {
    window.calls.push({cmd,args:{anmeldename:args.anmeldename}});
    if(args.kennwort!=='Erstkennwort-Ben') throw {art:'keine_berechtigung',meldung:'Keine Berechtigung: Anmeldename oder Kennwort ist falsch.'};
    window.testAnmeldung={...offen(null),kennwort_aendern:{...window.testUser,kennwort:'erstkennwort'},bisheriges_kennwort_noetig:true};
    return window.testAnmeldung;
  }
  if(cmd==='anmelden_uebergang') {
    window.calls.push({cmd,args});
    window.testAnmeldung={...offen(null),kennwort_aendern:{...window.testUser,kennwort:'keines'},bisheriges_kennwort_noetig:false};
    return window.testAnmeldung;
  }
  if(cmd==='kennwort_aendern') {
    window.calls.push({cmd,args});
    window.testAnmeldung={...offen(null),benutzer:{...window.testUser,kennwort:'gesetzt'}};
    return window.testAnmeldung;
  }
  if(cmd==='abmelden') {
    window.calls.push({cmd,args});
    window.testAnmeldung=offen(null);
    return window.testAnmeldung;
  }
  return base(cmd,args);
};
'''
errors = []
with sync_playwright() as p:
    executable = os.environ.get('BAUPROJEKT_CHROMIUM')
    browser = p.chromium.launch(headless=True, **({'executable_path': executable} if executable else {}))

    def page(extra=''):
        tab = browser.new_page(viewport={'width': 1280, 'height': 900}, timezone_id='Europe/Berlin')
        tab.on('pageerror', lambda e: errors.append(str(e)))
        tab.add_init_script(extra + 'window.fixture=' + json.dumps(fixture) + ';' + (root / 'tauri-mock.js').read_text(encoding='utf-8') + custom)
        tab.goto(os.environ.get('BAUPROJEKT_UI_URL', 'http://127.0.0.1:1430'))
        tab.get_by_role('heading', name='Anmeldung', exact=True).wait_for()
        return tab

    # Kein Übergang angeboten, wenn der Core ihn nicht erlaubt.
    tab = page()
    expect(tab.get_by_role('button', name='Als chefin anmelden und Kennwort festlegen')).to_have_count(0)
    expect(tab.get_by_role('heading', name='Projektverzeichnis', exact=True)).to_have_count(0)
    calls = tab.evaluate('window.calls.map(c=>c.cmd)')
    assert 'app_info' not in calls and 'projekte_auflisten' not in calls, calls

    tab.get_by_label('Anmeldename').fill('Ben')
    tab.get_by_label('Kennwort', exact=True).fill('falsch-falsch')
    tab.get_by_role('button', name='Anmelden', exact=True).click()
    expect(tab.get_by_role('alert')).to_contain_text('Anmeldename oder Kennwort ist falsch')
    expect(tab.get_by_label('Kennwort', exact=True)).to_have_value('')

    tab.get_by_label('Kennwort', exact=True).fill('Erstkennwort-Ben')
    tab.get_by_role('button', name='Anmelden', exact=True).click()
    tab.get_by_role('heading', name='Eigenes Kennwort festlegen').wait_for()
    tab.get_by_label('Bisheriges Kennwort bzw. Erstkennwort').fill('Erstkennwort-Ben')
    tab.get_by_label('Neues Kennwort', exact=True).fill('Ben-eigenes-Kennwort')
    tab.get_by_label('Neues Kennwort wiederholen').fill('anders')
    tab.get_by_role('button', name='Kennwort festlegen und weiter').click()
    expect(tab.get_by_role('alert')).to_contain_text('stimmen nicht überein')
    tab.get_by_label('Neues Kennwort wiederholen').fill('Ben-eigenes-Kennwort')
    tab.get_by_role('button', name='Kennwort festlegen und weiter').click()
    tab.get_by_role('heading', name='Projektverzeichnis', exact=True).wait_for()
    aendern = [c for c in tab.evaluate('window.calls') if c['cmd'] == 'kennwort_aendern']
    assert aendern == [{'cmd': 'kennwort_aendern', 'args': {'bisher': 'Erstkennwort-Ben', 'neu': 'Ben-eigenes-Kennwort'}}], aendern

    # Menü „Abmelden / Benutzer wechseln“ führt zurück zur Anmeldung.
    tab.evaluate('window.emitNative("menu","anwendung.abmelden")')
    tab.get_by_role('heading', name='Anmeldung', exact=True).wait_for()
    tab.close()

    # Übergang für Bestandsbüros ohne Administrator-Kennwort.
    tab = page('window.uebergang=true;')
    knopf = tab.get_by_role('button', name='Als chefin anmelden und Kennwort festlegen')
    expect(knopf).to_be_visible()
    knopf.click()
    tab.get_by_role('heading', name='Eigenes Kennwort festlegen').wait_for()
    expect(tab.get_by_label('Bisheriges Kennwort bzw. Erstkennwort')).to_have_count(0)
    tab.get_by_label('Neues Kennwort', exact=True).fill('Chefin-Kennwort-1')
    tab.get_by_label('Neues Kennwort wiederholen').fill('Chefin-Kennwort-1')
    tab.get_by_role('button', name='Kennwort festlegen und weiter').click()
    tab.get_by_role('heading', name='Projektverzeichnis', exact=True).wait_for()
    aendern = [c for c in tab.evaluate('window.calls') if c['cmd'] == 'kennwort_aendern']
    assert aendern[-1]['args'] == {'bisher': None, 'neu': 'Chefin-Kennwort-1'}, aendern
    tab.close()

    browser.close()

assert not errors, errors
print('Anmeldung: ok')
