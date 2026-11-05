<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { ask, save } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext, EINHEITEN, istDxf, istIfc, istPlan, type Project, type DocumentVersion, type PdfVorschauDaten, type GrundstueckKontext, type CadPlan, type BaugrenzenPruefung } from "../api";
  import type { Planaufmass, Planmarkierung, Planbedeutung, Planmesswert, Plangeometrie, Planausschnitt, Ausschnittart, Planaufgabe, Werkzeugvorlage, Gebaeude, Laengeneinheit, Planhintergrund, Zeichenelement, Zeichenform } from "../generated/plattform";
  import { ifcApi } from "../ifc/api";
  import type { Modellgeschoss } from "../generated/plattform";
  import { hgSkalieren, hgVerschieben, hgDrehen, hintergrundNeu, mppAusMassstab, elementPunkte, type Hintergrundansicht } from "./zeichnung";
  import { EINHEIT_METER } from "./pdfGeometry";
  import { FachSitzung } from "../FachSitzung.svelte";
  import Fachliste from "../Fachliste.svelte";
  import Symbolknopf from "../Symbolknopf.svelte";
  import WerkzeugIcon from "./WerkzeugIcon.svelte";
  import { plattformApi, type Datensatz } from "../plattformApi";
  import { ansichtsAbfrage } from "../ansichtsAbfrage.svelte";
  import PdfPlan from "./PdfPlan.svelte";
  import Planaufgaben from "./Planaufgaben.svelte";
  import { planApi } from "./api";
  import { verschieben, eckpunkte, meterProPunkt, type Point } from "./pdfGeometry";
  import { Bearbeitungsverlauf } from "./verlauf";
  import { drehen, punktAendern, punktEinfuegen, punktEntfernen, kantenmass, ausrichten } from "./bearbeitung";
  import { abbildenGeometrie, abbildenElement, verschiebung, spiegelung, elementSpiegeln, versatz, elementVersatz, geometrieStutzen, elementStutzen, reihe, befehlAuswerten, BEFEHLE, type Punkt } from "./konstruktion";
  import { aufmassUebertragen } from "./uebertragung";
  import { grundstuecksinformationen } from "../grundstuecksdaten";
  import { ABZUGSBEDEUTUNGEN, BAUTEILBEDEUTUNGEN } from "./grundstueck";
  import type { Tabstart } from "./tabs";
  import { BEDEUTUNGEN } from "./bedeutungen";
  import { verwendungenLaden, type Verwendung } from "./verwendung";
  import Quellenzustand from "./Quellenzustand.svelte";
  import DarstellungFelder from "./DarstellungFelder.svelte";
  import { LAGEPLAN_EBENEN } from "./darstellung";
  import { PLANSYMBOLE, oeffnungLage } from "./planzeichen";
  import type { Raumbuch, Plansymbol, Oeffnungsart } from "../generated/plattform";
  import { projektNavigation } from "../projektNavigation.svelte";
  import { tick } from "svelte";
  import { Uebernahme } from "./Uebernahme.svelte";
  import UebernahmeVorschau from "./UebernahmeVorschau.svelte";
  import { flurstueckVorbereiten, flurstueckeSpeichern } from "./flurstueck";
  let {projekt,benutzerId,darf,onprojekt,unterlage=null,modus="voll",start=null,entwurfObjekt="",sichtbar=true,onoeffnen,onneu,onstatus,onschliessen,onflurstueckfertig}:{projekt:Project;benutzerId:string;darf:boolean;onprojekt?:(p:Project)=>void;unterlage?:{dokumentId:string;nummer:number}|null;
    /** Flurstücksauftrag des Tabs übernommen oder beendet. */
    onflurstueckfertig?:()=>void;
    /** voll: Liste und Editor in einem; liste: nur die Liste, Öffnen/Neu gehen an den Aufrufer; tab: nur der Editor für `start`. */
    modus?:"voll"|"liste"|"tab";start?:Tabstart|null;entwurfObjekt?:string;sichtbar?:boolean;
    onoeffnen?:(f:Datensatz<Planaufmass>)=>void;onneu?:(start?:Tabstart)=>void;onstatus?:(st:{titel:string;geaendert:boolean;id?:string|null})=>void;onschliessen?:()=>void}=$props();
  // Flurstücke des Projekts für die Zuordnung von Flurstücksmarkierungen; Stand der Akte.
  const flurstuecke=$derived(grundstuecksinformationen(projekt.planungsrecht).flurstuecke);
  let dateien=$state<DocumentVersion[]>([]), ifcDateien=$state<DocumentVersion[]>([]), pdf=$state<PdfVorschauDaten|null>(null), pdfFehler=$state(""), grund=$state<GrundstueckKontext[]>([]);
  let gebaeude=$state<Datensatz<Gebaeude>[]>([]),listenFehler=$state(""),laedt=$state(false);
  // Erst nach dem ersten Laden der Unterlagen darf ein Tab seinen Start ausführen, sonst fehlt die Unterlage.
  let unterlagenGeladen=$state(false);
  // DXF-Unterlage: Zeichnung samt Ebenensichtbarkeit (nur Anzeige und Fang).
  let dxf=$state<CadPlan|null>(null), dxfSichtbar=$state<boolean[]>([]);
  let baugrenzenLaeuft=$state(false), baugrenzenErgebnis=$state<BaugrenzenPruefung|null>(null);
  const s=untrack(()=>new FachSitzung<Planaufmass>(projekt.id,"planaufmass",benutzerId,()=>({
    bezeichnung:"Neues Aufmaß",dokument:{dokument_id:dateien[0]?.document_id??"00000000-0000-0000-0000-000000000000",version:dateien[0]?.version??1},
    seite:1,kalibrierung:null,hoehenbezug:null,ebenen:[{id:crypto.randomUUID(),bezeichnung:"Aufmaß",sichtbar:true,gesperrt:false}],markierungen:[],
  }),p=>p.bezeichnung,entwurfObjekt));
  /** Leere eigene Zeichnung: kein Dokument, Blatt in Metern. */
  function leereZeichnung():Planaufmass{
    const {dokument:_d,...rest}=s.leer();void _d;
    return {...rest,bezeichnung:"Neue Zeichnung",kalibrierung:{art:"einheit",einheit:"meter"},seite:1,ebenen:[{id:crypto.randomUUID(),bezeichnung:"Zeichnung",sichtbar:true,gesperrt:false}]};
  }
  const bedeutungen=BEDEUTUNGEN;
  const arten: [Ausschnittart,string][]=[["grundriss","Grundriss"],["lageplan","Lageplan"],["schnitt","Schnitt"],["ansicht","Ansicht"],["detail","Detail"]];
  const mitHoehen=(a:Ausschnittart)=>a==="schnitt"||a==="ansicht"||a==="detail";
  // Werkzeug, Beschriftung, Tastaturkürzel.
  // Werkzeuge der linken Symbolleiste, in Gruppen: Auswahl, Zeichnen, Höhen, Blatt. [id, Titel, Taste]
  const werkzeuggruppen:[string,string,string][][]=[
    [["auswahl","Auswählen","V"],["verschieben","Verschieben","M"]],
    [["polygon","Polygon","P"],["rechteck","Rechteck","R"],["kreis","Kreis","K"],["strecke","Strecke / Dach","S"]],
    [["wand","Wand","W"],["masskette","Maßkette","L"],["text","Text","X"]],
    [["oeffnung","Fenster / Tür","F"],["symbol","Planzeichen","G"],["raumstempel","Raumstempel","N"]],
    [["kopieren","Kopieren","C"],["spiegeln","Spiegeln","I"],["stutzen","Stutzen / Dehnen","D"],["versatz","Versatz","O"],["reihe","Reihe","J"]],
    [["hoehenpunkt","Höhenpunkt","H"],["bezugslinie","Bezugslinie setzen","B"]],
    [["ausschnitt","Ausschnitt anlegen","A"],["kalibrieren","Zwei Punkte kalibrieren","Q"],["kontrolle","Kontrollstrecke messen","T"]],
  ];
  const werkzeuge=werkzeuggruppen.flat();
  let werkzeug=$state("auswahl"),auswahl=$state(""),ebene=$state(""),nenner=$state(100),referenz=$state(10),hwert=$state(0),hsystem=$state("m ü. NHN");
  let werte=$state<Planmesswert[]>([]),messFehler=$state(""),messLaeuft=$state(false),messStand=$state("");
  const verlauf=new Bearbeitungsverlauf<Planaufmass>();
  let undo=$state(0),redo=$state(0),verlaufGruppe="";
  let auswahlIds=$state<string[]>([]),punktIndex=$state(0),drehenGrad=$state(0),kanteMeter=$state(1),kanteWinkel=$state(0);
  let suche=$state(""),bedeutungFilter=$state(""),gebaeudeFilter=$state(""),geschossFilter=$state(""),nurFehler=$state(false),panel=$state("aufgaben"),hilfe=$state(false);
  // Rechte Spalte: drei Reiter, jeder zeigt nur seinen eigenen Inhalt.
  // „Eigenschaften“ ohne Auswahl zeigt das Blatt (Unterlage, Maßstab, Höhenbezug) und die Prüf- und Ausgabefunktionen.
  const REITER=[["aufgaben","Aufgaben"],["objekte","Objekte"],["eigenschaften","Eigenschaften"]] as const;
  function panelOeffnen(id:string){panel=id}
  // Optionsleiste: Name, Felder und nächster Schritt des aktiven Werkzeugs in einer Zeile über dem Plan.
  const MARKIERUNGSWERKZEUGE=["polygon","rechteck","kreis","strecke","hoehenpunkt"];
  // Versatz und Reihe wirken auf die Auswahl ohne Klicks im Plan; dort bleibt währenddessen das Auswählen aktiv.
  const AUSWAHLWERKZEUGE=["versatz","reihe"];
  // Werkzeugkiste: Ausklappliste am Werkzeug (kleines Dreieck oder rechte Maustaste).
  let vorlagenmenue=$state(""),vorlagenLage=$state({x:0,y:0});
  function vorlagenUmschalten(id:string,ziel:EventTarget|null){
    if(vorlagenmenue===id){vorlagenmenue="";return}
    const r=(ziel as HTMLElement|null)?.closest(".wzplatz")?.getBoundingClientRect();
    if(r)vorlagenLage={x:r.right+4,y:r.top};
    vorlagenmenue=id;
  }
  const SONDERWERKZEUGE:Record<string,[string,string]>={hgstrecke:["kalibrieren","Hintergrund einpassen"],hgverschieben:["verschieben","Hintergrund verschieben"],hgdrehen:["verschieben","Hintergrund drehen"],bezug:["hoehenpunkt","Bezugshöhe setzen"]};
  const werkzeugTitel=$derived(SONDERWERKZEUGE[werkzeug]?.[1]??werkzeuge.find(w=>w[0]===werkzeug)?.[1]??"Auswählen");
  const werkzeugSymbol=$derived(SONDERWERKZEUGE[werkzeug]?.[0]??werkzeug);
  const werkzeugSchritt=$derived.by(()=>{switch(werkzeug){
    case "auswahl":return "";
    case "polygon":return "Punkte setzen, Enter oder Klick auf den ersten Punkt schließt";
    case "rechteck":return "Zwei gegenüberliegende Ecken";
    case "kreis":return "Mittelpunkt, dann Radius";
    case "strecke":return "Anfangs- und Endpunkt";
    case "wand":case "masskette":return "Anfangs- und Endpunkt";
    case "text":return "Einfügepunkt anklicken";
    case "hoehenpunkt":return "Punkt anklicken";
    case "symbol":return "Klick setzt das Planzeichen";
    case "raumstempel":return planzeichen.raum?"Klick setzt den Raumstempel":"Zuerst einen Raum wählen";
    case "oeffnung":return "In eine Wand klicken";
    case "kopieren":return auswahlAnzahl?`Basispunkt, dann Zielpunkt · ${auswahlAnzahl} Objekt(e)`:"Zuerst Objekte wählen";
    case "spiegeln":return auswahlAnzahl?"Zwei Punkte der Spiegelachse":"Zuerst Objekte wählen";
    case "stutzen":return "Grenzkante mit zwei Punkten abgreifen";
    case "versatz":return auswahlAnzahl?`Parallele im Abstand · ${auswahlAnzahl} Objekt(e)`:"Objekte im Plan wählen";
    case "reihe":return auswahlAnzahl?`Kopien in gleichem Abstand · ${auswahlAnzahl} Objekt(e)`:"Objekte im Plan wählen";
    case "kalibrieren":return "Anfang und Ende der Referenzstrecke";
    case "bezug":return "Punkt mit der Bezugshöhe anklicken";
    case "bezugslinie":return "Anfang und Ende der Bezugslinie";
    case "kontrolle":return "Anfang und Ende einer bekannten Strecke";
    case "ausschnitt":return "Zwei gegenüberliegende Ecken des Rahmens";
    case "hgstrecke":return `Anfang und Ende einer bekannten Strecke (${z(hgStrecke)} m) anklicken`;
    case "hgverschieben":return "Punkt des Hintergrunds, dann Zielposition";
    case "hgdrehen":return "Drehpunkt anklicken";
    default:return "";
  }});
  // Kopfleiste: Menüs „Prüfen“ und „Ausgabe“; ihre Werkzeugfenster schweben über dem Plan, der bedienbar bleibt.
  const FENSTERTITEL:Record<string,string>={baufenster:"Lage im Baufenster (§ 23 BauNVO)",vergleich:"Planstände vergleichen",planblatt:"Planblatt PDF (maßstäblich)"};
  let menue=$state(""),werkzeugfenster=$state<""|"baufenster"|"vergleich"|"planblatt">("");
  function fensterOeffnen(f:"baufenster"|"vergleich"|"planblatt"){menue="";werkzeugfenster=f;if(f==="planblatt")void planlisteLaden()}
  function auswahlAufheben(){auswahl="";auswahlIds=[];elementAuswahl="";elementAuswahlIds=[];ebenenAuswahl=""}
  // Reiter „Objekte“: Baum aus Ebenen und ihren Objekten; eine gewählte Ebene zeigt ihre Eigenschaften.
  let ebenenAuswahl=$state(""),zugeklappt=$state<Record<string,boolean>>({}),filterOffen=$state(false);
  function ebeneWaehlen(id:string){auswahl="";auswahlIds=[];elementAuswahl="";elementAuswahlIds=[];ebenenAuswahl=id;panelOeffnen("eigenschaften")}
  function reiterTaste(e:KeyboardEvent){
    const i=REITER.findIndex(r=>r[0]===panel);let n=i;
    if(e.key==="ArrowRight")n=(i+1)%REITER.length;else if(e.key==="ArrowLeft")n=(i+REITER.length-1)%REITER.length;else if(e.key==="Home")n=0;else if(e.key==="End")n=REITER.length-1;else return;
    e.preventDefault();panel=REITER[n][0];void tick().then(()=>(e.currentTarget as HTMLElement|null)?.querySelector<HTMLElement>(`[data-reiter="${panel}"]`)?.focus());
  }
  let markierungsFehler=$state<{id:string;fehler:string}[]>([]);
  let kontrollMeter=$state(10),kontrolle=$state<{gemessen:number;soll:number;stand:string}|null>(null);
  let vergleich=$state(""),vergleichPdf=$state<PdfVorschauDaten|null>(null),vergleichDeckkraft=$state(.5);
  let vergleichAusrichtung=$state({dx:0,dy:0,grad:0,faktor:1}),vergleichBestaetigt=$state(false);
  let standardGebaeude=$state<string|null>(null),standardGeschoss=$state<string|null>(null),standardBedeutung=$state<Planbedeutung>("unbestimmt");
  let exportLaeuft=$state(false),nurSichtbare=$state(true);
  // Planlayout eigener Zeichnungen: Blatt, Maßstab, Nordpfeil, Plankopf aus der Planliste.
  let layout=$state({format:"a3" as import("../generated/plattform").Blattformat,hochformat:false,massstab:100,nord:true,nordwinkel:0,planinhalt:"",dokument:"",legende:true,ablegen:false});
  let planlisteZeilen=$state<import("../generated/plattform").Planzeile[]>([]);
  async function planlisteLaden(){try{planlisteZeilen=(await api.planliste(s.projektId)).zeilen}catch{planlisteZeilen=[]}}
  async function dxfAusgeben(){
    if(!s.eingabe||!s.id||exportLaeuft)return;
    if(s.hatAenderungen()){
      if(!darf||!(await ask("Für die DXF-Ausgabe zuerst speichern?",{title:"DXF ausgeben",okLabel:"Speichern und ausgeben",cancelLabel:"Abbrechen"}))||!(await s.speichern()))return;
    }
    const ziel=await save({defaultPath:`${s.eingabe.bezeichnung.replace(/[<>:"/\\|?*]/g,"_")}.dxf`,filters:[{name:"DXF",extensions:["dxf"]}]});if(!ziel)return;
    exportLaeuft=true;try{await api.planaufmassDxf(s.projektId,s.id,s.revision,nurSichtbare,ziel);s.meldung=`DXF ausgegeben (Meter, eine DXF-Ebene je Zeichenebene): ${ziel}`}catch(e){s.fehler=fehlertext(e)}finally{exportLaeuft=false}
  }
  async function layoutAusgeben(){
    if(!s.eingabe||!s.id||exportLaeuft)return;
    if(s.hatAenderungen()){
      if(!darf||!(await ask("Für das Planlayout die Zeichnung zuerst speichern?",{title:"Planlayout",okLabel:"Speichern und ausgeben",cancelLabel:"Abbrechen"}))||!(await s.speichern()))return;
    }
    const zeile=planlisteZeilen.find(z=>z.eintrag.dokument_id===layout.dokument);
    const ziel=await save({defaultPath:`${(zeile?.eintrag.plannummer||s.eingabe.bezeichnung).replace(/[<>:"/\\|?*]/g,"_")}.pdf`,filters:[{name:"Planblatt PDF",extensions:["pdf"]}]});if(!ziel)return;
    exportLaeuft=true;
    try{
      const v=await api.planlayoutExportieren(s.projektId,{aufmass_id:s.id,revision:s.revision,format:layout.format,hochformat:layout.hochformat,massstab:Math.round(layout.massstab),nordwinkel_grad:layout.nord?layout.nordwinkel:null,planinhalt:layout.planinhalt,dokument_id:layout.dokument||null,legende:layout.legende,nur_sichtbare_ebenen:nurSichtbare,ablegen:layout.ablegen&&darf},ziel);
      s.meldung=`Planblatt ausgegeben: ${ziel}${v?`. Im Projekt abgelegt als ${v.file_name}, Version ${v.version}${layout.dokument?"; in der Planliste den neuen Index setzen":""}.`:"."}`;
    }catch(e){s.fehler=fehlertext(e)}finally{exportLaeuft=false}
  }
  let betrachterSeite=$state<number|null>(null),sichtbarkeit=$state<Record<string,boolean>>({});
  let aktiv=$state(""),rahmenAuswahl=$state(""),vollbild=$state(false),masse=$state(true),hoeheEditor=$state(560);
  // Zeichenelemente und Hintergründe eigener Zeichnungen.
  let paletten=$state<HTMLElement>();
  // Neue Auswahl: Eigenschaften stehen oben, die Spalte beginnt wieder am Anfang.
  $effect(()=>{void auswahl;void elementAuswahl;if(panel==="eigenschaften")untrack(()=>paletten?.scrollTo({top:0}))});
  let elementAuswahl=$state(""),wanddicke=$state(.24),elementAuswahlIds=$state<string[]>([]);
  // Konstruktionsbefehle: Versatz und Reihe in Metern, Spiegeln wahlweise als Kopie.
  let konstruktion=$state({versatz:.5,anzahl:3,dx:1,dy:0,spiegelKopie:true});
  // Befehlszeile (F2): Kürzel wie L, PL, KO, SP, VS 0,5; Enter auf der Zeichenfläche wiederholt den letzten Befehl.
  let befehl=$state(""),letzterBefehl=$state(""),befehlFeld=$state<HTMLInputElement>();
  const befehlsliste=Object.entries(BEFEHLE);
  // Planzeichen, Raumstempel und Öffnungen: Vorwahl für den nächsten Klick.
  let planzeichen=$state<{symbol:Plansymbol;oeffnung:Oeffnungsart;raum:string}>({symbol:"baum_bestand",oeffnung:"fenster",raum:""});
  let raumbuecher=$state<Datensatz<Raumbuch>[]>([]);
  async function raumbuecherLaden(){try{raumbuecher=await plattformApi.liste<Raumbuch>(s.projektId,"raumbuch")}catch{raumbuecher=[]}}
  const raeume=$derived(Object.fromEntries(raumbuecher.flatMap(r=>r.daten.raeume.map(x=>[x.id,{nummer:x.nummer,bezeichnung:x.bezeichnung,flaeche_m2:x.flaeche_m2}]))));
  $effect(()=>{void s.projektId;untrack(()=>void raumbuecherLaden())});
  function lageplanEbenen(){
    const p=s.eingabe;if(!p||!darf||s.laeuft)return;
    merken();let n=0;
    for(const [name,d] of LAGEPLAN_EBENEN)if(!p.ebenen.some(e=>e.bezeichnung===name)){p.ebenen.push({id:crypto.randomUUID(),bezeichnung:name,sichtbar:true,gesperrt:false,darstellung:structuredClone(d)});n++}
    s.meldung=n?`${n} Lageplan-Ebenen angelegt (PlanZV/BauVorlV-Farben: Bestand grau, Abbruch gelb, Neubau rot).`:"Alle Lageplan-Ebenen sind schon vorhanden.";
  }
  // Einpunkt-Werkzeuge für Zeichenelemente: Klick setzt Planzeichen, Raumstempel oder Öffnung.
  function einpunktElement(w:string,punkt:Punkt){
    const p=s.eingabe;if(!p)return;
    const f=blattFaktor()??1;
    if(w==="symbol"){elementAnlegen({art:"symbol",punkt,symbol:planzeichen.symbol,groesse:(planzeichen.symbol.startsWith("baum")?5:planzeichen.symbol==="stellplatz"?5:1.5)/f,drehung_grad:0});werkzeug="symbol";return}
    if(w==="raumstempel"){
      const [rb,raum]=planzeichen.raum.split("|");
      if(!raum){s.fehler="Zuerst den Raum aus dem Raumbuch wählen (Hinweisleiste über der Zeichenfläche).";return}
      elementAnlegen({art:"raumstempel",punkt,raumbuch_id:rb,raum_id:raum,hoehe:.3/f});werkzeug="raumstempel";return;
    }
    // Öffnung: nächste Wand unter dem Klick.
    const breite=planzeichen.oeffnung==="fenster"?1.01:.885;
    let best:{e:Zeichenelement&{art:"wand"};d:number}|null=null;
    for(const e of p.elemente??[]){
      if(e.art!=="wand")continue;
      const dx=e.bis[0]-e.von[0],dy=e.bis[1]-e.von[1],l2=dx*dx+dy*dy;if(!l2)continue;
      const t=Math.max(0,Math.min(1,((punkt[0]-e.von[0])*dx+(punkt[1]-e.von[1])*dy)/l2));
      const d=Math.hypot(punkt[0]-(e.von[0]+t*dx),punkt[1]-(e.von[1]+t*dy));
      if(d<=e.dicke_m/f&&(!best||d<best.d))best={e:e as Zeichenelement&{art:"wand"},d};
    }
    if(!best){s.fehler="Für Fenster und Türen in eine Wand klicken.";return}
    if(p.ebenen.find(x=>x.id===best!.e.ebene_id)?.gesperrt){s.fehler="Die Ebene der Wand ist gesperrt.";return}
    const lage=oeffnungLage(best.e.von,best.e.bis,punkt,breite,f,best.e.oeffnungen);
    if(lage===null){s.fehler="Hier ist kein Platz für die Öffnung (Wand zu kurz oder schon belegt).";return}
    merken();
    best.e.oeffnungen=[...(best.e.oeffnungen??[]),planzeichen.oeffnung==="fenster"?{art:"fenster",lage_m:lage,breite_m:breite,hoehe_m:1.385,bruestung_m:.885,links:false}:{art:"tuer",lage_m:lage,breite_m:breite,hoehe_m:2.01,links:false}];
    elementWaehlen(best.e.id);werkzeug="oeffnung";panel="eigenschaften";
  }
  let hgQuellen=$state<Record<string,{pdfBase64?:string;plan?:import("../api").CadPlan["plan"]|null;name:string}>>({});
  const hgLaeuft=new Set<string>();
  // IFC-Hintergründe: Grundriss-Schnitt eines Geschosses, schon in Metern.
  let hgGeschosse=$state<Modellgeschoss[]>([]),hgGeschoss=$state("");
  const hgSchluessel=(h:Planhintergrund)=>`${h.dokument.dokument_id}|${h.dokument.version}${h.geschoss?`|${h.geschoss}`:""}`;
  const hgDateien=$derived([...dateien,...ifcDateien]);
  const hgDateiIfc=$derived(istIfc(ifcDateien.find(d=>`${d.document_id}|${d.version}`===hgDatei)?.file_name??""));
  $effect(()=>{
    const key=hgDatei,ifc=hgDateiIfc;hgGeschosse=[];hgGeschoss="";
    if(!ifc)return;const d=ifcDateien.find(d=>`${d.document_id}|${d.version}`===key);if(!d)return;
    let weg=false;
    void ifcApi.geschosse(s.projektId,{dokument_id:d.document_id,version:d.version}).then(g=>{if(!weg){hgGeschosse=g;hgGeschoss=g[0]?.global_id??""}}).catch(e=>{if(!weg)s.fehler=fehlertext(e)});
    return()=>{weg=true};
  });
  let hgDatei=$state(""),hgSeite=$state(1),hgStrecke=$state(10),hgZiel=$state(""),hgWinkel=$state(0);
  const el=$derived(s.eingabe?.elemente?.find(e=>e.id===elementAuswahl));
  const ELEMENTNAMEN:Record<string,string>={wand:"Wand",masskette:"Maßkette",symbol:"Planzeichen",raumstempel:"Raumstempel",text:"Text"};
  // Kopfzeile der Eigenschaften: Markierungen und Zeichenelemente zählen gemeinsam.
  const auswahlTitel=$derived.by(()=>{const n=auswahlIds.length+elementAuswahlIds.length;if(n>1)return `· ${n} ausgewählt`;if(m)return `· ${m.bezeichnung}`;if(el)return `· ${ELEMENTNAMEN[el.art]??"Zeichenelement"}`;return "· keine Auswahl"});
  const elementGesperrt=$derived(!darf||s.laeuft||!!s.eingabe?.ebenen.find(e=>e.id===el?.ebene_id)?.gesperrt);
  const istZeichnung=$derived(!!s.eingabe&&!s.eingabe.dokument);
  const einheitText=$derived(istZeichnung?"m":"Seitenpunkte");
  const hintergruende=$derived(s.eingabe?.hintergruende??[]);
  const hgAnsichten=$derived<Hintergrundansicht[]>(hintergruende.map(h=>{const q=hgQuellen[hgSchluessel(h)];return {h,pdfBase64:q?.pdfBase64,plan:q?.plan??null}}));
  const hgName=(h:Planhintergrund)=>{const d=hgDateien.find(d=>d.document_id===h.dokument.dokument_id&&d.version===h.dokument.version);return d?`${d.file_name}${h.geschoss?` · ${hgQuellen[hgSchluessel(h)]?.name??"Geschoss"}`:istDxf(d.file_name)?"":` · Seite ${h.seite}`}`:"Unterlage fehlt"};
  // Aus den Aufgaben vorbelegte Bedeutung und Zuordnung für die nächste Markierung.
  let vorbelegung=$state<Planaufgabe|null>(null);
  // Werkzeugkiste: Vorlagen aus dem Core; die gewählte Vorlage belegt Bedeutung, Namen und Zeichenebene der nächsten Markierung vor.
  let vorlagen=$state<Werkzeugvorlage[]>([]),vorlage=$state<Werkzeugvorlage|null>(null);
  const vorlagenGruppen=$derived([...new Map(vorlagen.map(v=>[v.gruppe,vorlagen.filter(x=>x.gruppe===v.gruppe)])).entries()]);
  const werkzeugFuer=(g:Werkzeugvorlage["geometrie"])=>g==="flaeche"?"polygon":g==="hoehenpunkt"?"hoehenpunkt":"strecke";
  onMount(()=>{planApi.werkzeugkiste().then(v=>vorlagen=v).catch(e=>{listenFehler=fehlertext(e)})});
  $effect(()=>{
    if(!vorlagenmenue)return;
    const zu=(e:PointerEvent)=>{if(!(e.target as Element|null)?.closest?.(".vorlagenmenue,.vorlagenpfeil"))vorlagenmenue=""};
    window.addEventListener("pointerdown",zu,true);
    return()=>window.removeEventListener("pointerdown",zu,true);
  });
  let editor=$state<HTMLDivElement>(), plan=$state<ReturnType<typeof PdfPlan>>();
  const m=$derived(s.eingabe?.markierungen.find(m=>m.id===auswahl));
  const nichtsGewaehlt=$derived(!m&&!el&&auswahlIds.length===0&&elementAuswahlIds.length===0);
  const gewaehlteEbene=$derived(nichtsGewaehlt?s.eingabe?.ebenen.find(e=>e.id===ebenenAuswahl):undefined);
  const filterAnzahl=$derived([bedeutungFilter,gebaeudeFilter,geschossFilter,nurFehler].filter(Boolean).length);
  const mitFilter=$derived(!!suche||filterAnzahl>0);
  const layer=$derived(s.eingabe?.ebenen.find(e=>e.id===ebene));
  const objektGesperrt=$derived(!darf||s.laeuft||!!s.eingabe?.ebenen.find(e=>e.id===m?.ebene_id)?.gesperrt);
  const quelleAlt=$derived(!!s.eingabe?.dokument&&dateien.some(d=>d.document_id===s.eingabe!.dokument!.dokument_id&&d.version>s.eingabe!.dokument!.version));
  const aktuelleDatei=$derived(dateien.find(d=>d.document_id===s.eingabe?.dokument?.dokument_id&&d.version===s.eingabe?.dokument?.version));
  const istDxfUnterlage=$derived(!!aktuelleDatei&&istDxf(aktuelleDatei.file_name));
  const pdfDateien=$derived(dateien.filter(d=>!istDxf(d.file_name)));
  const signatur=$derived(JSON.stringify(s.eingabe));
  const aktuelleWerte=$derived(signatur===messStand?werte:[]);
  const fehlerMap=$derived(new Map(markierungsFehler.map(f=>[f.id,f.fehler])));
  const gefilterteMarkierungen=$derived((s.eingabe?.markierungen??[]).filter(i=>(!aktiv||i.ausschnitt_id===aktiv)&&(!suche||i.bezeichnung.toLocaleLowerCase("de").includes(suche.toLocaleLowerCase("de")))&&(!bedeutungFilter||i.bedeutung===bedeutungFilter)&&(!gebaeudeFilter||i.gebaeude_id===gebaeudeFilter)&&(!geschossFilter||i.geschoss_id===geschossFilter)&&(!nurFehler||fehlerMap.has(i.id))));
  const wert=$derived(aktuelleWerte.find(w=>w.id===auswahl));
  const ohneEbene=$derived(gefilterteMarkierungen.filter(x=>!s.eingabe?.ebenen.some(e=>e.id===x.ebene_id)));
  const ausschnitte=$derived(s.eingabe?.ausschnitte??[]);
  const aktueller=$derived(ausschnitte.find(a=>a.id===aktiv));
  const kontrollStand=$derived(JSON.stringify([s.eingabe?.dokument,s.eingabe?.seite,aktueller?.id,aktueller?.kalibrierung??s.eingabe?.kalibrierung]));
  $effect(()=>{const stand=kontrollStand;untrack(()=>{if(kontrolle&&kontrolle.stand!==stand)kontrolle=null})});
  // Eigenschaften gelten für den geöffneten, sonst für den im Blatt gewählten Ausschnitt.
  const bearbeiteter=$derived(aktueller??ausschnitte.find(a=>a.id===rahmenAuswahl));
  const hoehenMoeglich=$derived(!aktueller||mitHoehen(aktueller.art));
  const nullpunkte=$derived(gebaeude.flatMap(g=>g.daten.geometrie?.nullpunkt_m_nhn!=null?[{name:g.daten.bezeichnung,wert:g.daten.geometrie.nullpunkt_m_nhn}]:[]));
  let listenGeneration=0;
  // Verwendung der Markierungen in Berechnungen und Fachobjekten (Nachverfolgung in den Eigenschaften).
  let verwendungen=$state<Map<string,Verwendung[]>>(new Map());
  const navigation=projektNavigation();
  // „Im Editor zeigen“: Sprungauftrag auf eine Markierung dieses Aufmaßes ausführen, sobald es geladen ist.
  let fokusNummer=0;
  $effect(()=>{
    const a=navigation?.auftrag,id=s.id,geladen=!!s.eingabe;
    if(!a||a.nummer===fokusNummer||!geladen||a.ziel.bereich!=="planaufmass"||a.ziel.objekt!==id||a.ziel.register!=="markierung"||!a.ziel.eintrag)return;
    fokusNummer=a.nummer;
    const m=s.eingabe!.markierungen.find(m=>m.id===a.ziel.eintrag);
    if(!m){s.meldung="Die gesuchte Markierung ist in diesem Aufmaß nicht mehr vorhanden.";return}
    untrack(()=>{if(m.ausschnitt_id&&aktiv!==m.ausschnitt_id)oeffnen(m.ausschnitt_id);waehlen(m.id);panelOeffnen("eigenschaften");void tick().then(()=>plan?.aufAuswahl())});
  });
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
    const [d,g,b,v]=await Promise.allSettled([api.dokumente(s.projektId),api.grundstueckskontext(s.projektId),plattformApi.liste<Gebaeude>(s.projektId,"gebaeude"),verwendungenLaden(projekt)]);
    if(n!==listenGeneration)return;
    if(v.status==="fulfilled")verwendungen=v.value;
    const ordnen=(a:DocumentVersion,b:DocumentVersion)=>a.file_name.localeCompare(b.file_name,"de")||b.version-a.version;
    if(d.status==="fulfilled"){dateien=d.value.filter(d=>istPlan(d.file_name)).sort(ordnen);ifcDateien=d.value.filter(d=>istIfc(d.file_name)).sort(ordnen)}
    grund=g.status==="fulfilled"?g.value:[];
    gebaeude=b.status==="fulfilled"?b.value:[];
    listenFehler=[...extra,...[d,g,b].flatMap(r=>r.status==="rejected"?[fehlertext(r.reason)]:[])].join(" · ");
    unterlagenGeladen=true;
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
    const vorhanden=s.liste.filter(f=>f.daten.dokument?.dokument_id===id).sort((a,b)=>(b.daten.dokument?.version??0)-(a.daten.dokument?.version??0))[0];
    if(modus==="liste"){if(vorhanden)onoeffnen?.(vorhanden);else onneu?.({dokument:{id:d.document_id,version:d.version,name:d.file_name}});return}
    if(vorhanden&&s.id===vorhanden.objekt)return;
    if(s.eingabe&&s.hatAenderungen()){
      if(!(await ask(`„${d.file_name}“ im Aufmaß öffnen? Offene Änderungen werden zuerst gespeichert.`,{title:"Unterlage öffnen",okLabel:"Speichern und öffnen",cancelLabel:"Hier bleiben"})))return;
      if(!(await s.speichern()))return;
    }
    if(vorhanden){s.bearbeiten(vorhanden);return}
    s.neu();if(!s.eingabe)return;
    s.eingabe={...s.leer(),bezeichnung:istDxf(d.file_name)?d.file_name:`${d.file_name} · Seite 1`,dokument:{dokument_id:d.document_id,version:d.version},seite:1};
    panel="eigenschaften";
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
  // Zeichnungs-Tab: Startauftrag ausführen, sobald Liste und Unterlagen da sind; danach meldet das Schließen des Eintrags den Tab ab.
  let gestartet=$state(false);
  $effect(()=>{
    const st=start,bereit=s.bereit,liste=s.liste,unterlagen=dateien;
    if(modus!=="tab"||!st||!bereit||gestartet||laedt||!unterlagenGeladen)return;
    untrack(()=>{gestartet=true;void tabStarten(st,liste,unterlagen)});
  });
  async function tabStarten(st:Tabstart,liste:Datensatz<Planaufmass>[],unterlagen:DocumentVersion[]){
    if(s.gerettet||s.eingabe)return;
    if(st.id){
      try{const f=liste.find(f=>f.objekt===st.id)??await plattformApi.lesen<Planaufmass>(s.projektId,s.art,st.id);if(f)s.bearbeiten(f);else s.fehler="Das Aufmaß ist nicht mehr vorhanden."}catch(e){s.fehler=fehlertext(e)}
      return;
    }
    s.neu();if(!s.eingabe)return;
    if(st.zeichnung){s.eingabe=leereZeichnung();panel="eigenschaften";return}
    const d=st.dokument?unterlagen.find(x=>x.document_id===st.dokument!.id&&x.version===st.dokument!.version):undefined;
    if(d){s.eingabe={...s.leer(),bezeichnung:istDxf(d.file_name)?d.file_name:`${d.file_name} · Seite 1`,dokument:{dokument_id:d.document_id,version:d.version},seite:1};panel="eigenschaften"}
  }
  $effect(()=>{if(modus==="tab"&&gestartet&&!s.eingabe&&!s.gerettet&&!s.laeuft)untrack(()=>onschliessen?.())});
  $effect(()=>{const titel=s.eingabe?.bezeichnung??"",geaendert=s.hatAenderungen(),id=s.original?s.id:null;untrack(()=>onstatus?.({titel,geaendert,id}))});
  /** Tab schließen: offene Änderungen nach Rückfrage verwerfen. */
  export async function tabSchliessen(){
    if(s.hatAenderungen()&&!(await ask("Die ungespeicherten Änderungen verwerfen?",{title:"Aufmaß",kind:"warning",okLabel:"Verwerfen",cancelLabel:"Weiter bearbeiten"})))return false;
    await s.verwerfen();return true;
  }
  const vormerken=ansichtsAbfrage(()=>s.projektId,["dokumente","fachobjekte"],listen);
  $effect(()=>{
    void projekt.version;untrack(vormerken);
  });
  $effect(()=>{
    const id=s.id;
    untrack(()=>{auswahl="";auswahlIds=[];elementAuswahl="";elementAuswahlIds=[];hgZiel="";betrachterSeite=null;sichtbarkeit={};ebene=s.eingabe?.ebenen[0]?.id??"";undo=0;redo=0;if(s.eingabe)verlauf.reset($state.snapshot(s.eingabe));werkzeug="auswahl";aktiv="";rahmenAuswahl="";
      try{const v=JSON.parse(localStorage.getItem(`bauprojekt.planpanel.${benutzerId}.${s.projektId}.${id}`)??"null");if(v){panel=["aufgaben","objekte","eigenschaften"].includes(v.panel)?v.panel:"aufgaben";suche=typeof v.suche==="string"?v.suche.slice(0,500):"";aktiv=v.aktiv??"";auswahl=v.auswahl??"";auswahlIds=auswahl?[auswahl]:[];bedeutungFilter=v.bedeutungFilter??"";gebaeudeFilter=v.gebaeudeFilter??"";geschossFilter=v.geschossFilter??"";nurFehler=v.nurFehler===true}}catch{}
      if(!s.eingabe?.kalibrierung){panel="eigenschaften";auswahl="";auswahlIds=[]}
    });
  });
  // Ein gespeicherter Stand ist eine eigene Rückgängig-Stufe: Eingaben danach
  // werden nicht mit den Eingaben davor zusammengefasst.
  $effect(()=>{void s.revision;untrack(()=>{verlaufGruppe="";verlauf.gruppeBeenden()})});
  $effect(()=>{const sig=signatur;if(!sig||sig==="null")return;untrack(()=>{verlauf.beobachtenSignatur(sig,verlaufGruppe);undo=verlauf.zurueck;redo=verlauf.vor})});
  $effect(()=>{const v={panel,suche,aktiv,auswahl,bedeutungFilter,gebaeudeFilter,geschossFilter,nurFehler},id=s.id;try{localStorage.setItem(`bauprojekt.planpanel.${benutzerId}.${s.projektId}.${id}`,JSON.stringify(v))}catch{}});
  $effect(()=>{const q=vergleich;let gone=false;vergleichPdf=null;if(q){const [id,v]=q.split("|");void api.pdfVorschau(id,Number(v)).then(p=>{if(!gone)vergleichPdf=p}).catch(e=>{if(!gone)s.fehler=fehlertext(e)})}return()=>{gone=true}});
  $effect(()=>{
    const d=s.eingabe?.dokument, datei=aktuelleDatei;
    let gone=false;pdf=null;dxf=null;pdfFehler="";
    if(!d||!datei)return;
    void istZeichnung;
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
  // Unterlagen der Hintergründe laden (einmal je Dokumentversion); Fehler erscheinen als Blattfehler.
  $effect(()=>{
    const hs=hintergruende, liste=hgDateien; if(!sichtbar)return;
    for(const h of hs){
      const key=hgSchluessel(h);
      if(hgQuellen[key]||hgLaeuft.has(key))continue;
      const d=liste.find(x=>x.document_id===h.dokument.dokument_id&&x.version===h.dokument.version); if(!d)continue;
      hgLaeuft.add(key);
      const lauf=h.geschoss?Promise.all([ifcApi.grundrissPlan(s.projektId,h.dokument,h.geschoss),ifcApi.geschosse(s.projektId,h.dokument)]).then(([plan,gs])=>{hgQuellen={...hgQuellen,[key]:{plan,name:gs.find(g=>g.global_id===h.geschoss)?.name??"Geschoss"}}}):istDxf(d.file_name)?api.cadPlan(d.document_id,d.version).then(c=>{hgQuellen={...hgQuellen,[key]:{plan:c.plan,name:d.file_name}}}):api.pdfVorschau(d.document_id,d.version).then(p=>{hgQuellen={...hgQuellen,[key]:{pdfBase64:p.pdf_base64,name:d.file_name}}});
      void lauf.catch(e=>{pdfFehler=`Hintergrund ${d.file_name}: ${fehlertext(e)}`}).finally(()=>hgLaeuft.delete(key));
    }
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
    // Auch neu messen, wenn sich darüber etwas ändert (Aktenkopf ein-/ausgeklappt,
    // Hinweisleisten) oder ein umgebender Bereich scrollt.
    const beobachter=new ResizeObserver(messen);
    for(let p=el.parentElement;p&&p!==document.body;p=p.parentElement)beobachter.observe(p);
    window.addEventListener("resize",messen);document.addEventListener("scroll",messen,{passive:true,capture:true});
    return()=>{cancelAnimationFrame(frame);beobachter.disconnect();window.removeEventListener("resize",messen);document.removeEventListener("scroll",messen,{capture:true})};
  });
  // Tastaturkürzel für Werkzeuge und Auswahl; Eingabefelder bleiben unberührt.
  // Kürzel gelten, solange die letzte Bedienung im Editor war, auch wenn der
  // Fokus danach auf das Dokument fiel (z. B. nach dem Entfernen eines Eintrags).
  let imEditor=false,werkzeugleiste=$state<HTMLElement>();
  // Editor und seine Werkzeugleiste (liegt in der Kopfzeile der Liste) gelten
  // gemeinsam als Editor: nach „Rückgängig“ per Knopf wirkt Strg+Umschalt+Z.
  const imBereich=(n:EventTarget|null)=>!!n&&(!!editor?.contains(n as Node)||!!werkzeugleiste?.contains(n as Node));
  $effect(()=>{
    const merke=(e:Event)=>{imEditor=imBereich(e.target)};
    document.addEventListener("pointerdown",merke,true);document.addEventListener("focusin",merke,true);
    return()=>{document.removeEventListener("pointerdown",merke,true);document.removeEventListener("focusin",merke,true)};
  });
  $effect(()=>{
    const taste=(e:KeyboardEvent)=>{
      if(!sichtbar||!s.eingabe||(e.target as Element).closest("input,select,textarea,[contenteditable]"))return;
      const drin=imBereich(e.target)||(e.target===document.body&&imEditor);
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
      if(e.key==="F2"){e.preventDefault();befehlFeld?.focus();return}
      if(e.key==="Escape"){
        if(vorlagenmenue){vorlagenmenue="";return}
        if(menue){menue="";return}
        if(werkzeugfenster){werkzeugfenster="";return}
        if(werkzeug!=="auswahl")werkzeug="auswahl";
        else if(!nichtsGewaehlt||ebenenAuswahl)auswahlAufheben();
        else if(aktiv)schliessen();
        else if(vollbild)vollbild=false;
        return;
      }
      const w=werkzeuge.find(w=>w[2].toLowerCase()===k);
      if(w&&!werkzeugGesperrt(w[0])){e.preventDefault();werkzeugWaehlen(w[0]);return}
      if(k==="z"){e.preventDefault();plan?.zoomfensterStarten();return}
      if(e.key==="Delete"&&!m&&auswahlElemente.length&&darf){e.preventDefault();elementEntfernen()}
      else if(e.key==="Delete"&&m&&!objektGesperrt){e.preventDefault();void entfernen()}
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
  async function entfernen(){
    if(!s.eingabe||!m||objektGesperrt)return;
    const ids=auswahlIds.length?auswahlIds:[auswahl];
    if(ids.some(id=>s.eingabe!.ebenen.find(e=>e.id===s.eingabe!.markierungen.find(m=>m.id===id)?.ebene_id)?.gesperrt))return;
    // Verknüpfte Werte verlieren ihre Quelle: vorher nennen, was daran hängt.
    const genutzt=ids.flatMap(id=>verwendungen.get(id)??[]);
    if(genutzt.length&&!(await ask(`Diese Markierung liefert Werte für:\n${[...new Set(genutzt.map(v=>v.verbraucher))].map(v=>`• ${v}`).join("\n")}\n\nNach dem Speichern des Aufmaßes melden diese Bereiche „Planmarkierung fehlt“. Dort die Verknüpfung lösen (der Wert bleibt als manuelle Angabe) oder neu verknüpfen. Markierung trotzdem entfernen?`,{title:"Verknüpfte Markierung entfernen",kind:"warning"})))return;
    merken();s.eingabe.markierungen=s.eingabe.markierungen.filter(i=>!ids.includes(i.id));auswahl="";auswahlIds=[];
  }
  function merken(){verlaufGruppe="";verlauf.gruppeBeenden();if(s.eingabe)verlauf.beobachten($state.snapshot(s.eingabe))}
  // Zeichenelemente: anlegen auf der aktiven Ebene, ändern per Ziehen, entfernen.
  function elementAnlegen(f:Zeichenform){
    const p=s.eingabe;if(!p||!darf||s.laeuft)return;
    if(!layer||layer.gesperrt||!layer.sichtbar){s.fehler="Eine sichtbare, ungesperrte Zeichenebene auswählen.";return}
    merken();
    const e:Zeichenelement={id:crypto.randomUUID(),ebene_id:layer.id,...f};
    p.elemente=[...(p.elemente??[]),e];
    auswahl="";auswahlIds=[];elementAuswahl=e.id;panel="eigenschaften";
    if(f.art==="text")werkzeug="auswahl";
  }
  function elementSetzen(id:string,e:Zeichenelement){
    const p=s.eingabe;if(!p||elementGesperrt)return;
    const i=(p.elemente??[]).findIndex(x=>x.id===id);if(i<0)return;
    merken();p.elemente![i]=e;
  }
  function elementEntfernen(){
    const p=s.eingabe;if(!p||!darf||s.laeuft)return;
    const ids=new Set(auswahlElemente.map(e=>e.id));if(!ids.size)return;
    if(auswahlElemente.some(e=>p.ebenen.find(x=>x.id===e.ebene_id)?.gesperrt)){s.fehler="Gesperrte Ebenen zuerst entsperren.";return}
    merken();p.elemente=(p.elemente??[]).filter(e=>!ids.has(e.id));elementAuswahl="";elementAuswahlIds=[];
  }
  function elementWaehlen(id:string){if(id)ebenenAuswahl="";elementAuswahl=id;elementAuswahlIds=id?[id]:[];if(id){auswahl="";auswahlIds=[];if(!AUSWAHLWERKZEUGE.includes(werkzeug))werkzeug="auswahl"}}
  // Fensterauswahl: Markierungen und Zeichenelemente gemeinsam; Shift/Strg ergänzt.
  function fensterWaehlen(ids:string[],eids:string[],mehrfach:boolean){
    betrachterSeite=null;
    if(mehrfach){ids=[...new Set([...auswahlIds,...ids])];eids=[...new Set([...elementAuswahlIds,...eids])]}
    auswahlIds=ids;auswahl=ids.at(-1)??"";elementAuswahlIds=eids;elementAuswahl=!ids.length&&eids.length===1?eids[0]:"";punktIndex=0;
    if(ids.length+eids.length)panelOeffnen("eigenschaften");
  }
  // Ziel der Konstruktionsbefehle: alle gewählten Markierungen und Elemente.
  const auswahlMarkierungen=$derived.by(()=>{const ids=auswahlIds.length?auswahlIds:auswahl?[auswahl]:[];return s.eingabe?.markierungen.filter(x=>ids.includes(x.id))??[]});
  const auswahlElemente=$derived.by(()=>{const ids=elementAuswahlIds.length?elementAuswahlIds:elementAuswahl?[elementAuswahl]:[];return (s.eingabe?.elemente??[]).filter(x=>ids.includes(x.id))});
  const auswahlAnzahl=$derived(auswahlMarkierungen.length+auswahlElemente.length);
  type Abbild={g:(x:Plangeometrie)=>Plangeometrie|null;e:(x:Zeichenelement)=>Zeichenelement|null};
  /**
   * Wendet Abbildungen auf die Auswahl an. `kopie`: jede Abbildung erzeugt Kopien
   * (Reihe = mehrere Abbildungen), sonst wird die Auswahl selbst geändert.
   * Nicht anwendbare Objekte (z. B. Stutzen eines Polygons) bleiben unverändert.
   */
  function konstruieren(titel:string,abbilder:Abbild[],kopie:boolean){
    const p=s.eingabe;if(!p||!darf||s.laeuft)return;
    const ms=auswahlMarkierungen,es=auswahlElemente;
    if(!ms.length&&!es.length){s.fehler="Zuerst Objekte auswählen (Klick, Shift+Klick oder Fenster aufziehen).";return}
    if([...ms,...es].some(x=>p.ebenen.find(e=>e.id===x.ebene_id)?.gesperrt)){s.fehler="Gesperrte Ebenen zuerst entsperren.";return}
    let geaendert=0,ausgelassen=0;
    const neueM:Planmarkierung[]=[],neueE:Zeichenelement[]=[];
    const plan=abbilder.map(a=>({m:ms.map(m=>a.g($state.snapshot(m.geometrie))),e:es.map(e=>a.e($state.snapshot(e)))}));
    if(plan.every(x=>x.m.every(g=>!g)&&x.e.every(e=>!e))){s.fehler=`${titel}: Für die gewählten Objekte nicht möglich.`;return}
    merken();
    plan.forEach((x,n)=>{
      const ids=new Map(ms.map(m=>[m.id,crypto.randomUUID()]));
      ms.forEach((m,i)=>{const g=x.m[i];if(!g){ausgelassen++;return}geaendert++;
        if(!kopie){m.geometrie=g;return}
        const k=structuredClone($state.snapshot(m));
        neueM.push({...k,id:ids.get(m.id)!,bezeichnung:`${m.bezeichnung} (${titel}${abbilder.length>1?` ${n+1}`:""})`,geometrie:g,hoehenzone_von:m.hoehenzone_von?ids.get(m.hoehenzone_von)??m.hoehenzone_von:undefined});
      });
      es.forEach((e,i)=>{const r=x.e[i];if(!r){ausgelassen++;return}geaendert++;
        if(!kopie){const j=(p.elemente??[]).findIndex(y=>y.id===e.id);if(j>=0)p.elemente![j]=r;return}
        neueE.push({...r,id:crypto.randomUUID()});
      });
    });
    if(kopie){
      p.markierungen.push(...neueM);p.elemente=[...(p.elemente??[]),...neueE];
      auswahlIds=neueM.map(m=>m.id);auswahl=auswahlIds.at(-1)??"";elementAuswahlIds=neueE.map(e=>e.id);elementAuswahl=!neueM.length&&neueE.length===1?neueE[0].id:"";
    }
    s.meldung=`${titel}: ${geaendert} ${kopie?"Kopie(n) angelegt":"Objekt(e) geändert"}${ausgelassen?`, ${ausgelassen} nicht anwendbar übersprungen`:""}.${neueM.length?" Kopierte Markierungen behalten Bedeutung und Zuordnung; Verknüpfungen in Berechnungen gelten weiter nur für das Original.":""}`;
  }
  const blattFaktor=()=>meterProPunkt(aktueller?.kalibrierung??s.eingabe?.kalibrierung??null);
  function auswahlKopieren(von:Punkt,bis:Punkt){
    const dx=bis[0]-von[0],dy=bis[1]-von[1];
    konstruieren("Kopie",[{g:x=>verschieben(x,dx,dy),e:x=>abbildenElement(x,verschiebung(dx,dy))}],true);
  }
  function auswahlSpiegeln(a:Punkt,b:Punkt){
    konstruieren(konstruktion.spiegelKopie?"Spiegelung":"gespiegelt",[{g:x=>abbildenGeometrie(x,spiegelung(a,b)),e:x=>elementSpiegeln(x,a,b)}],konstruktion.spiegelKopie);
  }
  function auswahlStutzen(a:Punkt,b:Punkt){
    konstruieren("Stutzen / Dehnen",[{g:x=>geometrieStutzen(x,a,b),e:x=>elementStutzen(x,a,b)}],false);
  }
  function auswahlVersatz(){
    const f=blattFaktor();if(f===null){s.fehler="Versatz benötigt einen Maßstab.";return}
    const d=konstruktion.versatz/f;
    konstruieren("Versatz",[{g:x=>versatz(x,d),e:x=>elementVersatz(x,d)}],true);
  }
  function auswahlReihe(){
    const f=blattFaktor();if(f===null){s.fehler="Reihe benötigt einen Maßstab.";return}
    const vs=reihe(konstruktion.anzahl,konstruktion.dx/f,-konstruktion.dy/f);
    if(!vs.length){s.fehler="Reihe: Anzahl ab 2 und einen Abstand angeben.";return}
    konstruieren("Reihe",vs.map(([dx,dy])=>({g:(x:Plangeometrie)=>verschieben(x,dx,dy),e:(x:Zeichenelement)=>abbildenElement(x,verschiebung(dx,dy))})),true);
  }
  // Hintergründe eigener Zeichnungen.
  function hintergrundHinzufuegen(){
    const p=s.eingabe;if(!p||!darf||!istZeichnung)return;
    const d=hgDateien.find(d=>`${d.document_id}|${d.version}`===hgDatei);if(!d){s.fehler="Eine Unterlage für den Hintergrund wählen.";return}
    const ifc=istIfc(d.file_name);
    if(ifc&&!hgGeschoss){s.fehler="Ein Geschoss des IFC-Modells wählen.";return}
    merken();
    const dxfDatei=istDxf(d.file_name)||ifc;
    const h=hintergrundNeu({dokument_id:d.document_id,version:d.version},dxfDatei?1:Math.max(1,Math.round(hgSeite)),dxfDatei?1:mppAusMassstab(nenner));
    if(ifc){h.geschoss=hgGeschoss;p.hintergruende=[...hintergruende,h];hgZiel=h.id;s.meldung="IFC-Grundriss eingefügt: Er liegt in Modellkoordinaten (Meter); Geschosse desselben Modells liegen deckungsgleich.";return}
    // Neue Hintergründe liegen versetzt, damit sie nicht übereinander verschwinden.
    h.lage=[hintergruende.length*2,hintergruende.length*2];
    p.hintergruende=[...hintergruende,h];hgZiel=h.id;
    if(dxfDatei)s.meldung="DXF-Hintergrund eingefügt: Zeichnungseinheit prüfen oder über eine bekannte Strecke einpassen.";
  }
  function hintergrundAendern(id:string,f:(h:Planhintergrund)=>Planhintergrund){
    const p=s.eingabe;if(!p||!darf)return;
    const i=hintergruende.findIndex(h=>h.id===id);if(i<0)return;
    merken();p.hintergruende![i]=f($state.snapshot(hintergruende[i]));
  }
  function hintergrundEntfernen(id:string){const p=s.eingabe;if(!p||!darf)return;merken();p.hintergruende=hintergruende.filter(h=>h.id!==id);if(hgZiel===id)hgZiel=""}
  const hgEinheit=(h:Planhintergrund)=>Object.entries(EINHEIT_METER).find(([,m])=>Math.abs(m-h.meter_pro_punkt)<1e-12)?.[0]??"";
  // Grund, warum ein Werkzeug gerade nicht wählbar ist; leer = wählbar.
  function werkzeugGesperrt(id:string):string{
    if(id==="auswahl"||id==="verschieben")return "";
    if(!darf)return "Nur mit Schreibrecht";
    if(id==="hoehenpunkt"&&!hoehenMoeglich)return "Höhen nur in Schnitt, Ansicht oder Detail";
    if(id==="bezugslinie")return !bearbeiteter?"Zuerst einen Ausschnitt wählen":!mitHoehen(bearbeiteter.art)?"Bezugslinie nur in Schnitt, Ansicht oder Detail":"";
    if(id==="ausschnitt")return aktiv?"Zuerst den Ausschnitt schließen":!(pdf||dxf||istZeichnung)?"Keine Unterlage geladen":"";
    if(id==="kalibrieren")return istZeichnung?"Eigene Zeichnungen rechnen in Metern":!(pdf||dxf)?"Keine Unterlage geladen":"";
    if(id==="kontrolle")return !(pdf||dxf||istZeichnung)?"Keine Unterlage geladen":"";
    if(id==="wand"||id==="masskette"||id==="text")return !(pdf||dxf||istZeichnung)?"Keine Unterlage geladen":"";
    if(id==="kopieren"||id==="spiegeln"||id==="stutzen")return !auswahlAnzahl?"Zuerst Objekte auswählen":"";
    if(id==="versatz"||id==="reihe")return blattFaktor()===null?"Zuerst den Maßstab festlegen":"";
    if(id==="symbol"||id==="raumstempel")return !(pdf||dxf||istZeichnung)?"Keine Unterlage geladen":"";
    if(id==="oeffnung")return !s.eingabe?.elemente?.some(e=>e.art==="wand")?"Zuerst Wände zeichnen":"";
    return "";
  }
  function werkzeugWaehlen(id:string){
    if(werkzeugGesperrt(id))return;
    vorbelegung=null;vorlage=null;vorlagenmenue="";
    if(id==="ausschnitt")rahmenAuswahl="";
    if((id==="bezugslinie"||id==="kalibrieren")&&bearbeiteter&&aktiv!==bearbeiteter.id&&id==="bezugslinie")oeffnen(bearbeiteter.id);
    werkzeug=id;
    if(id!=="auswahl"&&id!=="verschieben"&&werkzeuge.some(w=>w[0]===id))letzterBefehl=id;
  }
  function befehlAusfuehren(text:string){
    const t=text.trim();if(!t)return;
    const b=befehlAuswerten(t)??(werkzeuge.some(w=>w[0]===t)?{befehl:t,zahlen:[]}:null);
    if(!b){s.fehler=`Unbekannter Befehl „${t}“. Kürzel: L Strecke, PL Polygon, R Rechteck, K Kreis, W Wand, BM Maßkette, T Text, KO Kopieren, SP Spiegeln, ST Stutzen/Dehnen, VS Versatz, RH Reihe, E Entfernen, U Rückgängig, OR Ortho, WR Winkelraster.`;return}
    const [a,c,d]=b.zahlen;
    switch(b.befehl){
      case "versatz":if(a!==undefined)konstruktion.versatz=a;auswahlVersatz();break;
      case "reihe":if(a!==undefined)konstruktion.anzahl=a;if(c!==undefined)konstruktion.dx=c;if(d!==undefined)konstruktion.dy=d;auswahlReihe();break;
      case "entfernen":if(!m&&auswahlElemente.length)elementEntfernen();else void entfernen();return;
      case "zurueck":rueckwaerts();return;
      case "zoomfenster":plan?.zoomfensterStarten();return;
      case "einpassen":plan?.einpassen();return;
      case "ortho":plan?.orthoUmschalten();return;
      case "winkelraster":plan?.winkelrasterSetzen(a??0);return;
      default:{
        const grund=werkzeugGesperrt(b.befehl);
        if(grund){s.fehler=`${werkzeuge.find(w=>w[0]===b.befehl)?.[1]??b.befehl}: ${grund}.`;return}
        werkzeugWaehlen(b.befehl);return;
      }
    }
    letzterBefehl=t;
  }
  function wiederholen(){if(letzterBefehl&&werkzeug==="auswahl")befehlAusfuehren(letzterBefehl)}
  function verlaufStatus(){const eids=new Set(s.eingabe?.elemente?.map(e=>e.id));elementAuswahlIds=elementAuswahlIds.filter(id=>eids.has(id));undo=verlauf.zurueck;redo=verlauf.vor;const ids=new Set(s.eingabe?.markierungen.map(m=>m.id));auswahlIds=auswahlIds.filter(id=>ids.has(id));if(!ids.has(auswahl))auswahl="";if(!s.eingabe?.elemente?.some(e=>e.id===elementAuswahl))elementAuswahl="";if(!s.eingabe?.ausschnitte?.some(a=>a.id===aktiv))aktiv="";plan?.abbrechen()}
  function rueckwaerts(){
    if(!s.eingabe||!darf||s.laeuft)return;
    const p=verlauf.rueckwaerts($state.snapshot(s.eingabe));if(p)s.eingabe=p;verlaufStatus();
  }
  function vorwaerts(){
    if(!s.eingabe||!darf||s.laeuft)return;
    const p=verlauf.vorwaerts($state.snapshot(s.eingabe));if(p)s.eingabe=p;verlaufStatus();
  }
  function waehlen(id:string,mehrfach=false){betrachterSeite=null;if(id)ebenenAuswahl="";if(id){elementAuswahl="";if(!mehrfach)elementAuswahlIds=[]}else if(!mehrfach)elementAuswahlIds=[];if(mehrfach){auswahlIds=auswahlIds.includes(id)?auswahlIds.filter(x=>x!==id):[...auswahlIds,id];auswahl=auswahlIds.at(-1)??""}else{auswahl=id;auswahlIds=id?[id]:[]}punktIndex=0;if(!AUSWAHLWERKZEUGE.includes(werkzeug))werkzeug="auswahl"}
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
    if(werkzeug==="hgstrecke"&&g.art==="strecke"){
      if(!(hgStrecke>0)){s.fehler="Die tatsächliche Länge der Strecke in Metern angeben.";return}
      hintergrundAendern(hgZiel,h=>hgSkalieren(h,g.von,g.bis,hgStrecke));werkzeug="auswahl";return;
    }
    if(werkzeug==="hgverschieben"&&g.art==="strecke"){hintergrundAendern(hgZiel,h=>hgVerschieben(h,g.von,g.bis));werkzeug="auswahl";return}
    if(werkzeug==="hgdrehen"&&g.art==="hoehenpunkt"){hintergrundAendern(hgZiel,h=>hgDrehen(h,g.punkt,hgWinkel));werkzeug="auswahl";return}
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
    if((werkzeug==="symbol"||werkzeug==="raumstempel"||werkzeug==="oeffnung")&&g.art==="hoehenpunkt"){
      if(!layer||layer.gesperrt||!layer.sichtbar){if(werkzeug!=="oeffnung"){s.fehler="Eine sichtbare, ungesperrte Zeichenebene auswählen.";return}}
      einpunktElement(werkzeug,g.punkt as Punkt);return;
    }
    if((werkzeug==="kopieren"||werkzeug==="spiegeln"||werkzeug==="stutzen")&&g.art==="strecke"){
      const a=g.von as Punkt,b=g.bis as Punkt,w=werkzeug;werkzeug="auswahl";
      if(w==="kopieren")auswahlKopieren(a,b);else if(w==="spiegeln")auswahlSpiegeln(a,b);else auswahlStutzen(a,b);
      return;
    }
    if(!layer||layer.gesperrt||!layer.sichtbar){s.fehler="Eine sichtbare, ungesperrte Zeichenebene auswählen.";return}
    merken();
    const id=crypto.randomUUID();
    const v=vorbelegung,t=vorlage;
    const neu:Planmarkierung={id,ebene_id:layer.id,bezeichnung:v?`${v.gruppe} · ${v.titel}`:`Markierung ${p.markierungen.length+1}`,bedeutung:v?.bedeutung??"unbestimmt",geometrie:g,
      grundstueck_id:v?.grundstueck_id??(grund.length===1?grund[0].id:null),gebaeude_id:v?.gebaeude_id??standardGebaeude,geschoss_id:v?.geschoss_id??standardGeschoss};
    if(!v)neu.bedeutung=standardBedeutung;
    // Vorlage der Werkzeugkiste: Bedeutung und laufender Name („DIN 277 NUF 3“).
    if(!v&&t){neu.bedeutung=t.bedeutung;neu.bezeichnung=`${t.titel} ${p.markierungen.filter(m=>m.bedeutung===t.bedeutung).length+1}`}
    if(v?.flurstueck_id)neu.flurstueck_id=v.flurstueck_id;
    if(aktiv)neu.ausschnitt_id=aktiv;
    p.markierungen.push(neu);
    waehlen(id);vorbelegung=null;vorlage=null;panel="eigenschaften";
  }
  // Vorlage wählen: Werkzeug, Bedeutung und Zeichenebene in einem Schritt; die Ebene entsteht beim ersten Gebrauch.
  function vorlageWaehlen(t:Werkzeugvorlage){
    const p=s.eingabe;if(!p||!darf)return;
    const id=werkzeugFuer(t.geometrie);
    if(werkzeugGesperrt(id))return;
    if(t.geometrie==="hoehenpunkt"&&!hoehenMoeglich){s.meldung=`„${t.titel}“ ist eine Höhe: zuerst einen Schnitt oder eine Ansicht als Ausschnitt öffnen und dort die Bezugslinie setzen.`;return}
    // Im geöffneten Ausschnitt bleibt dessen Ebene maßgebend.
    if(!aktiv){
      let e=p.ebenen.find(e=>e.bezeichnung===t.ebene&&!e.gesperrt);
      if(!e){merken();e={id:crypto.randomUUID(),bezeichnung:t.ebene,sichtbar:true,gesperrt:false};p.ebenen.push(e)}
      e.sichtbar=true;ebene=e.id;
    }
    vorbelegung=null;vorlage=t;werkzeug=id;
  }
  // Flurstücksauftrag des Tabs: Grenze dieses Flurstücks zeichnen und die Fläche
  // übernehmen. Aufgabe, Messwert und Übernahme kommen vom Core wie im Aufgaben-Panel.
  const flurAuftrag=$derived(modus==="tab"?start?.flurstueck??null:null);
  const flurVorschau=new Uebernahme();
  let flurAufgabe=$state<Planaufgabe|null>(null),flurFehler=$state(""),flurLaeuft=$state(false),flurGeneration=0;
  $effect(()=>{
    const a=flurAuftrag,sig=signatur;
    if(!a||!s.eingabe){flurAufgabe=null;return}
    const n=++flurGeneration;
    const timer=setTimeout(()=>{void planApi.aufgaben(projekt.id,JSON.parse(sig)).then(l=>{if(n===flurGeneration){flurAufgabe=l.find(x=>x.verbraucher==="flurstueck"&&x.objekt===a.id)??null;flurFehler=flurAufgabe?"":"Das Flurstück ist nicht mehr gespeichert."}}).catch(e=>{if(n===flurGeneration)flurFehler=fehlertext(e)})},300);
    return()=>clearTimeout(timer);
  });
  /** Markierung dieses Aufmaßes für das Flurstück: noch frei oder schon verknüpft. */
  const flurMarkierung=(a:Planaufgabe|null|undefined,plan:Planaufmass|null|undefined)=>a?.markierung??(a?.gebunden&&plan?.markierungen.some(m=>m.id===a.gebunden)?a.gebunden:null);
  const flurFlaeche=$derived.by(()=>{const id=flurMarkierung(flurAufgabe,s.eingabe);return id?aktuelleWerte.find(w=>w.id===id)?.flaeche_m2??null:null});
  const ohneMassstab=$derived(!!s.eingabe?.dokument&&!istDxfUnterlage&&!(aktueller?.kalibrierung??s.eingabe?.kalibrierung));
  async function flurUebernehmen(){
    const a=flurAuftrag;if(!a||!darf||flurLaeuft)return;
    flurLaeuft=true;flurFehler="";
    try{
      // Die Bindung verweist auf den gespeicherten Stand dieses Aufmaßes.
      if((!s.original||s.hatAenderungen())&&!(await s.speichern()))return;
      const id=s.id;if(!id)throw Error("Aufmaß zuerst speichern.");
      const stand=await plattformApi.lesen<Planaufmass>(projekt.id,"planaufmass",id);if(!stand)throw Error("Aufmaß nicht mehr vorhanden.");
      const aufgabe=(await planApi.aufgaben(projekt.id,stand.daten)).find(x=>x.verbraucher==="flurstueck"&&x.objekt===a.id);
      if(!aufgabe)throw Error("Das Flurstück ist nicht mehr gespeichert.");
      const mid=flurMarkierung(aufgabe,stand.daten);
      if(!mid)throw Error("Noch keine Grenze für dieses Flurstück gezeichnet: „Grenze zeichnen“ wählen und das Polygon schließen.");
      const {basis,geprueft,vorher,nachher}=await flurstueckVorbereiten(projekt,a.id,{aufmass_id:id,revision:stand.revision,markierung_id:mid,groesse:aufgabe.groesse,ziel:aufgabe.ziel});
      const name=stand.daten.markierungen.find(m=>m.id===mid)?.bezeichnung??"Markierung";
      if(!(await flurVorschau.fragen([{ziel:`Flurstück ${a.bezeichnung} · Fläche`,vorher,nachher,quelle:`${stand.daten.bezeichnung} · ${name} · R${stand.revision}`}],true)))return;
      onprojekt?.(await flurstueckeSpeichern(basis,geprueft));
      onflurstueckfertig?.();
      const zurueck=navigation?.rueckweg;
      if(navigation&&zurueck?.bereich==="flurstuecke"){navigation.rueckweg=null;navigation.springen(zurueck)}
    }catch(e){flurFehler=fehlertext(e)}finally{flurLaeuft=false}
  }
  function aufgabeZeichnen(a:Planaufgabe){
    if(a.geometrie==="hoehenpunkt"&&!hoehenMoeglich){
      s.meldung=`„${a.titel}“ ist eine Höhe: zuerst einen Schnitt oder eine Ansicht als Ausschnitt öffnen und dort die Bezugslinie setzen.`;return;
    }
    vorbelegung=a;
    werkzeug=a.geometrie==="flaeche"?"polygon":a.geometrie==="hoehenpunkt"?"hoehenpunkt":"strecke";
  }
  async function seiteWechseln(n:number){if(!s.eingabe?.dokument||n===s.eingabe.seite||istDxfUnterlage)return;await quelleWechsel(s.eingabe.dokument,n)}
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
    s.neu();s.eingabe=kopie;vergleich="";vergleichBestaetigt=false;panel="eigenschaften";s.meldung="Übertragener Entwurf: Konturen prüfen, Maßstäbe kalibrieren und Höhenbezüge neu festlegen. Bestehende Berechnungen verwenden weiterhin das alte Aufmaß.";
  }
  async function exportieren(format:"pdf"|"csv"){
    if(!s.eingabe||!s.id||exportLaeuft)return;
    if(s.hatAenderungen()){
      if(!darf||!(await ask("Für die Ausgabe den aktuellen Aufmaßentwurf zuerst speichern?",{title:"Aufmaß ausgeben",okLabel:"Speichern und ausgeben",cancelLabel:"Abbrechen"}))||!(await s.speichern()))return;
    }
    const ziel=await save({defaultPath:`${s.eingabe.bezeichnung.replace(/[<>:"/\\|?*]/g,"_")}.${format}`,filters:[{name:format==="pdf"?"Aufmaßblatt PDF":"Messwerte CSV",extensions:[format]}]});if(!ziel)return;
    exportLaeuft=true;try{await planApi.exportieren(s.projektId,{aufmass_id:s.id,revision:s.revision,format,nur_sichtbare_ebenen:nurSichtbare},ziel);s.meldung=`${format.toUpperCase()} ausgegeben: ${ziel}`}catch(e){s.fehler=fehlertext(e)}finally{exportLaeuft=false}
  }
  async function quelleWechsel(dokument:NonNullable<Planaufmass["dokument"]>,seite:number){
    const p=s.eingabe;if(!p||!darf||s.laeuft||!Number.isInteger(seite)||seite<1)return;
    if(p.dokument?.dokument_id===dokument.dokument_id&&p.dokument.version===dokument.version&&p.seite===seite)return;
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

<UebernahmeVorschau v={flurVorschau}/>
<Fachliste titel="Aufmaß" {s} bereich="planaufmass" {darf} lesendBedienbar={true} zusatzBeschaeftigt={()=>laedt} kompakt={true} werkzeuge={vollbild?undefined:editorwerkzeuge} oeffnen={modus==="liste"?f=>onoeffnen?.(f):undefined} neu={modus==="liste"?()=>onneu?.():undefined}>
  {#snippet children(p)}
    {#if listenFehler}<p class="fehlertext" role="alert">{listenFehler}</p>{/if}
    {#if !dateien.length&&!istZeichnung}<p class="hinweisbox">Noch keine Unterlage vorhanden. Zuerst eine PDF oder DXF unter „Projektdateien“ importieren – oder eine eigene Zeichnung in Metern anlegen.</p>{/if}
    <Quellenzustand projektId={projekt.id} version={projekt.version} />
    {#if quelleAlt}<p class="hinweisbox" role="status">Eine neuere Version der Unterlage liegt vor. Dieses Aufmaß gehört weiterhin zur älteren Unterlage; Berechnungsübernahmen sind bis zur Prüfung der neuen Version gesperrt.</p>{/if}
    {#if pdfFehler}<p class="fehlertext" role="alert">{pdfFehler}</p>{/if}
    <div class="editor" class:vollbild bind:this={editor} style:height={vollbild?undefined:`${hoeheEditor}px`} onpointerdowncapture={()=>{verlaufGruppe="";verlauf.gruppeBeenden()}} oninputcapture={e=>{const el=e.target as HTMLElement;verlaufGruppe=`${auswahl}:${el.getAttribute("aria-label")??el.closest("label")?.textContent??el.tagName}`}}>
      {#if vollbild}<div class="vollbildleiste" role="toolbar" aria-label="Aufmaß im Vollbild">
        <strong>Aufmaß</strong><span class="meta">{s.eingabe?.bezeichnung}</span>
        <span role="status" class="meta">{s.laeuft?"Wird gespeichert …":s.hatAenderungen()?"Entwurf · ungespeichert":"Gespeichert"}</span>
        <button type="button" disabled={!darf||s.laeuft||!s.hatAenderungen()} onclick={()=>void s.speichern()}>Aufmaß speichern</button>
        <span class="editorwerkzeuge">{@render editorwerkzeuge()}</span>
      </div>{/if}
      {#if hilfe}<section class="kurzbefehle" aria-label="Kurzbefehle"><strong>Zeichnen und navigieren</strong><p>V Auswählen · M Verschieben · P Polygon · R Rechteck · K Kreis · S Strecke · W Wand · L Maßkette · X Text · H Höhenpunkt · Z Zoomfenster · F11 Vollbild · F1 Hilfe</p><p>Strg+Rad zoomt · Leertaste verschiebt · Shift bindet Winkel · F8 Ortho · Alt setzt Fang aus · Ziffern oder @ öffnen die Koordinateneingabe · Shift+Klick erweitert die Auswahl · Pfeile verschieben die Auswahl · Enter schließt bzw. wiederholt den letzten Befehl · Escape verwirft die laufende Aktion · Strg+Z zurück · Strg+Y / Strg+Shift+Z wiederholen.</p></section>{/if}
      <div class="arbeitsplatz">
        <div class="werkzeugleiste" role="toolbar" aria-label="PDF-Zeichenwerkzeuge" aria-orientation="vertical">
          {#each werkzeuggruppen as gruppe,gi}
            {#if gi}<span class="gruppentrenner" role="separator" aria-hidden="true"></span>{/if}
            {#each gruppe as [id,titel,kuerzel]}
              {@const aus=werkzeugGesperrt(id)}
              {@const mitVorlagen=vorlagen.some(v=>werkzeugFuer(v.geometrie)===id)}
              <span class="wzplatz" role="presentation" oncontextmenu={e=>{if(!mitVorlagen||aus||!darf)return;e.preventDefault();vorlagenUmschalten(id,e.currentTarget)}}>
                <Symbolknopf {titel} {kuerzel} tip="rechts" aktiv={werkzeug===id} disabled={!!aus} aria-description={aus||undefined} onclick={()=>werkzeugWaehlen(id)}><WerkzeugIcon art={id}/></Symbolknopf>
                {#if mitVorlagen}<button type="button" class="vorlagenpfeil" aria-label={`Vorlagen für ${titel}`} aria-haspopup="menu" aria-expanded={vorlagenmenue===id} title="Vorlagen (auch rechte Maustaste)" disabled={!!aus||!darf} onclick={e=>vorlagenUmschalten(id,e.currentTarget)}></button>{/if}
              </span>
            {/each}
          {/each}
          <span class="gruppentrenner" role="separator" aria-hidden="true"></span>
          <Symbolknopf titel="Duplizieren" kuerzel="Strg+D" tip="rechts" disabled={!m||objektGesperrt} onclick={duplizieren}><WerkzeugIcon art="duplizieren"/></Symbolknopf>
          <Symbolknopf titel={auswahlElemente.length&&!m?"Element entfernen":"Markierung entfernen"} kuerzel="Entf" tip="rechts" disabled={auswahlElemente.length&&!m?(!darf||s.laeuft):(!m||objektGesperrt)} onclick={()=>auswahlElemente.length&&!m?elementEntfernen():entfernen()}><WerkzeugIcon art="entfernen"/></Symbolknopf>
        </div>
        {#if vorlagenmenue}{@const vl=vorlagen.filter(v=>werkzeugFuer(v.geometrie)===vorlagenmenue)}
          <div class="vorlagenmenue" role="menu" aria-label={`Vorlagen ${werkzeuge.find(w=>w[0]===vorlagenmenue)?.[1]??""}`} style={`left:${vorlagenLage.x}px;top:${vorlagenLage.y}px`}>
            <div class="menuekopf">{werkzeuge.find(w=>w[0]===vorlagenmenue)?.[1]} mit Vorlage</div>
            {#each [...new Set(vl.map(v=>v.gruppe))] as gruppe (gruppe)}
              <div class="menuegruppe" role="presentation">{gruppe}</div>
              {#each vl.filter(v=>v.gruppe===gruppe) as t (t.id)}
                <button type="button" role="menuitem" class:aktiv={vorlage?.id===t.id} title={t.hinweis||undefined} onclick={()=>{vorlagenmenue="";vorlageWaehlen(t)}}><span>{t.titel}</span><span class="meta">{bedeutungen.find(b=>b[0]===t.bedeutung)?.[1]}</span></button>
              {/each}
            {/each}
          </div>
        {/if}
        <div class="flaeche">
          <div class="optionsleiste" role="toolbar" aria-label="Optionen des Werkzeugs">
            <span class="wzname"><WerkzeugIcon art={werkzeugSymbol}/>{werkzeugTitel}</span>
            {#if vorlage}<span role="status" class="chip">Vorlage {vorlage.titel} · {bedeutungen.find(b=>b[0]===vorlage!.bedeutung)?.[1]} · Ebene „{aktiv?layer?.bezeichnung??vorlage.ebene:vorlage.ebene}“<button type="button" class="link" title="Vorlage abbrechen" onclick={()=>{vorlage=null;werkzeug="auswahl"}}>✕</button></span>
            {:else if vorbelegung}<span role="status" class="chip" title={vorbelegung.hinweis||undefined}>Für {vorbelegung.gruppe} · {vorbelegung.titel} · {bedeutungen.find(b=>b[0]===vorbelegung!.bedeutung)?.[1]}<button type="button" class="link" title="Aufgabe abbrechen" onclick={()=>{vorbelegung=null;werkzeug="auswahl"}}>✕</button></span>
            {:else if MARKIERUNGSWERKZEUGE.includes(werkzeug)}
              <label>Bedeutung<select aria-label="Bedeutung neuer Markierungen" bind:value={standardBedeutung}>{#each bedeutungen as [id,t]}<option value={id}>{t}</option>{/each}</select></label>
              {#if gebaeude.length}<label>Gebäude<select aria-label="Gebäude neuer Markierungen" bind:value={standardGebaeude} onchange={()=>standardGeschoss=null}><option value={null}>–</option>{#each gebaeude as g}<option value={g.objekt}>{g.daten.bezeichnung}</option>{/each}</select></label>{/if}
              {#if standardGebaeude}<label>Geschoss<select aria-label="Geschoss neuer Markierungen" bind:value={standardGeschoss}><option value={null}>–</option>{#each gebaeude.find(g=>g.objekt===standardGebaeude)?.daten.geschosse??[] as g}<option value={g.id}>{g.bezeichnung}</option>{/each}</select></label>{/if}
            {/if}
            {#if werkzeug==="wand"}<label>Dicke (m)<input class="zahl" type="number" min=".01" max="10" step=".01" aria-label="Dicke neuer Wände" bind:value={wanddicke}/></label>
            {:else if werkzeug==="symbol"}<label>Planzeichen<select bind:value={planzeichen.symbol}>{#each PLANSYMBOLE as [v,t]}<option value={v}>{t}</option>{/each}</select></label>
            {:else if werkzeug==="raumstempel"}<label>Raum<select bind:value={planzeichen.raum}><option value="">Raum wählen …</option>{#each raumbuecher as rb (rb.objekt)}<optgroup label={rb.daten.bezeichnung}>{#each rb.daten.raeume as r (r.id)}<option value={`${rb.objekt}|${r.id}`}>{r.nummer} {r.bezeichnung} · {z(r.flaeche_m2)} m²</option>{/each}</optgroup>{/each}</select></label>
            {:else if werkzeug==="oeffnung"}<label>Öffnung<select bind:value={planzeichen.oeffnung}><option value="fenster">Fenster 1,01 m</option><option value="tuer">Tür 0,885 m</option></select></label>
            {:else if werkzeug==="spiegeln"}<label class="haken"><input type="checkbox" bind:checked={konstruktion.spiegelKopie}/>Original behalten</label>
            {:else if werkzeug==="versatz"}<label>Abstand (m, + außen/links)<input class="zahl" type="number" step="any" aria-label="Abstand der Parallele" bind:value={konstruktion.versatz}/></label><button type="button" class="sekundaer" disabled={!auswahlAnzahl||!darf||s.laeuft} onclick={auswahlVersatz}>Parallele anlegen</button>
            {:else if werkzeug==="reihe"}<label>Anzahl<input class="zahl" type="number" min="2" max="200" step="1" aria-label="Anzahl der Reihe" bind:value={konstruktion.anzahl}/></label><label>x (m)<input class="zahl" type="number" step="any" aria-label="Abstand x der Reihe" bind:value={konstruktion.dx}/></label><label>y (m, nach oben)<input class="zahl" type="number" step="any" aria-label="Abstand y der Reihe" bind:value={konstruktion.dy}/></label><button type="button" class="sekundaer" disabled={!auswahlAnzahl||!darf||s.laeuft} onclick={auswahlReihe}>Reihe anlegen</button>
            {:else if werkzeug==="kalibrieren"}<label>Referenz (m)<input class="zahl" type="number" min=".001" step="any" aria-label="Länge der Referenzstrecke" bind:value={referenz}/></label>
            {:else if werkzeug==="bezug"||werkzeug==="bezugslinie"}<label>Höhe (m)<input class="zahl" type="number" step="any" aria-label="Höhe des Bezugs" bind:value={hwert}/></label><label>System<input class="kurz" aria-label="Höhensystem des Bezugs" bind:value={hsystem}/></label>
            {:else if werkzeug==="kontrolle"}<label>Soll (m)<input class="zahl" type="number" min=".001" step="any" aria-label="Länge der Kontrollstrecke" bind:value={kontrollMeter}/></label>
            {:else if werkzeug==="hgstrecke"}<label>Strecke (m)<input class="zahl" type="number" min=".001" step="any" aria-label="Bekannte Strecke des Hintergrunds" bind:value={hgStrecke}/></label>
            {:else if werkzeug==="hgdrehen"}<label>Winkel (°)<input class="zahl" type="number" step="any" aria-label="Drehwinkel des Hintergrunds" bind:value={hgWinkel}/></label>{/if}
            {#if werkzeugSchritt}<span class="schritt" role="status">{werkzeugSchritt}</span>{/if}
            <span class="pfad" aria-label="Arbeitsbereich">
              <button type="button" class="link" disabled={!aktiv} onclick={schliessen}>{istZeichnung?"Eigene Zeichnung (Meter)":`${aktuelleDatei?.file_name??"Unterlage"}${istDxfUnterlage?"":` › Blatt ${p.seite}`}`}</button>
              {#if aktueller}<span aria-hidden="true">›</span><strong>{aktueller.bezeichnung}</strong>{#if aktueller.bezugslinie}<span class="meta">· Bezug {z(aktueller.bezugslinie.hoehe_m)} {aktueller.bezugslinie.system}</span>{/if}{/if}
            </span>
            <label class="befehlszeile"><span>Befehl</span><input bind:this={befehlFeld} bind:value={befehl} list="cad-befehle" autocomplete="off" spellcheck="false" disabled={!darf||s.laeuft}
              placeholder={letzterBefehl?`Enter wiederholt „${letzterBefehl}“`:"F2 · z. B. L, PL, KO, SP, VS 0,5, RH 4 1,2 0"} aria-label="Befehlszeile"
              onkeydown={e=>{if(e.key==="Enter"){e.preventDefault();const t=befehl||letzterBefehl;befehl="";befehlAusfuehren(t);plan?.fokus()}else if(e.key==="Escape"){e.preventDefault();e.stopPropagation();befehl="";plan?.fokus()}}}/></label>
            <datalist id="cad-befehle">{#each befehlsliste as [k,b]}<option value={k}>{b}</option>{/each}</datalist>
          </div>
          {#if betrachterSeite!==null}<p class="hinweisbox betrachtung" role="status">Betrachtung von Seite {betrachterSeite}. Das Aufmaß gehört weiterhin zu Seite {p.seite}. <button type="button" class="link" onclick={()=>betrachterSeite=null}>Zum Aufmaßblatt zurück</button>{#if darf}<button type="button" onclick={()=>void seiteWechseln(betrachterSeite!)}>Eigenes Aufmaß für diese Seite anlegen …</button>{/if}</p>{/if}
          {#if flurAuftrag}<div class="hinweisbox flurauftrag" role="status" aria-label="Flurstücksauftrag">
              <strong>Flurstück {flurAuftrag.bezeichnung}:</strong>
              <span>{flurFlaeche!=null?`Grenze gezeichnet · ${z(flurFlaeche)} m²`:flurMarkierung(flurAufgabe,p)?"Grenze gezeichnet; Fläche noch nicht messbar.":"Grenze als Polygon zeichnen und schließen."}</span>
              {#if ohneMassstab}<span class="meta">Zuerst den Maßstab festlegen: „Zwei Punkte kalibrieren“ (Q). Ohne Maßstab lässt sich keine Fläche messen.</span>{/if}
              {#if darf}<button type="button" class="sekundaer" disabled={!flurAufgabe||s.laeuft||flurLaeuft} onclick={()=>flurAufgabe&&aufgabeZeichnen(flurAufgabe)}>Grenze zeichnen</button>
                <button type="button" disabled={flurFlaeche==null||s.laeuft||flurLaeuft} onclick={()=>void flurUebernehmen()}>Fläche übernehmen und zurück</button>{/if}
              <button type="button" class="link" disabled={flurLaeuft} onclick={()=>onflurstueckfertig?.()}>Auftrag beenden</button>
              {#if flurFehler}<span class="fehlertext" role="alert">{flurFehler}</span>{/if}
            </div>{/if}
          {#if pdf||dxf||istZeichnung}
            <PdfPlan bind:this={plan} pdfBase64={pdf?.pdf_base64??""} plan={dxf?.plan??null} planSichtbar={dxfSichtbar} seite={betrachterSeite??p.seite} markierungen={betrachterSeite===null?p.markierungen:[]} ebenen={p.ebenen.map(e=>({...e,sichtbar:sichtbarkeit[e.id]??e.sichtbar}))} {auswahl} {auswahlIds} werkzeug={AUSWAHLWERKZEUGE.includes(werkzeug)?"auswahl":werkzeug} darf={darf&&!s.laeuft&&betrachterSeite===null}
              ansichtKey={`bauprojekt.planansicht.${benutzerId}.${s.projektId}.${s.id}.${p.dokument?.dokument_id??"zeichnung"}.${p.dokument?.version??0}`} fehlerIds={markierungsFehler.map(f=>f.id)} vergleichBase64={vergleichPdf?.pdf_base64??""} {vergleichDeckkraft}
              {vergleichAusrichtung}
              zeichnung={istZeichnung} hintergruende={hgAnsichten} elemente={betrachterSeite===null?p.elemente??[]:[]} {elementAuswahl} {elementAuswahlIds} {wanddicke} onfenster={fensterWaehlen} onwiederholen={wiederholen} {raeume}
              onelement={elementAnlegen} onelementauswahl={elementWaehlen} onelementedit={elementSetzen}
              ausschnitte={betrachterSeite===null?p.ausschnitte??[]:[]} {aktiv} {rahmenAuswahl} werte={aktuelleWerte} kalibrierung={p.kalibrierung} {masse}
              onfertig={fertig} onauswahl={waehlen} onverschieben={gemeinsamVerschieben} onpunkt={i=>{punktIndex=i;panelOeffnen("eigenschaften")}} onedit={(id,g)=>{if(!objektGesperrt){merken();const item=p.markierungen.find(m=>m.id===id);if(item)item.geometrie=g}}}
              onrahmen={(id,von,bis)=>{if(!darf)return;const a=p.ausschnitte?.find(a=>a.id===id);if(a){merken();a.von=von;a.bis=bis}}}
              onrahmenauswahl={id=>rahmenAuswahl=id} onoeffnen={oeffnen} onseite={seiteAnsehen}
              onfehler={e=>pdfFehler=e}/>
          {:else}<p class="meta leer">{laedt?"Unterlage wird geladen …":"Unter „Blatt und Einrichtung“ eine Unterlage auswählen."}</p>{/if}
          {#if messFehler}<p class="fehlertext" role="alert">Aufmaß prüfen: {messFehler}</p>{/if}
        </div>
        {#if werkzeugfenster}<div class="werkzeugfenster" role="dialog" aria-label={FENSTERTITEL[werkzeugfenster]}>
          <div class="palettenkopf"><h3>{FENSTERTITEL[werkzeugfenster]}</h3><button type="button" class="link kopflink" title="Schließen (Esc)" onclick={()=>werkzeugfenster=""}>Schließen</button></div>
          {#if werkzeugfenster==="baufenster"}
            <p class="meta">Im Lageplan das Baufenster (Baugrenzen / Baulinien) mit Bedeutung „Baufenster“ und den Gebäudeumriss mit Bedeutung „Vorhabenumriss“ zeichnen, je genau eine Markierung; bei mehreren Baugrundstücken dem Vorhabenumriss das Baugrundstück zuordnen. Die Zuordnung speichert beide Umrisse in Metern am Projekt; der GRZ-Nachweis bewertet die Lage.</p>
            <button type="button" disabled={!darf||s.laeuft||baugrenzenLaeuft||!s.id} onclick={()=>void baugrenzenZuordnen()}>{baugrenzenLaeuft?"Wird zugeordnet …":"Baufenster und Vorhabenumriss dem Projekt zuordnen"}</button>
            {#if baugrenzenErgebnis}<p role="status" class:fehlertext={!baugrenzenErgebnis.eingehalten}>Baufenster {z(baugrenzenErgebnis.baufenster_m2)} m² · Vorhaben {z(baugrenzenErgebnis.vorhaben_m2)} m² · außerhalb {z(baugrenzenErgebnis.ausserhalb_m2)} m² ({z(baugrenzenErgebnis.ausserhalb_prozent)} %) · größter Überstand {z(baugrenzenErgebnis.eckpunkt_ueberstand_m)} m · {baugrenzenErgebnis.eingehalten?"eingehalten":"nicht eingehalten"}</p>{/if}
            {#if projekt.planungsrecht.baugrenzen}<p class="meta">Gespeicherte Zuordnung: Dokumentversion {projekt.planungsrecht.baugrenzen.dokument_version}{projekt.planungsrecht.baugrenzen.grundstueck_schluessel?` · Baugrundstück ${projekt.planungsrecht.baugrenzen.grundstueck_schluessel}`:""}</p>{/if}
          {:else if werkzeugfenster==="vergleich"}
              <label>Vergleichsunterlage<select aria-label="Vergleichsunterlage" bind:value={vergleich}><option value="">Kein Vergleich</option>{#each pdfDateien as d}<option value={`${d.document_id}|${d.version}`}>{d.file_name} · V{d.version}</option>{/each}</select></label>
              {#if vergleich}<label>Deckkraft der Vergleichsunterlage<input type="range" min="0" max="1" step=".05" bind:value={vergleichDeckkraft}/></label><p class="meta">Vergleichsblatt auf das aktuelle Blatt ausrichten. Verschiebungen in physischen Seitenpunkten (72 pt = 1 Zoll); Rotation im Uhrzeigersinn.</p>
                <div class="kalibrierung"><label>Versatz X (pt)<input type="number" step="any" bind:value={vergleichAusrichtung.dx}/></label><label>Versatz Y (pt)<input type="number" step="any" bind:value={vergleichAusrichtung.dy}/></label><label>Drehung (°)<input type="number" step="any" bind:value={vergleichAusrichtung.grad}/></label><label>Skalierungsfaktor<input type="number" min=".001" step="any" bind:value={vergleichAusrichtung.faktor}/></label></div>
                <label><input type="checkbox" bind:checked={vergleichBestaetigt}/>Blattlage und Konturen anhand der Überlagerung geprüft</label>
                <button type="button" disabled={!darf||!vergleichPdf||s.laeuft||!vergleichBestaetigt} onclick={()=>void uebertragen()}>Als neues Aufmaß auf Vergleichsunterlage übertragen …</button>{/if}
          {:else if werkzeugfenster==="planblatt"}
                <div class="kalibrierung">
                  <label>Format<select bind:value={layout.format}>{#each ["a4","a3","a2","a1","a0"] as f}<option value={f}>{f.toUpperCase()}</option>{/each}</select></label>
                  <label>Lage<select bind:value={layout.hochformat}><option value={false}>Quer</option><option value={true}>Hoch</option></select></label>
                  <label>Maßstab 1 :<select bind:value={layout.massstab}>{#each [20,50,100,200,250,500,1000] as m}<option value={m}>{m}</option>{/each}</select></label>
                </div>
                <label>Plan der Planliste (Nummer und Index im Plankopf)<select bind:value={layout.dokument}><option value="">Ohne Planlisteneintrag</option>{#each planlisteZeilen as z (z.eintrag.dokument_id)}<option value={z.eintrag.dokument_id}>{z.eintrag.plannummer} · {z.eintrag.titel}</option>{/each}</select></label>
                <label>Planinhalt<input bind:value={layout.planinhalt} maxlength="300" placeholder={layout.dokument?"Titel aus der Planliste":s.eingabe?.bezeichnung}/></label>
                <div class="kalibrierung"><label><input type="checkbox" bind:checked={layout.nord}/>Nordpfeil</label>{#if layout.nord}<label>Richtung Nord (° gegen Uhrzeiger)<input type="number" step="any" bind:value={layout.nordwinkel}/></label>{/if}<label><input type="checkbox" bind:checked={layout.legende}/>Legende der Ebenen</label></div>
                <label><input type="checkbox" bind:checked={layout.ablegen} disabled={!darf}/>Im Projekt ablegen{layout.dokument?" (als neue Version des gewählten Plans)":""}</label>
                <p class="meta">Bauherr, Vorhaben, Grundstück und Entwurfsverfasser kommen aus Projekt, Bauantrag und Büroangaben. Passt die Zeichnung nicht auf das Blatt, wird kein Plan erzeugt: größeres Format oder kleineren Maßstab wählen.</p>
                <button type="button" class="sekundaer" disabled={exportLaeuft||s.laeuft} onclick={()=>void layoutAusgeben()}>Planblatt PDF …</button>
          {/if}
        </div>{/if}
        <aside bind:this={paletten}>
          <div class="panelwahl" role="tablist" aria-label="Planpanel" tabindex="-1" onkeydown={reiterTaste}>{#each REITER as [id,titel]}<button type="button" role="tab" id={`reiter-${id}`} data-reiter={id} aria-selected={panel===id} aria-controls="planpanel-inhalt" tabindex={panel===id?0:-1} class="sekundaer" class:aktiv={panel===id} onclick={()=>panelOeffnen(id)}>{titel}</button>{/each}</div>
          <div id="planpanel-inhalt" class="planpanel" role="tabpanel" aria-labelledby={`reiter-${panel}`}>
          {#if panel==="eigenschaften"&&!nichtsGewaehlt}<section class="palette" aria-label="Eigenschaften der Auswahl">
            <div class="palettenkopf"><h3>Eigenschaften</h3> <span class="meta">{auswahlTitel}</span><button type="button" class="link kopflink" title="Auswahl aufheben (Esc) und das Blatt zeigen" onclick={auswahlAufheben}>Zum Blatt</button></div>
            {#if el}
              <fieldset disabled={elementGesperrt} class="element-eigenschaften" aria-label="Zeichenelement">
                <legend>{el.art==="wand"?"Wand":el.art==="masskette"?"Maßkette":el.art==="symbol"?"Planzeichen":el.art==="raumstempel"?"Raumstempel":"Text"}</legend>
                {#if el.art==="wand"}<label>Wanddicke (m)<input type="number" min=".01" max="10" step=".01" bind:value={el.dicke_m}/></label>
                  {#if el.oeffnungen?.length}<div class="oeffnungen">
                    {#each el.oeffnungen as o,i}<div class="oeffnung" role="group" aria-label={`Öffnung ${i+1}`}>
                      <label>Art<select aria-label="Art" bind:value={o.art}><option value="fenster">Fenster</option><option value="tuer">Tür</option></select></label>
                      <label>ab (m)<input type="number" min="0" step=".01" aria-label="Lage ab Wandanfang" bind:value={o.lage_m}/></label>
                      <label>Breite<input type="number" min=".1" step=".005" aria-label="Breite" bind:value={o.breite_m}/></label>
                      <button type="button" class="link entfernen" title="Öffnung entfernen" aria-label="Öffnung entfernen" onclick={()=>{if(el?.art==="wand"){merken();el.oeffnungen=el.oeffnungen!.filter((_,j)=>j!==i)}}}>✕</button>
                      <label>Höhe<input type="number" min="0" step=".005" aria-label="Höhe" value={o.hoehe_m??""} onchange={e=>{const v=e.currentTarget.value;if(v)o.hoehe_m=Number(v);else delete o.hoehe_m}}/></label>
                      {#if o.art==="fenster"}<label>Brüstung<input type="number" min="0" step=".005" aria-label="Brüstung" value={o.bruestung_m??""} onchange={e=>{const v=e.currentTarget.value;if(v)o.bruestung_m=Number(v);else delete o.bruestung_m}}/></label>
                      {:else}<label>Anschlag<select aria-label="Anschlag" bind:value={o.links}><option value={false}>rechts</option><option value={true}>links</option></select></label>{/if}
                    </div>{/each}
                  </div>{:else}<p class="meta">Fenster und Türen mit dem Werkzeug „Fenster / Tür“ (F) in die Wand klicken.</p>{/if}
                {:else if el.art==="symbol"}<label>Planzeichen<select bind:value={el.symbol}>{#each PLANSYMBOLE as [v,t]}<option value={v}>{t}</option>{/each}</select></label><label>Größe ({einheitText})<input type="number" min=".01" step="any" bind:value={el.groesse}/></label><label>Drehung (°)<input type="number" step="any" bind:value={el.drehung_grad}/></label>
                {:else if el.art==="raumstempel"}<label>Raum<select value={`${el.raumbuch_id}|${el.raum_id}`} onchange={e=>{const [a,b]=e.currentTarget.value.split("|");if(el?.art==="raumstempel"&&b){el.raumbuch_id=a;el.raum_id=b}}}>{#if !raeume[el.raum_id]}<option value={`${el.raumbuch_id}|${el.raum_id}`}>Raum fehlt im Raumbuch</option>{/if}{#each raumbuecher as rb (rb.objekt)}<optgroup label={rb.daten.bezeichnung}>{#each rb.daten.raeume as r (r.id)}<option value={`${rb.objekt}|${r.id}`}>{r.nummer} {r.bezeichnung}</option>{/each}</optgroup>{/each}</select></label><label>Schrifthöhe ({einheitText})<input type="number" min=".01" step="any" bind:value={el.hoehe}/></label>
                {:else if el.art==="masskette"}<label>Abstand der Maßlinie ({einheitText})<input type="number" step="any" bind:value={el.abstand}/></label>
                {:else}<label>Text<input bind:value={el.inhalt} maxlength="500" required/></label><label>Schrifthöhe ({einheitText})<input type="number" min=".01" step="any" bind:value={el.hoehe}/></label><label>Drehung (°)<input type="number" step="any" bind:value={el.drehung_grad}/></label>{/if}
                <label>Zeichenebene<select aria-label="Zeichenebene des Elements" bind:value={el.ebene_id}>{#each p.ebenen as e}<option value={e.id}>{e.bezeichnung}</option>{/each}</select></label>
                <p class="meta">{"punkt" in el?`Bei ${z(el.punkt[0])}; ${z(el.punkt[1])} ${einheitText}.`:`Von ${z(el.von[0])}; ${z(el.von[1])} bis ${z(el.bis[0])}; ${z(el.bis[1])} ${einheitText}${(()=>{const f=meterProPunkt(aktueller?.kalibrierung??p.kalibrierung);const [a,b]=elementPunkte(el);return f?` · ${z(Math.hypot(b[0]-a[0],b[1]-a[1])*f)} m`:""})()}.`} Zeichenelemente liefern keine Messwerte; Endpunkte lassen sich mit „Auswählen“ ziehen.</p>
                <DarstellungFelder wert={el.darstellung} erbe={`Ebene „${p.ebenen.find(x=>x.id===el!.ebene_id)?.bezeichnung??""}“`} schraffur={el.art==="wand"||el.art==="symbol"} onaendern={d=>{merken();if(d)el!.darstellung=d;else delete el!.darstellung}}/>
                <div class="aktionen"><button type="button" class="sekundaer" title="Entf" onclick={elementEntfernen}>Element entfernen</button></div>
              </fieldset>
            {:else if !m&&!auswahlAnzahl}<p class="meta">Eine Markierung auf dem Plan oder unter „Objekte“ auswählen. Fenster aufziehen: von links nach rechts wählt Enthaltenes, von rechts nach links auch Berührtes.</p>{/if}
            {#if auswahlElemente.length>1}{@const ds=auswahlElemente.map(e=>JSON.stringify(e.darstellung??{}))}
              <section class="konstruieren" aria-label="Darstellung der gewählten Elemente"><h3>{auswahlElemente.length} Zeichenelemente</h3>
                <DarstellungFelder wert={new Set(ds).size===1?auswahlElemente[0].darstellung:undefined} gemischt={new Set(ds).size>1} erbe="Ebene" disabled={!darf||s.laeuft}
                  onaendern={d=>{if(auswahlElemente.some(e=>p.ebenen.find(x=>x.id===e.ebene_id)?.gesperrt)){s.fehler="Gesperrte Ebenen zuerst entsperren.";return}merken();for(const e of auswahlElemente){if(d)e.darstellung=structuredClone(d);else delete e.darstellung}}}/>
                <label>Zeichenebene<select aria-label="Zeichenebene der gewählten Elemente" value="" onchange={ev=>{const v=ev.currentTarget.value;if(!v)return;merken();for(const e of auswahlElemente)e.ebene_id=v}}><option value="">– beibehalten –</option>{#each p.ebenen as e}<option value={e.id}>{e.bezeichnung}</option>{/each}</select></label>
              </section>{/if}
            {#if auswahlIds.length>1}<section class="mehrfach"><h3>{auswahlIds.length} Markierungen ausgewählt</h3><fieldset disabled={!darf||s.laeuft}><div class="aktionen">{#each ["links","rechts","oben","unten"] as art}<button type="button" class="sekundaer" onclick={()=>auswahlAusrichten(art as "links"|"rechts"|"oben"|"unten")}>{art} ausrichten</button>{/each}</div>
              {#if auswahlIds.length===2}<div class="aktionen">{#each [["vereinen","Vereinen"],["abziehen","Zweite Fläche abziehen"],["schneiden","Schnittfläche"],["teilen","An zweiter Fläche teilen"]] as [op,t]}<button type="button" onclick={()=>void flaechenAktion(op as import("../generated/plattform").Flaechenoperation)}>{t}</button>{/each}</div>{/if}</fieldset></section>{/if}
            {#if m}
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
                <dl class="messwerte">
                  <dt>Fläche</dt><dd>{z(wert?.flaeche_m2)} m²</dd>
                  <dt>Länge / Umfang</dt><dd>{z(wert?.laenge_m)} m</dd>
                  <dt>{m.geometrie.art==="hoehenpunkt"?"Absolute Höhe":"Höhendifferenz"}</dt><dd>{z(wert?.hoehe_m)} m</dd>
                  <dt>Horizontale</dt><dd>{z(wert?.horizontale_m)} m</dd>
                  <dt>Neigung zur Horizontalen</dt><dd>{z(wert?.neigung_grad)}°</dd>
                </dl>
                {#if messLaeuft}<p class="meta" role="status">Messwerte werden aktualisiert …</p>{/if}
                {#each wert?.offen??[] as hinweis}<p class="hinweisbox">{hinweis}</p>{/each}
                <div class="aktionen">
                  <button type="button" class="sekundaer" title="Strg+D · Kopie derselben Kontur, z. B. als Gebäudegrundfläche und Geschoss-Außenkante" onclick={duplizieren}>Duplizieren</button>
                  <button type="button" class="sekundaer" title="Entf" onclick={entfernen}>Markierung entfernen</button>
                </div>
                <div class="verwendung" aria-label="Verwendung der Markierung">
                  <h4>Verwendung</h4>
                  {#if verwendungen.get(m.id)?.length}
                    <ul>{#each verwendungen.get(m.id)??[] as v}<li>
                      <button type="button" class="link" onclick={()=>navigation?.springen(v.ziel)}>{v.verbraucher}</button>
                      <span class="meta">→ {v.bindung.ziel} · R{v.bindung.revision}{s.revision&&v.bindung.revision!==s.revision?" · Aufmaß seitdem gespeichert":""}</span>
                    </li>{/each}</ul>
                  {:else}<p class="meta">Noch in keiner Berechnung verwendet. Übernahme über die Aufgaben oder im Zielbereich.</p>{/if}
                </div>
                <details><summary>Kontur drehen, Stützpunkte und Kanten</summary>
                  <label>Kontur drehen (°)<input type="number" step="any" bind:value={drehenGrad}/></label><button type="button" class="sekundaer" onclick={()=>geometrieSetzen(drehen($state.snapshot(m!.geometrie),drehenGrad))}>Drehung anwenden</button>
                  <label>Stützpunkt<select aria-label="Stützpunkt" bind:value={punktIndex}>{#each eckpunkte(m.geometrie) as p,i}<option value={i}>Punkt {i+1} · {z(p[0])}; {z(p[1])} pt</option>{/each}</select></label>
                  {#if m}{@const punkt=eckpunkte(m.geometrie)[punktIndex]??eckpunkte(m.geometrie)[0]}
                  {#if punkt}<label>X ({einheitText})<input type="number" step="any" value={punkt[0]} onchange={e=>geometrieSetzen(punktAendern($state.snapshot(m!.geometrie),punktIndex,[Number(e.currentTarget.value),punkt[1]]))}/></label><label>Y ({einheitText})<input type="number" step="any" value={punkt[1]} onchange={e=>geometrieSetzen(punktAendern($state.snapshot(m!.geometrie),punktIndex,[punkt[0],Number(e.currentTarget.value)]))}/></label>{/if}
                  {/if}
                  {#if m.geometrie.art==="polygon"}<div class="aktionen"><button type="button" class="sekundaer" onclick={()=>geometrieSetzen(punktEinfuegen($state.snapshot(m!.geometrie),punktIndex))}>Punkt nach Auswahl einfügen</button><button type="button" class="sekundaer" disabled={m.geometrie.punkte.length<=3} onclick={()=>{geometrieSetzen(punktEntfernen($state.snapshot(m!.geometrie),punktIndex));punktIndex=0}}>Punkt entfernen</button></div>{/if}
                  {#if m.geometrie.art==="polygon"||m.geometrie.art==="strecke"}<label>Kantenlänge (m)<input type="number" min=".001" step="any" bind:value={kanteMeter}/></label><label>Kantenwinkel (°)<input type="number" step="any" bind:value={kanteWinkel}/></label><button type="button" disabled={!meterProPunkt(aktueller?.kalibrierung??p.kalibrierung)} onclick={()=>{const f=meterProPunkt(aktueller?.kalibrierung??p.kalibrierung);if(f)geometrieSetzen(kantenmass($state.snapshot(m!.geometrie),punktIndex,kanteMeter/f,kanteWinkel))}}>Kantenmaß anwenden</button>{/if}
                </details>
              </fieldset>
              <p class="meta">Nach dem Speichern: {ziel(m.bedeutung)} Eckpunkte lassen sich mit „Auswählen“ ziehen, die ganze Markierung durch Ziehen der Fläche oder mit den Pfeiltasten.</p>
            {/if}
          </section>{/if}
          {#if panel==="aufgaben"}<section class="palette" aria-label="Aufgaben">
            <div class="palettenkopf"><h3>Aufgaben</h3></div>
            {#if s.id}<Planaufgaben projektId={projekt.id} {projekt} {onprojekt} plan={p} aufmassId={s.id} gespeichert={s.original!==null&&!s.hatAenderungen()} {darf} onzeichnen={aufgabeZeichnen} onwaehlen={id=>{waehlen(id);panelOeffnen("eigenschaften")}}/>{:else}<p class="meta">Aufgaben erscheinen, sobald das Aufmaß gespeichert ist.</p>{/if}
          </section>{/if}
          {#if panel==="objekte"}<section class="palette" aria-label="Objekte">
            <div class="palettenkopf"><h3>Objekte</h3> <span class="meta">· {ausschnitte.length} Ausschnitte · {p.ebenen.length} Ebenen · {p.markierungen.length} Markierungen{p.elemente?.length?` · ${p.elemente.length} Elemente`:""}</span></div>
            <h3 title="Je Zeichnung auf dem Blatt ein Ausschnitt. Doppelklick auf einen Rahmen oder „Öffnen“ zeigt nur diesen Bereich; neue Markierungen gehören dann dazu.">Ausschnitte <span class="meta">({ausschnitte.length})</span></h3>
            <div class="ausschnitte">
              {#each ausschnitte as a (a.id)}
                <div class="ausschnitt" class:aktiv={a.id===aktiv} class:gewaehlt={a.id===rahmenAuswahl}>
                  <button type="button" class="link" onclick={()=>{if(aktiv===a.id)return;rahmenAuswahl=a.id}} aria-label={`Ausschnitt ${a.bezeichnung} wählen`}>{a.bezeichnung}
                    <span>{arten.find(x=>x[0]===a.art)?.[1]} · {a.kalibrierung?massstabText(a.kalibrierung):p.kalibrierung?"Blattmaßstab":"Maßstab fehlt"}{mitHoehen(a.art)?` · ${a.bezugslinie?`Bezug ${z(a.bezugslinie.hoehe_m)} ${a.bezugslinie.system}`:"Bezugslinie fehlt"}`:""} · {anzahl(a).length} Markierungen</span></button>
                  {#if a.id===aktiv}<button type="button" class="sekundaer" onclick={schliessen}>Schließen</button>{:else}<button type="button" class="sekundaer" onclick={()=>oeffnen(a.id)}>Öffnen</button>{/if}
                </div>
              {/each}
            </div>
            {#if !aktiv}<button type="button" class="sekundaer" class:aktiv={werkzeug==="ausschnitt"} aria-pressed={werkzeug==="ausschnitt"} disabled={!darf||!(pdf||dxf||istZeichnung)} onclick={()=>{werkzeug="ausschnitt";rahmenAuswahl=""}}>Ausschnitt anlegen</button>{/if}
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
            <div class="baumkopf"><h3>Ebenen und Objekte{aktueller?` · ${aktueller.bezeichnung}`:""}</h3>
              <span class="menue"><button type="button" class="sekundaer" aria-haspopup="menu" aria-expanded={menue==="ebenen"} disabled={!darf||s.laeuft} onclick={()=>menue=menue==="ebenen"?"":"ebenen"}>Ebenen ▾</button>
                {#if menue==="ebenen"}<div class="menueliste" role="menu" aria-label="Ebenen">
                  <button type="button" role="menuitem" onclick={()=>{menue="";const id=crypto.randomUUID();p.ebenen.push({id,bezeichnung:`Ebene ${p.ebenen.length+1}`,sichtbar:true,gesperrt:false});ebene=id;ebeneWaehlen(id)}}>Ebene hinzufügen</button>
                  <button type="button" role="menuitem" title="Grundstücksgrenze, Bestand, Abbruch, Neubau, Baugrenze, Abstandsflächen …" onclick={()=>{menue="";lageplanEbenen()}}>Lageplan-Ebenen anlegen</button>
                  <button type="button" role="menuitem" disabled={!layer} title="Alle anderen Ebenen sperren" onclick={()=>{menue="";merken();for(const e of p.ebenen)e.gesperrt=e.id!==ebene}}>Nur aktive Ebene bearbeiten</button>
                  <button type="button" role="menuitem" disabled={!p.ebenen.some(e=>e.gesperrt)} onclick={()=>{menue="";merken();for(const e of p.ebenen)e.gesperrt=false}}>Alle entsperren</button>
                </div>{/if}
              </span>
            </div>
            <div class="suchzeile">
              <input type="search" aria-label="Markierungen suchen" placeholder="Suchen …" bind:value={suche}/>
              <button type="button" class="sekundaer" aria-expanded={filterOffen} onclick={()=>filterOffen=!filterOffen}>Filter{filterAnzahl?` (${filterAnzahl})`:""}</button>
            </div>
            {#if filterOffen}<div class="filter">
              <label>Bedeutung<select aria-label="Bedeutung filtern" bind:value={bedeutungFilter}><option value="">Alle Bedeutungen</option>{#each bedeutungen as [id,t]}<option value={id}>{t}</option>{/each}</select></label>
              <label>Gebäude<select aria-label="Gebäude filtern" bind:value={gebaeudeFilter} onchange={()=>geschossFilter=""}><option value="">Alle Gebäude</option>{#each gebaeude as g}<option value={g.objekt}>{g.daten.bezeichnung}</option>{/each}</select></label>
              {#if gebaeudeFilter}<label>Geschoss<select aria-label="Geschoss filtern" bind:value={geschossFilter}><option value="">Alle Geschosse</option>{#each gebaeude.find(g=>g.objekt===gebaeudeFilter)?.daten.geschosse??[] as g}<option value={g.id}>{g.bezeichnung}</option>{/each}</select></label>{/if}
              <label class="zeile"><input type="checkbox" bind:checked={nurFehler}/>Nur fehlerhafte Markierungen</label>
            </div>{/if}
            <ul class="baum" role="tree" aria-label="Ebenen und Objekte">
              {#each p.ebenen as e (e.id)}
                {@const ms=gefilterteMarkierungen.filter(x=>x.ebene_id===e.id)}
                {@const es=mitFilter?[]:(p.elemente??[]).filter(x=>x.ebene_id===e.id)}
                {@const offen=!zugeklappt[e.id]&&(ms.length+es.length>0)}
                {@const sichtbar=sichtbarkeit[e.id]??e.sichtbar}
                <li role="treeitem" aria-expanded={ms.length+es.length?offen:undefined} aria-selected={ebenenAuswahl===e.id} aria-label={`Ebene ${e.bezeichnung}`} class="ebene" class:aktiv={ebene===e.id} class:gewaehlt={ebenenAuswahl===e.id} class:aus={!sichtbar}>
                  <div class="ebenenzeile">
                    <button type="button" class="pfeil" class:offen aria-label={offen?`Ebene ${e.bezeichnung} zuklappen`:`Ebene ${e.bezeichnung} aufklappen`} disabled={!(ms.length+es.length)} onclick={()=>zugeklappt={...zugeklappt,[e.id]:offen}}></button>
                    <input type="radio" aria-label={`Auf Ebene ${e.bezeichnung} zeichnen`} title="Aktive Ebene: neue Objekte landen hier" name={`ebene-${s.id}`} value={e.id} bind:group={ebene}/>
                    <button type="button" class="name" title="Eigenschaften und Darstellung der Ebene" onclick={()=>ebeneWaehlen(e.id)}>{e.bezeichnung}{e.darstellung?" ◆":""}</button>
                    <span class="anzahl">{ms.length+es.length}</span>
                    <button type="button" class="umschalter" aria-label={`Ebene ${e.bezeichnung} sichtbar`} aria-pressed={sichtbar} title={sichtbar?"Sichtbar – ausblenden":"Ausgeblendet – einblenden"} onclick={()=>{if(darf)e.sichtbar=!sichtbar;else sichtbarkeit={...sichtbarkeit,[e.id]:!sichtbar}}}><WerkzeugIcon art={sichtbar?"auge":"augezu"}/></button>
                    <button type="button" class="umschalter" aria-label={`Ebene ${e.bezeichnung} gesperrt`} aria-pressed={e.gesperrt} title={e.gesperrt?"Gesperrt – entsperren":"Sperren"} disabled={!darf||s.laeuft} onclick={()=>{e.gesperrt=!e.gesperrt}}><WerkzeugIcon art={e.gesperrt?"schloss":"schlossauf"}/></button>
                  </div>
                  {#if offen}<ul role="group">
                    {#each ms as item (item.id)}<li role="treeitem" aria-selected={auswahlIds.includes(item.id)} class="objektzeile"><input type="checkbox" aria-label={`${item.bezeichnung} zur Auswahl hinzufügen`} checked={auswahlIds.includes(item.id)} onchange={()=>waehlen(item.id,true)}/><button type="button" class:aktiv={item.id===auswahl} onclick={()=>{waehlen(item.id);panelOeffnen("eigenschaften")}}>{item.bezeichnung}<span>{bedeutungen.find(b=>b[0]===item.bedeutung)?.[1]}{fehlerMap.has(item.id)?" · Fehler":""}{!aktiv&&item.ausschnitt_id?` · ${ausschnitte.find(a=>a.id===item.ausschnitt_id)?.bezeichnung??"Ausschnitt fehlt"}`:""}</span></button></li>{/each}
                    {#each es as x (x.id)}<li role="treeitem" aria-selected={elementAuswahlIds.includes(x.id)} class="objektzeile element"><button type="button" data-element={x.art} class:aktiv={x.id===elementAuswahl} onclick={()=>{elementWaehlen(x.id);panelOeffnen("eigenschaften")}}>{x.art==="wand"?`Wand${x.oeffnungen?.length?` · ${x.oeffnungen.length} Öffnung(en)`:""}`:x.art==="masskette"?"Maßkette":x.art==="symbol"?`Planzeichen ${PLANSYMBOLE.find(([k])=>k===x.symbol)?.[1]??""}`:x.art==="raumstempel"?`Raumstempel ${raeume[x.raum_id]?.nummer??"(Raum fehlt)"}`:`Text „${x.inhalt}“`}<span>Zeichenelement</span></button></li>{/each}
                  </ul>{/if}
                </li>
              {/each}
              {#if ohneEbene.length}<li role="treeitem" aria-expanded="true" aria-selected="false" class="ebene"><div class="ebenenzeile"><span class="name">Ohne Ebene</span><span class="anzahl">{ohneEbene.length}</span></div>
                <ul role="group">{#each ohneEbene as item (item.id)}<li role="treeitem" aria-selected={auswahlIds.includes(item.id)} class="objektzeile"><button type="button" class:aktiv={item.id===auswahl} onclick={()=>{waehlen(item.id);panelOeffnen("eigenschaften")}}>{item.bezeichnung}<span>{bedeutungen.find(b=>b[0]===item.bedeutung)?.[1]}</span></button></li>{/each}</ul></li>{/if}
            </ul>
            {#if !p.markierungen.length&&!p.elemente?.length}<p class="meta">Ein Zeichenwerkzeug wählen (Tasten P, R, K, S, H) und Punkte auf dem Plan setzen.</p>
            {:else if mitFilter&&!gefilterteMarkierungen.length}<p class="meta">Keine Markierung passt zu Suche und Filter.</p>{/if}
          </section>{/if}
          {#if panel==="eigenschaften"&&gewaehlteEbene}{@const e=gewaehlteEbene}<section class="palette" aria-label="Eigenschaften der Ebene">
            <div class="palettenkopf"><h3>Eigenschaften</h3> <span class="meta">· Ebene {e.bezeichnung}</span><button type="button" class="link kopflink" title="Ebene abwählen (Esc) und das Blatt zeigen" onclick={auswahlAufheben}>Zum Blatt</button></div>
            <fieldset disabled={!darf||s.laeuft}>
              <label>Name<input aria-label="Ebenenname" bind:value={e.bezeichnung} required/></label>
              <div class="ebenenschalter">
                <label><input type="radio" name={`ebene-eig-${s.id}`} value={e.id} bind:group={ebene}/>Aktive Ebene</label>
                <label><input type="checkbox" checked={sichtbarkeit[e.id]??e.sichtbar} onchange={ev=>{if(darf)e.sichtbar=ev.currentTarget.checked;else sichtbarkeit={...sichtbarkeit,[e.id]:ev.currentTarget.checked}}}/>Sichtbar</label>
                <label><input type="checkbox" bind:checked={e.gesperrt}/>Gesperrt</label>
              </div>
            </fieldset>
            <DarstellungFelder wert={e.darstellung} disabled={!darf||s.laeuft} onaendern={d=>{merken();if(d)e.darstellung=d;else delete e.darstellung}}/>
            <p class="meta">Elemente mit eigener Darstellung überschreiben die Ebene.</p>
            <div class="aktionen">
              <button type="button" class="sekundaer" disabled={!darf||s.laeuft} title="Alle anderen Ebenen sperren" onclick={()=>{merken();for(const x of p.ebenen)x.gesperrt=x.id!==e.id;ebene=e.id}}>Nur diese Ebene bearbeiten</button>
              <button type="button" class="sekundaer" disabled={!darf||s.laeuft||p.ebenen.length<2||p.markierungen.some(x=>x.ebene_id===e.id)||(p.elemente??[]).some(x=>x.ebene_id===e.id)} title="Nur leere Ebenen lassen sich entfernen" onclick={()=>{merken();p.ebenen=p.ebenen.filter(x=>x.id!==e.id);if(ebene===e.id)ebene=p.ebenen[0]?.id??"";ebenenAuswahl=""}}>Ebene entfernen</button>
            </div>
          </section>{/if}
          {#if panel==="eigenschaften"&&nichtsGewaehlt&&!gewaehlteEbene}<section class="palette" aria-label="Blatt und Einrichtung">
            <div class="palettenkopf"><h3>Blatt und Einrichtung</h3> <span class="meta">{!p.kalibrierung?"· Maßstab fehlt":`· ${massstabText(p.kalibrierung)}`}{p.hoehenbezug?` · Bezug ${z(p.hoehenbezug.hoehe_m)} ${p.hoehenbezug.system}`:""}</span></div>
            <fieldset class="quellzeile" disabled={!darf||s.laeuft}>
              <label>Bezeichnung des Aufmaßes<input bind:value={p.bezeichnung} required/></label>
              {#if istZeichnung}<p class="meta">Eigene Zeichnung: Das Blatt rechnet in Metern (1 Einheit = 1 m, y nach unten) und wächst mit den Inhalten. Unterlagen liegen als Hintergründe darunter.</p>
              {:else}<label>Unterlage<select aria-label="Unterlage" value={`${p.dokument!.dokument_id}|${p.dokument!.version}`} onchange={e=>source(e.currentTarget.value)}>
                <option value="">Unterlage wählen</option>
                {#each dateien as d}<option value={`${d.document_id}|${d.version}`}>{d.file_name} · V{d.version}</option>{/each}
              </select></label>{/if}
              {#if !istDxfUnterlage&&!istZeichnung}<label>Seite<input type="number" min="1" max={pdf?.seiten.length??1} step="1" value={p.seite} onchange={e=>{const n=Number(e.currentTarget.value);e.currentTarget.value=String(p.seite);void seiteWechseln(n)}} required/></label>{/if}
            </fieldset>
            {#if istZeichnung}
              <h3>Hintergründe <span class="meta">({hintergruende.length})</span></h3>
              <p class="meta">PDF-Seiten, DXF-Zeichnungen oder Geschosse eines IFC-Modells als Unterlage unter die Zeichnung legen und einpassen: Maßstab setzen oder über eine bekannte Strecke skalieren, verschieben, drehen. Fang greift an ihren Vektoren.</p>
              <fieldset disabled={!darf||s.laeuft} class="quellzeile">
                <label>Unterlage<select aria-label="Hintergrund-Unterlage" bind:value={hgDatei}><option value="">Unterlage wählen</option>{#each hgDateien as d}<option value={`${d.document_id}|${d.version}`}>{d.file_name} · V{d.version}</option>{/each}</select></label>
                <div class="kalibrierung">
                  {#if hgDateiIfc}<label>Geschoss<select aria-label="Geschoss des IFC-Modells" bind:value={hgGeschoss}>{#each hgGeschosse as g (g.global_id)}<option value={g.global_id}>{g.name}</option>{/each}</select></label>
                  {:else if hgDatei&&!istDxf(dateien.find(d=>`${d.document_id}|${d.version}`===hgDatei)?.file_name??"")}<label>Seite<input type="number" min="1" step="1" bind:value={hgSeite}/></label><label>Maßstab 1 :<input type="number" min=".001" step="any" bind:value={nenner}/></label>{/if}
                  <button type="button" class="sekundaer" disabled={!hgDatei||(hgDateiIfc&&!hgGeschoss)} onclick={hintergrundHinzufuegen}>Hintergrund hinzufügen</button>
                </div>
              </fieldset>
              {#each hintergruende as h (h.id)}
                <fieldset disabled={!darf||s.laeuft} class="ausschnitt-eigenschaften hintergrund" class:gewaehlt={hgZiel===h.id}>
                  <legend><button type="button" class="link" onclick={()=>hgZiel=h.id}>{hgName(h)}</button></legend>
                  <div class="zeile">
                    <label title="Sichtbar"><input type="checkbox" checked={h.sichtbar} onchange={e=>{const an=e.currentTarget.checked;hintergrundAendern(h.id,x=>({...x,sichtbar:an}))}}/>Sichtbar</label>
                    <label>Deckkraft<input type="range" min="0" max="1" step=".05" value={h.deckkraft} onchange={e=>{const v=Number(e.currentTarget.value);hintergrundAendern(h.id,x=>({...x,deckkraft:v}))}}/></label>
                  </div>
                  <div class="kalibrierung">
                    <label>Lage X (m)<input type="number" step="any" value={Math.round(h.lage[0]*1000)/1000} onchange={e=>{const v=Number(e.currentTarget.value);hintergrundAendern(h.id,x=>({...x,lage:[v,x.lage[1]]}))}}/></label>
                    <label>Lage Y (m)<input type="number" step="any" value={Math.round(h.lage[1]*1000)/1000} onchange={e=>{const v=Number(e.currentTarget.value);hintergrundAendern(h.id,x=>({...x,lage:[x.lage[0],v]}))}}/></label>
                    <label>Drehung (°)<input type="number" step="any" value={Math.round(h.drehung_grad*100)/100} onchange={e=>{const v=Number(e.currentTarget.value);hintergrundAendern(h.id,x=>({...x,drehung_grad:v}))}}/></label>
                    {#if h.geschoss}<span class="meta">IFC-Modell in Metern</span>
                    {:else if istDxf(hgName(h))}<label>Zeichnungseinheit<select aria-label="Zeichnungseinheit des Hintergrunds" value={hgEinheit(h)} onchange={e=>{const v=e.currentTarget.value as Laengeneinheit;if(v)hintergrundAendern(h.id,x=>({...x,meter_pro_punkt:EINHEIT_METER[v]??x.meter_pro_punkt}))}}><option value="">Eigener Faktor</option>{#each EINHEITEN as e}<option value={e.code}>{e.name}</option>{/each}</select></label>
                    {:else}<label>Maßstab 1 :<input type="number" min=".001" step="any" value={Math.round(h.meter_pro_punkt*72/0.0254*1000)/1000} onchange={e=>{const v=Number(e.currentTarget.value);if(v>0)hintergrundAendern(h.id,x=>({...x,meter_pro_punkt:mppAusMassstab(v)}))}}/></label>{/if}
                  </div>
                  <div class="kalibrierung">
                    <label>Strecke (m)<input type="number" min=".001" step="any" bind:value={hgStrecke}/></label>
                    <button type="button" class="sekundaer" class:aktiv={werkzeug==="hgstrecke"&&hgZiel===h.id} onclick={()=>{hgZiel=h.id;werkzeug="hgstrecke"}}>Über Strecke einpassen</button>
                    <button type="button" class="sekundaer" class:aktiv={werkzeug==="hgverschieben"&&hgZiel===h.id} onclick={()=>{hgZiel=h.id;werkzeug="hgverschieben"}}>Verschieben</button>
                    <label>Winkel (°)<input type="number" step="any" bind:value={hgWinkel}/></label>
                    <button type="button" class="sekundaer" class:aktiv={werkzeug==="hgdrehen"&&hgZiel===h.id} onclick={()=>{hgZiel=h.id;werkzeug="hgdrehen"}}>Um Punkt drehen</button>
                    <button type="button" class="sekundaer" onclick={()=>hintergrundEntfernen(h.id)}>Entfernen</button>
                  </div>
                </fieldset>
              {/each}
              <h3>Wandstärke</h3>
              <div class="kalibrierung"><label>Dicke neuer Wände (m)<input type="number" min=".01" max="10" step=".01" bind:value={wanddicke}/></label></div>
            {/if}
            <h3>{istZeichnung?"Maßeinheit":istDxfUnterlage?"Zeichnungseinheit":"Blattmaßstab"}</h3>
            {#if istZeichnung}<p class="meta">Meter – fest für eigene Zeichnungen. Maßeingabe: Ziffer tippen, Länge in m, optional „;Winkel“.</p>{/if}
            <fieldset disabled={!darf||s.laeuft}><div class="kalibrierung" hidden={istZeichnung}>
              {#if istDxfUnterlage}
                <label>Zeichnungseinheit<select aria-label="Zeichnungseinheit" value={p.kalibrierung?.art==="einheit"?p.kalibrierung.einheit:""} onchange={e=>{const v=e.currentTarget.value as Laengeneinheit|"";if(v)p.kalibrierung={art:"einheit",einheit:v}}}><option value="">Einheit wählen</option>{#each EINHEITEN as e}<option value={e.code}>{e.name}</option>{/each}</select></label>
                <span class="meta">{dxf?.plan.einheit?`Laut DXF-Kopf: ${EINHEITEN.find(e=>e.code===dxf!.plan.einheit)?.name}`:"Die DXF nennt keine Einheit; Zeichnungseinheit wählen und mit einer Kontrollstrecke prüfen."}</span>
              {:else}
                <label>Maßstab 1 :<input type="number" min=".001" step="any" bind:value={nenner}/></label>
                <button type="button" class="sekundaer" onclick={()=>{if(aktueller)aktueller.kalibrierung={art:"massstab",nenner};else p.kalibrierung={art:"massstab",nenner}}}>Maßstab setzen</button>
              {/if}
              <label>Referenzstrecke (m)<input type="number" min=".001" step="any" bind:value={referenz}/></label>
              <button type="button" class="sekundaer" class:aktiv={werkzeug==="kalibrieren"} aria-pressed={werkzeug==="kalibrieren"} onclick={()=>werkzeug="kalibrieren"}>Zwei Punkte kalibrieren</button>
              <span class="meta">{!p.kalibrierung?"Maßstab fehlt":massstabText(p.kalibrierung)}{aktueller?` · gilt für „${aktueller.bezeichnung}“`:""}</span>
            </div>
            <h3>Höhenbezug des Blattes</h3>
            <p class="meta">Nur für Blätter ohne Ausschnitte: Höhen werden dann senkrecht zur Seite gemessen. Bei mehreren Schnitten je Blatt die Bezugslinie je Ausschnitt setzen (unter „Objekte“).</p>
            <div class="kalibrierung"><label>Höhensystem<input bind:value={hsystem}/></label><label>Bezugshöhe (m)<input type="number" step="any" bind:value={hwert}/></label>
              <button type="button" class="sekundaer" class:aktiv={werkzeug==="bezug"} aria-pressed={werkzeug==="bezug"} onclick={()=>werkzeug="bezug"}>Bezugspunkt setzen</button>
              <span>{p.hoehenbezug?`${z(p.hoehenbezug.hoehe_m)} ${p.hoehenbezug.system}`:"Noch kein Höhenbezug des Blattes"}</span>
            </div>
            <h3>Kontrollstrecke</h3>
            <div class="kalibrierung"><label>Kontrollstrecke (m)<input type="number" min=".001" step="any" bind:value={kontrollMeter}/></label><button type="button" class="sekundaer" class:aktiv={werkzeug==="kontrolle"} aria-pressed={werkzeug==="kontrolle"} onclick={()=>werkzeug="kontrolle"}>Kontrollstrecke messen</button>
              {#if kontrolle}<span role="status">Kontrollvorschau: {z(kontrolle.gemessen)} m · Soll {z(kontrolle.soll)} m · Abweichung {z((kontrolle.gemessen/kontrolle.soll-1)*100)} %{Math.abs(kontrolle.gemessen/kontrolle.soll-1)>.01?" · Maßstab oder Unterlage prüfen":""}</span>{/if}
            </div></fieldset>
            {#if dxf}<h3>Zeichnungsebenen der DXF <span class="meta">· {dxf.plan.ebenen.length} Ebenen · {dxf.plan.linien.length} Linienzüge</span></h3>
              <div class="dxf-ebenen">{#each dxf.plan.ebenen as e,i}<label><input type="checkbox" checked={dxfSichtbar[i]??true} onchange={ev=>{const an=ev.currentTarget.checked;dxfSichtbar=dxfSichtbar.map((v,j)=>j===i?an:v)}}/><span class="farbe" style:background={e.farbe??"#222"}></span>{e.name}</label>{/each}</div>
              {#if dxf.plan.hinweise.length}<ul class="meta">{#each dxf.plan.hinweise as h}<li>{h}</li>{/each}</ul>{/if}
            {/if}
          </section>
          {/if}
          </div>
        </aside>
      </div>
    </div>
  {/snippet}
</Fachliste>

  {#snippet editorwerkzeuge()}<span class="editorwerkzeuge" bind:this={werkzeugleiste}>
    <Symbolknopf titel="Rückgängig" kuerzel="Strg+Z" disabled={!darf||!undo} onclick={rueckwaerts}><WerkzeugIcon art="undo"/></Symbolknopf>
    <Symbolknopf titel="Wiederholen" kuerzel="Strg+Y" disabled={!darf||!redo} onclick={vorwaerts}><WerkzeugIcon art="redo"/></Symbolknopf>
    <span class="trenner" aria-hidden="true"></span>
    <span class="menue">
      <button type="button" class="sekundaer menueknopf" aria-haspopup="menu" aria-expanded={menue==="pruefen"} onclick={()=>menue=menue==="pruefen"?"":"pruefen"}>Prüfen ▾</button>
      {#if menue==="pruefen"}<div class="menueliste" role="menu" aria-label="Prüfen">
        <button type="button" role="menuitem" onclick={()=>fensterOeffnen("baufenster")}>Lage im Baufenster (§ 23 BauNVO) …</button>
        {#if !istDxfUnterlage&&!istZeichnung}<button type="button" role="menuitem" onclick={()=>fensterOeffnen("vergleich")}>Planstände vergleichen …</button>{/if}
      </div>{/if}
    </span>
    <span class="menue">
      <button type="button" class="sekundaer menueknopf" aria-haspopup="menu" aria-expanded={menue==="ausgabe"} onclick={()=>menue=menue==="ausgabe"?"":"ausgabe"}>Ausgabe ▾</button>
      {#if menue==="ausgabe"}<div class="menueliste" role="menu" aria-label="Ausgabe">
        <button type="button" role="menuitem" disabled={exportLaeuft||s.laeuft} onclick={()=>{menue="";void exportieren("pdf")}}>Aufmaßblatt PDF …</button>
        <button type="button" role="menuitem" disabled={exportLaeuft||s.laeuft} onclick={()=>{menue="";void exportieren("csv")}}>Messwerte CSV …</button>
        <button type="button" role="menuitem" disabled={exportLaeuft||s.laeuft} title="Zeichnung und Markierungen in Metern für das Büro-CAD (Ebenen, Farben, Strichstärken)" onclick={()=>{menue="";void dxfAusgeben()}}>DXF fürs CAD …</button>
        {#if istZeichnung}<button type="button" role="menuitem" onclick={()=>fensterOeffnen("planblatt")}>Planblatt PDF (maßstäblich) …</button>{/if}
        <label class="menuehaken"><input type="checkbox" bind:checked={nurSichtbare}/>Nur sichtbare Ebenen ausgeben</label>
      </div>{/if}
    </span>
    <span class="trenner" aria-hidden="true"></span>
    <Symbolknopf titel="Maße anzeigen" aktiv={masse} onclick={()=>masse=!masse}><WerkzeugIcon art="masse"/></Symbolknopf>
    <Symbolknopf titel={vollbild?"Vollbild beenden":"Vollbild"} kuerzel="F11" aktiv={vollbild} onclick={()=>vollbild=!vollbild}><WerkzeugIcon art="vollbild"/></Symbolknopf>
    <Symbolknopf titel="Kurzbefehle" kuerzel="F1" aktiv={hilfe} aria-expanded={hilfe} onclick={()=>hilfe=!hilfe}><WerkzeugIcon art="hilfe"/></Symbolknopf>
  </span>{/snippet}

<style>
  .editorwerkzeuge{display:contents}
  /* Zeichenarbeitsplatz: Schnellleiste oben, Symbolleiste links, Zeichenfläche in der Mitte, Paletten rechts. */
  .editor{display:flex;flex-direction:column;min-height:480px;border:1px solid var(--border);background:var(--panel)}
  .editor.vollbild{position:fixed;inset:0;z-index:40;height:auto;border:0}
  .vollbildleiste{display:flex;gap:8px;align-items:center;padding:4px 8px;background:var(--kopfzeile);border-bottom:1px solid var(--border)}
  .vollbildleiste .editorwerkzeuge{display:inline-flex;gap:2px;align-items:center;margin-left:auto}
  .werkzeugleiste :global(svg),.optionsleiste :global(svg){width:16px;height:16px;stroke:currentColor;fill:none;stroke-width:1.5;stroke-linecap:round;stroke-linejoin:round}
  .trenner{width:1px;height:20px;background:var(--border);margin:0 5px}
  .kurzbefehle{border-bottom:1px solid var(--border);padding:.4rem .8rem;background:var(--leiste)}
  .kurzbefehle p{margin:.3rem 0;font-size:.85rem}
  .arbeitsplatz{position:relative;display:grid;grid-template-columns:auto minmax(0,1fr) 320px;flex:1;min-height:0}
  .werkzeugleiste{display:flex;flex-direction:column;flex-wrap:nowrap;align-items:center;gap:2px;padding:6px 4px;background:var(--kopfzeile);border-right:1px solid var(--border);overflow:auto}
  .wzplatz{position:relative;display:flex}
  .vorlagenpfeil{position:absolute;right:0;bottom:0;width:9px;height:9px;min-height:0;min-width:0;padding:0;border:0;background:transparent;cursor:pointer}
  .vorlagenpfeil::after{content:"";position:absolute;right:1px;bottom:1px;border-style:solid;border-width:0 0 5px 5px;border-color:transparent transparent currentColor transparent;color:var(--muted)}
  .vorlagenpfeil:hover::after,.vorlagenpfeil[aria-expanded=true]::after{color:var(--accent)}
  .vorlagenpfeil:disabled{display:none}
  .wzplatz .vorlagenpfeil:hover:not(:disabled),.wzplatz .vorlagenpfeil:focus-visible{background:transparent}
  .vorlagenmenue{position:fixed;z-index:40;min-width:17rem;max-height:70vh;overflow:auto;display:flex;flex-direction:column;padding:4px 0;background:var(--panel);border:1px solid var(--border);box-shadow:0 6px 18px rgb(0 0 0 / .18);font-size:.88rem}
  .vorlagenmenue .menuekopf{padding:4px 12px 6px;font-weight:600;border-bottom:1px solid var(--border-innen)}
  .vorlagenmenue .menuegruppe{padding:6px 12px 2px;font-size:.75rem;text-transform:uppercase;letter-spacing:.04em;color:var(--muted)}
  .vorlagenmenue button{display:flex;justify-content:space-between;gap:1rem;border:0;border-radius:0;background:transparent;text-align:left;padding:4px 12px 4px 20px;min-height:0;font-weight:400;color:var(--fg)}
  .vorlagenmenue button:hover,.vorlagenmenue button:focus-visible,.vorlagenmenue button.aktiv{background:var(--auswahl,var(--kopfzeile))}
  .vorlagenmenue button .meta{font-size:.8rem}
  .gruppentrenner{width:20px;border-top:1px solid var(--border);margin:4px 0}
  .flaeche{position:relative;display:flex;flex-direction:column;min-height:0;min-width:0;overflow:hidden}
  .flaeche > :global(.pdf-plan){flex:1;min-height:0}
  .befehlszeile{margin-left:auto;display:flex;gap:.3rem;align-items:center;color:var(--muted)}
  .befehlszeile input{width:16rem;min-height:22px;padding:0 .4rem;font-family:ui-monospace,monospace;font-size:.82rem}
  .optionsleiste{flex:none;display:flex;gap:.6rem;align-items:center;padding:2px 10px;min-height:32px;background:var(--leiste);border-bottom:1px solid var(--border-innen);font-size:.85rem;white-space:nowrap;overflow:hidden}
  .optionsleiste .wzname{display:flex;gap:.35rem;align-items:center;font-weight:600;padding-right:.6rem;border-right:1px solid var(--border-innen)}
  .optionsleiste label{display:flex;gap:.3rem;align-items:center;color:var(--muted)}
  .optionsleiste select,.optionsleiste input{min-height:24px;padding:1px 4px;color:var(--fg);font-size:.85rem}
  .optionsleiste select{max-width:14rem}
  .optionsleiste input.zahl{width:5.5rem}
  .optionsleiste input.kurz{width:6rem}
  .optionsleiste .chip{display:inline-flex;gap:.4rem;align-items:center;padding:1px 8px;border:1px solid var(--accent);color:var(--accent);background:var(--panel);border-radius:2px;min-width:0;overflow:hidden;text-overflow:ellipsis}
  .optionsleiste .chip .link{min-height:0;padding:0 2px}
  .optionsleiste .schritt{color:var(--muted);font-style:italic;min-width:0;overflow:hidden;text-overflow:ellipsis}
  .optionsleiste .pfad{margin-left:auto;display:flex;gap:.4rem;align-items:center;min-width:0;overflow:hidden}
  .optionsleiste .pfad .link{min-height:20px;padding:0;color:var(--muted)}
  .optionsleiste .pfad .link:disabled{color:var(--muted);background:transparent}
  .betrachtung{margin:0;border-left:0;border-right:0;border-top:0}
  .leer{padding:2rem;text-align:center}
  .flaeche > .fehlertext{margin:0;padding:4px 10px;border-top:1px solid var(--border)}
  aside{border-left:1px solid var(--border);overflow:auto;min-height:0;background:var(--leiste);display:flex;flex-direction:column}
  .panelwahl{position:sticky;top:0;z-index:2;display:flex;gap:0;padding:0;background:var(--kopfzeile);border-bottom:1px solid var(--border)}
  .panelwahl button{flex:1;font-size:.85rem;padding:.35rem .3rem;border:0;border-right:1px solid var(--border);border-radius:0;background:transparent;min-height:28px}
  .panelwahl button.aktiv{background:var(--panel);box-shadow:inset 0 2px 0 var(--accent);color:var(--accent);font-weight:600}
  .palette{flex:none;background:var(--panel);border-bottom:1px solid var(--border);padding:0 10px 10px}
  .palettenkopf{margin:0 -10px;padding:5px 10px;font-size:.8rem;font-weight:600;text-transform:uppercase;letter-spacing:.05em;color:var(--muted);background:var(--kopfzeile);border-bottom:1px solid var(--border-innen);display:flex;gap:.4rem;align-items:baseline}
  .palettenkopf .meta{text-transform:none;letter-spacing:0;font-weight:400;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .palettenkopf h3{margin:0!important;font:inherit!important;text-transform:inherit;letter-spacing:inherit;color:inherit!important}
  .planpanel{display:flex;flex-direction:column}
  .menue{position:relative;display:inline-flex}
  .menueknopf{min-height:28px;padding:2px 10px;white-space:nowrap}
  .menueliste{position:absolute;right:0;top:calc(100% + 2px);z-index:30;min-width:16rem;display:flex;flex-direction:column;padding:4px 0;background:var(--panel);border:1px solid var(--border);box-shadow:0 6px 18px rgb(0 0 0 / .18)}
  .menueliste button{border:0;border-radius:0;background:transparent;text-align:left;padding:6px 12px;min-height:0;font-weight:400;color:var(--fg)}
  .menueliste button:hover:not(:disabled),.menueliste button:focus-visible{background:var(--auswahl,var(--kopfzeile))}
  .menuehaken{display:flex;gap:.4rem;align-items:center;padding:6px 12px;border-top:1px solid var(--border-innen);margin-top:4px;font-size:.85rem}
  .werkzeugfenster{position:absolute;top:56px;right:328px;z-index:20;width:min(360px,60%);max-height:calc(100% - 64px);overflow:auto;background:var(--panel);border:1px solid var(--border);box-shadow:0 6px 18px rgb(0 0 0 / .18);padding:0 10px 10px;display:grid;gap:.5rem}
  .werkzeugfenster label:not(:has(> input[type=checkbox])){display:flex;flex-direction:column;gap:2px;font-size:.85rem;color:var(--muted)}
  .werkzeugfenster label input:not([type=checkbox]),.werkzeugfenster label select{width:100%;min-height:28px}
  .palettenkopf .kopflink{margin-left:auto;text-transform:none;letter-spacing:0;font-weight:400;font-size:.8rem;white-space:nowrap}
  /* Aufklappbares Planlayout: Pfeil und Überschrift in einer Zeile. */
  .palette h3{margin:.8rem 0 .3rem;font-size:.9rem}
  .palette h4{margin:.6rem 0 .2rem;font-size:.8rem;color:var(--muted)}
  .palette fieldset{border:0;padding:0;display:grid;gap:.5rem;margin:.4rem 0 0}
  .palette label:not(:has(> input[type=checkbox])):not(:has(> input[type=radio])){display:flex;flex-direction:column;gap:2px;margin:0;font-size:.85rem;color:var(--muted)}
  .palette label input:not([type=checkbox]):not([type=radio]),.palette label select{width:100%;min-height:28px;padding:2px 6px;color:var(--fg)}
  .palette .aktionen{display:flex;gap:.4rem;flex-wrap:wrap;margin-top:.4rem}
  .verwendung{margin-top:.6rem;padding-top:.4rem;border-top:1px solid var(--border-innen)}
  .verwendung h4{margin:0 0 .2rem;font-size:.8rem;text-transform:uppercase;letter-spacing:.04em;color:var(--muted)}
  .verwendung ul{margin:0;padding:0;list-style:none}
  .verwendung li{margin:.15rem 0}
  .dxf-ebenen{display:flex;flex-wrap:wrap;gap:.3rem .8rem;padding:.3rem 0}
  .dxf-ebenen label{display:flex;gap:.35rem;align-items:center}
  .dxf-ebenen .farbe{width:.8rem;height:.8rem;border:1px solid var(--control-border);display:inline-block}
  .objektzeile{display:flex;gap:.4rem;align-items:center;content-visibility:auto;contain-intrinsic-size:auto 48px}
  .objektzeile button{flex:1}
  .mehrfach{padding:.5rem;border:1px solid var(--border);margin:.5rem 0}
  /* Öffnungen als zweizeilige Gruppen: passen in die Palettenbreite, nichts wird abgeschnitten. */
  .oeffnungen{display:grid;gap:.4rem}
  .oeffnung{display:grid;grid-template-columns:minmax(0,1.3fr) minmax(0,1fr) minmax(0,1fr) auto;gap:.3rem .4rem;align-items:end;padding:.35rem .4rem;border:1px solid var(--border-innen);background:var(--leiste)}
  .oeffnung label:not(:has(> input[type=checkbox])){font-size:.75rem}
  .oeffnung input,.oeffnung select{min-width:0;font-size:.8rem}
  .oeffnung .entfernen{align-self:end;padding:0 .3rem 6px;grid-column:4;grid-row:1}
  .oeffnung label:nth-of-type(4),.oeffnung label:nth-of-type(5){grid-row:2}
  .konstruieren{padding:.5rem;border:1px solid var(--border);margin:.5rem 0}
  .konstruieren h3{margin:0 0 .4rem;font-size:.9rem}
  .konstruieren .zeile{display:flex;gap:.35rem;align-items:center;margin-top:.4rem}
  .quellzeile{display:grid;gap:.5rem}
  .kalibrierung{display:flex;gap:.5rem;flex-wrap:wrap;align-items:end;margin:.3rem 0}
  .kalibrierung label{flex:1 1 7rem}
  .kalibrierung input[type=number]{width:100%}
  .ausschnitte{display:grid;gap:.3rem}
  .ausschnitt{display:flex;gap:.4rem;align-items:center;padding:.25rem .4rem;border:1px solid transparent}
  .ausschnitt.gewaehlt{border-color:var(--accent)}
  .ausschnitt.aktiv{background:var(--auswahl);border-color:var(--accent)}
  .ausschnitt>button.link{flex:1;text-align:left;font-weight:600}
  .ausschnitt>button.link span{display:block;font-size:.75rem;font-weight:400}
  .ausschnitt-eigenschaften{margin:.6rem 0;padding:.5rem!important;border:1px solid var(--border)!important}
  .ausschnitt-eigenschaften legend{font-weight:600;padding:0 .3rem}
  .hintergrund.gewaehlt{border-color:var(--accent)!important}
  .hintergrund legend .link{font-weight:600;padding:0;min-height:0}
  .element-eigenschaften{padding:.5rem!important;border:1px solid var(--border)!important}
  .element-eigenschaften legend{font-weight:600;padding:0 .3rem}
  .ausschnitt-eigenschaften .zeile{display:flex;flex-wrap:wrap;gap:.3rem .7rem;align-items:center}
  .ausschnitt-eigenschaften .zeile label{flex:0 0 7rem}
  /* Baum im Reiter „Objekte“: Ebene mit Auge und Schloss, darunter ihre Objekte. */
  .ebenenschalter{display:flex;flex-wrap:wrap;gap:.3rem 1rem;font-size:.85rem}
  .ebenenschalter label{display:flex;gap:.3rem;align-items:center}
  .baumkopf{display:flex;align-items:center;gap:.5rem;margin-top:.8rem}
  .baumkopf h3{margin:0;flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .baumkopf .menueliste{min-width:14rem}
  .suchzeile{display:flex;gap:.4rem;margin:.4rem 0}
  .suchzeile input{flex:1;min-width:0}
  .filter{display:grid;gap:.3rem;padding:.4rem;margin-bottom:.4rem;background:var(--leiste);border:1px solid var(--border-innen)}
  .baum,.baum ul{list-style:none;margin:0;padding:0}
  .baum{border-top:1px solid var(--border-innen)}
  .baum li{padding:0;border-top:0;gap:.3rem;align-items:center}
  .baum li.ebene{display:block;border-bottom:1px solid var(--border-innen)}
  .ebenenzeile{display:flex;align-items:center;gap:.25rem;min-height:30px;padding:0 2px}
  .ebene.aktiv>.ebenenzeile .name{font-weight:600}
  .ebene.gewaehlt>.ebenenzeile{background:var(--auswahl);box-shadow:inset 2px 0 0 var(--accent)}
  .ebene.aus>.ebenenzeile .name{color:var(--muted);text-decoration:line-through}
  .ebenenzeile .pfeil{width:18px;min-width:18px;min-height:18px;padding:0;border:0;background:transparent;position:relative}
  .ebenenzeile .pfeil::before{content:"";position:absolute;left:6px;top:5px;border-style:solid;border-width:4px 0 4px 6px;border-color:transparent transparent transparent var(--muted);transition:transform .1s}
  .ebenenzeile .pfeil.offen::before{transform:rotate(90deg)}
  .ebenenzeile .pfeil:disabled::before{opacity:.25}
  .ebenenzeile .name{flex:1;min-width:0;min-height:24px;padding:0 .3rem;border:0;background:transparent;text-align:left;font-weight:400;color:var(--fg);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .ebenenzeile .name:hover{text-decoration:underline}
  .ebenenzeile .anzahl{font-size:.75rem;color:var(--muted);font-variant-numeric:tabular-nums;min-width:1.5rem;text-align:right}
  .ebenenzeile .umschalter{width:26px;min-width:26px;min-height:24px;padding:0;border:0;background:transparent;color:var(--muted);display:inline-flex;align-items:center;justify-content:center}
  .ebenenzeile .umschalter[aria-pressed=true]{color:var(--fg)}
  .ebenenzeile .umschalter :global(svg){width:15px;height:15px;stroke:currentColor;fill:none;stroke-width:1.4}
  .baum ul[role=group]{padding:0 0 .3rem 1.4rem}
  .baum .objektzeile{min-height:0}
  .baum .objektzeile button{text-align:left;min-height:26px;padding:2px 6px;border:0;background:transparent;font-weight:400;color:var(--fg);display:flex;justify-content:space-between;gap:.5rem}
  .baum .objektzeile button:hover{background:var(--leiste)}
  .baum .objektzeile button.aktiv{background:var(--auswahl);box-shadow:inset 2px 0 0 var(--accent)}
  .baum .objektzeile button span{font-size:.75rem;color:var(--muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  .messwerte{display:grid;grid-template-columns:1fr auto;gap:.3rem .6rem;font-variant-numeric:tabular-nums;margin:.4rem 0;padding:.4rem .6rem;background:var(--leiste);border:1px solid var(--border-innen)}
  .messwerte dt{color:var(--muted)}.messwerte dd{margin:0;font-weight:600}
  @media(max-width:1100px){.werkzeugfenster{right:8px}.arbeitsplatz{grid-template-columns:auto minmax(0,1fr);grid-template-rows:minmax(0,1fr) auto}aside{grid-column:1 / -1;border-left:0;border-top:1px solid var(--border);max-height:45%}}
</style>
