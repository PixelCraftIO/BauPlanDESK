/** Vollständige, unveränderliche Bearbeitungsstände. Ein Drag wird erst beim
 * Abschluss übernommen; aufeinanderfolgende Eingaben eines Feldes gruppiert. */
export class Bearbeitungsverlauf<T> {
  private aktuell: string | null = null;
  private vorher: string[] = [];
  private nachher: string[] = [];
  private gruppe = "";
  private zeit = 0;
  constructor(private readonly grenze = 50) {}
  get zurueck() { return this.vorher.length; }
  get vor() { return this.nachher.length; }
  gruppeBeenden(){this.gruppe="";}
  reset(wert: T) { this.aktuell = JSON.stringify(wert); this.vorher=[];this.nachher=[];this.gruppe=""; }
  beobachten(wert: T, gruppe = "", zeit = Date.now()) {
    this.beobachtenSignatur(JSON.stringify(wert),gruppe,zeit);
  }
  beobachtenSignatur(neu:string,gruppe="",zeit=Date.now()){
    if (this.aktuell === null) { this.aktuell=neu;return; }
    if (neu===this.aktuell) return;
    if (!gruppe || gruppe!==this.gruppe || zeit-this.zeit>800 || this.nachher.length) {
      this.vorher=[...this.vorher.slice(-(this.grenze-1)),this.aktuell];
      let bytes=this.vorher.reduce((s,v)=>s+v.length*2,0);
      while(this.vorher.length>1&&bytes>16*1024*1024)bytes-=this.vorher.shift()!.length*2;
    }
    this.aktuell=neu;this.nachher=[];this.gruppe=gruppe;this.zeit=zeit;
  }
  rueckwaerts(wert: T): T | null {
    this.beobachten(wert);
    const alt=this.vorher.pop(); if(alt===undefined)return null;
    this.nachher.push(this.aktuell!);this.aktuell=alt;this.gruppe="";
    return JSON.parse(alt) as T;
  }
  vorwaerts(wert: T): T | null {
    this.beobachten(wert);
    const neu=this.nachher.pop();if(neu===undefined)return null;
    this.vorher.push(this.aktuell!);this.aktuell=neu;this.gruppe="";
    return JSON.parse(neu) as T;
  }
}
