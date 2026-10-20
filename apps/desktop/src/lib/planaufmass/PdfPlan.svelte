<script lang="ts">
  import type { PDFDocumentProxy, PDFDocumentLoadingTask, RenderTask } from "pdfjs-dist";
  import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
  import type { Plangeometrie, Planmarkierung, Planebene } from "../generated/plattform";
  import { extractGeometry, snap, type Matrix, type Point, type SnapGeometry } from "./pdfGeometry";
  let { pdfBase64, seite, markierungen, ebenen, auswahl, werkzeug, darf,
    onfertig, onauswahl, onedit, onfehler }:
    {pdfBase64:string;seite:number;markierungen:Planmarkierung[];ebenen:Planebene[];
    auswahl:string;werkzeug:string;darf:boolean;
    onfertig:(g:Plangeometrie)=>void;onauswahl:(id:string)=>void;
    onedit:(id:string,g:Plangeometrie)=>void;onfehler:(s:string)=>void}=$props();
  let doc=$state<PDFDocumentProxy|null>(null), svg=$state<SVGSVGElement>(), seitenbild=$state("");
  let scroll=$state<HTMLDivElement>(), breite=$state(600), hoehe=$state(800), zoom=$state(1);
  let bereit=$state(false), fang=$state(true), geometry=$state<SnapGeometry>({points:[],segments:[],limited:false});
  let punkte=$state<Point[]>([]), cursor=$state<Point|null>(null), fangart=$state("");
  let drag=$state<{id:string;index:number;g:Plangeometrie}|null>(null);
  let pan: {x:number;y:number;left:number;top:number}|null=null;
  let edit=$state<Plangeometrie|null>(null);
  const angezeigt=$derived(markierungen.filter(m=>ebenen.find(e=>e.id===m.ebene_id)?.sichtbar));
  const gewaehlt=$derived(angezeigt.find(m=>m.id===auswahl));
  const gesperrt=$derived(!darf || !!ebenen.find(e=>e.id===gewaehlt?.ebene_id)?.gesperrt);
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
  $effect(()=>{
    const d=doc,page=seite; let gone=false,task:RenderTask|undefined,url="";
    bereit=false;geometry={points:[],segments:[],limited:false};punkte=[];edit=null;
    if(!d)return;
    void(async()=>{
      const p=await d.getPage(page);if(gone)return;
      const viewport=p.getViewport({scale:1});
      const scale=Math.min(3,2200/Math.max(viewport.width,viewport.height));
      const raster=p.getViewport({scale});
      const buffer=document.createElement("canvas");buffer.width=Math.ceil(raster.width);buffer.height=Math.ceil(raster.height);
      task=p.render({canvasContext:buffer.getContext("2d")!,viewport:raster});await task.promise;
      const {OPS}=await import("pdfjs-dist");const list=await p.getOperatorList();
      const blob=await new Promise<Blob>((resolve,reject)=>buffer.toBlob(b=>b?resolve(b):reject(Error("PDF-Seitenbild konnte nicht erstellt werden")),"image/png"));
      if(gone)return;
      url=URL.createObjectURL(blob);seitenbild=url;
      breite=viewport.width;hoehe=viewport.height;
      geometry=extractGeometry(list,OPS,viewport.transform as Matrix);
      zoom=Math.min(1,(scroll?.clientWidth??800)/breite);
      bereit=true;
    })().catch(e=>{if(!gone)onfehler(`PDF-Seite: ${String(e)}`)});
    return()=>{gone=true;task?.cancel();if(url)URL.revokeObjectURL(url)};
  });
  $effect(()=>{void werkzeug;punkte=[];cursor=null;drag=null;edit=null});
  function vertices(g:Plangeometrie):Point[] {
    if(g.art==="polygon")return g.punkte;
    if(g.art==="kreis")return[g.mitte,g.rand];
    if(g.art==="hoehenpunkt")return[g.punkt];
    return[g.von,g.bis];
  }
  function changed(g:Plangeometrie,index:number,p:Point):Plangeometrie {
    const copy=structuredClone(g);
    if(copy.art==="polygon")copy.punkte[index]=p;
    else if(copy.art==="kreis"){if(index===0)copy.mitte=p;else copy.rand=p}
    else if(copy.art==="hoehenpunkt")copy.punkt=p;
    else {if(index===0)copy.von=p;else copy.bis=p}
    return copy;
  }
  function position(e:PointerEvent):Point {
    const r=svg!.getBoundingClientRect();
    const p:Point=[Math.max(0,Math.min(breite,(e.clientX-r.left)/r.width*breite)),Math.max(0,Math.min(hoehe,(e.clientY-r.top)/r.height*hoehe))];
    const f=fang&&!e.altKey?snap(p,geometry,8/zoom):null;
    fangart=f?.kind??"";
    return f?.point??p;
  }
  function shape(ps:Point[]):Plangeometrie|null {
    if((werkzeug==="hoehenpunkt"||werkzeug==="bezug") && ps.length)return{art:"hoehenpunkt",punkt:ps[0]};
    if(werkzeug==="polygon")return ps.length>=3?{art:"polygon",punkte:ps}:null;
    if(ps.length<2)return null;
    if(werkzeug==="rechteck")return{art:"rechteck",von:ps[0],bis:ps[1]};
    if(werkzeug==="kreis")return{art:"kreis",mitte:ps[0],rand:ps[1]};
    return{art:"strecke",von:ps[0],bis:ps[1]};
  }
  function finish(){const g=shape(punkte);if(g){onfertig(g);punkte=[];cursor=null}}
  function down(e:PointerEvent) {
    if(!bereit)return;
    if(e.button===1 || werkzeug==="verschieben"){
      e.preventDefault();pan={x:e.clientX,y:e.clientY,left:scroll!.scrollLeft,top:scroll!.scrollTop};
      svg?.setPointerCapture(e.pointerId);return;
    }
    if(e.button!==0)return;
    const target=e.target as SVGElement;
    const id=target.closest("[data-markierung]")?.getAttribute("data-markierung");
    if(werkzeug==="auswahl"){
      if(id)onauswahl(id);
      const index=target.getAttribute("data-ecke");
      if(index!==null && gewaehlt && !gesperrt){
        drag={id:gewaehlt.id,index:Number(index),g:structuredClone($state.snapshot(gewaehlt.geometrie))};
        svg?.setPointerCapture(e.pointerId);
      }
      return;
    }
    if(!darf)return;
    const p=position(e);
    if(werkzeug==="polygon"&&punkte.length>=3&&Math.hypot(p[0]-punkte[0][0],p[1]-punkte[0][1])<8/zoom){finish();return}
    punkte=[...punkte,p];
    if(werkzeug!=="polygon" && (punkte.length===2||werkzeug==="hoehenpunkt"||werkzeug==="bezug"))finish();
  }
  function move(e:PointerEvent) {
    if(pan){scroll!.scrollLeft=pan.left-(e.clientX-pan.x);scroll!.scrollTop=pan.top-(e.clientY-pan.y);return}
    if(!bereit)return;
    cursor=position(e);
    if(drag)edit=changed(drag.g,drag.index,cursor);
  }
  function up(e:PointerEvent){
    if(drag&&edit)onedit(drag.id,edit);
    drag=null;edit=null;pan=null;
    if(svg?.hasPointerCapture(e.pointerId))svg.releasePointerCapture(e.pointerId);
  }
  function key(e:KeyboardEvent){
    if((e.target as Element).closest("input,select,textarea,button"))return;
    if(e.key==="Escape"){punkte=[];cursor=null;drag=null;edit=null}
    else if(e.key==="Enter"){e.preventDefault();finish()}
    else if(e.key==="Backspace"){e.preventDefault();punkte=punkte.slice(0,-1)}
  }
  const preview=$derived(punkte.length&&cursor?shape([...punkte,cursor]):null);
</script>

<div class="pdf-plan">
  <div class="plan-kopf">
    <button type="button" class="sekundaer" disabled={!bereit} onclick={()=>zoom=Math.max(.1,zoom/1.25)} aria-label="Verkleinern">−</button>
    <span>{Math.round(zoom*100)} %</span>
    <button type="button" class="sekundaer" disabled={!bereit} onclick={()=>zoom=Math.min(8,zoom*1.25)} aria-label="Vergrößern">+</button>
    <button type="button" class="sekundaer" disabled={!bereit} onclick={()=>zoom=(scroll?.clientWidth??800)/breite}>Einpassen</button>
    <label><input type="checkbox" bind:checked={fang}/> Punktfang</label>
    {#if punkte.length}<button type="button" onclick={finish} disabled={werkzeug==="polygon"&&punkte.length<3}>Abschließen</button><button type="button" class="sekundaer" onclick={()=>punkte=[]}>Abbrechen</button>{/if}
  </div>
  <div class="plan-scroll" bind:this={scroll}>
    {#if !bereit}<p role="status">PDF-Seite wird vorbereitet …</p>{/if}
    <div class="papier" style:width={`${breite*zoom}px`} style:height={`${hoehe*zoom}px`} style:visibility={bereit?"visible":"hidden"}>
      <img src={seitenbild||undefined} alt="" draggable="false"/>
      <!-- Zeicheneditor mit eigener Tastaturbedienung, keine dekorative SVG. -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
      <svg bind:this={svg} viewBox={`0 0 ${breite} ${hoehe}`} role="application" aria-label="PDF-Zeichenfläche. Punkte anklicken, Enter beendet, Escape bricht ab. Mittlere Maustaste verschiebt." tabindex="0"
        onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={()=>{drag=null;edit=null;pan=null}} onkeydown={key}>
        {#each angezeigt as m (m.id)}
          <g data-markierung={m.id} class:gewaehlt={m.id===auswahl}>
            {@render figur(edit&&drag?.id===m.id?edit:m.geometrie,m.id===auswahl)}
            <text x={vertices(m.geometrie)[0][0]+5/zoom} y={vertices(m.geometrie)[0][1]-5/zoom} font-size={12/zoom}>{m.bezeichnung}</text>
          </g>
        {/each}
        {#if preview}<g class="entwurf">{@render figur(preview,false)}</g>{/if}
        {#if punkte.length}<polyline points={[...punkte,...(cursor?[cursor]:[])].map(p=>p.join(",")).join(" ")} class="vorschau"/>{/if}
        {#each punkte as p}<circle cx={p[0]} cy={p[1]} r={4/zoom} class="ecke"/>{/each}
        {#if gewaehlt&&werkzeug==="auswahl"&&!gesperrt}
          <g data-markierung={gewaehlt.id}>
            {#each vertices(edit??gewaehlt.geometrie) as p,i}<circle data-ecke={i} cx={p[0]} cy={p[1]} r={5/zoom} class="ecke"/>{/each}
          </g>
        {/if}
        {#if cursor&&fangart}<path d={`M ${cursor[0]-7/zoom} ${cursor[1]} h ${14/zoom} M ${cursor[0]} ${cursor[1]-7/zoom} v ${14/zoom}`} class="fang"/>{/if}
      </svg>
    </div>
  </div>
  <p class="meta status" role="status">{fangart || (geometry.points.length?`${geometry.points.length} PDF-Vektorpunkte`:"Keine Vektorpunkte – manuell zeichnen")}
    {geometry.limited?" · Vektorumfang begrenzt":""} · Alt setzt den Fang vorübergehend aus. Enter schließt, Escape bricht ab.</p>
</div>

{#snippet figur(g:Plangeometrie,selected:boolean)}
  {#if g.art==="polygon"}<polygon points={g.punkte.map(p=>p.join(",")).join(" ")} class:aktiv={selected}/>
  {:else if g.art==="rechteck"}<rect x={Math.min(g.von[0],g.bis[0])} y={Math.min(g.von[1],g.bis[1])} width={Math.abs(g.von[0]-g.bis[0])} height={Math.abs(g.von[1]-g.bis[1])} class:aktiv={selected}/>
  {:else if g.art==="kreis"}<circle cx={g.mitte[0]} cy={g.mitte[1]} r={Math.hypot(g.rand[0]-g.mitte[0],g.rand[1]-g.mitte[1])} class:aktiv={selected}/>
  {:else if g.art==="strecke"}<line x1={g.von[0]} y1={g.von[1]} x2={g.bis[0]} y2={g.bis[1]} class:aktiv={selected}/>
  {:else}<circle cx={g.punkt[0]} cy={g.punkt[1]} r={4/zoom} class:aktiv={selected}/>{/if}
{/snippet}

<style>
  .plan-kopf{display:flex;flex-wrap:wrap;gap:.5rem;align-items:center;padding:.5rem}
  .plan-kopf label{display:flex;gap:.4rem;align-items:center;margin-left:auto}
  .plan-scroll{height:65vh;min-height:420px;overflow:auto;background:#dce3e8;padding:8px}
  .papier{position:relative;background:white;box-shadow:0 2px 8px #16324730}
  img,svg{position:absolute;inset:0;width:100%;height:100%}
  svg{touch-action:none;outline-offset:2px}
  svg :is(polygon,rect,circle,line,polyline,path){vector-effect:non-scaling-stroke;stroke:#126d97;stroke-width:1.8;fill:#168bb12b}
  svg .aktiv{stroke:#ad530a;fill:#e394222b;stroke-width:2.5}
  svg .ecke{fill:white;stroke:#ad530a;cursor:crosshair}
  svg .vorschau,svg .fang{fill:none;stroke:#ad530a;pointer-events:none}
  svg .vorschau{stroke-dasharray:5 4}
  svg text{fill:#174864;paint-order:stroke;stroke:white;stroke-width:3px;stroke-linejoin:round;pointer-events:none}
  .entwurf{opacity:.6;pointer-events:none}
  .status{padding:.5rem;margin:0;min-height:2em}
</style>
