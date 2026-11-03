// Kollisionsfreie Beschriftung der Markierungen im Zeicheneditor.
// Jede Beschriftung bekommt die erste freie von mehreren Lagen um ihre Figur.
// Findet sich keine, bleibt sie ausgeblendet; der Name steht dann im Tooltip
// der Figur und erscheint, sobald die Markierung ausgewählt ist.
export type Punkt = [number, number];

export interface Beschriftungswunsch {
  id: string;
  text: string;
  /** Eckpunkte der Figur in Seitenpunkten. */
  punkte: Punkt[];
  /** Ausgewählte Markierungen werden immer und zuerst beschriftet. */
  erzwingen?: boolean;
}

export interface Beschriftungslage {
  x: number;
  y: number;
  anker: "start" | "middle";
}

export interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Geschätzte Textbreite: mittlere Zeichenbreite der Systemschrift. */
export function textbreite(text: string, schrift: number): number {
  return text.length * schrift * 0.56;
}

function ueberlappt(a: Box, b: Box): boolean {
  return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
}

function umgrenzung(ps: Punkt[]): Box {
  const xs = ps.map((p) => p[0]),
    ys = ps.map((p) => p[1]);
  const x = Math.min(...xs),
    y = Math.min(...ys);
  return { x, y, w: Math.max(...xs) - x, h: Math.max(...ys) - y };
}

/**
 * @param schrift Schriftgröße in Seitenpunkten (Bildschirmgröße / Zoom)
 * @param abstand kleiner Versatz zur Figur in Seitenpunkten
 * @param fest Flächen, die nicht überdeckt werden dürfen
 * @returns Lage je Markierung oder `null`, wenn die Beschriftung ausgeblendet wird
 */
export function beschriftungenPlatzieren(
  wuensche: Beschriftungswunsch[],
  schrift: number,
  abstand: number,
  /** Schon belegte Flächen, z. B. die Namen der Planausschnitte. */
  fest: Box[] = [],
): Map<string, Beschriftungslage | null> {
  const ergebnis = new Map<string, Beschriftungslage | null>();
  const belegt: Box[] = [...fest];
  const mitFigur = wuensche
    .filter((w) => w.punkte.length && w.text)
    .map((w) => ({ w, box: umgrenzung(w.punkte) }));
  // Reihenfolge: ausgewählte zuerst, dann große Figuren vor kleinen (die
  // großen tragen die Übersicht, z. B. Grundflächen).
  mitFigur.sort(
    (a, b) =>
      Number(!!b.w.erzwingen) - Number(!!a.w.erzwingen) ||
      b.box.w * b.box.h - a.box.w * a.box.h,
  );
  const h = schrift * 1.2;
  for (const { w, box } of mitFigur) {
    const breite = textbreite(w.text, schrift);
    const [x0, y0] = w.punkte[0];
    const mx = box.x + box.w / 2,
      my = box.y + box.h / 2;
    // Lagen als Grundlinienpunkt + Anker; die Box beschreibt die Textfläche.
    const lagen: Beschriftungslage[] = [
      { x: x0 + abstand, y: y0 - abstand, anker: "start" }, // über der ersten Ecke (bisher)
      { x: box.x + abstand, y: box.y + schrift + abstand, anker: "start" }, // innen oben links
      { x: mx, y: my + schrift * 0.35, anker: "middle" }, // Mitte der Figur
      { x: box.x + abstand, y: box.y + box.h + schrift + abstand, anker: "start" }, // unter der Figur
      { x: box.x + abstand, y: box.y - abstand, anker: "start" }, // über der Figur
    ];
    const boxVon = (l: Beschriftungslage): Box => ({
      x: l.anker === "middle" ? l.x - breite / 2 : l.x,
      y: l.y - schrift,
      w: breite,
      h,
    });
    let gewaehlt: Beschriftungslage | null = null;
    for (const l of lagen) {
      if (!belegt.some((b) => ueberlappt(b, boxVon(l)))) {
        gewaehlt = l;
        break;
      }
    }
    if (!gewaehlt && w.erzwingen) gewaehlt = lagen[0];
    if (gewaehlt) belegt.push(boxVon(gewaehlt));
    ergebnis.set(w.id, gewaehlt);
  }
  for (const w of wuensche) if (!ergebnis.has(w.id)) ergebnis.set(w.id, null);
  return ergebnis;
}
