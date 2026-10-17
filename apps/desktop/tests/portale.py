"""Portalablauf mit echten Core-Ständen; nur der native Transport ist simuliert."""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich, register, gliederung
root = Path(__file__).resolve().parent
art = Path(os.environ.get('BAUPROJEKT_PORTAL_ARTEFAKTE', '/tmp/bauprojekt-portale'))
wf = json.loads((art / 'workflow.json').read_text())
out = Path(os.environ.get('BAUPROJEKT_PORTAL_UI_ARTEFAKTE', '/tmp/bauprojekt-portale-ui'))
out.mkdir(parents=True, exist_ok=True)
custom = r'''
const base=window.__TAURI_INTERNALS__.invoke, clone=x=>JSON.parse(JSON.stringify(x));
window.testUser=clone(window.portalData.benutzer);window.portalAktuell=clone(window.portalData.bereit);
window.portalFail=false;window.portalEreignisse=0;window.portalDelay=0;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 if(cmd==='portalprofile')return clone(window.portalData.profile);
 if(cmd==='xbau_nachrichten')return [];
 if(cmd==='fachobjekte'&&args.art==='verfahren')return args.projektId===window.fixture.id?[clone(window.portalAktuell)]:[];
 if(cmd==='dokumente_auflisten')return args.projektId===window.fixture.id?clone(window.portalData.dateien):[];
 if(cmd==='plugin:dialog|save'){window.calls.push({cmd,args});return '/tmp/Portalunterlagen.zip';}
 if(cmd.startsWith('portal_')){
  window.calls.push({cmd,args:clone(args)});
  if(cmd==='portal_ereignis_erfassen'){
   if(window.portalDelay)await new Promise(r=>setTimeout(r,window.portalDelay));
   if(window.portalFail)throw {art:'konflikt',meldung:'Test: Akte wurde inzwischen geändert'};
   window.portalEreignisse++;
   window.portalAktuell=clone(window.portalEreignisse===1?window.portalData.ungeklaert:window.portalData.belegt);
   return clone(window.portalAktuell);
  }
  if(cmd==='portal_rueckmeldung_erfassen'){window.portalAktuell=clone(window.portalData.rueckmeldung);return clone(window.portalAktuell);}
  if(cmd==='portal_abgabe_pruefen')return clone(window.portalData.vorbereitung);
  if(['portal_oeffnen','portal_paket_exportieren','portal_datei_exportieren'].includes(cmd))return;
 }
 return base(cmd,args);
};
'''
errors=[]
with sync_playwright() as pw:
    browser=pw.chromium.launch(headless=True, executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
    init='window.fixture='+json.dumps(wf['projekt'])+';window.portalData='+json.dumps(wf)+';'+(root/'tauri-mock.js').read_text()+custom
    url=os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1451')
    def start(extra=''):
        p=browser.new_page(viewport={'width':1600,'height':1100},locale='de-DE')
        p.on('pageerror',lambda e:errors.append(str(e)));p.add_init_script(init+extra);p.goto(url)
        p.locator('.projekttabelle').get_by_role('button',name=wf['projekt']['name'],exact=True).click()
        bereich(p,'Einreichen / Portal')
        return p
    page=start()
    def area():return page.locator('.projekt-sitzung:visible [data-portale]')
    def step(name):area().get_by_role('button',name=name,exact=True).click()
    expect(area()).to_contain_text('Festgehaltenes Portalziel')
    page.screenshot(path=str(out/'portal-ziel-hell.png'))
    step('3 · Übergabe / Beleg');area().get_by_label('Portalabgabe',exact=True).select_option(wf['abgabe'])
    expect(area()).to_contain_text('Vorbereitet · keine Abgabe belegt')
    area().get_by_role('button',name='Portal im Systembrowser öffnen',exact=True).click()
    expect(area()).to_contain_text('Kein Versandstatus geändert.')
    area().get_by_role('button',name='Bereitstellungs-ZIP exportieren …',exact=True).click()
    expect(area()).to_contain_text('Export bestätigt keine Portalabgabe.')
    assert page.evaluate('window.calls.filter(c=>c.cmd==="portal_ereignis_erfassen").length')==0
    area().get_by_label('Portalreferenz',exact=True).fill('PORTAL-2026-42')
    # Belegentwürfe sind zusätzlich an den konkreten Abgabestand gebunden.
    area().get_by_label('Portalabgabe',exact=True).select_option('')
    area().get_by_label('Portalabgabe',exact=True).select_option(wf['abgabe'])
    expect(area().get_by_label('Portalreferenz',exact=True)).to_have_value('PORTAL-2026-42')
    bereich(page,'Stammdaten')
    bereich(page,'Einreichen / Portal')
    expect(area().get_by_label('Portalreferenz',exact=True)).to_have_value('PORTAL-2026-42')
    # Ein anderes Projekt erhält weder die Verfahrensakte noch deren Entwurf.
    bereich(page,'Projekte')
    page.locator('.projekttabelle').get_by_role('button',name='Umbau Bürogebäude',exact=True).click()
    bereich(page,'Einreichen / Portal')
    expect(area()).to_contain_text('Für die Portalunterstützung zuerst eine Verfahrensakte')
    page.locator('.tab-auswahl').filter(has_text=wf['projekt']['name']).click()
    expect(area().get_by_label('Portalreferenz',exact=True)).to_have_value('PORTAL-2026-42')
    area().get_by_label('Belegdatum',exact=True).fill('2026-10-15')
    area().get_by_label('Originalbeleg',exact=True).select_option(f"{wf['beleg']['dokument_id']}:{wf['beleg']['version']}")
    # Ein Konflikt darf den Entwurf nicht löschen.
    page.evaluate('window.portalFail=true')
    area().get_by_role('button',name='Beleg und Verfahrensstand speichern',exact=True).click()
    expect(area().get_by_role('alert')).to_contain_text('inzwischen geändert')
    expect(area().get_by_label('Portalreferenz',exact=True)).to_have_value('PORTAL-2026-42')
    page.evaluate('window.portalFail=false')
    area().get_by_role('button',name='Beleg und Verfahrensstand speichern',exact=True).click()
    expect(area()).to_contain_text('Übergabe / Dateiabgleich ungeklärt')
    expect(area().get_by_label('Portalreferenz',exact=True)).to_have_value('')
    # Vollständiger Dateiabgleich wird ausdrücklich bestätigt; kein Export leitet ihn ab.
    area().get_by_label('Portalreferenz',exact=True).fill('PORTAL-2026-42')
    area().get_by_label('Originalbeleg',exact=True).select_option(f"{wf['beleg']['dokument_id']}:{wf['beleg']['version']}")
    area().get_by_label('Dateiabgleich',exact=True).select_option('vollstaendig')
    area().get_by_label('Ich habe alle unten aufgeführten konkreten Dateiversionen mit der tatsächlichen Portalabgabe abgeglichen.',exact=True).check()
    first=wf['ungeklaert']['daten']['portal']['vorgaenge'][0]['abgaben'][0]['ereignisse'][0]['id']
    area().get_by_label('Berichtigung von',exact=True).select_option(first)
    page.evaluate('window.portalDelay=300')
    area().get_by_role('button',name='Beleg und Verfahrensstand speichern',exact=True).click()
    expect(area().get_by_label('Verfahrensakte',exact=True)).to_be_disabled()
    # Eine verspätete Antwort bleibt bei der ursprünglichen Projektakte.
    page.locator('.tab-auswahl').filter(has_text='Umbau Bürogebäude').click()
    page.wait_for_function('window.portalEreignisse===2')
    expect(area()).to_contain_text('Für die Portalunterstützung zuerst eine Verfahrensakte')
    page.locator('.tab-auswahl').filter(has_text=wf['projekt']['name']).click()
    expect(area()).to_contain_text('Portalbeleg und Verfahrensstand gemeinsam gespeichert.')
    expect(area()).to_contain_text('Abgabe belegt')
    sent=page.evaluate('window.calls.filter(c=>c.cmd==="portal_ereignis_erfassen").at(-1).args.wunsch')
    assert sent['dateiabgleich']=='vollstaendig' and sent['beleg']==wf['beleg'] and sent['berichtigt']==first
    assert sent['bestaetigte_dateien']==[d['original']['dokument'] for d in wf['bereit']['daten']['portal']['vorgaenge'][0]['abgaben'][0]['dateien']]
    page.screenshot(path=str(out/'portal-beleg-hell.png'))
    step('4 · Rückmeldungen')
    area().get_by_label('Betreff',exact=True).fill('Nachforderung zum Bauantrag')
    original=next(d for d in wf['dateien'] if d['file_name']=='Nachforderung.pdf')
    area().get_by_label('Original der Rückmeldung',exact=True).select_option(f"{original['document_id']}:{original['version']}")
    area().get_by_role('button',name='Rückmeldung in der Verfahrensakte speichern',exact=True).click()
    expect(area()).to_contain_text('Rückmeldung mit Originalbezug als Kommunikation gespeichert.')
    page.evaluate("document.documentElement.dataset.theme='dark';document.documentElement.style.colorScheme='dark'")
    page.screenshot(path=str(out/'portal-rueckmeldung-dunkel.png'))
    for width in [900,600]:
        page.set_viewport_size({'width':width,'height':1000})
        assert page.evaluate('document.documentElement.scrollWidth<=innerWidth'),width
    reader=start(';window.testUser.rolle="lesen";')
    reader.locator('[data-portale]').get_by_role('button',name='3 · Übergabe / Beleg',exact=True).click()
    reader.locator('[data-portale]').get_by_label('Portalabgabe',exact=True).select_option(wf['abgabe'])
    expect(reader.locator('[data-portale]').get_by_role('button',name='Beleg und Verfahrensstand speichern',exact=True)).to_be_disabled()
    expect(reader.locator('[data-portale]').get_by_role('button',name='Bereitstellungs-ZIP exportieren …',exact=True)).to_be_enabled()
    assert not errors,errors
    browser.close()
print('Portal: Originalexport, keine fingierte Abgabe, Konflikterhalt, expliziter Dateiabgleich, Berichtigung, Rückmeldung, Hell/Dunkel, Responsivität und Leserechte geprüft.')
