"""Real feature editors: independent writes, crash recovery and conflict preservation."""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
root=Path(__file__).resolve().parent
fixture=json.loads((root/'fixtures/projekt.json').read_text());fixture['name']='Plattformprojekt'
out=Path(os.environ.get('BAUPROJEKT_UI_ARTEFAKTE',str(root.parents[2]/'docs/plattform-umsetzung/screenshots')));out.mkdir(parents=True,exist_ok=True)
custom=r'''
const original=window.__TAURI_INTERNALS__.invoke;
window.failFeature=false;window.cancelBackup=true;
window.sicherungsManifest={schema_version:1,erstellt_unix:0,dateien:[{name:'bauprojekt.db',sha256:'a'.repeat(64),bytes:100}]};
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 if(cmd==='formularbibliothek')return {root:'/tmp/Formulare',vorlagen:[]};
 if(cmd==='projektordner_status')return '/tmp/Projekte';
 if(cmd==='plugin:dialog|save'){window.calls.push({cmd,args});return window.cancelBackup?null:'/tmp/sicherung.zip';}
 if(cmd==='plugin:dialog|open'){window.calls.push({cmd,args});return args.options?.directory?'/tmp/Profile':'/tmp/sicherung.zip';}
 if(['sicherung_erstellen','sicherung_pruefen'].includes(cmd)){window.calls.push({cmd,args});return window.sicherungsManifest;}
 if(cmd==='sicherung_wiederherstellen'){window.calls.push({cmd,args});return args.ziel;}
 if(['profil_aktivieren','standardprofil_aktivieren'].includes(cmd)){window.calls.push({cmd,args});return;}
 if(cmd==='dokumente_auflisten')return [{document_id:'43216955-c69e-4373-87eb-b3251f26964c',project_id:args.projektId,version:1,file_name:'Grundriss.pdf',storage_key:'test/grundriss.pdf',sha256:'a'.repeat(64),size_bytes:100,created_by:'Test',created_at:window.fixture.created_at}];
 if(!['fachobjekte','fachverlauf','fachobjekt_speichern','raumbuch_summen'].includes(cmd))return original(cmd,args);
 window.calls.push({cmd,args:JSON.parse(JSON.stringify(args))});
 const key='test.features.'+args.projektId+'.'+args.art;
 const rows=JSON.parse(localStorage.getItem(key)||'[]');
 if(cmd==='fachobjekte')return rows;
 if(cmd==='fachverlauf')return rows.filter(f=>f.objekt===args.id);
 if(cmd==='raumbuch_summen')return {anzahl:1,flaeche_m2:20,volumen_m3:50};
 if(window.failFeature)throw {art:'konflikt',meldung:'Fachstand inzwischen geändert. Entwurf bleibt erhalten.'};
 const prev=rows.find(f=>f.objekt===args.id);
 if((prev?.revision||0)!==args.basis)throw {art:'konflikt',meldung:'Versionskonflikt'};
 const f={projekt_id:args.projektId,modul:args.art,objekt:args.id,schema_version:1,revision:args.basis+1,daten:args.daten};
 localStorage.setItem(key,JSON.stringify([...rows.filter(x=>x.objekt!==args.id),f]));return f;
};
'''
errors=[]
with sync_playwright() as p:
    browser=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
    page=browser.new_page(viewport={'width':1440,'height':1000},color_scheme='light');page.on('pageerror',lambda e:errors.append(str(e)))
    page.add_init_script('window.fixture='+json.dumps(fixture)+';'+(root/'tauri-mock.js').read_text()+custom)
    page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1434'))
    def open_project():
        table=page.locator('.projekttabelle')
        if table.is_visible():table.get_by_role('button',name='Plattformprojekt',exact=True).click()
    def view(name):page.get_by_role('complementary',name='Arbeitsbereiche').get_by_role('button',name=name,exact=True).click()
    def section():return page.locator('.projekt-sitzung:visible > main > section:not([hidden])')
    open_project();view('Gebäude / Geschosse');section().get_by_role('button',name='Neu anlegen',exact=True).click()
    section().get_by_label('Gebäudebezeichnung',exact=True).fill('Haus A')
    section().get_by_role('button',name='Geschoss hinzufügen',exact=True).click()
    section().get_by_label('Bezeichnung',exact=True).fill('Erdgeschoss')
    section().get_by_label('Einstufung',exact=True).select_option(label='Vollgeschoss')
    section().get_by_label('Grundlage der Einstufung',exact=True).fill('Fachlich geprüftes Vollgeschoss')
    section().get_by_role('button',name='Speichern',exact=True).click()
    expect(section().get_by_role('cell',name='Haus A',exact=True)).to_be_visible()
    view('Aufgaben / Fristen');section().get_by_role('button',name='Neu anlegen',exact=True).click()
    section().get_by_label('Aufgabe',exact=True).fill('Unterlagen abstimmen')
    page.wait_for_function("Object.keys(localStorage).some(k=>k.startsWith('bauprojekt.entwurf.v1.')&&k.endsWith('.aufgaben'))")
    page.reload();open_project();view('Aufgaben / Fristen')
    section().get_by_role('button',name='Entwurf wiederherstellen',exact=True).click()
    expect(section().get_by_label('Aufgabe',exact=True)).to_have_value('Unterlagen abstimmen')
    page.evaluate('window.failFeature=true');section().get_by_role('button',name='Speichern',exact=True).click()
    expect(section().get_by_role('alert')).to_contain_text('Entwurf bleibt erhalten')
    expect(section().get_by_label('Aufgabe',exact=True)).to_have_value('Unterlagen abstimmen')
    page.evaluate('window.failFeature=false');section().get_by_role('button',name='Speichern',exact=True).click()
    expect(section().get_by_role('status')).to_have_text('Gespeichert.')
    view('Flächen- und Raumbuch');section().get_by_role('button',name='Neu anlegen',exact=True).click()
    section().get_by_label('Bezeichnung',exact=True).fill('Raumbuch Haus A')
    section().get_by_role('button',name='Raum hinzufügen',exact=True).click()
    section().get_by_label('Raumnummer',exact=True).fill('001');section().get_by_label('Raumbezeichnung',exact=True).fill('Büro')
    section().get_by_label('Raumfläche m²',exact=True).fill('20');section().get_by_label('Lichte Raumhöhe m',exact=True).fill('2.5')
    section().get_by_role('button',name='Speichern und Summen ermitteln',exact=True).click()
    expect(section().get_by_role('status').filter(has_text='1 Räume')).to_contain_text('20 m²')
    page.screenshot(path=str(out/'raumbuch.png'))
    view('Projektdateien')
    section().get_by_label('Weitere Aktionen für Grundriss.pdf',exact=True).click()
    section().get_by_role('button',name='Dokumentangaben …',exact=True).click()
    dialog=page.get_by_role('dialog')
    dialog.get_by_label('Dokumentart',exact=True).fill('Bauzeichnung')
    dialog.get_by_label('Plannummer',exact=True).fill('A-101')
    dialog.get_by_label('Revisionsindex',exact=True).fill('B')
    dialog.get_by_role('button',name='Speichern',exact=True).click()
    expect(dialog.get_by_role('status')).to_have_text('Gespeichert.')
    dialog.get_by_role('button',name='Schließen',exact=True).click()
    page.wait_for_function("window.calls.some(c=>c.cmd==='fachobjekt_speichern'&&c.args.art==='dokumentdaten'&&c.args.daten.plannummer==='A-101')")
    assert not page.evaluate("window.calls.some(c=>c.cmd==='projekt_aktualisieren')")
    page.get_by_role('complementary',name='Arbeitsbereiche').get_by_role('button',name='Einstellungen',exact=True).click()
    page.get_by_role('button',name='Sicherung erstellen …',exact=True).click()
    assert not page.evaluate("window.calls.some(c=>c.cmd==='sicherung_erstellen')")
    page.evaluate('window.cancelBackup=false')
    page.get_by_role('button',name='Sicherung erstellen …',exact=True).click()
    expect(page.get_by_role('status').filter(has_text='Sicherung erstellt und geprüft')).to_be_visible()
    page.get_by_role('button',name='In neues Profil wiederherstellen …',exact=True).click()
    expect(page.get_by_role('status').filter(has_text='Wiederhergestellt und geprüft')).to_be_visible()
    assert not page.evaluate("window.calls.some(c=>c.cmd==='profil_aktivieren')")
    page.get_by_role('button',name='Wiederhergestelltes Profil nach Neustart verwenden',exact=True).click()
    page.wait_for_function("window.calls.some(c=>c.cmd==='profil_aktivieren')")
    page.screenshot(path=str(out/'sicherungen.png'))
    assert not errors,errors
    browser.close()
print('Plattform: Fachmodule, Entwurfsrettung, Dokumentangaben und getrennte Sicherungs-/Profilabläufe geprüft.')
