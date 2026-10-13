<script lang="ts">
  import { api, fehlertext, GEBIETE, LAENDER, type AbstandsflaecheErgebnis, type Baugebiet, type Berechnung, type Bundesland, type RulePackInfo } from "./api";

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
  let land = $state<Bundesland>(startLand);
  let gebiet = $state<Baugebiet>("WA");
  let wandhoehe = $state(6.5);
  let ergebnis = $state<Berechnung<AbstandsflaecheErgebnis> | null>(null);

  const verfuegbar = $derived(new Set(regelpakete.map((r) => r.land)));

  async function berechnen(event: SubmitEvent) {
    event.preventDefault();
    try {
      ergebnis = await api.abstandsflaeche(land, { wandhoehe_m: wandhoehe, gebiet });
    } catch (e) {
      ergebnis = null;
      onfehler(fehlertext(e));
    }
  }
</script>

<form onsubmit={berechnen}>
  <select bind:value={land}>
    {#each laender as l}
      <option value={l} disabled={!verfuegbar.has(l)}>{l}{verfuegbar.has(l) ? "" : " (kein Regelpaket)"}</option>
    {/each}
  </select>
  <select bind:value={gebiet}>
    {#each gebiete as g}<option value={g}>{g}</option>{/each}
  </select>
  <label>H <input type="number" min="0" step="0.01" bind:value={wandhoehe} /> m</label>
  <button type="submit">Berechnen</button>
</form>

{#if ergebnis}
  <p class="ergebnis">
    T = <strong>{ergebnis.ergebnis.tiefe_m.toFixed(2)} m</strong>
    ({ergebnis.ergebnis.faktor} · H, mind. {ergebnis.ergebnis.mindesttiefe_m} m)
  </p>
  <p class="meta">
    {ergebnis.ergebnis.rechtsgrundlage} · Regelpaket {ergebnis.regelpaket.land} {ergebnis.regelpaket.version}
  </p>
{/if}
