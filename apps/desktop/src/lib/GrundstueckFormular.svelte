<script lang="ts">
  import { regelwerk } from "./baunvo";
  import type { FlaechenAufstellung, GrundstueckErgebnis } from "./api";
  let { aufstellung, basisFallback, ergebnis, mitPlan, flurstuecksBasis = null }: { flurstuecksBasis?: { basis: number; abzug: number } | null; aufstellung: FlaechenAufstellung; basisFallback: number; ergebnis: GrundstueckErgebnis | null; mitPlan: boolean } = $props();
  const regel = $derived(regelwerk(aufstellung.bau_nvo));
  function aktivieren() { aufstellung.grundstueck = { basisflaeche_m2: basisFallback, nicht_massgebend_m2: 0, abzug_begruendung: "", gemeinschaftsanteile: [] }; }
  const z = (n: number) => n.toLocaleString("de-DE", { maximumFractionDigits: 3 });
</script>
<section class="flaechenformular">
  <h3>Baugrundstück und zugeordnete Flächen</h3>
  {#if !aufstellung.grundstueck}
    <p class="meta">Gespeicherte maßgebende Grundstücksfläche: {z(basisFallback)} m²</p><button type="button" class="sekundaer" onclick={aktivieren}>Grundstücksflächen aufschlüsseln</button>
  {:else}
    {#if flurstuecksBasis}<p class="hinweisbox">Aus den Flurstücken: {z(flurstuecksBasis.basis)} m² − {z(flurstuecksBasis.abzug)} m² Abzüge = {z(flurstuecksBasis.basis-flurstuecksBasis.abzug)} m² Bauland.</p>
    {:else}<div class="raster">
      <label>Fläche des eigenen Baugrundstücks (m²)<input type="number" min="0.01" step="any" bind:value={aufstellung.grundstueck.basisflaeche_m2} required /></label>
      <label>Davon nicht maßgebende Fläche (m²)<input type="number" min="0" step="any" bind:value={aufstellung.grundstueck.nicht_massgebend_m2} required /></label>
      {#if aufstellung.grundstueck.nicht_massgebend_m2 > 0}<label>Grund für den Abzug<input bind:value={aufstellung.grundstueck.abzug_begruendung} required placeholder="z. B. Fläche außerhalb des maßgebenden Baulands" /></label>{/if}
    </div>{/if}
    <p class="meta">Außerhalb dieses Baugrundstücks gelegene Gemeinschaftsanlagen im selben Plangebiet können mit dem durch den Bebauungsplan zugeordneten Anteil hinzugerechnet werden. Bereits in der Grundstücksfläche enthaltene Flächen nicht erneut erfassen.</p>
    {#each aufstellung.grundstueck.gemeinschaftsanteile as a, i}
      <fieldset><legend>Zugeordnete Gemeinschaftsfläche {i + 1}</legend>
        <div class="raster">
          <label>Anlage / Flurstück<input bind:value={a.bezeichnung} required maxlength="500" placeholder="z. B. Gemeinschaftsspielplatz, Flurstück 123/5" /></label>
          <label>Gesamtfläche der Anlage (m²)<input type="number" min="0" step="any" bind:value={a.gesamtflaeche_m2} required /></label>
          <label>Zugeordneter Anteil laut Plan (%)<input type="number" min="0" max="100" step="any" bind:value={a.anteil_prozent} required /></label>
          <label>Planfestsetzung / Zuordnung<textarea bind:value={a.festsetzung} required maxlength="4000"></textarea></label>
        </div>
        {#if ergebnis}<p class="meta">Hinzugerechneter Anteil: {z(ergebnis.anteile_m2[i])} m²</p>{/if}
        <button type="button" class="sekundaer" onclick={() => aufstellung.grundstueck!.gemeinschaftsanteile.splice(i, 1)}>Zuordnung entfernen</button>
      </fieldset>
    {/each}
    {#if regel.gemeinschaftsanteile && mitPlan}<button type="button" class="sekundaer" onclick={() => aufstellung.grundstueck!.gemeinschaftsanteile.push({ bezeichnung: "", gesamtflaeche_m2: 0, anteil_prozent: 100, festsetzung: "", im_plangebiet: true, ausserhalb_baugrundstueck: true })}>Zugeordnete Gemeinschaftsfläche hinzufügen</button>
    {:else}<p class="meta">{!mitPlan ? "Eine Zurechnung benötigt einen Bebauungsplan." : "Eine Zurechnung ist für diese BauNVO-Fassung nicht vorkonfiguriert."}</p>{/if}
    {#if ergebnis}<p class="hinweisbox">{z(ergebnis.basis_m2)} m² Grundstück − {z(ergebnis.abzug_m2)} m² Abzüge + {z(ergebnis.zuschlag_m2)} m² zugeordnete Flächen = <strong>{z(ergebnis.massgebend_m2)} m² maßgebende Bezugsfläche</strong> für GRZ, GFZ und BMZ.</p>{/if}
  {/if}
</section>
