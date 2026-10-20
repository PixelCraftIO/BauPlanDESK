<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext, type Project, type DocumentVersion, type PdfVorschauDaten, type GrundstueckKontext } from "../api";
  import type { Planaufmass, Planmarkierung, Planbedeutung, Planmesswert, Plangeometrie, Planausschnitt, Ausschnittart, Planaufgabe, Gebaeude } from "../generated/plattform";
  import { FachSitzung } from "../FachSitzung.svelte";
  import Fachliste from "../Fachliste.svelte";
  import { plattformApi, type Datensatz } from "../plattformApi";
  import { abonnieren } from "../projektEreignisse";
  import PdfPlan from "./PdfPlan.svelte";
  import Planaufgaben from "./Planaufgaben.svelte";
  import { planApi } from "./api";
  import { verschieben, type Point } from "./pdfGeometry";
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
    ["nullpunkt","Gebäudenullpunkt ±0,00"],["fussboden","OK Fertigfußboden"],["decke","OK Decke"],["fundamentsohle","Fundamentsohle"],["hoehenmass","Höhenmaß (senkrechte Strecke)"],
  ];
  const arten: [Ausschnittart,string][]=[["grundriss","Grundriss"],["lageplan","Lageplan"],["schnitt","Schnitt"],["ansicht","Ansicht"],["detail","Detail"]];
  const mitHoehen=(a:Ausschnittart)=>a==="schnitt"||a==="ansicht"||a==="detail";
  // Werkzeug, Beschriftung, Tastaturkürzel.
  const werkzeuge:[string,string,string][]=[["auswahl","Auswählen","V"],["verschieben","Verschieben","M"],["polygon","Polygon","P"],["rechteck","Rechteck","R"],["kreis","Kreis","K"],["strecke","Strecke / Dach","S"],["hoehenpunkt","Höhenpunkt","H"]];
  let werkzeug=$state("auswahl"),auswahl=$state(""),ebene=$state(""),nenner=$state(100),referenz=$state(10),hwert=$state(0),hsystem=$state("m ü. NHN");
  let werte=$state<Planmesswert[]>([]),messFehler=$state(""),messLaeuft=$state(false),messStand=$state("");
  let undo=$state<Planmarkierung[][]>([]),redo=$state<Planmarkierung[][]>([]);
  let aktiv=$state(""),rahmenAuswahl=$state(""),vollbild=$state(false),masse=$state(true),hoeheEditor=$state(560),einrichtungOffen=$state(true);
  // Aus den Aufgaben vorbelegte Bedeutung und Zuordnung für die nächste Markierung.
  let vorbelegung=$state<Planaufgabe|null>(null);
  let editor=$state<HTMLDivElement>(), plan=$state<ReturnType<typeof PdfPlan>>();
  const m=$derived(s.eingabe?.markierungen.find(m=>m.id===auswahl));
  const layer=$derived(s.eingabe?.ebenen.find(e=>e.id===ebene));
  const objektGesperrt=$derived(!darf||s.laeuft||!!s.eingabe?.ebenen.find(e=>e.id===m?.ebene_id)?.gesperrt);
  const quelleAlt=$derived(s.eingabe&&dateien.some(d=>d.document_id===s.eingabe!.dokument.dokument_id&&d.version>s.eingabe!.dokument.version));
  const signatur=$derived(JSON.stringify(s.eingabe));
  const aktuelleWerte=$derived(signatur===messStand?werte:[]);
  const wert=$derived(aktuelleWerte.find(w=>w.id===auswahl));
  const ausschnitte=$derived(s.eingabe?.ausschnitte??[]);
  const aktueller=$derived(ausschnitte.find(a=>a.id===aktiv));
  // Eigenschaften gelten für den geöffneten, sonst für den im Blatt gewählten Ausschnitt.
  const bearbeiteter=$derived(aktueller??ausschnitte.find(a=>a.id===rahmenAuswahl));
  const hoehenMoeglich=$derived(!aktueller||mitHoehen(aktueller.art));
  const nullpunkte=$derived(gebaeude.flatMap(g=>g.daten.geometrie?.nullpunkt_m_nhn!=null?[{name:g.daten.bezeichnung,wert:g.daten.geometrie.nullpunkt_m_nhn}]:[]));
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
    untrack(()=>{auswahl="";ebene=s.eingabe?.ebenen[0]?.id??"";undo=[];redo=[];werkzeug="auswahl";aktiv="";rahmenAuswahl="";einrichtungOffen=!s.eingabe?.kalibrierung;void id});
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
  // Die Zeichenfläche nutzt die Fensterhöhe unterhalb ihres oberen Rands; im
  // Vollbild das ganze Fenster.
  $effect(()=>{
    const el=editor; if(!el)return;
    let frame=0;
    const messen=()=>{cancelAnimationFrame(frame);frame=requestAnimationFrame(()=>{
      const top=el.getBoundingClientRect().top;
      hoeheEditor=Math.min(window.innerHeight-16,Math.max(480,window.innerHeight-top-16));
    })};
    messen();
    window.addEventListener("resize",messen);window.addEventListener("scroll",messen,{passive:true});
    return()=>{cancelAnimationFrame(frame);window.removeEventListener("resize",messen);window.removeEventListener("scroll",messen)};
  });
  // Tastaturkürzel für Werkzeuge und Auswahl; Eingabefelder bleiben unberührt.
  // Kürzel gelten, solange die letzte Bedienung im Editor war, auch wenn der
  // Fokus danach auf das Dokument fiel (z. B. nach dem Entfernen eines Eintrags).
  let imEditor=false;
  $effect(()=>{
    const merke=(e:Event)=>{imEditor=!!editor?.contains(e.target as Node)};
    document.addEventListener("pointerdown",merke,true);document.addEventListener("focusin",merke,true);
    return()=>{document.removeEventListener("pointerdown",merke,true);document.removeEventListener("focusin",merke,true)};
  });
  $effect(()=>{
    const taste=(e:KeyboardEvent)=>{
      if(!s.eingabe||(e.target as Element).closest("input,select,textarea,[contenteditable]"))return;
      const drin=editor?.contains(e.target as Node)||(e.target===document.body&&imEditor);
      if(!drin&&!vollbild)return;
      const k=e.key.toLowerCase();
      if(e.ctrlKey||e.metaKey){
        if(k==="z"){e.preventDefault();rueckwaerts()}
        else if(k==="y"){e.preventDefault();vorwaerts()}
        else if(k==="d"){e.preventDefault();duplizieren()}
        return;
      }
      if(e.altKey)return;
      if(e.key==="Escape"){
        if(werkzeug!=="auswahl")werkzeug="auswahl";
        else if(aktiv)schliessen();
        else if(vollbild)vollbild=false;
        return;
      }
      const w=werkzeuge.find(w=>w[2].toLowerCase()===k);
      if(w&&!(w[0]==="hoehenpunkt"&&!hoehenMoeglich)){e.preventDefault();werkzeug=w[0];vorbelegung=null;return}
      if(k==="z"){e.preventDefault();plan?.zoomfensterStarten();return}
      if(e.key==="Delete"&&m&&!objektGesperrt){e.preventDefault();entfernen()}
      else if(e.key.startsWith("Arrow")&&m&&!objektGesperrt){
        e.preventDefault();
        const d=e.shiftKey?10:1;
        const dx=e.key==="ArrowLeft"?-d:e.key==="ArrowRight"?d:0, dy=e.key==="ArrowUp"?-d:e.key==="ArrowDown"?d:0;
        merken();m.geometrie=verschieben(m.geometrie,dx,dy);
      }
      else if(e.key==="PageDown"||e.key==="PageUp"){
        if(!ausschnitte.length)return;
        e.preventDefault();
        const i=ausschnitte.findIndex(a=>a.id===aktiv);
        const n=(i+(e.key==="PageDown"?1:-1)+ausschnitte.length)%ausschnitte.length;
        oeffnen(ausschnitte[n].id);
      }
      else if(e.key==="F11"){e.preventDefault();vollbild=!vollbild}
    };
    window.addEventListener("keydown",taste);
    return()=>window.removeEventListener("keydown",taste);
  });
  // Wohin eine gespeicherte Markierung übernommen wird; nur Bedienhinweis.
  function ziel(b:Planbedeutung){
    if(b==="gebaeudegrundflaeche")return "Grundlagen → Gebäude / Geschosse: am Feld „Gebäudegrundfläche“ über „Aus PDF-Aufmaß“ übernehmen. Von dort geht sie in GRZ des Baugrundstücks ein.";
    if(b==="geschoss_aussenkante")return "Grundlagen → Gebäude / Geschosse: im jeweiligen Geschoss über „Aus PDF-Aufmaß“ übernehmen. Von dort geht sie in GFZ, Geschossigkeit und Vollgeschossprüfung ein.";
    if(b==="raum"||b==="aufenthaltsraum"||b==="nuf"||b==="tf"||b==="vf")return "Berechnungen → Flächen- und Raumbuch: „Räume aus PDF-Aufmaß übernehmen“ (Gebäude und Geschoss zuordnen). DIN 277 und Wohnfläche können außerdem unter „Quellen / Übernahmen“ verknüpft werden.";
    if(b==="nullpunkt"||b==="fussboden"||b==="decke"||b==="fundamentsohle"||b==="hoehenmass")return "Gebäude / Geschosse → „Höhen und alle Verknüpfungen aus PDF-Aufmaß“ oder direkt hier unter „Aufgaben → Übernehmen“.";
    if(b==="gelaende"||b==="traufe"||b==="first"||b==="dach"||b==="aussenwand")return "Je nach Ziel: Abstandsflächen → Grundlagen → „PDF-Aufmaß verknüpfen“ (manuelle Wand), Gebäudehöhen → „Quellen / Übernahmen“ oder Gebäude / Geschosse → „Höhen und alle Verknüpfungen aus PDF-Aufmaß“.";
    if(b==="flurstueck")return "Eine direkte Übernahme in Flurstücke und Baugrundstücksermittlung gibt es noch nicht; die Fläche dient vorerst zur Kontrolle der Katasterangaben.";
    if(b==="unbestimmt")return "Zuerst eine Bedeutung wählen; sie bestimmt, welche Berechnung den Messwert übernehmen kann.";
    return "In der Fachberechnung eine Position anlegen und unter „Quellen / Übernahmen → PDF-Aufmaß“ verknüpfen.";
  }
  function duplizieren(){
    if(!s.eingabe||!m||objektGesperrt)return;
    merken();
    const kopie={...structuredClone($state.snapshot(m)),id:crypto.randomUUID(),bezeichnung:`${m.bezeichnung} (Kopie)`};
    s.eingabe.markierungen.push(kopie);auswahl=kopie.id;
  }
  function entfernen(){
    if(!s.eingabe||!m)return;
    merken();s.eingabe.markierungen=s.eingabe.markierungen.filter(i=>i.id!==auswahl);auswahl="";
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
      const k={art:"strecke" as const,von:g.von,bis:g.bis,meter:referenz};
      // Im geöffneten Ausschnitt gilt die Kalibrierung nur dort.
      if(aktueller)aktueller.kalibrierung=k;else p.kalibrierung=k;
      werkzeug="auswahl";return;
    }
    if(werkzeug==="bezug"&&g.art==="hoehenpunkt"){
      p.hoehenbezug={punkt:g.punkt,hoehe_m:hwert,system:hsystem};werkzeug="auswahl";return;
    }
    if(werkzeug==="bezugslinie"&&g.art==="strecke"){
      const a=bearbeiteter; if(!a)return;
      a.bezugslinie={von:g.von,bis:g.bis,hoehe_m:hwert,system:hsystem};werkzeug="auswahl";return;
    }
    if(werkzeug==="ausschnitt"&&g.art==="rechteck"){
      ausschnittAnlegen(g.von,g.bis);werkzeug="auswahl";return;
    }
    if(!layer||layer.gesperrt||!layer.sichtbar){s.fehler="Eine sichtbare, ungesperrte Zeichenebene auswählen.";return}
    merken();
    const id=crypto.randomUUID();
    const v=vorbelegung;
    const neu:Planmarkierung={id,ebene_id:layer.id,bezeichnung:v?`${v.gruppe} · ${v.titel}`:`Markierung ${p.markierungen.length+1}`,bedeutung:v?.bedeutung??"unbestimmt",geometrie:g,
      grundstueck_id:v?.grundstueck_id??(grund.length===1?grund[0].id:null),gebaeude_id:v?.gebaeude_id??null,geschoss_id:v?.geschoss_id??null};
    if(aktiv)neu.ausschnitt_id=aktiv;
    p.markierungen.push(neu);
    auswahl=id;werkzeug="auswahl";vorbelegung=null;
  }
  function aufgabeZeichnen(a:Planaufgabe){
    if(a.geometrie==="hoehenpunkt"&&!hoehenMoeglich){
      s.meldung=`„${a.titel}“ ist eine Höhe: zuerst einen Schnitt oder eine Ansicht als Ausschnitt öffnen und dort die Bezugslinie setzen.`;return;
    }
    vorbelegung=a;
    werkzeug=a.geometrie==="flaeche"?"polygon":a.geometrie==="hoehenpunkt"?"hoehenpunkt":"strecke";
  }
  function seiteWechseln(n:number){
    if(!s.eingabe||n===s.eingabe.seite)return;
    s.eingabe.seite=n;basisReset();
  }
  function ausschnittAnlegen(von:Point,bis:Point){
    const p=s.eingabe;if(!p)return;
    p.ausschnitte??=[];
    const n=p.ausschnitte.length+1;
    const a:Planausschnitt={id:crypto.randomUUID(),bezeichnung:`Ausschnitt ${n}`,art:"grundriss",von,bis};
    p.ausschnitte.push(a);
    // Eine Zeichenebene je Ausschnitt; Umbenennen folgt beim Namen des Ausschnitts.
    p.ebenen.push({id:crypto.randomUUID(),bezeichnung:a.bezeichnung,sichtbar:true,gesperrt:false});
    rahmenAuswahl=a.id;
  }
  function oeffnen(id:string){
    const a=ausschnitte.find(a=>a.id===id);if(!a||!s.eingabe)return;
    aktiv=id;rahmenAuswahl="";auswahl="";werkzeug="auswahl";
    const e=s.eingabe.ebenen.find(e=>e.bezeichnung===a.bezeichnung&&!e.gesperrt);
    if(e)ebene=e.id;
  }
  function schliessen(){aktiv="";auswahl="";werkzeug="auswahl";plan?.einpassen()}
  function umbenennen(a:Planausschnitt,name:string){
    const e=s.eingabe?.ebenen.find(e=>e.bezeichnung===a.bezeichnung);
    a.bezeichnung=name;
    if(e)e.bezeichnung=name;
  }
  async function ausschnittEntfernen(a:Planausschnitt){
    const p=s.eingabe;if(!p)return;
    const n=p.markierungen.filter(m=>m.ausschnitt_id===a.id).length;
    if(n&&!(await ask(`${n} Markierung(en) gehören zu „${a.bezeichnung}“. Sie bleiben erhalten und rechnen dann mit Maßstab und Höhenbezug des Blattes; Höhenpunkte verlieren ihren Bezug. Ausschnitt trotzdem entfernen?`,{title:"Ausschnitt entfernen",kind:"warning"})))return;
    merken();
    for(const m of p.markierungen)if(m.ausschnitt_id===a.id)delete m.ausschnitt_id;
    p.ausschnitte=(p.ausschnitte??[]).filter(x=>x.id!==a.id);
    if(aktiv===a.id)schliessen();
    if(rahmenAuswahl===a.id)rahmenAuswahl="";
  }
  function ausschnittDuplizieren(a:Planausschnitt){
    const p=s.eingabe;if(!p)return;
    const b=rahmenVon(a);
    const kopie:Planausschnitt={...structuredClone($state.snapshot(a)),id:crypto.randomUUID(),bezeichnung:`${a.bezeichnung} (Kopie)`,von:[b.x+b.w+20,b.y],bis:[b.x+2*b.w+20,b.y+b.h]};
    delete kopie.bezugslinie;
    p.ausschnitte=[...(p.ausschnitte??[]),kopie];rahmenAuswahl=kopie.id;
  }
  const rahmenVon=(a:{von:Point;bis:Point})=>({x:Math.min(a.von[0],a.bis[0]),y:Math.min(a.von[1],a.bis[1]),w:Math.abs(a.bis[0]-a.von[0]),h:Math.abs(a.bis[1]-a.von[1])});
  function source(value:string){
    if(!s.eingabe)return;
    const d=dateien.find(d=>`${d.document_id}|${d.version}`===value);
    if(d){s.eingabe.dokument={dokument_id:d.document_id,version:d.version};s.eingabe.seite=1;basisReset()}
  }
  function basisReset(){
    if(!s.eingabe)return;
    s.eingabe.kalibrierung=null;s.eingabe.hoehenbezug=null;
    // Rahmen bleiben als Vorschlag; ihre Maßstäbe und Bezugslinien sind neu zu bestätigen.
    for(const a of s.eingabe.ausschnitte??[]){delete a.kalibrierung;delete a.bezugslinie}
    aktiv="";
    s.meldung="Unterlage geändert. Maßstab und Höhenbezug neu bestimmen und alle Markierungen kontrollieren.";
  }
  const massstabText=(k:Planaufmass["kalibrierung"]|undefined)=>!k?"":k.art==="massstab"?`1 : ${z(k.nenner)}`:`kalibriert über ${z(k.meter)} m`;
  const z=(n:number|null|undefined)=>n==null?"—":n.toLocaleString("de-DE",{maximumFractionDigits:3});
  const anzahl=(a:Planausschnitt)=>s.eingabe?.markierungen.filter(m=>m.ausschnitt_id===a.id)??[];
</script>

<Fachliste titel="PDF-Aufmaß" {s} bereich="planaufmass" {darf} zusatzBeschaeftigt={()=>laedt}>
  {#snippet children(p)}
    <p class="meta">PDF-Pläne unter „Projektdateien“ importieren. Je Blatt ein Aufmaß; mehrere Zeichnungen auf einem Blatt (Grundriss, Schnitte, Ansichten) als Ausschnitte mit eigenem Maßstab und eigener Bezugslinie anlegen. Die Original-PDF bleibt erhalten.</p>
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
    <details class="einrichtung" bind:open={einrichtungOffen}>
      <summary>Blattmaßstab und Höhenbezug des Blattes <span class="meta">{!p.kalibrierung?"· Maßstab fehlt":`· ${massstabText(p.kalibrierung)}`}{p.hoehenbezug?` · Bezug ${z(p.hoehenbezug.hoehe_m)} ${p.hoehenbezug.system}`:""}</span></summary>
      <div class="kalibrierung">
        <label>Maßstab 1 :<input type="number" min=".001" step="any" bind:value={nenner}/></label>
        <button type="button" class="sekundaer" onclick={()=>{if(aktueller)aktueller.kalibrierung={art:"massstab",nenner};else p.kalibrierung={art:"massstab",nenner}}}>Maßstab setzen</button>
        <label>Referenzstrecke (m)<input type="number" min=".001" step="any" bind:value={referenz}/></label>
        <button type="button" class:aktiv={werkzeug==="kalibrieren"} onclick={()=>werkzeug="kalibrieren"}>Zwei Punkte kalibrieren</button>
        <span class="meta">{!p.kalibrierung?"Maßstab fehlt":massstabText(p.kalibrierung)}{aktueller?` · gilt für „${aktueller.bezeichnung}“`:""}</span>
      </div>
      <p class="meta">Blattweiter Höhenbezug nur für Blätter ohne Ausschnitte: Höhen werden dann senkrecht zur Seite gemessen. Bei mehreren Schnitten je Blatt die Bezugslinie je Ausschnitt setzen.</p>
      <div class="kalibrierung"><label>Höhensystem<input bind:value={hsystem}/></label><label>Bezugshöhe (m)<input type="number" step="any" bind:value={hwert}/></label>
        <button type="button" class="sekundaer" onclick={()=>werkzeug="bezug"}>Bezugspunkt setzen</button>
        <span>{p.hoehenbezug?`${z(p.hoehenbezug.hoehe_m)} ${p.hoehenbezug.system}`:"Noch kein Höhenbezug des Blattes"}</span>
      </div>
    </details>
    {#if pdfFehler}<p class="fehlertext" role="alert">{pdfFehler}</p>{/if}
    <div class="editor" class:vollbild bind:this={editor} style:height={vollbild?undefined:`${hoeheEditor}px`}>
      <div class="werkzeuge" role="toolbar" aria-label="PDF-Zeichenwerkzeuge">
        <span class="pfad" aria-label="Arbeitsbereich">
          <button type="button" class="link" disabled={!aktiv} onclick={schliessen}>Blatt {p.seite}</button>
          {#if aktueller}<span aria-hidden="true">›</span><strong>{aktueller.bezeichnung}</strong>{/if}
        </span>
        {#each werkzeuge as [id,titel,kuerzel]}<button type="button" class:aktiv={werkzeug===id} aria-pressed={werkzeug===id} disabled={id==="hoehenpunkt"&&!hoehenMoeglich} title={id==="hoehenpunkt"&&!hoehenMoeglich?"Höhen nur in Schnitt, Ansicht oder Detail":`Taste ${kuerzel}`} onclick={()=>{werkzeug=id;vorbelegung=null}}>{titel}</button>{/each}
        <button type="button" class="sekundaer" disabled={!undo.length} title="Strg+Z" onclick={rueckwaerts}>Rückgängig</button>
        <button type="button" class="sekundaer" disabled={!redo.length} title="Strg+Y" onclick={vorwaerts}>Wiederholen</button>
        <label class="umschalter"><input type="checkbox" bind:checked={masse}/>Maße anzeigen</label>
        <button type="button" class="sekundaer rechts" aria-pressed={vollbild} title="F11" onclick={()=>vollbild=!vollbild}>{vollbild?"Vollbild beenden":"Vollbild"}</button>
      </div>
      {#if vorbelegung}<p role="status" class="hinweisbox schmal">Zeichnen für <strong>{vorbelegung.gruppe} · {vorbelegung.titel}</strong> ({bedeutungen.find(b=>b[0]===vorbelegung!.bedeutung)?.[1]}){vorbelegung.hinweis?` · ${vorbelegung.hinweis}`:""}. Bedeutung und Zuordnung werden gesetzt. <button type="button" class="link" onclick={()=>{vorbelegung=null;werkzeug="auswahl"}}>Abbrechen</button></p>{/if}
      {#if werkzeug==="kalibrieren"||werkzeug==="bezug"||werkzeug==="bezugslinie"||werkzeug==="ausschnitt"}<p role="status" class="hinweisbox schmal">{werkzeug==="kalibrieren"?"Anfang und Ende der bekannten Referenzstrecke anklicken.":werkzeug==="bezug"?"Den Punkt mit der angegebenen Bezugshöhe auf der PDF anklicken.":werkzeug==="bezugslinie"?`Bezugslinie (z. B. ±0,00) anklicken: Anfang und Ende der Linie; Höhe ${z(hwert)} ${hsystem}.`:"Rahmen um die Zeichnung aufziehen: zwei gegenüberliegende Ecken anklicken."}</p>{/if}
      <div class="arbeitsplatz">
        <div class="flaeche">
          {#if pdf}
            <PdfPlan bind:this={plan} pdfBase64={pdf.pdf_base64} seite={p.seite} markierungen={p.markierungen} ebenen={p.ebenen} {auswahl} {werkzeug} darf={darf&&!s.laeuft}
              ausschnitte={p.ausschnitte??[]} {aktiv} {rahmenAuswahl} werte={aktuelleWerte} kalibrierung={p.kalibrierung} {masse}
              onfertig={fertig} onauswahl={id=>auswahl=id} onedit={(id,g)=>{if(!objektGesperrt){merken();const item=p.markierungen.find(m=>m.id===id);if(item)item.geometrie=g}}}
              onrahmen={(id,von,bis)=>{const a=p.ausschnitte?.find(a=>a.id===id);if(a){a.von=von;a.bis=bis}}}
              onrahmenauswahl={id=>rahmenAuswahl=id} onoeffnen={oeffnen} onseite={seiteWechseln}
              onfehler={e=>pdfFehler=e}/>
          {:else}<p class="meta">{laedt?"PDF wird geladen …":"Eine PDF-Unterlage auswählen."}</p>{/if}
          {#if messFehler}<p class="fehlertext" role="alert">Aufmaß prüfen: {messFehler}</p>{/if}
        </div>
        <aside>
          {#if s.id}<Planaufgaben projektId={projekt.id} plan={p} aufmassId={s.id} gespeichert={s.original!==null&&!s.hatAenderungen()} {darf} onzeichnen={aufgabeZeichnen} onwaehlen={id=>{auswahl=id;werkzeug="auswahl"}}/>{/if}
          <h3>Ausschnitte <span class="meta">({ausschnitte.length})</span></h3>
          <p class="meta">Je Zeichnung auf dem Blatt ein Ausschnitt. Doppelklick auf einen Rahmen oder „Öffnen“ zeigt nur diesen Bereich; neue Markierungen gehören dann dazu.</p>
          <div class="ausschnitte">
            {#each ausschnitte as a (a.id)}
              <div class="ausschnitt" class:aktiv={a.id===aktiv} class:gewaehlt={a.id===rahmenAuswahl}>
                <button type="button" class="link" onclick={()=>{if(aktiv===a.id)return;rahmenAuswahl=a.id}} aria-label={`Ausschnitt ${a.bezeichnung} wählen`}>{a.bezeichnung}
                  <span>{arten.find(x=>x[0]===a.art)?.[1]} · {a.kalibrierung?massstabText(a.kalibrierung):p.kalibrierung?"Blattmaßstab":"Maßstab fehlt"}{mitHoehen(a.art)?` · ${a.bezugslinie?`Bezug ${z(a.bezugslinie.hoehe_m)} ${a.bezugslinie.system}`:"Bezugslinie fehlt"}`:""} · {anzahl(a).length} Markierungen</span></button>
                {#if a.id===aktiv}<button type="button" class="sekundaer" onclick={schliessen}>Schließen</button>{:else}<button type="button" class="sekundaer" onclick={()=>oeffnen(a.id)}>Öffnen</button>{/if}
              </div>
            {/each}
          </div>
          {#if !aktiv}<button type="button" class:aktiv={werkzeug==="ausschnitt"} aria-pressed={werkzeug==="ausschnitt"} disabled={!darf||!pdf} onclick={()=>{werkzeug="ausschnitt";rahmenAuswahl=""}}>Ausschnitt anlegen</button>{/if}
          {#if bearbeiteter}
            {@const a=bearbeiteter}
            <fieldset disabled={!darf||s.laeuft} class="ausschnitt-eigenschaften">
              <legend>Ausschnitt „{a.bezeichnung}“</legend>
              <label>Name<input aria-label="Ausschnittname" value={a.bezeichnung} required oninput={e=>umbenennen(a,e.currentTarget.value)}/></label>
              <label>Art<select aria-label="Ausschnittart" bind:value={a.art} onchange={()=>{if(!mitHoehen(a.art))delete a.bezugslinie}}>{#each arten as [v,t]}<option value={v}>{t}</option>{/each}</select></label>
              <div class="zeile">
                <span>Maßstab: {a.kalibrierung?massstabText(a.kalibrierung):p.kalibrierung?`Blatt (${massstabText(p.kalibrierung)})`:"fehlt"}</span>
                <button type="button" class="link" onclick={()=>{a.kalibrierung={art:"massstab",nenner}}}>1 : {z(nenner)} setzen</button>
                <button type="button" class="link" onclick={()=>{if(aktiv!==a.id)oeffnen(a.id);werkzeug="kalibrieren"}}>Zwei Punkte kalibrieren</button>
                {#if a.kalibrierung}<button type="button" class="link" onclick={()=>{delete a.kalibrierung}}>Blattmaßstab verwenden</button>{/if}
              </div>
              {#if mitHoehen(a.art)}
                <div class="zeile">
                  <span>Bezugslinie: {a.bezugslinie?`${z(a.bezugslinie.hoehe_m)} ${a.bezugslinie.system}`:"fehlt – ±0,00-Linie des Schnitts anklicken"}</span>
                  <label>Höhe (m)<input type="number" step="any" bind:value={hwert}/></label>
                  <label>System<input bind:value={hsystem}/></label>
                  {#each nullpunkte as n}<button type="button" class="link" onclick={()=>{hwert=n.wert;hsystem="m ü. NHN"}}>Nullpunkt {n.name} ({z(n.wert)} m ü. NHN)</button>{/each}
                  <button type="button" class:aktiv={werkzeug==="bezugslinie"} onclick={()=>{if(aktiv!==a.id)oeffnen(a.id);werkzeug="bezugslinie"}}>Bezugslinie setzen</button>
                  {#if a.bezugslinie}<button type="button" class="link" onclick={()=>{delete a.bezugslinie}}>Bezugslinie entfernen</button>{/if}
                </div>
              {:else}<p class="meta">Grundriss und Lageplan messen Flächen und Längen; Höhen stammen aus Schnitten, Ansichten oder Details.</p>{/if}
              <div class="aktionen">
                <button type="button" class="sekundaer" onclick={()=>ausschnittDuplizieren(a)}>Duplizieren</button>
                <button type="button" class="sekundaer" onclick={()=>void ausschnittEntfernen(a)}>Entfernen</button>
              </div>
            </fieldset>
          {/if}
          <h3>Zeichenebenen</h3>
          {#each p.ebenen as e (e.id)}
            <div class="ebene"><input type="radio" aria-label={`Auf Ebene ${e.bezeichnung} zeichnen`} name={`ebene-${s.id}`} value={e.id} bind:group={ebene}/>
              <input aria-label="Ebenenname" bind:value={e.bezeichnung} required/>
              <label title="Sichtbar"><input type="checkbox" bind:checked={e.sichtbar}/>An</label>
              <label title="Gesperrt"><input type="checkbox" bind:checked={e.gesperrt}/>Sperre</label>
            </div>
          {/each}
          <button type="button" class="sekundaer" onclick={()=>{const id=crypto.randomUUID();p.ebenen.push({id,bezeichnung:`Ebene ${p.ebenen.length+1}`,sichtbar:true,gesperrt:false});ebene=id}}>Ebene hinzufügen</button>
          <h3>Markierungen <span class="meta">({aktiv?`${anzahl(aktueller!).length} von `:""}{p.markierungen.length})</span></h3>
          <div class="markierungen">
            {#each p.markierungen.filter(i=>!aktiv||i.ausschnitt_id===aktiv) as item (item.id)}<button type="button" class:aktiv={item.id===auswahl} onclick={()=>{auswahl=item.id;werkzeug="auswahl"}}>{item.bezeichnung}<span>{bedeutungen.find(b=>b[0]===item.bedeutung)?.[1]}{!aktiv&&item.ausschnitt_id?` · ${ausschnitte.find(a=>a.id===item.ausschnitt_id)?.bezeichnung??"Ausschnitt fehlt"}`:""}</span></button>{/each}
            {#if !p.markierungen.length}<p class="meta">Ein Zeichenwerkzeug wählen (Tasten P, R, K, S, H) und Punkte auf dem Plan setzen.</p>{/if}
          </div>
          {#if m}
            <h3>Eigenschaften</h3>
            <fieldset disabled={objektGesperrt}>
              <label>Bezeichnung<input bind:value={m.bezeichnung} required/></label>
              <label>Bedeutung<select aria-label="Bedeutung" bind:value={m.bedeutung}>{#each bedeutungen as [v,t]}<option value={v}>{t}</option>{/each}</select></label>
              {#if ausschnitte.length}<label>Ausschnitt<select aria-label="Ausschnitt der Markierung" value={m.ausschnitt_id??""} onchange={e=>{const v=e.currentTarget.value;if(v)m.ausschnitt_id=v;else delete m.ausschnitt_id}}><option value="">Ganzes Blatt</option>{#each ausschnitte as a}<option value={a.id}>{a.bezeichnung}</option>{/each}</select></label>{/if}
              <label>Zeichenebene<select bind:value={m.ebene_id}>{#each p.ebenen as e}<option value={e.id}>{e.bezeichnung}</option>{/each}</select></label>
              <label>Baugrundstück<select bind:value={m.grundstueck_id}><option value={null}>Nicht zugeordnet</option>{#each grund as g}<option value={g.id}>{g.bezeichnung}</option>{/each}</select></label>
              <label>Gebäude<select bind:value={m.gebaeude_id} onchange={()=>{if(m)m.geschoss_id=null}}><option value={null}>Nicht zugeordnet</option>{#each gebaeude as g}<option value={g.objekt}>{g.daten.bezeichnung}</option>{/each}</select></label>
              {#if m.gebaeude_id}<label>Geschoss<select bind:value={m.geschoss_id}><option value={null}>Nicht zugeordnet</option>{#each gebaeude.find(g=>g.objekt===m.gebaeude_id)?.daten.geschosse??[] as g,i}<option value={g.id}>{g.bezeichnung||`Geschoss ${i+1} (ohne Bezeichnung)`}</option>{/each}</select></label>{/if}
              {#if (m.bedeutung==="gebaeudegrundflaeche"||m.bedeutung==="geschoss_aussenkante")&&!m.gebaeude_id}<p class="hinweisbox">Für die Übernahme ein Gebäude zuordnen.{gebaeude.length?"":" Gebäude zuerst unter Grundlagen → Gebäude / Geschosse anlegen und speichern."}</p>
              {:else if m.bedeutung==="geschoss_aussenkante"&&!m.geschoss_id}<p class="hinweisbox">Für die Geschossfläche ein Geschoss zuordnen.{gebaeude.find(g=>g.objekt===m.gebaeude_id)?.daten.geschosse.length?"":" Für dieses Gebäude sind noch keine Geschosse gespeichert; ohne einzelne Geschosse dient die Gebäudegrundfläche als Fläche des Einzelgeschosses."}</p>
              {:else if m.bedeutung==="gebaeudegrundflaeche"&&m.geschoss_id}<p class="hinweisbox">Die Gebäudegrundfläche gilt für das ganze Gebäude; die Geschosszuordnung wird dafür nicht verwendet. Für eine Geschossfläche die Markierung duplizieren und die Kopie als „Geschoss – Außenkante“ kennzeichnen.</p>{/if}
              <div class="aktionen">
                <button type="button" class="sekundaer" title="Strg+D · Kopie derselben Kontur, z. B. als Gebäudegrundfläche und Geschoss-Außenkante" onclick={duplizieren}>Duplizieren</button>
                <button type="button" class="sekundaer" title="Entf" onclick={entfernen}>Markierung entfernen</button>
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
            <p class="meta">Nach dem Speichern: {ziel(m.bedeutung)} Eckpunkte lassen sich mit „Auswählen“ ziehen, die ganze Markierung durch Ziehen der Fläche oder mit den Pfeiltasten.</p>
          {/if}
        </aside>
      </div>
    </div>
  {/snippet}
</Fachliste>

<style>
  .quellzeile{display:grid;grid-template-columns:1fr 2fr 6rem;gap:1rem;margin-bottom:1rem}
  .einrichtung{margin-bottom:.5rem}
  .einrichtung summary{cursor:pointer}
  .kalibrierung,.werkzeuge{display:flex;gap:.6rem;flex-wrap:wrap;align-items:end;margin:.5rem 0}
  .kalibrierung input[type=number]{width:8rem}
  .werkzeuge{align-items:center;margin:0 0 .4rem}
  .werkzeuge .rechts{margin-left:auto}
  .werkzeuge .umschalter{display:flex;gap:.3rem;align-items:center;font-size:.85rem}
  .pfad{display:flex;gap:.4rem;align-items:center;margin-right:.6rem;white-space:nowrap}
  .editor{display:flex;flex-direction:column;min-height:480px}
  .editor.vollbild{position:fixed;inset:0;z-index:40;background:var(--bg,#f4f6f8);padding:.5rem;height:auto}
  .arbeitsplatz{display:grid;grid-template-columns:minmax(0,1fr) 320px;gap:1rem;flex:1;min-height:0}
  .flaeche{display:flex;flex-direction:column;min-height:0}
  aside{border-left:1px solid var(--linie,#ccd5df);padding-left:1rem;overflow:auto;min-height:0}
  aside h3{margin:.8rem 0 .3rem}
  .schmal{margin:0 0 .4rem}
  .ausschnitte{display:grid;gap:.3rem}
  .ausschnitt{display:flex;gap:.4rem;align-items:center;padding:.25rem .4rem;border:1px solid transparent;border-radius:4px}
  .ausschnitt.gewaehlt{border-color:#5a3d8f}
  .ausschnitt.aktiv{background:#5a3d8f14;border-color:#5a3d8f}
  .ausschnitt>button.link{flex:1;text-align:left;font-weight:600}
  .ausschnitt>button.link span{display:block;font-size:.75rem;font-weight:400}
  .ausschnitt-eigenschaften{margin:.6rem 0;padding:.5rem;border:1px solid var(--linie,#ccd5df);border-radius:4px}
  .ausschnitt-eigenschaften legend{font-weight:600;padding:0 .3rem}
  .ausschnitt-eigenschaften .zeile{display:flex;flex-wrap:wrap;gap:.3rem .7rem;align-items:center}
  .ausschnitt-eigenschaften .zeile input{width:7rem}
  .ebene{display:flex;gap:.3rem;align-items:center;margin-bottom:.5rem}
  .ebene>input:not([type=radio]){min-width:3rem;width:100%}
  .ebene label{font-size:.75rem;white-space:nowrap}
  .markierungen{max-height:220px;overflow:auto;display:grid;gap:.25rem}
  .markierungen button{text-align:left}.markierungen span{display:block;font-size:.75rem;font-weight:400}
  .messwerte{display:grid;grid-template-columns:1fr auto;gap:.4rem;font-variant-numeric:tabular-nums}.messwerte dd{margin:0;font-weight:600}
  fieldset{border:0;padding:0;display:grid;gap:.5rem}
  .aktionen{display:flex;gap:.5rem;flex-wrap:wrap}
  @media(max-width:1100px){.arbeitsplatz{grid-template-columns:1fr}aside{border-left:0;padding-left:0;max-height:40vh}.quellzeile{grid-template-columns:1fr}}
</style>
