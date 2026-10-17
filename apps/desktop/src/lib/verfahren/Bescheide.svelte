<script lang="ts">
  import type { Bescheid } from "../generated/plattform";
  import type { DocumentVersion, Benutzer } from "../api";
  import Dokumentfeld from "./Dokumentfeld.svelte";
  import Dokumentliste from "./Dokumentliste.svelte";
  import Fristfeld from "./Fristfeld.svelte";
  import Zustaendigkeit from "./Zustaendigkeit.svelte";
  import { heute } from "./api";
  let {
    eintraege = $bindable(),
    dateien,
    benutzer,
    abgegeben,
    alt = [],
    ohneZustellung = [],
  }: {
    eintraege: Bescheid[];
    dateien: DocumentVersion[];
    /** Dokumentversionen aus versendeten oder eingegangenen Abgabeständen. */
    abgegeben: DocumentVersion[];
    benutzer: Benutzer[];
    alt?: string[];
    /** Gespeicherte Bescheide, deren Zustelldatum noch einmal nachgetragen werden darf. */
    ohneZustellung?: string[];
  } = $props();
  let original = $state<import("../generated/plattform").Dokumentbezug | null>(
    null,
  );
  function neu() {
    if (!original) return;
    eintraege = [
      ...eintraege,
      {
        id: crypto.randomUUID(),
        entscheidung: "genehmigt",
        datum: heute(),
        zugestellt: null,
        original: structuredClone($state.snapshot(original)),
        genehmigte_unterlagen: [],
        auflagen: [],
        ersetzt: null,
      },
    ];
  }
</script>

<h3>Bescheide und genehmigte Unterlagen</h3>
<Dokumentfeld
  {dateien}
  bind:wert={original}
  label="Neuer Bescheid – Original"
/><button type="button" class="sekundaer" disabled={!original} onclick={neu}
  >Bescheid erfassen</button
>
{#each eintraege as b (b.id)}{@const fest = alt.includes(b.id)}<details data-uebersicht-eintrag={b.id} open>
    <summary
      >{b.datum} · {b.entscheidung}{eintraege.some((n) => n.ersetzt === b.id)
        ? " · durch Folgebescheid ersetzt"
        : ""}</summary
    >
    <div class="raster">
      <label
        >Entscheidung<select disabled={fest} bind:value={b.entscheidung}
            ><option value="genehmigt">Genehmigt</option><option
              value="teilweise_genehmigt">Teilweise genehmigt</option
            ><option value="abgelehnt">Abgelehnt</option><option
              value="sonstiges">Sonstige Entscheidung</option
            ></select
          ></label
        ><label
          >Bescheiddatum<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Bescheiddatum"
            type="date"
            disabled={fest}
            required
            bind:value={b.datum}
          /></label
        ><label
          >Zustelldatum<input
            type="date"
            disabled={fest && !ohneZustellung.includes(b.id)}
            value={b.zugestellt ?? ""}
            onchange={(e) => (b.zugestellt = e.currentTarget.value || null)}
          /></label
        ><label
          >Ersetzt früheren Bescheid<select
            disabled={fest}
            bind:value={b.ersetzt}
            ><option value={null}>Keinen</option
            >{#each eintraege.filter((n) => n.id !== b.id) as n}<option
                value={n.id}>{n.datum} · {n.entscheidung}</option
              >{/each}</select
          ></label
        >
      </div>
    <fieldset disabled={fest}>
      {#if abgegeben.length || b.genehmigte_unterlagen.length}<Dokumentliste
          dateien={fest ? dateien : abgegeben}
          bind:werte={b.genehmigte_unterlagen}
          label="Ausdrücklich genehmigte Unterlagen"
        />{:else}<p class="hinweis">
          Genehmigte Unterlagen stammen aus einem versendeten Abgabestand.
          Versand im Register Abgaben erfassen.
        </p>{/if}
    </fieldset>
    <h4>Nebenbestimmungen / offene Pflichten</h4>
    <button
      type="button"
      class="sekundaer"
      onclick={() =>
        b.auflagen.push({
          id: crypto.randomUUID(),
          titel: "",
          ausloeser: "",
          verantwortlich: null,
          frist: null,
          erledigt: false,
          nachweise: [],
        })}>Auflage hinzufügen</button
    >{#each b.auflagen as p (p.id)}<fieldset data-uebersicht-eintrag={p.id}>
        <legend>{p.titel || "Neue Auflage"}</legend>
        <div class="raster">
          <label
            >Auflage<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Auflage"
              required
              maxlength="500"
              bind:value={p.titel}
            /></label
          ><label
            >Fälligkeit / Auslöser<input
              maxlength="4000"
              bind:value={p.ausloeser}
            /></label
          ><Zustaendigkeit {benutzer} bind:wert={p.verantwortlich} />
        </div>
        <Fristfeld {dateien} bind:wert={p.frist} /><Dokumentliste
          {dateien}
          bind:werte={p.nachweise}
          label="Erfüllungsbelege"
        /><label
          ><input type="checkbox" bind:checked={p.erledigt} />Erfüllt (Beleg
          erforderlich)</label
        >
      </fieldset>{/each}
  </details>{/each}
