<script lang="ts">
  /** Grundriss-Schnitt eines Geschosses (vom Core geschnitten, Meter). Klick
   * wählt ein Element; mit `onpunkt` liefert ein Klick stattdessen den
   * Modellpunkt (Werkzeuge der Nacharbeit). */
  import type { Modellgrundriss, Planpolygon } from "../generated/plattform";
  let { grundriss, auswahl = "", markiert = [], onauswahl, onpunkt, hinweis = "", zusatz = [] }: {
    grundriss: Modellgrundriss;
    auswahl?: string;
    markiert?: string[];
    onauswahl?: (globalId: string) => void;
    onpunkt?: (p: [number, number]) => void;
    hinweis?: string;
    /** Zusätzliche Umrisse (z. B. Vorschau eines neuen Raums). */
    zusatz?: Planpolygon[];
  } = $props();
  const rand = 1;
  const basis = $derived({ x: grundriss.min[0] - rand, y: -grundriss.max[1] - rand, w: grundriss.max[0] - grundriss.min[0] + 2 * rand, h: grundriss.max[1] - grundriss.min[1] + 2 * rand });
  let zoom = $state(1), versatz = $state<[number, number]>([0, 0]);
  const box = $derived({ x: basis.x + versatz[0] + basis.w * (1 - 1 / zoom) / 2, y: basis.y + versatz[1] + basis.h * (1 - 1 / zoom) / 2, w: basis.w / zoom, h: basis.h / zoom });
  const r = (n: number) => +n.toFixed(4);
  const pfad = (ps: Planpolygon[]) => ps.map(p => [p.aussen, ...(p.innen ?? [])].map(ring => ring.map((q, i) => `${i ? "L" : "M"}${r(q[0])} ${r(-q[1])}`).join(" ") + " Z").join(" ")).join(" ");
  const mitte = (ps: Planpolygon[]) => {
    const a = ps[0]?.aussen ?? []; if (!a.length) return [0, 0];
    const xs = a.map(p => p[0]), ys = a.map(p => p[1]);
    return [(Math.min(...xs) + Math.max(...xs)) / 2, -(Math.min(...ys) + Math.max(...ys)) / 2];
  };
  const raeume = $derived(grundriss.teile.filter(t => t.raum));
  const bauteile = $derived(grundriss.teile.filter(t => !t.raum));
  const mark = $derived(new Set(markiert));
  let svg = $state<SVGSVGElement>();
  let ziehen: { x: number; y: number; v: [number, number]; bewegt: boolean } | null = null;
  function modellpunkt(e: MouseEvent): [number, number] | null {
    const m = svg?.getScreenCTM(); if (!m || !svg) return null;
    const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(m.inverse());
    return [p.x, -p.y];
  }
  function rad(e: WheelEvent) {
    e.preventDefault();
    zoom = Math.min(40, Math.max(0.5, zoom * (e.deltaY < 0 ? 1.2 : 1 / 1.2)));
  }
  function runter(e: PointerEvent) { ziehen = { x: e.clientX, y: e.clientY, v: [...versatz], bewegt: false }; }
  function bewegen(e: PointerEvent) {
    if (!ziehen || !svg || !(e.buttons & 1)) return;
    const s = box.w / svg.clientWidth, dx = e.clientX - ziehen.x, dy = e.clientY - ziehen.y;
    if (Math.hypot(dx, dy) > 3) ziehen.bewegt = true;
    if (ziehen.bewegt) versatz = [ziehen.v[0] - dx * s, ziehen.v[1] - dy * s];
  }
  function hoch(e: PointerEvent, gid: string | null) {
    const war = ziehen; ziehen = null;
    if (war?.bewegt) return;
    if (onpunkt) { const p = modellpunkt(e); if (p) onpunkt([+p[0].toFixed(4), +p[1].toFixed(4)]); return; }
    if (gid) onauswahl?.(gid);
  }
  function taste(e: KeyboardEvent, gid: string) { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onauswahl?.(gid); } }
</script>

<div class="grundriss">
  <div class="leiste" role="toolbar" aria-label="Grundriss">
    <button type="button" class="sekundaer" onclick={() => (zoom = Math.min(40, zoom * 1.5))}>Vergrößern</button>
    <button type="button" class="sekundaer" onclick={() => (zoom = Math.max(0.5, zoom / 1.5))}>Verkleinern</button>
    <button type="button" class="sekundaer" onclick={() => { zoom = 1; versatz = [0, 0]; }}>Alles zeigen</button>
    <span class="meta">Schnitt {grundriss.schnitthoehe_m.toLocaleString("de-DE")} m über Geschossebene{hinweis ? ` · ${hinweis}` : ""}</span>
  </div>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <svg bind:this={svg} role="application" aria-label="Grundriss-Schnitt" class:punktwahl={!!onpunkt} viewBox={`${box.x} ${box.y} ${box.w} ${box.h}`} preserveAspectRatio="xMidYMid meet"
    onwheel={rad} onpointerdown={runter} onpointermove={bewegen} onpointerup={e => hoch(e, null)}>
    {#each raeume as t (t.global_id)}
      <path class="raum" class:gewaehlt={t.global_id === auswahl} class:markiert={mark.has(t.global_id)} d={pfad(t.polygone)} fill-rule="evenodd" role="button" tabindex="0" aria-label={`Raum ${t.name}`}
        onpointerup={e => { e.stopPropagation(); hoch(e, t.global_id); }} onkeydown={e => taste(e, t.global_id)}><title>{t.titel} {t.name}</title></path>
    {/each}
    {#each bauteile as t (t.global_id)}
      <path class="bauteil" class:projektion={t.projektion} class:gewaehlt={t.global_id === auswahl} class:markiert={mark.has(t.global_id)} d={pfad(t.polygone)} fill-rule="evenodd" role="button" tabindex="0" aria-label={`${t.titel} ${t.name}`}
        onpointerup={e => { e.stopPropagation(); hoch(e, t.global_id); }} onkeydown={e => taste(e, t.global_id)}><title>{t.titel} {t.name}</title></path>
    {/each}
    {#each zusatz as z, i (i)}<path class="zusatz" d={pfad([z])} fill-rule="evenodd"/>{/each}
    {#each raeume as t (t.global_id)}
      {@const m = mitte(t.polygone)}
      <text class="raumname" x={m[0]} y={m[1]} font-size={Math.max(0.18, box.w / 70)}>{t.name}</text>
    {/each}
  </svg>
</div>

<style>
  .grundriss{display:flex;flex-direction:column;height:100%;min-height:0}
  .leiste{display:flex;gap:4px;align-items:center;padding:4px;border-bottom:1px solid var(--border-innen);background:var(--leiste)}
  .leiste button{min-height:24px;padding:0 8px;font-size:.85rem}
  svg{flex:1;min-height:320px;width:100%;background:#fff;touch-action:none;cursor:default}
  svg.punktwahl{cursor:crosshair}
  .raum{fill:#e7eef6;stroke:#7a8a99;stroke-width:.02;cursor:pointer}
  .bauteil{fill:#4a5866;stroke:#1d2731;stroke-width:.01;cursor:pointer}
  .bauteil.projektion{fill:#9aa6b2;fill-opacity:.7}
  .markiert{fill:#f2b84b}
  .gewaehlt{fill:#1d6fd6;fill-opacity:.85}
  .raum.gewaehlt{fill-opacity:.35}
  .zusatz{fill:#1d6fd6;fill-opacity:.25;stroke:#1d6fd6;stroke-width:.04;stroke-dasharray:.15 .1;pointer-events:none}
  .raumname{text-anchor:middle;dominant-baseline:middle;fill:#1d2731;pointer-events:none}
  path:focus-visible{outline:none;stroke:#1d6fd6;stroke-width:.06}
</style>
