export interface Wertvergleich { ziel:string;vorher:string;nachher:string;quelle:string }
export class Uebernahme {
  daten=$state<{zeilen:Wertvergleich[];speichern:boolean;antwort:(ok:boolean)=>void}|null>(null);
  fragen(zeilen:Wertvergleich[],speichern=false):Promise<boolean>{
    this.antwort(false);
    return new Promise(resolve=>{this.daten={zeilen,speichern,antwort:resolve}});
  }
  antwort(ok:boolean){const v=this.daten;this.daten=null;v?.antwort(ok)}
}
/** Stabile Fachpfade enthalten Objekt-IDs; niemals die Position in einer Liste. */
export function zielfeld(plan:unknown,pfad:string):unknown {
  let v=plan;
  for(const p of pfad.split(".")){
    if(Array.isArray(v))v=v.find(x=>x&&typeof x==="object"&&x.id===p);
    else if(v&&typeof v==="object")v=(v as Record<string,unknown>)[p];else return undefined;
  }
  return v;
}
export function werttext(v:unknown,einheit=""):string{
  return typeof v==="number"?`${v.toLocaleString("de-DE",{maximumFractionDigits:4})}${einheit?` ${einheit}`:""}`:v==null?"—":String(v);
}
