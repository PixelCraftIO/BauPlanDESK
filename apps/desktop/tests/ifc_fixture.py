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
const istIfcDok=d=>d&&d.dokument_id===ifcDok.document_id&&d.version===ifcDok.version;
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 const c=({fachobjektseite:'fachobjekte'})[cmd]??cmd;
 if(c==="dokumente_auflisten"){const r=await ifcBasis(cmd,args);return [...(Array.isArray(r)?r:[]),ifcDok];}
 if(c.startsWith("ifc_")){
   window.ifcAufrufe.push({cmd:c,args:kopie(args)});
   if(!istIfcDok(args.dokument))throw "Dokument ist kein IFC-Modell des Projekts";
   if(c==="ifc_auszug")return structuredClone(IFC.auszug);
   if(c==="ifc_geschosse")return structuredClone(IFC.geschosse);
   if(c==="ifc_grundriss"){const g=structuredClone(IFC.grundrisse[args.geschoss]);if(!g)throw "Geschoss ist nicht im Modell";if(args.schnitthoeheM!=null)g.schnitthoehe_m=args.schnitthoeheM;return g;}
   if(c==="ifc_grundriss_plan")return structuredClone(IFC.plaene[args.geschoss]);
   if(c==="ifc_datei")return IFC.datei_base64;
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
