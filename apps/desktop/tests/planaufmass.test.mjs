import {test} from "node:test";
import assert from "node:assert/strict";
import {readFile} from "node:fs/promises";
import ts from "typescript";
const source=await readFile(new URL("../src/lib/planaufmass/pdfGeometry.ts",import.meta.url),"utf8");
const {outputText}=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}});
const {extractGeometry,snap,meterProPunkt,vorschauMass,verschieben,rahmen}=await import("data:text/javascript;base64,"+Buffer.from(outputText).toString("base64"));
const ops=Object.fromEntries(["save","restore","transform","paintFormXObjectBegin","paintFormXObjectEnd","constructPath","moveTo","lineTo","rectangle","closePath","curveTo","curveTo2","curveTo3","stroke","fill","eoFill","fillStroke","eoFillStroke","closeStroke","closeFillStroke","closeEOFillStroke","endPath"].map((s,i)=>[s,i+1]));
const list=(...commands)=>({fnArray:commands.map(c=>ops[c[0]]),argsArray:commands.map(c=>c[1])});
const identity=[1,0,0,1,0,0];
test("PDF-Transformation, verschachtelte Form und Seitenrotation liegen im gleichen Koordinatensystem",()=>{
  const r=extractGeometry(list(["save",[]],["transform",[2,0,0,2,10,20]],["paintFormXObjectBegin",[[1,0,0,1,5,0],null]],
    ["constructPath",[[ops.moveTo,ops.lineTo],[0,0,10,0]]],["stroke",[]],["paintFormXObjectEnd",[]],["restore",[]],
    ["constructPath",[[ops.moveTo,ops.lineTo],[0,0,10,0]]],["stroke",[]]),ops,[0,1,1,0,0,0]);
  assert.deepEqual(r.segments,[[[20,20],[20,40]],[[0,0],[0,10]]]);
});
test("Kurvenendpunkte ohne falsche gerade Fangkante und ungezeichnete Clipping-Pfade",()=>{
  const r=extractGeometry(list(["constructPath",[[ops.moveTo,ops.curveTo],[0,0,0,10,10,10,10,0]]],["stroke",[]],
    ["constructPath",[[ops.rectangle],[100,100,20,20]]],["endPath",[]]),ops,identity);
  assert.deepEqual(r.points,[[0,0],[10,0]]);
  assert.equal(r.segments.length,0);
});
test("Endpunkt, Mittelpunkt und Kante fangen innerhalb der bildschirmabhängigen Toleranz",()=>{
  const g={points:[[0,0],[100,0]],segments:[[[0,0],[100,0]]],limited:false};
  assert.deepEqual(snap([1,1],g,4),{point:[0,0],kind:"Endpunkt"});
  assert.deepEqual(snap([51,1],g,4),{point:[50,0],kind:"Mittelpunkt"});
  assert.deepEqual(snap([20,1],g,4),{point:[20,0],kind:"Kante"});
  assert.equal(snap([20,5],g,4),null);
});
test("Rechteck wird geschlossen; harte Umfangsgrenze bleibt sichtbar",()=>{
  const commands=list(["constructPath",[[ops.rectangle],[0,0,10,20]]],["stroke",[]]);
  assert.equal(extractGeometry(commands,ops,identity).segments.length,4);
  assert.equal(extractGeometry(commands,ops,identity,2).limited,true);
});

test("Vorschaumaße folgen der Kalibrierung und bleiben ohne Maßstab leer",()=>{
  const f=meterProPunkt({art:"strecke",von:[0,0],bis:[100,0],meter:10});
  assert.equal(f,0.1);
  assert.deepEqual(vorschauMass({art:"rechteck",von:[0,0],bis:[100,50]},f),{laenge_m:30,flaeche_m2:50});
  assert.deepEqual(vorschauMass({art:"polygon",punkte:[[0,0],[100,0],[100,50],[0,50]]},f),{laenge_m:30,flaeche_m2:50});
  assert.deepEqual(vorschauMass({art:"strecke",von:[0,0],bis:[30,40]},f),{laenge_m:5,flaeche_m2:null});
  assert.deepEqual(vorschauMass({art:"rechteck",von:[0,0],bis:[100,50]},null),{laenge_m:null,flaeche_m2:null});
  assert.ok(Math.abs(meterProPunkt({art:"massstab",nenner:100})-100*0.0254/72)<1e-12);
  assert.equal(meterProPunkt(null),null);
});
test("Verschieben erhält die Form; Umgrenzung umschließt alle Stützpunkte",()=>{
  assert.deepEqual(verschieben({art:"polygon",punkte:[[0,0],[10,0]]},5,-5),{art:"polygon",punkte:[[5,-5],[15,-5]]});
  assert.deepEqual(verschieben({art:"kreis",mitte:[0,0],rand:[1,0]},1,1),{art:"kreis",mitte:[1,1],rand:[2,1]});
  assert.deepEqual(rahmen([[10,20],[30,5],[15,40]]),{x:10,y:5,w:20,h:35});
});
