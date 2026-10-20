import {test} from "node:test";
import assert from "node:assert/strict";
import {readFile} from "node:fs/promises";
import ts from "typescript";
const source=await readFile(new URL("../src/lib/planaufmass/pdfGeometry.ts",import.meta.url),"utf8");
const {outputText}=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}});
const {extractGeometry,snap}=await import("data:text/javascript;base64,"+Buffer.from(outputText).toString("base64"));
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
