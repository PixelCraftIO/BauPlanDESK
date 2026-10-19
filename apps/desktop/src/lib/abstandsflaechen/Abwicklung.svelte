<script lang="ts">
  import type { WandErgebnis } from "../generated/plattform";
  // Darstellung des im Core berechneten Profils; keine eigene Rechnung.
  let { ergebnis }: { ergebnis: WandErgebnis } = $props();
  const profil = $derived(ergebnis.profil ?? []);
  const box = $derived.by(() => {
    const g = Math.min(...profil.map((p) => p.gelaende_m));
    const o = Math.max(...profil.map((p) => p.oberkante_m));
    const l = Math.max(ergebnis.laenge_m, 1);
    const h = Math.max(o - g, 0.5);
    const pad = Math.max(l, h) * 0.06;
    return { g, o, l, h, pad };
  });
  const xy = (s: number, h: number) => `${s},${-h}`;
  const z = (n: number | null | undefined) =>
    n == null ? "—" : n.toLocaleString("de-DE", { maximumFractionDigits: 2 });
</script>

{#if profil.length >= 2}<figure>
    <svg
      role="img"
      aria-label={`Wandabwicklung ${ergebnis.bezeichnung}`}
      viewBox={`${-box.pad} ${-box.o - box.pad} ${box.l + 2 * box.pad} ${box.h + 2 * box.pad}`}
    >
      <polygon
        points={[
          ...profil.map((p) => xy(p.station_m, p.oberkante_m)),
          ...[...profil].reverse().map((p) => xy(p.station_m, p.gelaende_m)),
        ].join(" ")}
        fill="#367bb722"
        stroke="none"
      />
      <polyline
        points={profil.map((p) => xy(p.station_m, p.oberkante_m)).join(" ")}
        fill="none"
        stroke="currentColor"
        vector-effect="non-scaling-stroke"
      />
      <polyline
        points={profil.map((p) => xy(p.station_m, p.gelaende_m)).join(" ")}
        fill="none"
        stroke="#8c5a1a"
        stroke-width="2"
        vector-effect="non-scaling-stroke"
      />
      {#each (ergebnis.wandteile ?? []).slice(1) as t}<line
          x1={t.von_m}
          x2={t.von_m}
          y1={-box.g}
          y2={-box.o}
          stroke="currentColor"
          stroke-dasharray="4 3"
          vector-effect="non-scaling-stroke"
        />{/each}
    </svg>
    <figcaption>
      Abwicklung in Wandrichtung (Anfang links): Gelände braun, Wandoberkante
      schwarz, Wandteilgrenzen gestrichelt.
    </figcaption>
  </figure>
  <table class="ergebnistabelle">
    <thead
      ><tr
        ><th>Wandteil</th><th>Station (m)</th><th>H an den Kanten (m)</th><th>Maßgebliche Wandhöhe (m)</th><th
          >H mit Dach (m)</th
        ><th>Tiefe (m)</th></tr
      ></thead
    >
    <tbody
      >{#each ergebnis.wandteile ?? [] as t, i}<tr
          ><td>{i + 1}</td><td>{z(t.von_m)} – {z(t.bis_m)}</td><td>{z(t.h_anfang_m)} / {z(t.h_ende_m)}</td><td
            >{z(t.wandhoehe_m)}</td
          ><td>{z(t.h_m)}</td><td>{z(t.tiefe_m)}</td></tr
        >{/each}</tbody
    >
  </table>{/if}

<style>
  svg {
    width: 100%;
    height: 180px;
    background: var(--panel, #f8fafc);
    border: 1px solid var(--border, #ccd5df);
  }
  figure {
    margin: 0.5rem 0;
  }
  figcaption {
    font-size: 0.85rem;
  }
</style>
