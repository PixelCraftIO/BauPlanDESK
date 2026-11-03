import type { Point, Segment, SnapGeometry } from "./pdfGeometry";
export interface Fangoptionen { endpunkte:boolean; mittelpunkte:boolean; kanten:boolean; schnittpunkte:boolean; eigene:boolean; lot:boolean; verlaengerung:boolean }
export const standardFang:Fangoptionen={endpunkte:true,mittelpunkte:true,kanten:true,schnittpunkte:false,eigene:true,lot:true,verlaengerung:true};
type Kandidat={punkt:Point;art:string};
/** Räumlicher Index wird je Geometriestand aufgebaut, nicht je Zeigerbewegung. */
export class Fangindex {
  private punkte=new Map<string,Kandidat[]>();
  private segmente=new Map<string,Segment[]>();
  private lange:Segment[]=[];
  private alle:Kandidat[]=[];
  private zelle:number;
  constructor(private g:SnapGeometry){
    let minX=Infinity,minY=Infinity,maxX=-Infinity,maxY=-Infinity;
    for(const p of g.points){minX=Math.min(minX,p[0]);maxX=Math.max(maxX,p[0]);minY=Math.min(minY,p[1]);maxY=Math.max(maxY,p[1])}
    this.zelle=Math.max(1,Number.isFinite(minX)?Math.max(maxX-minX,maxY-minY)/128:32);
    const einfuegen=(p:Point,art:string)=>{const k=this.key(p),v={punkt:p,art};this.alle.push(v);const bucket=this.punkte.get(k)??[];bucket.push(v);this.punkte.set(k,bucket)};
    for(const p of g.points)einfuegen(p,"Endpunkt");
    for(const s of g.segments){
      einfuegen([(s[0][0]+s[1][0])/2,(s[0][1]+s[1][1])/2],"Mittelpunkt");
      const [x0,y0,x1,y1]=this.bereich(s[0],s[1]);
      if((x1-x0+1)*(y1-y0+1)>256){this.lange.push(s);continue}
      for(let x=x0;x<=x1;x++)for(let y=y0;y<=y1;y++){const k=`${x}:${y}`,b=this.segmente.get(k)??[];b.push(s);this.segmente.set(k,b)}
    }
  }
  private key(p:Point){return `${Math.floor(p[0]/this.zelle)}:${Math.floor(p[1]/this.zelle)}`}
  private bereich(a:Point,b:Point){return [Math.floor(Math.min(a[0],b[0])/this.zelle),Math.floor(Math.min(a[1],b[1])/this.zelle),Math.floor(Math.max(a[0],b[0])/this.zelle),Math.floor(Math.max(a[1],b[1])/this.zelle)]}
  /** Kanten der Unterlage in der Nähe von p (für Lotfang). */
  kantenNahe(p:Point,toleranz:number):Segment[]{
    const [x0,y0,x1,y1]=this.bereich([p[0]-toleranz,p[1]-toleranz],[p[0]+toleranz,p[1]+toleranz]);
    if((x1-x0+1)*(y1-y0+1)>4096)return [];
    const ss=new Set<Segment>(this.lange);
    for(let x=x0;x<=x1;x++)for(let y=y0;y<=y1;y++)for(const s of this.segmente.get(`${x}:${y}`)??[])ss.add(s);
    return [...ss].slice(0,512);
  }
  suchen(p:Point,toleranz:number,o:Fangoptionen=standardFang):{point:Point;kind:string}|null{
    const [x0,y0,x1,y1]=this.bereich([p[0]-toleranz,p[1]-toleranz],[p[0]+toleranz,p[1]+toleranz]);
    let ps:Kandidat[]=[],ss=new Set<Segment>(this.lange);
    if((x1-x0+1)*(y1-y0+1)>4096){ps=this.alle;ss=new Set(this.g.segments)}
    else for(let x=x0;x<=x1;x++)for(let y=y0;y<=y1;y++){const k=`${x}:${y}`;ps.push(...(this.punkte.get(k)??[]));for(const s of this.segmente.get(k)??[])ss.add(s)}
    let best:{point:Point;kind:string}|null=null,d=toleranz;
    const pruefen=(q:Point,kind:string)=>{const n=Math.hypot(q[0]-p[0],q[1]-p[1]);if(n<d){d=n;best={point:q,kind}}};
    for(const q of ps)if((q.art==="Endpunkt"&&o.endpunkte)||(q.art==="Mittelpunkt"&&o.mittelpunkte))pruefen(q.punkt,q.art);
    if(best)return best;
    const kanten=[...ss];
    if(o.schnittpunkte&&kanten.length<=128)for(let i=0;i<kanten.length;i++)for(let j=i+1;j<kanten.length;j++){
      const [a,b]=kanten[i],[c,e]=kanten[j],r:Point=[b[0]-a[0],b[1]-a[1]],s:Point=[e[0]-c[0],e[1]-c[1]];
      const det=r[0]*s[1]-r[1]*s[0];if(Math.abs(det)<1e-12)continue;
      const q:Point=[c[0]-a[0],c[1]-a[1]],t=(q[0]*s[1]-q[1]*s[0])/det,u=(q[0]*r[1]-q[1]*r[0])/det;
      if(t>=0&&t<=1&&u>=0&&u<=1)pruefen([a[0]+t*r[0],a[1]+t*r[1]],"Schnittpunkt");
    }
    if(best)return best;
    if(o.kanten)for(const [a,b]of kanten){const dx=b[0]-a[0],dy=b[1]-a[1],l=dx*dx+dy*dy;if(!l)continue;const t=Math.max(0,Math.min(1,((p[0]-a[0])*dx+(p[1]-a[1])*dy)/l));pruefen([a[0]+t*dx,a[1]+t*dy],"Kante")}
    return best;
  }
}
export function streckenachsen(von:Point,bis:Point,bezug?:{von:Point;bis:Point}|null){
  const l=bezug?Math.hypot(bezug.bis[0]-bezug.von[0],bezug.bis[1]-bezug.von[1]):0;
  const x:Point=bezug&&l?[(bezug.bis[0]-bezug.von[0])/l,(bezug.bis[1]-bezug.von[1])/l]:[1,0];
  const v:Point=[bis[0]-von[0],bis[1]-von[1]];
  return {horizontal:Math.abs(v[0]*x[0]+v[1]*x[1]),hoehe:Math.abs(v[0]*x[1]-v[1]*x[0])};
}
