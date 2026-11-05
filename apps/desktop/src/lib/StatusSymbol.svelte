<script lang="ts" module>
  /** Rot: ungespeichert · Gelb: noch Arbeit offen · Grün: vollständig und gespeichert. */
  export type Statusart = "rot" | "gelb" | "gruen";
  export const STATUSWORT: Record<Statusart, string> = {
    rot: "Ungespeicherte Änderungen",
    gelb: "Noch zu erledigen",
    gruen: "Vollständig und gespeichert",
  };
</script>

<script lang="ts">
  let { art, text = "" }: { art: Statusart; text?: string } = $props();
  const titel = $derived(text ? `${STATUSWORT[art]}:\n${text}` : STATUSWORT[art]);
</script>

<!-- Form unterscheidet die Zustände auch ohne Farbe: Ausrufezeichen, halber Kreis, Haken. -->
<span class="statussymbol {art}" role="img" aria-label={titel.replace(/\n/g, " ")} title={titel}>
  <svg viewBox="0 0 16 16" aria-hidden="true">
    {#if art === "rot"}
      <circle cx="8" cy="8" r="7.2" class="voll" />
      <rect x="7" y="3.4" width="2" height="6.2" rx="1" class="zeichen" />
      <circle cx="8" cy="12" r="1.15" class="zeichen" />
    {:else if art === "gelb"}
      <circle cx="8" cy="8" r="6.4" class="ring" />
      <path d="M8 1.6a6.4 6.4 0 0 1 0 12.8z" class="voll" />
    {:else}
      <circle cx="8" cy="8" r="7.2" class="voll" />
      <path d="M4.6 8.3l2.3 2.3 4.6-4.9" class="haken" />
    {/if}
  </svg>
</span>

<style>
  .statussymbol {
    display: inline-flex;
    flex: none;
    width: 14px;
    height: 14px;
  }
  svg {
    width: 14px;
    height: 14px;
  }
  .rot {
    --farbe: #c0392b;
  }
  .gelb {
    --farbe: #d39200;
  }
  .gruen {
    --farbe: #2e7d45;
  }
  .voll {
    fill: var(--farbe);
  }
  .ring {
    fill: var(--panel, #fff);
    stroke: var(--farbe);
    stroke-width: 1.6;
  }
  .zeichen {
    fill: #fff;
  }
  .haken {
    fill: none;
    stroke: #fff;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
