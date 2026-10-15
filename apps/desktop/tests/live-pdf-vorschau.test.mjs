import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import ts from 'typescript';
const source=await readFile(new URL('../src/lib/LivePdfVorschau.ts',import.meta.url),'utf8');
const {outputText}=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}});
const {LivePdfVorschau}=await import('data:text/javascript;base64,'+Buffer.from(outputText).toString('base64'));
const flush=()=>new Promise(setImmediate);
test('schnelle Eingaben bündeln; ohne Änderungen Original verwenden',async t=>{
 t.mock.timers.enable({apis:['setTimeout','Date']});
 const aufrufe=[],bilder=[];
 const v=new LivePdfVorschau(async(p,w)=>{aufrufe.push([p,w]);return 'gefüllt';},(p,s)=>bilder.push([p,s]),assert.fail);
 v.planen('Original',{Name:'D'},'1');v.planen('Original',{Name:'Daniel'},'2');
 t.mock.timers.tick(179);await flush();assert.equal(aufrufe.length,0);
 t.mock.timers.tick(1);await flush();assert.deepEqual(aufrufe,[['Original',{Name:'Daniel'}]]);assert.deepEqual(bilder,[['gefüllt','2']]);
 v.planen('Original',{},'3');t.mock.timers.tick(180);await flush();assert.equal(aufrufe.length,1);assert.deepEqual(bilder.at(-1),['Original','3']);v.beenden();
});
test('eine laufende Anfrage; alte Antwort darf neuere Eingaben nicht ersetzen',async t=>{
 t.mock.timers.enable({apis:['setTimeout','Date']});
 const aufrufe=[],bilder=[],fehler=[];
 let fertig;
 const v=new LivePdfVorschau((p,w)=>{aufrufe.push(w);return new Promise(resolve=>fertig=resolve);},(p,s)=>bilder.push([p,s]),e=>fehler.push(e));
 v.planen('Original',{Name:'Alt'},'alt');t.mock.timers.tick(180);await flush();
 v.planen('Original',{Name:'Zwischenstand'},'zwischen');v.planen('Original',{Name:'Neu'},'neu');t.mock.timers.tick(180);await flush();assert.equal(aufrufe.length,1);
 fertig('altes Bild');await flush();assert.deepEqual(bilder,[]);t.mock.timers.tick(0);await flush();assert.deepEqual(aufrufe,[{Name:'Alt'},{Name:'Neu'}]);
 fertig('neues Bild');await flush();assert.deepEqual(bilder,[['neues Bild','neu']]);assert.deepEqual(fehler,[]);v.beenden();
});
test('Schließen verwirft geplante und laufende Antworten',async t=>{
 t.mock.timers.enable({apis:['setTimeout','Date']});
 let fertig;let aufrufe=0;const bilder=[];
 const v=new LivePdfVorschau(()=>{aufrufe++;return new Promise(resolve=>fertig=resolve);},p=>bilder.push(p),assert.fail);
 v.planen('Original',{Name:'Alt'},'alt');t.mock.timers.tick(180);await flush();v.planen('Original',{Name:'Neu'},'neu');v.beenden();fertig('spät');await flush();t.mock.timers.tick(500);await flush();assert.equal(aufrufe,1);assert.deepEqual(bilder,[]);
});
test('veraltete Fehler ignorieren; nach aktuellem Fehler weiterarbeiten',async t=>{
 t.mock.timers.enable({apis:['setTimeout','Date']});
 let ablehnen;const fehler=[],bilder=[];
 const v=new LivePdfVorschau(()=>new Promise((resolve,reject)=>ablehnen=reject),p=>bilder.push(p),e=>fehler.push(e.message));
 v.planen('Original',{Name:'Alt'},'alt');t.mock.timers.tick(180);await flush();v.planen('Original',{Name:'Neu'},'neu');ablehnen(new Error('alt'));await flush();t.mock.timers.tick(180);await flush();ablehnen(new Error('aktuell'));await flush();assert.deepEqual(fehler,['aktuell']);
 v.planen('Original',{},'leer');t.mock.timers.tick(180);await flush();assert.deepEqual(bilder,['Original']);v.beenden();
});
