<script lang="ts">
  /**
   * Begründete Berichtigung eines gespeicherten Werts. Das Feld bleibt
   * gesperrt, bis die Berichtigung geöffnet wird; der Core trägt alten und
   * neuen Wert beim Speichern in die Verfahrenschronik ein.
   */
  let {
    bezeichnung,
    offen = $bindable(false),
    onvormerken,
  }: {
    bezeichnung: string;
    offen?: boolean;
    onvormerken: (grund: string) => void;
  } = $props();
  let grund = $state(""),
    vorgemerkt = $state(false);
</script>

{#if !offen}<button
    type="button"
    class="link"
    onclick={() => {
      offen = true;
      vorgemerkt = false;
    }}>{bezeichnung} berichtigen</button
  >{:else}<div class="raster">
    <label
      >Grund der Berichtigung<span class="pflicht" aria-hidden="true"> *</span
      ><input
        aria-label="Grund der Berichtigung"
        maxlength="4000"
        bind:value={grund}
      /></label
    ><button
      type="button"
      class="sekundaer"
      disabled={!grund.trim()}
      onclick={() => {
        onvormerken(grund.trim());
        vorgemerkt = true;
      }}>Berichtigung vormerken</button
    >
  </div>
  {#if vorgemerkt}<p class="meta" role="status">
      Berichtigung vorgemerkt. Beim Speichern trägt die Verfahrenschronik alten
      und neuen Wert mit dem Grund ein.
    </p>{/if}{/if}
