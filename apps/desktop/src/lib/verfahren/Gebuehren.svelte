<script lang="ts">
  import { datumDe } from "../api";
  import type { Gebuehrenbescheid, Kommunikation } from "../generated/plattform";
  import type { DocumentVersion, Benutzer } from "../api";
  import Fristfeld from "./Fristfeld.svelte";
  import Zustaendigkeit from "./Zustaendigkeit.svelte";
  import { heute } from "./api";
  let {
    eintraege = $bindable(),
    kommunikation,
    dateien,
    benutzer,
    alt = [],
  }: {
    eintraege: Gebuehrenbescheid[];
    kommunikation: Kommunikation[];
    dateien: DocumentVersion[];
    benutzer: Benutzer[];
    alt?: string[];
  } = $props();
  eintraege ??= [];
  const eingehend = $derived(kommunikation.filter((k) => k.eingehend));
  let schreiben = $state("");
  function neu() {
    const k = eingehend.find((k) => k.id === schreiben);
    if (!k) return;
    eintraege = [
      ...eintraege,
      {
        id: crypto.randomUUID(),
        kommunikation: k.id,
        datum: k.datum,
        betrag_cent: null,
        vorauszahlung: false,
        verwendungszweck: "",
        frist: null,
        bezahlt: null,
        verantwortlich: null,
      },
    ];
    schreiben = "";
  }
  const euro = (c: number | null) =>
    c === null ? "" : (c / 100).toFixed(2).replace(".", ",");
</script>

<h3>Gebührenbescheide</h3>
<p class="meta">
  Die Zahlungsfrist erscheint in den Wiedervorlagen, bis die Zahlung erfasst
  ist. Gebührenbescheide aus dem XBau-Postfach werden dort fachlich übernommen.
</p>
{#if eingehend.length}<div class="raster">
    <label
      >Gebührenbescheid (Schreiben)<select bind:value={schreiben}
        ><option value="">Kommunikation auswählen</option
        >{#each eingehend as k (k.id)}<option value={k.id}
            >{datumDe(k.datum)} · {k.betreff}</option
          >{/each}</select
      ></label
    >
  </div>
  <button type="button" class="sekundaer" disabled={!schreiben} onclick={neu}
    >Gebührenbescheid erfassen</button
  >{/if}
{#each eintraege as g (g.id)}<fieldset data-uebersicht-eintrag={g.id}>
    <legend
      >{g.vorauszahlung ? "Vorauszahlung" : "Gebühren"} vom {datumDe(g.datum)}{g.bezahlt
        ? ` · bezahlt am ${g.bezahlt}`
        : ""}</legend
    >
    {#if !alt.includes(g.id)}<button
        type="button"
        class="link"
        onclick={() => (eintraege = eintraege.filter((n) => n.id !== g.id))}
        >Noch nicht gespeicherten Eintrag verwerfen</button
      >{/if}
    <div class="raster">
      <label
        >Betrag in €<input
          inputmode="decimal"
          value={euro(g.betrag_cent)}
          onchange={(e) => {
            const v = e.currentTarget.value.trim().replace(",", ".");
            g.betrag_cent = v ? Math.round(Number(v) * 100) : null;
          }}
        /></label
      ><label
        >Verwendungszweck<input
          bind:value={g.verwendungszweck}
          maxlength="500"
        /></label
      ><label class="haken"
        ><input type="checkbox" bind:checked={g.vorauszahlung} />Vorauszahlung</label
      ><label
        >Bezahlt am<input
          type="date"
          max={heute()}
          value={g.bezahlt ?? ""}
          onchange={(e) => (g.bezahlt = e.currentTarget.value || null)}
        /></label
      ><Zustaendigkeit {benutzer} bind:wert={g.verantwortlich} />
    </div>
    <Fristfeld {dateien} bind:wert={g.frist} />
  </fieldset>{/each}
