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
