/** PDF.js adapter only. Coordinates are physical page points, top left, y down.
 * Curves supply endpoints, never straight chords presented as PDF edges.
 */
export type Point = [number, number];
export type Segment = [Point, Point];
export type Matrix = [number, number, number, number, number, number];
export interface SnapGeometry { points: Point[]; segments: Segment[]; limited: boolean }
export const transform = (p: Point, m: Matrix): Point =>
  [m[0] * p[0] + m[2] * p[1] + m[4], m[1] * p[0] + m[3] * p[1] + m[5]];
export function multiply(a: Matrix, b: Matrix): Matrix {
  return [
    a[0]*b[0]+a[2]*b[1], a[1]*b[0]+a[3]*b[1],
    a[0]*b[2]+a[2]*b[3], a[1]*b[2]+a[3]*b[3],
    a[0]*b[4]+a[2]*b[5]+a[4], a[1]*b[4]+a[3]*b[5]+a[5],
  ];
}
export function extractGeometry(
  list: { fnArray: number[]; argsArray: unknown[][] },
  ops: Record<string, number>, viewport: Matrix, limit = 100000,
): SnapGeometry {
  const result: SnapGeometry = { points: [], segments: [], limited: false };
  let matrix: Matrix = [...viewport], stack: Matrix[] = [];
  let current: Point | null = null, start: Point | null = null;
  let points: Point[] = [], segments: Segment[] = [];
  const mapped = (x: number, y: number) => transform([x,y], matrix);
  const line = (p: Point) => {
    if (current) segments.push([current,p]);
    points.push(p); current = p;
  };
  const close = () => { if (start && current) { segments.push([current,start]); current=start; } };
  const flush = (paint: boolean) => {
    if (paint) { result.points.push(...points); result.segments.push(...segments); }
    points=[]; segments=[]; current=null; start=null;
  };
  const paint = new Set(["stroke","fill","eoFill","fillStroke","eoFillStroke"].map(k=>ops[k]));
  const closePaint = new Set(["closeStroke","closeFillStroke","closeEOFillStroke"].map(k=>ops[k]));
  for (let i=0;i<list.fnArray.length;i++) {
    if (result.points.length+points.length+result.segments.length+segments.length>limit) { result.limited=true; break; }
    const op=list.fnArray[i], a=list.argsArray[i] ?? [];
    if (op===ops.save) stack.push([...matrix]);
    else if (op===ops.restore) matrix=stack.pop() ?? [...viewport];
    else if (op===ops.transform) matrix=multiply(matrix,a as Matrix);
    else if (op===ops.paintFormXObjectBegin) {
      stack.push([...matrix]);
      if (a[0]) matrix=multiply(matrix,a[0] as Matrix);
    } else if (op===ops.paintFormXObjectEnd) matrix=stack.pop() ?? [...viewport];
    else if (op===ops.constructPath) {
      const path=a[0] as number[], values=a[1] as number[];
      let j=0;
      for (const cmd of path) {
        if (cmd===ops.moveTo) {
          current=mapped(values[j++],values[j++]); start=current; points.push(current);
        } else if (cmd===ops.lineTo) line(mapped(values[j++],values[j++]));
        else if (cmd===ops.rectangle) {
          const [x,y,w,h]=values.slice(j,j+4); j+=4;
          current=mapped(x,y); start=current; points.push(current);
          line(mapped(x+w,y)); line(mapped(x+w,y+h)); line(mapped(x,y+h)); close();
        } else if (cmd===ops.closePath) close();
        else if (cmd===ops.curveTo || cmd===ops.curveTo2 || cmd===ops.curveTo3) {
          const n=cmd===ops.curveTo?6:4;
          current=mapped(values[j+n-2],values[j+n-1]); points.push(current); j+=n;
        } else { result.limited=true; break; }
        if (points.length+segments.length>limit) { result.limited=true; break; }
      }
    } else if (op===ops.closePath) close();
    else if (paint.has(op)) flush(true);
    else if (closePaint.has(op)) { close(); flush(true); }
    else if (op===ops.endPath) flush(false);
  }
  return result;
}
export function snap(point: Point, geometry: SnapGeometry, tolerance: number):
  { point: Point; kind: string } | null {
  let best: {point:Point;kind:string} | null=null, distance=tolerance;
  const consider=(p:Point,kind:string)=>{
    const d=Math.hypot(p[0]-point[0],p[1]-point[1]);
    if(d<distance) { distance=d; best={point:p,kind}; }
  };
  // Prefer intentional vertices/midpoints to projections on nearby edges.
  for(const p of geometry.points) consider(p,"Endpunkt");
  for(const [a,b] of geometry.segments) consider([(a[0]+b[0])/2,(a[1]+b[1])/2],"Mittelpunkt");
  if(best) return best;
  for(const [a,b] of geometry.segments) {
    const dx=b[0]-a[0],dy=b[1]-a[1], l=dx*dx+dy*dy;
    if(l===0) continue;
    const t=Math.max(0,Math.min(1,((point[0]-a[0])*dx+(point[1]-a[1])*dy)/l));
    consider([a[0]+t*dx,a[1]+t*dy],"Kante");
  }
  return best;
}

// Vorschauhilfen für die Zeichenfläche. Maßgebend bleiben die Werte des Cores
// (planaufmass_pruefen); diese Funktionen zeigen nur während des Zeichnens an.
type Kalibrierung = { art: "massstab"; nenner: number } | { art: "strecke"; von: Point; bis: Point; meter: number };
type Geometrie = import("../generated/plattform").Plangeometrie;
export function meterProPunkt(k: Kalibrierung | null | undefined): number | null {
  if (!k) return null;
  if (k.art === "massstab") return k.nenner > 0 ? (k.nenner * 0.0254) / 72 : null;
  const l = Math.hypot(k.bis[0] - k.von[0], k.bis[1] - k.von[1]);
  return l > 0 && k.meter > 0 ? k.meter / l : null;
}
/** Länge/Umfang und Fläche in Seitenpunkten bzw. Metern; nur Vorschau. */
export function vorschauMass(g: Geometrie, f: number | null): { laenge_m: number | null; flaeche_m2: number | null } {
  let laenge = 0, flaeche = 0;
  if (g.art === "flaeche") {
    for(const p of g.polygone){
      const a=vorschauMass({art:"polygon",punkte:p.aussen},1);flaeche+=a.flaeche_m2??0;laenge+=a.laenge_m??0;
      for(const r of p.innen??[]){const v=vorschauMass({art:"polygon",punkte:r},1);flaeche-=v.flaeche_m2??0;laenge+=v.laenge_m??0}
    }
  } else if (g.art === "polygon") {
    const n = g.punkte.length;
    for (let i = 0; i < n; i++) {
      const a = g.punkte[i], b = g.punkte[(i + 1) % n];
      laenge += Math.hypot(b[0] - a[0], b[1] - a[1]);
      flaeche += a[0] * b[1] - b[0] * a[1];
    }
    flaeche = Math.abs(flaeche) / 2;
  } else if (g.art === "rechteck") {
    const w = Math.abs(g.bis[0] - g.von[0]), h = Math.abs(g.bis[1] - g.von[1]);
    laenge = 2 * (w + h); flaeche = w * h;
  } else if (g.art === "kreis") {
    const r = Math.hypot(g.rand[0] - g.mitte[0], g.rand[1] - g.mitte[1]);
    laenge = 2 * Math.PI * r; flaeche = Math.PI * r * r;
  } else if (g.art === "strecke") {
    laenge = Math.hypot(g.bis[0] - g.von[0], g.bis[1] - g.von[1]);
  } else return { laenge_m: null, flaeche_m2: null };
  if (f === null) return { laenge_m: null, flaeche_m2: null };
  return { laenge_m: laenge * f, flaeche_m2: g.art === "strecke" ? null : flaeche * f * f };
}
export function verschieben<G extends Geometrie>(g: G, dx: number, dy: number): G {
  const v = (p: Point): Point => [p[0] + dx, p[1] + dy];
  if (g.art === "polygon") return { ...g, punkte: g.punkte.map(v) };
  if (g.art === "flaeche") return {...g,polygone:g.polygone.map(p=>({...p,aussen:p.aussen.map(v),innen:p.innen?.map(r=>r.map(v))}))};
  if (g.art === "kreis") return { ...g, mitte: v(g.mitte), rand: v(g.rand) };
  if (g.art === "hoehenpunkt") return { ...g, punkt: v(g.punkt) };
  return { ...g, von: v(g.von), bis: v(g.bis) };
}
export function eckpunkte(g: Geometrie): Point[] {
  if(g.art==="flaeche")return g.polygone.flatMap(p=>[...p.aussen,...(p.innen??[]).flat()]);
  if (g.art === "polygon") return g.punkte;
  if (g.art === "kreis") return [g.mitte, g.rand];
  if (g.art === "hoehenpunkt") return [g.punkt];
  return [g.von, g.bis];
}
/** Umgrenzendes Rechteck in Seitenpunkten. */
export function rahmen(ps: Point[]): { x: number; y: number; w: number; h: number } {
  const xs = ps.map((p) => p[0]), ys = ps.map((p) => p[1]);
  const x = Math.min(...xs), y = Math.min(...ys);
  return { x, y, w: Math.max(...xs) - x, h: Math.max(...ys) - y };
}
