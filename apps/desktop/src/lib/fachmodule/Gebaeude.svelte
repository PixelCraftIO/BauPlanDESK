<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type { Project, GrundstueckKontext } from "../api";
  import { api, fehlertext } from "../api";
  import type {
    Gebaeude,
    NachweisAnsicht,
    FachErgebnis,
  } from "../generated/plattform";
  import { plattformApi } from "../plattformApi";
  import { FachSitzung } from "../FachSitzung.svelte";
  import Fachliste from "../Fachliste.svelte";
  let {
    projekt,
    benutzerId,
    darf,
  }: { projekt: Project; benutzerId: string; darf: boolean } = $props();
  const s = untrack(
    () =>
      new FachSitzung<Gebaeude>(
        projekt.id,
        "gebaeude",
        benutzerId,
        () => ({
          bezeichnung: "",
          grundstueck_ids: [],
          geschosse: [],
          quelle: null,
        }),
        (g) => g.bezeichnung,
      ),
  );
  let nachweise = $state<NachweisAnsicht[]>([]);
  async function quellstaende() {
    try {
      nachweise = await plattformApi.nachweise(projekt.id);
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
  onMount(quellstaende);
  function quellen(geschossId: string) {
    return nachweise.flatMap((n) => {
      if (
        !n.aktuell ||
        !n.stand.vollstaendig ||
        n.stand.eingabe.art !== "fach" ||
        n.stand.eingabe.plan.modul !== "vollgeschosse" ||
        n.stand.eingabe.plan.gebaeude_id !== s.id
      )
        return [];
      const input = n.stand.eingabe.plan.eingaben as {
        geschosse: { id: string; gebaeude_geschoss_id: string | null }[];
      };
      const r = n.stand.ergebnis as unknown as FachErgebnis;
      return r.kennzahlen
        .filter(
          (k) =>
            k.id.startsWith("geschoss:") &&
            input.geschosse.some(
              (s) =>
                `geschoss:${s.id}` === k.id &&
                s.gebaeude_geschoss_id === geschossId,
            ),
        )
        .map((k) => ({
          nachweis: n.stand.id,
          kennzahl: k.id,
          wert: k.wert,
          titel: `${n.stand.bezeichnung} · ${k.bezeichnung}`,
        }));
    });
  }
  let grund = $state<GrundstueckKontext[]>([]);
  onMount(async () => {
    try {
      grund = await api.grundstueckskontext(projekt.id);
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  });
</script>

<Fachliste titel="Gebäude und Geschosse" {s} bereich="gebaeude" {darf}>
  {#snippet children(g)}
    <label
      >Gebäudebezeichnung<input
        bind:value={g.bezeichnung}
        required
        maxlength="500"
      /></label
    >
    <fieldset>
      <legend>Zugeordnete Baugrundstücke</legend>{#each grund as k (k.id)}<label
          ><input
            type="checkbox"
            checked={g.grundstueck_ids.includes(k.id)}
            onchange={(e) => {
              g.grundstueck_ids = e.currentTarget.checked
                ? [...g.grundstueck_ids, k.id]
                : g.grundstueck_ids.filter((id) => id !== k.id);
            }}
          />{k.bezeichnung}</label
        >{:else}<p class="meta">
          Zunächst Baugrundstücke ermitteln. Die Zuordnung kann anschließend
          ergänzt werden.
        </p>{/each}
    </fieldset>
    <h3>Geschosse</h3>
    <button type="button" class="sekundaer" onclick={quellstaende}
      >Vollgeschossnachweise aktualisieren</button
    >
    {#each g.geschosse as geschoss, i (geschoss.id)}<fieldset>
        <legend>Geschoss {i + 1}</legend>
        <div class="raster">
          <label
            >Bezeichnung<input
              bind:value={geschoss.bezeichnung}
              required
            /></label
          ><label
            >Einstufung<select
              aria-label="Einstufung"
              bind:value={geschoss.vollgeschoss}
              disabled={!!geschoss.einstufungsquelle}
              ><option value={null}>Noch zu prüfen</option><option value={true}
                >Vollgeschoss</option
              ><option value={false}>Nichtvollgeschoss</option></select
            ></label
          ><label
            >Grundlage der Einstufung<input
              bind:value={geschoss.grundlage}
              required={geschoss.vollgeschoss !== null}
              placeholder="Anwendbare Vorschrift und Herleitung"
            /></label
          >
        </div>
        <label
          >Aus Vollgeschossnachweis übernehmen<select
            value={geschoss.einstufungsquelle
              ? `${geschoss.einstufungsquelle.nachweis_id}|${geschoss.einstufungsquelle.kennzahl}`
              : ""}
            onchange={(e) => {
              const q = quellen(geschoss.id).find(
                (q) => `${q.nachweis}|${q.kennzahl}` === e.currentTarget.value,
              );
              geschoss.einstufungsquelle = q
                ? { nachweis_id: q.nachweis, kennzahl: q.kennzahl }
                : undefined;
              if (q) geschoss.vollgeschoss = q.wert === 1;
            }}
            ><option value="">Manuelle Einstufung</option
            >{#each quellen(geschoss.id) as q}<option
                value={`${q.nachweis}|${q.kennzahl}`}>{q.titel}</option
              >{/each}</select
          ></label
        >
        {#if geschoss.einstufungsquelle}<p class="meta">
            Die GFZ-/Geschossigkeitsberechnung prüft diesen Quellnachweis bei
            jeder Verwendung auf Aktualität.
          </p>{/if}
        <button
          type="button"
          class="link"
          onclick={() => g.geschosse.splice(i, 1)}>Geschoss entfernen</button
        >
      </fieldset>{/each}
    <button
      type="button"
      class="sekundaer"
      onclick={() =>
        g.geschosse.push({
          id: crypto.randomUUID(),
          bezeichnung: "",
          vollgeschoss: null,
          grundlage: "",
        })}>Geschoss hinzufügen</button
    >
    <p class="meta">
      Gebäude und Geschosse besitzen feste Identitäten. Die
      Vollgeschosseinstufung wird mit ihrer fachlichen Grundlage erfasst.
    </p>
  {/snippet}
</Fachliste>
