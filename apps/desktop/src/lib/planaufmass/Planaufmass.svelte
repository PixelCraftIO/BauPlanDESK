<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { api, fehlertext, type Project, type DocumentVersion, type PdfVorschauDaten, type GrundstueckKontext } from "../api";
  import type { Planaufmass, Planmarkierung, Planbedeutung, Planmesswert, Plangeometrie, Gebaeude } from "../generated/plattform";
  import { FachSitzung } from "../FachSitzung.svelte";
  import Fachliste from "../Fachliste.svelte";
  import { plattformApi, type Datensatz } from "../plattformApi";
  import { abonnieren } from "../projektEreignisse";
  import PdfPlan from "./PdfPlan.svelte";
  import { planApi } from "./api";
  let {projekt,benutzerId,darf}:{projekt:Project;benutzerId:string;darf:boolean}=$props();
  let dateien=$state<DocumentVersion[]>([]), pdf=$state<PdfVorschauDaten|null>(null), pdfFehler=$state(""), grund=$state<GrundstueckKontext[]>([]);
  let gebaeude=$state<Datensatz<Gebaeude>[]>([]),listenFehler=$state(""),laedt=$state(false);
  const s=untrack(()=>new FachSitzung<Planaufmass>(projekt.id,"planaufmass",benutzerId,()=>({
    bezeichnung:"Neues PDF-Aufmaß",dokument:{dokument_id:dateien[0]?.document_id??"00000000-0000-0000-0000-000000000000",version:dateien[0]?.version??1},
    seite:1,kalibrierung:null,hoehenbezug:null,ebenen:[{id:crypto.randomUUID(),bezeichnung:"Aufmaß",sichtbar:true,gesperrt:false}],markierungen:[],
  }),p=>p.bezeichnung));
  const bedeutungen: [Planbedeutung,string][]=[
    ["unbestimmt","Noch nicht zugeordnet"],["flurstueck","Flurstück"],["private_gruenflaeche","Private Grünfläche"],
    ["versiegelung","Versiegelte Fläche"],["geschoss_aussenkante","Geschoss – Außenkante"],["gebaeudegrundflaeche","Gebäudegrundfläche – horizontale Projektion"],["raum","Raum"],["aufenthaltsraum","Aufenthaltsraum"],
    ["nuf","DIN 277 – Nutzungsfläche (NUF)"],["tf","DIN 277 – Technikfläche (TF)"],["vf","DIN 277 – Verkehrsfläche (VF)"],
    ["kgf","DIN 277 – Konstruktionsfläche (KGF)"],["aussenwand","Außenwand"],["gelaende","Gelände"],["traufe","Traufe"],["first","First"],["dach","Dach"],
  ];
  const werkzeuge=[["auswahl","Auswählen"],["verschieben","Verschieben"],["polygon","Polygon"],["rechteck","Rechteck"],["kreis","Kreis"],["strecke","Strecke / Dach"],["hoehenpunkt","Höhenpunkt"]];
  let werkzeug=$state("auswahl"),auswahl=$state(""),ebene=$state(""),nenner=$state(100),referenz=$state(10),hwert=$state(0),hsystem=$state("m ü. NHN");
  let werte=$state<Planmesswert[]>([]),messFehler=$state(""),messLaeuft=$state(false),messStand=$state("");
  let undo=$state<Planmarkierung[][]>([]),redo=$state<Planmarkierung[][]>([]);
  const m=$derived(s.eingabe?.markierungen.find(m=>m.id===auswahl));
  const layer=$derived(s.eingabe?.ebenen.find(e=>e.id===ebene));
  const objektGesperrt=$derived(!darf||s.laeuft||!!s.eingabe?.ebenen.find(e=>e.id===m?.ebene_id)?.gesperrt);
  const quelleAlt=$derived(s.eingabe&&dateien.some(d=>d.document_id===s.eingabe!.dokument.dokument_id&&d.version>s.eingabe!.dokument.version));
  const signatur=$derived(JSON.stringify(s.eingabe));
  const aktuelleWerte=$derived(signatur===messStand?werte:[]);
  const wert=$derived(aktuelleWerte.find(w=>w.id===auswahl));
  let listenGeneration=0;
  async function listen(){
    const n=++listenGeneration;
    const [d,g,b]=await Promise.allSettled([api.dokumente(s.projektId),api.grundstueckskontext(s.projektId),plattformApi.liste<Gebaeude>(s.projektId,"gebaeude")]);
    if(n!==listenGeneration)return;
    if(d.status==="fulfilled")dateien=d.value.filter(d=>d.file_name.toLowerCase().endsWith(".pdf")).sort((a,b)=>a.file_name.localeCompare(b.file_name,"de")||b.version-a.version);
    grund=g.status==="fulfilled"?g.value:[];
    gebaeude=b.status==="fulfilled"?b.value:[];
    listenFehler=[d,g,b].flatMap(r=>r.status==="rejected"?[fehlertext(r.reason)]:[]).join(" · ");
  }
  onMount(()=>{void listen();return()=>{listenGeneration++}});
  $effect(()=>abonnieren(s.projektId,["dokumente","fachobjekte"],()=>void listen()));
  $effect(()=>{
    void projekt.version;untrack(()=>void listen());
  });
  $effect(()=>{
    const id=s.id;
    untrack(()=>{auswahl="";ebene=s.eingabe?.ebenen[0]?.id??"";undo=[];redo=[];werkzeug="auswahl";void id});
  });
  $effect(()=>{
    const d=s.eingabe?.dokument;
    let gone=false;pdf=null;pdfFehler="";
    if(!d)return;
    laedt=true;
    void api.pdfVorschau(d.dokument_id,d.version).then(p=>{if(!gone)pdf=p}).catch(e=>{if(!gone)pdfFehler=fehlertext(e)}).finally(()=>{if(!gone)laedt=false});
    return()=>{gone=true};
  });
  $effect(()=>{
    const sig=signatur;
    let gone=false;messFehler="";messLaeuft=!!s.eingabe;
    const timer=setTimeout(()=>{
      if(!s.eingabe)return;
      const plan=JSON.parse(sig) as Planaufmass;
      void planApi.pruefen(s.projektId,plan).then(w=>{if(!gone){werte=w;messStand=sig}})
        .catch(e=>{if(!gone){werte=[];messStand="";messFehler=fehlertext(e)}}).finally(()=>{if(!gone)messLaeuft=false});
    },250);
    return()=>{gone=true;clearTimeout(timer)};
  });
  // Wohin eine gespeicherte Markierung übernommen wird; nur Bedienhinweis.
  function ziel(b:Planbedeutung){
    if(b==="gebaeudegrundflaeche")return "Grundlagen → Gebäude / Geschosse: am Feld „Gebäudegrundfläche“ über „Aus PDF-Aufmaß“ übernehmen. Von dort geht sie in GRZ des Baugrundstücks ein.";
    if(b==="geschoss_aussenkante")return "Grundlagen → Gebäude / Geschosse: im jeweiligen Geschoss über „Aus PDF-Aufmaß“ übernehmen. Von dort geht sie in GFZ, Geschossigkeit und Vollgeschossprüfung ein.";
    if(b==="raum"||b==="aufenthaltsraum"||b==="nuf"||b==="tf"||b==="vf")return "Berechnungen → Flächen- und Raumbuch: „Räume aus PDF-Aufmaß übernehmen“ (Gebäude und Geschoss zuordnen). DIN 277 und Wohnfläche können außerdem unter „Quellen / Übernahmen“ verknüpft werden.";
    if(b==="gelaende"||b==="traufe"||b==="first"||b==="dach"||b==="aussenwand")return "Je nach Ziel: Abstandsflächen → Grundlagen → „PDF-Aufmaß verknüpfen“ (manuelle Wand), Gebäudehöhen → „Quellen / Übernahmen“ oder Gebäude / Geschosse → „Höhen und alle Verknüpfungen aus PDF-Aufmaß“.";
    if(b==="flurstueck")return "Eine direkte Übernahme in Flurstücke und Baugrundstücksermittlung gibt es noch nicht; die Fläche dient vorerst zur Kontrolle der Katasterangaben.";
    if(b==="unbestimmt")return "Zuerst eine Bedeutung wählen; sie bestimmt, welche Berechnung den Messwert übernehmen kann.";
    return "In der Fachberechnung eine Position anlegen und unter „Quellen / Übernahmen → PDF-Aufmaß“ verknüpfen.";
  }
  function duplizieren(){
    if(!s.eingabe||!m)return;
    merken();
    const kopie={...structuredClone($state.snapshot(m)),id:crypto.randomUUID(),bezeichnung:`${m.bezeichnung} (Kopie)`};
    s.eingabe.markierungen.push(kopie);auswahl=kopie.id;
  }
  function merken(){if(s.eingabe){undo=[...undo.slice(-49),structuredClone($state.snapshot(s.eingabe.markierungen))];redo=[]}}
  function rueckwaerts(){
    if(!s.eingabe||!undo.length)return;
    redo=[...redo,structuredClone($state.snapshot(s.eingabe.markierungen))];s.eingabe.markierungen=undo.at(-1)!;undo=undo.slice(0,-1);
  }
  function vorwaerts(){
    if(!s.eingabe||!redo.length)return;
    undo=[...undo,structuredClone($state.snapshot(s.eingabe.markierungen))];s.eingabe.markierungen=redo.at(-1)!;redo=redo.slice(0,-1);
  }
  function fertig(g:Plangeometrie){
    const p=s.eingabe;if(!p||!darf||s.laeuft)return;
    if(werkzeug==="kalibrieren"&&g.art==="strecke"){
      if(!(referenz>0)){s.fehler="Referenzlänge in Metern angeben.";return}
      p.kalibrierung={art:"strecke",von:g.von,bis:g.bis,meter:referenz};werkzeug="auswahl";return;
    }
    if(werkzeug==="bezug"&&g.art==="hoehenpunkt"){
      p.hoehenbezug={punkt:g.punkt,hoehe_m:hwert,system:hsystem};werkzeug="auswahl";return;
    }
    if(!layer||layer.gesperrt||!layer.sichtbar){s.fehler="Eine sichtbare, ungesperrte Zeichenebene auswählen.";return}
    merken();
    const id=crypto.randomUUID();
    p.markierungen.push({id,ebene_id:layer.id,bezeichnung:`Markierung ${p.markierungen.length+1}`,bedeutung:"unbestimmt",geometrie:g,
      grundstueck_id:grund.length===1?grund[0].id:null,gebaeude_id:null,geschoss_id:null});
    auswahl=id;werkzeug="auswahl";
  }
  function source(value:string){
    if(!s.eingabe)return;
    const d=dateien.find(d=>`${d.document_id}|${d.version}`===value);
    if(d){s.eingabe.dokument={dokument_id:d.document_id,version:d.version};s.eingabe.seite=1;basisReset()}
  }
  function basisReset(){
    if(!s.eingabe)return;
    s.eingabe.kalibrierung=null;s.eingabe.hoehenbezug=null;
    s.meldung="Unterlage geändert. Maßstab und Höhenbezug neu bestimmen und alle Markierungen kontrollieren.";
  }
  const z=(n:number|null|undefined)=>n==null?"—":n.toLocaleString("de-DE",{maximumFractionDigits:3});
</script>

<Fachliste titel="PDF-Aufmaß" {s} bereich="planaufmass" {darf} zusatzBeschaeftigt={()=>laedt}>
  {#snippet children(p)}
    <p class="meta">PDF-Pläne unter „Projektdateien“ importieren. Je Blatt oder Planausschnitt ein Aufmaß mit eigenem Maßstab anlegen. Die Zeichnungen liegen auf separaten Ebenen; die Original-PDF bleibt erhalten.</p>
    {#if listenFehler}<p class="fehlertext" role="alert">{listenFehler}</p>{/if}
    {#if !dateien.length}<p class="hinweisbox">Noch keine PDF-Unterlage vorhanden. Zuerst eine PDF unter „Projektdateien“ importieren.</p>{/if}
    <div class="quellzeile">
      <label>Bezeichnung<input bind:value={p.bezeichnung} required/></label>
      <label>PDF-Unterlage<select value={`${p.dokument.dokument_id}|${p.dokument.version}`} onchange={e=>source(e.currentTarget.value)}>
        <option value="">Unterlage wählen</option>
        {#each dateien as d}<option value={`${d.document_id}|${d.version}`}>{d.file_name} · V{d.version}</option>{/each}
      </select></label>
      <label>Seite<input type="number" min="1" max={pdf?.seiten.length??1} step="1" bind:value={p.seite} onchange={basisReset} required/></label>
    </div>
    {#if quelleAlt}<p class="hinweisbox" role="status">Eine neuere PDF-Version liegt vor. Dieses Aufmaß gehört weiterhin zur älteren Unterlage; Berechnungsübernahmen sind bis zur Prüfung der neuen Version gesperrt.</p>{/if}
    <div class="kalibrierung">
      <label>Maßstab 1 :<input type="number" min=".001" step="any" bind:value={nenner}/></label>
      <button type="button" class="sekundaer" onclick={()=>p.kalibrierung={art:"massstab",nenner}}>Maßstab setzen</button>
      <label>Referenzstrecke (m)<input type="number" min=".001" step="any" bind:value={referenz}/></label>
      <button type="button" class:aktiv={werkzeug==="kalibrieren"} onclick={()=>werkzeug="kalibrieren"}>Zwei Punkte kalibrieren</button>
      <span class="meta">{!p.kalibrierung?"Maßstab fehlt":p.kalibrierung.art==="massstab"?`1 : ${z(p.kalibrierung.nenner)}`:`Kalibriert über ${z(p.kalibrierung.meter)} m`}</span>
    </div>
    <details><summary>Höhenbezug für Ansichten und Schnitte</summary>
      <p class="meta">Höhen werden senkrecht zur dargestellten Seite gemessen. Ansicht vorab lotrecht ausrichten. Ein Höhenpunkt benötigt einen bekannten Bezug; eine Strecke liefert eine Höhendifferenz.</p>
      <div class="kalibrierung"><label>Höhensystem<input bind:value={hsystem}/></label><label>Bezugshöhe (m)<input type="number" step="any" bind:value={hwert}/></label>
        <button type="button" onclick={()=>werkzeug="bezug"}>Bezugspunkt setzen</button>
        <span>{p.hoehenbezug?`${z(p.hoehenbezug.hoehe_m)} ${p.hoehenbezug.system}`:"Noch kein Höhenbezug"}</span>
      </div>
    </details>
    <div class="werkzeuge" role="toolbar" aria-label="PDF-Zeichenwerkzeuge">
      {#each werkzeuge as [id,titel]}<button type="button" class:aktiv={werkzeug===id} aria-pressed={werkzeug===id} onclick={()=>werkzeug=id}>{titel}</button>{/each}
      <button type="button" class="sekundaer" disabled={!undo.length} onclick={rueckwaerts}>Rückgängig</button>
      <button type="button" class="sekundaer" disabled={!redo.length} onclick={vorwaerts}>Wiederholen</button>
    </div>
    {#if werkzeug==="kalibrieren"||werkzeug==="bezug"}<p role="status" class="hinweisbox">{werkzeug==="kalibrieren"?"Anfang und Ende der bekannten Referenzstrecke anklicken.":"Den Punkt mit der angegebenen Bezugshöhe auf der PDF anklicken."}</p>{/if}
    {#if pdfFehler}<p class="fehlertext" role="alert">{pdfFehler}</p>{/if}
    <div class="arbeitsplatz">
      <div>
        {#if pdf}
          <PdfPlan pdfBase64={pdf.pdf_base64} seite={p.seite} markierungen={p.markierungen} ebenen={p.ebenen} {auswahl} {werkzeug} darf={darf&&!s.laeuft}
            onfertig={fertig} onauswahl={id=>auswahl=id} onedit={(id,g)=>{if(!objektGesperrt){merken();const item=p.markierungen.find(m=>m.id===id);if(item)item.geometrie=g}}}
            onfehler={e=>pdfFehler=e}/>
        {:else}<p class="meta">{laedt?"PDF wird geladen …":"Eine PDF-Unterlage auswählen."}</p>{/if}
        {#if messFehler}<p class="fehlertext" role="alert">Aufmaß prüfen: {messFehler}</p>{/if}
      </div>
      <aside>
        <h3>Zeichenebenen</h3>
        {#each p.ebenen as e (e.id)}
          <div class="ebene"><input type="radio" aria-label={`Auf Ebene ${e.bezeichnung} zeichnen`} name={`ebene-${s.id}`} value={e.id} bind:group={ebene}/>
            <input aria-label="Ebenenname" bind:value={e.bezeichnung} required/>
            <label title="Sichtbar"><input type="checkbox" bind:checked={e.sichtbar}/>An</label>
            <label title="Gesperrt"><input type="checkbox" bind:checked={e.gesperrt}/>Sperre</label>
          </div>
        {/each}
        <button type="button" class="sekundaer" onclick={()=>{const id=crypto.randomUUID();p.ebenen.push({id,bezeichnung:`Ebene ${p.ebenen.length+1}`,sichtbar:true,gesperrt:false});ebene=id}}>Ebene hinzufügen</button>
        <h3>Markierungen <span class="meta">({p.markierungen.length})</span></h3>
        <div class="markierungen">
          {#each p.markierungen as item (item.id)}<button type="button" class:aktiv={item.id===auswahl} onclick={()=>{auswahl=item.id;werkzeug="auswahl"}}>{item.bezeichnung}<span>{bedeutungen.find(b=>b[0]===item.bedeutung)?.[1]}</span></button>{/each}
          {#if !p.markierungen.length}<p class="meta">Ein Zeichenwerkzeug wählen und Punkte auf dem Plan setzen.</p>{/if}
        </div>
        {#if m}
          <h3>Eigenschaften</h3>
          <fieldset disabled={objektGesperrt}>
            <label>Bezeichnung<input bind:value={m.bezeichnung} required/></label>
            <label>Bedeutung<select aria-label="Bedeutung" bind:value={m.bedeutung}>{#each bedeutungen as [v,t]}<option value={v}>{t}</option>{/each}</select></label>
            <label>Zeichenebene<select bind:value={m.ebene_id}>{#each p.ebenen as e}<option value={e.id}>{e.bezeichnung}</option>{/each}</select></label>
            <label>Baugrundstück<select bind:value={m.grundstueck_id}><option value={null}>Nicht zugeordnet</option>{#each grund as g}<option value={g.id}>{g.bezeichnung}</option>{/each}</select></label>
            <label>Gebäude<select bind:value={m.gebaeude_id} onchange={()=>{if(m)m.geschoss_id=null}}><option value={null}>Nicht zugeordnet</option>{#each gebaeude as g}<option value={g.objekt}>{g.daten.bezeichnung}</option>{/each}</select></label>
            {#if m.gebaeude_id}<label>Geschoss<select bind:value={m.geschoss_id}><option value={null}>Nicht zugeordnet</option>{#each gebaeude.find(g=>g.objekt===m.gebaeude_id)?.daten.geschosse??[] as g,i}<option value={g.id}>{g.bezeichnung||`Geschoss ${i+1} (ohne Bezeichnung)`}</option>{/each}</select></label>{/if}
            {#if (m.bedeutung==="gebaeudegrundflaeche"||m.bedeutung==="geschoss_aussenkante")&&!m.gebaeude_id}<p class="hinweisbox">Für die Übernahme ein Gebäude zuordnen.{gebaeude.length?"":" Gebäude zuerst unter Grundlagen → Gebäude / Geschosse anlegen und speichern."}</p>
            {:else if m.bedeutung==="geschoss_aussenkante"&&!m.geschoss_id}<p class="hinweisbox">Für die Geschossfläche ein Geschoss zuordnen.{gebaeude.find(g=>g.objekt===m.gebaeude_id)?.daten.geschosse.length?"":" Für dieses Gebäude sind noch keine Geschosse gespeichert; ohne einzelne Geschosse dient die Gebäudegrundfläche als Fläche des Einzelgeschosses."}</p>
            {:else if m.bedeutung==="gebaeudegrundflaeche"&&m.geschoss_id}<p class="hinweisbox">Die Gebäudegrundfläche gilt für das ganze Gebäude; die Geschosszuordnung wird dafür nicht verwendet. Für eine Geschossfläche die Markierung duplizieren und die Kopie als „Geschoss – Außenkante“ kennzeichnen.</p>{/if}
            <div class="aktionen">
              <button type="button" class="sekundaer" title="Kopie derselben Kontur, z. B. als Gebäudegrundfläche und Geschoss-Außenkante" onclick={duplizieren}>Duplizieren</button>
              <button type="button" class="sekundaer" onclick={()=>{merken();p.markierungen=p.markierungen.filter(i=>i.id!==auswahl);auswahl=""}}>Markierung entfernen</button>
            </div>
          </fieldset>
          <dl class="messwerte">
            <dt>Fläche</dt><dd>{z(wert?.flaeche_m2)} m²</dd>
            <dt>Länge / Umfang</dt><dd>{z(wert?.laenge_m)} m</dd>
            <dt>{m.geometrie.art==="hoehenpunkt"?"Absolute Höhe":"Höhendifferenz"}</dt><dd>{z(wert?.hoehe_m)} m</dd>
            <dt>Horizontale</dt><dd>{z(wert?.horizontale_m)} m</dd>
            <dt>Neigung zur Horizontalen</dt><dd>{z(wert?.neigung_grad)}°</dd>
          </dl>
          {#if messLaeuft}<p class="meta" role="status">Messwerte werden aktualisiert …</p>{/if}
          {#each wert?.offen??[] as hinweis}<p class="hinweisbox">{hinweis}</p>{/each}
          <p class="meta">Nach dem Speichern: {ziel(m.bedeutung)} Eckpunkte lassen sich mit „Auswählen“ ziehen.</p>
        {/if}
      </aside>
    </div>
  {/snippet}
</Fachliste>

<style>
  .quellzeile{display:grid;grid-template-columns:1fr 2fr 6rem;gap:1rem;margin-bottom:1rem}
  .kalibrierung,.werkzeuge{display:flex;gap:.6rem;flex-wrap:wrap;align-items:end;margin:.7rem 0}
  .kalibrierung input[type=number]{width:8rem}
  .arbeitsplatz{display:grid;grid-template-columns:minmax(0,1fr) 300px;gap:1rem}
  aside{border-left:1px solid var(--linie,#ccd5df);padding-left:1rem}
  .ebene{display:flex;gap:.3rem;align-items:center;margin-bottom:.5rem}
  .ebene>input:not([type=radio]){min-width:3rem;width:100%}
  .ebene label{font-size:.75rem;white-space:nowrap}
  .markierungen{max-height:220px;overflow:auto;display:grid;gap:.25rem}
  .markierungen button{text-align:left}.markierungen span{display:block;font-size:.75rem;font-weight:400}
  .messwerte{display:grid;grid-template-columns:1fr auto;gap:.4rem;font-variant-numeric:tabular-nums}.messwerte dd{margin:0;font-weight:600}
  fieldset{border:0;padding:0;display:grid;gap:.5rem}
  .aktionen{display:flex;gap:.5rem;flex-wrap:wrap}
  @media(max-width:1100px){.arbeitsplatz{grid-template-columns:1fr}aside{border-left:0;padding-left:0}.quellzeile{grid-template-columns:1fr}}
</style>
