<script lang="ts">
  import { datumDe } from "../api";
  import type {
    BautechnischerNachweis,
    Bauvorbereitung,
    Bescheid,
    Phasencheckliste,
    Phase,
  } from "../generated/plattform";
  import { fehlertext, type DocumentVersion } from "../api";
  import Dokumentliste from "./Dokumentliste.svelte";
  import {
    FAELLIGKEIT,
    NACHWEISART,
    PRUEFART,
    heute,
    verfahrenApi,
  } from "./api";
  let {
    nachweise = $bindable(),
    vorbereitung = $bindable(),
    bescheide,
    dateien,
    projektId,
    verfahrenId,
    revision,
  }: {
    nachweise: BautechnischerNachweis[];
    vorbereitung: Bauvorbereitung | undefined;
    bescheide: Bescheid[];
    dateien: DocumentVersion[];
    projektId: string;
    verfahrenId: string | null;
    revision: number;
  } = $props();
  nachweise ??= [];
  let checklisten = $state<Partial<Record<Phase, Phasencheckliste>>>({}),
    fehler = $state("");
  $effect(() => {
    const id = verfahrenId;
    void revision;
    if (!id || !revision) return;
    for (const phase of ["baubeginn", "nutzungsaufnahme"] as Phase[])
      verfahrenApi.checkliste(projektId, id, phase).then(
        (c) => (checklisten[phase] = c),
        (e) => (fehler = fehlertext(e)),
      );
  });
  const vb = $derived(
    vorbereitung ?? {
      bauleitung: "",
      bauleitung_benannt: null,
      absteckung: null,
      baubeginnanzeige: null,
      nutzungsanzeige: null,
    },
  );
  function setzen<K extends keyof Bauvorbereitung>(
    k: K,
    w: Bauvorbereitung[K],
  ) {
    vorbereitung = { ...vb, [k]: w };
  }
  function neu() {
    nachweise = [
      ...nachweise,
      {
        id: crypto.randomUUID(),
        art: "standsicherheit",
        bezeichnung: "",
        pruefung: "keine",
        pruefer: "",
        faellig: "vor_baubeginn",
        vorgelegt: null,
        bescheinigt: null,
      },
    ];
  }
  const auflagen = $derived(
    bescheide.flatMap((b) =>
      b.auflagen
        .filter((n) => n.art === "auflage" || n.art === "bedingung")
        .map((n) => ({ ...n, datum: b.datum })),
    ),
  );
</script>

<h3>Bauausführung</h3>
{#if fehler}<p class="meta">{fehler}</p>{/if}
{#each ["baubeginn", "nutzungsaufnahme"] as Phase[] as phase}{@const c =
    checklisten[phase]}{#if c}<section class="checkliste">
      <h4>
        Voraussetzungen {phase === "baubeginn"
          ? "des Baubeginns"
          : "der Nutzungsaufnahme"}{c.bereit ? " · alle belegt" : ""}
      </h4>
      <ul>
        {#each c.voraussetzungen as v}<li>
            {v.erfuellt === true ? "✓" : v.erfuellt === false ? "✗" : "?"}
            {v.titel}{v.hinweis ? ` – ${v.hinweis}` : ""}{v.quelle
              ? ` (${v.quelle})`
              : ""}
          </li>{/each}
      </ul>
      {#each c.hinweise as h}<p class="hinweis">{h}</p>{/each}
    </section>{/if}{/each}
<h4>Bauleitung und Anzeigen</h4>
<div class="raster">
  <label
    >Bauleiter/in<input
      maxlength="500"
      value={vb.bauleitung}
      oninput={(e) => setzen("bauleitung", e.currentTarget.value)}
    /></label
  >
  {#each [["bauleitung_benannt", "Der Behörde benannt am"], ["absteckung", "Grundriss und Höhenlage abgesteckt am"], ["baubeginnanzeige", "Baubeginn angezeigt am"], ["nutzungsanzeige", "Nutzungsaufnahme angezeigt am"]] as [k, titel]}<label
      >{titel}<input
        type="date"
        max={heute()}
        value={vb[k as keyof Bauvorbereitung] ?? ""}
        onchange={(e) =>
          setzen(
            k as "absteckung",
            (e.currentTarget.value || null) as string | null,
          )}
      /></label
    >{/each}
</div>
<h4>Bautechnische Nachweise</h4>
<button type="button" class="sekundaer" onclick={neu}>Nachweis erfassen</button>
{#each nachweise as n (n.id)}<fieldset>
    <legend>{NACHWEISART[n.art]}{n.bezeichnung ? ` – ${n.bezeichnung}` : ""}</legend>
    <div class="raster">
      <label
        >Art<select bind:value={n.art}
          >{#each Object.entries(NACHWEISART) as [w, t]}<option value={w}
              >{t}</option
            >{/each}</select
        ></label
      ><label>Bezeichnung<input maxlength="500" bind:value={n.bezeichnung} /></label
      ><label
        >Prüfung<select bind:value={n.pruefung}
          >{#each Object.entries(PRUEFART) as [w, t]}<option value={w}
              >{t}</option
            >{/each}</select
        ></label
      ><label>Prüfer/in<input maxlength="500" bind:value={n.pruefer} /></label
      ><label
        >Fällig<select bind:value={n.faellig}
          >{#each Object.entries(FAELLIGKEIT) as [w, t]}<option value={w}
              >{t}</option
            >{/each}</select
        ></label
      ><label
        >Vorgelegt am<input
          type="date"
          max={heute()}
          value={n.vorgelegt ?? ""}
          onchange={(e) => (n.vorgelegt = e.currentTarget.value || null)}
        /></label
      ><label
        >Geprüft/bescheinigt am<input
          type="date"
          max={heute()}
          value={n.bescheinigt ?? ""}
          onchange={(e) => (n.bescheinigt = e.currentTarget.value || null)}
        /></label
      >
    </div>
    <Dokumentliste
      {dateien}
      bind:werte={() => n.dokumente ?? [], (w) => (n.dokumente = w)}
      label="Nachweis, Prüfbericht, Bescheinigung"
    />
  </fieldset>{/each}
{#if auflagen.length}<h4>Auflagen und Bedingungen nach Fälligkeit</h4>
  <p class="meta">Fälligkeit im Register Bescheide zuordnen.</p>
  {#each Object.entries(FAELLIGKEIT) as [f, t]}{@const liste = auflagen.filter(
      (a) => (a.faelligkeit ?? "") === f,
    )}{#if liste.length}<h5>{t}</h5>
      <ul>
        {#each liste as a}<li>
            {a.erledigt ? "✓" : "○"}
            {a.titel} <span class="meta">(Bescheid vom {datumDe(a.datum)})</span>
          </li>{/each}
      </ul>{/if}{/each}{/if}
