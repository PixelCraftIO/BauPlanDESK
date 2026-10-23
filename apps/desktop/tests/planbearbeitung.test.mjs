import {test} from "node:test";
import assert from "node:assert/strict";
import {readFile} from "node:fs/promises";
import ts from "typescript";
const base=new URL("../src/lib/planaufmass/",import.meta.url);
const urls={};
async function laden(name){
  let s=await readFile(new URL(name,base),"utf8");
  for(const [name,url] of Object.entries(urls))s=s.replaceAll(`from "./${name}"`,`from "${url}"`);
  const {outputText}=ts.transpileModule(s,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}});
  const url="data:text/javascript;base64,"+Buffer.from(outputText).toString("base64");urls[name.replace(/\.ts$/,"")]=url;
  return import(url);
}
const geometry=await laden("pdfGeometry.ts"),edit=await laden("bearbeitung.ts"),{Bearbeitungsverlauf}=await laden("verlauf.ts"),{Fangindex,standardFang,streckenachsen}=await laden("fang.ts"),{aufmassUebertragen}=await laden("uebertragung.ts");
test("Vollständiges Rückgängig stellt Ausschnitt, Ebene, Maßstab und Markierungsreferenz gemeinsam wieder her",()=>{
  const a={kalibrierung:{art:"massstab",nenner:100},ausschnitte:[{id:"schnitt",ebene_id:"ebene"}],ebenen:[{id:"ebene",sichtbar:true}],markierungen:[{id:"m",ausschnitt_id:"schnitt"}]};
  const h=new Bearbeitungsverlauf();h.reset(a);
  const b=structuredClone(a);b.ausschnitte=[];delete b.markierungen[0].ausschnitt_id;b.kalibrierung.nenner=200;
  h.beobachten(b);assert.deepEqual(h.rueckwaerts(b),a);assert.deepEqual(h.vorwaerts(a),b);
  const c=structuredClone(a);c.ebenen[0].sichtbar=false;h.rueckwaerts(b);h.beobachten(c);assert.equal(h.vor,0);
});
test("Eingaben eines Feldes werden gruppiert; andere Felder und spätere Aktionen bleiben getrennt",()=>{
  const h=new Bearbeitungsverlauf(3);h.reset({name:"A"});h.beobachten({name:"AB"},"name",1);h.beobachten({name:"ABC"},"name",2);assert.equal(h.zurueck,1);
  assert.deepEqual(h.rueckwaerts({name:"ABC"}),{name:"A"});
  for(let n=0;n<10;n++)h.beobachten({name:String(n)});assert.equal(h.zurueck,3);
});
test("Punkte einfügen/entfernen, Kantenmaß und Drehen erhalten echte Geometrien",()=>{
  const g={art:"polygon",punkte:[[0,0],[10,0],[10,10],[0,10]]};
  const p=edit.punktEinfuegen(g,0);assert.deepEqual(p.punkte[1],[5,0]);assert.deepEqual(edit.punktEntfernen(p,1),g);
  assert.deepEqual(edit.kantenmass(g,0,20,90).punkte[1].map(x=>Math.round(x)),[0,-20]);
  const r=edit.drehen({art:"rechteck",von:[0,0],bis:[10,20]},90);
  assert.equal(r.art,"polygon");assert.ok(Math.abs(geometry.vorschauMass(r,1).flaeche_m2-200)<1e-8);
});
test("Zusammengesetzte Flächen erhalten Innenringe beim Verschieben und Drehen",()=>{
  const g={art:"flaeche",polygone:[{aussen:[[0,0],[10,0],[10,10],[0,10]],innen:[[[2,2],[4,2],[4,4],[2,4]]]}]};
  const v=geometry.vorschauMass(g,1);assert.equal(v.flaeche_m2,96);assert.equal(v.laenge_m,48);
  assert.equal(geometry.eckpunkte(g).length,8);
  assert.equal(geometry.vorschauMass(edit.drehen(geometry.verschieben(g,50,30),30),1).flaeche_m2.toFixed(6),"96.000000");
});
test("Räumlicher Fang findet Enden, Mittelpunkte, Kanten und optionale Schnittpunkte",()=>{
  const g={points:[[0,0],[100,0],[50,-50],[50,50]],segments:[[[0,0],[100,0]],[[50,-50],[50,50]]],limited:false},i=new Fangindex(g);
  assert.equal(i.suchen([1,1],4).kind,"Endpunkt");assert.equal(i.suchen([50,1],4).kind,"Mittelpunkt");
  assert.equal(i.suchen([20,1],4).kind,"Kante");assert.equal(i.suchen([20,10],4),null);
  assert.equal(i.suchen([50,1],4,{...standardFang,endpunkte:false,mittelpunkte:false,schnittpunkte:true}).kind,"Schnittpunkt");
  assert.equal(i.suchen([20,1],4,{...standardFang,endpunkte:false,mittelpunkte:false,kanten:false,schnittpunkte:false}),null);
});
test("Vorschauachsen folgen der schrägen Bezugslinie statt den Seitenachsen",()=>{
  const a=30*Math.PI/180,x=[Math.cos(a),Math.sin(a)],b={von:[0,0],bis:[x[0]*100,x[1]*100]};
  const v=streckenachsen([0,0],[x[0]*30-x[1]*40,x[1]*30+x[0]*40],b);
  assert.ok(Math.abs(v.horizontal-30)<1e-8);assert.ok(Math.abs(v.hoehe-40)<1e-8);
});
test("Übertragung ist eine inverse Ausrichtung mit neuen IDs; alte Bezüge bleiben unverändert",()=>{
  const p={bezeichnung:"EG",dokument:{dokument_id:"alt",version:1},seite:1,kalibrierung:{art:"massstab",nenner:100},hoehenbezug:null,ebenen:[{id:"e",bezeichnung:"EG",sichtbar:true,gesperrt:false}],ausschnitte:[{id:"a",ebene_id:"e",bezeichnung:"EG",art:"grundriss",von:[0,0],bis:[100,100]}],markierungen:[{id:"m",ebene_id:"e",ausschnitt_id:"a",bezeichnung:"Raum",bedeutung:"raum",geometrie:{art:"rechteck",von:[10,20],bis:[30,60]}}]};
  const alt=JSON.stringify(p);let n=0;
  const neu=aufmassUebertragen(p,{dokument_id:"neu",version:2},2,{dx:10,dy:20,grad:0,faktor:2},()=>`id${++n}`);
  assert.equal(JSON.stringify(p),alt);assert.equal(neu.kalibrierung,null);assert.equal(neu.dokument.version,2);
  assert.equal(neu.markierungen[0].ebene_id,neu.ebenen[0].id);assert.equal(neu.markierungen[0].ausschnitt_id,neu.ausschnitte[0].id);
  assert.deepEqual(neu.markierungen[0].geometrie.punkte[0],[0,0]);assert.deepEqual(neu.markierungen[0].geometrie.punkte[2],[10,20]);
  assert.throws(()=>aufmassUebertragen(p,p.dokument,1,{dx:0,dy:0,grad:0,faktor:0}),/positive/);
});
