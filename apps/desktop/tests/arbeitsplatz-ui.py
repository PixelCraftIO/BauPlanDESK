"""Arbeitscheck, unverwechselbare Versionen, Aktualisierungsfehler und Skalierung."""
from pathlib import Path
import json, os
from playwright.sync_api import sync_playwright, expect
from navigation_hilfe import bereich
root=Path(__file__).resolve().parent
fixture=json.loads((root/'fixtures/projekt.json').read_text())
out=Path(os.environ.get('BAUPROJEKT_ARBEITSPLATZ_ARTEFAKTE','/tmp/bauplan-arbeitsplatz-ui'));out.mkdir(parents=True,exist_ok=True)
custom=r'''
const base=window.__TAURI_INTERNALS__.invoke;
window.failOverview=false;
const doc={document_id:'11111111-1111-4111-8111-111111111111',project_id:window.fixture.id,file_name:'Lageplan.pdf',version:1,created_at:window.fixture.created_at,created_by:'Test',size_bytes:12,mime_type:'application/pdf'};
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 cmd=({fachobjektseite:'fachobjekte',fachverlaufsseite:'fachverlauf',druckartefakt_erstellen:'druckausgabe_erstellen'})[cmd]??cmd;
 if(cmd==='projektuebersicht'){
  if(window.failOverview)throw Error('Test: Übersicht nicht erreichbar');
  const d=await base(cmd,args);
  if(args.projektId!==window.fixture.id)return d;
  d.zahlen.nachweise_pruefen=1;
  d.punkte=[{id:'grund',thema:'grundlagen',dringlichkeit:'handeln',titel:'BauNVO prüfen',hinweis:'Die maßgebende Fassung festlegen.',faellig:null,verantwortlich:'user',ziel:{bereich:'rechtsgrundlagen',objekt:null,register:null,eintrag:null}},{id:'nachweis',thema:'nachweise',dringlichkeit:'pruefen',titel:'GRZ neu berechnen',hinweis:'Grundlagen geändert.',faellig:null,verantwortlich:null,ziel:{bereich:'grz',objekt:null,register:null,eintrag:null}}];
  d.empfaenger=[{empfaenger:'Bauamt',ziel:{bereich:'einreichungen',objekt:null,register:null,eintrag:null},dokumente:[{dokument_id:doc.document_id,dateiname:doc.file_name,eingereichte_version:1,aktuelle_version:2,eingereicht_am:'2026-10-20',status:'Inhalt geändert'}]}];
  return d;
 }
 if(cmd==='fachobjekte' && args.art==='unterlagen')return [{projekt_id:args.projektId,modul:'unterlagen',objekt:'22222222-2222-4222-8222-222222222222',revision:1,schema_version:1,daten:{vorlage:null,bezeichnung:'Lageplan zur Abgabe',erforderlich:true,dokument:{dokument_id:doc.document_id,version:1},status:'freigegeben',bemerkung:''}}];
 if(cmd==='dokument_historie')return [doc,{...doc,version:2}];
 if(cmd==='dokumente_auflisten')return [doc,{...doc,version:2}];
 if(cmd==='formularbibliothek')return {root:'/tmp/Formulare',vorlagen:[]};
 if(cmd==='projektordner_status')return '/tmp/Projekte';
 return base(cmd,args);
};
'''
errors=[]
with sync_playwright() as pw:
 browser=pw.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=browser.new_page(viewport={'width':1440,'height':1000},locale='de-DE')
 page.on('pageerror',lambda e:errors.append(str(e)))
 page.add_init_script('window.fixture='+json.dumps(fixture)+';'+(root/'tauri-mock.js').read_text()+custom)
 page.goto(os.environ.get('BAUPROJEKT_UI_URL','http://127.0.0.1:1465'))
 page.locator('.projekttabelle').get_by_role('button',name=fixture['name'],exact=True).click()
 def active():return page.locator('.projekt-sitzung:visible')
 def overview():return active().get_by_role('region',name='Zentrale Projektübersicht',exact=True)
 check=overview().get_by_role('region',name='Arbeitscheck vor der Abgabe')
 expect(check).to_be_visible()
 check.locator('summary').filter(has_text='Arbeitscheck vor der Abgabe').click()
 check.locator('summary').filter(has_text='Grundlagen').click()
 check.get_by_role('button',name='Prüfpunkt öffnen: BauNVO prüfen',exact=True).click()
 plan=active().get_by_label('Planbezeichnung',exact=True);plan.fill('Entwurf bleibt erhalten')
 bereich(page,'Projektübersicht')
 expect(overview()).to_contain_text('ungespeicherte Änderungen vor')
 overview().get_by_label('Zuständigkeit',exact=True).select_option('ich')
 expect(overview().get_by_role('region',name='Handlungsbedarf').get_by_role('listitem')).to_have_count(1)
 overview().get_by_role('button',name='Alle Filter zurücksetzen',exact=True).click()
 expect(overview().get_by_role('region',name='Handlungsbedarf').get_by_role('listitem')).to_have_count(2)
 overview().get_by_role('button',name='Lageplan.pdf',exact=True).click()
 dialog=page.get_by_role('dialog')
 expect(dialog).to_contain_text('Verknüpfter Dokumentstand: Version 1')
 expect(dialog.locator('li.aktiv')).to_contain_text('Version 1')
 expect(dialog).to_contain_text('Version 2 (aktuell)')
 dialog.get_by_role('button',name='Schließen',exact=True).click()
 bereich(page,'Unterlagenregister')
 active().get_by_role('button',name='Öffnen',exact=True).click()
 expect(active().get_by_label('Speicherstatus')).to_contain_text('Gespeichert')
 expect(active()).to_contain_text('Zugeordnet ist Version 1; im Projekt liegt Version 2 vor.')
 expect(active().get_by_role('button',name='Eintragsliste anzeigen (1)',exact=True)).to_be_visible()
 active().get_by_role('button',name='Zugeordnete Dokumentversion ansehen',exact=True).click()
 expect(dialog).to_contain_text('Verknüpfter Dokumentstand: Version 1')
 dialog.get_by_role('button',name='Schließen',exact=True).click()
 bereich(page,'Rechtsgrundlagen');expect(plan).to_have_value('Entwurf bleibt erhalten')
 bereich(page,'Einreichen / Nachreichen')
 expect(active().get_by_role('region',name='Arbeitscheck vor der Abgabe')).to_be_visible()
 page.evaluate('window.failOverview=true')
 active().get_by_role('button',name='Verfahren aktualisieren',exact=True).click()
 expect(active().get_by_role('region',name='Arbeitscheck vor der Abgabe')).to_have_count(0)
 expect(active()).to_contain_text('Arbeitscheck nicht verfügbar')
 page.evaluate('window.failOverview=false')
 active().get_by_role('button',name='Verfahren aktualisieren',exact=True).click()
 expect(active().get_by_role('region',name='Arbeitscheck vor der Abgabe')).to_be_visible()
 for theme in ['light','dark']:
  page.evaluate('(t)=>{document.documentElement.dataset.theme=t;document.documentElement.style.colorScheme=t}',theme)
  for name,slug in [('Projektübersicht','uebersicht'),('Einreichen / Nachreichen','einreichen'),('Rechtsgrundlagen','eingaben'),('Projektdateien','dateien')]:
   bereich(page,name)
   for scale in [1,1.25,1.5]:
    # CSS zoom scales text and controls and reduces effective layout width.
    page.evaluate('(s)=>document.documentElement.style.zoom=String(s)',scale)
    assert page.evaluate('document.documentElement.scrollWidth <= document.documentElement.clientWidth+1'),(name,theme,scale)
    if scale in [1,1.5]:page.screenshot(path=str(out/f'{slug}-{theme}-{scale}.png'))
   page.evaluate('document.documentElement.style.zoom="1"')
 page.set_viewport_size({'width':600,'height':900})
 bereich(page,'Projektübersicht')
 assert page.evaluate('document.documentElement.scrollWidth<=innerWidth')
 assert not errors,errors
 assert not page.evaluate("window.calls.some(c=>/fachobjekt_speichern|dokument_exportieren|version_wiederherstellen/.test(c.cmd))")
 browser.close()
print('Arbeitsplatz: Prüfpunktnavigation, Entwurfserhalt, Zuständigkeit, genaue Dokumentversion, Fehlerzustand, Hell/Dunkel und 100–150 % geprüft.')
