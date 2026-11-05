<script lang="ts">
  import type {
    AbstandsPlan,
    AbstandsErgebnis,
    APunkt,
  } from "../generated/plattform";
  let {
    plan,
    ergebnis,
    auswahl = $bindable(""),
  }: {
    plan: AbstandsPlan;
    ergebnis: AbstandsErgebnis | null;
    auswahl?: string;
  } = $props();
  const alle = $derived(
    [
      ...plan.grundstuecke.flatMap((g) => g.grenze),
      ...plan.flaechen.flatMap((f) => f.polygon),
      ...(plan.umrisse ?? []).flatMap((u) => u.ecken),
      ...plan.waende
        .filter((w) => !w.breite_aus_ansicht)
        .flatMap((w) => [w.von, w.bis]),
      ...(ergebnis?.waende.flatMap((w) => w.polygon) ?? []),
    ].filter((p) => Number.isFinite(p.x) && Number.isFinite(p.y)),
  );
  const box = $derived.by(() => {
    let x = Infinity,
      y = Infinity,
      maxx = -Infinity,
      maxy = -Infinity;
    for (const p of alle) {
      x = Math.min(x, p.x);
      y = Math.min(y, p.y);
      maxx = Math.max(maxx, p.x);
      maxy = Math.max(maxy, p.y);
    }
    if (!alle.length) {
      x = 0;
      y = 0;
      maxx = 10;
      maxy = 10;
    }
    const w = Math.max(10, maxx - x),
      h = Math.max(10, maxy - y);
    return { x, y, w, h, pad: Math.max(w, h) * 0.06 };
  });
  const points = (p: APunkt[]) => p.map((p) => `${p.x},${-p.y}`).join(" ");
  const path = (p: APunkt[]) =>
    p.length ? "M " + p.map((p) => `${p.x} ${-p.y}`).join(" L ") + " Z" : "";
</script>

<figure>
  <svg
    role="img"
    aria-label="Abstandsflächenplan mit Grundstücksgrenzen und Konflikten"
    viewBox={`${box.x - box.pad} ${-box.y - box.h - box.pad} ${box.w + 2 * box.pad} ${box.h + 2 * box.pad}`}
  >
    <title
      >Grundstücke schwarz, Abstandsflächen blau, Konflikte rot, zusätzliche
      Flächen grün/grau</title
    >
    {#each plan.grundstuecke as g}<path
        d={[path(g.grenze), ...(g.innenringe ?? []).map(path)].join(" ")}
        fill="none"
        stroke="currentColor"
        vector-effect="non-scaling-stroke"
      />{/each}
    {#each plan.flaechen as f}<polygon
        points={points(f.polygon)}
        fill={f.art === "baukoerper" ? "#9993" : "#39895a18"}
        stroke={f.art === "baukoerper" ? "#777" : "#39895a"}
        stroke-dasharray={f.wirksam ? "" : "4 3"}
        vector-effect="non-scaling-stroke"
      />{/each}
    {#each plan.umrisse ?? [] as u}<polygon
        points={points(u.ecken)}
        fill="#5553"
        stroke="#555"
        vector-effect="non-scaling-stroke"
      />{/each}
    {#each ergebnis?.waende ?? [] as e}<polygon
        points={points(e.polygon)}
        fill={e.id === auswahl ? "#367bb744" : "#367bb718"}
        stroke="#367bb7"
        vector-effect="non-scaling-stroke"
      />{#each e.konflikte as k}<path
          d={[path(k.aussen), ...k.innen.map(path)].join(" ")}
          fill="#ce443c66"
          fill-rule="evenodd"
          stroke="#ce443c"
          vector-effect="non-scaling-stroke"
        />{/each}{/each}
    <!-- Wände mit Breite aus der Ansicht haben keine Lage im Plan. -->
    {#each plan.waende as w, i}{#if !w.breite_aus_ansicht}<line
        role="button"
        tabindex="0"
        aria-label={`W${i + 1}: ${w.bezeichnung} auswählen`}
        aria-pressed={auswahl === w.id}
        onclick={() => (auswahl = w.id)}
        onkeydown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            auswahl = w.id;
          }
        }}
        x1={w.von.x}
        y1={-w.von.y}
        x2={w.bis.x}
        y2={-w.bis.y}
        stroke={w.id === auswahl ? "#bd6c15" : "#235b86"}
        stroke-width={w.id === auswahl ? 4 : 2}
        vector-effect="non-scaling-stroke"
      /><text
        x={(w.von.x + w.bis.x) / 2}
        y={-(w.von.y + w.bis.y) / 2 - box.pad * 0.15}
        font-size={box.pad * 0.65}
        fill="currentColor">W{i + 1}</text
      >{/if}{/each}
    {#each ergebnis?.ueberdeckungen ?? [] as u}{#each u.polygone as p}<path
          d={[path(p.aussen), ...p.innen.map(path)].join(" ")}
          fill={u.zulaessig ? "#8b419322" : "#8b41934d"}
          fill-rule="evenodd"
        />{/each}{/each}
  </svg>
  <figcaption>
    Planskizze in gemeinsamem Meterbezug; kein vermessener Lageplan. Blau:
    Abstandsfläche · Rot: Konflikt · Violett: Überdeckung (hell: nach
    Winkelregel zulässig) · Grau: Gebäudeumriss. Die Wandwahl hebt denselben
    Abschnitt hervor.
  </figcaption>
</figure>

<style>
  figure {
    margin: 1rem 0;
  }
  svg {
    width: 100%;
    height: 420px;
    max-width: 100%;
    background: var(--panel, #f8fafc);
    border: 1px solid var(--border, #ccd5df);
  }
  line[role="button"] {
    cursor: pointer;
    pointer-events: stroke;
  }
  line[role="button"]:focus {
    outline: none;
    stroke: var(--focus, #c17b20);
    stroke-width: 4;
  }
  figcaption {
    font-size: 0.85rem;
    margin-top: 0.5rem;
  }
</style>
