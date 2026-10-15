<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type { Project } from "../api";
  import { fehlertext } from "../api";
  import type { Raumbuch, Gebaeude, RaumSummen } from "../generated/plattform";
  import { plattformApi, type Datensatz } from "../plattformApi";
  import { FachSitzung } from "../FachSitzung.svelte";
  import Fachliste from "../Fachliste.svelte";
  let {
    projekt,
    benutzerId,
    darf,
  }: { projekt: Project; benutzerId: string; darf: boolean } = $props();
  let gebaeude = $state<Datensatz<Gebaeude>[]>([]);
  let summen = $state<RaumSummen | null>(null);
  const s = untrack(
    () =>
      new FachSitzung<Raumbuch>(
        projekt.id,
        "raumbuch",
        benutzerId,
        () => ({
          bezeichnung: "",
          gebaeude_id: gebaeude[0]?.objekt ?? "",
          geschoss_id: null,
          raeume: [],
        }),
        (r) => r.bezeichnung,
      ),
  );
  const geschosse = $derived(
    gebaeude.find((g) => g.objekt === s.eingabe?.gebaeude_id)?.daten
      .geschosse ?? [],
  );
  $effect(() => {
    void s.id;
    void s.revision;
    if (s.hatAenderungen()) summen = null;
  });
  onMount(async () => {
    try {
      gebaeude = await plattformApi.liste<Gebaeude>(projekt.id, "gebaeude");
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  });
  async function berechnen() {
    if (!s.id) return;
    try {
      if (await s.speichern())
        summen = await plattformApi.raumsummen(projekt.id, s.id);
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
  const z = (n: number) =>
    n.toLocaleString("de-DE", { maximumFractionDigits: 2 });
</script>

<Fachliste titel="Flächen- und Raumbuch" {s} bereich="raumbuch" {darf}>
  {#snippet children(r)}<div class="raster">
      <label>Bezeichnung<input bind:value={r.bezeichnung} required /></label
      ><label
        >Gebäude<select
          bind:value={r.gebaeude_id}
          onchange={() => (r.geschoss_id = null)}
          required
          ><option value="">Gebäude auswählen</option
          >{#each gebaeude as g (g.objekt)}<option value={g.objekt}
              >{g.daten.bezeichnung}</option
            >{/each}</select
        ></label
      ><label
        >Geschoss<select bind:value={r.geschoss_id}
          ><option value={null}>Alle Geschosse des Gebäudes</option
          >{#each geschosse as g (g.id)}<option value={g.id}
              >{g.bezeichnung}</option
            >{/each}</select
        ></label
      >
    </div>
    <div class="tabellen-scroll">
      <table class="ergebnistabelle">
        <thead
          ><tr
            ><th>Raumnummer</th><th>Bezeichnung</th><th>Nutzung</th><th
              >Erfasste Fläche m²</th
            ><th>Lichte Höhe m</th><th></th></tr
          ></thead
        ><tbody
          >{#each r.raeume as raum, i (raum.id)}<tr
              ><td
                ><input
                  aria-label="Raumnummer"
                  bind:value={raum.nummer}
                  required
                /></td
              ><td
                ><input
                  aria-label="Raumbezeichnung"
                  bind:value={raum.bezeichnung}
                  required
                /></td
              ><td
                ><input
                  aria-label="Raumnutzung"
                  bind:value={raum.nutzung}
                /></td
              ><td
                ><input
                  aria-label="Raumfläche m²"
                  type="number"
                  min="0"
                  step="any"
                  bind:value={raum.flaeche_m2}
                  required
                /></td
              ><td
                ><input
                  aria-label="Lichte Raumhöhe m"
                  type="number"
                  min="0.01"
                  step="any"
                  bind:value={raum.hoehe_m}
                /></td
              ><td
                ><button
                  type="button"
                  class="link"
                  onclick={() => r.raeume.splice(i, 1)}>Entfernen</button
                ></td
              ></tr
            >{/each}</tbody
        >
      </table>
    </div>
    <button
      type="button"
      class="sekundaer"
      onclick={() =>
        r.raeume.push({
          id: crypto.randomUUID(),
          nummer: "",
          bezeichnung: "",
          nutzung: "",
          flaeche_m2: 0,
          hoehe_m: null,
        })}>Raum hinzufügen</button
    ><button type="button" disabled={s.laeuft} onclick={berechnen}
      >Speichern und Summen ermitteln</button
    >
    <p class="meta">
      Summe der erfassten Raumflächen. Dies ist kein Nachweis nach WoFlV oder
      DIN 277; deren Anrechnungsregeln sind getrennte Fachmodule.
    </p>
    {#if summen}<p role="status">
        {summen.anzahl} Räume · {z(summen.flaeche_m2)} m² · {summen.volumen_m3 ===
        null
          ? "Volumen unvollständig"
          : `${z(summen.volumen_m3)} m³`}
      </p>{/if}
  {/snippet}
</Fachliste>
