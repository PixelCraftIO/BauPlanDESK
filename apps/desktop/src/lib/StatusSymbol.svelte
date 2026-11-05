<script lang="ts" module>
  import type { Bereichszustand } from "./generated/plattform";
  /**
   * Statusschema des Projektbaums, nach Dringlichkeit geordnet. Grün gibt es
   * nur mit fachlichem Prüfsignal, nie allein, weil ein Wert eingetragen ist.
   */
  export type Statusart = "rot" | "blau" | "orange" | "gelb" | "grau" | "gruen";
  export const STATUSWORT: Record<Statusart, string> = {
    rot: "Problem",
    blau: "Nicht gespeichert",
    orange: "Veraltet",
    gelb: "In Arbeit",
    grau: "Zu erledigen",
    gruen: "Geprüft",
  };
  /** Erklärung je Zustand für Legende und Hilfe. */
  export const STATUSERKLAERUNG: Record<Statusart, string> = {
    rot: "Anforderung nicht eingehalten, Prüffehler oder Frist überschritten.",
    blau: "Änderungen in diesem Bereich sind noch nicht gespeichert.",
    orange: "Eine Grundlage hat sich geändert: Nachweis neu erstellen bzw. neu prüfen.",
    gelb: "Begonnen, aber Angaben, Messwerte, Prüfung oder Freigabe fehlen.",
    grau: "Wird benötigt, es ist aber noch nichts erfasst.",
    gruen: "Vollständig, aktuell und fachlich geprüft bzw. freigegeben.",
  };
  /** Dringlichkeit: das höchste Symbol eines Zweigs gewinnt. */
  export const RANG: Record<Statusart, number> = {
    rot: 6,
    blau: 5,
    orange: 4,
    gelb: 3,
    grau: 2,
    gruen: 1,
  };
  export const REIHENFOLGE = (Object.keys(RANG) as Statusart[]).sort(
    (a, b) => RANG[b] - RANG[a],
  );
  /** Symbol zu einem Bereichszustand des Core; ohne Symbol: `null`. */
  export function statusartVon(z: Bereichszustand | undefined): Statusart | null {
    switch (z) {
      case "problem":
        return "rot";
      case "veraltet":
        return "orange";
      case "in_arbeit":
        return "gelb";
      case "zu_erledigen":
        return "grau";
      case "geprueft":
        return "gruen";
      default:
        return null;
    }
  }
</script>

<script lang="ts">
  let { art, text = "" }: { art: Statusart; text?: string } = $props();
  const titel = $derived(text ? `${STATUSWORT[art]}:\n${text}` : STATUSWORT[art]);
</script>

<!-- Die Form unterscheidet die Zustände auch ohne Farbe: Kreuz, Punkt, Pfeil,
     halber Kreis, leerer Ring, Haken. -->
<span class="statussymbol {art}" role="img" aria-label={titel.replace(/\n/g, " ")} title={titel}>
  <svg viewBox="0 0 16 16" aria-hidden="true">
    {#if art === "rot"}
      <circle cx="8" cy="8" r="7.2" class="voll" />
      <path d="M5.3 5.3l5.4 5.4M10.7 5.3l-5.4 5.4" class="strich" />
    {:else if art === "blau"}
      <circle cx="8" cy="8" r="6.4" class="ring" />
      <circle cx="8" cy="8" r="3.6" class="voll" />
    {:else if art === "orange"}
      <circle cx="8" cy="8" r="7.2" class="voll" />
      <path d="M11.4 8.6a3.5 3.5 0 1 1-1.1-3.2" class="strich" />
      <path d="M10.9 3.3v2.6H8.3" class="strich" />
    {:else if art === "gelb"}
      <circle cx="8" cy="8" r="6.4" class="ring" />
      <path d="M8 1.6a6.4 6.4 0 0 1 0 12.8z" class="voll" />
    {:else if art === "grau"}
      <circle cx="8" cy="8" r="6.2" class="ring" />
    {:else}
      <circle cx="8" cy="8" r="7.2" class="voll" />
      <path d="M4.6 8.3l2.3 2.3 4.6-4.9" class="strich" />
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
  .blau {
    --farbe: #1f63b8;
  }
  .orange {
    --farbe: #c8580a;
  }
  .gelb {
    --farbe: #c99000;
  }
  .grau {
    --farbe: #737a84;
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
  .strich {
    fill: none;
    stroke: #fff;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
