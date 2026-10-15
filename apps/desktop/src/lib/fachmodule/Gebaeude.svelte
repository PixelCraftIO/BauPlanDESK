<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type { Project, GrundstueckKontext } from "../api";
  import { api, fehlertext } from "../api";
  import type { Gebaeude } from "../generated/plattform";
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
