<script lang="ts">
  import type {
    Bescheid,
    Bescheidfristen,
    Fristberechnung,
    Verfahrensart,
  } from "../generated/plattform";
  import type { DocumentVersion, Benutzer } from "../api";
  import Dokumentfeld from "./Dokumentfeld.svelte";
  import Dokumentliste from "./Dokumentliste.svelte";
  import Fristfeld from "./Fristfeld.svelte";
  import Zustaendigkeit from "./Zustaendigkeit.svelte";
  import { heute, verfahrenApi, NEBENBESTIMMUNG } from "./api";
  let {
    eintraege = $bindable(),
    dateien,
    benutzer,
    abgegeben,
    projektId,
    art,
    alt = [],
    auflagenAlt = [],
    ohneZustellung = [],
  }: {
    eintraege: Bescheid[];
    dateien: DocumentVersion[];
    /** Dokumentversionen aus versendeten Abgabeständen und externen Einreichungen. */
    abgegeben: DocumentVersion[];
    benutzer: Benutzer[];
    projektId: string;
    art: Verfahrensart;
    alt?: string[];
    /** Gespeicherte Nebenbestimmungen; ihr Wortlaut bleibt laut Core unverändert. */
    auflagenAlt?: string[];
    /** Gespeicherte Bescheide, deren Zustelldatum noch einmal nachgetragen werden darf. */
    ohneZustellung?: string[];
  } = $props();
  // Fristen berechnet der Core aus Zustellung, Entscheidung und Bundesland,
  // auch für noch nicht gespeicherte Angaben.
  let fristen = $state<Bescheidfristen[]>([]);
  $effect(() => {
    const auftrag = { art, bescheide: $state.snapshot(eintraege) as Bescheid[] };
    let aktuell = true;
    verfahrenApi.bescheidfristen(projektId, auftrag).then(
      (r) => aktuell && (fristen = r),
      () => aktuell && (fristen = []),
    );
    return () => (aktuell = false);
  });
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
{#snippet fristzeile(titel: string, w: Fristberechnung | null)}
  {#if w}<p>
      <strong>{titel}: bis {w.ende}</strong> · {w.grundlage}
    </p>
    <ul class="meta">
      {#each w.hinweise as h}<li>{h}</li>{/each}
    </ul>{/if}
{/snippet}
{#each eintraege as b (b.id)}{@const fest = alt.includes(b.id)}{@const f =
    fristen.find((f) => f.bescheid === b.id)}<details data-uebersicht-eintrag={b.id} open>
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
            max={heute()}
            bind:value={b.datum}
          /></label
        ><label
          >Zustelldatum<input
            type="date"
            max={heute()}
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
          Genehmigte Unterlagen stammen aus einem versendeten Abgabestand oder
          einer externen Einreichung. Beides im Register Abgabestände erfassen.
        </p>{/if}
    </fieldset>
    {#if f}<section aria-label="Berechnete Fristen">
        <h4>Berechnete Fristen</h4>
        {@render fristzeile("Rechtsbehelfsfrist", f.rechtsbehelfsfrist)}
        {@render fristzeile("Geltungsdauer der Genehmigung", f.geltungsdauer)}
        {#each f.hinweise as h}<p class="meta">{h}</p>{/each}
      </section>{/if}
    <h4>Nebenbestimmungen / offene Pflichten</h4>
    <button
      type="button"
      class="sekundaer"
      onclick={() =>
        b.auflagen.push({
          id: crypto.randomUUID(),
          art: "auflage",
          titel: "",
          ausloeser: "",
          verantwortlich: null,
          frist: null,
          erledigt: false,
          nachweise: [],
        })}>Nebenbestimmung hinzufügen</button
    >{#each b.auflagen as p (p.id)}<fieldset data-uebersicht-eintrag={p.id}>
        <legend
          >{NEBENBESTIMMUNG[p.art ?? "auflage"]}: {p.titel || "neu"}</legend
        >
        <div class="raster">
          <label
            >Art<select
              value={p.art ?? "auflage"}
              onchange={(e) =>
                (p.art = e.currentTarget.value as typeof p.art)}
              >{#each Object.entries(NEBENBESTIMMUNG) as [wert, titel]}<option
                  value={wert}>{titel}</option
                >{/each}</select
            ></label
          ><label
            >Nebenbestimmung<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Nebenbestimmung"
              required
              maxlength="500"
              readonly={auflagenAlt.includes(p.id)}
              bind:value={p.titel}
            /></label
          ><label
            >Fälligkeit / Auslöser<input
              maxlength="4000"
              readonly={auflagenAlt.includes(p.id)}
              bind:value={p.ausloeser}
            /></label
          ><Zustaendigkeit {benutzer} bind:wert={p.verantwortlich} />
        </div>
        {#if auflagenAlt.includes(p.id)}<p class="meta">
            Wortlaut aus dem Bescheid, nach dem Speichern unveränderlich.
          </p>{/if}
        <Fristfeld {dateien} bind:wert={p.frist} /><Dokumentliste
          {dateien}
          bind:werte={p.nachweise}
          label="Erfüllungsbelege"
        />{#if p.art === "hinweis"}<p class="meta">
            Ein Hinweis begründet keine Pflicht und wird nicht wiedervorgelegt.
          </p>{:else}<label
            ><input type="checkbox" bind:checked={p.erledigt} />Erfüllt (Beleg
            erforderlich)</label
          >{/if}{#if p.art === "befristung"}<p class="meta">
            Die Frist ist das Ablaufdatum der Befristung.
          </p>{/if}
      </fieldset>{/each}
  </details>{/each}
