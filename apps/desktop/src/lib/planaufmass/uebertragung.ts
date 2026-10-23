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
    markierungen:p.markierungen.map(m=>({...m,id:markierungen.get(m.id)!,ebene_id:ebenen.get(m.ebene_id)!,ausschnitt_id:m.ausschnitt_id?ausschnitte.get(m.ausschnitt_id):undefined,hoehenzone_von:m.hoehenzone_von?markierungen.get(m.hoehenzone_von):undefined,geometrie:abbilden(m.geometrie,punkt)})),
  };
}
