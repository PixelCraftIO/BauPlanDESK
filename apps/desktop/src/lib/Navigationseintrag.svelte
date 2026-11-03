<script lang="ts">
  import { tick } from "svelte";
  import type { ModulId } from "./module";
  let {
    id,
    titel,
    kurz,
    grund,
    entwurf,
    aktiv,
    favorit,
    onoeffnen,
    onfavorit,
  }: {
    id: ModulId;
    titel: string;
    kurz: string;
    grund: string | null;
    entwurf: boolean;
    aktiv: boolean;
    favorit: boolean;
    onoeffnen: (id: ModulId) => void;
    onfavorit: () => void;
  } = $props();
  const uid = $props.id();
  let offen = $state(false),
    x = $state(0),
    y = $state(0),
    unterdrueckt = $state(false);
  let tooltip = $state<HTMLDivElement>();
  function zeigen(el: HTMLElement) {
    const r = el.getBoundingClientRect();
    x = Math.max(8, Math.min(r.right + 8, window.innerWidth - 360));
    y = Math.max(8, Math.min(r.top, window.innerHeight - 230));
    unterdrueckt = false;
    offen = true;
    void tick().then(() => {
      if (!offen || !tooltip) return;
      const z =
        Number.parseFloat(getComputedStyle(document.documentElement).zoom) || 1;
      const b = tooltip.getBoundingClientRect();
      x = Math.max(8, Math.min(r.right + 8, innerWidth - b.width - 8)) / z;
      y = Math.max(8, Math.min(r.top, innerHeight - b.height - 8)) / z;
    });
  }
</script>

<div class="navigationseintrag" class:gesperrt={!!grund}>
  {#if grund}
    <div
      class="sperrhinweis"
      role="group"
      aria-label={`${titel}: Verfügbarkeit`}
      onmouseenter={(e) => zeigen(e.currentTarget)}
      onmouseleave={() => (offen = false)}
    >
      <button disabled aria-label={titel} title={grund}
        >{kurz} <span aria-hidden="true">🔒</span></button
      >
      <button
        class="info link"
        onkeydown={(e) => {
          if (e.key === "Escape") {
            offen = false;
            unterdrueckt = true;
          }
        }}
        aria-label={`${titel}: Voraussetzungen anzeigen`}
        aria-describedby={uid}
        onclick={(e) => zeigen(e.currentTarget)}
        onfocus={(e) => zeigen(e.currentTarget)}
        onblur={() => (offen = false)}>ⓘ</button
      >
      <div
        class="sperrtext"
        bind:this={tooltip}
        hidden={!offen || unterdrueckt}
        style:left={`${x}px`}
        style:top={`${y}px`}
        role="tooltip"
        id={uid}
      >
        <strong>Was noch zu erledigen ist</strong>
        <p>{grund}</p>
      </div>
    </div>
    <button
      type="button"
      class="entwurfsweg link"
      onclick={() => onoeffnen(entwurf ? id : "rechtsgrundlagen")}
      aria-label={`${titel}: ${entwurf ? "Entwurf öffnen" : "Rechtsgrundlagen öffnen"}`}
      >{entwurf ? "Entwurf öffnen" : "Rechtsgrundlagen öffnen"} →</button
    >
  {:else}<button
      class:aktiv
      aria-current={aktiv ? "page" : undefined}
      aria-label={titel}
      title={titel}
      onclick={() => onoeffnen(id)}>{kurz}</button
    >{/if}
  <button
    class="favorit link"
    aria-label={`${titel}: ${favorit ? "Favorit entfernen" : "Als Favorit merken"}`}
    aria-pressed={favorit}
    onclick={onfavorit}>{favorit ? "★" : "☆"}</button
  >
</div>

<style>
  .navigationseintrag {
    position: relative;
    padding-right: 1.6rem;
  }
  .navigationseintrag > button:not(.favorit),
  .sperrhinweis > button:not(.info) {
    display: block;
    width: 100%;
    text-align: left;
    white-space: normal;
    overflow-wrap: anywhere;
    line-height: 1.35;
    padding-left: 18px;
  }
  .favorit {
    position: absolute;
    right: 0;
    top: 0.5rem;
    padding: 0.15rem;
  }
  /* Leerer Stern nur beim Überfahren oder per Tastatur; gemerkte Einträge
     zeigen den gefüllten Stern dauerhaft. Bei Touch-Bedienung bleibt er da. */
  @media (hover: hover) {
    .favorit[aria-pressed="false"] {
      opacity: 0;
    }
    .navigationseintrag:hover > .favorit,
    .navigationseintrag:focus-within > .favorit,
    .favorit:focus-visible {
      opacity: 1;
    }
  }
  .info {
    position: absolute;
    right: 0;
    top: 0.5rem;
    padding: 0.15rem;
  }
  .sperrhinweis {
    position: relative;
    padding-right: 1.4rem;
  }
  .sperrtext {
    position: fixed;
    width: min(22rem, 80vw);
    padding: 1rem;
    background: var(--panel);
    color: var(--fg);
    border: 1px solid var(--border);
    box-shadow: 0 4px 16px #0003;
    z-index: 100;
  }
  .sperrtext[hidden] {
    display: none;
  }
  .entwurfsweg {
    margin: 0 0 0.4rem 1rem;
    font-size: 0.8rem;
  }
</style>
