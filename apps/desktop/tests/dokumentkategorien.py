"""Produktionsoberfläche mit Core-Vertragsdaten; Persistenz und Rechte in Rust geprüft."""
from pathlib import Path
import json
import os
from playwright.sync_api import sync_playwright, expect

root = Path(__file__).resolve().parent
fixtures = Path(os.environ.get('BAUPROJEKT_KATEGORIEN_ARTEFAKTE', '/tmp/bauprojekt-kategorien'))
wf = json.loads((fixtures / 'workflow.json').read_text())
out = Path(os.environ.get('BAUPROJEKT_KATEGORIEN_UI_ARTEFAKTE', str(root.parents[2] / 'docs/dokumentkategorien')))
out.mkdir(parents=True, exist_ok=True)
custom = '''
const original=window.__TAURI_INTERNALS__.invoke;
const clone=v=>JSON.parse(JSON.stringify(v));
const wf=window.workflow;
window.catalog=clone(wf.katalog);window.docs=clone(wf.dokumente);window.submissionHistory=[clone(wf.paket)];
window.mapping=Object.fromEntries(window.docs.map((d,i)=>[d.document_id,{dokument_id:d.document_id,revision:1,kategorie:i===0?'lageplan':'grz'}]));
window.templates={};window.failCategory=false;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 const handled=['kategorienkatalog','kategorienkatalog_speichern','dokumentkategorien','dokumentkategorie_speichern','vorlagenkategorie','vorlagenkategorie_speichern','dokumente_auflisten','formularbibliothek','einreichungen','einreichungs_vergleich','plugin:dialog|open','dokument_importieren'];
 if(!handled.includes(cmd))return original(cmd,args);
 window.calls.push({cmd,args:clone(args)});
 switch(cmd){
 case 'kategorienkatalog':return clone(window.catalog);
 case 'kategorienkatalog_speichern':if(args.katalog.revision!==window.catalog.revision)throw {art:'konflikt',meldung:'Katalog inzwischen geändert'};window.catalog=clone({...args.katalog,revision:args.katalog.revision+1});return clone(window.catalog);
 case 'dokumentkategorien':return args.projektId===wf.projekt.id?clone(Object.values(window.mapping)):[];
 case 'dokumentkategorie_speichern':{
  if(window.failCategory || window.failOnDoc===args.dokumentId)throw {art:'konflikt',meldung:'Kategorie inzwischen geändert. Bitte erneut prüfen.'};
  const d=window.mapping[args.dokumentId];if(d.revision!==args.zuordnung.revision)throw {art:'konflikt',meldung:'Veraltete Kategorie'};
  window.mapping[args.dokumentId]={dokument_id:args.dokumentId,...clone(args.zuordnung),revision:d.revision+1};return clone(window.mapping[args.dokumentId]);
 }
 case 'vorlagenkategorie':return clone(window.templates[args.hash]??{revision:0,kategorie:null});
 case 'vorlagenkategorie_speichern':window.templates[args.hash]={...clone(args.zuordnung),revision:args.zuordnung.revision+1};return clone(window.templates[args.hash]);
 case 'dokumente_auflisten':return args.projektId===wf.projekt.id?clone(window.docs):[];
 case 'formularbibliothek':return {root:'C:\\BauProjektIO\\Formulare',vorlagen:clone(wf.vorlagen)};
 case 'einreichungen':return clone(window.submissionHistory);
 case 'einreichungs_vergleich':return window.docs.map(d=>({aktuell:clone(d),zuletzt_eingereicht:null,paket_id:null,eingereicht_am:null,status:'neu',kategorie:window.mapping[d.document_id]?.kategorie?(()=>{const k=window.catalog.kategorien.find(k=>k.id===window.mapping[d.document_id].kategorie);return {id:k.id,name:k.name,hauptkategorie:window.catalog.kategorien.find(p=>p.id===k.eltern_id)?.name??null};})():null}));
 case 'plugin:dialog|open':return 'C:/Import/Anschreiben.docx';
 case 'dokument_importieren':{
  const d={...clone(window.docs[0]),document_id:crypto.randomUUID(),version:1,file_name:'Anschreiben.docx'};
  window.docs.push(d);window.mapping[d.document_id]={dokument_id:d.document_id,revision:0,kategorie:args.kategorie??null};return clone(d);
 }
 }
};
'''
errors = []
with sync_playwright() as p:
    browser = p.chromium.launch(headless=True, executable_path=os.environ.get('BAUPROJEKT_CHROMIUM', '/usr/bin/chromium'))
    page = browser.new_page(viewport={'width': 1680, 'height': 1100})
    page.on('pageerror', lambda e: errors.append(str(e)))
    page.add_init_script('window.fixture=' + json.dumps(wf['projekt']) + ';window.workflow=' + json.dumps(wf) + ';' + (root / 'tauri-mock.js').read_text() + custom)
    page.goto(os.environ.get('BAUPROJEKT_UI_URL', 'http://127.0.0.1:1440'))
    page.locator('.projekttabelle').get_by_role('button', name=wf['projekt']['name'], exact=True).click()

    def view(name):
        page.locator('.aktennavigation').get_by_role('button', name=name, exact=True).click()

    view('Projektdateien')
    table = page.get_by_role('table', name='Projektdateien')
    expect(table.get_by_role('row').filter(has=page.get_by_role('cell',name=wf['dokumente'][0]['file_name'],exact=True)).locator('td').nth(2)).to_contain_text('Lageplan')
    page.get_by_role('combobox', name='Kategorie filtern', exact=True).select_option('berechnung')
    expect(table.get_by_role('cell', name=wf['dokumente'][1]['file_name'], exact=True)).to_be_visible()
    expect(table.get_by_role('cell', name=wf['dokumente'][0]['file_name'], exact=True)).to_have_count(0)
    page.get_by_role('combobox', name='Kategorie filtern', exact=True).select_option('')
    page.get_by_label('Sortierung', exact=True).select_option('kategorie')
    expect(table.get_by_role('rowheader', name='Berechnung / GRZ-Berechnung', exact=True)).to_be_visible()
    page.get_by_role('button', name='Sichtbare auswählen', exact=True).click()
    page.get_by_role('button', name='Kategorie zuordnen (2)', exact=True).click()
    dialog = page.get_by_role('dialog', name='Kategorie zuordnen')
    dialog.get_by_label('Kategorie', exact=True).select_option('anschreiben')
    dialog.get_by_role('button', name='Abbrechen', exact=True).click()
    assert not page.evaluate('window.calls.some(c=>c.cmd==="dokumentkategorie_speichern")')
    page.get_by_role('button', name='Kategorie zuordnen (2)', exact=True).click()
    dialog.get_by_label('Kategorie', exact=True).select_option('anschreiben')
    page.evaluate('window.failCategory=true')
    dialog.get_by_role('button', name='Zuordnung speichern', exact=True).click()
    expect(dialog.get_by_role('alert')).to_contain_text('Kategorie inzwischen geändert')
    expect(dialog.get_by_label('Kategorie', exact=True)).to_have_value('anschreiben')
    page.evaluate('window.failCategory=false')
    dialog.get_by_role('button', name='Zuordnung speichern', exact=True).click()
    expect(dialog).to_have_count(0)
    assert page.evaluate('Object.values(window.mapping).every(k=>k.kategorie==="anschreiben")')
    assert page.evaluate('window.docs.every(d=>d.version===window.workflow.dokumente.find(x=>x.document_id===d.document_id).version)')
    # Ein Fehler nach der ersten Speicherung erhält nur die noch offenen Aufträge.
    page.get_by_role('button', name='Sichtbare auswählen', exact=True).click()
    page.get_by_role('button', name='Kategorie zuordnen (2)', exact=True).click()
    dialog.get_by_label('Kategorie', exact=True).select_option('anschreiben')
    page.evaluate('window.failOnDoc=window.docs[0].document_id')
    dialog.get_by_role('button', name='Zuordnung speichern', exact=True).click()
    expect(dialog.get_by_role('alert')).to_contain_text('1 Zuordnung(en) gespeichert')
    expect(dialog.get_by_text('1 ausgewählte Unterlage(n). Die Zuordnung gilt auch für künftige Dateiversionen.', exact=True)).to_be_visible()
    page.evaluate('window.failOnDoc=null')
    dialog.get_by_role('button', name='Zuordnung speichern', exact=True).click()
    expect(dialog).to_have_count(0)
    page.get_by_label('Kategorie für neue Importe', exact=True).select_option('anschreiben')
    page.get_by_role('button', name='Dateien importieren …', exact=True).click()
    expect(table.get_by_role('cell', name='Anschreiben.docx', exact=True)).to_be_visible()
    assert page.evaluate('window.calls.find(c=>c.cmd==="dokument_importieren").args.kategorie') == 'anschreiben'
    page.screenshot(path=str(out / '01-dokumentkategorien.png'))

    page.get_by_role('button', name='Einstellungen', exact=True).click()
    settings = page.locator('.kategorien-einstellungen')
    settings.locator('summary').click()
    settings.get_by_role('button', name='Kategorie hinzufügen', exact=True).click()
    settings.get_by_label('Name', exact=True).fill('Brandschutzkonzept')
    settings.get_by_label('Hauptkategorie', exact=True).select_option('gutachten')
    settings.get_by_role('button', name='Kategorie speichern', exact=True).click()
    expect(settings.get_by_role('cell', name='Gutachten / Brandschutzkonzept', exact=True)).to_be_visible()
    row = settings.get_by_role('row').filter(has=page.get_by_role('cell', name='Lageplan', exact=True))
    row.get_by_role('button', name='Bearbeiten', exact=True).click()
    settings.get_by_label('Name', exact=True).fill('Amtlicher Lageplan')
    settings.get_by_role('button', name='Kategorie speichern', exact=True).click()
    expect(settings.get_by_role('cell', name='Amtlicher Lageplan', exact=True)).to_be_visible()
    page.screenshot(path=str(out / '02-kategorienkatalog.png'))

    page.get_by_role('button', name='Formularbibliothek', exact=True).click()
    page.get_by_role('button', name='Kategorie ändern: Muster.pdf', exact=True).click()
    template_dialog = page.get_by_role('dialog', name='Standardkategorie der Vorlage')
    template_dialog.get_by_label('Kategorie', exact=True).select_option('baubeschreibung')
    template_dialog.get_by_role('button', name='Standardkategorie speichern', exact=True).click()
    expect(template_dialog).to_have_count(0)
    expect(page.get_by_role('cell').filter(has_text='Baubeschreibung')).to_be_visible()
    page.screenshot(path=str(out / '03-vorlagenkategorie.png'))

    # Return to the retained project tab; refreshing gets the changed catalog.
    page.get_by_role('navigation', name='Geöffnete Projekte').get_by_role('button', name=wf['projekt']['name'], exact=True).click()
    view('Formulare')
    page.get_by_role('button', name='Bibliothek aktualisieren', exact=True).click()
    expect(page.get_by_role('cell', name='Baubeschreibung', exact=True)).to_be_visible()
    view('Einreichungen')
    page.get_by_role('combobox', name='Kategorie filtern', exact=True).select_option('anschreiben')
    page.get_by_role('button', name='Gefilterte Unterlagen auswählen', exact=True).click()
    expect(page.get_by_text('3 Unterlage(n) ausgewählt, auch außerhalb des Filters', exact=True)).to_be_visible()
    package = page.locator('details.paket')
    package.locator('summary').click()
    expect(package.get_by_role('cell', name='Lageplan', exact=True)).to_be_visible()
    expect(package.get_by_role('cell', name='Amtlicher Lageplan', exact=True)).to_have_count(0)
    page.screenshot(path=str(out / '04-kategoriehistorie.png'))
    assert not errors, errors
    browser.close()
print('Dokumentkategorien: Filter, Gruppen, Sammelzuordnung, Abbruch/Konflikt, Import, Katalog, Vorlagen und historische Kategorien erfolgreich geprüft')
