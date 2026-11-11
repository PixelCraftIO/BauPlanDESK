<script lang="ts">
  import { regelwerk } from "./baunvo";
  import type { FlaechenAufstellung, GrundstueckErgebnis } from "./api";
  let {
    aufstellung,
    basisFallback,
    ergebnis,
    mitPlan,
    flurstuecksBasis = null,
  }: {
    flurstuecksBasis?: { basis: number; abzug: number } | null;
    aufstellung: FlaechenAufstellung;
    basisFallback: number;
    ergebnis: GrundstueckErgebnis | null;
    mitPlan: boolean;
  } = $props();
  const regel = $derived(regelwerk(aufstellung.bau_nvo));
  // `|| 0` verhindert „-0“ (z. B. bei 0 − 0 Gemeinschaftsanteilen).
  const z = (n: number) =>
    (Math.round(n * 1000) / 1000 || 0).toLocaleString("de-DE", {
      maximumFractionDigits: 3,
    });
  function aktivieren() {
    aufstellung.grundstueck = {
      basisflaeche_m2: basisFallback,
      nicht_massgebend_m2: 0,
      abzug_begruendung: "",
      gemeinschaftsanteile: [],
    };
  }
</script>

<h3>Maßgebende Grundstücksfläche</h3>
{#if !aufstellung.grundstueck}<button class="sekundaer" onclick={aktivieren}
    >Flächenansatz anlegen</button
  >{:else}
  {#if !flurstuecksBasis}<div class="raster">
      <label
        >Eigentumsfläche m²<input
          type="number"
          min="0.01"
          step="any"
          bind:value={aufstellung.grundstueck.basisflaeche_m2}
        /></label
      ><label
        >Abzugsfläche m²<input
          type="number"
          min="0"
          step="any"
          bind:value={aufstellung.grundstueck.nicht_massgebend_m2}
        /></label
      ><label
        >Abzugsgrundlage<input
          bind:value={aufstellung.grundstueck.abzug_begruendung}
        /></label
      >
    </div>{/if}
  <table class="ergebnistabelle">
    <thead><tr><th>Rechenschritt</th><th>Fläche</th></tr></thead><tbody
      ><tr
        ><td>Eigentumsfläche der zugeordneten Flurstücke</td><td
          >{z(
            flurstuecksBasis?.basis ?? aufstellung.grundstueck.basisflaeche_m2,
          )} m²</td
        ></tr
      ><tr
        ><td>Abzüge</td><td
          >− {z(
            flurstuecksBasis?.abzug ??
              aufstellung.grundstueck.nicht_massgebend_m2,
          )} m²</td
        ></tr
      >{#if ergebnis}<tr
          ><td>Zugeordnete Gemeinschaftsanteile</td><td
            >+ {z(ergebnis.zuschlag_m2)} m²</td
          ></tr
        ><tr
          ><td><strong>Maßgebende Bezugsfläche</strong></td><td
            ><strong>{z(ergebnis.massgebend_m2)} m²</strong></td
          ></tr
        >{/if}</tbody
    >
  </table>
  {#if aufstellung.grundstueck.gemeinschaftsanteile.length}<h3>
      Zugeordnete Gemeinschaftsanlagen
    </h3>
    <div class="tabellen-scroll">
      <table class="ergebnistabelle">
        <thead
          ><tr
            ><th>Anlage</th><th>Gesamtfläche m²</th><th>Anteil %</th><th
              >Festsetzung / Zuordnung</th
            ><th></th></tr
          ></thead
        ><tbody
          >{#each aufstellung.grundstueck.gemeinschaftsanteile as a, i}<tr
              ><td
                ><input
                  aria-label="Gemeinschaftsanlage"
                  bind:value={a.bezeichnung}
                /></td
              ><td
                ><input
                  aria-label="Gemeinschaftsfläche"
                  type="number"
                  min="0"
                  step="any"
                  bind:value={a.gesamtflaeche_m2}
                /></td
              ><td
                ><input
                  aria-label="Zugeordneter Anteil"
                  type="number"
                  min="0"
                  max="100"
                  step="any"
                  bind:value={a.anteil_prozent}
                /></td
              ><td
                ><input
                  aria-label="Festsetzung zur Gemeinschaftsanlage"
                  bind:value={a.festsetzung}
                /></td
              ><td
                ><button
                  type="button"
                  class="link"
                  onclick={() =>
                    aufstellung.grundstueck!.gemeinschaftsanteile.splice(i, 1)}
                  >Entfernen</button
                ></td
              ></tr
            >{/each}</tbody
        >
      </table>
    </div>{/if}
  {#if mitPlan && regel.gemeinschaftsanteile}<button
      type="button"
      class="sekundaer"
      onclick={() =>
        aufstellung.grundstueck!.gemeinschaftsanteile.push({
          bezeichnung: "",
          gesamtflaeche_m2: 0,
          anteil_prozent: 100,
          festsetzung: "",
          im_plangebiet: true,
          ausserhalb_baugrundstueck: true,
        })}>Gemeinschaftsanteil hinzufügen</button
    >{/if}
  <details>
    <summary>Grundlage der Flächenzurechnung</summary>
    <p class="meta">
      Nur außerhalb dieses Baugrundstücks liegende Anlagen im selben Plangebiet
      mit ausdrücklicher Zuordnung erfassen. Bereits enthaltene Flächen werden
      nicht erneut hinzugerechnet.
    </p>
  </details>
{/if}

<style>
  table {
    margin: 1rem 0;
    width: 100%;
  }
  th {
    text-align: left;
  }
  input {
    min-width: 5rem;
    width: 100%;
  }
  details {
    margin: 1rem 0;
  }
</style>
