"""Gebäude/Geschosse: Flurstücke aus gespeicherten Grundstücksinformationen
anzeigen, neues Gebäude anlegen und mit Flurstückszuordnung speichern.

Regression: Bei Projekten mit gespeicherten Grundstücksinformationen brach die
Flurstücksliste ab und „Neu anlegen“ schlug fehl (structuredClone auf einem
reaktiven Zustand). Geprüft werden Bestandsformat und getrennte Fakten.
"""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich

root = Path(__file__).resolve().parent
basis = json.loads((root / 'fixtures/projekt.json').read_text(encoding='utf-8'))


def getrennt(p):
    """Projekt mit Grundstücksinformationen statt Bestandsformat."""
    p = json.loads(json.dumps(p))
    r = p['planungsrecht']
    r['grundstuecksinformationen'] = {'flurstuecke': [
        {k: f[k] for k in ('id', 'gemarkung', 'flur', 'nummer', 'flaeche_m2', 'grundbuchbezirk', 'grundbuchblatt')}
        | {'laufende_nummer': f['laufende_nummer'], 'grundbuchauszug': None, 'eigentuemer': '', 'bemerkung': ''}
        for f in r['baugrundstuecke']['flurstuecke']]}
    return p


transport = r'''
{
 const original = window.__TAURI_INTERNALS__.invoke, kopie = v => JSON.parse(JSON.stringify(v));
 window.gebaeude = []; window.gespeichert = [];
 window.__TAURI_INTERNALS__.invoke = async (cmd, args = {}) => {
  if (cmd === 'fachobjekte' && args.art === 'gebaeude') return kopie(window.gebaeude);
  if (cmd === 'gebaeude_auswerten') return {rechenstand: 'test', gebaeude_id: args.id, bezeichnung: args.gebaeude.bezeichnung, typ: null, grundflaeche_m2: null, geschossflaechensumme_m2: null, geschosse: [], offen: [], hoehen_nhn: {gelaende_m: null, traufe_m: null, first_m: null}};
  if (cmd === 'fachobjekt_speichern' && args.art === 'gebaeude') {
   window.gespeichert.push(kopie(args.daten));
   const stand = {projekt_id: args.projektId, modul: 'gebaeude', objekt: args.id, schema_version: 1, revision: args.basis + 1, daten: kopie(args.daten)};
   window.gebaeude = [...window.gebaeude.filter(g => g.objekt !== args.id), stand];
   return kopie(stand);
  }
  return original(cmd, args);
 };
}
'''

fehler = []
with sync_playwright() as pw:
    b = pw.chromium.launch(headless=True, executable_path=os.environ.get('BAUPROJEKT_CHROMIUM', '/usr/bin/chromium'))
    for name, projekt in (('Bestandsformat', basis), ('Grundstücksinformationen', getrennt(basis))):
        page = b.new_page(viewport={'width': 1500, 'height': 1000}, locale='de-DE')
        page.on('pageerror', lambda e: fehler.append(f'{name}: {e}'))
        page.add_init_script('window.fixture=' + json.dumps(projekt) + ';' + (root / 'tauri-mock.js').read_text(encoding='utf-8') + transport)
        page.goto(os.environ.get('BAUPROJEKT_UI_URL', 'http://127.0.0.1:1430'))
        page.locator('.projekttabelle').get_by_role('button', name=projekt['name'], exact=True).click()
        bereich(page, 'Gebäude / Geschosse')
        akte = page.locator('.projekt-sitzung:visible')
        akte.get_by_role('button', name='Neu anlegen', exact=True).click()
        akte.get_by_label('Gebäudebezeichnung').fill('Wohnhaus')
        lage = akte.locator('fieldset').filter(has=page.get_by_text('Lage auf Flurstücken', exact=True))
        flur = lage.get_by_role('checkbox', name='Muenchen · Flur 1 · 100/1')
        expect(flur).to_be_visible()
        flur.check()
        akte.get_by_role('button', name='Speichern', exact=True).click()
        page.wait_for_function('window.gespeichert.length === 1')
        daten = page.evaluate('window.gespeichert[0]')
        assert daten['flurstueck_ids'] == ['a'], (name, daten)
        assert daten['grundstueck_ids'] == [], (name, daten)
        page.close()
    b.close()
assert not fehler, fehler
print('Gebäude/Flurstücke: Bestandsformat und Grundstücksinformationen in Ordnung')
