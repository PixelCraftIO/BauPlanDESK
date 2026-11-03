<script lang="ts">
  import { onMount, untrack } from "svelte";
  import {
    api,
    fehlertext,
    type Project,
    type Benutzer,
    type DocumentVersion,
  } from "../api";
  import type {
    Tagebucheintrag,
    Tagebuchart,
    Verfahrensakte,
  } from "../generated/plattform";
  import { FachSitzung } from "../FachSitzung.svelte";
  import Fachliste from "../Fachliste.svelte";
  import Dokumentliste from "../verfahren/Dokumentliste.svelte";
  import { plattformApi } from "../plattformApi";
  import { ansichtsAbfrage } from "../ansichtsAbfrage.svelte";
  let {
    projekt,
    benutzerId,
    darf,
  }: { projekt: Project; benutzerId: string; darf: boolean } = $props();
  const ARTEN: [Tagebuchart, string][] = [
    ["notiz", "Notiz"],
    ["telefonat", "Telefonat"],
    ["besprechung", "Besprechung"],
    ["ortstermin", "Ortstermin"],
    ["vorfall", "Vorfall / Mangel"],
    ["entscheidung_bauherr", "Entscheidung Bauherr"],
    ["sonstiges", "Sonstiges"],
  ];
  const artText = (a: Tagebuchart) => ARTEN.find(([k]) => k === a)?.[1] ?? a;
  const heute = () => {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  };
  const s = untrack(
    () =>
      new FachSitzung<Tagebucheintrag>(
        projekt.id,
        "projekttagebuch",
        benutzerId,
        () => ({
          datum: heute(),
          art: "notiz",
          titel: "",
          text: "",
          beteiligte: "",
          anhaenge: [],
          verfahren: null,
          wiedervorlage: null,
        }),
        (e) =>
          `${new Date(e.datum + "T12:00:00").toLocaleDateString("de-DE")} · ${artText(e.art)} · ${e.titel}${e.wiedervorlage && !e.wiedervorlage.erledigt ? ` · Wiedervorlage ${new Date(e.wiedervorlage.datum + "T12:00:00").toLocaleDateString("de-DE")}` : ""}`,
      ),
  );
  let benutzer = $state<Benutzer[]>([]),
    dateien = $state<DocumentVersion[]>([]),
    verfahren = $state<{ id: string; bezeichnung: string }[]>([]);
  async function dateienLaden() {
    try {
      dateien = await api.dokumente(projekt.id);
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
  ansichtsAbfrage(() => projekt.id, ["dokumente"], dateienLaden);
  onMount(async () => {
    try {
      const [b, v] = await Promise.all([
        api.benutzerAuflisten(),
        plattformApi.liste<Verfahrensakte>(projekt.id, "verfahren"),
      ]);
      benutzer = b;
      verfahren = v.map((f) => ({ id: f.objekt, bezeichnung: f.daten.bezeichnung }));
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  });
</script>

<Fachliste titel="Projekttagebuch" {s} bereich="projekttagebuch" {darf}>
  {#snippet children(e)}<p class="meta">
      Vorfälle, Telefonate, Besprechungen und Entscheidungen außerhalb der
      Behördenpost. Schreiben der Behörde gehören in die Verfahrensakte
      (Kommunikation).
    </p>
    <div class="raster">
      <label
        >Datum<span class="pflicht" aria-hidden="true"> *</span><input
          type="date"
          required
          max={heute()}
          bind:value={e.datum}
        /></label
      ><label
        >Art<select bind:value={e.art}
          >{#each ARTEN as [k, t]}<option value={k}>{t}</option>{/each}</select
        ></label
      ><label
        >Betreff<span class="pflicht" aria-hidden="true"> *</span><input
          aria-label="Betreff"
          bind:value={e.titel}
          required
          maxlength="300"
        /></label
      ><label
        >Beteiligte<input
          bind:value={e.beteiligte}
          maxlength="2000"
          placeholder="z. B. Bauherr Müller, Statikbüro Schmidt"
        /></label
      ><label
        >Bezug zum Verfahren<select bind:value={e.verfahren}
          ><option value={null}>Kein Verfahren</option
          >{#each verfahren as v (v.id)}<option value={v.id}
              >{v.bezeichnung}</option
            >{/each}</select
        ></label
      >
    </div>
    <label
      >Inhalt<textarea bind:value={e.text} maxlength="20000" rows="6"
      ></textarea></label
    >
    <Dokumentliste
      {dateien}
      bind:werte={e.anhaenge}
      label="Anhänge (Fotos, Protokolle, Schreiben)"
    />
    <fieldset class="wiedervorlage">
      <legend>Wiedervorlage</legend>
      {#if e.wiedervorlage}
        <div class="raster">
          <label
            >Am<input type="date" bind:value={e.wiedervorlage.datum} /></label
          ><label
            >Zuständig<select bind:value={e.wiedervorlage.verantwortlich}
              ><option value={null}>Noch nicht zugeordnet</option
              >{#each benutzer.filter((b) => b.aktiv) as b (b.id)}<option
                  value={b.id}>{b.anzeigename}</option
                >{/each}</select
            ></label
          ><label class="checkbox"
            ><input
              type="checkbox"
              bind:checked={e.wiedervorlage.erledigt}
            />Erledigt</label
          >
        </div>
        <button
          type="button"
          class="link"
          onclick={() => (e.wiedervorlage = null)}
          >Wiedervorlage entfernen</button
        >
      {:else}
        <button
          type="button"
          class="sekundaer"
          onclick={() =>
            (e.wiedervorlage = {
              datum: heute(),
              verantwortlich: benutzerId,
              erledigt: false,
            })}>Wiedervorlage setzen …</button
        >
        <span class="meta"
          >Offene Wiedervorlagen erscheinen in der Projektübersicht, im
          Büro-Cockpit und im Kalender-Abo.</span
        >
      {/if}
    </fieldset>{/snippet}
</Fachliste>
