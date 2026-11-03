/**
 * Konstruktionsbefehle des Zeicheneditors (Spiegeln, Versatz, Stutzen/Dehnen,
 * Reihe, Fensterauswahl). Reine Geometrie in Blattkoordinaten (Seitenpunkte bzw.
 * Meter bei eigenen Zeichnungen, y nach unten); Messwerte rechnet weiterhin der
 * Core. Ohne Laufzeit-Importe, damit `node --test` die Datei direkt prüfen kann.
 */
import type { Plangeometrie, Zeichenform } from "../generated/plattform";

export type Punkt = [number, number];

const abstand = (a: Punkt, b: Punkt) => Math.hypot(b[0] - a[0], b[1] - a[1]);

/** Alle Punkte einer Geometrie (Rechteck als vier Ecken). */
export function punkteVon(g: Plangeometrie): Punkt[] {
  if (g.art === "flaeche") return g.polygone.flatMap(p => [...p.aussen, ...(p.innen ?? []).flat()]) as Punkt[];
  if (g.art === "polygon") return g.punkte as Punkt[];
  if (g.art === "kreis") return [g.mitte, g.rand] as Punkt[];
  if (g.art === "hoehenpunkt") return [g.punkt as Punkt];
  if (g.art === "rechteck") return [g.von, [g.bis[0], g.von[1]], g.bis, [g.von[0], g.bis[1]]] as Punkt[];
  return [g.von, g.bis] as Punkt[];
}

/** Kanten zum Schnitttest der Kreuzungsauswahl. */
export function kantenVon(g: Plangeometrie): [Punkt, Punkt][] {
  const ring = (ps: Punkt[]) => ps.map((p, i) => [p, ps[(i + 1) % ps.length]] as [Punkt, Punkt]);
  if (g.art === "flaeche") return g.polygone.flatMap(p => [p.aussen, ...(p.innen ?? [])].flatMap(r => ring(r as Punkt[])));
  if (g.art === "polygon" || g.art === "rechteck") return ring(punkteVon(g));
  if (g.art === "kreis") {
    const r = abstand(g.mitte as Punkt, g.rand as Punkt);
    const ps: Punkt[] = Array.from({ length: 24 }, (_, i) => [g.mitte[0] + Math.cos(i * Math.PI / 12) * r, g.mitte[1] + Math.sin(i * Math.PI / 12) * r]);
    return ring(ps);
  }
  if (g.art === "strecke") return [[g.von as Punkt, g.bis as Punkt]];
  return [];
}

/** Punkte eines Zeichenelements (Wand, Maßkette: Achse; Text: Einfügepunkt). */
export const elementpunkte = (f: Zeichenform): Punkt[] => ("punkt" in f ? [f.punkt as Punkt] : [f.von as Punkt, f.bis as Punkt]);

/** Wendet `punkt` auf jede Koordinate an; Rechtecke werden dabei zu Polygonen (gedrehte Lage bleibt erhalten). */
export function abbildenGeometrie(g: Plangeometrie, punkt: (p: Punkt) => Punkt): Plangeometrie {
  if (g.art === "polygon") return { ...g, punkte: g.punkte.map(p => punkt(p as Punkt)) };
  if (g.art === "flaeche") return { ...g, polygone: g.polygone.map(p => ({ ...p, aussen: p.aussen.map(q => punkt(q as Punkt)), innen: p.innen?.map(r => r.map(q => punkt(q as Punkt))) })) };
  if (g.art === "kreis") return { ...g, mitte: punkt(g.mitte as Punkt), rand: punkt(g.rand as Punkt) };
  if (g.art === "hoehenpunkt") return { ...g, punkt: punkt(g.punkt as Punkt) };
  if (g.art === "rechteck") return { art: "polygon", punkte: punkteVon(g).map(punkt) };
  return { ...g, von: punkt(g.von as Punkt), bis: punkt(g.bis as Punkt) };
}
export function abbildenElement<T extends Zeichenform>(f: T, punkt: (p: Punkt) => Punkt): T {
  if ("punkt" in f) return { ...f, punkt: punkt(f.punkt as Punkt) };
  return { ...f, von: punkt(f.von as Punkt), bis: punkt(f.bis as Punkt) };
}

export const verschiebung = (dx: number, dy: number) => (p: Punkt): Punkt => [p[0] + dx, p[1] + dy];

/** Spiegelung an der Geraden a–b; bei a = b unverändert. */
export function spiegelung(a: Punkt, b: Punkt): (p: Punkt) => Punkt {
  const dx = b[0] - a[0], dy = b[1] - a[1], l2 = dx * dx + dy * dy;
  if (!(l2 > 0)) return p => p;
  return p => {
    const t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2;
    const f: Punkt = [a[0] + t * dx, a[1] + t * dy];
    return [2 * f[0] - p[0], 2 * f[1] - p[1]];
  };
}
/** Spiegelt ein Element; Maßketten behalten ihre Seite relativ zum Bauteil (Abstand wechselt das Vorzeichen). */
export function elementSpiegeln<T extends Zeichenform>(f: T, a: Punkt, b: Punkt): T {
  const s = abbildenElement(f, spiegelung(a, b));
  if (s.art === "masskette") return { ...s, abstand: -s.abstand };
  if (s.art === "wand" && s.oeffnungen?.length) {
    // Achse läuft gespiegelt weiter; Türanschlag wechselt die Seite.
    return { ...s, oeffnungen: s.oeffnungen.map(o => ({ ...o, links: !o.links })) };
  }
  if (s.art === "text" || s.art === "symbol") {
    // Text bleibt lesbar: nur die Richtung wird gespiegelt.
    const w = (Math.atan2(b[1] - a[1], b[0] - a[0]) * 180) / Math.PI;
    return { ...s, drehung_grad: ((-2 * w - s.drehung_grad) % 360 + 360) % 360 };
  }
  return s;
}

/** Linksdrehende Fläche (Shoelace, y nach unten) → positive Zahl. */
function flaecheVorzeichen(ps: Punkt[]): number {
  let s = 0;
  for (let i = 0; i < ps.length; i++) { const a = ps[i], b = ps[(i + 1) % ps.length]; s += a[0] * b[1] - b[0] * a[1]; }
  return s / 2;
}
/** Schnittpunkt der unendlichen Geraden a1–a2 und b1–b2; parallel → null. */
export function geradenschnitt(a1: Punkt, a2: Punkt, b1: Punkt, b2: Punkt): Punkt | null {
  const r: Punkt = [a2[0] - a1[0], a2[1] - a1[1]], s: Punkt = [b2[0] - b1[0], b2[1] - b1[1]];
  const n = r[0] * s[1] - r[1] * s[0];
  if (Math.abs(n) < 1e-12 * (abstand(a1, a2) * abstand(b1, b2) || 1)) return null;
  const t = ((b1[0] - a1[0]) * s[1] - (b1[1] - a1[1]) * s[0]) / n;
  return [a1[0] + t * r[0], a1[1] + t * r[1]];
}
const parallele = (a: Punkt, b: Punkt, d: number): [Punkt, Punkt] => {
  const l = abstand(a, b) || 1, nx = -(b[1] - a[1]) / l * d, ny = (b[0] - a[0]) / l * d;
  return [[a[0] + nx, a[1] + ny], [b[0] + nx, b[1] + ny]];
};
/**
 * Versatz (Parallele) im Abstand `d`. Strecke: positiv = links der Richtung
 * von→bis auf dem Bildschirm (y nach unten); Polygon/Rechteck: positiv = nach
 * außen (Ecken auf Gehrung); Kreis: Radius + d. Höhenpunkt/Fläche: null.
 */
export function versatz(g: Plangeometrie, d: number): Plangeometrie | null {
  if (!Number.isFinite(d) || d === 0) return null;
  if (g.art === "strecke") { const [von, bis] = parallele(g.von as Punkt, g.bis as Punkt, -d); return { ...g, von, bis }; }
  if (g.art === "kreis") {
    const r = abstand(g.mitte as Punkt, g.rand as Punkt), n = r + d;
    if (!(r > 0) || !(n > 0)) return null;
    return { ...g, rand: [g.mitte[0] + (g.rand[0] - g.mitte[0]) * n / r, g.mitte[1] + (g.rand[1] - g.mitte[1]) * n / r] };
  }
  if (g.art !== "polygon" && g.art !== "rechteck") return null;
  const ps = punkteVon(g);
  if (ps.length < 3) return null;
  // Außen liegt rechts der Kantenrichtung bei positiver Fläche (y nach unten).
  const aussen = flaecheVorzeichen(ps) > 0 ? -d : d;
  const kanten = ps.map((p, i) => parallele(p, ps[(i + 1) % ps.length], aussen));
  const neu: Punkt[] = ps.map((_, i) => {
    const vor = kanten[(i - 1 + ps.length) % ps.length], nach = kanten[i];
    return geradenschnitt(vor[0], vor[1], nach[0], nach[1]) ?? nach[0];
  });
  return { art: "polygon", punkte: neu };
}
export function elementVersatz<T extends Zeichenform>(f: T, d: number): T | null {
  if ("punkt" in f || !Number.isFinite(d) || d === 0) return null;
  const [von, bis] = parallele(f.von as Punkt, f.bis as Punkt, -d);
  return { ...f, von, bis };
}

/**
 * Stutzen/Dehnen: verlegt das Ende der Strecke von→bis, das näher am Schnitt mit
 * der Grenzkante liegt, auf diesen Schnitt. So wird gekürzt, wenn die Grenze
 * die Strecke kreuzt, und verlängert, wenn sie dahinter liegt. Parallel → null.
 */
export function stutzen(von: Punkt, bis: Punkt, grenzeA: Punkt, grenzeB: Punkt): [Punkt, Punkt] | null {
  const s = geradenschnitt(von, bis, grenzeA, grenzeB);
  if (!s) return null;
  const neu: [Punkt, Punkt] = abstand(von, s) < abstand(bis, s) ? [s, bis] : [von, s];
  return abstand(neu[0], neu[1]) > 0 ? neu : null;
}
/** Stutzen/Dehnen für Strecken und Zeichenelemente mit Achse; sonst null. */
export function geometrieStutzen(g: Plangeometrie, a: Punkt, b: Punkt): Plangeometrie | null {
  if (g.art !== "strecke") return null;
  const r = stutzen(g.von as Punkt, g.bis as Punkt, a, b);
  return r ? { ...g, von: r[0], bis: r[1] } : null;
}
export function elementStutzen<T extends Zeichenform>(f: T, a: Punkt, b: Punkt): T | null {
  if ("punkt" in f) return null;
  const r = stutzen(f.von as Punkt, f.bis as Punkt, a, b);
  return r ? { ...f, von: r[0], bis: r[1] } : null;
}
/** Grenzkante einer Geometrie: Strecke selbst, sonst die Kante, die dem Punkt `nahe` am nächsten liegt. */
export function grenzkante(g: Plangeometrie, nahe?: Punkt): [Punkt, Punkt] | null {
  const ks = kantenVon(g);
  if (!ks.length) return null;
  if (!nahe || ks.length === 1) return ks[0];
  return ks.reduce((best, k) => (streckenabstand(nahe, k) < streckenabstand(nahe, best) ? k : best));
}
export function streckenabstand(p: Punkt, [a, b]: [Punkt, Punkt]): number {
  const dx = b[0] - a[0], dy = b[1] - a[1], l2 = dx * dx + dy * dy;
  const t = l2 > 0 ? Math.max(0, Math.min(1, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2)) : 0;
  return abstand(p, [a[0] + t * dx, a[1] + t * dy]);
}

/** Reihe: Versätze der Kopien 1 … anzahl-1 (das Original bleibt Nr. 0); höchstens 200 Kopien. */
export function reihe(anzahl: number, dx: number, dy: number): Punkt[] {
  const n = Math.min(200, Math.floor(anzahl));
  if (!(n >= 2) || !Number.isFinite(dx) || !Number.isFinite(dy) || (dx === 0 && dy === 0)) return [];
  return Array.from({ length: n - 1 }, (_, i) => [dx * (i + 1), dy * (i + 1)] as Punkt);
}

/** Achsparalleles Rechteck aus zwei Ecken. */
export type Fenster = { x0: number; y0: number; x1: number; y1: number };
export const fenster = (a: Punkt, b: Punkt): Fenster => ({ x0: Math.min(a[0], b[0]), y0: Math.min(a[1], b[1]), x1: Math.max(a[0], b[0]), y1: Math.max(a[1], b[1]) });
const drin = (f: Fenster, p: Punkt) => p[0] >= f.x0 && p[0] <= f.x1 && p[1] >= f.y0 && p[1] <= f.y1;
function schneiden(a: Punkt, b: Punkt, c: Punkt, d: Punkt): boolean {
  const o = (p: Punkt, q: Punkt, r: Punkt) => Math.sign((q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]));
  return o(a, b, c) !== o(a, b, d) && o(c, d, a) !== o(c, d, b);
}
/**
 * Fensterauswahl wie in CAD-Programmen: `kreuzend = false` (Aufziehen von links
 * nach rechts) wählt nur vollständig Enthaltenes, `kreuzend = true` (von rechts
 * nach links) auch Berührtes.
 */
export function imFenster(f: Fenster, punkte: Punkt[], kanten: [Punkt, Punkt][], kreuzend: boolean): boolean {
  if (!punkte.length) return false;
  if (!kreuzend) return punkte.every(p => drin(f, p));
  if (punkte.some(p => drin(f, p))) return true;
  const r: Punkt[] = [[f.x0, f.y0], [f.x1, f.y0], [f.x1, f.y1], [f.x0, f.y1]];
  const rand = r.map((p, i) => [p, r[(i + 1) % 4]] as [Punkt, Punkt]);
  if (kanten.some(([a, b]) => rand.some(([c, d]) => schneiden(a, b, c, d)))) return true;
  // Fenster ganz in einer Fläche: berührt sie ebenfalls.
  return kanten.length > 2 && innerhalb(r[0], kanten);
}
function innerhalb(p: Punkt, kanten: [Punkt, Punkt][]): boolean {
  let x = false;
  for (const [a, b] of kanten)
    if ((a[1] > p[1]) !== (b[1] > p[1]) && p[0] < ((b[0] - a[0]) * (p[1] - a[1])) / (b[1] - a[1]) + a[0]) x = !x;
  return x;
}

// ── Präzisionseingabe ────────────────────────────────────────────────────────

/** Rastet die Richtung a→p auf Vielfache von `rasterGrad` (Länge bleibt). */
export function winkelRasten(a: Punkt, p: Punkt, rasterGrad: number): Punkt {
  if (!(rasterGrad > 0)) return p;
  const l = abstand(a, p), schritt = (rasterGrad * Math.PI) / 180;
  const w = Math.round(Math.atan2(p[1] - a[1], p[0] - a[0]) / schritt) * schritt;
  return [a[0] + Math.cos(w) * l, a[1] + Math.sin(w) * l];
}

/** Lotfang: Fußpunkt des Lots vom letzten Punkt auf eine Kante, wenn der Zeiger nahe daran ist. */
export function lotfang(p: Punkt, von: Punkt, kanten: [Punkt, Punkt][], toleranz: number): Punkt | null {
  let best: Punkt | null = null, d = toleranz;
  for (const [a, b] of kanten) {
    const dx = b[0] - a[0], dy = b[1] - a[1], l2 = dx * dx + dy * dy;
    if (!(l2 > 0)) continue;
    const t = ((von[0] - a[0]) * dx + (von[1] - a[1]) * dy) / l2;
    if (t < 0 || t > 1) continue;
    const f: Punkt = [a[0] + t * dx, a[1] + t * dy];
    if (abstand(f, von) < 1e-9) continue;
    const n = abstand(f, p);
    if (n < d) { d = n; best = f; }
  }
  return best;
}

/** Verlängerungsfang: Zeiger nahe der gedachten Verlängerung einer Kante (bis `weite` über das Ende hinaus). */
export function verlaengerungsfang(p: Punkt, kanten: [Punkt, Punkt][], toleranz: number, weite: number): Punkt | null {
  let best: Punkt | null = null, d = toleranz;
  for (const [a, b] of kanten) {
    const dx = b[0] - a[0], dy = b[1] - a[1], l = Math.hypot(dx, dy);
    if (!(l > 0)) continue;
    const t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (l * l);
    const ueber = t < 0 ? -t * l : t > 1 ? (t - 1) * l : 0;
    if (ueber <= toleranz || ueber > weite) continue;
    const f: Punkt = [a[0] + t * dx, a[1] + t * dy], n = abstand(f, p);
    if (n < d) { d = n; best = f; }
  }
  return best;
}

/**
 * Koordinateneingabe in Metern (Komma oder Punkt als Dezimalzeichen):
 * - `5,20` Länge in Zeigerrichtung, `5,20;90` oder `5,20<90` Länge und Winkel
 *   (Grad, 0 = rechts, 90 = oben),
 * - `@3;4` relativ zum letzten Punkt (x nach rechts, y nach oben),
 * - beim Rechteck `Breite;Höhe`.
 */
export type Koordinateneingabe =
  | { art: "polar"; laenge: number; winkel: number | null }
  | { art: "relativ"; dx: number; dy: number }
  | { art: "rechteck"; breite: number; hoehe: number };
export function eingabeAuswerten(text: string, rechteck = false): Koordinateneingabe | null {
  const t = text.trim().replace(/\s+/g, " ");
  if (!t) return null;
  const zahl = (s: string | undefined) => (s === undefined || s.trim() === "" ? NaN : Number(s.trim().replace(",", ".")));
  if (t.startsWith("@")) {
    const [x, y] = t.slice(1).split(/[;\s]+/);
    const dx = zahl(x), dy = zahl(y ?? "0");
    return Number.isFinite(dx) && Number.isFinite(dy) && (dx !== 0 || dy !== 0) ? { art: "relativ", dx, dy } : null;
  }
  const [a, b] = t.split(/[;<\s]+/);
  if (rechteck) {
    const breite = zahl(a), hoehe = zahl(b ?? a);
    return breite > 0 && hoehe > 0 ? { art: "rechteck", breite, hoehe } : null;
  }
  const laenge = zahl(a);
  if (!(laenge > 0)) return null;
  const w = zahl(b);
  return { art: "polar", laenge, winkel: Number.isFinite(w) ? w : null };
}
/** Zielpunkt einer Eingabe in Blattkoordinaten (y nach unten); `faktor` = Meter je Blatteinheit. */
export function eingabePunkt(e: Koordinateneingabe, letzter: Punkt, faktor: number, zeiger: Punkt | null): Punkt {
  if (e.art === "relativ") return [letzter[0] + e.dx / faktor, letzter[1] - e.dy / faktor];
  if (e.art === "rechteck") return [letzter[0] + e.breite / faktor, letzter[1] + e.hoehe / faktor];
  const w = e.winkel !== null ? (e.winkel * Math.PI) / 180 : zeiger ? -Math.atan2(zeiger[1] - letzter[1], zeiger[0] - letzter[0]) : 0;
  return [letzter[0] + (Math.cos(w) * e.laenge) / faktor, letzter[1] - (Math.sin(w) * e.laenge) / faktor];
}

// ── Befehlszeile ─────────────────────────────────────────────────────────────

/** Kürzel der Befehlszeile (deutsch und wie in gängigen CAD-Programmen) → Befehl. */
export const BEFEHLE: Record<string, string> = {
  V: "auswahl", AW: "auswahl", PAN: "verschieben",
  L: "strecke", LI: "strecke", LINIE: "strecke", S: "strecke",
  PL: "polygon", P: "polygon", POLYGON: "polygon",
  R: "rechteck", REC: "rechteck", RECHTECK: "rechteck",
  K: "kreis", C: "kreis", KREIS: "kreis",
  W: "wand", WAND: "wand",
  BM: "masskette", MK: "masskette", DIM: "masskette", MASS: "masskette",
  T: "text", TX: "text", TEXT: "text",
  H: "hoehenpunkt", HP: "hoehenpunkt",
  KO: "kopieren", CO: "kopieren", CP: "kopieren", KOPIEREN: "kopieren",
  SP: "spiegeln", MI: "spiegeln", SPIEGELN: "spiegeln",
  ST: "stutzen", TR: "stutzen", DE: "stutzen", EX: "stutzen", STUTZEN: "stutzen", DEHNEN: "stutzen",
  VS: "versatz", O: "versatz", OFFSET: "versatz", VERSATZ: "versatz",
  RH: "reihe", AR: "reihe", REIHE: "reihe",
  E: "entfernen", DEL: "entfernen", ENTF: "entfernen", LOESCHEN: "entfernen",
  U: "zurueck", Z: "zoomfenster", ZE: "einpassen",
  FE: "oeffnung", TU: "oeffnung", OEFFNUNG: "oeffnung",
  SY: "symbol", PZ: "symbol", SYMBOL: "symbol",
  RS: "raumstempel", RAUM: "raumstempel",
  OR: "ortho", ORTHO: "ortho", WR: "winkelraster",
};
/** Zerlegt `VS 0,5` in Befehl und Zahlen; unbekannte Kürzel → null. */
export function befehlAuswerten(text: string): { befehl: string; zahlen: number[] } | null {
  const [kopf, ...rest] = text.trim().split(/\s+/);
  if (!kopf) return null;
  const befehl = BEFEHLE[kopf.toUpperCase().replace("Ö", "OE")];
  if (!befehl) return null;
  const zahlen = rest.map(z => Number(z.replace(",", "."))).filter(Number.isFinite);
  return { befehl, zahlen };
}
