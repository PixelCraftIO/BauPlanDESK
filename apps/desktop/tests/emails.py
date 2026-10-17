"""Produktionsoberfläche mit tatsächlich von Core gelesenen EML-/MSG-Inhalten."""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich, register, gliederung
root=Path(__file__).resolve().parent
wf=json.loads((Path(os.environ.get('BAUPROJEKT_EMAIL_ARTEFAKTE','/tmp/bauprojekt-emails'))/'workflow.json').read_text())
out=Path(os.environ.get('BAUPROJEKT_EMAIL_UI_ARTEFAKTE',str(root.parents[2]/'docs/email-archiv')));out.mkdir(parents=True,exist_ok=True)
custom=r'''
const base=window.__TAURI_INTERNALS__.invoke, clone=v=>JSON.parse(JSON.stringify(v));
const wf=window.mailWorkflow;
window.mailDocs=[clone(wf.eml1.dokument),clone(wf.eml2.dokument),clone(wf.msg.dokument)];
window.mailKategorien=clone(wf.zuordnungen);window.mailFailKategorie=false;window.mailFail=false;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 const handled=['kategorienkatalog','dokumentkategorien','dokumentkategorie_speichern','dokumente_auflisten','dokument_historie','email_laden','email_anhang_ablegen','fachobjekt_speichern'];
 if(!handled.includes(cmd))return base(cmd,args);
 window.calls.push({cmd,args:clone(args)});
 switch(cmd){
 case 'kategorienkatalog':return clone(wf.katalog);
 case 'dokumentkategorien':return clone(window.mailKategorien);
 case 'dokumentkategorie_speichern':{
   if(window.mailFailKategorie)throw {art:'konflikt',meldung:'Kategorie inzwischen geändert. Eingabe bleibt erhalten.'};
   const k=window.mailKategorien.find(k=>k.dokument_id===args.dokumentId);
   if(k.revision!==args.zuordnung.revision)throw {art:'konflikt',meldung:'Veraltete Zuordnung'};
   Object.assign(k,clone(args.zuordnung),{revision:k.revision+1});return clone(k);
 }
 case 'fachobjekt_speichern':if(window.mailMetaFail)throw {art:'eingabe',meldung:'Dokumentangaben konnten nicht gespeichert werden'};return {projekt_id:args.projektId,modul:args.art,objekt:args.id,schema_version:1,revision:args.basis+1,daten:clone(args.daten)};
 case 'dokumente_auflisten':return clone(window.mailDocs);
 case 'dokument_historie':return clone(window.mailDocs.filter(d=>d.document_id===args.dokumentId));
 case 'email_anhang_ablegen':return clone(wf.anhaenge[args.dokumentId===wf.msg.dokument.document_id?1:0]);
 case 'email_laden':{
   if(window.mailFail)throw {art:'eingabe',meldung:'Die E-Mail konnte nicht gelesen werden. Das Original bleibt erhalten.'};
   const data=args.dokumentId===wf.msg.dokument.document_id?wf.msg:args.version===1?wf.eml1:wf.eml2;
   if(window.mailHoldNext){window.mailHoldNext=false;await new Promise(r=>window.releaseMail=r);}
   const result=clone(data);
   if(window.mailMalicious)result.inhalt.text+='<img src="https://mail-tracker.invalid/pixel" onerror="window.mailCode=true"><script>window.mailCode=true</script>';
   return result;
 }
 }
};
'''
errors=[];requests=[]
with sync_playwright() as p:
    browser=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
    page=browser.new_page(viewport={'width':1680,'height':1100})
    page.on('pageerror',lambda e:errors.append(str(e)));page.on('request',lambda r:requests.append(r.url))
    page.add_init_script('window.fixture='+json.dumps(wf['projekt'])+';window.mailWorkflow='+json.dumps(wf)+';'+(root/'tauri-mock.js').read_text()+custom)
    page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1441'))
    page.locator('.projekttabelle').get_by_role('button',name=wf['projekt']['name'],exact=True).click()
    def view(n):bereich(page,n)
    view('Projektdateien')
    expect(page.get_by_role('button',name='Alle Unterlagen (2)',exact=True)).to_be_visible()
    expect(page.get_by_role('button',name='Ohne Kategorie (1)',exact=True)).to_be_visible()
    page.get_by_role('button',name='Ohne Kategorie (1)',exact=True).click()
    table=page.get_by_role('table',name='Projektdateien')
    expect(table.locator('tbody tr')).to_have_count(1)
    expect(table.get_by_role('cell',name='Bauantrag.eml',exact=True)).to_be_visible()
    page.get_by_role('button',name='Alle Unterlagen (2)',exact=True).click()
    expect(page.get_by_role('combobox',name='Kategorie filtern',exact=True).locator('option[value="email"]')).to_have_text('E-Mail (1)')
    row=table.get_by_role('row').filter(has=page.get_by_role('cell',name='Bauantrag.eml',exact=True))
    row.locator('summary').click();row.get_by_role('button',name='Dokumentangaben …',exact=True).click()
    meta=page.get_by_role('dialog',name='Dokumentangaben',exact=True)
    expect(meta.get_by_label('Kategorie',exact=True)).to_have_value('')
    meta.get_by_label('Kategorie',exact=True).select_option('email')
    page.evaluate('window.mailFailKategorie=true')
    meta.get_by_role('button',name='Speichern',exact=True).click()
    expect(meta.get_by_role('alert')).to_contain_text('Kategorie inzwischen geändert')
    expect(meta.get_by_label('Kategorie',exact=True)).to_have_value('email')
    page.evaluate('window.mailFailKategorie=false')
    meta.get_by_role('button',name='Speichern',exact=True).click()
    expect(meta.get_by_text('Kategorie gespeichert.',exact=True)).to_be_visible()
    meta.get_by_role('button',name='Schließen',exact=True).click()
    expect(page.get_by_role('button',name='Ohne Kategorie (0)',exact=True)).to_be_visible()
    assert not page.evaluate('window.calls.some(c=>c.cmd==="fachobjekt_speichern")'), 'Kategorie allein legt keine leeren Dokumentangaben an'
    assert page.evaluate('window.mailDocs.length')==3, 'Zwei Versionen sind ein Dokument'
    # Kategorie und versionsbezogene Angaben haben getrennte Commit-Grenzen.
    row.get_by_role('button',name='Dokumentangaben …',exact=True).click()
    meta.get_by_label('Kategorie',exact=True).select_option('anschreiben')
    meta.get_by_label('Dokumentart',exact=True).fill('E-Mail-Nachricht')
    page.evaluate('window.mailMetaFail=true')
    meta.get_by_role('button',name='Speichern',exact=True).click()
    expect(meta.get_by_role('alert')).to_contain_text('Dokumentangaben konnten nicht gespeichert werden')
    expect(meta.get_by_text('Kategorie gespeichert.',exact=True)).to_be_visible()
    before=page.evaluate('window.calls.filter(c=>c.cmd==="dokumentkategorie_speichern").length')
    page.evaluate('window.mailMetaFail=false')
    meta.get_by_role('button',name='Speichern',exact=True).click()
    expect(meta.get_by_text('Gespeichert.',exact=True)).to_be_visible()
    assert before==page.evaluate('window.calls.filter(c=>c.cmd==="dokumentkategorie_speichern").length')
    meta.get_by_label('Kategorie',exact=True).select_option('email')
    meta.get_by_role('button',name='Speichern',exact=True).click()
    expect(meta.get_by_role('button',name='Speichern',exact=True)).to_be_disabled()
    meta.get_by_role('button',name='Schließen',exact=True).click()
    row.get_by_role('button',name='E-Mail lesen',exact=True).click()
    mail=page.get_by_role('dialog',name='Archivierte E-Mail',exact=True)
    expect(mail.get_by_role('heading',name='Bauantrag korrigiert',exact=True)).to_be_visible()
    expect(mail.get_by_text('Daniel Müller <daniel@example.org>',exact=True)).to_be_visible()
    expect(mail.get_by_role('table',name='E-Mail-Anhänge').get_by_role('cell',name='Lageplan.pdf',exact=True)).to_be_visible()
    mail.get_by_label('E-Mail-Version',exact=True).select_option('1')
    expect(mail.get_by_role('heading',name='Bauantrag Musterstrasse',exact=True)).to_be_visible()
    mail.get_by_role('button',name='Als Projektunterlage übernehmen',exact=True).click()
    expect(mail.get_by_role('status').filter(has_text='Lageplan.pdf im Projekt gespeichert.')).to_be_visible()
    imported=page.evaluate("window.calls.find(c=>c.cmd==='email_anhang_ablegen').args")
    assert imported['dokumentId']==wf['eml1']['dokument']['document_id'] and imported['version']==1 and imported['index']==0
    page.screenshot(path=str(out/'01-eml.png'))
    # Eine ältere Anfrage darf die danach ausgewählte Version nicht ersetzen.
    mail.get_by_label('E-Mail-Version',exact=True).select_option('2')
    expect(mail.get_by_role('heading',name='Bauantrag korrigiert',exact=True)).to_be_visible()
    page.evaluate('window.mailHoldNext=true')
    mail.get_by_label('E-Mail-Version',exact=True).select_option('1')
    page.wait_for_function('typeof window.releaseMail === "function"')
    mail.get_by_label('E-Mail-Version',exact=True).select_option('2')
    expect(mail.get_by_role('heading',name='Bauantrag korrigiert',exact=True)).to_be_visible()
    page.evaluate('window.releaseMail()')
    expect(mail.get_by_role('heading',name='Bauantrag korrigiert',exact=True)).to_be_visible()
    mail.get_by_role('button',name='Schließen',exact=True).click()
    msgrow=table.get_by_role('row').filter(has=page.get_by_role('cell',name='Nachforderung.msg',exact=True))
    msgrow.get_by_role('button',name='E-Mail lesen',exact=True).click()
    expect(mail.get_by_role('heading',name='Nachforderung Lageplan',exact=True)).to_be_visible()
    expect(mail.get_by_text('Bauamt München <amt@example.org>',exact=True)).to_be_visible()
    expect(mail.locator('pre')).to_contain_text('Bitte ergänzen Sie den Lageplan.')
    page.screenshot(path=str(out/'02-msg.png'))
    mail.get_by_role('button',name='Schließen',exact=True).click()
    page.evaluate('window.mailFail=true')
    msgrow.get_by_role('button',name='E-Mail lesen',exact=True).click()
    expect(mail.get_by_role('alert')).to_contain_text('Das Original bleibt erhalten')
    page.evaluate('window.mailFail=false;window.mailMalicious=true')
    mail.get_by_role('button',name='Erneut lesen',exact=True).click()
    expect(mail.locator('pre')).to_contain_text('<img src=')
    assert not page.evaluate('window.mailCode')
    assert not any('mail-tracker.invalid' in r for r in requests)
    expect(mail.locator('img,script')).to_have_count(0)
    mail.get_by_role('button',name='Schließen',exact=True).click()
    assert not page.evaluate('window.calls.some(c=>["dokument_importieren","formular_im_projekt_speichern"].includes(c.cmd))')
    assert not errors,errors
    browser.close()
print('E-Mail-Archiv und Kategorien: Originalversionen, EML/MSG, Metadatenkategorie, Anzahlen, Schnellfilter, Fehler und veraltete Antworten erfolgreich geprüft')
