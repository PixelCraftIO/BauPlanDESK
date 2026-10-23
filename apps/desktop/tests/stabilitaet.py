"""Fehler- und Entwurfsregressionen gegen die echte Svelte-Editorimplementierung.
Benötigt Vite auf BAUPROJEKT_DEV_UI_URL (Standard :1467), Chromium und Playwright.
Keine produktiven Projekte; IPC und Speicherfehler werden kontrolliert simuliert.
"""
import json, os
from pathlib import Path
from playwright.sync_api import sync_playwright
root = Path(__file__).resolve().parent
with sync_playwright() as p:
    browser = p.chromium.launch(headless=True, executable_path=os.environ.get('BAUPROJEKT_CHROMIUM', '/usr/bin/chromium'))
    page = browser.new_page()
    errors = []
    page.on('pageerror', lambda e: errors.append(str(e)))
    page.add_init_script('window.fixture=' + (root/'fixtures/projekt.json').read_text() + ';' + (root/'tauri-mock.js').read_text())
    page.goto(os.environ.get('BAUPROJEKT_DEV_UI_URL', 'http://127.0.0.1:1467'))
    result = page.evaluate('''async () => {
      const {FachSitzung} = await import('/src/lib/FachSitzung.svelte.ts');
      const {entwurfspeicher} = await import('/src/lib/entwurfspeicher.ts');
      const tick = () => new Promise(resolve => queueMicrotask(resolve));
      const assert = (wert, text) => { if(!wert) throw Error(text); };
      const vorher = window.__TAURI_INTERNALS__.invoke;
      const pending = [];
      window.__TAURI_INTERNALS__.invoke = async (cmd,args) => cmd === 'fachobjektseite'
        ? new Promise((resolve,reject) => pending.push({resolve,reject})) : vorher(cmd,args);
      const s = new FachSitzung('audit','aufgaben','audit',()=>({titel:'Neu',zeilen:[]}),x=>x.titel);
      const a=s.laden(), b=s.laden();
      pending[1].resolve([{objekt:'x',revision:2,daten:{titel:'neu'}}]); await b;
      assert(s.laeuft, 'Beschäftigtstatus muss ältere laufende Anfrage einschließen');
      pending[0].resolve([{objekt:'x',revision:1,daten:{titel:'alt'}}]); await a;
      assert(s.liste[0].revision===2 && !s.laeuft, 'Ältere Ladeantwort darf aktuellen Stand nicht ersetzen');
      const c=s.laden(), d=s.laden();
      pending[3].resolve([{objekt:'x',revision:3,daten:{titel:'neu'}}]);await d;
      pending[2].reject(Error('überholt'));await c;
      assert(!s.fehler && s.liste[0].revision===3,'Überholter Fehler darf neue Liste nicht verwerfen');
      let writes=0, fail=false;
      window.__TAURI_INTERNALS__.invoke=async(cmd,args)=>{
        if(cmd==='fachobjekt_speichern') { if(fail)throw Error('Versionskonflikt');writes++;return {objekt:args.id,revision:1,daten:args.daten}; }
        if(cmd==='fachobjektseite')return [];
        return vorher(cmd,args);
      };
      s.neu();
      const remove=Storage.prototype.removeItem;
      Storage.prototype.removeItem=()=>{throw new DOMException('Speicher gesperrt','SecurityError')};
      const ok=await s.speichern();Storage.prototype.removeItem=remove;
      assert(ok && writes===1 && s.revision===1 && !s.hatAenderungen(),'Backend-Erfolg trotz lokaler Bereinigungsstörung verbindlich anzeigen');
      assert(s.meldung.includes('Gespeichert.'),'Speichererfolg muss sichtbar bleiben');
      const key='bauprojekt.entwurf.v1.audit.draft.aufgaben';
      localStorage.setItem(key,JSON.stringify({schema:1,id:'entwurf',revision:0,daten:{titel:'Altbestand'},original:null}));
      const draft=new FachSitzung('draft','aufgaben','audit',()=>({titel:'Neu',zeilen:[]}),x=>x.titel);
      await draft.laden();assert(draft.gerettet,'Alter localStorage-Entwurf wird übernommen');
      draft.wiederherstellen();assert(draft.eingabe.titel==='Altbestand','Migrierter Entwurf bleibt lesbar');
      draft.eingabe.titel='Geändert';draft.eingabe.zeilen=Array.from({length:1000},(_,i)=>({nr:i,text:'Raum '.repeat(40)}));
      await tick();await draft.entwurfFlush();
      const gesichert=await entwurfspeicher.lesen(key);
      assert(JSON.parse(gesichert.datenJson).zeilen.length===1000 && !localStorage.getItem(key),'Großer Entwurf asynchron und ohne doppelten localStorage-Stand');
      fail=true;assert(!await draft.speichern() && draft.hatAenderungen(),'Backendfehler erhält Eingabe');
      assert(JSON.parse((await entwurfspeicher.lesen(key)).datenJson).titel==='Geändert','Backendfehler erhält lokale Sicherung');
      fail=false;
      draft.entwurfSichern();assert(await draft.speichern(),'Erneutes Speichern erfolgreich');
      // Auch ein bereits geplanter Sicherungstimer darf den gespeicherten Entwurf nicht erneut anlegen.
      await new Promise(resolve=>setTimeout(resolve,400));
      assert(!await entwurfspeicher.lesen(key),'Timer darf bereinigten Entwurf nicht wiederherstellen');
      const put=IDBObjectStore.prototype.put;
      draft.eingabe.titel='Nicht verloren';await tick();
      IDBObjectStore.prototype.put=()=>{throw new DOMException('Quota','QuotaExceededError')};
      await draft.entwurfFlush();IDBObjectStore.prototype.put=put;
      assert(draft.hatAenderungen() && draft.eingabe.titel==='Nicht verloren' && draft.meldung.includes('nicht verfügbar'),'Speicherquotenfehler erhält bearbeitbare Daten und meldet fehlende Sicherung');
      await draft.entwurfFlush();await draft.verwerfen();
      assert(!draft.eingabe && !await entwurfspeicher.lesen(key),'Verwerfen entfernt bestätigten Entwurf');
      // Abgeschlossene Antworten werden nicht gecacht; parallele Leser teilen nur den laufenden Request.
      const {leseAbfrage,leseAbfragenVerwerfen}=await import('/src/lib/leseAbfrage.ts');
      let anzahl=0, fertig;const laden=()=>{anzahl++;return new Promise(r=>fertig=r)};
      const r1=leseAbfrage('test','projekt',laden),r2=leseAbfrage('test','projekt',laden);
      assert(anzahl===1,'Parallele identische Leseabfragen bündeln');fertig(1);await Promise.all([r1,r2]);
      const r3=leseAbfrage('test','projekt',laden);assert(anzahl===2,'Abgeschlossene Antwort nicht wiederverwenden');
      leseAbfragenVerwerfen();const fertigAlt=fertig;const r4=leseAbfrage('test','projekt',laden);assert(anzahl===3,'Änderung trennt Anfragegruppen');fertigAlt(2);fertig(3);await Promise.all([r3,r4]);
      s.beenden();draft.beenden();window.__TAURI_INTERNALS__.invoke=vorher;
      return {ladeantworten:true,speichererfolg:true,entwurfMigration:true,grosserEntwurf:true,versionsfehler:true,quotenfehler:true,timerBereinigung:true,leseabfragen:true};
    }''')
    assert not errors, errors
    print(json.dumps(result,ensure_ascii=False,indent=2))
    browser.close()
