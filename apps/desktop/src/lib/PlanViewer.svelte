<script lang="ts" module>
  export type Werkzeug = "verschieben" | "laenge" | "flaeche" | "auswahl";
</script>

<script lang="ts">
  // Zeichnet einen Plan auf ein Canvas und nimmt Aufmaßpunkte entgegen.
  // Gemessen wird nicht hier, sondern in core-lib (onaufmass → api.aufmass).
  import { onMount, untrack } from "svelte";
  import type { Plan, Punkt } from "./api";

  let {
    plan,
    sichtbar,
    werkzeug,
    onaufmass,
    onfertig,
    oncursor,
  }: {
    plan: Plan;
    sichtbar: boolean[];
    werkzeug: Werkzeug;
    /** Laufende Messung (inkl. Cursorpunkt) zur Live-Anzeige. */
    onaufmass: (punkte: Punkt[], geschlossen: boolean) => void;
    /** Abgeschlossene Messung bzw. angeklickte geschlossene Linie. */
    onfertig: (punkte: Punkt[], geschlossen: boolean) => void;
    oncursor: (p: Punkt | null) => void;
  } = $props();

  let huelle: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let breite = $state(0);
  let hoehe = $state(0);

  // Ansicht: Bildschirm = (x·s + ox, −y·s + oy), x/y relativ zu `basis`.
  // Relative Koordinaten halten Landeskoordinaten (UTM) im Genauigkeits-
  // bereich von Canvas-Pfaden.
  let s = $state(1);
  let ox = $state(0);
  let oy = $state(0);

  let punkte = $state<Punkt[]>([]); // Messpunkte in Zeichnungskoordinaten
  let cursor = $state<Punkt | null>(null);
  let fang = $state<Punkt | null>(null);
  let markiert = $state<number | null>(null); // Index in plan.linien

  const basis = $derived<Punkt>(plan.grenzen?.min ?? [0, 0]);

  // Ein Path2D je Ebene und Farbe, einmal pro Plan aufgebaut.
  const pfade = $derived.by(() => {
    const m = new Map<string, { ebene: number; farbe: string | null; pfad: Path2D }>();
    const [bx, by] = basis;
    for (const l of plan.linien) {
      const k = `${l.ebene}|${l.farbe ?? ""}`;
      let e = m.get(k);
      if (!e) m.set(k, (e = { ebene: l.ebene, farbe: l.farbe, pfad: new Path2D() }));
      const [x0, y0] = l.punkte[0];
      e.pfad.moveTo(x0 - bx, y0 - by);
      for (let i = 1; i < l.punkte.length; i++) e.pfad.lineTo(l.punkte[i][0] - bx, l.punkte[i][1] - by);
      if (l.geschlossen) e.pfad.closePath();
    }
    return [...m.values()];
  });

  // Rasterindex aller Eckpunkte für den Punktfang.
  const raster = $derived.by(() => {
    const g = plan.grenzen;
    const zelle = g ? Math.max(g.max[0] - g.min[0], g.max[1] - g.min[1], 1e-9) / 256 : 1;
    const zellen = new Map<string, { p: Punkt; ebene: number }[]>();
    for (const l of plan.linien)
      for (const p of l.punkte) {
        const k = `${Math.floor(p[0] / zelle)}|${Math.floor(p[1] / zelle)}`;
        let z = zellen.get(k);
        if (!z) zellen.set(k, (z = []));
        z.push({ p, ebene: l.ebene });
      }
    return { zelle, zellen };
  });

  const nachZeichnung = (sx: number, sy: number): Punkt => [(sx - ox) / s + basis[0], -(sy - oy) / s + basis[1]];
  const nachBildschirm = (p: Punkt): [number, number] => [(p[0] - basis[0]) * s + ox, -(p[1] - basis[1]) * s + oy];

  export function einpassen() {
    const g = plan.grenzen;
    if (!g || !breite || !hoehe) return;
    const w = Math.max(g.max[0] - g.min[0], 1e-9);
    const h = Math.max(g.max[1] - g.min[1], 1e-9);
    s = Math.min(breite / w, hoehe / h) * 0.92;
    ox = (breite - w * s) / 2;
    oy = hoehe - (hoehe - h * s) / 2;
  }

  export function abbrechen() {
    punkte = [];
    markiert = null;
  }

  function farbeVon(name: string) {
    return getComputedStyle(huelle).getPropertyValue(name).trim();
  }

  function zeichnen() {
    const ctx = canvas?.getContext("2d");
    if (!ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const vg = farbeVon("--fg");
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = farbeVon("--bg");
    ctx.fillRect(0, 0, breite, hoehe);

    // Geometrie in Zeichnungskoordinaten zeichnen; Linienbreite 1 px.
    ctx.setTransform(dpr * s, 0, 0, -dpr * s, dpr * ox, dpr * oy);
    ctx.lineWidth = 1 / s;
    ctx.lineJoin = "round";
    for (const { ebene, farbe, pfad } of pfade) {
      if (!sichtbar[ebene]) continue;
      ctx.strokeStyle = farbe ?? plan.ebenen[ebene]?.farbe ?? vg;
      ctx.stroke(pfad);
    }

    // Texte nur, wenn sie lesbar groß sind.
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.textBaseline = "alphabetic";
    for (const t of plan.texte) {
      const px = t.hoehe * s;
      if (!sichtbar[t.ebene] || px < 4) continue;
      const [x, y] = nachBildschirm(t.position);
      if (x < -500 || y < -100 || x > breite + 500 || y > hoehe + 500) continue;
      ctx.save();
      ctx.translate(x, y);
      ctx.rotate((-t.drehung * Math.PI) / 180);
      ctx.fillStyle = t.farbe ?? plan.ebenen[t.ebene]?.farbe ?? vg;
      ctx.font = `${px}px system-ui, sans-serif`;
      ctx.textAlign = t.ausrichtung === "mitte" ? "center" : t.ausrichtung === "rechts" ? "right" : "left";
      t.inhalt.split("\n").forEach((zeile, i) => ctx.fillText(zeile, 0, i * t.zeilenabstand * s));
      ctx.restore();
    }

    const akzent = farbeVon("--accent");
    const linie = (ps: Punkt[], geschlossen: boolean, fuellen: boolean) => {
      if (ps.length === 0) return;
      ctx.beginPath();
      ps.forEach((p, i) => {
        const [x, y] = nachBildschirm(p);
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      });
      if (geschlossen) ctx.closePath();
      if (fuellen) {
        ctx.globalAlpha = 0.18;
        ctx.fill();
        ctx.globalAlpha = 1;
      }
      ctx.stroke();
    };
    ctx.strokeStyle = akzent;
    ctx.fillStyle = akzent;
    ctx.lineWidth = 2;

    if (markiert !== null) linie(plan.linien[markiert].punkte, true, true);

    if (punkte.length) {
      const flaeche = werkzeug === "flaeche";
      const mitCursor = cursor ? [...punkte, fang ?? cursor] : punkte;
      linie(mitCursor, flaeche && mitCursor.length > 2, flaeche);
      for (const p of punkte) {
        const [x, y] = nachBildschirm(p);
        ctx.fillRect(x - 3, y - 3, 6, 6);
      }
    }
    if (fang && werkzeug !== "verschieben") {
      const [x, y] = nachBildschirm(fang);
      ctx.strokeRect(x - 6, y - 6, 12, 12);
    }
  }

  // Neu zeichnen, wenn sich etwas Sichtbares ändert.
  $effect(() => {
    void [pfade, sichtbar.join(), s, ox, oy, breite, hoehe, punkte, cursor, fang, markiert, werkzeug];
    zeichnen();
  });

  // Neuer Plan: Messung verwerfen und einpassen.
  $effect(() => {
    void plan;
    untrack(() => {
      abbrechen();
      einpassen();
    });
  });

  // Werkzeugwechsel bricht die laufende Messung ab.
  $effect(() => {
    if (werkzeug) untrack(() => {
      punkte = [];
      if (werkzeug !== "auswahl") markiert = null;
    });
  });

  function fangpunkt(sx: number, sy: number): Punkt | null {
    const radius = 10 / s; // 10 px
    const [x, y] = nachZeichnung(sx, sy);
    const { zelle, zellen } = raster;
    let best: Punkt | null = null;
    let bestD = radius;
    const n = Math.ceil(radius / zelle);
    const cx = Math.floor(x / zelle);
    const cy = Math.floor(y / zelle);
    // Stark herausgezoomt deckt der Fangradius einen großen Teil des Plans ab: kein Fang.
    if (n > 32) return null;
    for (let i = -n; i <= n; i++)
      for (let j = -n; j <= n; j++)
        for (const { p, ebene } of zellen.get(`${cx + i}|${cy + j}`) ?? []) {
          if (!sichtbar[ebene]) continue;
          const d = Math.hypot(p[0] - x, p[1] - y);
          if (d < bestD) [best, bestD] = [p, d];
        }
    return best;
  }

  function innerhalb(p: Punkt, ps: Punkt[]) {
    let drin = false;
    for (let i = 0, j = ps.length - 1; i < ps.length; j = i++) {
      const [xi, yi] = ps[i];
      const [xj, yj] = ps[j];
      if (yi > p[1] !== yj > p[1] && p[0] < ((xj - xi) * (p[1] - yi)) / (yj - yi) + xi) drin = !drin;
    }
    return drin;
  }

  /** Kleinste sichtbare geschlossene Linie unter dem Punkt (Rangfolge nach Umgebungsrechteck). */
  function geschlosseneLinieBei(p: Punkt): number | null {
    let best: number | null = null;
    let bestGroesse = Infinity;
    plan.linien.forEach((l, i) => {
      if (!l.geschlossen || !sichtbar[l.ebene] || l.punkte.length < 3) return;
      let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
      for (const [x, y] of l.punkte) {
        x0 = Math.min(x0, x); y0 = Math.min(y0, y); x1 = Math.max(x1, x); y1 = Math.max(y1, y);
      }
      if (p[0] < x0 || p[0] > x1 || p[1] < y0 || p[1] > y1) return;
      const groesse = (x1 - x0) * (y1 - y0);
      if (groesse < bestGroesse && innerhalb(p, l.punkte)) [best, bestGroesse] = [i, groesse];
    });
    return best;
  }

  let ziehen: { x: number; y: number; ox: number; oy: number } | null = null;

  function lokal(e: MouseEvent): [number, number] {
    const r = canvas.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top];
  }

  function druecken(e: PointerEvent) {
    canvas.setPointerCapture(e.pointerId);
    if (e.button === 1 || e.button === 2 || (e.button === 0 && werkzeug === "verschieben")) {
      ziehen = { x: e.clientX, y: e.clientY, ox, oy };
      e.preventDefault();
    }
  }

  function bewegen(e: PointerEvent) {
    if (ziehen) {
      ox = ziehen.ox + e.clientX - ziehen.x;
      oy = ziehen.oy + e.clientY - ziehen.y;
      return;
    }
    const [sx, sy] = lokal(e);
    cursor = nachZeichnung(sx, sy);
    fang = werkzeug === "laenge" || werkzeug === "flaeche" ? fangpunkt(sx, sy) : null;
    oncursor(fang ?? cursor);
    if (punkte.length) onaufmass([...punkte, fang ?? cursor], werkzeug === "flaeche");
  }

  function loslassen(e: PointerEvent) {
    const warZiehen = ziehen && (Math.abs(e.clientX - ziehen.x) > 3 || Math.abs(e.clientY - ziehen.y) > 3);
    ziehen = null;
    if (warZiehen || e.button !== 0) return;
    const [sx, sy] = lokal(e);
    const p = fang ?? nachZeichnung(sx, sy);
    if (werkzeug === "laenge" || werkzeug === "flaeche") {
      // Doppelklick liefert zwei Klicks auf dieselbe Stelle: nur einmal zählen.
      const letzter = punkte.at(-1);
      if (!letzter || letzter[0] !== p[0] || letzter[1] !== p[1]) punkte = [...punkte, p];
      onaufmass(punkte, werkzeug === "flaeche");
    } else if (werkzeug === "auswahl") {
      markiert = geschlosseneLinieBei(p);
      if (markiert !== null) onfertig(plan.linien[markiert].punkte, true);
    }
  }

  export function abschliessen() {
    const flaeche = werkzeug === "flaeche";
    if (punkte.length >= (flaeche ? 3 : 2)) onfertig(punkte, flaeche);
    punkte = [];
  }

  export function letztenPunktEntfernen() {
    punkte = punkte.slice(0, -1);
    if (punkte.length) onaufmass(punkte, werkzeug === "flaeche");
  }

  function rad(e: WheelEvent) {
    e.preventDefault();
    const [sx, sy] = lokal(e);
    const f = Math.exp(-e.deltaY * (e.deltaMode === 1 ? 0.05 : 0.0015));
    const neu = Math.min(Math.max(s * f, 1e-6), 1e7);
    // Punkt unter dem Cursor bleibt stehen.
    ox = sx - ((sx - ox) * neu) / s;
    oy = sy - ((sy - oy) * neu) / s;
    s = neu;
  }

  onMount(() => {
    const ro = new ResizeObserver(([eintrag]) => {
      const erst = breite === 0;
      breite = eintrag.contentRect.width;
      hoehe = eintrag.contentRect.height;
      const dpr = window.devicePixelRatio || 1;
      canvas.width = Math.round(breite * dpr);
      canvas.height = Math.round(hoehe * dpr);
      if (erst) einpassen();
      zeichnen();
    });
    ro.observe(huelle);
    const dunkel = matchMedia("(prefers-color-scheme: dark)");
    dunkel.addEventListener("change", zeichnen);
    return () => {
      ro.disconnect();
      dunkel.removeEventListener("change", zeichnen);
    };
  });
</script>

<div class="planflaeche" bind:this={huelle}>
  <canvas
    bind:this={canvas}
    class:messen={werkzeug !== "verschieben"}
    onpointerdown={druecken}
    onpointermove={bewegen}
    onpointerup={loslassen}
    onpointerleave={() => { cursor = null; fang = null; oncursor(null); }}
    ondblclick={abschliessen}
    oncontextmenu={(e) => e.preventDefault()}
    onwheel={rad}
  ></canvas>
</div>
