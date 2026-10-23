import type { Plangeometrie } from "../generated/plattform";
import { eckpunkte, verschieben, type Point } from "./pdfGeometry";

/** Darstellungstransformationen in Seitenpunkten, keine Fachberechnung. */
export function drehen(g: Plangeometrie, grad: number, mitte?: Point): Plangeometrie {
  const ps=eckpunkte(g); if(!ps.length)return g;
  const o=mitte??[ps.reduce((s,p)=>s+p[0],0)/ps.length,ps.reduce((s,p)=>s+p[1],0)/ps.length];
  const a=grad*Math.PI/180,c=Math.cos(a),s=Math.sin(a);
  const punkt=(p:Point):Point=>[o[0]+(p[0]-o[0])*c-(p[1]-o[1])*s,o[1]+(p[0]-o[0])*s+(p[1]-o[1])*c];
  return abbilden(g,punkt);
}
export function abbilden(g: Plangeometrie, punkt:(p:Point)=>Point): Plangeometrie {
  if(g.art==="polygon")return {...g,punkte:g.punkte.map(punkt)};
  if(g.art==="flaeche")return {...g,polygone:g.polygone.map(p=>({...p,aussen:p.aussen.map(punkt),innen:p.innen?.map(r=>r.map(punkt))}))};
  if(g.art==="kreis")return {...g,mitte:punkt(g.mitte),rand:punkt(g.rand)};
  if(g.art==="hoehenpunkt")return {...g,punkt:punkt(g.punkt)};
  if(g.art==="rechteck")return {art:"polygon",punkte:[g.von,[g.bis[0],g.von[1]],g.bis,[g.von[0],g.bis[1]]].map(p=>punkt(p as Point))};
  return {...g,von:punkt(g.von),bis:punkt(g.bis)};
}
export function punktAendern(g:Plangeometrie,index:number,p:Point):Plangeometrie {
  const copy=structuredClone(g);
  if(copy.art==="polygon")copy.punkte[index]=p;
  else if(copy.art==="flaeche"){
    let n=index;
    for(const poly of copy.polygone)for(const ring of [poly.aussen,...(poly.innen??[])]){if(n<ring.length){ring[n]=p;return copy}n-=ring.length}
  }
  else if(copy.art==="kreis"){if(index===0)copy.mitte=p;else copy.rand=p}
  else if(copy.art==="hoehenpunkt")copy.punkt=p;
  else if(index===0)copy.von=p;else copy.bis=p;
  return copy;
}
export function punktEinfuegen(g:Plangeometrie,index:number):Plangeometrie {
  if(g.art!=="polygon")return g;
  const ps=g.punkte,a=ps[index],b=ps[(index+1)%ps.length];
  return {...g,punkte:[...ps.slice(0,index+1),[(a[0]+b[0])/2,(a[1]+b[1])/2],...ps.slice(index+1)]};
}
export function punktEntfernen(g:Plangeometrie,index:number):Plangeometrie {
  if(g.art!=="polygon"||g.punkte.length<=3)return g;
  return {...g,punkte:g.punkte.filter((_,i)=>i!==index)};
}
export function kantenmass(g:Plangeometrie,index:number,laengePunkte:number,winkelGrad:number):Plangeometrie {
  const ps=eckpunkte(g),a=ps[index];if(!a||!(laengePunkte>0)||!Number.isFinite(winkelGrad))return g;
  const w=winkelGrad*Math.PI/180;
  return punktAendern(g,(index+1)%ps.length,[a[0]+Math.cos(w)*laengePunkte,a[1]-Math.sin(w)*laengePunkte]);
}
export function ausrichten(gs:Plangeometrie[],art:"links"|"rechts"|"oben"|"unten"):Plangeometrie[]{
  const grenzen=gs.map(g=>{const ps=eckpunkte(g);return {x:ps.map(p=>p[0]),y:ps.map(p=>p[1])}});
  const ziel=art==="links"?Math.min(...grenzen.flatMap(g=>g.x)):art==="rechts"?Math.max(...grenzen.flatMap(g=>g.x)):art==="oben"?Math.min(...grenzen.flatMap(g=>g.y)):Math.max(...grenzen.flatMap(g=>g.y));
  return gs.map((g,i)=>verschieben(g,art==="links"?ziel-Math.min(...grenzen[i].x):art==="rechts"?ziel-Math.max(...grenzen[i].x):0,art==="oben"?ziel-Math.min(...grenzen[i].y):art==="unten"?ziel-Math.max(...grenzen[i].y):0));
}
