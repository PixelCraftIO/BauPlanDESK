<script lang="ts">
  import type {
    Kommunikation,
    Ruecknahme,
    Verfahrensstatus,
    Vollstaendigkeit,
  } from "../generated/plattform";
  import { heute } from "./api";
  let {
    status,
    kommunikation,
    vollstaendigkeit = $bindable(),
    ruecknahme = $bindable(),
  }: {
    status: Verfahrensstatus;
    kommunikation: Kommunikation[];
    vollstaendigkeit?: Vollstaendigkeit;
    ruecknahme?: Ruecknahme;
  } = $props();
  const eingehend = $derived(kommunikation.filter((k) => k.eingehend));
</script>

{#if status === "vollstaendig" || vollstaendigkeit}<fieldset>
    <legend>Vollständigkeit</legend>
    {#if vollstaendigkeit}<div class="raster">
        <label
          >Bestätigt am<span class="pflicht" aria-hidden="true"> *</span><input
            aria-label="Vollständigkeit bestätigt am"
            type="date"
            required
            bind:value={vollstaendigkeit.datum}
          /></label
        ><label
          >Bestätigungsschreiben<select
            aria-label="Bestätigungsschreiben"
            bind:value={vollstaendigkeit.kommunikation}
            >{#each eingehend as k (k.id)}<option value={k.id}
                >{k.datum} · {k.betreff}</option
              >{/each}</select
          ></label
        >
      </div>
      <p class="meta">
        Behördliche Bearbeitungsfristen knüpfen je nach Landesrecht an dieses
        Datum an.
      </p>{:else if eingehend.length}<button
        type="button"
        class="sekundaer"
        onclick={() =>
          (vollstaendigkeit = {
            datum: heute(),
            kommunikation: eingehend[0].id,
          })}>Bestätigung der Behörde erfassen</button
      >{:else}<p class="meta">
        Zuerst das Bestätigungsschreiben der Behörde im Register Kommunikation
        erfassen.
      </p>{/if}
  </fieldset>{/if}
{#if status === "zurueckgenommen" || ruecknahme}<fieldset>
    <legend>Rücknahme</legend>
    {#if ruecknahme}<div class="raster">
        <label
          >Zurückgenommen am<span class="pflicht" aria-hidden="true"> *</span
          ><input
            aria-label="Zurückgenommen am"
            type="date"
            required
            bind:value={ruecknahme.datum}
          /></label
        ><label
          >Rücknahmeschreiben<select
            aria-label="Rücknahmeschreiben"
            bind:value={ruecknahme.kommunikation}
            >{#each kommunikation as k (k.id)}<option value={k.id}
                >{k.datum} · {k.betreff}</option
              >{/each}</select
          ></label
        >
      </div>{:else if kommunikation.length}<button
        type="button"
        class="sekundaer"
        onclick={() =>
          (ruecknahme = {
            datum: heute(),
            kommunikation: kommunikation[0].id,
          })}>Rücknahme erfassen</button
      >{:else}<p class="meta">
        Zuerst das Rücknahmeschreiben im Register Kommunikation erfassen.
      </p>{/if}
  </fieldset>{/if}
