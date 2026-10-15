<script lang="ts">
  import type { Kommunikation, Verfahrensakte } from "../generated/plattform";
  import type { DocumentVersion } from "../api";
  import Dokumentfeld from "./Dokumentfeld.svelte";
  import Bezugwahl from "./Bezugwahl.svelte";
  import { heute } from "./api";
  let {
    eintraege = $bindable(),
    akte,
    dateien,
    alt = [],
  }: {
    eintraege: Kommunikation[];
    akte: Verfahrensakte;
    dateien: DocumentVersion[];
    alt?: string[];
  } = $props();
</script>

<h3>Kommunikationsakte</h3>
<p class="meta">
  Originale bleiben nach dem Speichern unverändert. Korrekturen als Folgeeintrag
  erfassen und mit dem ursprünglichen Eintrag verknüpfen.
</p>
<button
  type="button"
  class="sekundaer"
  onclick={() =>
    (eintraege = [
      ...eintraege,
      {
        id: crypto.randomUUID(),
        art: "brief",
        eingehend: true,
        datum: heute(),
        betreff: "",
        beteiligte: "",
        inhalt: "",
        original: null,
        bezuege: [],
      },
    ])}>Kommunikation erfassen</button
>
{#each eintraege as c (c.id)}<details open={!alt.includes(c.id)}>
    <summary>{c.datum} · {c.betreff || "Neue Kommunikation"}</summary>
    <fieldset disabled={alt.includes(c.id)}>
      <div class="raster">
        <label
          >Betreff<input
            required
            maxlength="500"
            bind:value={c.betreff}
          /></label
        ><label
          >Art<select bind:value={c.art}
            ><option value="email">E-Mail</option><option value="brief"
              >Brief</option
            ><option value="portal">Portalnachricht</option><option
              value="telefon">Telefonat</option
            ><option value="besprechung">Besprechung</option></select
          ></label
        ><label
          >Ereignisdatum<input
            type="date"
            required
            bind:value={c.datum}
          /></label
        ><label
          >Richtung<select bind:value={c.eingehend}
            ><option value={true}>Eingehend</option><option value={false}
              >Ausgehend</option
            ></select
          ></label
        ><label
          >Beteiligte<input bind:value={c.beteiligte} maxlength="4000" /></label
        ><Dokumentfeld
          {dateien}
          bind:wert={c.original}
          label="Original / Gesprächsbeleg"
        />
      </div>
      <label
        >Inhalt / Gesprächsnotiz<textarea
          rows="5"
          maxlength="100000"
          bind:value={c.inhalt}
        ></textarea></label
      >
    </fieldset>
    <Bezugwahl {akte} bind:werte={c.bezuege} />
  </details>{/each}
