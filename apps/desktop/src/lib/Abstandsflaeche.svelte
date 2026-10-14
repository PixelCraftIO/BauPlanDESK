<script lang="ts">
  import {
    api,
    fehlertext,
    GEBIETE,
    LAENDER,
    type AbstandsflaecheErgebnis,
    type Baugebiet,
    type Berechnung,
    type Bundesland,
    type RulePackInfo,
  } from "./api";

  let {
    startLand,
    regelpakete,
    onfehler,
  }: {
    startLand: Bundesland;
    regelpakete: RulePackInfo[];
    onfehler: (meldung: string) => void;
  } = $props();

  const laender = LAENDER.map((l) => l.code);
  const gebiete = GEBIETE.map((g) => g.code);
  // svelte-ignore state_referenced_locally
  const land = $derived(startLand);
  let gebiet = $state<Baugebiet>("WA");
  let wandhoehe = $state(6.5);
  let ergebnis = $state<Berechnung<AbstandsflaecheErgebnis> | null>(null);

  $effect(() => {
    void land;
    ergebnis = null;
  });
  const verfuegbar = $derived(new Set(regelpakete.map((r) => r.land)));

  async function berechnen(event: SubmitEvent) {
    event.preventDefault();
    try {
      ergebnis = await api.abstandsflaeche(land, {
        wandhoehe_m: wandhoehe,
        gebiet,
      });
    } catch (e) {
      ergebnis = null;
      onfehler(fehlertext(e));
    }
  }
</script>

<form onsubmit={berechnen}>
  <p class="meta">
    Landesbauordnung: {LAENDER.find((l) => l.code === land)?.name}. Auswahl
    unter Rechtsgrundlagen.
  </p>
  <select bind:value={gebiet}>
    {#each gebiete as g}<option value={g}>{g}</option>{/each}
  </select>
  <label
    >H <input type="number" min="0" step="0.01" bind:value={wandhoehe} /> m</label
  >
  <button type="submit" disabled={!verfuegbar.has(land)}>Berechnen</button>
</form>

{#if ergebnis}
  <p class="ergebnis">
    T = <strong>{ergebnis.ergebnis.tiefe_m.toFixed(2)} m</strong>
    ({ergebnis.ergebnis.faktor} · H, mind. {ergebnis.ergebnis.mindesttiefe_m} m)
  </p>
  <p class="meta">
    {ergebnis.ergebnis.rechtsgrundlage} · Regelpaket {ergebnis.regelpaket.land}
    {ergebnis.regelpaket.version}
  </p>
{/if}
