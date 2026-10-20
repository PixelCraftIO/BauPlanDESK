<script lang="ts">
  import { onMount } from "svelte";
  import { fehlertext } from "../api";
  import type { FachPlan, AbstandsPlan, Gebaeude, Planaufmass, Planmesswert, Planziel, Planmessgroesse } from "../generated/plattform";
  import { plattformApi, type Datensatz } from "../plattformApi";
  import { abonnieren } from "../projektEreignisse";
  import { planApi } from "./api";
  type Zielplan=FachPlan|AbstandsPlan|Gebaeude;
  let {projektId,plan,objektId="",onplan,onbusy}:{projektId:string;plan:Zielplan;objektId?:string;onplan:(p:Zielplan)=>void;onbusy:(b:boolean)=>void}=$props();
  let aufmasse=$state<{stand:Datensatz<Planaufmass>;werte:Planmesswert[];fehler:string}[]>([]);
  let ziele=$state<Planziel[]>([]),ziel=$state(""),quelle=$state(""),fehler=$state(""),busy=$state(false),meldung=$state("");
  const groesse=$derived(ziele.find(z=>z.ziel===ziel)?.groesse);
  function wert(w:Planmesswert,g:Planmessgroesse|undefined){
    return g==="flaeche"?w.flaeche_m2:g==="hoehe"?w.hoehe_m:g==="horizontale"?w.horizontale_m:g==="laenge"?w.laenge_m:g==="neigung"?w.neigung_grad:null;
  }
  const optionen=$derived(aufmasse.flatMap(a=>a.stand.daten.markierungen.flatMap(m=>{
    const w=a.werte.find(w=>w.id===m.id),v=w?wert(w,groesse):null;
    if(v==null||m.bedeutung==="unbestimmt")return [];
    if("modul" in plan){
      if(m.grundstueck_id!==plan.grundstueck_id||m.gebaeude_id!==plan.gebaeude_id)return [];
    } else if("waende" in plan) {
      const wand=plan.waende.find(w=>w.id===ziel.split(".")[1]);
      if(!wand||m.gebaeude_id!==(wand.gebaeude_id??null))return [];
    } else if(m.gebaeude_id!==objektId || (ziel.startsWith("geschosse.") && m.geschoss_id!==ziel.split(".")[1]))return [];
    return [{value:`${a.stand.objekt}|${m.id}`,a,m,v}];
  })));
  let generation=0;
  async function laden(){
    const n=++generation;
    try{
      const liste=await plattformApi.liste<Planaufmass>(projektId,"planaufmass");
      const daten=await Promise.all(liste.map(async stand=>{
        try{return{stand,werte:await planApi.pruefen(projektId,stand.daten),fehler:""}}
        catch(e){return{stand,werte:[],fehler:fehlertext(e)}}
      }));
      if(n===generation){aufmasse=daten;fehler=""}
    }catch(e){if(n===generation)fehler=fehlertext(e)}
  }
  onMount(()=>{void laden();return()=>{generation++}});
  $effect(()=>abonnieren(projektId,["fachobjekte","dokumente"],()=>void laden()));
  $effect(()=>{
    const snapshot=JSON.stringify(plan);let gone=false;
    const anfrage="modul" in plan?planApi.ziele(JSON.parse(snapshot)):"waende" in plan?planApi.abstandsziele(JSON.parse(snapshot)):planApi.gebaeudeziele(JSON.parse(snapshot));
    void anfrage.then(z=>{if(!gone)ziele=z}).catch(e=>{if(!gone)fehler=fehlertext(e)});
    return()=>{gone=true};
  });
  async function uebernehmen(aktualisieren=false){
    const src=optionen.find(q=>q.value===quelle);
    if(!aktualisieren&&(!src||!groesse))return;
    const before=JSON.stringify(plan),neu=JSON.parse(before) as Zielplan;
    if(aktualisieren){
      neu.planbindungen=(neu.planbindungen??[]).map(b=>({...b,revision:aufmasse.find(a=>a.stand.objekt===b.aufmass_id)?.stand.revision??b.revision}));
    }else if(src&&groesse){
      neu.planbindungen=[...(neu.planbindungen??[]).filter(b=>b.ziel!==ziel),{
        aufmass_id:src.a.stand.objekt,revision:src.a.stand.revision,markierung_id:src.m.id,groesse,ziel,
      }];
      if("modul" in neu)neu.uebernahmen=neu.uebernahmen.filter(u=>u.ziel!==ziel);
    }
    busy=true;onbusy(true);fehler="";meldung="";
    try{
      const neuPlan=await ("modul" in neu?planApi.einsetzen(projektId,neu):"waende" in neu?planApi.abstand(projektId,neu):planApi.gebaeude(projektId,objektId,neu));
      if(JSON.stringify(plan)!==before)throw Error("Eingaben wurden zwischenzeitlich geändert. Übernahme erneut ausführen.");
      onplan(neuPlan);meldung="Messwerte im Entwurf eingesetzt. Angaben prüfen und speichern.";
    }catch(e){fehler=fehlertext(e)}finally{busy=false;onbusy(false)}
  }
</script>

<section>
  <h3>PDF-Aufmaß verknüpfen</h3>
  <p class="meta">Gespeicherte Markierungen aus „PDF-Aufmaß“ mit einer Position dieser Berechnung verbinden. Grundstück und Gebäude müssen übereinstimmen. Dieselbe Markierung darf mehrere Berechnungen versorgen. Fachliche Anrechnung und Regelprüfung erfolgen in der Berechnung.</p>
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  {#each aufmasse.filter(a=>a.fehler) as a}<p class="hinweisbox">{a.stand.daten.bezeichnung}: {a.fehler}</p>{/each}
  {#if !ziele.length}<p class="meta">{"modul" in plan?"Zuerst unter „Eingaben“ eine Position für das Aufmaß anlegen.":"Zuerst unter „Wände / Bauteile“ eine manuelle Wand anlegen."} Dieses Modul bietet nur fachlich unterstützte Zielgrößen an.</p>{/if}
  <label>Zielfeld<select bind:value={ziel}><option value="">Position und Messgröße auswählen</option>{#each ziele as z}<option value={z.ziel}>{z.titel}</option>{/each}</select></label>
  <label>Planmarkierung<select bind:value={quelle}><option value="">Markierung auswählen</option>{#each optionen as q}<option value={q.value}>{q.a.stand.daten.bezeichnung} · {q.m.bezeichnung} · {q.v.toLocaleString("de-DE",{maximumFractionDigits:3})} {groesse==="flaeche"?"m²":groesse==="neigung"?"°":"m"}</option>{/each}</select></label>
  <button type="button" disabled={busy||!optionen.some(q=>q.value===quelle)||!groesse} onclick={()=>uebernehmen()}>Verknüpfen und Messwert einsetzen</button>
  <ul>
    {#each plan.planbindungen??[] as b}
      {@const a=aufmasse.find(a=>a.stand.objekt===b.aufmass_id)}
      <li>{a?.stand.daten.bezeichnung??"Aufmaß fehlt"} · {a?.stand.daten.markierungen.find(m=>m.id===b.markierung_id)?.bezeichnung??"Markierung fehlt"}
        → {ziele.find(z=>z.ziel===b.ziel)?.titel??"Zielposition fehlt"} · R{b.revision}
        {#if a&&a.stand.revision!==b.revision}<strong> – Quelle geändert</strong>{/if}
        <button type="button" class="link" disabled={busy} onclick={()=>plan.planbindungen=plan.planbindungen?.filter(x=>x!==b)}>Verknüpfung lösen</button>
      </li>
    {/each}
  </ul>
  {#if plan.planbindungen?.length}<button type="button" class="sekundaer" disabled={busy} onclick={()=>uebernehmen(true)}>Aktuellen Aufmaßstand übernehmen</button>{/if}
  {#if meldung}<p role="status">{meldung}</p>{/if}
</section>
