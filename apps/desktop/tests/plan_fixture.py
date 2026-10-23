import base64

def pdf_fixture():
    stream = b"0.6 w 0.3 0.4 0.5 RG 70 70 300 200 re S 220 70 m 220 270 l S 70 170 m 220 170 l S"
    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 400] /Resources << >> /Contents 4 0 R >>",
        b"<< /Length " + str(len(stream)).encode() + b" >>\nstream\n" + stream + b"\nendstream",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 400] /Resources << >> /Contents 4 0 R >>",
    ]
    data = b"%PDF-1.4\n"
    offsets = [0]
    for i, obj in enumerate(objects, 1):
        offsets.append(len(data))
        data += f"{i} 0 obj\n".encode() + obj + b"\nendobj\n"
    start = len(data)
    data += f"xref\n0 {len(offsets)}\n0000000000 65535 f \n".encode()
    data += b"".join(f"{o:010d} 00000 n \n".encode() for o in offsets[1:])
    data += f"trailer\n<< /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{start}\n%%EOF".encode()
    return base64.b64encode(data).decode()

custom = r"""
const original=window.__TAURI_INTERNALS__.invoke;
window.planSaved=null;window.planSaves=[];window.gebaeudeSaved=null;
const GID="7a1c0d4e-1b2c-4d5e-8f90-123456789abc",SID="5b2d1e5f-2c3d-4e6f-9a01-23456789abcd";
const haus={projekt_id:window.fixture.id,modul:"gebaeude",objekt:GID,revision:3,schema_version:1,daten:{bezeichnung:"Lagerhalle",grundstueck_ids:[],flurstueck_ids:[],geschosse:[{id:SID,bezeichnung:"EG",vollgeschoss:null,grundlage:""}],quelle:null}};
const doc={document_id:"b316da8b-cbfb-4ae6-931b-8765fb6454b1",project_id:window.fixture.id,version:1,file_name:"Grundrisse.pdf",sha256:"0".repeat(64),size_bytes:1000,created_at:window.fixture.created_at,created_by:"Test",storage_key:"test.pdf"};
window.__TAURI_INTERNALS__.invoke=async(cmd,args={})=>{
 cmd=({fachobjektseite:'fachobjekte',fachverlaufsseite:'fachverlauf',druckartefakt_erstellen:'druckausgabe_erstellen'})[cmd]??cmd;
 if(cmd==="dokumente_auflisten")return [doc,{...doc,document_id:"d316da8b-cbfb-4ae6-931b-8765fb6454b2",file_name:"Vergleich.pdf"}];
 if(cmd==="planaufmass_diagnose")return {markierungen:args.plan.markierungen.map(m=>({id:m.id,wert:m.bezeichnung==="Fehler"?null:{id:m.id,laenge_m:args.plan.kalibrierung?30:null,flaeche_m2:args.plan.kalibrierung?50:null,horizontale_m:null,hoehe_m:null,neigung_grad:null,offen:[]},fehler:m.bezeichnung==="Fehler"?"Kontur pruefen":null}))};
 if(cmd==="fachobjekt_lesen"&&args.art==="planaufmass")return window.planSaves.find(f=>f.objekt===args.id)??window.planSaved;
 if(cmd==="fachobjekt_lesen"&&args.art==="gebaeude")return structuredClone(haus);
 if(cmd==="planaufmass_exportieren"){window.planExport=args;return null;}
 if(cmd==="pdf_vorschau")return {dokument:{...doc,document_id:args.dokumentId??doc.document_id,version:args.version??1},pdf_base64:window.planPdf,felder:[],seiten:[1,2].map(n=>({nummer:n,breite_mm:211,hoehe_mm:141,rotation:0,platzierbar:true}))};
 if(cmd==='fachobjekte'&&args.art==="planaufmass")return structuredClone(window.planSaves.length?window.planSaves:window.planSaved?[window.planSaved]:[]);
 if(cmd==='fachobjekte'&&args.art==="gebaeude")return [structuredClone(haus)];
 if(cmd==="planaufmass_aufgaben"){
   const ms=args.plan.markierungen;
   const find=(b,flaeche)=>ms.find(m=>m.bedeutung===b&&m.gebaeude_id===GID&&(flaeche?["polygon","rechteck","kreis"].includes(m.geometrie.art):m.geometrie.art==="hoehenpunkt"))?.id??null;
   const basis={verbraucher:"gebaeude",objekt:GID,objekt_name:"Lagerhalle",gruppe:"Lagerhalle",gebaeude_id:GID,geschoss_id:null,grundstueck_id:null,wert_vorhanden:false,gebunden:haus.daten.planbindungen?.[0]?.markierung_id??null,hinweis:null};
   return [
     {...basis,ziel:"geometrie.grundflaeche_m2",titel:"Gebäudegrundfläche · horizontale Projektion",groesse:"flaeche",bedeutung:"gebaeudegrundflaeche",geometrie:"flaeche",markierung:basis.gebunden?null:find("gebaeudegrundflaeche",true)},
     {...basis,gebunden:null,ziel:"geometrie.traufe_m",titel:"Traufe",groesse:"hoehe",bedeutung:"traufe",geometrie:"hoehenpunkt",markierung:find("traufe",false),hinweis:"Schnitt/Ansicht mit Bezugslinie in m ü. NHN"},
   ];
 }
 if(cmd==="planaufmass_gebaeude")return {...args.plan,geometrie:{...args.plan.geometrie,grundflaeche_m2:50}};
 if(cmd==="fachobjekt_speichern"&&args.art==="gebaeude"){
   window.gebaeudeSaved={basis:args.basis,daten:structuredClone(args.daten)};haus.daten=structuredClone(args.daten);haus.revision=args.basis+1;
   return structuredClone(haus);
 }
 if(cmd==="planaufmass_pruefen")return args.plan.markierungen.map(m=>({id:m.id,laenge_m:args.plan.kalibrierung?30:null,flaeche_m2:args.plan.kalibrierung?50:null,horizontale_m:null,hoehe_m:null,neigung_grad:null,offen:[]}));
 if(cmd==="fachobjekt_speichern"&&args.art==="planaufmass"){
   window.planSaved={projekt_id:args.projektId,modul:args.art,objekt:args.id,revision:args.basis+1,schema_version:1,daten:structuredClone(args.daten)};
   window.planSaves=[...window.planSaves.filter(f=>f.objekt!==args.id),structuredClone(window.planSaved)];
   return structuredClone(window.planSaved);
 }
 return original(cmd,args);
};
"""
