"""Reproduzierbare synthetische Messung; Entwicklungsbuild, keine SLA-Prüfung.
Vite unter BAUPROJEKT_DEV_UI_URL (:1467). Ergebnisse als JSON auf stdout.
"""
from pathlib import Path
import json,os
from playwright.sync_api import sync_playwright
root=Path(__file__).resolve().parent
with sync_playwright() as p:
 b=p.chromium.launch(headless=True,executable_path=os.environ.get('BAUPROJEKT_CHROMIUM','/usr/bin/chromium'))
 page=b.new_page()
 page.add_init_script('window.fixture='+(root/'fixtures/projekt.json').read_text()+';'+(root/'tauri-mock.js').read_text())
 page.goto(os.environ.get('BAUPROJEKT_DEV_UI_URL','http://127.0.0.1:1467'))
 result=page.evaluate('''async()=>{
 const {FachSitzung}=await import('/src/lib/FachSitzung.svelte.ts');
 const {entwurfspeicher}=await import('/src/lib/entwurfspeicher.ts');
 const rows=[];
 for(const anzahl of [100,1000,10000]){
   const s=new FachSitzung('performance','aufgaben','test',()=>({}),()=> '');
   s.bereit=true;s.id='benchmark';
   s.eingabe={rows:Array.from({length:anzahl},(_,i)=>({id:String(i),name:'Raum '+i,area:12.5,note:'x'.repeat(200)}))};
   s.original=JSON.parse(JSON.stringify(s.eingabe));s.eingabe.rows[0].area=15;
   const bytes=JSON.stringify(s.eingabe).length;
   let start=performance.now();s.hatAenderungen();const cold=performance.now()-start;
   start=performance.now();for(let i=0;i<20;i++)s.hatAenderungen();const warm=(performance.now()-start)/20;
   start=performance.now();for(let i=0;i<10;i++)await s.entwurfFlush();const flush=(performance.now()-start)/10;
   rows.push({rows:anzahl,inputBytes:bytes,firstDirtyCheckMs:cold,repeatedDirtyCheckMs:warm,asyncDraftFlushMs:flush});
   await entwurfspeicher.loeschen('bauprojekt.entwurf.v1.test.performance.aufgaben');
 }
 return rows;
}''')
 print(json.dumps({'context':'Vite development build; Chromium; synthetic arrays; repeated checks use derived signatures; flush includes cached JSON snapshots and IndexedDB transaction','results':result},indent=2))
 b.close()
