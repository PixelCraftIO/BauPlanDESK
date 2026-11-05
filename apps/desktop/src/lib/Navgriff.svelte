<script lang="ts">
  import {
    navbreite,
    navbreiteSetzen,
    NAVBREITE_MAX,
    NAVBREITE_MIN,
    NAVBREITE_STANDARD,
  } from "./navbreite.svelte";

  // Griff am rechten Rand der Projektnavigation: Ziehen verstellt die Breite,
  // Pfeiltasten in 16-px-Schritten, Doppelklick oder Pos1 stellt sie zurück.
  let ziehen = $state<{ x: number; px: number; zoom: number } | null>(null);

  function start(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    const zoom =
      Number.parseFloat(getComputedStyle(document.documentElement).zoom) || 1;
    ziehen = { x: e.clientX, px: navbreite.px, zoom };
  }
  function bewegen(e: PointerEvent) {
    if (!ziehen) return;
    navbreiteSetzen(ziehen.px + (e.clientX - ziehen.x) / ziehen.zoom, false);
  }
  function ende() {
    if (!ziehen) return;
    ziehen = null;
    navbreiteSetzen(navbreite.px);
  }
  function taste(e: KeyboardEvent) {
    const schritt = e.shiftKey ? 64 : 16;
    if (e.key === "ArrowLeft") navbreiteSetzen(navbreite.px - schritt);
    else if (e.key === "ArrowRight") navbreiteSetzen(navbreite.px + schritt);
    else if (e.key === "Home") navbreiteSetzen(NAVBREITE_STANDARD);
    else return;
    e.preventDefault();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="navgriff"
  class:aktiv={!!ziehen}
  role="separator"
  aria-orientation="vertical"
  aria-label="Breite der Projektnavigation"
  aria-valuemin={NAVBREITE_MIN}
  aria-valuemax={NAVBREITE_MAX}
  aria-valuenow={navbreite.px}
  title="Breite ziehen · Doppelklick: Standardbreite"
  tabindex="0"
  onpointerdown={start}
  onpointermove={bewegen}
  onpointerup={ende}
  onpointercancel={ende}
  ondblclick={() => navbreiteSetzen(NAVBREITE_STANDARD)}
  onkeydown={taste}
></div>

<style>
  .navgriff {
    position: absolute;
    top: 0;
    bottom: 0;
    left: calc(var(--navspalte) - 3px);
    width: 7px;
    z-index: 5;
    cursor: col-resize;
    touch-action: none;
  }
  .navgriff::after {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: 3px;
    width: 1px;
    background: transparent;
    transition: background-color 0.12s;
  }
  .navgriff:hover::after,
  .navgriff:focus-visible::after,
  .navgriff.aktiv::after {
    left: 2px;
    width: 3px;
    background: var(--accent);
  }
  .navgriff:focus-visible {
    outline: none;
  }
  :global(body:has(.navgriff.aktiv)) {
    cursor: col-resize;
    user-select: none;
  }
  @media (max-width: 600px) {
    .navgriff {
      display: none;
    }
  }
</style>
