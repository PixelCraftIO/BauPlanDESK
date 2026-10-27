<script lang="ts">
  import { bereichAktiv } from "../ansichtsAbfrage.svelte";
  import Symbolknopf from "../Symbolknopf.svelte";
  import WerkzeugIcon from "./WerkzeugIcon.svelte";
  const sichtbar = bereichAktiv();
  import { tick, untrack, onDestroy } from "svelte";
  import type { PDFDocumentProxy, PDFDocumentLoadingTask, PDFPageProxy, RenderTask } from "pdfjs-dist";
  import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
  import type { Plangeometrie, Planmarkierung, Planebene, Planausschnitt, Planmesswert, Plankalibrierung, Zeichenelement, Zeichenform } from "../generated/plattform";
  import { hgTransform, hgPunkt, blattmass, elementPunkte, elementSetzen, elementVerschieben, wandKontur, masskette, type Hintergrundansicht } from "./zeichnung";
  import { extractGeometry, snap, meterProPunkt, vorschauMass, verschieben, eckpunkte, rahmen as umgrenzung, type Matrix, type Point, type SnapGeometry } from "./pdfGeometry";
  import { Fangindex, standardFang, streckenachsen, type Fangoptionen } from "./fang";
  import { punktAendern } from "./bearbeitung";
  import { dxfBlatt, dxfPfade, dxfTexte, dxfGeometrie } from "./dxf";
  import type { Plan } from "../api";
  // Zeichenfläche: Anzeige, Fang, Zoom, Drehung und Eingabe. Fachwerte (Längen,
  // Flächen, Höhen) stammen vom Core; hier werden nur Vorschauen während des
  // Zeichnens gezeigt. Gespeicherte Koordinaten sind immer Seitenpunkte: bei
  // PDF-Blättern physische Seitenpunkte, bei DXF-Zeichnungen (`plan`)
  // Zeichnungseinheiten ab der linken oberen Ecke des Zeichnungsumfangs.
  let { pdfBase64 = "", seite, plan = null, planSichtbar = [], markierungen, ebenen, auswahl, werkzeug, darf,
    ausschnitte = [], aktiv = "", rahmenAuswahl = "", werte = [], kalibrierung = null, masse = true,
    auswahlIds = [], ansichtKey = "", fehlerIds = [], vergleichBase64 = "", vergleichDeckkraft = .5, vergleichAusrichtung = {dx:0,dy:0,grad:0,faktor:1},
    zeichnung = false, hintergruende = [], elemente = [], elementAuswahl = "", wanddicke = .24,
    onelement = () => {}, onelementauswahl = () => {}, onelementedit = () => {},
    onverschieben = () => {}, onpunkt = () => {},
    onfertig, onauswahl, onedit, onfehler, onrahmen = () => {}, onrahmenauswahl = () => {}, onoeffnen = () => {}, onseite = () => {} }:
    {pdfBase64?:string;seite:number;plan?:Plan|null;planSichtbar?:boolean[];markierungen:Planmarkierung[];ebenen:Planebene[];
    auswahl:string;werkzeug:string;darf:boolean;
    ausschnitte?:Planausschnitt[];aktiv?:string;rahmenAuswahl?:string;werte?:Planmesswert[];kalibrierung?:Plankalibrierung|null;masse?:boolean;
    onfertig:(g:Plangeometrie)=>void;onauswahl:(id:string,mehrfach?:boolean)=>void;
    auswahlIds?:string[];ansichtKey?:string;fehlerIds?:string[];vergleichBase64?:string;vergleichDeckkraft?:number;
    vergleichAusrichtung?:{dx:number;dy:number;grad:number;faktor:number};
    /** Eigene Zeichnung: Blatt in Metern, Unterlagen als eingepasste Hintergründe. */
    zeichnung?:boolean;hintergruende?:Hintergrundansicht[];elemente?:Zeichenelement[];elementAuswahl?:string;wanddicke?:number;
    onelement?:(f:Zeichenform)=>void;onelementauswahl?:(id:string)=>void;onelementedit?:(id:string,e:Zeichenelement)=>void;
    onverschieben?:(ids:string[],dx:number,dy:number)=>void;onpunkt?:(index:number)=>void;
    onedit:(id:string,g:Plangeometrie)=>void;onfehler:(s:string)=>void;
    onrahmen?:(id:string,von:Point,bis:Point)=>void;onrahmenauswahl?:(id:string)=>void;onoeffnen?:(id:string)=>void;onseite?:(n:number)=>void}=$props();
  const PAD=8;
  let ansichtGeneration=0;
  let doc=$state<PDFDocumentProxy|null>(null), page:PDFPageProxy|null=null, svg=$state<SVGSVGElement>(), seitenbild=$state("");
  let scroll=$state<HTMLDivElement>(), breite=$state(600), hoehe=$state(800), zoom=$state(1), drehung=$state(0);
  let bereit=$state(false), fang=$state(true), geometry=$state<SnapGeometry>({points:[],segments:[],limited:false});
  let punkte=$state<Point[]>([]), cursor=$state<Point|null>(null), fangart=$state("");
  let drag=$state<{id:string;index:number;g:Plangeometrie}|null>(null);
  let schieben=$state<{id:string;start:Point;g:Plangeometrie;bewegt:boolean}|null>(null);
  let rahmenDrag=$state<{id:string;ecke:number;von:Point;bis:Point;start:Point;bewegt:boolean}|null>(null);
  let pan: {x:number;y:number;left:number;top:number}|null=null;
  let leertaste=$state(false);
  let edit=$state<Plangeometrie|null>(null);
  let zoomfenster=$state(false), zf=$state<Point|null>(null);
  let masseingabe=$state<string|null>(null), masseingabeFeld=$state<HTMLInputElement>();
  let seitenbilder=$state<{nummer:number;url:string}[]>([]), miniaturen=$state(false), seitenzahl=$state(1);
  let rasterScale=0, rasterTask:RenderTask|undefined, rasterTimer:ReturnType<typeof setTimeout>|undefined;
  const zeiger=new Map<number,{x:number;y:number}>();
  let pinch:{abstand:number;mitte:{x:number;y:number}}|null=null;
  let fangoptionen=$state<Fangoptionen>({...standardFang}), fangOffen=$state(false);
  let rahmenVorschau=$state<{id:string;von:Point;bis:Point}|null>(null), touchPunkt:Point|null=null;
  let vergleichBild=$state("");
  let vergleichGroesse=$state({breite:0,hoehe:0});
  // Eigene Zeichnung: gerasterte PDF-Hintergründe je Dokumentversion und Seite; Fang an deren Vektoren.
  let hgBilder=$state<Record<string,{url:string;breite:number;hoehe:number;geometry:SnapGeometry}>>({});
  const hgLaeuft=new Set<string>();
  const hgKey=(h:Hintergrundansicht)=>`${h.h.dokument.dokument_id}|${h.h.dokument.version}|${h.h.seite}`;
  let elementDrag=$state<{id:string;index:number;e:Zeichenelement;start:Point;bewegt:boolean}|null>(null);
  let elementEdit=$state<Zeichenelement|null>(null);
  const rasterId=`r${Math.random().toString(36).slice(2,8)}`;
  const fangindex=$derived(new Fangindex(geometry));
  const wertMap=$derived(new Map(werte.map(w=>[w.id,w])));
  const ebenenMap=$derived(new Map(ebenen.map(e=>[e.id,e])));
  const angezeigt=$derived(markierungen.filter(m=>ebenenMap.get(m.ebene_id)?.sichtbar));
  const gewaehlt=$derived(angezeigt.find(m=>m.id===auswahl));
  const angezeigteElemente=$derived(elemente.filter(e=>ebenenMap.get(e.ebene_id)?.sichtbar));
  const gewaehltesElement=$derived(angezeigteElemente.find(e=>e.id===elementAuswahl));
  const elementGesperrt=$derived(!darf||!!ebenen.find(e=>e.id===gewaehltesElement?.ebene_id)?.gesperrt);
  const elementWerkzeug=$derived(werkzeug==="wand"||werkzeug==="masskette"||werkzeug==="text");
  // Hintergründe mit Darstellung: DXF als Pfade, PDF als Seitenbild.
  const hgAnsicht=$derived(hintergruende.flatMap(hg=>{
    if(hg.plan){const b=dxfBlatt(hg.plan);if(!b)return [];const alle=hg.plan.ebenen.map(()=>true);return [{hg,breite:b.breite,hoehe:b.hoehe,url:"",pfade:dxfPfade(hg.plan,b,alle),geometry:dxfGeometrie(hg.plan,b,alle,20000)}]}
    const b=hgBilder[hgKey(hg)];return b?[{hg,breite:b.breite,hoehe:b.hoehe,url:b.url,pfade:[] as {farbe:string;d:string}[],geometry:b.geometry}]:[];
  }));
  const gesperrt=$derived(!darf || !!ebenen.find(e=>e.id===gewaehlt?.ebene_id)?.gesperrt);
  const ausschnitt=$derived(ausschnitte.find(a=>a.id===aktiv));
  // Markierungen anderer Ausschnitte bleiben sichtbar, aber nicht bedienbar.
  const fremd=(m:Planmarkierung)=>!!aktiv&&(m.ausschnitt_id??"")!==aktiv;
  const faktor=$derived(meterProPunkt(ausschnitt?.kalibrierung??kalibrierung));
  const aktiveKalibrierung=$derived(ausschnitt?.kalibrierung??kalibrierung);
  const gedreht=$derived(drehung%180!==0);
  const blatt=$derived(plan?dxfBlatt(plan):null);
  const pfade=$derived(plan&&blatt?dxfPfade(plan,blatt,planSichtbar):[]);
  const texte=$derived(plan&&blatt?dxfTexte(plan,blatt,planSichtbar):[]);
  // Zoomgrenzen in Pixel je Seitenpunkt: PDF-Blätter liegen bei 0,05–12,
  // Zeichnungseinheiten reichen von Millimetern bis Kilometern.
  const zMin=$derived(Math.min(.05,40/Math.max(breite,hoehe))), zMax=$derived(Math.max(12,20000/Math.min(breite,hoehe)));
  // Größe des gedrehten Blattes in Seitenpunkten (Box, in der gescrollt wird).
  const boxB=$derived(gedreht?hoehe:breite), boxH=$derived(gedreht?breite:hoehe);
  const z=(n:number,stellen=2)=>n.toLocaleString("de-DE",{maximumFractionDigits:stellen});
  const rect=(a:{id?:string;von:Point;bis:Point})=>{const r=a.id&&rahmenVorschau?.id===a.id?rahmenVorschau:a;return {x:Math.min(r.von[0],r.bis[0]),y:Math.min(r.von[1],r.bis[1]),w:Math.abs(r.bis[0]-r.von[0]),h:Math.abs(r.bis[1]-r.von[1])}};
  // Seitenpunkt → Punkt der gedrehten Box (Seitenpunkte, ohne Zoom).
  function boxPunkt(p:Point):Point{
    if(drehung===90)return [hoehe-p[1],p[0]];
    if(drehung===180)return [breite-p[0],hoehe-p[1]];
    if(drehung===270)return [p[1],breite-p[0]];
    return p;
  }
  function boxRect(r:{x:number;y:number;w:number;h:number}){
    return umgrenzung([[r.x,r.y],[r.x+r.w,r.y],[r.x+r.w,r.y+r.h],[r.x,r.y+r.h]].map(p=>boxPunkt(p as Point)));
  }
  const blattTransform=$derived(drehung===90?"rotate(90deg) translateY(-100%)":drehung===180?"rotate(180deg) translate(-100%,-100%)":drehung===270?"rotate(270deg) translateX(-100%)":"none");
  $effect(()=>{
    if(!sichtbar())return;
    const source=pdfBase64; let gone=false; let loader:PDFDocumentLoadingTask|undefined;
    doc=null;punkte=[];seitenbilder=[];seitenzahl=1;
    if(!source)return;
    bereit=false;
    void(async()=>{
      const {getDocument,GlobalWorkerOptions}=await import("pdfjs-dist");
      if(gone)return;
      GlobalWorkerOptions.workerSrc=workerUrl;
      loader=getDocument({data:Uint8Array.from(atob(source),c=>c.charCodeAt(0)),isEvalSupported:false,
        cMapUrl:new URL("./pdfjs/cmaps/",document.baseURI).href,cMapPacked:true,
        standardFontDataUrl:new URL("./pdfjs/standard_fonts/",document.baseURI).href});
      const d=await loader.promise;if(!gone){doc=d;seitenzahl=d.numPages}
    })().catch(e=>{if(!gone)onfehler(`PDF: ${String(e)}`)});
    return()=>{gone=true;void loader?.destroy();for(const s of seitenbilder)URL.revokeObjectURL(s.url)};
  });
  async function rastern(p:PDFPageProxy,scale:number){
    rasterTask?.cancel();
    const raster=p.getViewport({scale});
    const buffer=document.createElement("canvas");buffer.width=Math.ceil(raster.width);buffer.height=Math.ceil(raster.height);
    const task=p.render({canvasContext:buffer.getContext("2d")!,viewport:raster});rasterTask=task;
    await task.promise;
    const blob=await new Promise<Blob>((resolve,reject)=>buffer.toBlob(b=>b?resolve(b):reject(Error("PDF-Seitenbild konnte nicht erstellt werden")),"image/png"));
    return URL.createObjectURL(blob);
  }
  $effect(()=>{
    if(!sichtbar())return;
    const d=doc,nummer=seite; let gone=false,url="";
    bereit=false;geometry={points:[],segments:[],limited:false};punkte=[];edit=null;page=null;rasterScale=0;drehung=0;
    if(!d)return;
    void(async()=>{
      const p=await d.getPage(nummer);if(gone)return;
      const viewport=p.getViewport({scale:1});
      const scale=Math.min(3,2200/Math.max(viewport.width,viewport.height));
      url=await rastern(p,scale);
      if(gone){URL.revokeObjectURL(url);return}
      const {OPS}=await import("pdfjs-dist");const list=await p.getOperatorList();
      if(gone){URL.revokeObjectURL(url);return;}
      seitenbild=url;rasterScale=scale;page=p;
      breite=viewport.width;hoehe=viewport.height;
      geometry=extractGeometry(list,OPS,viewport.transform as Matrix);
      // Beim Öffnen Breite einpassen, höchstens 100 %; „Einpassen“ zeigt das ganze Blatt.
      zoom=Math.min(1,((scroll?.clientWidth??800)-2*PAD)/breite);
      bereit=true;
      await ansichtLaden(nummer);
    })().catch(e=>{if(!gone&&!String(e).includes("cancelled"))onfehler(`PDF-Seite: ${String(e)}`)});
    return()=>{gone=true;rasterTask?.cancel();if(url)URL.revokeObjectURL(url);const bild=untrack(()=>seitenbild);if(bild)URL.revokeObjectURL(bild)};
  });
  async function ansichtLaden(nummer:number){
    if(!ansichtKey)return;
    try{
      const v=JSON.parse(localStorage.getItem(`${ansichtKey}:${nummer}`)??"null");
      if(v&&v.zoom>=zMin&&v.zoom<=zMax&&[0,90,180,270].includes(v.drehung)){
        zoom=v.zoom;drehung=v.drehung;await tick();if(scroll){scroll.scrollLeft=Number(v.x)||0;scroll.scrollTop=Number(v.y)||0}
      }
    }catch{/* Ansichtssicherung ist optional. */}
  }
  // DXF-Unterlage: der Zeichnungsumfang ist das Blatt; Fang an sichtbaren Ebenen.
  $effect(()=>{
    if(!sichtbar())return;
    const p=plan,b=blatt,vis=planSichtbar; if(!p)return;
    untrack(()=>{
      punkte=[];edit=null;page=null;rasterScale=0;cursor=null;
      if(!b){bereit=false;geometry={points:[],segments:[],limited:false};onfehler("Die DXF-Zeichnung enthält keine darstellbaren Elemente.");return}
      const neu=!bereit||breite!==b.breite||hoehe!==b.hoehe;
      breite=b.breite;hoehe=b.hoehe;geometry=dxfGeometrie(p,b,vis);
      if(neu){drehung=0;zoom=passend();bereit=true;void ansichtLaden(1)}
    });
  });
  // PDF-Hintergründe einer Zeichnung rastern (einmal je Dokumentversion und Seite).
  $effect(()=>{
    if(!sichtbar())return;
    for(const hg of hintergruende){
      const key=hgKey(hg), quelle=hg.pdfBase64;
      if(!quelle||hg.plan||hgBilder[key]||hgLaeuft.has(key))continue;
      hgLaeuft.add(key);
      void(async()=>{
        const {getDocument,GlobalWorkerOptions,OPS}=await import("pdfjs-dist");
        GlobalWorkerOptions.workerSrc=workerUrl;
        const loader=getDocument({data:Uint8Array.from(atob(quelle),c=>c.charCodeAt(0)),isEvalSupported:false,
          cMapUrl:new URL("./pdfjs/cmaps/",document.baseURI).href,cMapPacked:true,standardFontDataUrl:new URL("./pdfjs/standard_fonts/",document.baseURI).href});
        try{
          const d=await loader.promise, p=await d.getPage(Math.min(hg.h.seite,d.numPages)), v=p.getViewport({scale:1});
          const scale=Math.min(2,2200/Math.max(v.width,v.height)), vp=p.getViewport({scale}), c=document.createElement("canvas");
          c.width=Math.ceil(vp.width);c.height=Math.ceil(vp.height);
          await p.render({canvasContext:c.getContext("2d")!,viewport:vp}).promise;
          const list=await p.getOperatorList();
          const blob=await new Promise<Blob|null>(r=>c.toBlob(r)); if(!blob)return;
          hgBilder={...hgBilder,[key]:{url:URL.createObjectURL(blob),breite:v.width,hoehe:v.height,geometry:extractGeometry(list,OPS,v.transform as Matrix)}};
        }finally{hgLaeuft.delete(key);void loader.destroy()}
      })().catch(e=>onfehler(`Hintergrund: ${String(e)}`));
    }
  });
  onDestroy(()=>{for(const b of Object.values(hgBilder))URL.revokeObjectURL(b.url)});
  // Eigene Zeichnung: Blatt in Metern, so groß wie die Inhalte samt Rand; Fang an den Hintergründen.
  $effect(()=>{
    if(!zeichnung||!sichtbar())return;
    const punkte=[...markierungen.flatMap(m=>eckpunkte(m.geometrie)),...elemente.flatMap(e=>elementPunkte(e)),...ausschnitte.flatMap(a=>[a.von,a.bis])];
    const b=blattmass(punkte,hgAnsicht.map(a=>({h:a.hg.h,breite:a.breite,hoehe:a.hoehe})));
    const g:SnapGeometry={points:[],segments:[],limited:false};
    for(const a of hgAnsicht){
      if(!a.hg.h.sichtbar)continue;
      const t=(p:Point)=>hgPunkt(a.hg.h,p);
      g.points.push(...a.geometry.points.map(t));
      g.segments.push(...a.geometry.segments.map(s=>[t(s[0]),t(s[1])] as [Point,Point]));
      g.limited||=a.geometry.limited;
    }
    untrack(()=>{
      const neu=!bereit||breite!==b.breite||hoehe!==b.hoehe;
      breite=b.breite;hoehe=b.hoehe;geometry=g;page=null;rasterScale=0;
      if(!bereit){drehung=0;zoom=passend();bereit=true;void ansichtLaden(1)}
      else if(neu)zoom=Math.max(zMin,Math.min(zMax,zoom));
    });
  });
  // Nach dem Zoomen scharf nachrastern; nie unter die bisherige Auflösung.
  $effect(()=>{
    const zielZoom=zoom; void bereit;
    clearTimeout(rasterTimer);
    rasterTimer=setTimeout(()=>{
      const p=page; if(!p||!bereit)return;
      const grenze=Math.min(6,4096/Math.max(breite,hoehe));
      const noetig=Math.min(grenze,zielZoom*(window.devicePixelRatio||1));
      if(noetig<=rasterScale*1.05)return;
      const alt=seitenbild;
      void rastern(p,noetig).then(u=>{if(page!==p){URL.revokeObjectURL(u);return}seitenbild=u;rasterScale=noetig;if(alt)URL.revokeObjectURL(alt)}).catch(()=>{});
    },200);
    return()=>clearTimeout(rasterTimer);
  });
  $effect(()=>{void werkzeug;untrack(abbrechen)});
  $effect(()=>{
    const b=vergleichBase64,n=seite;if(!b){vergleichBild="";return}
    let gone=false,url="",loader:PDFDocumentLoadingTask|undefined,task:RenderTask|undefined;
    void(async()=>{const {getDocument}=await import("pdfjs-dist");loader=getDocument({data:Uint8Array.from(atob(b),c=>c.charCodeAt(0)),isEvalSupported:false});
      const d=await loader.promise,p=await d.getPage(Math.min(n,d.numPages)),v=p.getViewport({scale:1});
      vergleichGroesse={breite:v.width,hoehe:v.height};const scale=Math.min(2,2200/Math.max(v.width,v.height)),vp=p.getViewport({scale}),c=document.createElement("canvas");c.width=Math.ceil(vp.width);c.height=Math.ceil(vp.height);
      task=p.render({canvasContext:c.getContext("2d")!,viewport:vp});await task.promise;
      const blob=await new Promise<Blob|null>(r=>c.toBlob(r));if(!blob)return;url=URL.createObjectURL(blob);if(gone)URL.revokeObjectURL(url);else vergleichBild=url;
    })().catch(e=>{if(!gone)onfehler(`Planvergleich: ${String(e)}`)});
    return()=>{gone=true;task?.cancel();void loader?.destroy();if(url)URL.revokeObjectURL(url)};
  });
  let ansichtTimer:ReturnType<typeof setTimeout>|undefined,ansichtPending:{key:string;wert:string}|null=null;
  function ansichtSchreiben(){const p=ansichtPending;ansichtPending=null;if(p)try{localStorage.setItem(p.key,p.wert)}catch{}}
  function ansichtSichern(){if(!ansichtKey||!bereit||!scroll)return;ansichtPending={key:`${ansichtKey}:${seite}`,wert:JSON.stringify({zoom,drehung,x:scroll.scrollLeft,y:scroll.scrollTop})};clearTimeout(ansichtTimer);ansichtTimer=setTimeout(ansichtSchreiben,120)}
  onDestroy(()=>{ansichtGeneration++;clearTimeout(ansichtTimer);ansichtSchreiben()});
  $effect(()=>{void zoom;void drehung;if(bereit)untrack(ansichtSichern)});
  export function abbrechen(){
    punkte=[];cursor=null;drag=null;edit=null;schieben=null;rahmenDrag=null;rahmenVorschau=null;pan=null;pinch=null;touchPunkt=null;leertaste=false;masseingabe=null;elementDrag=null;elementEdit=null;
    for(const id of zeiger.keys())if(svg?.hasPointerCapture(id))svg.releasePointerCapture(id);zeiger.clear();
  }
  $effect(()=>{window.addEventListener("blur",abbrechen);return()=>window.removeEventListener("blur",abbrechen)});
  $effect(()=>{if(masseingabe!==null)masseingabeFeld?.focus()});
  // Ausschnitt öffnen: Rahmen einpassen.
  $effect(()=>{
    const a=ausschnitt; void bereit;
    if(!a||!bereit)return;
    void aufBereich(rect(a));
  });
  function passend(){
    const w=(scroll?.clientWidth??800)-2*PAD, h=(scroll?.clientHeight??600)-2*PAD;
    return Math.max(zMin,Math.min(w/boxB,h/boxH));
  }
  async function aufBereich(r:{x:number;y:number;w:number;h:number}){
    const el=scroll,n=++ansichtGeneration;if(!el)return;
    const b=boxRect(r);
    const w=el.clientWidth-2*PAD, h=el.clientHeight-2*PAD;
    zoom=Math.max(zMin,Math.min(zMax,Math.min(w/(b.w+40),h/(b.h+40))));
    await tick();
    if(n!==ansichtGeneration||scroll!==el||!el.isConnected)return;
    el.scrollLeft=(b.x+b.w/2)*zoom+PAD-el.clientWidth/2;
    el.scrollTop=(b.y+b.h/2)*zoom+PAD-el.clientHeight/2;
  }
  export function einpassen(){
    if(ausschnitt)return void aufBereich(rect(ausschnitt));
    zoom=passend();
  }
  export function aufAuswahl(){
    if(gewaehlt)void aufBereich(umgrenzung(eckpunkte(gewaehlt.geometrie)));
  }
  export function zoomfensterStarten(){zoomfenster=!zoomfenster;zf=null}
  export function drehen(schritt:number){drehung=((drehung+schritt)%360+360)%360;punkte=[];cursor=null}
  async function zoomAn(clientX:number,clientY:number,faktorZoom:number){
    const el=scroll,n=++ansichtGeneration;if(!el)return;
    const r=el.getBoundingClientRect();
    const px=(el.scrollLeft+(clientX-r.left)-PAD)/zoom, py=(el.scrollTop+(clientY-r.top)-PAD)/zoom;
    const neu=Math.max(zMin,Math.min(zMax,zoom*faktorZoom));
    if(neu===zoom)return;
    zoom=neu;
    await tick();
    if(n!==ansichtGeneration||scroll!==el||!el.isConnected)return;
    el.scrollLeft=px*neu+PAD-(clientX-r.left);
    el.scrollTop=py*neu+PAD-(clientY-r.top);
  }
  function zoomMitte(faktorZoom:number){
    if(!scroll)return;
    const r=scroll.getBoundingClientRect();
    void zoomAn(r.left+r.width/2,r.top+r.height/2,faktorZoom);
  }
  // Strg/Cmd+Rad und Touchpad-Pinch zoomen um den Mauszeiger; Rad allein scrollt.
  $effect(()=>{
    const el=scroll; if(!el)return;
    const rad=(e:WheelEvent)=>{
      if(!(e.ctrlKey||e.metaKey)||!bereit)return;
      e.preventDefault();
      const delta=e.deltaMode===1?e.deltaY*16:e.deltaMode===2?e.deltaY*400:e.deltaY;
      void zoomAn(e.clientX,e.clientY,Math.exp(-delta*0.0015));
    };
    el.addEventListener("wheel",rad,{passive:false});
    return()=>el.removeEventListener("wheel",rad);
  });
  $effect(()=>{
    const down=(e:KeyboardEvent)=>{
      if(e.code!=="Space"||(e.target as Element).closest("input,select,textarea,button"))return;
      if(svg&&(svg===e.target||svg.contains(e.target as Node)||scroll?.contains(e.target as Node))){e.preventDefault();leertaste=true}
    };
    const up=(e:KeyboardEvent)=>{if(e.code==="Space")leertaste=false};
    window.addEventListener("keydown",down);window.addEventListener("keyup",up);
    return()=>{window.removeEventListener("keydown",down);window.removeEventListener("keyup",up)};
  });
  function changed(g:Plangeometrie,index:number,p:Point):Plangeometrie {
    return punktAendern(g,index,p);
  }
  // Bildschirm → Seitenpunkt; die Matrix enthält Zoom und Drehung der Darstellung.
  function roh(e:{clientX:number;clientY:number}):Point{
    const ctm=svg?.getScreenCTM();
    if(!ctm)return [0,0];
    const p=new DOMPoint(e.clientX,e.clientY).matrixTransform(ctm.inverse());
    return [Math.max(0,Math.min(breite,p.x)),Math.max(0,Math.min(hoehe,p.y))];
  }
  function position(e:PointerEvent):Point {
    let p=roh(e);
    const f=fang&&!e.altKey?fangindex.suchen(p,8/zoom,fangoptionen):null;
    const eigene=fang&&fangoptionen.eigene&&!e.altKey?snap(p,{points:eigenePunkte(),segments:eigeneKanten(),limited:false},8/zoom):null;
    if(eigene&&(!f||Math.hypot(eigene.point[0]-p[0],eigene.point[1]-p[1])<Math.hypot(f.point[0]-p[0],f.point[1]-p[1]))){p=eigene.point;fangart=`Eigene ${eigene.kind}`}else{p=f?.point??p;fangart=f?.kind??""}
    // Shift rastet auf Waagerechte/Senkrechte/45° zum letzten Punkt.
    if(e.shiftKey&&punkte.length){
      const a=punkte[punkte.length-1], dx=p[0]-a[0], dy=p[1]-a[1];
      const winkel=Math.round(Math.atan2(dy,dx)/(Math.PI/4))*(Math.PI/4), l=Math.hypot(dx,dy);
      p=[a[0]+Math.cos(winkel)*l,a[1]+Math.sin(winkel)*l];
    }
    return p;
  }
  function elementKanten():[Point,Point][]{return angezeigteElemente.filter(e=>e.id!==elementAuswahl&&e.art!=="text").flatMap(e=>{
    if(e.art==="wand"&&faktor!==null){const k=wandKontur(e.von,e.bis,e.dicke_m/faktor);return k.map((p,i)=>[p,k[(i+1)%4]] as [Point,Point])}
    return [[elementPunkte(e)[0],elementPunkte(e)[1]] as [Point,Point]];
  })}
  function eigeneKanten():[Point,Point][]{return [...elementKanten(),...angezeigt.filter(m=>!auswahlIds.includes(m.id)&&m.id!==auswahl).flatMap(m=>{
    const g=m.geometrie,ps=eckpunkte(g);if(g.art==="polygon")return ps.map((p,i)=>[p,ps[(i+1)%ps.length]] as [Point,Point]);
    if(g.art==="strecke")return [[g.von,g.bis] as [Point,Point]];
    if(g.art==="rechteck"){const a=g.von,b=g.bis,r:Point[]=[a,[b[0],a[1]],b,[a[0],b[1]]];return r.map((p,i)=>[p,r[(i+1)%4]] as [Point,Point])}return [];
  })]}
  // Fang auch an eigenen Markierungen und Bezugslinien.
  function eigenePunkte():Point[]{
    const ps:Point[]=[];
    for(const m of angezeigt)if(m.id!==auswahl&&!auswahlIds.includes(m.id))ps.push(...eckpunkte(m.geometrie));
    for(const a of ausschnitte)if(a.bezugslinie)ps.push(a.bezugslinie.von,a.bezugslinie.bis);
    for(const e of angezeigteElemente)if(e.id!==elementAuswahl){ps.push(...elementPunkte(e));if(e.art==="wand"&&faktor!==null)ps.push(...wandKontur(e.von,e.bis,e.dicke_m/faktor))}
    return ps;
  }
  function shape(ps:Point[]):Plangeometrie|null {
    if((werkzeug==="hoehenpunkt"||werkzeug==="bezug"||werkzeug==="text") && ps.length)return{art:"hoehenpunkt",punkt:ps[0]};
    if(werkzeug==="polygon")return ps.length>=3?{art:"polygon",punkte:ps}:null;
    if(ps.length<2)return null;
    if(werkzeug==="rechteck"||werkzeug==="ausschnitt")return{art:"rechteck",von:ps[0],bis:ps[1]};
    if(werkzeug==="kreis")return{art:"kreis",mitte:ps[0],rand:ps[1]};
    return{art:"strecke",von:ps[0],bis:ps[1]};
  }
  // Zeichenelement aus den gesetzten Punkten: Wand und Maßkette aus zwei Punkten, Text aus einem.
  function elementForm(ps:Point[]):Zeichenform|null{
    if(werkzeug==="text")return ps.length?{art:"text",punkt:ps[0],inhalt:"Text",hoehe:zeichnung?.25:12,drehung_grad:0}:null;
    if(ps.length<2||Math.hypot(ps[1][0]-ps[0][0],ps[1][1]-ps[0][1])===0)return null;
    if(werkzeug==="wand")return{art:"wand",von:ps[0],bis:ps[1],dicke_m:wanddicke};
    return{art:"masskette",von:ps[0],bis:ps[1],abstand:zeichnung?-.8:-20};
  }
  function finish(){
    if(elementWerkzeug){const f=elementForm(punkte);if(f){onelement(f);punkte=[];cursor=null}return}
    const g=shape(punkte);if(g){onfertig(g);punkte=[];cursor=null}
  }
  function punktSetzen(p:Point){
    if(werkzeug==="polygon"&&punkte.length>=3&&Math.hypot(p[0]-punkte[0][0],p[1]-punkte[0][1])<8/zoom){finish();return}
    punkte=[...punkte,p];
    if(werkzeug!=="polygon" && (punkte.length===2||werkzeug==="hoehenpunkt"||werkzeug==="bezug"||werkzeug==="text"))finish();
  }
  // Maßeingabe: „5,20“ (Länge in m, Richtung wie zuletzt bzw. nach rechts),
  // „5,20;90“ (Länge; Winkel in Grad, 90 = nach oben), beim Rechteck „Breite;Höhe“.
  function masseUebernehmen(){
    const text=(masseingabe??"").trim(); masseingabe=null;
    if(!text||!punkte.length)return;
    if(faktor===null){onfehler("Maßeingabe benötigt einen Maßstab.");return}
    const zahl=(t:string)=>Number(t.replace(",","."));
    const [a,b]=text.split(/[;\s]+/);
    const letzter=punkte[punkte.length-1];
    if(werkzeug==="rechteck"){
      const w=zahl(a), h=zahl(b??a);
      if(!(w>0&&h>0))return;
      punktSetzen([letzter[0]+w/faktor,letzter[1]+h/faktor]);return;
    }
    const l=zahl(a); if(!(l>0))return;
    let winkel:number;
    if(b!==undefined&&b!=="")winkel=zahl(b)*Math.PI/180;
    else if(cursor){winkel=-Math.atan2(cursor[1]-letzter[1],cursor[0]-letzter[0])}
    else winkel=0;
    punktSetzen([letzter[0]+Math.cos(winkel)*l/faktor,letzter[1]-Math.sin(winkel)*l/faktor]);
  }
  function down(e:PointerEvent) {
    if(!bereit)return;
    svg?.focus({preventScroll:true});
    zeiger.set(e.pointerId,{x:e.clientX,y:e.clientY});
    if(zeiger.size===2){
      const [a,b]=[...zeiger.values()];
      pinch={abstand:Math.hypot(a.x-b.x,a.y-b.y),mitte:{x:(a.x+b.x)/2,y:(a.y+b.y)/2}};
      punkte=[];drag=null;schieben=null;rahmenDrag=null;rahmenVorschau=null;edit=null;pan=null;touchPunkt=null;
      svg?.setPointerCapture(e.pointerId);return;
    }
    if(e.button===1 || werkzeug==="verschieben" || leertaste){
      e.preventDefault();pan={x:e.clientX,y:e.clientY,left:scroll!.scrollLeft,top:scroll!.scrollTop};
      svg?.setPointerCapture(e.pointerId);return;
    }
    if(e.button!==0)return;
    if(zoomfenster){
      const p=roh(e);
      if(!zf){zf=p;return}
      const r=umgrenzung([zf,p]);zoomfenster=false;zf=null;
      if(r.w>2&&r.h>2)void aufBereich(r);
      return;
    }
    const target=e.target as SVGElement;
    const id=target.closest("[data-markierung]")?.getAttribute("data-markierung");
    const rahmenId=target.closest("[data-rahmen]")?.getAttribute("data-rahmen");
    if(werkzeug==="auswahl"){
      const index=target.getAttribute("data-ecke");
      if(id){
        const m=angezeigt.find(m=>m.id===id);
        if(m&&fremd(m))return;
        onauswahl(id,e.shiftKey||e.ctrlKey||e.metaKey);
        if(e.shiftKey||e.ctrlKey||e.metaKey)return;
        if(index!==null && gewaehlt && gewaehlt.id===id && !gesperrt){
          drag={id:gewaehlt.id,index:Number(index),g:structuredClone($state.snapshot(gewaehlt.geometrie))};
          onpunkt(Number(index));
          svg?.setPointerCapture(e.pointerId);
        } else if(m && darf && !ebenen.find(x=>x.id===m.ebene_id)?.gesperrt){
          schieben={id,start:roh(e),g:structuredClone($state.snapshot(m.geometrie)),bewegt:false};
          svg?.setPointerCapture(e.pointerId);
        }
        return;
      }
      if(rahmenId&&!aktiv){
        const a=ausschnitte.find(a=>a.id===rahmenId);
        if(!a)return;
        if(e.detail>=2){onoeffnen(rahmenId);return}
        onrahmenauswahl(rahmenId);
        if(darf){
          const ecke=target.getAttribute("data-rahmenecke");
          rahmenDrag={id:rahmenId,ecke:ecke===null?-1:Number(ecke),von:[...a.von],bis:[...a.bis],start:roh(e),bewegt:false};
          svg?.setPointerCapture(e.pointerId);
        }
        return;
      }
      const elId=target.closest("[data-element]")?.getAttribute("data-element");
      if(elId){
        const el=angezeigteElemente.find(e=>e.id===elId); if(!el)return;
        onelementauswahl(elId);onauswahl("");
        if(darf&&!ebenen.find(x=>x.id===el.ebene_id)?.gesperrt){
          const ecke=target.getAttribute("data-elementecke");
          elementDrag={id:elId,index:ecke===null?-1:Number(ecke),e:structuredClone($state.snapshot(el)),start:roh(e),bewegt:false};
          svg?.setPointerCapture(e.pointerId);
        }
        return;
      }
      onauswahl("");onrahmenauswahl("");onelementauswahl("");
      return;
    }
    if(!darf)return;
    if(e.pointerType==="touch"){touchPunkt=position(e);svg?.setPointerCapture(e.pointerId)}else punktSetzen(position(e));
  }
  function move(e:PointerEvent) {
    if(zeiger.has(e.pointerId))zeiger.set(e.pointerId,{x:e.clientX,y:e.clientY});
    if(pinch&&zeiger.size===2){
      const [a,b]=[...zeiger.values()];
      const abstand=Math.hypot(a.x-b.x,a.y-b.y), mitte={x:(a.x+b.x)/2,y:(a.y+b.y)/2};
      if(scroll){scroll.scrollLeft-=mitte.x-pinch.mitte.x;scroll.scrollTop-=mitte.y-pinch.mitte.y}
      if(pinch.abstand>0)void zoomAn(mitte.x,mitte.y,abstand/pinch.abstand);
      pinch={abstand,mitte};return;
    }
    if(pan){scroll!.scrollLeft=pan.left-(e.clientX-pan.x);scroll!.scrollTop=pan.top-(e.clientY-pan.y);return}
    if(!bereit)return;
    if(rahmenDrag){
      const p=roh(e), dx=p[0]-rahmenDrag.start[0], dy=p[1]-rahmenDrag.start[1];
      if(Math.hypot(dx,dy)*zoom>3)rahmenDrag.bewegt=true;
      if(!rahmenDrag.bewegt)return;
      const r=rect({von:rahmenDrag.von,bis:rahmenDrag.bis});
      let von:Point=[r.x,r.y], bis:Point=[r.x+r.w,r.y+r.h];
      if(rahmenDrag.ecke<0){von=[r.x+dx,r.y+dy];bis=[r.x+r.w+dx,r.y+r.h+dy]}
      else{
        const q=position(e);
        if(rahmenDrag.ecke===0)von=q;else if(rahmenDrag.ecke===1){von=[von[0],q[1]];bis=[q[0],bis[1]]}
        else if(rahmenDrag.ecke===2)bis=q;else{von=[q[0],von[1]];bis=[bis[0],q[1]]}
      }
      rahmenVorschau={id:rahmenDrag.id,von,bis};return;
    }
    cursor=zoomfenster?roh(e):position(e);
    if(elementDrag){
      const p=roh(e), dx=p[0]-elementDrag.start[0], dy=p[1]-elementDrag.start[1];
      if(Math.hypot(dx,dy)*zoom>3)elementDrag.bewegt=true;
      if(elementDrag.bewegt)elementEdit=elementDrag.index<0?elementVerschieben(elementDrag.e,dx,dy):elementSetzen(elementDrag.e,elementDrag.index,cursor);
      return;
    }
    if(drag)edit=changed(drag.g,drag.index,cursor);
    if(schieben){
      const p=roh(e), dx=p[0]-schieben.start[0], dy=p[1]-schieben.start[1];
      if(Math.hypot(dx,dy)*zoom>3)schieben.bewegt=true;
      if(schieben.bewegt)edit=verschieben(schieben.g,dx,dy);
    }
  }
  function up(e:PointerEvent){
    if(touchPunkt&&zeiger.size===1&&!pan&&!pinch){punktSetzen(touchPunkt);touchPunkt=null}
    zeiger.delete(e.pointerId);
    if(zeiger.size<2)pinch=null;
    if(drag&&edit)onedit(drag.id,edit);
    if(schieben?.bewegt&&edit){
      if(auswahlIds.length>1&&auswahlIds.includes(schieben.id)){const a=eckpunkte(schieben.g)[0],b=eckpunkte(edit)[0];onverschieben(auswahlIds,b[0]-a[0],b[1]-a[1])}
      else onedit(schieben.id,edit);
    }
    if(rahmenDrag?.bewegt&&rahmenVorschau)onrahmen(rahmenVorschau.id,rahmenVorschau.von,rahmenVorschau.bis);
    if(elementDrag?.bewegt&&elementEdit)onelementedit(elementDrag.id,elementEdit);
    rahmenVorschau=null;elementDrag=null;elementEdit=null;
    drag=null;edit=null;pan=null;schieben=null;rahmenDrag=null;
    if(svg?.hasPointerCapture(e.pointerId))svg.releasePointerCapture(e.pointerId);
  }
  function key(e:KeyboardEvent){
    if((e.target as Element).closest("input,select,textarea,button"))return;
    if(e.key==="Escape"){
      if(zoomfenster){e.stopPropagation();zoomfenster=false;zf=null}
      else if(punkte.length||drag||schieben||rahmenDrag||elementDrag){e.stopPropagation();abbrechen()}
    }
    else if(e.key==="Enter"){e.preventDefault();finish()}
    else if(e.key==="Backspace"){e.preventDefault();punkte=punkte.slice(0,-1)}
    else if(/^[0-9,.]$/.test(e.key)&&punkte.length&&["polygon","strecke","rechteck","wand","masskette"].includes(werkzeug)){e.preventDefault();masseingabe=e.key}
  }
  const preview=$derived(punkte.length&&cursor?shape([...punkte,cursor]):null);
  // Höhenvorschau am Cursor: Bezugslinie des Ausschnitts, sonst Seitenvertikale (nur Anzeige).
  const hoehenVorschau=$derived.by(()=>{
    if(werkzeug!=="hoehenpunkt"||!cursor||faktor===null)return null;
    const b=ausschnitt?.bezugslinie; if(!b)return null;
    const l=Math.hypot(b.bis[0]-b.von[0],b.bis[1]-b.von[1]); if(!l)return null;
    const d=[(b.bis[0]-b.von[0])/l,(b.bis[1]-b.von[1])/l]; let n=[d[1],-d[0]];
    if(n[1]>0||(n[1]===0&&n[0]>0))n=[-n[0],-n[1]];
    return b.hoehe_m+((cursor[0]-b.von[0])*n[0]+(cursor[1]-b.von[1])*n[1])*faktor;
  });
  const hoehenHilfe=$derived.by(()=>{
    if(werkzeug!=="hoehenpunkt"||!cursor)return null;
    const b=ausschnitt?.bezugslinie;
    const l=b?Math.hypot(b.bis[0]-b.von[0],b.bis[1]-b.von[1]):0;
    const d:Point=b&&l?[(b.bis[0]-b.von[0])/l,(b.bis[1]-b.von[1])/l]:[1,0];
    const s=Math.max(breite,hoehe);
    return {x1:cursor[0]-d[0]*s,y1:cursor[1]-d[1]*s,x2:cursor[0]+d[0]*s,y2:cursor[1]+d[1]*s};
  });
  const vorschau=$derived.by(()=>{
    if(hoehenVorschau!==null)return `${z(hoehenVorschau)} ${ausschnitt?.bezugslinie?.system??"m"}`;
    if(!preview||werkzeug==="ausschnitt")return "";
    const m=vorschauMass(preview,faktor);
    if(preview.art==="strecke"&&cursor&&punkte.length){
      const achsen=streckenachsen(punkte[0],cursor,ausschnitt?.bezugslinie),dy=achsen.hoehe,dx=achsen.horizontal;
      return faktor===null?"Maßstab fehlt":`${z(m.laenge_m!)} m · Δh ${z(dy*faktor)} m · ⟷ ${z(dx*faktor)} m`;
    }
    if(m.flaeche_m2!==null)return `${z(m.flaeche_m2)} m² · Umfang ${z(m.laenge_m!)} m`;
    if(m.laenge_m!==null)return `${z(m.laenge_m)} m`;
    return faktor===null?"Maßstab fehlt":"";
  });
  function beschriftung(m:Planmarkierung){
    if(!masse)return "";
    const w=wertMap.get(m.id); if(!w)return "";
    if(m.geometrie.art==="hoehenpunkt")return w.hoehe_m!==null?`${z(w.hoehe_m)} m`:"";
    if(m.geometrie.art==="strecke")return w.laenge_m!==null?`${z(w.laenge_m)} m${w.hoehe_m?` · Δh ${z(w.hoehe_m)} m`:""}`:"";
    return w.flaeche_m2!==null?`${z(w.flaeche_m2)} m²`:"";
  }
  function flaechenpfad(g:Extract<Plangeometrie,{art:"flaeche"}>){return g.polygone.flatMap(p=>[p.aussen,...(p.innen??[])]).map(r=>r.map((p,i)=>`${i?"L":"M"}${p[0]} ${p[1]}`).join(" ")+" Z").join(" ")}
  function miniatur(node:HTMLElement,n:number){
    const d=doc;let gone=false,task:RenderTask|undefined;
    const observer=new IntersectionObserver(entries=>{if(!entries.some(e=>e.isIntersecting)||!d)return;observer.disconnect();
      void(async()=>{const p=await d.getPage(n);if(gone)return;const v=p.getViewport({scale:1}),vp=p.getViewport({scale:140/Math.max(v.width,v.height)}),c=document.createElement("canvas");c.width=Math.ceil(vp.width);c.height=Math.ceil(vp.height);
        task=p.render({canvasContext:c.getContext("2d")!,viewport:vp});await task.promise;if(!gone)seitenbilder=[...seitenbilder.filter(x=>x.nummer!==n),{nummer:n,url:c.toDataURL("image/png")}];
      })().catch(e=>{if(!gone)onfehler(`Seitenvorschau: ${String(e)}`)});
    },{rootMargin:"200px"});observer.observe(node);
    return {destroy(){gone=true;observer.disconnect();task?.cancel()}};
  }
  const abdunkeln=$derived.by(()=>{
    if(!ausschnitt)return "";
    const r=rect(ausschnitt);
    return `M0 0H${breite}V${hoehe}H0Z M${r.x} ${r.y}H${r.x+r.w}V${r.y+r.h}H${r.x}Z`;
  });
  const cursorKlasse=$derived(leertaste||pan||werkzeug==="verschieben"?"greifen":zoomfenster?"zoomen":werkzeug==="auswahl"?"":"zeichnen");
  const eingabePos=$derived(cursor?boxPunkt(cursor):null);
</script>

<div class="pdf-plan">
  <div class="plan-kopf" role="toolbar" aria-label="Ansicht">
    <Symbolknopf titel="Verkleinern" disabled={!bereit} onclick={()=>zoomMitte(1/1.25)}><WerkzeugIcon art="minus"/></Symbolknopf>
    <span class="zoom">{Math.round(zoom*100)} %</span>
    <Symbolknopf titel="Vergrößern" disabled={!bereit} onclick={()=>zoomMitte(1.25)}><WerkzeugIcon art="plus"/></Symbolknopf>
    <Symbolknopf titel="Einpassen" disabled={!bereit} onclick={einpassen}><WerkzeugIcon art="einpassen"/></Symbolknopf>
    {#if !plan&&!zeichnung}<button type="button" class="sekundaer text" disabled={!bereit} onclick={()=>{zoom=1}} title="72 Punkte je Zoll">1:1</button>{/if}
    <Symbolknopf titel="Auf Auswahl" disabled={!bereit||!gewaehlt} onclick={aufAuswahl}><WerkzeugIcon art="auswahlzoom"/></Symbolknopf>
    <Symbolknopf titel="Zoomfenster" kuerzel="Z" aktiv={zoomfenster} disabled={!bereit} onclick={zoomfensterStarten}><WerkzeugIcon art="zoomfenster"/></Symbolknopf>
    <span class="trenner" aria-hidden="true"></span>
    <Symbolknopf titel="Nach links drehen" disabled={!bereit} onclick={()=>drehen(-90)}><WerkzeugIcon art="links"/></Symbolknopf>
    <Symbolknopf titel="Nach rechts drehen" disabled={!bereit} onclick={()=>drehen(90)}><WerkzeugIcon art="rechts"/></Symbolknopf>
    {#if seitenzahl>1}<Symbolknopf titel={`Seiten (${seitenzahl})`} aktiv={miniaturen} onclick={()=>miniaturen=!miniaturen}><WerkzeugIcon art="seiten"/></Symbolknopf>{/if}
    <span class="trenner" aria-hidden="true"></span>
    <label class="fangschalter" class:aktiv={fang} title="Punktfang"><input type="checkbox" bind:checked={fang}/><WerkzeugIcon art="fang"/><span>Punktfang</span></label>
    <button type="button" class="sekundaer text" aria-expanded={fangOffen} onclick={()=>fangOffen=!fangOffen}>Fangoptionen</button>
    {#if punkte.length}<span class="trenner" aria-hidden="true"></span><button type="button" class="text" onclick={finish} disabled={(werkzeug==="polygon"&&punkte.length<3)}>Abschließen</button><button type="button" class="sekundaer text" onclick={()=>punkte=[]}>Abbrechen</button>{/if}
  </div>
  {#if fangOffen}<div class="fangoptionen" aria-label="Fangoptionen">
    {#each [["endpunkte","Endpunkte"],["mittelpunkte","Mittelpunkte"],["kanten","Kanten"],["schnittpunkte","Schnittpunkte"],["eigene","Eigene Konturen"]] as [id,titel]}<label><input type="checkbox" checked={fangoptionen[id as keyof Fangoptionen]} onchange={e=>fangoptionen={...fangoptionen,[id]:e.currentTarget.checked}}/>{titel}</label>{/each}
  </div>{/if}
  <div class="plan-koerper">
    {#if miniaturen&&seitenzahl>1}
      <nav class="seitenleiste" aria-label="Seiten der Unterlage">
        {#each Array.from({length:seitenzahl},(_,i)=>i+1) as n (`${doc?.fingerprints[0]}:${n}`)}
          {@const bild=seitenbilder.find(s=>s.nummer===n)}
          <button type="button" use:miniatur={n} class:aktiv={n===seite} onclick={()=>{if(n!==seite)onseite(n)}} aria-label={`Seite ${n}`} aria-current={n===seite?"page":undefined}>
            {#if bild}<img src={bild.url} alt=""/>{:else}<span class="platzhalter">…</span>{/if}
            <span>{n}</span>
          </button>
        {/each}
      </nav>
    {/if}
    <div class="plan-scroll" bind:this={scroll} onscroll={ansichtSichern} class:greifen={cursorKlasse==="greifen"}>
      {#if !bereit}<p role="status">{zeichnung?"Zeichnung wird vorbereitet …":"Unterlage wird vorbereitet …"}</p>{/if}
      <div class="papier" style:width={`${boxB*zoom}px`} style:height={`${boxH*zoom}px`} style:visibility={bereit?"visible":"hidden"}>
        <div class="blatt" style:width={`${breite*zoom}px`} style:height={`${hoehe*zoom}px`} style:transform={blattTransform}>
          {#if !plan}<img src={seitenbild||undefined} alt="" draggable="false"/>{/if}
          {#if vergleichBild}<img class="vergleich" src={vergleichBild} alt="Vergleichsunterlage" style:opacity={vergleichDeckkraft} style:width={`${vergleichGroesse.breite/breite*100}%`} style:height={`${vergleichGroesse.hoehe/hoehe*100}%`} style:transform={`translate(${vergleichAusrichtung.dx*zoom}px,${vergleichAusrichtung.dy*zoom}px) rotate(${vergleichAusrichtung.grad}deg) scale(${vergleichAusrichtung.faktor})`} draggable="false"/>{/if}
          <!-- Zeicheneditor mit eigener Tastaturbedienung, keine dekorative SVG. -->
          <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
          <svg bind:this={svg} viewBox={`0 0 ${breite} ${hoehe}`} role="application" class={cursorKlasse}
            aria-label="Zeichenfläche. Punkte anklicken, Enter beendet, Escape bricht ab. Strg+Mausrad zoomt, Leertaste oder mittlere Maustaste verschiebt. Ziffern öffnen die Maßeingabe." tabindex="0"
            onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={abbrechen} onkeydown={key}>
            {#if zeichnung}
              <defs>
                <pattern id={`${rasterId}f`} width="1" height="1" patternUnits="userSpaceOnUse"><path d="M1 0V1M0 1H1" class="raster"/></pattern>
                <pattern id={`${rasterId}g`} width="10" height="10" patternUnits="userSpaceOnUse"><path d="M10 0V10M0 10H10" class="raster grob"/></pattern>
              </defs>
              <rect class="rasterflaeche" width={breite} height={hoehe} style:fill={zoom>=10?`url(#${rasterId}f)`:"none"}/>
              <rect class="rasterflaeche" width={breite} height={hoehe} style:fill={`url(#${rasterId}g)`}/>
            {/if}
            {#each hgAnsicht as a (a.hg.h.id)}{#if a.hg.h.sichtbar}
              <g class="hintergrund" transform={hgTransform(a.hg.h)} opacity={a.hg.h.deckkraft} aria-hidden="true">
                {#if a.url}<image href={a.url} x="0" y="0" width={a.breite} height={a.hoehe} preserveAspectRatio="none"/>{/if}
                {#each a.pfade as pf (pf.farbe)}<path d={pf.d} style:stroke={pf.farbe}/>{/each}
              </g>
            {/if}{/each}
            {#if plan&&blatt}<g class="unterlage" aria-hidden="true">
              {#each pfade as pf (pf.farbe)}<path d={pf.d} style:stroke={pf.farbe}/>{/each}
              {#each texte as t}{#if t.hoehe*zoom>=4}<text x={t.x} y={t.y} font-size={t.hoehe} text-anchor={t.anker} style:fill={t.farbe} transform={t.drehung?`rotate(${-t.drehung} ${t.x} ${t.y})`:undefined}>{#each t.zeilen as zeile,i}<tspan x={t.x} dy={i?t.zeilenabstand:0}>{zeile}</tspan>{/each}</text>{/if}{/each}
            </g>{/if}
            {#if aktiveKalibrierung?.art==="strecke"}<g class="referenz" aria-label={`Referenzstrecke ${z(aktiveKalibrierung.meter)} m`}>
              <line x1={aktiveKalibrierung.von[0]} y1={aktiveKalibrierung.von[1]} x2={aktiveKalibrierung.bis[0]} y2={aktiveKalibrierung.bis[1]}/>
              {#each [aktiveKalibrierung.von,aktiveKalibrierung.bis] as p}<circle cx={p[0]} cy={p[1]} r={4/zoom}/>{/each}
              <text x={aktiveKalibrierung.von[0]} y={aktiveKalibrierung.von[1]+16/zoom} font-size={11/zoom}>Referenz {z(aktiveKalibrierung.meter)} m</text>
            </g>{/if}
            {#if abdunkeln}<path d={abdunkeln} class="abdunkeln" fill-rule="evenodd"/>{/if}
            {#each ausschnitte as a (a.id)}
              {@const r=rect(a)}
              {#if !aktiv}
                <g data-rahmen={a.id} class="rahmen" class:gewaehlt={a.id===rahmenAuswahl}>
                  <rect x={r.x} y={r.y} width={r.w} height={r.h}/>
                  <text x={r.x+6/zoom} y={r.y+16/zoom} font-size={13/zoom}>{a.bezeichnung}</text>
                  {#if a.id===rahmenAuswahl&&darf}
                    {#each [[r.x,r.y],[r.x+r.w,r.y],[r.x+r.w,r.y+r.h],[r.x,r.y+r.h]] as [x,y],i}<rect data-rahmenecke={i} x={x-5/zoom} y={y-5/zoom} width={10/zoom} height={10/zoom} class="rahmenecke"/>{/each}
                  {/if}
                </g>
              {/if}
              {#if a.bezugslinie&&(!aktiv||a.id===aktiv)}
                <g class="bezugslinie"><line x1={a.bezugslinie.von[0]} y1={a.bezugslinie.von[1]} x2={a.bezugslinie.bis[0]} y2={a.bezugslinie.bis[1]}/>
                  <text x={a.bezugslinie.von[0]} y={a.bezugslinie.von[1]-4/zoom} font-size={11/zoom}>{z(a.bezugslinie.hoehe_m)} {a.bezugslinie.system}</text></g>
              {/if}
            {/each}
            {#each angezeigt as m (m.id)}
              <g data-markierung={m.id} class:gewaehlt={m.id===auswahl||auswahlIds.includes(m.id)} class:fehler={fehlerIds.includes(m.id)} class:fremd={fremd(m)}>
                {@render figur(edit&&(drag?.id===m.id||schieben?.id===m.id)?edit:m.geometrie,m.id===auswahl)}
                <text x={eckpunkte(m.geometrie)[0][0]+5/zoom} y={eckpunkte(m.geometrie)[0][1]-5/zoom} font-size={12/zoom}>{m.bezeichnung}{beschriftung(m)?` · ${beschriftung(m)}`:""}</text>
              </g>
            {/each}
            {#each angezeigteElemente as e (e.id)}
              <g data-element={e.id} class="element" class:gewaehlt={e.id===elementAuswahl}>
                {@render element(elementEdit&&elementDrag?.id===e.id?elementEdit:e)}
              </g>
            {/each}
            {#if gewaehltesElement&&werkzeug==="auswahl"&&!elementGesperrt}
              <g data-element={gewaehltesElement.id}>
                {#each elementPunkte(elementEdit&&elementDrag?.id===gewaehltesElement.id?elementEdit:gewaehltesElement) as p,i}<circle data-elementecke={i} cx={p[0]} cy={p[1]} r={7/zoom} class="ecke"/>{/each}
              </g>
            {/if}
            {#if hoehenHilfe}<line x1={hoehenHilfe.x1} y1={hoehenHilfe.y1} x2={hoehenHilfe.x2} y2={hoehenHilfe.y2} class="hoehenhilfe"/>{/if}
            {#if preview&&werkzeug==="wand"&&preview.art==="strecke"&&faktor!==null}<g class="entwurf"><polygon points={wandKontur(preview.von,preview.bis,wanddicke/faktor).map(p=>p.join(",")).join(" ")} class="wand"/></g>
            {:else if preview}<g class="entwurf" class:rahmenentwurf={werkzeug==="ausschnitt"}>{@render figur(preview,false)}</g>{/if}
            {#if zoomfenster&&zf&&cursor}{@const r=umgrenzung([zf,cursor])}<rect x={r.x} y={r.y} width={r.w} height={r.h} class="zoomrahmen"/>{/if}
            {#if punkte.length}<polyline points={[...punkte,...(cursor?[cursor]:[])].map(p=>p.join(",")).join(" ")} class="vorschau"/>{/if}
            {#each punkte as p}<circle cx={p[0]} cy={p[1]} r={4/zoom} class="ecke"/>{/each}
            {#if gewaehlt&&werkzeug==="auswahl"&&!gesperrt&&!fremd(gewaehlt)}
              <g data-markierung={gewaehlt.id}>
                {#each eckpunkte(edit??gewaehlt.geometrie) as p,i}<circle data-ecke={i} cx={p[0]} cy={p[1]} r={7/zoom} class="ecke"/>{/each}
              </g>
            {/if}
            {#if cursor&&fangart}<path d={`M ${cursor[0]-7/zoom} ${cursor[1]} h ${14/zoom} M ${cursor[0]} ${cursor[1]-7/zoom} v ${14/zoom}`} class="fang"/>{/if}
            {#if cursor&&vorschau}<text class="livemass" x={cursor[0]+12/zoom} y={cursor[1]+18/zoom} font-size={12/zoom}>{vorschau}</text>{/if}
          </svg>
        </div>
        {#if masseingabe!==null&&eingabePos}
          <div class="masseingabe" style:left={`${eingabePos[0]*zoom+14}px`} style:top={`${eingabePos[1]*zoom+24}px`}>
            <input bind:this={masseingabeFeld} bind:value={masseingabe} aria-label="Maßeingabe" placeholder={werkzeug==="rechteck"?"Breite;Höhe in m":"Länge in m[;Winkel°]"}
              onkeydown={e=>{if(e.key==="Escape"){e.preventDefault();e.stopPropagation();masseingabe=null;svg?.focus()}else if(e.key==="Enter"){e.preventDefault();e.stopPropagation();masseUebernehmen();svg?.focus()}}}/>
          </div>
        {/if}
      </div>
    </div>
  </div>
  <p class="meta status" role="status">{vorschau?`Vorschau: ${vorschau} · `:""}{fangart || (zeichnung?`Eigene Zeichnung in Metern · Blatt ${z(breite,0)} × ${z(hoehe,0)} m${geometry.points.length?` · ${geometry.points.length} Fangpunkte der Hintergründe`:""}`:geometry.points.length?`${geometry.points.length} Vektorpunkte der Unterlage`:"Keine Vektorpunkte – manuell zeichnen")}
    {geometry.limited?" · Vektorumfang begrenzt":""}{drehung?` · Ansicht ${drehung}° gedreht`:""} · Strg+Rad zoomt, Leertaste verschiebt, Shift rastet, Alt setzt den Fang aus, Ziffern öffnen die Maßeingabe. Enter schließt, Escape bricht ab.</p>
</div>

{#snippet figur(g:Plangeometrie,selected:boolean)}
  {#if g.art==="flaeche"}<path d={flaechenpfad(g)} fill-rule="evenodd" class:aktiv={selected}/>
  {:else if g.art==="polygon"}<polygon points={g.punkte.map(p=>p.join(",")).join(" ")} class:aktiv={selected}/>
  {:else if g.art==="rechteck"}<rect x={Math.min(g.von[0],g.bis[0])} y={Math.min(g.von[1],g.bis[1])} width={Math.abs(g.von[0]-g.bis[0])} height={Math.abs(g.von[1]-g.bis[1])} class:aktiv={selected}/>
  {:else if g.art==="kreis"}<circle cx={g.mitte[0]} cy={g.mitte[1]} r={Math.hypot(g.rand[0]-g.mitte[0],g.rand[1]-g.mitte[1])} class:aktiv={selected}/>
  {:else if g.art==="strecke"}<line x1={g.von[0]} y1={g.von[1]} x2={g.bis[0]} y2={g.bis[1]} class:aktiv={selected}/>
  {:else}<circle cx={g.punkt[0]} cy={g.punkt[1]} r={4/zoom} class:aktiv={selected}/>{/if}
{/snippet}

{#snippet element(e:Zeichenelement)}
  {#if e.art==="wand"}
    {#if faktor!==null}<polygon points={wandKontur(e.von,e.bis,e.dicke_m/faktor).map(p=>p.join(",")).join(" ")} class="wand"/>
    {:else}<line x1={e.von[0]} y1={e.von[1]} x2={e.bis[0]} y2={e.bis[1]} class="wand"/>{/if}
    <line x1={e.von[0]} y1={e.von[1]} x2={e.bis[0]} y2={e.bis[1]} class="wandachse"/>
  {:else if e.art==="masskette"}
    {@const k=masskette(e.von,e.bis,e.abstand)}
    {#each k.hilfen as [a,b]}<line x1={a[0]} y1={a[1]} x2={b[0]+k.ueber[0]*4/zoom} y2={b[1]+k.ueber[1]*4/zoom} class="masshilfe"/>{/each}
    <line x1={k.linie[0][0]} y1={k.linie[0][1]} x2={k.linie[1][0]} y2={k.linie[1][1]} class="masslinie"/>
    {#each k.linie as p}<line x1={p[0]-4/zoom} y1={p[1]+4/zoom} x2={p[0]+4/zoom} y2={p[1]-4/zoom} class="masslinie" transform={`rotate(${k.winkel} ${p[0]} ${p[1]})`}/>{/each}
    <text class="masstext" x={k.mitte[0]} y={k.mitte[1]-3/zoom} font-size={11/zoom} text-anchor="middle" transform={`rotate(${k.winkel} ${k.mitte[0]} ${k.mitte[1]})`}>{faktor===null?"— m":`${z(k.laenge*faktor)} m`}</text>
  {:else}
    <text class="freitext" x={e.punkt[0]} y={e.punkt[1]} font-size={e.hoehe} transform={e.drehung_grad?`rotate(${-e.drehung_grad} ${e.punkt[0]} ${e.punkt[1]})`:undefined}>{e.inhalt}</text>
  {/if}
{/snippet}

<style>
  .pdf-plan{display:flex;flex-direction:column;height:100%;min-height:0}
  .plan-kopf{display:flex;flex-wrap:wrap;gap:2px;align-items:center;padding:2px 6px;background:var(--leiste);border-bottom:1px solid var(--border-innen)}
  .plan-kopf .zoom{min-width:3.2rem;text-align:center;font-size:.85rem;font-variant-numeric:tabular-nums;color:var(--muted)}
  .plan-kopf .text{min-height:26px;padding:0 8px;font-size:.85rem}
  .plan-kopf .trenner{width:1px;height:18px;background:var(--border);margin:0 4px}
  .plan-kopf :global(svg){width:16px;height:16px;stroke:currentColor;fill:none;stroke-width:1.5;stroke-linecap:round;stroke-linejoin:round}
  .fangschalter{display:inline-flex;gap:5px;align-items:center;min-height:26px;padding:0 7px;border:1px solid transparent;font-size:.85rem;color:var(--muted);cursor:pointer}
  .fangschalter.aktiv{color:var(--accent);background:var(--accent-soft);border-color:var(--accent)}
  .fangschalter input{margin:0;width:14px;height:14px}
  .fangschalter :global(svg){display:none}
  .fangoptionen{display:flex;flex-wrap:wrap;gap:.7rem;padding:4px 8px;font-size:.85rem;background:var(--kopfzeile);border-bottom:1px solid var(--border-innen)}
  .fangoptionen label{display:flex;gap:.3rem;align-items:center}
  .vergleich{pointer-events:none;mix-blend-mode:multiply;transform-origin:0 0}
  .plan-koerper{display:flex;flex:1;min-height:0}
  .seitenleiste{width:150px;overflow:auto;background:#cfd8e0;padding:6px;display:grid;gap:6px;align-content:start}
  .seitenleiste button{display:grid;gap:2px;justify-items:center;padding:4px;background:white;border:2px solid transparent;border-radius:4px;cursor:pointer}
  .seitenleiste button.aktiv{border-color:#126d97}
  .seitenleiste img{max-width:100%;height:auto;display:block}
  .seitenleiste .platzhalter{height:60px;display:grid;place-items:center;color:#777}
  .plan-scroll{flex:1;min-height:0;overflow:auto;background:var(--fenster,#dce3e8);padding:8px;overscroll-behavior:contain}
  .plan-scroll.greifen{cursor:grab}
  .papier{position:relative;background:white;box-shadow:0 2px 8px #16324730}
  .blatt{position:absolute;left:0;top:0;transform-origin:0 0}
  .blatt>img,.blatt>svg{position:absolute;inset:0;width:100%;height:100%}
  svg{touch-action:none;outline-offset:2px}
  svg.zeichnen{cursor:crosshair}
  svg.greifen{cursor:grab}
  svg.zoomen{cursor:zoom-in}
  svg :is(polygon,rect,circle,line,polyline,path){vector-effect:non-scaling-stroke;stroke:#126d97;stroke-width:1.8;fill:#168bb12b}
  svg .aktiv{stroke:#ad530a;fill:#e394222b;stroke-width:2.5}
  svg .ecke{fill:white;stroke:#ad530a;cursor:crosshair}
  svg .vorschau,svg .fang{fill:none;stroke:#ad530a;pointer-events:none}
  svg .vorschau{stroke-dasharray:5 4}
  svg text{fill:#174864;paint-order:stroke;stroke:white;stroke-width:3px;stroke-linejoin:round;pointer-events:none}
  svg .livemass{fill:#ad530a;font-weight:600}
  svg .hoehenhilfe{stroke:#1a7f37;stroke-dasharray:3 5;fill:none;pointer-events:none;opacity:.8}
  svg .zoomrahmen{fill:#126d9714;stroke:#126d97;stroke-dasharray:4 3;pointer-events:none}
  .entwurf{opacity:.6;pointer-events:none}
  .rahmenentwurf :is(rect){fill:none;stroke:#5a3d8f;stroke-dasharray:6 4}
  svg .abdunkeln{fill:#1f2d3a;fill-opacity:.32;stroke:none;pointer-events:none}
  svg .rahmen rect{fill:#5a3d8f0f;stroke:#5a3d8f;stroke-dasharray:6 4;cursor:pointer}
  svg .rahmen.gewaehlt rect{stroke-width:2.5;fill:#5a3d8f1f}
  svg .rahmen text{fill:#5a3d8f;font-weight:600}
  svg .rahmenecke{fill:white;stroke:#5a3d8f;stroke-dasharray:none;cursor:nwse-resize}
  svg .bezugslinie line{stroke:#1a7f37;stroke-width:2;stroke-dasharray:10 4 2 4;pointer-events:none}
  svg .bezugslinie text{fill:#1a7f37}
  svg .referenz{pointer-events:none}
  svg .referenz line{stroke:#5a3d8f;stroke-dasharray:8 3;fill:none}
  svg .referenz circle{stroke:#5a3d8f;fill:white}
  svg .referenz text{fill:#5a3d8f}
  svg .fremd{opacity:.35;pointer-events:none}
  svg .rasterflaeche{stroke:none;pointer-events:none}
  svg .raster{stroke:#dde3e8;fill:none;stroke-width:1}
  svg .raster.grob{stroke:#bcc6cf}
  svg .hintergrund{pointer-events:none}
  svg .hintergrund path{fill:none;stroke-width:1}
  svg .element .wand{fill:#6b747d33;stroke:#2b3138;stroke-width:1.4}
  svg .element .wandachse{stroke:#2b3138;stroke-width:.8;stroke-dasharray:6 3;opacity:.5;fill:none}
  svg .element .masslinie,svg .element .masshilfe{stroke:#1d4e89;stroke-width:1;fill:none}
  svg .element .masshilfe{opacity:.7}
  svg .element .masstext{fill:#1d4e89;font-weight:600}
  svg .element .freitext{fill:#2b3138;stroke:none;paint-order:normal;font-family:system-ui,sans-serif}
  svg .element.gewaehlt :is(polygon,line){stroke:#ad530a;stroke-width:2.2}
  svg .element.gewaehlt text{fill:#ad530a}
  .entwurf .wand{fill:#6b747d33;stroke:#ad530a}
  svg .unterlage{pointer-events:none}
  svg .unterlage path{fill:none;stroke-width:1}
  svg .unterlage text{stroke:none;paint-order:normal;font-family:system-ui,sans-serif;font-weight:400}
  svg .fehler :is(path,polygon,rect,circle,line){stroke:#b02d25;stroke-dasharray:4 3}
  svg .gewaehlt :is(path,polygon,rect,circle,line){stroke:#ad530a;stroke-width:2.5}
  .masseingabe{position:absolute;z-index:2;margin:0}
  .masseingabe input{width:11rem;font-size:.85rem;padding:.2rem .4rem;border:1px solid #ad530a;border-radius:3px;background:white}
  .status{padding:3px 8px;margin:0;font-size:.8rem;min-height:1.6em;border-top:1px solid var(--border-innen);background:var(--leiste);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
</style>
