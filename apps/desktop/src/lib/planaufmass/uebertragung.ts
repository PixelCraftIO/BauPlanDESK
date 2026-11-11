import type { Planaufmass } from "../generated/plattform";
import { abbilden } from "./bearbeitung";
import type { Point } from "./pdfGeometry";
export interface Planausrichtung { dx:number;dy:number;grad:number;faktor:number }
/** Die Vergleichsunterlage wird auf das alte Blatt ausgerichtet. Für die
 * Übertragung der Konturen gilt die inverse Abbildung. Neue IDs erhalten
 * bestehende Verbraucher am alten Aufmaß; Bezüge müssen neu bestätigt werden. */
export function aufmassUebertragen(p:Planaufmass,dokument:Planaufmass["dokument"],seite:number,t:Planausrichtung,id=()=>crypto.randomUUID()):Planaufmass {
  if(![t.dx,t.dy,t.grad,t.faktor].every(Number.isFinite)||t.faktor<=0)throw Error("Ausrichtung benötigt endliche Werte und eine positive Skalierung.");
  const a=-t.grad*Math.PI/180,c=Math.cos(a),s=Math.sin(a);
  const punkt=(p:Point):Point=>{const x=(p[0]-t.dx)/t.faktor,y=(p[1]-t.dy)/t.faktor;return [x*c-y*s,x*s+y*c]};
  const ebenen=new Map(p.ebenen.map(e=>[e.id,id()])),ausschnitte=new Map((p.ausschnitte??[]).map(a=>[a.id,id()])),markierungen=new Map(p.markierungen.map(m=>[m.id,id()]));
  return {...structuredClone(p),bezeichnung:`${p.bezeichnung} · übertragener Entwurf`,dokument,seite,kalibrierung:null,hoehenbezug:null,
    ebenen:p.ebenen.map(e=>({...e,id:ebenen.get(e.id)!})),
    ausschnitte:(p.ausschnitte??[]).map(a=>{const ps=[a.von,[a.bis[0],a.von[1]],a.bis,[a.von[0],a.bis[1]]].map(p=>punkt(p as Point));return {...a,id:ausschnitte.get(a.id)!,ebene_id:a.ebene_id?ebenen.get(a.ebene_id):undefined,von:[Math.min(...ps.map(p=>p[0])),Math.min(...ps.map(p=>p[1]))],bis:[Math.max(...ps.map(p=>p[0])),Math.max(...ps.map(p=>p[1]))],kalibrierung:undefined,bezugslinie:undefined}}),
    markierungen:p.markierungen.map(m=>({...m,id:markierungen.get(m.id)!,ebene_id:ebenen.get(m.ebene_id)!,ausschnitt_id:m.ausschnitt_id?ausschnitte.get(m.ausschnitt_id):undefined,hoehenzone_von:m.hoehenzone_von?markierungen.get(m.hoehenzone_von):undefined,kontur_von:m.kontur_von?markierungen.get(m.kontur_von):undefined,geometrie:abbilden(m.geometrie,punkt)})),
  };
}
/** Wechselt den Planstand im selben Aufmaß: Alle Kennungen bleiben, damit
 * Verknüpfungen der Verbraucher weiter auf ihre Markierung zeigen und nur
 * den neuen Aufmaßstand übernehmen müssen. Konturen, Rahmen, Maßstäbe und
 * Höhenbezüge wandern mit der Ausrichtung auf das neue Blatt. */
export function planstandWechseln(p:Planaufmass,dokument:Planaufmass["dokument"],seite:number,t:Planausrichtung):Planaufmass {
  if(![t.dx,t.dy,t.grad,t.faktor].every(Number.isFinite)||t.faktor<=0)throw Error("Ausrichtung benötigt endliche Werte und eine positive Skalierung.");
  const a=-t.grad*Math.PI/180,c=Math.cos(a),s=Math.sin(a);
  const punkt=(p:Point):Point=>{const x=(p[0]-t.dx)/t.faktor,y=(p[1]-t.dy)/t.faktor;return [x*c-y*s,x*s+y*c]};
  // Ein Punkt des neuen Blatts steht für `faktor` Punkte des alten.
  const massstab=<K extends Planaufmass["kalibrierung"]|undefined>(k:K):K=>!k?k:k.art==="massstab"?{...k,nenner:Math.round(k.nenner*t.faktor*1e6)/1e6}:k.art==="strecke"?{...k,von:punkt(k.von as Point),bis:punkt(k.bis as Point)}:k;
  const kopie=structuredClone(p);
  return {...kopie,dokument,seite,kalibrierung:massstab(kopie.kalibrierung),
    hoehenbezug:kopie.hoehenbezug?{...kopie.hoehenbezug,punkt:punkt(kopie.hoehenbezug.punkt as Point)}:null,
    ausschnitte:kopie.ausschnitte?.map(a=>{const ps=[a.von,[a.bis[0],a.von[1]],a.bis,[a.von[0],a.bis[1]]].map(p=>punkt(p as Point));return {...a,von:[Math.min(...ps.map(p=>p[0])),Math.min(...ps.map(p=>p[1]))] as Point,bis:[Math.max(...ps.map(p=>p[0])),Math.max(...ps.map(p=>p[1]))] as Point,kalibrierung:massstab(a.kalibrierung),bezugslinie:a.bezugslinie?{...a.bezugslinie,von:punkt(a.bezugslinie.von as Point),bis:punkt(a.bezugslinie.bis as Point)}:undefined}}),
    markierungen:kopie.markierungen.map(m=>({...m,geometrie:abbilden(m.geometrie,punkt)})),
    elemente:kopie.elemente?.map(e=>"punkt" in e?{...e,punkt:punkt(e.punkt as Point)}:{...e,von:punkt(e.von as Point),bis:punkt(e.bis as Point)}),
  };
}
