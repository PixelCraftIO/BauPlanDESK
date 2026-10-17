<script lang="ts">
  import type { Anforderung } from "../generated/plattform";
  import type { DocumentVersion, Benutzer } from "../api";
  import Dokumentliste from "./Dokumentliste.svelte";
  import Zustaendigkeit from "./Zustaendigkeit.svelte";
  let {
    eintraege = $bindable(),
    dateien,
    benutzer,
    darfFreigeben,
  }: {
    eintraege: Anforderung[];
    dateien: DocumentVersion[];
    benutzer: Benutzer[];
    darfFreigeben: boolean;
  } = $props();
</script>

<h3>Anforderungen und Vollständigkeit</h3>
<p class="meta">
  Eine Kategorie allein erfüllt keine Anforderung. Erforderlichkeit und genaue
  Unterlagenstände werden ausdrücklich zugeordnet und geprüft.
</p>
<button
  type="button"
  class="sekundaer"
  onclick={() =>
    eintraege.push({
      id: crypto.randomUUID(),
      titel: "",
      erforderlich: "ungeklaert",
      grund: "",
      quelle: "Manuell",
      dokumente: [],
      freigegeben: false,
      verantwortlich: null,
    })}>Anforderung ergänzen</button
>
{#each eintraege as a (a.id)}<details
    data-uebersicht-eintrag={a.id}
    open={!a.freigegeben}
  >
    <summary
      >{a.titel || "Neue Anforderung"} · {a.erforderlich} · {a.freigegeben
        ? "Freigegeben"
        : "Zu prüfen"}</summary
    >
    <div class="raster">
      <label
        >Anforderung<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Anforderung"
          required
          maxlength="500"
          bind:value={a.titel}
        /></label
      ><label
        >Erforderlichkeit<select
          bind:value={a.erforderlich}
          onchange={() => (a.freigegeben = false)}
          ><option value="ungeklaert">Noch zu klären</option><option
            value="erforderlich">Erforderlich</option
          ><option value="nicht_erforderlich">Nicht erforderlich</option
          ></select
        ></label
      ><Zustaendigkeit {benutzer} bind:wert={a.verantwortlich} />
    </div>
    <label
      >Quelle / Katalogstand<input
        maxlength="4000"
        bind:value={a.quelle}
      /></label
    ><label
      >Begründung<textarea rows="3" maxlength="4000" bind:value={a.grund}
      ></textarea></label
    ><Dokumentliste {dateien} bind:werte={a.dokumente} /><label
      ><input
        type="checkbox"
        disabled={!darfFreigeben}
        bind:checked={a.freigegeben}
      />Fachlich freigegeben (Projektleitung)</label
    >
  </details>{/each}
