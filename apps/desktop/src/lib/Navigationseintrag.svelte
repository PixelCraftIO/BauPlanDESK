<script lang="ts">
  import { tick } from "svelte";
  import type { ModulId } from "./module";
  import StatusSymbol, { type Statusart } from "./StatusSymbol.svelte";
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
    status = null,
    nichtBenoetigt = false,
    kennzeichenbar = false,
    onbenoetigt,
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
    /** Statussymbol am Eintrag (rot/gelb/grün) mit Begründung. */
    status?: { art: Statusart; text: string } | null;
    nichtBenoetigt?: boolean;
    /** Leerer Datenbereich: lässt sich als „nicht benötigt“ kennzeichnen. */
    kennzeichenbar?: boolean;
    onbenoetigt?: (benoetigt: boolean) => void;
  } = $props();
  let menue = $state(false);
  let eintrag = $state<HTMLDivElement>();
  // Das Aktionsmenü schließt bei Klick daneben und mit Escape, wo immer der Fokus steht.
  $effect(() => {
    if (!menue) return;
    const zu = (e: PointerEvent) => {
      if (!eintrag?.contains(e.target as Node)) menue = false;
    };
    const taste = (e: KeyboardEvent) => {
      if (e.key === "Escape") menue = false;
    };
    window.addEventListener("pointerdown", zu, true);
    window.addEventListener("keydown", taste);
    return () => {
      window.removeEventListener("pointerdown", zu, true);
      window.removeEventListener("keydown", taste);
    };
  });
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

<div class="navigationseintrag" bind:this={eintrag} class:gesperrt={!!grund} class:nichtbenoetigt={nichtBenoetigt}>
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
  <span class="rechts">
    {#if onbenoetigt && (kennzeichenbar || nichtBenoetigt)}<button
        type="button"
        class="mehr link"
        aria-label={`${titel}: Weitere Aktionen`}
        aria-haspopup="menu"
        aria-expanded={menue}
        onclick={() => (menue = !menue)}
        onkeydown={(e) => {
          if (e.key === "Escape") menue = false;
        }}>⋯</button
      >{/if}
    <button
      class="favorit link"
      aria-label={`${titel}: ${favorit ? "Favorit entfernen" : "Als Favorit merken"}`}
      aria-pressed={favorit}
      onclick={onfavorit}>{favorit ? "★" : "☆"}</button
    >
    {#if status}<StatusSymbol art={status.art} text={status.text} />{:else}<span
        class="statusplatz"
        aria-hidden="true"
      ></span>{/if}
  </span>
  {#if menue && onbenoetigt}<div class="eintragsmenue" role="menu" aria-label={`${titel}: Aktionen`}>
      <button
        type="button"
        role="menuitem"
        onclick={() => {
          menue = false;
          onbenoetigt(nichtBenoetigt);
        }}
        >{nichtBenoetigt
          ? "Wieder als benötigt führen"
          : "Als nicht benötigt kennzeichnen"}</button
      >
    </div>{/if}
</div>

<style>
  .navigationseintrag {
    position: relative;
  }
  .rechts {
    position: absolute;
    right: 2px;
    top: 4px;
    height: 24px;
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .statusplatz {
    width: 14px;
  }
  .mehr {
    padding: 0 2px;
    min-height: 0;
    line-height: 1;
  }
  .nichtbenoetigt > button:not(.favorit) {
    color: var(--muted);
    font-style: italic;
  }
  .eintragsmenue {
    position: absolute;
    right: 4px;
    top: 28px;
    z-index: 50;
    min-width: 14rem;
    background: var(--panel);
    border: 1px solid var(--border);
    box-shadow: 0 4px 16px #0003;
    padding: 4px 0;
  }
  .eintragsmenue button {
    display: block;
    width: 100%;
    text-align: left;
    padding: 5px 10px;
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
    padding-right: 52px;
  }
  .favorit {
    padding: 0 0.15rem;
    min-height: 0;
  }
  /* Leerer Stern nur beim Überfahren oder per Tastatur; gemerkte Einträge
     zeigen den gefüllten Stern dauerhaft. Bei Touch-Bedienung bleibt er da. */
  @media (hover: hover) {
    .favorit[aria-pressed="false"],
    .mehr[aria-expanded="false"] {
      opacity: 0;
    }
    .navigationseintrag:hover .favorit,
    .navigationseintrag:focus-within .favorit,
    .navigationseintrag:hover .mehr,
    .navigationseintrag:focus-within .mehr {
      opacity: 1;
    }
  }
  /* ⓘ steht links neben der Leiste mit Stern und Statussymbol (`.rechts`),
     sonst verdeckt die Leiste den Knopf. */
  .info {
    position: absolute;
    right: 54px;
    top: 4px;
    height: 24px;
    min-height: 0;
    padding: 0 0.15rem;
  }
  .sperrhinweis {
    position: relative;
  }
  .sperrhinweis > button:not(.info) {
    padding-right: calc(54px + 1.4rem);
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
