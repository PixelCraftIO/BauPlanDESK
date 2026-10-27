<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { ask, save } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext, EINHEITEN, istDxf, istPlan, type Project, type DocumentVersion, type PdfVorschauDaten, type GrundstueckKontext, type CadPlan, type BaugrenzenPruefung } from "../api";
  import type { Planaufmass, Planmarkierung, Planbedeutung, Planmesswert, Plangeometrie, Planausschnitt, Ausschnittart, Planaufgabe, Gebaeude, Laengeneinheit } from "../generated/plattform";
  import { FachSitzung } from "../FachSitzung.svelte";
  import Fachliste from "../Fachliste.svelte";
  import { plattformApi, type Datensatz } from "../plattformApi";
  import { ansichtsAbfrage } from "../ansichtsAbfrage.svelte";
  import PdfPlan from "./PdfPlan.svelte";
  import Planaufgaben from "./Planaufgaben.svelte";
  import { planApi } from "./api";
  import { verschieben, eckpunkte, meterProPunkt, type Point } from "./pdfGeometry";
  import { Bearbeitungsverlauf } from "./verlauf";
  import { drehen, punktAendern, punktEinfuegen, punktEntfernen, kantenmass, ausrichten } from "./bearbeitung";
  import { aufmassUebertragen } from "./uebertragung";
  import { grundstuecksinformationen } from "../grundstuecksdaten";
  import { ABZUGSBEDEUTUNGEN, BAUTEILBEDEUTUNGEN } from "./grundstueck";
  let {projekt,benutzerId,darf,onprojekt,unterlage=null}:{projekt:Project;benutzerId:string;darf:boolean;onprojekt?:(p:Project)=>void;unterlage?:{dokumentId:string;nummer:number}|null}=$props();
  // Flurstücke des Projekts für die Zuordnung von Flurstücksmarkierungen; Stand der Akte.
  const flurstuecke=$derived(grundstuecksinformationen(projekt.planungsrecht).flurstuecke);
  let dateien=$state<DocumentVersion[]>([]), pdf=$state<PdfVorschauDaten|null>(null), pdfFehler=$state(""), grund=$state<GrundstueckKontext[]>([]);
  let gebaeude=$state<Datensatz<Gebaeude>[]>([]),listenFehler=$state(""),laedt=$state(false);
  // DXF-Unterlage: Zeichnung samt Ebenensichtbarkeit (nur Anzeige und Fang).
  let dxf=$state<CadPlan|null>(null), dxfSichtbar=$state<boolean[]>([]);
  let baugrenzenLaeuft=$state(false), baugrenzenErgebnis=$state<BaugrenzenPruefung|null>(null);
  const s=untrack(()=>new FachSitzung<Planaufmass>(projekt.id,"planaufmass",benutzerId,()=>({
    bezeichnung:"Neues Aufmaß",dokument:{dokument_id:dateien[0]?.document_id??"00000000-0000-0000-0000-000000000000",version:dateien[0]?.version??1},
    seite:1,kalibrierung:null,hoehenbezug:null,ebenen:[{id:crypto.randomUUID(),bezeichnung:"Aufmaß",sichtbar:true,gesperrt:false}],markierungen:[],
  }),p=>p.bezeichnung));
  const bedeutungen: [Planbedeutung,string][]=[
    ["unbestimmt","Noch nicht zugeordnet"],["flurstueck","Flurstück (Flurkarte / Lageplan)"],["baufenster","Baufenster (Baugrenze / Baulinie)"],["vorhabenumriss","Vorhabenumriss (Gebäude im Lageplan)"],["private_gruenflaeche","Private Grünfläche"],
    ["oeffentliche_gruenflaeche","Öffentliche Grünfläche"],["verkehrsflaeche","Verkehrsfläche"],["ausserhalb_bauland","Fläche außerhalb des Baulands"],["sonstiger_abzug","Sonstige nicht maßgebende Fläche"],
    ["zufahrt","Zufahrt"],["stellplatz","Stellplatz"],["garage","Garage / Carport"],["nebenanlage","Nebenanlage (§ 14 BauNVO)"],["terrasse","Terrasse"],["unterirdisch","Unterirdische Anlage"],
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
  const verlauf=new Bearbeitungsverlauf<Planaufmass>();
  let undo=$state(0),redo=$state(0),verlaufGruppe="";
  let auswahlIds=$state<string[]>([]),punktIndex=$state(0),drehenGrad=$state(0),kanteMeter=$state(1),kanteWinkel=$state(0);
  let suche=$state(""),bedeutungFilter=$state(""),gebaeudeFilter=$state(""),geschossFilter=$state(""),nurFehler=$state(false),panel=$state("aufgaben"),hilfe=$state(false);
  let markierungsFehler=$state<{id:string;fehler:string}[]>([]);
  let kontrollMeter=$state(10),kontrolle=$state<{gemessen:number;soll:number;stand:string}|null>(null);
  let vergleich=$state(""),vergleichPdf=$state<PdfVorschauDaten|null>(null),vergleichDeckkraft=$state(.5);
  let vergleichAusrichtung=$state({dx:0,dy:0,grad:0,faktor:1}),vergleichBestaetigt=$state(false);
  let standardGebaeude=$state<string|null>(null),standardGeschoss=$state<string|null>(null),standardBedeutung=$state<Planbedeutung>("unbestimmt");
  let exportLaeuft=$state(false),nurSichtbare=$state(true);
  let betrachterSeite=$state<number|null>(null),sichtbarkeit=$state<Record<string,boolean>>({});
  let aktiv=$state(""),rahmenAuswahl=$state(""),vollbild=$state(false),masse=$state(true),hoeheEditor=$state(560),einrichtungOffen=$state(true);
  // Aus den Aufgaben vorbelegte Bedeutung und Zuordnung für die nächste Markierung.
  let vorbelegung=$state<Planaufgabe|null>(null);
  let editor=$state<HTMLDivElement>(), plan=$state<ReturnType<typeof PdfPlan>>();
  const m=$derived(s.eingabe?.markierungen.find(m=>m.id===auswahl));
  const layer=$derived(s.eingabe?.ebenen.find(e=>e.id===ebene));
  const objektGesperrt=$derived(!darf||s.laeuft||!!s.eingabe?.ebenen.find(e=>e.id===m?.ebene_id)?.gesperrt);
  const quelleAlt=$derived(s.eingabe&&dateien.some(d=>d.document_id===s.eingabe!.dokument.dokument_id&&d.version>s.eingabe!.dokument.version));
  const aktuelleDatei=$derived(dateien.find(d=>d.document_id===s.eingabe?.dokument.dokument_id&&d.version===s.eingabe?.dokument.version));
  const istDxfUnterlage=$derived(!!aktuelleDatei&&istDxf(aktuelleDatei.file_name));
  const pdfDateien=$derived(dateien.filter(d=>!istDxf(d.file_name)));
  const signatur=$derived(JSON.stringify(s.eingabe));
  const aktuelleWerte=$derived(signatur===messStand?werte:[]);
  const fehlerMap=$derived(new Map(markierungsFehler.map(f=>[f.id,f.fehler])));
  const gefilterteMarkierungen=$derived((s.eingabe?.markierungen??[]).filter(i=>(!aktiv||i.ausschnitt_id===aktiv)&&(!suche||i.bezeichnung.toLocaleLowerCase("de").includes(suche.toLocaleLowerCase("de")))&&(!bedeutungFilter||i.bedeutung===bedeutungFilter)&&(!gebaeudeFilter||i.gebaeude_id===gebaeudeFilter)&&(!geschossFilter||i.geschoss_id===geschossFilter)&&(!nurFehler||fehlerMap.has(i.id))));
  const wert=$derived(aktuelleWerte.find(w=>w.id===auswahl));
  const ausschnitte=$derived(s.eingabe?.ausschnitte??[]);
  const aktueller=$derived(ausschnitte.find(a=>a.id===aktiv));
  const kontrollStand=$derived(JSON.stringify([s.eingabe?.dokument,s.eingabe?.seite,aktueller?.id,aktueller?.kalibrierung??s.eingabe?.kalibrierung]));
  $effect(()=>{const stand=kontrollStand;untrack(()=>{if(kontrolle&&kontrolle.stand!==stand)kontrolle=null})});
  // Eigenschaften gelten für den geöffneten, sonst für den im Blatt gewählten Ausschnitt.
  const bearbeiteter=$derived(aktueller??ausschnitte.find(a=>a.id===rahmenAuswahl));
  const hoehenMoeglich=$derived(!aktueller||mitHoehen(aktueller.art));
  const nullpunkte=$derived(gebaeude.flatMap(g=>g.daten.geometrie?.nullpunkt_m_nhn!=null?[{name:g.daten.bezeichnung,wert:g.daten.geometrie.nullpunkt_m_nhn}]:[]));
  let listenGeneration=0;
  // Je Projekt einmal: CAD-Aufmaße der früheren DXF-Ansicht als Aufmaße übernehmen (nur mit Schreibrecht).
  const migriert=new Set<string>();
  async function listen(){
    const n=++listenGeneration;
    const extra:string[]=[];
    if(darf&&!migriert.has(s.projektId)){
      migriert.add(s.projektId);
      try{const r=await planApi.cadUebernehmen(s.projektId);if(n!==listenGeneration)return;
        if(r.uebernommen.length){s.meldung=`Aus der früheren DXF-Ansicht übernommen: ${r.uebernommen.join(", ")}. Bedeutungen und Zuordnungen dort ergänzen.`;void s.laden()}
        if(r.uebersprungen.length)extra.push(`CAD-Aufmaße nicht übernommen: ${r.uebersprungen.join(" · ")}`);
      }catch(e){if(n!==listenGeneration)return;extra.push(fehlertext(e))}
    }
    const [d,g,b]=await Promise.allSettled([api.dokumente(s.projektId),api.grundstueckskontext(s.projektId),plattformApi.liste<Gebaeude>(s.projektId,"gebaeude")]);
    if(n!==listenGeneration)return;
    if(d.status==="fulfilled")dateien=d.value.filter(d=>istPlan(d.file_name)).sort((a,b)=>a.file_name.localeCompare(b.file_name,"de")||b.version-a.version);
    grund=g.status==="fulfilled"?g.value:[];
    gebaeude=b.status==="fulfilled"?b.value:[];
    listenFehler=[...extra,...[d,g,b].flatMap(r=>r.status==="rejected"?[fehlertext(r.reason)]:[])].join(" · ");
  }
  // Von außen gewünschte Unterlage (Projektdateien „Im Aufmaß öffnen“, Projektstart, Projektübersicht).
  let unterlageNummer=0;
  $effect(()=>{
    const w=unterlage, liste=dateien, bereit=s.bereit;
    untrack(()=>{if(w&&bereit&&liste.length&&w.nummer!==unterlageNummer){unterlageNummer=w.nummer;void unterlageOeffnen(w.dokumentId)}});
  });
  async function unterlageOeffnen(id:string){
    const d=dateien.filter(x=>x.document_id===id).sort((a,b)=>b.version-a.version)[0];
    if(!d){s.meldung="Die gewünschte Unterlage ist kein PDF- oder DXF-Dokument dieses Projekts.";return}
    // Bestehendes Aufmaß dieser Unterlage (neueste Dokumentversion zuerst), sonst neuer Entwurf.
    const vorhanden=s.liste.filter(f=>f.daten.dokument.dokument_id===id).sort((a,b)=>b.daten.dokument.version-a.daten.dokument.version)[0];
    if(vorhanden&&s.id===vorhanden.objekt)return;
    if(s.eingabe&&s.hatAenderungen()){
      if(!(await ask(`„${d.file_name}“ im Aufmaß öffnen? Offene Änderungen werden zuerst gespeichert.`,{title:"Unterlage öffnen",okLabel:"Speichern und öffnen",cancelLabel:"Hier bleiben"})))return;
      if(!(await s.speichern()))return;
    }
    if(vorhanden){s.bearbeiten(vorhanden);return}
    s.neu();if(!s.eingabe)return;
    s.eingabe={...s.leer(),bezeichnung:istDxf(d.file_name)?d.file_name:`${d.file_name} · Seite 1`,dokument:{dokument_id:d.document_id,version:d.version},seite:1};
    einrichtungOffen=true;
  }
  async function baugrenzenZuordnen(){
    if(!s.eingabe||!s.id||!darf||baugrenzenLaeuft)return;
    if(s.hatAenderungen()&&(!(await ask("Für die Zuordnung den aktuellen Aufmaßentwurf zuerst speichern?",{title:"Lage im Baufenster",okLabel:"Speichern und zuordnen",cancelLabel:"Abbrechen"}))||!(await s.speichern())))return;
    baugrenzenLaeuft=true;baugrenzenErgebnis=null;
    try{
      const r=await planApi.baugrenzen(s.projektId,{aufmass_id:s.id,revision:s.revision});
      onprojekt?.(r.projekt);baugrenzenErgebnis=r.pruefung;
      s.meldung=r.pruefung.eingehalten?"Lage im Baufenster dem Projekt zugeordnet: Das Vorhaben liegt im Baufenster.":`Lage im Baufenster dem Projekt zugeordnet: ${z(r.pruefung.ausserhalb_m2)} m² (${z(r.pruefung.ausserhalb_prozent)} %) liegen außerhalb.`;
    }catch(e){s.fehler=fehlertext(e)}finally{baugrenzenLaeuft=false}
  }
  onMount(()=>()=>{listenGeneration++});
  const vormerken=ansichtsAbfrage(()=>s.projektId,["dokumente","fachobjekte"],listen);
  $effect(()=>{
    void projekt.version;untrack(vormerken);
  });
  $effect(()=>{
    const id=s.id;
    untrack(()=>{auswahl="";auswahlIds=[];betrachterSeite=null;sichtbarkeit={};ebene=s.eingabe?.ebenen[0]?.id??"";undo=0;redo=0;if(s.eingabe)verlauf.reset($state.snapshot(s.eingabe));werkzeug="auswahl";aktiv="";rahmenAuswahl="";einrichtungOffen=!s.eingabe?.kalibrierung;
      try{const v=JSON.parse(localStorage.getItem(`bauprojekt.planpanel.${benutzerId}.${s.projektId}.${id}`)??"null");if(v){panel=["aufgaben","objekte","eigenschaften"].includes(v.panel)?v.panel:"aufgaben";suche=typeof v.suche==="string"?v.suche.slice(0,500):"";aktiv=v.aktiv??"";auswahl=v.auswahl??"";auswahlIds=auswahl?[auswahl]:[];bedeutungFilter=v.bedeutungFilter??"";gebaeudeFilter=v.gebaeudeFilter??"";geschossFilter=v.geschossFilter??"";nurFehler=v.nurFehler===true}}catch{}
    });
  });
  $effect(()=>{const sig=signatur;if(!sig||sig==="null")return;untrack(()=>{verlauf.beobachtenSignatur(sig,verlaufGruppe);undo=verlauf.zurueck;redo=verlauf.vor})});
  $effect(()=>{const v={panel,suche,aktiv,auswahl,bedeutungFilter,gebaeudeFilter,geschossFilter,nurFehler},id=s.id;try{localStorage.setItem(`bauprojekt.planpanel.${benutzerId}.${s.projektId}.${id}`,JSON.stringify(v))}catch{}});
  $effect(()=>{const q=vergleich;let gone=false;vergleichPdf=null;if(q){const [id,v]=q.split("|");void api.pdfVorschau(id,Number(v)).then(p=>{if(!gone)vergleichPdf=p}).catch(e=>{if(!gone)s.fehler=fehlertext(e)})}return()=>{gone=true}});
  $effect(()=>{
    const d=s.eingabe?.dokument, datei=aktuelleDatei;
    let gone=false;pdf=null;dxf=null;pdfFehler="";
    if(!d||!datei)return;
    laedt=true;
    if(istDxf(datei.file_name)){
      void api.cadPlan(d.dokument_id,d.version).then(c=>{
        if(gone)return;
        dxf=c;dxfSichtbar=c.plan.ebenen.map(e=>e.sichtbar);
        // Neuer Entwurf ohne Kalibrierung: die im DXF-Kopf genannte Einheit gilt; nennt die Datei keine, bleibt die Wahl offen.
        const p=untrack(()=>s.eingabe);
        if(p&&darf&&!p.kalibrierung&&c.plan.einheit&&untrack(()=>s.revision)===0)p.kalibrierung={art:"einheit",einheit:c.plan.einheit};
      }).catch(e=>{if(!gone)pdfFehler=fehlertext(e)}).finally(()=>{if(!gone)laedt=false});
    }else{
      void api.pdfVorschau(d.dokument_id,d.version).then(p=>{if(!gone)pdf=p}).catch(e=>{if(!gone)pdfFehler=fehlertext(e)}).finally(()=>{if(!gone)laedt=false});
    }
    return()=>{gone=true};
  });
  $effect(()=>{
    const sig=signatur;
    let gone=false;messFehler="";messLaeuft=!!s.eingabe;
    const timer=setTimeout(()=>{
      if(!s.eingabe)return;
      const plan=JSON.parse(sig) as Planaufmass;
      void planApi.diagnose(s.projektId,plan).then(w=>{if(!gone){werte=w.markierungen.flatMap(m=>m.wert?[m.wert]:[]);markierungsFehler=w.markierungen.flatMap(m=>m.fehler?[{id:m.id,fehler:m.fehler}]:[]);messStand=sig}})
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
        if(k==="z"){e.preventDefault();if(e.shiftKey)vorwaerts();else rueckwaerts()}
        else if(k==="y"){e.preventDefault();vorwaerts()}
        else if(k==="d"){e.preventDefault();duplizieren()}
        return;
      }
      if(e.altKey)return;
      if(e.key==="F1"){e.preventDefault();hilfe=!hilfe;return}
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
        gemeinsamVerschieben(auswahlIds.length?auswahlIds:[m.id],dx,dy);
      }
      else if(e.key==="PageDown"||e.key==="PageUp"){
        if(!ausschnitte.length)return;
        e.preventDefault();
        const i=ausschnitte.findIndex(a=>a.id===aktiv);
        const n=i<0?(e.key==="PageDown"?0:ausschnitte.length-1):(i+(e.key==="PageDown"?1:-1)+ausschnitte.length)%ausschnitte.length;
        oeffnen(ausschnitte[n].id);
      }
      else if(e.key==="F11"){e.preventDefault();vollbild=!vollbild}
    };
    window.addEventListener("keydown",taste);
    return()=>window.removeEventListener("keydown",taste);
  });
  // Wohin eine gespeicherte Markierung übernommen wird; nur Bedienhinweis.
  function ziel(b:Planbedeutung){
    if(b==="gebaeudegrundflaeche")return "Grundlagen → Gebäude / Geschosse: am Feld „Gebäudegrundfläche“ über „Aus Aufmaß“ übernehmen. Von dort geht sie in GRZ des Baugrundstücks ein.";
    if(b==="geschoss_aussenkante")return "Grundlagen → Gebäude / Geschosse: im jeweiligen Geschoss über „Aus Aufmaß“ übernehmen. Von dort geht sie in GFZ, Geschossigkeit und Vollgeschossprüfung ein.";
    if(b==="raum"||b==="aufenthaltsraum"||b==="nuf"||b==="tf"||b==="vf")return "Berechnungen → Flächen- und Raumbuch: „Räume aus Aufmaß übernehmen“ (Gebäude und Geschoss zuordnen). DIN 277 und Wohnfläche können außerdem unter „Quellen / Übernahmen“ verknüpft werden.";
    if(b==="nullpunkt"||b==="fussboden"||b==="decke"||b==="fundamentsohle"||b==="hoehenmass")return "Gebäude / Geschosse → „Höhen und alle Verknüpfungen aus Aufmaß“ oder direkt hier unter „Aufgaben → Übernehmen“.";
    if(b==="gelaende"||b==="traufe"||b==="first"||b==="dach"||b==="aussenwand")return "Je nach Ziel: Abstandsflächen → Grundlagen → „Aufmaß verknüpfen“ (manuelle Wand), Gebäudehöhen → „Quellen / Übernahmen“ oder Gebäude / Geschosse → „Höhen und alle Verknüpfungen aus Aufmaß“.";
    if(ABZUGSBEDEUTUNGEN.includes(b))return "Berechnungen → Baugrundstück → Flächenabzüge: „Abzugsflächen aus Aufmaß übernehmen“ (Flurstück zuordnen). Die Abzugsart folgt der Bedeutung, die rechtliche Grundlage wird dort ergänzt.";
    if(BAUTEILBEDEUTUNGEN.includes(b))return "Berechnungen → Baugrundstück (GRZ): „Bauteile aus Aufmaß übernehmen“ (Baugrundstück zuordnen). Anrechnung nach Bauteiltyp und BauNVO-Fassung; die Hauptanlage kommt über die Gebäudegrundfläche.";
    if(b==="baufenster"||b==="vorhabenumriss")return "Hier im Aufmaß: „Lage im Baufenster → Baufenster und Vorhabenumriss dem Projekt zuordnen“ (je genau eine Markierung). Der GRZ-Nachweis zeigt dann die Lage im Baufenster.";
    if(b==="flurstueck")return "Grundlagen → Flurstücke: über „Aufgaben → Übernehmen“ (Flurstück zuordnen). Die Katasterfläche bleibt Rechenwert und zeigt die Abweichung; ohne amtlichen Wert kann die gezeichnete Fläche als Flurstücksfläche gelten.";
    if(b==="unbestimmt")return "Zuerst eine Bedeutung wählen; sie bestimmt, welche Berechnung den Messwert übernehmen kann.";
    return "In der Fachberechnung eine Position anlegen und unter „Quellen / Übernahmen → Aufmaß“ verknüpfen.";
  }
  function duplizieren(){
    if(!s.eingabe||!m||objektGesperrt)return;
    merken();
    const ms=s.eingabe.markierungen.filter(x=>(auswahlIds.length?auswahlIds:[m!.id]).includes(x.id));
    if(ms.some(m=>s.eingabe!.ebenen.find(e=>e.id===m.ebene_id)?.gesperrt))return;
    const ids=new Map(ms.map(m=>[m.id,crypto.randomUUID()]));
    const kopien=ms.map(m=>({...structuredClone($state.snapshot(m)),id:ids.get(m.id)!,bezeichnung:`${m.bezeichnung} (Kopie)`,hoehenzone_von:m.hoehenzone_von?ids.get(m.hoehenzone_von)??m.hoehenzone_von:undefined}));
    s.eingabe.markierungen.push(...kopien);auswahlIds=kopien.map(m=>m.id);auswahl=auswahlIds.at(-1)??"";
  }
  function entfernen(){
    if(!s.eingabe||!m||objektGesperrt)return;
    const ids=auswahlIds.length?auswahlIds:[auswahl];
    if(ids.some(id=>s.eingabe!.ebenen.find(e=>e.id===s.eingabe!.markierungen.find(m=>m.id===id)?.ebene_id)?.gesperrt))return;
    merken();s.eingabe.markierungen=s.eingabe.markierungen.filter(i=>!ids.includes(i.id));auswahl="";auswahlIds=[];
  }
  function merken(){verlaufGruppe="";verlauf.gruppeBeenden();if(s.eingabe)verlauf.beobachten($state.snapshot(s.eingabe))}
  function verlaufStatus(){undo=verlauf.zurueck;redo=verlauf.vor;const ids=new Set(s.eingabe?.markierungen.map(m=>m.id));auswahlIds=auswahlIds.filter(id=>ids.has(id));if(!ids.has(auswahl))auswahl="";if(!s.eingabe?.ausschnitte?.some(a=>a.id===aktiv))aktiv="";plan?.abbrechen()}
  function rueckwaerts(){
    if(!s.eingabe||!darf||s.laeuft)return;
    const p=verlauf.rueckwaerts($state.snapshot(s.eingabe));if(p)s.eingabe=p;verlaufStatus();
  }
  function vorwaerts(){
    if(!s.eingabe||!darf||s.laeuft)return;
    const p=verlauf.vorwaerts($state.snapshot(s.eingabe));if(p)s.eingabe=p;verlaufStatus();
  }
  function waehlen(id:string,mehrfach=false){betrachterSeite=null;if(mehrfach){auswahlIds=auswahlIds.includes(id)?auswahlIds.filter(x=>x!==id):[...auswahlIds,id];auswahl=auswahlIds.at(-1)??""}else{auswahl=id;auswahlIds=id?[id]:[]}punktIndex=0;werkzeug="auswahl"}
  function gemeinsamVerschieben(ids:string[],dx:number,dy:number){if(!s.eingabe||!darf||s.laeuft)return;const ms=s.eingabe.markierungen.filter(m=>ids.includes(m.id));if(ms.some(m=>s.eingabe!.ebenen.find(e=>e.id===m.ebene_id)?.gesperrt))return;merken();for(const m of ms)m.geometrie=verschieben(m.geometrie,dx,dy)}
  function geometrieSetzen(g:Plangeometrie){if(!m||objektGesperrt)return;merken();m.geometrie=g}
  async function flaechenAktion(operation:import("../generated/plattform").Flaechenoperation){
    if(!s.eingabe||!darf||auswahlIds.length!==2)return;
    const p=s.eingabe,ms=auswahlIds.map(id=>p.markierungen.find(m=>m.id===id)!);
    if(ms.some(m=>p.ebenen.find(e=>e.id===m.ebene_id)?.gesperrt)){s.fehler="Gesperrte Markierungen zuerst entsperren.";return}
    if(ms[0].ausschnitt_id!==ms[1].ausschnitt_id){s.fehler="Für eine Flächenoperation beide Markierungen demselben Ausschnitt zuordnen.";return}
    if(!(await ask(`„${ms[0].bezeichnung}“ mit „${ms[1].bezeichnung}“ bearbeiten? Die erste Markierung wird geändert; die zweite bleibt als Hilfskontur erhalten. Kreise werden dabei in 512 Segmente zerlegt. Bestehende Verknüpfungen der ersten Kontur sind anschließend neu zu prüfen.`,{title:"Flächen bearbeiten",kind:"warning"})))return;
    try{const stand=JSON.stringify(p),result=await planApi.flaechenoperation(ms[0].geometrie,ms[1].geometrie,operation);if(s.eingabe!==p||JSON.stringify(p)!==stand)throw Error("Aufmaß wurde geändert. Operation erneut ausführen.");merken();ms[0].geometrie=result[0];if(result[1])p.markierungen.push({...structuredClone($state.snapshot(ms[0])),id:crypto.randomUUID(),bezeichnung:`${ms[0].bezeichnung} · Rest`,geometrie:result[1]});s.meldung="Flächen im Entwurf bearbeitet. Hilfskontur bleibt erhalten; für Summen ihre fachliche Bedeutung prüfen."}catch(e){s.fehler=fehlertext(e)}
  }
  function auswahlAusrichten(art:"links"|"rechts"|"oben"|"unten"){if(!s.eingabe||!darf)return;const ms=s.eingabe.markierungen.filter(m=>auswahlIds.includes(m.id));if(ms.some(m=>s.eingabe!.ebenen.find(e=>e.id===m.ebene_id)?.gesperrt))return;merken();const gs=ausrichten(ms.map(m=>$state.snapshot(m.geometrie)),art);ms.forEach((m,i)=>m.geometrie=gs[i])}
  function fertig(g:Plangeometrie){
    const p=s.eingabe;if(!p||!darf||s.laeuft)return;
    if(werkzeug==="kontrolle"&&g.art==="strecke"){
      if(!Number.isFinite(kontrollMeter)||kontrollMeter<=0){s.fehler="Eine positive Kontrolllänge in Metern angeben.";return}
      const f=meterProPunkt(aktueller?.kalibrierung??p.kalibrierung);if(f===null){s.fehler="Zuerst einen Maßstab setzen.";return}
      kontrolle={gemessen:Math.hypot(g.bis[0]-g.von[0],g.bis[1]-g.von[1])*f,soll:kontrollMeter,stand:kontrollStand};werkzeug="auswahl";return;
    }
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
      grundstueck_id:v?.grundstueck_id??(grund.length===1?grund[0].id:null),gebaeude_id:v?.gebaeude_id??standardGebaeude,geschoss_id:v?.geschoss_id??standardGeschoss};
    if(!v)neu.bedeutung=standardBedeutung;
    if(v?.flurstueck_id)neu.flurstueck_id=v.flurstueck_id;
    if(aktiv)neu.ausschnitt_id=aktiv;
    p.markierungen.push(neu);
    waehlen(id);vorbelegung=null;panel="eigenschaften";
  }
  function aufgabeZeichnen(a:Planaufgabe){
    if(a.geometrie==="hoehenpunkt"&&!hoehenMoeglich){
      s.meldung=`„${a.titel}“ ist eine Höhe: zuerst einen Schnitt oder eine Ansicht als Ausschnitt öffnen und dort die Bezugslinie setzen.`;return;
    }
    vorbelegung=a;
    werkzeug=a.geometrie==="flaeche"?"polygon":a.geometrie==="hoehenpunkt"?"hoehenpunkt":"strecke";
  }
  async function seiteWechseln(n:number){if(!s.eingabe||n===s.eingabe.seite||istDxfUnterlage)return;await quelleWechsel(s.eingabe.dokument,n)}
  function seiteAnsehen(n:number){betrachterSeite=n===s.eingabe?.seite?null:n;plan?.abbrechen();}
  function ausschnittAnlegen(von:Point,bis:Point){
    const p=s.eingabe;if(!p)return;
    p.ausschnitte??=[];
    const n=p.ausschnitte.length+1;
    const a:Planausschnitt={id:crypto.randomUUID(),bezeichnung:`Ausschnitt ${n}`,art:"grundriss",von,bis};
    p.ausschnitte.push(a);
    // Eine Zeichenebene je Ausschnitt; Umbenennen folgt beim Namen des Ausschnitts.
    a.ebene_id=crypto.randomUUID();p.ebenen.push({id:a.ebene_id,bezeichnung:a.bezeichnung,sichtbar:true,gesperrt:false});
    rahmenAuswahl=a.id;
  }
  function oeffnen(id:string){
    const a=ausschnitte.find(a=>a.id===id);if(!a||!s.eingabe)return;
    aktiv=id;rahmenAuswahl="";auswahl="";werkzeug="auswahl";
    const e=s.eingabe.ebenen.find(e=>(a.ebene_id?e.id===a.ebene_id:e.bezeichnung===a.bezeichnung)&&!e.gesperrt);
    if(e)ebene=e.id;
  }
  function schliessen(){aktiv="";auswahl="";werkzeug="auswahl";plan?.einpassen()}
  function umbenennen(a:Planausschnitt,name:string){
    const e=s.eingabe?.ebenen.find(e=>a.ebene_id?e.id===a.ebene_id:e.bezeichnung===a.bezeichnung);
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
  function ausschnittDuplizieren(a:Planausschnitt,mitMarkierungen=false){
    const p=s.eingabe;if(!p)return;
    merken();
    const kopie:Planausschnitt={...structuredClone($state.snapshot(a)),id:crypto.randomUUID(),ebene_id:crypto.randomUUID(),bezeichnung:`${a.bezeichnung} (Kopie)`};
    p.ebenen.push({id:kopie.ebene_id!,bezeichnung:kopie.bezeichnung,sichtbar:true,gesperrt:false});
    if(mitMarkierungen){const ms=p.markierungen.filter(m=>m.ausschnitt_id===a.id),ids=new Map(ms.map(m=>[m.id,crypto.randomUUID()]));
      p.markierungen.push(...ms.map(m=>({...structuredClone($state.snapshot(m)),id:ids.get(m.id)!,ebene_id:kopie.ebene_id!,ausschnitt_id:kopie.id,hoehenzone_von:m.hoehenzone_von?ids.get(m.hoehenzone_von)??m.hoehenzone_von:undefined})));
    }else{delete kopie.bezugslinie;delete kopie.kalibrierung}
    p.ausschnitte=[...(p.ausschnitte??[]),kopie];rahmenAuswahl=kopie.id;oeffnen(kopie.id);
  }
  const rahmenVon=(a:{von:Point;bis:Point})=>({x:Math.min(a.von[0],a.bis[0]),y:Math.min(a.von[1],a.bis[1]),w:Math.abs(a.bis[0]-a.von[0]),h:Math.abs(a.bis[1]-a.von[1])});
  async function source(value:string){
    if(!s.eingabe)return;
    const d=dateien.find(d=>`${d.document_id}|${d.version}`===value);
    if(d)await quelleWechsel({dokument_id:d.document_id,version:d.version},1);
  }
  async function uebertragen(){
    if(!s.eingabe||!darf||!vergleichPdf||!vergleichBestaetigt)return;
    if(!(await ask("Ein neues Aufmaß mit neuen Markierungs-IDs auf der ausgerichteten Vergleichsunterlage anlegen? Der alte Stand und alle bisherigen Verknüpfungen bleiben erhalten. Maßstäbe und Höhenbezüge im neuen Entwurf neu festlegen.",{title:"Aufmaß übertragen",okLabel:"Neuen Entwurf anlegen",cancelLabel:"Abbrechen"})))return;
    const kopie=aufmassUebertragen($state.snapshot(s.eingabe),{dokument_id:vergleichPdf.dokument.document_id,version:vergleichPdf.dokument.version},Math.min(s.eingabe.seite,vergleichPdf.seiten.length),$state.snapshot(vergleichAusrichtung));
    if(s.hatAenderungen()&&!(await s.speichern()))return;
    s.neu();s.eingabe=kopie;vergleich="";vergleichBestaetigt=false;einrichtungOffen=true;s.meldung="Übertragener Entwurf: Konturen prüfen, Maßstäbe kalibrieren und Höhenbezüge neu festlegen. Bestehende Berechnungen verwenden weiterhin das alte Aufmaß.";
  }
  async function exportieren(format:"pdf"|"csv"){
    if(!s.eingabe||!s.id||exportLaeuft)return;
    if(s.hatAenderungen()){
      if(!darf||!(await ask("Für die Ausgabe den aktuellen Aufmaßentwurf zuerst speichern?",{title:"Aufmaß ausgeben",okLabel:"Speichern und ausgeben",cancelLabel:"Abbrechen"}))||!(await s.speichern()))return;
    }
    const ziel=await save({defaultPath:`${s.eingabe.bezeichnung.replace(/[<>:"/\\|?*]/g,"_")}.${format}`,filters:[{name:format==="pdf"?"Aufmaßblatt PDF":"Messwerte CSV",extensions:[format]}]});if(!ziel)return;
    exportLaeuft=true;try{await planApi.exportieren(s.projektId,{aufmass_id:s.id,revision:s.revision,format,nur_sichtbare_ebenen:nurSichtbare},ziel);s.meldung=`${format.toUpperCase()} ausgegeben: ${ziel}`}catch(e){s.fehler=fehlertext(e)}finally{exportLaeuft=false}
  }
  async function quelleWechsel(dokument:Planaufmass["dokument"],seite:number){
    const p=s.eingabe;if(!p||!darf||s.laeuft||!Number.isInteger(seite)||seite<1)return;
    if(p.dokument.dokument_id===dokument.dokument_id&&p.dokument.version===dokument.version&&p.seite===seite)return;
    if(p.markierungen.length||(p.ausschnitte?.length??0)||s.revision>0){
      if(!(await ask("Ein eigenes Aufmaß für das andere Blatt öffnen? Der bisherige Stand bleibt erhalten; offene Änderungen werden zuerst gespeichert. Konturen werden nicht auf das andere Blatt verschoben.",{title:"Anderes Blatt öffnen",okLabel:"Speichern und Blatt öffnen",cancelLabel:"Hier bleiben"})))return;
      if(s.hatAenderungen()&&!(await s.speichern()))return;
      s.neu();if(!s.eingabe)return;
      const datei=dateien.find(d=>d.document_id===dokument.dokument_id&&d.version===dokument.version);
      s.eingabe={...s.leer(),bezeichnung:datei&&istDxf(datei.file_name)?datei.file_name:`${datei?.file_name??"Unterlage"} · Seite ${seite}`,dokument,seite};
    }else{p.dokument=dokument;p.seite=seite;basisReset()}
    plan?.abbrechen();vergleich="";kontrolle=null;
  }
  function basisReset(){
    if(!s.eingabe)return;
    s.eingabe.kalibrierung=null;s.eingabe.hoehenbezug=null;
    // Rahmen bleiben als Vorschlag; ihre Maßstäbe und Bezugslinien sind neu zu bestätigen.
    for(const a of s.eingabe.ausschnitte??[]){delete a.kalibrierung;delete a.bezugslinie}
    aktiv="";
    s.meldung="Unterlage geändert. Maßstab und Höhenbezug neu bestimmen und alle Markierungen kontrollieren.";
  }
  const massstabText=(k:Planaufmass["kalibrierung"]|undefined)=>!k?"":k.art==="einheit"?`Zeichnungseinheit ${EINHEITEN.find(e=>e.code===k.einheit)?.name??k.einheit}`:k.art==="massstab"?`1 : ${z(k.nenner)}`:`kalibriert über ${z(k.meter)} m`;
  const z=(n:number|null|undefined)=>n==null?"—":n.toLocaleString("de-DE",{maximumFractionDigits:3});
  const anzahl=(a:Planausschnitt)=>s.eingabe?.markierungen.filter(m=>m.ausschnitt_id===a.id)??[];
</script>

<Fachliste titel="Aufmaß" {s} bereich="planaufmass" {darf} lesendBedienbar={true} zusatzBeschaeftigt={()=>laedt}>
  {#snippet children(p)}
    <p class="meta">PDF-Pläne und DXF-Zeichnungen unter „Projektdateien“ importieren. Je PDF-Blatt bzw. DXF-Zeichnung ein Aufmaß; mehrere Zeichnungen auf einem Blatt (Grundriss, Schnitte, Ansichten) als Ausschnitte mit eigenem Maßstab und eigener Bezugslinie anlegen. Die Originaldatei bleibt erhalten.</p>
    {#if listenFehler}<p class="fehlertext" role="alert">{listenFehler}</p>{/if}
    {#if !dateien.length}<p class="hinweisbox">Noch keine Unterlage vorhanden. Zuerst eine PDF oder DXF unter „Projektdateien“ importieren.</p>{/if}
    <fieldset class="quellzeile" disabled={!darf||s.laeuft}>
      <label>Bezeichnung<input bind:value={p.bezeichnung} required/></label>
      <label>Unterlage<select aria-label="Unterlage" value={`${p.dokument.dokument_id}|${p.dokument.version}`} onchange={e=>source(e.currentTarget.value)}>
        <option value="">Unterlage wählen</option>
        {#each dateien as d}<option value={`${d.document_id}|${d.version}`}>{d.file_name} · V{d.version}</option>{/each}
      </select></label>
      {#if !istDxfUnterlage}<label>Seite<input type="number" min="1" max={pdf?.seiten.length??1} step="1" value={p.seite} onchange={e=>{const n=Number(e.currentTarget.value);e.currentTarget.value=String(p.seite);void seiteWechseln(n)}} required/></label>{/if}
    </fieldset>
    {#if quelleAlt}<p class="hinweisbox" role="status">Eine neuere Version der Unterlage liegt vor. Dieses Aufmaß gehört weiterhin zur älteren Unterlage; Berechnungsübernahmen sind bis zur Prüfung der neuen Version gesperrt.</p>{/if}
    <details class="einrichtung" bind:open={einrichtungOffen}>
      <summary>{istDxfUnterlage?"Zeichnungseinheit und Höhenbezug der Zeichnung":"Blattmaßstab und Höhenbezug des Blattes"} <span class="meta">{!p.kalibrierung?"· Maßstab fehlt":`· ${massstabText(p.kalibrierung)}`}{p.hoehenbezug?` · Bezug ${z(p.hoehenbezug.hoehe_m)} ${p.hoehenbezug.system}`:""}</span></summary>
      <fieldset disabled={!darf||s.laeuft}><div class="kalibrierung">
        {#if istDxfUnterlage}
          <label>Zeichnungseinheit<select aria-label="Zeichnungseinheit" value={p.kalibrierung?.art==="einheit"?p.kalibrierung.einheit:""} onchange={e=>{const v=e.currentTarget.value as Laengeneinheit|"";if(v)p.kalibrierung={art:"einheit",einheit:v}}}><option value="">Einheit wählen</option>{#each EINHEITEN as e}<option value={e.code}>{e.name}</option>{/each}</select></label>
          <span class="meta">{dxf?.plan.einheit?`Laut DXF-Kopf: ${EINHEITEN.find(e=>e.code===dxf!.plan.einheit)?.name}`:"Die DXF nennt keine Einheit; Zeichnungseinheit wählen und mit einer Kontrollstrecke prüfen."}</span>
        {:else}
        <label>Maßstab 1 :<input type="number" min=".001" step="any" bind:value={nenner}/></label>
        <button type="button" class="sekundaer" onclick={()=>{if(aktueller)aktueller.kalibrierung={art:"massstab",nenner};else p.kalibrierung={art:"massstab",nenner}}}>Maßstab setzen</button>
        {/if}
        <label>Referenzstrecke (m)<input type="number" min=".001" step="any" bind:value={referenz}/></label>
        <button type="button" class:aktiv={werkzeug==="kalibrieren"} onclick={()=>werkzeug="kalibrieren"}>Zwei Punkte kalibrieren</button>
        <span class="meta">{!p.kalibrierung?"Maßstab fehlt":massstabText(p.kalibrierung)}{aktueller?` · gilt für „${aktueller.bezeichnung}“`:""}</span>
      </div>
      <p class="meta">Blattweiter Höhenbezug nur für Blätter ohne Ausschnitte: Höhen werden dann senkrecht zur Seite gemessen. Bei mehreren Schnitten je Blatt die Bezugslinie je Ausschnitt setzen.</p>
      <div class="kalibrierung"><label>Höhensystem<input bind:value={hsystem}/></label><label>Bezugshöhe (m)<input type="number" step="any" bind:value={hwert}/></label>
        <button type="button" class="sekundaer" onclick={()=>werkzeug="bezug"}>Bezugspunkt setzen</button>
        <span>{p.hoehenbezug?`${z(p.hoehenbezug.hoehe_m)} ${p.hoehenbezug.system}`:"Noch kein Höhenbezug des Blattes"}</span>
      </div>
      <div class="kalibrierung"><label>Kontrollstrecke (m)<input type="number" min=".001" step="any" bind:value={kontrollMeter}/></label><button type="button" class="sekundaer" onclick={()=>werkzeug="kontrolle"}>Kontrollstrecke messen</button>
        {#if kontrolle}<span role="status">Kontrollvorschau: {z(kontrolle.gemessen)} m · Soll {z(kontrolle.soll)} m · Abweichung {z((kontrolle.gemessen/kontrolle.soll-1)*100)} %{Math.abs(kontrolle.gemessen/kontrolle.soll-1)>.01?" · Maßstab oder Unterlage prüfen":""}</span>{/if}
      </div></fieldset>
    </details>
    {#if dxf}<details class="einrichtung"><summary>Zeichnungsebenen der DXF <span class="meta">· {dxf.plan.ebenen.length} Ebenen · {dxf.plan.linien.length} Linienzüge</span></summary>
      <div class="dxf-ebenen">{#each dxf.plan.ebenen as e,i}<label><input type="checkbox" checked={dxfSichtbar[i]??true} onchange={ev=>{const an=ev.currentTarget.checked;dxfSichtbar=dxfSichtbar.map((v,j)=>j===i?an:v)}}/><span class="farbe" style:background={e.farbe??"#222"}></span>{e.name}</label>{/each}</div>
      {#if dxf.plan.hinweise.length}<ul class="meta">{#each dxf.plan.hinweise as h}<li>{h}</li>{/each}</ul>{/if}
    </details>{/if}
    <details class="vergleichsbereich"><summary>Lage im Baufenster (§ 23 BauNVO)</summary>
      <p class="meta">Im Lageplan das Baufenster (Baugrenzen / Baulinien) mit Bedeutung „Baufenster“ und den Gebäudeumriss mit Bedeutung „Vorhabenumriss“ zeichnen, je genau eine Markierung; bei mehreren Baugrundstücken dem Vorhabenumriss das Baugrundstück zuordnen. Die Zuordnung speichert beide Umrisse in Metern am Projekt; der GRZ-Nachweis bewertet die Lage.</p>
      <button type="button" disabled={!darf||s.laeuft||baugrenzenLaeuft||!s.id} onclick={()=>void baugrenzenZuordnen()}>{baugrenzenLaeuft?"Wird zugeordnet …":"Baufenster und Vorhabenumriss dem Projekt zuordnen"}</button>
      {#if baugrenzenErgebnis}<p role="status" class:fehlertext={!baugrenzenErgebnis.eingehalten}>Baufenster {z(baugrenzenErgebnis.baufenster_m2)} m² · Vorhaben {z(baugrenzenErgebnis.vorhaben_m2)} m² · außerhalb {z(baugrenzenErgebnis.ausserhalb_m2)} m² ({z(baugrenzenErgebnis.ausserhalb_prozent)} %) · größter Überstand {z(baugrenzenErgebnis.eckpunkt_ueberstand_m)} m · {baugrenzenErgebnis.eingehalten?"eingehalten":"nicht eingehalten"}</p>{/if}
      {#if projekt.planungsrecht.baugrenzen}<p class="meta">Gespeicherte Zuordnung: Dokumentversion {projekt.planungsrecht.baugrenzen.dokument_version}{projekt.planungsrecht.baugrenzen.grundstueck_schluessel?` · Baugrundstück ${projekt.planungsrecht.baugrenzen.grundstueck_schluessel}`:""}</p>{/if}
    </details>
    {#if !istDxfUnterlage}<details class="vergleichsbereich"><summary>Planstände vergleichen</summary>
      <label>Vergleichsunterlage<select aria-label="Vergleichsunterlage" bind:value={vergleich}><option value="">Kein Vergleich</option>{#each pdfDateien as d}<option value={`${d.document_id}|${d.version}`}>{d.file_name} · V{d.version}</option>{/each}</select></label>
      {#if vergleich}<label>Deckkraft der Vergleichsunterlage<input type="range" min="0" max="1" step=".05" bind:value={vergleichDeckkraft}/></label><p class="meta">Vergleichsblatt auf das aktuelle Blatt ausrichten. Verschiebungen in physischen Seitenpunkten (72 pt = 1 Zoll); Rotation im Uhrzeigersinn.</p>
        <div class="kalibrierung"><label>Versatz X (pt)<input type="number" step="any" bind:value={vergleichAusrichtung.dx}/></label><label>Versatz Y (pt)<input type="number" step="any" bind:value={vergleichAusrichtung.dy}/></label><label>Drehung (°)<input type="number" step="any" bind:value={vergleichAusrichtung.grad}/></label><label>Skalierungsfaktor<input type="number" min=".001" step="any" bind:value={vergleichAusrichtung.faktor}/></label></div>
        <label><input type="checkbox" bind:checked={vergleichBestaetigt}/>Blattlage und Konturen anhand der Überlagerung geprüft</label>
        <button type="button" disabled={!darf||!vergleichPdf||s.laeuft||!vergleichBestaetigt} onclick={()=>void uebertragen()}>Als neues Aufmaß auf Vergleichsunterlage übertragen …</button>{/if}
    </details>{/if}
    {#if pdfFehler}<p class="fehlertext" role="alert">{pdfFehler}</p>{/if}
    <div class="editor" class:vollbild bind:this={editor} style:height={vollbild?undefined:`${hoeheEditor}px`} onpointerdowncapture={()=>{verlaufGruppe="";verlauf.gruppeBeenden()}} oninputcapture={e=>{const el=e.target as HTMLElement;verlaufGruppe=`${auswahl}:${el.getAttribute("aria-label")??el.closest("label")?.textContent??el.tagName}`}}>
      <div class="werkzeuge" role="toolbar" aria-label="PDF-Zeichenwerkzeuge">
        <span class="pfad" aria-label="Arbeitsbereich">
          <button type="button" class="link" disabled={!aktiv} onclick={schliessen}>{istDxfUnterlage?"Zeichnung":`Blatt ${p.seite}`}</button>
          {#if aktueller}<span aria-hidden="true">›</span><strong>{aktueller.bezeichnung}</strong>{/if}
        </span>
        {#each werkzeuge as [id,titel,kuerzel]}<button type="button" class:aktiv={werkzeug===id} aria-pressed={werkzeug===id} disabled={(id==="hoehenpunkt"&&!hoehenMoeglich)||(!darf&&id!=="auswahl"&&id!=="verschieben")} title={id==="hoehenpunkt"&&!hoehenMoeglich?"Höhen nur in Schnitt, Ansicht oder Detail":`Taste ${kuerzel}`} onclick={()=>{werkzeug=id;vorbelegung=null}}>{titel}</button>{/each}
        <button type="button" class="sekundaer" disabled={!darf||!undo} title="Strg+Z" onclick={rueckwaerts}>Rückgängig</button>
        <button type="button" class="sekundaer" disabled={!darf||!redo} title="Strg+Y / Strg+Umschalt+Z" onclick={vorwaerts}>Wiederholen</button>
        <label class="umschalter"><input type="checkbox" bind:checked={masse}/>Maße anzeigen</label>
        <button type="button" class="sekundaer rechts" aria-pressed={vollbild} title="F11" onclick={()=>vollbild=!vollbild}>{vollbild?"Vollbild beenden":"Vollbild"}</button>
        <button type="button" class="sekundaer" aria-expanded={hilfe} onclick={()=>hilfe=!hilfe}>Kurzbefehle</button>
        <span class="speicherstatus" role="status">{s.laeuft?"Wird gespeichert …":s.hatAenderungen()?"Entwurf · ungespeichert":"Gespeichert"}</span>
        <button type="button" disabled={!darf||s.laeuft||!s.hatAenderungen()} onclick={()=>s.speichern()}>Aufmaß speichern</button>
        <button type="button" class="sekundaer" disabled={exportLaeuft||s.laeuft} onclick={()=>void exportieren("pdf")}>Aufmaßblatt PDF …</button>
        <button type="button" class="sekundaer" disabled={exportLaeuft||s.laeuft} onclick={()=>void exportieren("csv")}>Messwerte CSV …</button>
        <label class="umschalter"><input type="checkbox" bind:checked={nurSichtbare}/>Nur sichtbare Ebenen ausgeben</label>
      </div>
      {#if hilfe}<section class="kurzbefehle" aria-label="Kurzbefehle"><strong>Zeichnen und navigieren</strong><p>V Auswählen · M Verschieben · P Polygon · R Rechteck · K Kreis · S Strecke · H Höhenpunkt · Z Zoomfenster · F11 Vollbild · F1 Hilfe</p><p>Strg+Rad zoomt · Leertaste verschiebt · Shift bindet Winkel · Alt setzt Fang aus · Shift+Klick erweitert die Auswahl · Pfeile verschieben die Auswahl · Enter schließt · Escape verwirft die laufende Aktion · Strg+Z zurück · Strg+Y / Strg+Shift+Z wiederholen.</p></section>{/if}
      {#if vorbelegung}<p role="status" class="hinweisbox schmal">Zeichnen für <strong>{vorbelegung.gruppe} · {vorbelegung.titel}</strong> ({bedeutungen.find(b=>b[0]===vorbelegung!.bedeutung)?.[1]}){vorbelegung.hinweis?` · ${vorbelegung.hinweis}`:""}. Bedeutung und Zuordnung werden gesetzt. <button type="button" class="link" onclick={()=>{vorbelegung=null;werkzeug="auswahl"}}>Abbrechen</button></p>{/if}
      {#if werkzeug==="kalibrieren"||werkzeug==="bezug"||werkzeug==="bezugslinie"||werkzeug==="ausschnitt"}<p role="status" class="hinweisbox schmal">{werkzeug==="kalibrieren"?"Anfang und Ende der bekannten Referenzstrecke anklicken.":werkzeug==="bezug"?"Den Punkt mit der angegebenen Bezugshöhe auf der PDF anklicken.":werkzeug==="bezugslinie"?`Bezugslinie (z. B. ±0,00) anklicken: Anfang und Ende der Linie; Höhe ${z(hwert)} ${hsystem}.`:"Rahmen um die Zeichnung aufziehen: zwei gegenüberliegende Ecken anklicken."}</p>{/if}
      <div class="arbeitsplatz">
        <div class="flaeche">
          {#if betrachterSeite!==null}<p class="hinweisbox" role="status">Betrachtung von Seite {betrachterSeite}. Das Aufmaß gehört weiterhin zu Seite {p.seite}. <button type="button" class="link" onclick={()=>betrachterSeite=null}>Zum Aufmaßblatt zurück</button>{#if darf}<button type="button" onclick={()=>void seiteWechseln(betrachterSeite!)}>Eigenes Aufmaß für diese Seite anlegen …</button>{/if}</p>{/if}
          {#if pdf||dxf}
            <PdfPlan bind:this={plan} pdfBase64={pdf?.pdf_base64??""} plan={dxf?.plan??null} planSichtbar={dxfSichtbar} seite={betrachterSeite??p.seite} markierungen={betrachterSeite===null?p.markierungen:[]} ebenen={p.ebenen.map(e=>({...e,sichtbar:sichtbarkeit[e.id]??e.sichtbar}))} {auswahl} {auswahlIds} {werkzeug} darf={darf&&!s.laeuft&&betrachterSeite===null}
              ansichtKey={`bauprojekt.planansicht.${benutzerId}.${s.projektId}.${s.id}.${p.dokument.dokument_id}.${p.dokument.version}`} fehlerIds={markierungsFehler.map(f=>f.id)} vergleichBase64={vergleichPdf?.pdf_base64??""} {vergleichDeckkraft}
              {vergleichAusrichtung}
              ausschnitte={betrachterSeite===null?p.ausschnitte??[]:[]} {aktiv} {rahmenAuswahl} werte={aktuelleWerte} kalibrierung={p.kalibrierung} {masse}
              onfertig={fertig} onauswahl={waehlen} onverschieben={gemeinsamVerschieben} onpunkt={i=>{punktIndex=i;panel="eigenschaften"}} onedit={(id,g)=>{if(!objektGesperrt){merken();const item=p.markierungen.find(m=>m.id===id);if(item)item.geometrie=g}}}
              onrahmen={(id,von,bis)=>{if(!darf)return;const a=p.ausschnitte?.find(a=>a.id===id);if(a){merken();a.von=von;a.bis=bis}}}
              onrahmenauswahl={id=>rahmenAuswahl=id} onoeffnen={oeffnen} onseite={seiteAnsehen}
              onfehler={e=>pdfFehler=e}/>
          {:else}<p class="meta">{laedt?"Unterlage wird geladen …":"Eine Unterlage auswählen."}</p>{/if}
          {#if messFehler}<p class="fehlertext" role="alert">Aufmaß prüfen: {messFehler}</p>{/if}
        </div>
        <aside>
          <nav class="panelwahl" aria-label="Planpanel">{#each [["aufgaben","Aufgaben"],["objekte","Objekte"],["eigenschaften","Eigenschaften"]] as [id,titel]}<button type="button" class="sekundaer" class:aktiv={panel===id} aria-pressed={panel===id} onclick={()=>panel=id}>{titel}</button>{/each}</nav>
          {#if panel==="aufgaben"&&s.id}<Planaufgaben projektId={projekt.id} {projekt} {onprojekt} plan={p} aufmassId={s.id} gespeichert={s.original!==null&&!s.hatAenderungen()} {darf} onzeichnen={aufgabeZeichnen} onwaehlen={id=>{waehlen(id);panel="eigenschaften"}}/>{/if}
          {#if panel==="aufgaben"}<details><summary>Voreinstellung für freie Markierungen</summary><fieldset disabled={!darf}>
            <label>Bedeutung neuer Markierungen<select aria-label="Bedeutung neuer Markierungen" bind:value={standardBedeutung}>{#each bedeutungen as [id,t]}<option value={id}>{t}</option>{/each}</select></label>
            <label>Gebäude neuer Markierungen<select aria-label="Gebäude neuer Markierungen" bind:value={standardGebaeude} onchange={()=>standardGeschoss=null}><option value={null}>Nicht zugeordnet</option>{#each gebaeude as g}<option value={g.objekt}>{g.daten.bezeichnung}</option>{/each}</select></label>
            {#if standardGebaeude}<label>Geschoss neuer Markierungen<select aria-label="Geschoss neuer Markierungen" bind:value={standardGeschoss}><option value={null}>Nicht zugeordnet</option>{#each gebaeude.find(g=>g.objekt===standardGebaeude)?.daten.geschosse??[] as g}<option value={g.id}>{g.bezeichnung}</option>{/each}</select></label>{/if}
          </fieldset></details>{/if}
          {#if panel==="objekte"}
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
          {#if !aktiv}<button type="button" class:aktiv={werkzeug==="ausschnitt"} aria-pressed={werkzeug==="ausschnitt"} disabled={!darf||!(pdf||dxf)} onclick={()=>{werkzeug="ausschnitt";rahmenAuswahl=""}}>Ausschnitt anlegen</button>{/if}
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
                <button type="button" class="sekundaer" onclick={()=>ausschnittDuplizieren(a)}>Nur Rahmen kopieren</button>
                <button type="button" class="sekundaer" onclick={()=>ausschnittDuplizieren(a,true)}>Mit Markierungen kopieren</button>
                <button type="button" class="sekundaer" onclick={()=>void ausschnittEntfernen(a)}>Entfernen</button>
              </div>
            </fieldset>
          {/if}
          <h3>Zeichenebenen</h3>
          {#each p.ebenen as e (e.id)}
            <div class="ebene"><input type="radio" aria-label={`Auf Ebene ${e.bezeichnung} zeichnen`} name={`ebene-${s.id}`} value={e.id} bind:group={ebene}/>
              <input aria-label="Ebenenname" bind:value={e.bezeichnung} disabled={!darf||s.laeuft} required/>
              <label title="Sichtbar"><input type="checkbox" checked={sichtbarkeit[e.id]??e.sichtbar} onchange={ev=>{if(darf)e.sichtbar=ev.currentTarget.checked;else sichtbarkeit={...sichtbarkeit,[e.id]:ev.currentTarget.checked}}}/>An</label>
              <label title="Gesperrt"><input type="checkbox" bind:checked={e.gesperrt} disabled={!darf||s.laeuft}/>Sperre</label>
            </div>
          {/each}
          <button type="button" class="sekundaer" disabled={!darf||s.laeuft} onclick={()=>{const id=crypto.randomUUID();p.ebenen.push({id,bezeichnung:`Ebene ${p.ebenen.length+1}`,sichtbar:true,gesperrt:false});ebene=id}}>Ebene hinzufügen</button>
          <h3>Markierungen <span class="meta">({aktiv?`${anzahl(aktueller!).length} von `:""}{p.markierungen.length})</span></h3>
          <label>Suchen<input type="search" aria-label="Markierungen suchen" bind:value={suche}/></label>
          <label>Bedeutung filtern<select aria-label="Bedeutung filtern" bind:value={bedeutungFilter}><option value="">Alle Bedeutungen</option>{#each bedeutungen as [id,t]}<option value={id}>{t}</option>{/each}</select></label>
          <label>Gebäude filtern<select aria-label="Gebäude filtern" bind:value={gebaeudeFilter} onchange={()=>geschossFilter=""}><option value="">Alle Gebäude</option>{#each gebaeude as g}<option value={g.objekt}>{g.daten.bezeichnung}</option>{/each}</select></label>
          {#if gebaeudeFilter}<label>Geschoss filtern<select aria-label="Geschoss filtern" bind:value={geschossFilter}><option value="">Alle Geschosse</option>{#each gebaeude.find(g=>g.objekt===gebaeudeFilter)?.daten.geschosse??[] as g}<option value={g.id}>{g.bezeichnung}</option>{/each}</select></label>{/if}
          <label><input type="checkbox" bind:checked={nurFehler}/>Nur fehlerhafte Markierungen</label>
          <div class="markierungen">
            {#each gefilterteMarkierungen as item (item.id)}<div class="objektzeile"><input type="checkbox" aria-label={`${item.bezeichnung} zur Auswahl hinzufügen`} checked={auswahlIds.includes(item.id)} onchange={()=>waehlen(item.id,true)}/><button type="button" class:aktiv={item.id===auswahl} onclick={()=>{waehlen(item.id);panel="eigenschaften"}}>{item.bezeichnung}<span>{bedeutungen.find(b=>b[0]===item.bedeutung)?.[1]}{fehlerMap.has(item.id)?" · Fehler":""}{!aktiv&&item.ausschnitt_id?` · ${ausschnitte.find(a=>a.id===item.ausschnitt_id)?.bezeichnung??"Ausschnitt fehlt"}`:""}</span></button></div>{/each}
            {#if !p.markierungen.length}<p class="meta">Ein Zeichenwerkzeug wählen (Tasten P, R, K, S, H) und Punkte auf dem Plan setzen.</p>{/if}
          </div>
          {/if}
          {#if panel==="eigenschaften"&&!m}<p class="meta">Eine Markierung auf dem Plan oder unter „Objekte“ auswählen.</p>{/if}
          {#if auswahlIds.length>1}<section class="mehrfach"><h3>{auswahlIds.length} Markierungen ausgewählt</h3><fieldset disabled={!darf||s.laeuft}><div class="aktionen">{#each ["links","rechts","oben","unten"] as art}<button type="button" class="sekundaer" onclick={()=>auswahlAusrichten(art as "links"|"rechts"|"oben"|"unten")}>{art} ausrichten</button>{/each}</div>
            {#if auswahlIds.length===2}<div class="aktionen">{#each [["vereinen","Vereinen"],["abziehen","Zweite Fläche abziehen"],["schneiden","Schnittfläche"],["teilen","An zweiter Fläche teilen"]] as [op,t]}<button type="button" onclick={()=>void flaechenAktion(op as import("../generated/plattform").Flaechenoperation)}>{t}</button>{/each}</div>{/if}</fieldset></section>{/if}
          {#if m&&panel==="eigenschaften"}
            <h3>Eigenschaften</h3>
            <fieldset disabled={objektGesperrt}>
              <label>Bezeichnung<input bind:value={m.bezeichnung} required/></label>
              {#if fehlerMap.has(m.id)}<p class="fehlertext" role="alert">{fehlerMap.get(m.id)}</p>{/if}
              <label>Bedeutung<select aria-label="Bedeutung" bind:value={m.bedeutung} onchange={()=>{if(m&&m.bedeutung!=="flurstueck"&&!ABZUGSBEDEUTUNGEN.includes(m.bedeutung))delete m.flurstueck_id}}>{#each bedeutungen as [v,t]}<option value={v}>{t}</option>{/each}</select></label>
              {#if m.bedeutung==="flurstueck"||ABZUGSBEDEUTUNGEN.includes(m.bedeutung)}
                <label>Flurstück<select aria-label="Flurstück" value={m.flurstueck_id??""} onchange={e=>{const v=e.currentTarget.value;if(v)m!.flurstueck_id=v;else delete m!.flurstueck_id}}><option value="">Nicht zugeordnet</option>{#each flurstuecke as f (f.id)}<option value={f.id}>{f.gemarkung} {f.nummer}{f.flur?` (Flur ${f.flur})`:""}</option>{/each}{#if m.flurstueck_id&&!flurstuecke.some(f=>f.id===m!.flurstueck_id)}<option value={m.flurstueck_id}>Flurstück nicht mehr erfasst</option>{/if}</select></label>
                {#if !m.flurstueck_id}<p class="hinweisbox">Für die Übernahme (Flurstücksfläche bzw. Flächenabzug des Flurstücks) das Flurstück zuordnen; ohne Zuordnung dient die Fläche nur zur Kontrolle.{flurstuecke.length?"":" Flurstücke zuerst unter Grundlagen → Flurstücke anlegen und speichern."}</p>{/if}
              {/if}
              {#if ["raum","aufenthaltsraum","nuf","tf","vf"].includes(m.bedeutung)}
                {@const raeume=p.markierungen.filter(x=>x.id!==m!.id&&!x.hoehenzone_von&&["raum","aufenthaltsraum","nuf","tf","vf"].includes(x.bedeutung)&&x.gebaeude_id===m!.gebaeude_id&&x.geschoss_id===m!.geschoss_id)}
                {#if raeume.length||m.hoehenzone_von}<label>Höhenzone von Raum<select aria-label="Höhenzone von Raum" value={m.hoehenzone_von??""} onchange={e=>{const v=e.currentTarget.value;if(v)m!.hoehenzone_von=v;else delete m!.hoehenzone_von}}><option value="">Keine – eigener Raum</option>{#each raeume as x (x.id)}<option value={x.id}>{x.bezeichnung}</option>{/each}{#if m.hoehenzone_von&&!raeume.some(x=>x.id===m!.hoehenzone_von)}<option value={m.hoehenzone_von}>Raum nicht mehr passend</option>{/if}</select></label>{/if}
              {/if}
              {#if ausschnitte.length}<label>Ausschnitt<select aria-label="Ausschnitt der Markierung" value={m.ausschnitt_id??""} onchange={e=>{const v=e.currentTarget.value;if(v)m.ausschnitt_id=v;else delete m.ausschnitt_id}}><option value="">Ganzes Blatt</option>{#each ausschnitte as a}<option value={a.id}>{a.bezeichnung}</option>{/each}</select></label>{/if}
              <label>Zeichenebene<select aria-label="Zeichenebene" bind:value={m.ebene_id}>{#each p.ebenen as e}<option value={e.id}>{e.bezeichnung}</option>{/each}</select></label>
              <label>Baugrundstück<select aria-label="Baugrundstück" bind:value={m.grundstueck_id}><option value={null}>Nicht zugeordnet</option>{#each grund as g}<option value={g.id}>{g.bezeichnung}</option>{/each}</select></label>
              <label>Gebäude<select aria-label="Gebäude" bind:value={m.gebaeude_id} onchange={()=>{if(m)m.geschoss_id=null}}><option value={null}>Nicht zugeordnet</option>{#each gebaeude as g}<option value={g.objekt}>{g.daten.bezeichnung}</option>{/each}</select></label>
              {#if m.gebaeude_id}<label>Geschoss<select aria-label="Geschoss" bind:value={m.geschoss_id}><option value={null}>Nicht zugeordnet</option>{#each gebaeude.find(g=>g.objekt===m.gebaeude_id)?.daten.geschosse??[] as g,i}<option value={g.id}>{g.bezeichnung||`Geschoss ${i+1} (ohne Bezeichnung)`}</option>{/each}</select></label>{/if}
              {#if (m.bedeutung==="gebaeudegrundflaeche"||m.bedeutung==="geschoss_aussenkante")&&!m.gebaeude_id}<p class="hinweisbox">Für die Übernahme ein Gebäude zuordnen.{gebaeude.length?"":" Gebäude zuerst unter Grundlagen → Gebäude / Geschosse anlegen und speichern."}</p>
              {:else if m.bedeutung==="geschoss_aussenkante"&&!m.geschoss_id}<p class="hinweisbox">Für die Geschossfläche ein Geschoss zuordnen.{gebaeude.find(g=>g.objekt===m.gebaeude_id)?.daten.geschosse.length?"":" Für dieses Gebäude sind noch keine Geschosse gespeichert; ohne einzelne Geschosse dient die Gebäudegrundfläche als Fläche des Einzelgeschosses."}</p>
              {:else if m.bedeutung==="gebaeudegrundflaeche"&&m.geschoss_id}<p class="hinweisbox">Die Gebäudegrundfläche gilt für das ganze Gebäude; die Geschosszuordnung wird dafür nicht verwendet. Für eine Geschossfläche die Markierung duplizieren und die Kopie als „Geschoss – Außenkante“ kennzeichnen.</p>{/if}
              <div class="aktionen">
                <button type="button" class="sekundaer" title="Strg+D · Kopie derselben Kontur, z. B. als Gebäudegrundfläche und Geschoss-Außenkante" onclick={duplizieren}>Duplizieren</button>
                <button type="button" class="sekundaer" title="Entf" onclick={entfernen}>Markierung entfernen</button>
              </div>
              <label>Kontur drehen (°)<input type="number" step="any" bind:value={drehenGrad}/></label><button type="button" class="sekundaer" onclick={()=>geometrieSetzen(drehen($state.snapshot(m!.geometrie),drehenGrad))}>Drehung anwenden</button>
              <details><summary>Stützpunkte und Kanten bearbeiten</summary>
                <label>Stützpunkt<select aria-label="Stützpunkt" bind:value={punktIndex}>{#each eckpunkte(m.geometrie) as p,i}<option value={i}>Punkt {i+1} · {z(p[0])}; {z(p[1])} pt</option>{/each}</select></label>
                {#if m}{@const punkt=eckpunkte(m.geometrie)[punktIndex]??eckpunkte(m.geometrie)[0]}
                {#if punkt}<label>X (Seitenpunkte)<input type="number" step="any" value={punkt[0]} onchange={e=>geometrieSetzen(punktAendern($state.snapshot(m!.geometrie),punktIndex,[Number(e.currentTarget.value),punkt[1]]))}/></label><label>Y (Seitenpunkte)<input type="number" step="any" value={punkt[1]} onchange={e=>geometrieSetzen(punktAendern($state.snapshot(m!.geometrie),punktIndex,[punkt[0],Number(e.currentTarget.value)]))}/></label>{/if}
                {/if}
                {#if m.geometrie.art==="polygon"}<div class="aktionen"><button type="button" class="sekundaer" onclick={()=>geometrieSetzen(punktEinfuegen($state.snapshot(m!.geometrie),punktIndex))}>Punkt nach Auswahl einfügen</button><button type="button" class="sekundaer" disabled={m.geometrie.punkte.length<=3} onclick={()=>{geometrieSetzen(punktEntfernen($state.snapshot(m!.geometrie),punktIndex));punktIndex=0}}>Punkt entfernen</button></div>{/if}
                {#if m.geometrie.art==="polygon"||m.geometrie.art==="strecke"}<label>Kantenlänge (m)<input type="number" min=".001" step="any" bind:value={kanteMeter}/></label><label>Kantenwinkel (°)<input type="number" step="any" bind:value={kanteWinkel}/></label><button type="button" disabled={!meterProPunkt(aktueller?.kalibrierung??p.kalibrierung)} onclick={()=>{const f=meterProPunkt(aktueller?.kalibrierung??p.kalibrierung);if(f)geometrieSetzen(kantenmass($state.snapshot(m!.geometrie),punktIndex,kanteMeter/f,kanteWinkel))}}>Kantenmaß anwenden</button>{/if}
              </details>
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
  .panelwahl{position:sticky;top:0;z-index:2;display:flex;gap:.25rem;padding:.4rem 0;background:var(--bg,#f4f6f8);border-bottom:2px solid var(--linie,#ccd5df)}
  .panelwahl button{flex:1;font-size:.8rem;padding:.4rem .3rem}
  .panelwahl button.aktiv{box-shadow:inset 0 -3px var(--accent,#126d97)}
  .speicherstatus{font-size:.8rem;font-variant-numeric:tabular-nums}
  .kurzbefehle{border:1px solid var(--linie,#ccd5df);padding:.6rem;margin:.4rem 0;background:var(--bg,#f4f6f8)}
  .kurzbefehle p{margin:.4rem 0;font-size:.85rem}
  .dxf-ebenen{display:flex;flex-wrap:wrap;gap:.4rem 1rem;padding:.3rem 0}
  .dxf-ebenen label{display:flex;gap:.35rem;align-items:center}
  .dxf-ebenen .farbe{width:.8rem;height:.8rem;border:1px solid #888;display:inline-block}
  .vergleichsbereich{padding:.5rem 0;border-top:1px solid var(--linie,#ccd5df)}
  .vergleichsbereich>label{display:flex;gap:.6rem;align-items:center;margin:.4rem 0}
  .objektzeile{display:flex;gap:.4rem;align-items:center;content-visibility:auto;contain-intrinsic-size:auto 48px}
  .objektzeile button{flex:1}
  .mehrfach{padding:.5rem;border:1px solid var(--linie,#ccd5df);margin:.5rem 0}
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
