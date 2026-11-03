import {test} from "node:test";
import assert from "node:assert/strict";
import {readFile} from "node:fs/promises";
import ts from "typescript";
const source=await readFile(new URL("../src/lib/planaufmass/beschriftungen.ts",import.meta.url),"utf8");
const {outputText}=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}});
const {beschriftungenPlatzieren,textbreite}=await import("data:text/javascript;base64,"+Buffer.from(outputText).toString("base64"));

const quadrat=(x,y,s)=>[[x,y],[x+s,y],[x+s,y+s],[x,y+s]];
const box=(l,text,schrift)=>{const w=textbreite(text,schrift);return {x:l.anker==="middle"?l.x-w/2:l.x,y:l.y-schrift,w,h:schrift*1.2}};
const ueberlappt=(a,b)=>a.x<b.x+b.w&&b.x<a.x+a.w&&a.y<b.y+b.h&&b.y<a.y+a.h;

test("Freistehende Markierung behält die bisherige Lage über der ersten Ecke",()=>{
  const r=beschriftungenPlatzieren([{id:"a",text:"Raum 1",punkte:quadrat(100,100,50)}],12,5);
  assert.deepEqual(r.get("a"),{x:105,y:95,anker:"start"});
});

test("Markierungen mit gleicher Ecke werden ohne Überlappung beschriftet",()=>{
  const wuensche=[
    {id:"gross",text:"Grundfläche KG · 90 m²",punkte:quadrat(100,100,300)},
    {id:"klein",text:"Hobbyraum · 23 m²",punkte:quadrat(100,100,120)},
    {id:"mittel",text:"Technik · 8 m²",punkte:quadrat(100,100,200)},
  ];
  const r=beschriftungenPlatzieren(wuensche,12,5);
  const boxen=wuensche.map(w=>r.get(w.id)&&box(r.get(w.id),w.text,12)).filter(Boolean);
  assert.equal(boxen.length,3,"alle drei finden Platz");
  for(let i=0;i<boxen.length;i++)for(let j=i+1;j<boxen.length;j++)assert.ok(!ueberlappt(boxen[i],boxen[j]),`${i} und ${j} überlappen`);
  // Die größte Figur bekommt die Standardlage.
  assert.deepEqual(r.get("gross"),{x:105,y:95,anker:"start"});
});

test("Ohne freien Platz wird ausgeblendet, ausgewählte Markierungen bleiben sichtbar",()=>{
  const wuensche=Array.from({length:12},(_,i)=>({id:`m${i}`,text:"Fläche mit langer Bezeichnung",punkte:quadrat(0,0,4)}));
  const r=beschriftungenPlatzieren(wuensche,12,5);
  assert.ok([...r.values()].some(l=>l===null),"einige ausgeblendet");
  const mitAuswahl=beschriftungenPlatzieren(wuensche.map((w,i)=>i===11?{...w,erzwingen:true}:w),12,5);
  assert.deepEqual(mitAuswahl.get("m11"),{x:5,y:-5,anker:"start"});
});

test("Leere Texte und Figuren ohne Punkte erzeugen keine Beschriftung",()=>{
  const r=beschriftungenPlatzieren([{id:"a",text:"",punkte:quadrat(0,0,10)},{id:"b",text:"x",punkte:[]}],12,5);
  assert.equal(r.get("a"),null);assert.equal(r.get("b"),null);
});

test("Feste Flächen (Ausschnittnamen) werden nicht überdeckt",()=>{
  const fest=[{x:100,y:80,w:200,h:20}];
  const r=beschriftungenPlatzieren([{id:"a",text:"Außenkante KG · 99 m²",punkte:quadrat(100,100,300)}],12,5,fest);
  const l=r.get("a");
  assert.ok(l);
  assert.ok(!ueberlappt(box(l,"Außenkante KG · 99 m²",12),fest[0]));
});
