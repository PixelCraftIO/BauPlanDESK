<script lang="ts">
  import { projektNavigation } from "../projektNavigation.svelte";
  import type { Nachforderung, Verfahrensakte } from "../generated/plattform";
  import type { DocumentVersion, Benutzer } from "../api";
  import Dokumentliste from "./Dokumentliste.svelte";
  import Fristfeld from "./Fristfeld.svelte";
  import Zustaendigkeit from "./Zustaendigkeit.svelte";
  let {
    eintraege = $bindable(),
    akte,
    dateien,
    benutzer,
    alt = [],
  }: {
    eintraege: Nachforderung[];
    akte: Verfahrensakte;
    dateien: DocumentVersion[];
    benutzer: Benutzer[];
    /** Gespeicherte Forderungspunkte; ihr Wortlaut bleibt laut Core unverändert. */
    alt?: string[];
  } = $props();
  const navigation = projektNavigation();
  let original = $state("");
  function neu() {
    if (!original) return;
    eintraege = [
      ...eintraege,
      { id: crypto.randomUUID(), kommunikation: original, punkte: [] },
    ];
  }
  function punkt(n: Nachforderung) {
    n.punkte.push({
      id: crypto.randomUUID(),
      titel: "",
      grundlage: "",
      verantwortlich: null,
      frist: null,
      status: "offen",
      antwort: "",
      dokumente: [],
      abgaben: [],
      bestaetigung: null,
    });
  }
</script>

<h3>Nachforderungen und Antworten</h3>
<label
  >Eingegangenes Schreiben / Gesprächsnotiz<select bind:value={original}
    ><option value="">Kommunikation auswählen</option
    >{#each akte.kommunikation.filter((c) => c.eingehend) as c}<option
        value={c.id}>{c.datum} · {c.betreff}</option
      >{/each}</select
  ></label
><button type="button" class="sekundaer" disabled={!original} onclick={neu}
  >Nachforderung anlegen</button
>
{#each eintraege as n (n.id)}{@const kommunikation = akte.kommunikation.find(
    (c) => c.id === n.kommunikation,
  )}
  <section>
    <h4>
      {akte.kommunikation.find((c) => c.id === n.kommunikation)?.betreff ??
        "Nachforderung"}
    </h4>
    {#if kommunikation}<p class="meta">
        Originaleingang: {kommunikation.datum} · {kommunikation.betreff}
      </p>
      {#if kommunikation.original}<button
          type="button"
          class="link"
          onclick={() => {
            const d = kommunikation.original;
            if (d)
              navigation?.springen({
                bereich: "dokumente",
                objekt: d.dokument_id,
                register: "historie",
                eintrag: String(d.version),
              });
          }}>Originalschreiben in genau dieser Version ansehen →</button
        >{/if}{/if}
    <p>
      {n.punkte.filter((p) => ["bestaetigt", "entfallen"].includes(p.status))
        .length} von {n.punkte.length} Punkten abgeschlossen
    </p>
    <button type="button" class="sekundaer" onclick={() => punkt(n)}
      >Forderungspunkt hinzufügen</button
    >
    {#each n.punkte as p (p.id)}<details
        data-uebersicht-eintrag={p.id}
        open={p.status !== "bestaetigt"}
      >
        <summary>{p.titel || "Neuer Punkt"} · {p.status}</summary>
        {#if alt.includes(p.id)}<p class="meta">
            Wortlaut und Grundlage aus dem behördlichen Schreiben bleiben nach
            dem Speichern unverändert. Ein abweichender Wortlaut ist ein neuer
            Punkt; der bisherige wird begründet als entfallen geführt.
          </p>{/if}
        <div class="raster">
          <label
            >Forderung<span class="pflicht" aria-hidden="true"> *</span><input
              aria-label="Forderung"
              required
              maxlength="500"
              readonly={alt.includes(p.id)}
              bind:value={p.titel}
            /></label
          ><label
            >Grundlage / Fundstelle<input
              maxlength="4000"
              readonly={alt.includes(p.id)}
              bind:value={p.grundlage}
            /></label
          ><Zustaendigkeit {benutzer} bind:wert={p.verantwortlich} /><label
            >Bearbeitungsstand<select bind:value={p.status}
              ><option value="offen">Offen</option><option value="in_arbeit"
                >In Arbeit</option
              ><option value="vorbereitet">Antwort vorbereitet</option><option
                value="eingereicht">Antwort eingereicht</option
              ><option value="bestaetigt">Behördlich bestätigt</option><option
                value="entfallen">Entfallen (Begründung erforderlich)</option
              ></select
            ></label
          >
        </div>
        <Fristfeld {dateien} bind:wert={p.frist} /><label
          >Antwort / Begründung<textarea
            rows="4"
            maxlength="16000"
            bind:value={p.antwort}
          ></textarea></label
        ><Dokumentliste
          {dateien}
          bind:werte={p.dokumente}
          label="Antwortunterlagen"
        />
        <fieldset>
          <legend>Antwort-Abgabestände</legend>{#each akte.abgaben as b}<label
              ><input
                type="checkbox"
                checked={p.abgaben.includes(b.id)}
                onchange={(e) =>
                  (p.abgaben = e.currentTarget.checked
                    ? [...p.abgaben, b.id]
                    : p.abgaben.filter((id) => id !== b.id))}
              />{b.titel}</label
            >{/each}
        </fieldset>
        <label
          >Behördliche Bestätigung<select bind:value={p.bestaetigung}
            ><option value={null}>Noch keine Rückmeldung</option
            >{#each akte.kommunikation.filter((c) => c.eingehend) as c}<option
                value={c.id}>{c.datum} · {c.betreff}</option
              >{/each}</select
          ></label
        >
      </details>{/each}
  </section>{/each}
