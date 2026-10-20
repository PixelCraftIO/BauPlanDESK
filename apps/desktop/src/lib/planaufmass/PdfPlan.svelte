<script lang="ts">
  import { tick } from "svelte";
  import type { PDFDocumentProxy, PDFDocumentLoadingTask, PDFPageProxy, RenderTask } from "pdfjs-dist";
  import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
  import type { Plangeometrie, Planmarkierung, Planebene, Planausschnitt, Planmesswert, Plankalibrierung } from "../generated/plattform";
  import { extractGeometry, snap, meterProPunkt, vorschauMass, verschieben, eckpunkte, rahmen as umgrenzung, type Matrix, type Point, type SnapGeometry } from "./pdfGeometry";
  // Zeichenfläche: Anzeige, Fang, Zoom und Eingabe. Fachwerte (Längen, Flächen,
  // Höhen) stammen vom Core; hier werden nur Vorschauen während des Zeichnens gezeigt.
  let { pdfBase64, seite, markierungen, ebenen, auswahl, werkzeug, darf,
    ausschnitte = [], aktiv = "", rahmenAuswahl = "", werte = [], kalibrierung = null, masse = true,
    onfertig, onauswahl, onedit, onfehler, onrahmen = () => {}, onrahmenauswahl = () => {}, onoeffnen = () => {} }:
    {pdfBase64:string;seite:number;markierungen:Planmarkierung[];ebenen:Planebene[];
    auswahl:string;werkzeug:string;darf:boolean;
    ausschnitte?:Planausschnitt[];aktiv?:string;rahmenAuswahl?:string;werte?:Planmesswert[];kalibrierung?:Plankalibrierung|null;masse?:boolean;
    onfertig:(g:Plangeometrie)=>void;onauswahl:(id:string)=>void;
    onedit:(id:string,g:Plangeometrie)=>void;onfehler:(s:string)=>void;
    onrahmen?:(id:string,von:Point,bis:Point)=>void;onrahmenauswahl?:(id:string)=>void;onoeffnen?:(id:string)=>void}=$props();
  const PAD=8;
  let doc=$state<PDFDocumentProxy|null>(null), page:PDFPageProxy|null=null, svg=$state<SVGSVGElement>(), seitenbild=$state("");
  let scroll=$state<HTMLDivElement>(), breite=$state(600), hoehe=$state(800), zoom=$state(1);
  let bereit=$state(false), fang=$state(true), geometry=$state<SnapGeometry>({points:[],segments:[],limited:false});
  let punkte=$state<Point[]>([]), cursor=$state<Point|null>(null), fangart=$state("");
  let drag=$state<{id:string;index:number;g:Plangeometrie}|null>(null);
  let schieben=$state<{id:string;start:Point;g:Plangeometrie;bewegt:boolean}|null>(null);
  let rahmenDrag=$state<{id:string;ecke:number;von:Point;bis:Point;start:Point;bewegt:boolean}|null>(null);
  let pan: {x:number;y:number;left:number;top:number}|null=null;
  let leertaste=$state(false);
  let edit=$state<Plangeometrie|null>(null);
  let rasterScale=0, rasterTask:RenderTask|undefined, rasterTimer:ReturnType<typeof setTimeout>|undefined;
  const zeiger=new Map<number,{x:number;y:number}>();
  let pinch:{abstand:number;mitte:{x:number;y:number}}|null=null;
  const angezeigt=$derived(markierungen.filter(m=>ebenen.find(e=>e.id===m.ebene_id)?.sichtbar));
  const gewaehlt=$derived(angezeigt.find(m=>m.id===auswahl));
  const gesperrt=$derived(!darf || !!ebenen.find(e=>e.id===gewaehlt?.ebene_id)?.gesperrt);
  const ausschnitt=$derived(ausschnitte.find(a=>a.id===aktiv));
  // Markierungen anderer Ausschnitte bleiben sichtbar, aber nicht bedienbar.
  const fremd=(m:Planmarkierung)=>!!aktiv&&(m.ausschnitt_id??"")!==aktiv;
  const faktor=$derived(meterProPunkt(ausschnitt?.kalibrierung??kalibrierung));
  const z=(n:number,stellen=2)=>n.toLocaleString("de-DE",{maximumFractionDigits:stellen});
  const rect=(a:{von:Point;bis:Point})=>({x:Math.min(a.von[0],a.bis[0]),y:Math.min(a.von[1],a.bis[1]),w:Math.abs(a.bis[0]-a.von[0]),h:Math.abs(a.bis[1]-a.von[1])});
  $effect(()=>{
    const source=pdfBase64; let gone=false; let loader:PDFDocumentLoadingTask|undefined;
    doc=null;bereit=false;punkte=[];
    void(async()=>{
      const {getDocument,GlobalWorkerOptions}=await import("pdfjs-dist");
      if(gone)return;
      GlobalWorkerOptions.workerSrc=workerUrl;
      loader=getDocument({data:Uint8Array.from(atob(source),c=>c.charCodeAt(0)),isEvalSupported:false,
        cMapUrl:new URL("./pdfjs/cmaps/",document.baseURI).href,cMapPacked:true,
        standardFontDataUrl:new URL("./pdfjs/standard_fonts/",document.baseURI).href});
      const d=await loader.promise;if(!gone)doc=d;
    })().catch(e=>{if(!gone)onfehler(`PDF: ${String(e)}`)});
    return()=>{gone=true;void loader?.destroy()};
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
    const d=doc,nummer=seite; let gone=false,url="";
    bereit=false;geometry={points:[],segments:[],limited:false};punkte=[];edit=null;page=null;rasterScale=0;
    if(!d)return;
    void(async()=>{
      const p=await d.getPage(nummer);if(gone)return;
      const viewport=p.getViewport({scale:1});
      const scale=Math.min(3,2200/Math.max(viewport.width,viewport.height));
      url=await rastern(p,scale);
      const {OPS}=await import("pdfjs-dist");const list=await p.getOperatorList();
      if(gone)return;
      seitenbild=url;rasterScale=scale;page=p;
      breite=viewport.width;hoehe=viewport.height;
      geometry=extractGeometry(list,OPS,viewport.transform as Matrix);
      // Beim Öffnen Breite einpassen, höchstens 100 %; „Einpassen“ zeigt das ganze Blatt.
      zoom=Math.min(1,((scroll?.clientWidth??800)-2*PAD)/breite);
      bereit=true;
    })().catch(e=>{if(!gone&&!String(e).includes("cancelled"))onfehler(`PDF-Seite: ${String(e)}`)});
    return()=>{gone=true;rasterTask?.cancel();if(url)URL.revokeObjectURL(url)};
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
      void rastern(p,noetig).then(u=>{if(page!==p)return;seitenbild=u;rasterScale=noetig;if(alt)URL.revokeObjectURL(alt)}).catch(()=>{});
    },200);
    return()=>clearTimeout(rasterTimer);
  });
  $effect(()=>{void werkzeug;punkte=[];cursor=null;drag=null;edit=null;schieben=null;rahmenDrag=null});
  // Ausschnitt öffnen: Rahmen einpassen.
  $effect(()=>{
    const a=ausschnitt; void bereit;
    if(!a||!bereit)return;
    void aufBereich(rect(a));
  });
  function passend(){
    const w=(scroll?.clientWidth??800)-2*PAD, h=(scroll?.clientHeight??600)-2*PAD;
    return Math.max(.05,Math.min(w/breite,h/hoehe));
  }
  async function aufBereich(r:{x:number;y:number;w:number;h:number}){
    if(!scroll)return;
    const w=scroll.clientWidth-2*PAD, h=scroll.clientHeight-2*PAD;
    zoom=Math.max(.05,Math.min(12,Math.min(w/(r.w+40),h/(r.h+40))));
    await tick();
    scroll.scrollLeft=(r.x+r.w/2)*zoom+PAD-scroll.clientWidth/2;
    scroll.scrollTop=(r.y+r.h/2)*zoom+PAD-scroll.clientHeight/2;
  }
  export function einpassen(){
    if(ausschnitt)return void aufBereich(rect(ausschnitt));
    zoom=passend();
  }
  export function aufAuswahl(){
    if(gewaehlt)void aufBereich(umgrenzung(eckpunkte(gewaehlt.geometrie)));
  }
  async function zoomAn(clientX:number,clientY:number,faktorZoom:number){
    if(!scroll)return;
    const r=scroll.getBoundingClientRect();
    const px=(scroll.scrollLeft+(clientX-r.left)-PAD)/zoom, py=(scroll.scrollTop+(clientY-r.top)-PAD)/zoom;
    const neu=Math.max(.05,Math.min(12,zoom*faktorZoom));
    if(neu===zoom)return;
    zoom=neu;
    await tick();
    scroll.scrollLeft=px*neu+PAD-(clientX-r.left);
    scroll.scrollTop=py*neu+PAD-(clientY-r.top);
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
    const copy=structuredClone(g);
    if(copy.art==="polygon")copy.punkte[index]=p;
    else if(copy.art==="kreis"){if(index===0)copy.mitte=p;else copy.rand=p}
    else if(copy.art==="hoehenpunkt")copy.punkt=p;
    else {if(index===0)copy.von=p;else copy.bis=p}
    return copy;
  }
  function roh(e:PointerEvent):Point{
    const r=svg!.getBoundingClientRect();
    return [Math.max(0,Math.min(breite,(e.clientX-r.left)/r.width*breite)),Math.max(0,Math.min(hoehe,(e.clientY-r.top)/r.height*hoehe))];
  }
  function position(e:PointerEvent):Point {
    let p=roh(e);
    // Shift rastet auf Waagerechte/Senkrechte/45° zum letzten Punkt.
    if(e.shiftKey&&punkte.length){
      const a=punkte[punkte.length-1], dx=p[0]-a[0], dy=p[1]-a[1];
      const winkel=Math.round(Math.atan2(dy,dx)/(Math.PI/4))*(Math.PI/4), l=Math.hypot(dx,dy);
      p=[a[0]+Math.cos(winkel)*l,a[1]+Math.sin(winkel)*l];
    }
    const f=fang&&!e.altKey?snap(p,{points:[...geometry.points,...eigenePunkte()],segments:geometry.segments,limited:geometry.limited},8/zoom):null;
    fangart=f?.kind??"";
    return f?.point??p;
  }
  // Fang auch an eigenen Markierungen und Bezugslinien.
  function eigenePunkte():Point[]{
    const ps:Point[]=[];
    for(const m of angezeigt)if(m.id!==auswahl)ps.push(...eckpunkte(m.geometrie));
    for(const a of ausschnitte)if(a.bezugslinie)ps.push(a.bezugslinie.von,a.bezugslinie.bis);
    return ps;
  }
  function shape(ps:Point[]):Plangeometrie|null {
    if((werkzeug==="hoehenpunkt"||werkzeug==="bezug") && ps.length)return{art:"hoehenpunkt",punkt:ps[0]};
    if(werkzeug==="polygon")return ps.length>=3?{art:"polygon",punkte:ps}:null;
    if(ps.length<2)return null;
    if(werkzeug==="rechteck"||werkzeug==="ausschnitt")return{art:"rechteck",von:ps[0],bis:ps[1]};
    if(werkzeug==="kreis")return{art:"kreis",mitte:ps[0],rand:ps[1]};
    return{art:"strecke",von:ps[0],bis:ps[1]};
  }
  function finish(){const g=shape(punkte);if(g){onfertig(g);punkte=[];cursor=null}}
  function down(e:PointerEvent) {
    if(!bereit)return;
    zeiger.set(e.pointerId,{x:e.clientX,y:e.clientY});
    if(zeiger.size===2){
      const [a,b]=[...zeiger.values()];
      pinch={abstand:Math.hypot(a.x-b.x,a.y-b.y),mitte:{x:(a.x+b.x)/2,y:(a.y+b.y)/2}};
      punkte=[];drag=null;schieben=null;rahmenDrag=null;pan=null;
      svg?.setPointerCapture(e.pointerId);return;
    }
    if(e.button===1 || werkzeug==="verschieben" || leertaste){
      e.preventDefault();pan={x:e.clientX,y:e.clientY,left:scroll!.scrollLeft,top:scroll!.scrollTop};
      svg?.setPointerCapture(e.pointerId);return;
    }
    if(e.button!==0)return;
    const target=e.target as SVGElement;
    const id=target.closest("[data-markierung]")?.getAttribute("data-markierung");
    const rahmenId=target.closest("[data-rahmen]")?.getAttribute("data-rahmen");
    if(werkzeug==="auswahl"){
      const index=target.getAttribute("data-ecke");
      if(id){
        const m=angezeigt.find(m=>m.id===id);
        if(m&&fremd(m))return;
        onauswahl(id);
        if(index!==null && gewaehlt && gewaehlt.id===id && !gesperrt){
          drag={id:gewaehlt.id,index:Number(index),g:structuredClone($state.snapshot(gewaehlt.geometrie))};
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
      onauswahl("");onrahmenauswahl("");
      return;
    }
    if(!darf)return;
    const p=position(e);
    if(werkzeug==="polygon"&&punkte.length>=3&&Math.hypot(p[0]-punkte[0][0],p[1]-punkte[0][1])<8/zoom){finish();return}
    punkte=[...punkte,p];
    if(werkzeug!=="polygon" && (punkte.length===2||werkzeug==="hoehenpunkt"||werkzeug==="bezug"))finish();
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
      onrahmen(rahmenDrag.id,von,bis);return;
    }
    cursor=position(e);
    if(drag)edit=changed(drag.g,drag.index,cursor);
    if(schieben){
      const p=roh(e), dx=p[0]-schieben.start[0], dy=p[1]-schieben.start[1];
      if(Math.hypot(dx,dy)*zoom>3)schieben.bewegt=true;
      if(schieben.bewegt)edit=verschieben(schieben.g,dx,dy);
    }
  }
  function up(e:PointerEvent){
    zeiger.delete(e.pointerId);
    if(zeiger.size<2)pinch=null;
    if(drag&&edit)onedit(drag.id,edit);
    if(schieben?.bewegt&&edit)onedit(schieben.id,edit);
    drag=null;edit=null;pan=null;schieben=null;rahmenDrag=null;
    if(svg?.hasPointerCapture(e.pointerId))svg.releasePointerCapture(e.pointerId);
  }
  function key(e:KeyboardEvent){
    if((e.target as Element).closest("input,select,textarea,button"))return;
    if(e.key==="Escape"){
      if(punkte.length||drag||schieben){e.stopPropagation();punkte=[];cursor=null;drag=null;edit=null;schieben=null}
    }
    else if(e.key==="Enter"){e.preventDefault();finish()}
    else if(e.key==="Backspace"){e.preventDefault();punkte=punkte.slice(0,-1)}
  }
  const preview=$derived(punkte.length&&cursor?shape([...punkte,cursor]):null);
  const vorschau=$derived.by(()=>{
    if(!preview||werkzeug==="ausschnitt")return "";
    const m=vorschauMass(preview,faktor);
    if(preview.art==="strecke"&&cursor&&punkte.length){
      const dy=Math.abs(cursor[1]-punkte[0][1]), dx=Math.abs(cursor[0]-punkte[0][0]);
      return faktor===null?"Maßstab fehlt":`${z(m.laenge_m!)} m · Δh ${z(dy*faktor)} m · ⟷ ${z(dx*faktor)} m`;
    }
    if(m.flaeche_m2!==null)return `${z(m.flaeche_m2)} m² · Umfang ${z(m.laenge_m!)} m`;
    if(m.laenge_m!==null)return `${z(m.laenge_m)} m`;
    return faktor===null?"Maßstab fehlt":"";
  });
  function beschriftung(m:Planmarkierung){
    if(!masse)return "";
    const w=werte.find(w=>w.id===m.id); if(!w)return "";
    if(m.geometrie.art==="hoehenpunkt")return w.hoehe_m!==null?`${z(w.hoehe_m)} m`:"";
    if(m.geometrie.art==="strecke")return w.laenge_m!==null?`${z(w.laenge_m)} m${w.hoehe_m?` · Δh ${z(w.hoehe_m)} m`:""}`:"";
    return w.flaeche_m2!==null?`${z(w.flaeche_m2)} m²`:"";
  }
  const abdunkeln=$derived.by(()=>{
    if(!ausschnitt)return "";
    const r=rect(ausschnitt);
    return `M0 0H${breite}V${hoehe}H0Z M${r.x} ${r.y}H${r.x+r.w}V${r.y+r.h}H${r.x}Z`;
  });
  const cursorKlasse=$derived(leertaste||pan||werkzeug==="verschieben"?"greifen":werkzeug==="auswahl"?"":"zeichnen");
</script>

<div class="pdf-plan">
  <div class="plan-kopf">
    <button type="button" class="sekundaer" disabled={!bereit} onclick={()=>zoomMitte(1/1.25)} aria-label="Verkleinern">−</button>
    <span class="zoom">{Math.round(zoom*100)} %</span>
    <button type="button" class="sekundaer" disabled={!bereit} onclick={()=>zoomMitte(1.25)} aria-label="Vergrößern">+</button>
    <button type="button" class="sekundaer" disabled={!bereit} onclick={einpassen}>Einpassen</button>
    <button type="button" class="sekundaer" disabled={!bereit} onclick={()=>{zoom=1}} title="72 Punkte je Zoll">1:1</button>
    <button type="button" class="sekundaer" disabled={!bereit||!gewaehlt} onclick={aufAuswahl}>Auf Auswahl</button>
    <label><input type="checkbox" bind:checked={fang}/> Punktfang</label>
    {#if punkte.length}<button type="button" onclick={finish} disabled={(werkzeug==="polygon"&&punkte.length<3)}>Abschließen</button><button type="button" class="sekundaer" onclick={()=>punkte=[]}>Abbrechen</button>{/if}
  </div>
  <div class="plan-scroll" bind:this={scroll} class:greifen={cursorKlasse==="greifen"}>
    {#if !bereit}<p role="status">PDF-Seite wird vorbereitet …</p>{/if}
    <div class="papier" style:width={`${breite*zoom}px`} style:height={`${hoehe*zoom}px`} style:visibility={bereit?"visible":"hidden"}>
      <img src={seitenbild||undefined} alt="" draggable="false"/>
      <!-- Zeicheneditor mit eigener Tastaturbedienung, keine dekorative SVG. -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
      <svg bind:this={svg} viewBox={`0 0 ${breite} ${hoehe}`} role="application" class={cursorKlasse}
        aria-label="PDF-Zeichenfläche. Punkte anklicken, Enter beendet, Escape bricht ab. Strg+Mausrad zoomt, Leertaste oder mittlere Maustaste verschiebt." tabindex="0"
        onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up} onkeydown={key}>
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
          <g data-markierung={m.id} class:gewaehlt={m.id===auswahl} class:fremd={fremd(m)}>
            {@render figur(edit&&(drag?.id===m.id||schieben?.id===m.id)?edit:m.geometrie,m.id===auswahl)}
            <text x={eckpunkte(m.geometrie)[0][0]+5/zoom} y={eckpunkte(m.geometrie)[0][1]-5/zoom} font-size={12/zoom}>{m.bezeichnung}{beschriftung(m)?` · ${beschriftung(m)}`:""}</text>
          </g>
        {/each}
        {#if preview}<g class="entwurf" class:rahmenentwurf={werkzeug==="ausschnitt"}>{@render figur(preview,false)}</g>{/if}
        {#if punkte.length}<polyline points={[...punkte,...(cursor?[cursor]:[])].map(p=>p.join(",")).join(" ")} class="vorschau"/>{/if}
        {#each punkte as p}<circle cx={p[0]} cy={p[1]} r={4/zoom} class="ecke"/>{/each}
        {#if gewaehlt&&werkzeug==="auswahl"&&!gesperrt&&!fremd(gewaehlt)}
          <g data-markierung={gewaehlt.id}>
            {#each eckpunkte(edit??gewaehlt.geometrie) as p,i}<circle data-ecke={i} cx={p[0]} cy={p[1]} r={5/zoom} class="ecke"/>{/each}
          </g>
        {/if}
        {#if cursor&&fangart}<path d={`M ${cursor[0]-7/zoom} ${cursor[1]} h ${14/zoom} M ${cursor[0]} ${cursor[1]-7/zoom} v ${14/zoom}`} class="fang"/>{/if}
        {#if cursor&&vorschau}<text class="livemass" x={cursor[0]+12/zoom} y={cursor[1]+18/zoom} font-size={12/zoom}>{vorschau}</text>{/if}
      </svg>
    </div>
  </div>
  <p class="meta status" role="status">{vorschau?`Vorschau: ${vorschau} · `:""}{fangart || (geometry.points.length?`${geometry.points.length} PDF-Vektorpunkte`:"Keine Vektorpunkte – manuell zeichnen")}
    {geometry.limited?" · Vektorumfang begrenzt":""} · Strg+Rad zoomt, Leertaste verschiebt, Shift rastet, Alt setzt den Fang aus. Enter schließt, Escape bricht ab.</p>
</div>

{#snippet figur(g:Plangeometrie,selected:boolean)}
  {#if g.art==="polygon"}<polygon points={g.punkte.map(p=>p.join(",")).join(" ")} class:aktiv={selected}/>
  {:else if g.art==="rechteck"}<rect x={Math.min(g.von[0],g.bis[0])} y={Math.min(g.von[1],g.bis[1])} width={Math.abs(g.von[0]-g.bis[0])} height={Math.abs(g.von[1]-g.bis[1])} class:aktiv={selected}/>
  {:else if g.art==="kreis"}<circle cx={g.mitte[0]} cy={g.mitte[1]} r={Math.hypot(g.rand[0]-g.mitte[0],g.rand[1]-g.mitte[1])} class:aktiv={selected}/>
  {:else if g.art==="strecke"}<line x1={g.von[0]} y1={g.von[1]} x2={g.bis[0]} y2={g.bis[1]} class:aktiv={selected}/>
  {:else}<circle cx={g.punkt[0]} cy={g.punkt[1]} r={4/zoom} class:aktiv={selected}/>{/if}
{/snippet}

<style>
  .pdf-plan{display:flex;flex-direction:column;height:100%;min-height:0}
  .plan-kopf{display:flex;flex-wrap:wrap;gap:.5rem;align-items:center;padding:.4rem .5rem}
  .plan-kopf .zoom{min-width:3.5rem;text-align:center;font-variant-numeric:tabular-nums}
  .plan-kopf label{display:flex;gap:.4rem;align-items:center;margin-left:auto}
  .plan-scroll{flex:1;min-height:320px;overflow:auto;background:#dce3e8;padding:8px;overscroll-behavior:contain}
  .plan-scroll.greifen{cursor:grab}
  .papier{position:relative;background:white;box-shadow:0 2px 8px #16324730}
  img,svg{position:absolute;inset:0;width:100%;height:100%}
  svg{touch-action:none;outline-offset:2px}
  svg.zeichnen{cursor:crosshair}
  svg.greifen{cursor:grab}
  svg :is(polygon,rect,circle,line,polyline,path){vector-effect:non-scaling-stroke;stroke:#126d97;stroke-width:1.8;fill:#168bb12b}
  svg .aktiv{stroke:#ad530a;fill:#e394222b;stroke-width:2.5}
  svg .ecke{fill:white;stroke:#ad530a;cursor:crosshair}
  svg .vorschau,svg .fang{fill:none;stroke:#ad530a;pointer-events:none}
  svg .vorschau{stroke-dasharray:5 4}
  svg text{fill:#174864;paint-order:stroke;stroke:white;stroke-width:3px;stroke-linejoin:round;pointer-events:none}
  svg .livemass{fill:#ad530a;font-weight:600}
  .entwurf{opacity:.6;pointer-events:none}
  .rahmenentwurf :is(rect){fill:none;stroke:#5a3d8f;stroke-dasharray:6 4}
  svg .abdunkeln{fill:#1f2d3a;fill-opacity:.32;stroke:none;pointer-events:none}
  svg .rahmen rect{fill:#5a3d8f0f;stroke:#5a3d8f;stroke-dasharray:6 4;cursor:pointer}
  svg .rahmen.gewaehlt rect{stroke-width:2.5;fill:#5a3d8f1f}
  svg .rahmen text{fill:#5a3d8f;font-weight:600}
  svg .rahmenecke{fill:white;stroke:#5a3d8f;stroke-dasharray:none;cursor:nwse-resize}
  svg .bezugslinie line{stroke:#1a7f37;stroke-width:2;stroke-dasharray:10 4 2 4;pointer-events:none}
  svg .bezugslinie text{fill:#1a7f37}
  svg .fremd{opacity:.35;pointer-events:none}
  .status{padding:.4rem .5rem;margin:0;min-height:2em}
</style>
