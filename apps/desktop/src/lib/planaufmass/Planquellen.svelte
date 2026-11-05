<script lang="ts">
  import { onMount } from "svelte";
  import { fehlertext } from "../api";
  import type { FachPlan, AbstandsPlan, Gebaeude, Planziel } from "../generated/plattform";
  import { bereichAktiv, ansichtsAbfrage } from "../ansichtsAbfrage.svelte";
  import { planApi } from "./api";
  import { aufmasseLaden, messwert as wert, hoehensystem, type Aufmassquelle } from "./quellen";
  import { Uebernahme, zielfeld, werttext } from "./Uebernahme.svelte";
  import UebernahmeVorschau from "./UebernahmeVorschau.svelte";
  const v=new Uebernahme();
  type Zielplan=FachPlan|AbstandsPlan|Gebaeude;
  let {projektId,plan,objektId="",onplan,onbusy}:{projektId:string;plan:Zielplan;objektId?:string;onplan:(p:Zielplan)=>void;onbusy:(b:boolean)=>void}=$props();
  let aufmasse=$state<Aufmassquelle[]>([]);
  let ziele=$state<Planziel[]>([]),ziel=$state(""),quelle=$state(""),fehler=$state(""),busy=$state(false),meldung=$state("");
  const groesse=$derived(ziele.find(z=>z.ziel===ziel)?.groesse);
  const optionen=$derived(aufmasse.flatMap(a=>a.stand.daten.markierungen.flatMap(m=>{
    const w=a.werte.find(w=>w.id===m.id),v=w?wert(w,groesse):null;
    if(v==null||m.bedeutung==="unbestimmt")return [];
    if("modul" in plan){
      if(m.grundstueck_id!==plan.grundstueck_id||m.gebaeude_id!==plan.gebaeude_id)return [];
      // Vorauswahl entsprechend der Übernahmeprüfung im Core (planaufmass/service.rs).
      const art=m.geometrie.art;
      if(plan.modul==="hoehen"&&ziel.startsWith("punkte.")){
        const system=(plan.eingaben as {hoehenbezug?:unknown}|null)?.hoehenbezug;
        if(art!=="hoehenpunkt"||hoehensystem(a.stand.daten,m)!==system)return [];
      }else if(groesse==="hoehe"&&art!=="strecke")return [];
      if(plan.modul==="din277"&&ziel.startsWith("positionen.")&&!["nuf","tf","vf","kgf"].includes(m.bedeutung))return [];
    } else if("waende" in plan) {
      // Lage aus Linien „Abstandsfläche“, Grenzen aus zugeordneten Flurstücken (planaufmass/abstand.rs).
      if(ziel.endsWith(".grenze"))return m.bedeutung==="flurstueck"?[{value:`${a.stand.objekt}|${m.id}`,a,m,v}]:[];
      const wand=plan.waende.find(w=>w.id===ziel.split(".")[1]);
      if(ziel.endsWith(".ansicht")){
        const art=a.stand.daten.ausschnitte?.find(x=>x.id===m.ausschnitt_id)?.art;
        if(!wand||m.bedeutung!=="abstandsflaeche"||!["ansicht","schnitt","detail"].includes(art??"")||(m.gebaeude_id!=null&&m.gebaeude_id!==(wand.gebaeude_id??null)))return [];
        return [{value:`${a.stand.objekt}|${m.id}`,a,m,v}];
      }
      if(ziel.endsWith(".lage")){
        if(!wand||m.bedeutung!=="abstandsflaeche"||m.gebaeude_id!==(wand.gebaeude_id??null))return [];
        return [{value:`${a.stand.objekt}|${m.id}`,a,m,v}];
      }
      if(!wand||m.gebaeude_id!==(wand.gebaeude_id??null))return [];
      // Eckhöhen absolut im Höhensystem des Plans, Dachwerte aus Strecken (planaufmass/abstand.rs).
      const absolut=/^(gelaende|abschluss)_[ab]_m$/.test(ziel.split(".")[2]??"");
      if(absolut?(m.geometrie.art!=="hoehenpunkt"||hoehensystem(a.stand.daten,m)!==plan.hoehenbezug):m.geometrie.art!=="strecke")return [];
    } else if(m.gebaeude_id!==objektId || (ziel.startsWith("geschosse.") && m.geschoss_id!==ziel.split(".")[1]))return [];
    // Flächen nur aus passend gekennzeichneten Konturen; der Core weist andere ab.
    else if(groesse==="flaeche" && m.bedeutung!==(ziel.startsWith("geschosse.")?"geschoss_aussenkante":"gebaeudegrundflaeche"))return [];
    return [{value:`${a.stand.objekt}|${m.id}`,a,m,v}];
  })));
  let generation=0;
  async function laden(){
    const n=++generation;
    try{
      const daten=await aufmasseLaden(projektId);
      if(n===generation){aufmasse=daten;fehler=""}
    }catch(e){if(n===generation)fehler=fehlertext(e)}
  }
  onMount(()=>()=>{generation++});
  const sichtbar=bereichAktiv();
  ansichtsAbfrage(()=>projektId,["fachobjekte","dokumente"],laden);
  $effect(()=>{
    if(!sichtbar()) return;
    const snapshot=JSON.stringify(plan);let gone=false;
    const anfrage="modul" in plan?planApi.ziele(JSON.parse(snapshot)):"waende" in plan?planApi.abstandsziele(JSON.parse(snapshot)):planApi.gebaeudeziele(JSON.parse(snapshot));
    void anfrage.then(z=>{if(!gone)ziele=z}).catch(e=>{if(!gone)fehler=fehlertext(e)});
    return()=>{gone=true};
  });
  function loesen(b:NonNullable<Zielplan["planbindungen"]>[number]){
    plan.planbindungen=plan.planbindungen?.filter(x=>x!==b);
    // Der Wert bleibt als manuelle Eingabe; der PDF-Herkunftstext entfällt.
    if("modul" in plan){
      const [tabelle,id]=b.ziel.split(".");
      const zeilen=(plan.eingaben as Record<string,unknown>|null)?.[tabelle];
      const zeile=Array.isArray(zeilen)?zeilen.find(z=>z?.id===id) as Record<string,unknown>|undefined:undefined;
      if(zeile&&typeof zeile.quelle==="string"&&zeile.quelle.startsWith("PDF "))zeile.quelle="";
    }
  }
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
      const zeilen=(neu.planbindungen??[]).filter(b=>aktualisieren||b.ziel===ziel).map(b=>{const z=ziele.find(z=>z.ziel===b.ziel),a=aufmasse.find(a=>a.stand.objekt===b.aufmass_id),einheit=b.groesse==="flaeche"?"m²":b.groesse==="neigung"?"°":"m";return {ziel:z?.titel??b.ziel,vorher:werttext(zielfeld("modul" in plan?plan.eingaben:plan,b.ziel),einheit),nachher:werttext(zielfeld("modul" in neuPlan?neuPlan.eingaben:neuPlan,b.ziel),einheit),quelle:`${a?.stand.daten.bezeichnung??"Aufmaß"} · ${a?.stand.daten.markierungen.find(m=>m.id===b.markierung_id)?.bezeichnung??"Markierung"} · R${b.revision}`}});
      if(!(await v.fragen(zeilen)))return;
      if(JSON.stringify(plan)!==before)throw Error("Eingaben wurden zwischenzeitlich geändert. Übernahme erneut ausführen.");
      onplan(neuPlan);meldung="Messwerte im Entwurf eingesetzt. Angaben prüfen und speichern.";
    }catch(e){fehler=fehlertext(e)}finally{busy=false;onbusy(false)}
  }
</script>
<UebernahmeVorschau {v}/>

<section>
  <h3>Aufmaß verknüpfen</h3>
  <p class="meta">Gespeicherte Markierungen aus „Aufmaß“ mit einer Position dieser Berechnung verbinden. Grundstück und Gebäude müssen übereinstimmen. Dieselbe Markierung darf mehrere Berechnungen versorgen. Fachliche Anrechnung und Regelprüfung erfolgen in der Berechnung.</p>
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  {#if "modul" in plan&&plan.gebaeudedaten_uebernehmen===true}<p class="hinweisbox">Die Werte werden derzeit aus Gebäude/Geschosse übernommen. Gebäudeübernahme und Aufmaß dürfen nicht gleichzeitig dieselben Eingaben liefern. Flächen und Höhen besser im Gebäude aus dem Aufmaß übernehmen; sie gelangen dann über die Gebäudeübernahme hierher.
    <button type="button" class="link" disabled={busy} onclick={()=>{if("modul" in plan)plan.gebaeudedaten_uebernehmen=false}}>Gebäudeübernahme für diese Berechnung beenden</button></p>{/if}
  {#each aufmasse.filter(a=>a.fehler) as a}<p class="hinweisbox">{a.stand.daten.bezeichnung}: {a.fehler}</p>{/each}
  {#if !ziele.length}<p class="meta">{"modul" in plan?"Zuerst unter „Eingaben“ eine Position für das Aufmaß anlegen.":"waende" in plan?"Zuerst unter „Wände / Bauteile“ eine manuelle Wand anlegen.":"Gebäude zuerst speichern."} Dieses Modul bietet nur fachlich unterstützte Zielgrößen an.</p>{/if}
  <label>Zielfeld<select bind:value={ziel}><option value="">Position und Messgröße auswählen</option>{#each ziele as z}<option value={z.ziel}>{z.titel}</option>{/each}</select></label>
  <label>Planmarkierung<select bind:value={quelle}><option value="">Markierung auswählen</option>{#each optionen as q}<option value={q.value}>{q.a.stand.daten.bezeichnung}{q.m.ausschnitt_id?` › ${q.a.stand.daten.ausschnitte?.find(x=>x.id===q.m.ausschnitt_id)?.bezeichnung??"Ausschnitt"}`:""} · {q.m.bezeichnung} · {q.v.toLocaleString("de-DE",{maximumFractionDigits:3})} {groesse==="flaeche"?"m²":groesse==="neigung"?"°":"m"}</option>{/each}</select></label>
  <button type="button" disabled={busy||!optionen.some(q=>q.value===quelle)||!groesse||("modul" in plan&&plan.gebaeudedaten_uebernehmen===true)} onclick={()=>uebernehmen()}>Verknüpfen und Messwert einsetzen</button>
  {#if ziel&&!optionen.length}<p class="hinweisbox">Keine passende Markierung. Angeboten werden nur gespeicherte Markierungen mit bestimmter Bedeutung und messbarem Wert, deren Baugrundstück und Gebäude{"modul" in plan?" mit dieser Berechnung":""} übereinstimmen{ziel.startsWith("geschosse.")?" und die diesem Geschoss zugeordnet sind":""}. {groesse==="hoehe"?"Absolute Höhen stammen aus Höhenpunkten mit Höhenbezug, Höhendifferenzen aus Strecken.":groesse==="flaeche"?"Flächen benötigen einen Maßstab und eine passende Bedeutung.":""}</p>{/if}
  <ul>
    {#each plan.planbindungen??[] as b}
      {@const a=aufmasse.find(a=>a.stand.objekt===b.aufmass_id)}
      <li>{a?.stand.daten.bezeichnung??"Aufmaß fehlt"} · {a?.stand.daten.markierungen.find(m=>m.id===b.markierung_id)?.bezeichnung??"Markierung fehlt"}
        → {ziele.find(z=>z.ziel===b.ziel)?.titel??"Zielposition fehlt"} · R{b.revision}
        {#if a&&a.stand.revision!==b.revision}<strong> – Aufmaß inzwischen gespeichert (R{a.stand.revision})</strong>{/if}
        <button type="button" class="link" disabled={busy} onclick={()=>loesen(b)}>Verknüpfung lösen</button>
      </li>
    {/each}
  </ul>
  {#if plan.planbindungen?.some(b=>aufmasse.some(a=>a.stand.objekt===b.aufmass_id&&a.stand.revision!==b.revision))}<p class="meta">Rein gestalterische Änderungen (Ebenen, andere Markierungen) lassen die Werte gültig. Wurden Kontur, Maßstab, Bezeichnung oder Zuordnung der verknüpften Markierung geändert, meldet die Prüfung dies; dann den aktuellen Stand übernehmen.</p>{/if}
  {#if plan.planbindungen?.length}<button type="button" class="sekundaer" disabled={busy} onclick={()=>uebernehmen(true)}>Aktuellen Aufmaßstand übernehmen</button>{/if}
  {#if meldung}<p role="status">{meldung}</p>{/if}
</section>
