"""Testtransport für IFC-Modelle: Fixture aus `cargo run -p core-lib --features testdaten --example ifc_fixture`."""
from pathlib import Path
import json

root = Path(__file__).resolve().parent

def ifc_daten():
    return json.loads((root / 'fixtures/ifc-efh.json').read_text(encoding='utf-8'))

ifc_mock = r"""
const ifcBasis=window.__TAURI_INTERNALS__.invoke;
const kopie=v=>v===undefined?undefined:JSON.parse(JSON.stringify(v));
const IFC=window.ifcFixture;
window.ifcAkten=window.ifcAkten??[];window.ifcAufrufe=[];
const ifcDok={document_id:"c416da8b-cbfb-4ae6-931b-8765fb6454c1",project_id:window.fixture.id,version:1,file_name:IFC.datei_name,sha256:"0".repeat(64),size_bytes:40000,created_at:window.fixture.created_at,created_by:"Test",storage_key:"efh.ifc"};
window.ifcDok=ifcDok;
const idsDok={...ifcDok,document_id:"d416da8b-cbfb-4ae6-931b-8765fb6454d1",file_name:"buero.ids",storage_key:"buero.ids"};
const istIfcDok=d=>d&&d.dokument_id===ifcDok.document_id&&(d.version===1||d.version===2);
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 const c=({fachobjektseite:'fachobjekte'})[cmd]??cmd;
 if(c==="dokumente_auflisten"){const r=await ifcBasis(cmd,args);return [...(Array.isArray(r)?r:[]),ifcDok,{...ifcDok,version:2},idsDok];}
 if(["ifc_auszug","ifc_geschosse","ifc_grundriss","ifc_grundriss_plan","ifc_datei"].includes(c)){
   window.ifcAufrufe.push({cmd:c,args:kopie(args)});
   if(!istIfcDok(args.dokument))throw "Dokument ist kein IFC-Modell des Projekts";
   if(c==="ifc_auszug")return structuredClone(IFC.auszug);
   if(c==="ifc_geschosse")return structuredClone(IFC.geschosse);
   if(c==="ifc_grundriss"){const g=structuredClone(IFC.grundrisse[args.geschoss]);if(!g)throw "Geschoss ist nicht im Modell";if(args.schnitthoeheM!=null)g.schnitthoehe_m=args.schnitthoeheM;return g;}
   if(c==="ifc_grundriss_plan")return structuredClone(IFC.plaene[args.geschoss]);
   if(c==="ifc_datei")return IFC.datei_base64;
 }
 if(c==="ifc_akte_auszug"||c==="ifc_akte_grundriss"||c==="ifc_raumvorschlaege"){
   window.ifcAufrufe.push({cmd:c,args:kopie(args)});
   if(!window.ifcAkten.some(a=>a.objekt===args.akteId))throw "Modellakte fehlt im Projekt";
   if(c==="ifc_akte_auszug")return kopie(IFC.auszug);
   if(c==="ifc_raumvorschlaege")return kopie((IFC.raumvorschlaege[args.geschoss]??[]).map((v,i)=>i===0?{...v,vorhanden:null}:v));
   const g=kopie(IFC.grundrisse[args.geschoss]);if(!g)throw "Geschoss ist nicht im Modell";if(args.schnitthoeheM!=null)g.schnitthoehe_m=args.schnitthoeheM;return g;
 }
 if(c==="ifc_abgleich"){window.ifcAufrufe.push({cmd:c,args:kopie(args)});const akte=window.ifcAkten.find(a=>a.objekt===args.akteId);return {...kopie(IFC.abgleich_v2),nacharbeit:(akte.daten.nacharbeit??[]).map(()=>"wirksam")};}
 if(c==="ifc_reimport"){
   window.ifcAufrufe.push({cmd:c,args:kopie(args)});
   const w=args.wunsch,akte=window.ifcAkten.find(a=>a.objekt===w.akte_id);
   if(!akte||akte.revision!==w.revision)throw "Modellakte wurde inzwischen geändert. Bitte neu laden.";
   const neu={...kopie(akte),revision:akte.revision+1};neu.daten.dokument=kopie(w.dokument);
   window.ifcAkten=[...window.ifcAkten.filter(a=>a.objekt!==akte.objekt),neu];return kopie(neu);
 }
 if(c==="ifc_auswerten"){window.ifcAufrufe.push({cmd:c,args:kopie(args)});return kopie(IFC.auswertung);}
 if(c==="ifc_pruefen"){
   window.ifcAufrufe.push({cmd:c,args:kopie(args)});
   if(args.ids)return {regelwerk:"Büro",befunde:[{regel:"IDS:F-FENSTER",stufe:"fehler",titel:"Fenster",text:"Im Modell fehlt ein solches Element.",elemente:[],aktion:null,eigenschaft:null}],fehler:1,warnungen:0,hinweise:0,nicht_geprueft:[]};
   return kopie(IFC.pruefung);
 }
 if(c==="ifc_uebernehmen"){
   window.ifcAufrufe.push({cmd:c,args:kopie(args)});
   const w=args.wunsch,akte=window.ifcAkten.find(a=>a.objekt===w.akte_id);
   if(!akte||akte.revision!==w.revision)throw "Modellakte wurde inzwischen geändert. Bitte neu laden.";
   const gid=w.gebaeude_id??"9e0c1d4e-1b2c-4d5e-8f90-123456789abc";
   const geschosse=IFC.geschosse.map((g,i)=>({global_id:g.global_id,geschoss_id:`6b2d1e5f-2c3d-4e6f-9a01-23456789ab0${i}`,aufmass_id:`7c2d1e5f-2c3d-4e6f-9a01-23456789ab0${i}`,markierungen:[]}));
   const zeichnungen=geschosse.map((g,i)=>({projekt_id:args.projektId,modul:"planaufmass",objekt:g.aufmass_id,revision:1,schema_version:1,daten:{bezeichnung:`${akte.daten.bezeichnung} · ${IFC.geschosse[i].name}`,seite:1,kalibrierung:{art:"einheit",einheit:"meter"},hoehenbezug:null,ebenen:[],markierungen:[],hintergruende:[{id:crypto.randomUUID(),dokument:akte.daten.dokument,seite:1,ursprung:[0,0],lage:[2,2],drehung_grad:0,meter_pro_punkt:1,deckkraft:.6,sichtbar:true,geschoss:g.global_id}]}}));
   window.planSaves=[...window.planSaves.filter(f=>!zeichnungen.some(z=>z.objekt===f.objekt)),...zeichnungen];
   const neu={...akte,revision:akte.revision+1,daten:{...akte.daten,uebernahme:{gebaeude_id:gid,dokument:akte.daten.dokument,geschosse}}};
   window.ifcAkten=[...window.ifcAkten.filter(a=>a.objekt!==akte.objekt),neu];
   return kopie({akte:neu,gebaeude:{projekt_id:args.projektId,modul:"gebaeude",objekt:gid,revision:1,schema_version:1,daten:{bezeichnung:"Wohnhaus",grundstueck_ids:[],geschosse:[],quelle:null}},zeichnungen,raumbuecher:w.raumbuch?geschosse.map(g=>({projekt_id:args.projektId,modul:"raumbuch",objekt:crypto.randomUUID(),revision:1,schema_version:1,daten:{}})):[],hinweise:[]});
 }
 if(c==="fachobjekte"&&args.art==="ifcmodell")return structuredClone(window.ifcAkten);
 if(c==="fachobjekt_lesen"&&args.art==="ifcmodell")return structuredClone(window.ifcAkten.find(a=>a.objekt===args.id)??null);
 if(c==="fachobjekt_speichern"&&args.art==="ifcmodell"){
   const f={projekt_id:args.projektId,modul:"ifcmodell",objekt:args.id,revision:args.basis+1,schema_version:1,daten:kopie(args.daten)};
   window.ifcAkten=[...window.ifcAkten.filter(a=>a.objekt!==args.id),f];return structuredClone(f);
 }
 return ifcBasis(cmd,args);
};
"""

def init_script(fixture, extra=''):
    from plan_fixture import pdf_fixture, custom
    return ('window.fixture=' + json.dumps(fixture) + ';window.planPdf=' + json.dumps(pdf_fixture()) + ';window.ifcFixture=' + json.dumps(ifc_daten()) + ';'
            + (root / 'tauri-mock.js').read_text(encoding='utf-8') + custom + ifc_mock + extra)
