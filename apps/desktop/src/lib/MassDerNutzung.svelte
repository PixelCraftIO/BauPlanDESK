<script lang="ts">
  import { api, fehlertext, GEBIETE, type Baugebiet, type Pruefergebnis } from "./api";

  let {
    grundstuecksflaeche,
    onfehler,
  }: { grundstuecksflaeche: number | null; onfehler: (m: string) => void } = $props();

  // svelte-ignore state_referenced_locally
  let eingabe = $state({
    grundstuecksflaeche_m2: grundstuecksflaeche ?? 0,
    grundflaeche_m2: 0,
    grundflaeche_nebenanlagen_m2: 0,
    geschossflaeche_m2: 0,
    baumasse_m3: null as number | null,
  });
  let gebiet = $state<Baugebiet>("WA");
  let ergebnis = $state<Pruefergebnis | null>(null);

  async function berechnen(event: SubmitEvent) {
    event.preventDefault();
    try {
      ergebnis = await api.massDerNutzung({ ...eingabe, baumasse_m3: eingabe.baumasse_m3 || null }, gebiet);
    } catch (e) {
      ergebnis = null;
      onfehler(fehlertext(e));
    }
  }

  const z = (n: number) => n.toLocaleString("de-DE", { maximumFractionDigits: 2 });
  const ok = (b: boolean | null) => (b === null ? "–" : b ? "✓ eingehalten" : "✗ überschritten");
</script>

<form class="raster" onsubmit={berechnen}>
  <label>Baugebiet
    <select bind:value={gebiet}>
      {#each GEBIETE as g}<option value={g.code}>{g.code} – {g.name}</option>{/each}
    </select>
  </label>
  <label>Grundstücksfläche (m²)<input type="number" min="0.01" step="0.01" bind:value={eingabe.grundstuecksflaeche_m2} required /></label>
  <label>Grundfläche Hauptanlagen (m²)<input type="number" min="0" step="0.01" bind:value={eingabe.grundflaeche_m2} /></label>
  <label>Garagen, Stellplätze, Nebenanlagen (m²)<input type="number" min="0" step="0.01" bind:value={eingabe.grundflaeche_nebenanlagen_m2} /></label>
  <label>Geschossfläche (m²)<input type="number" min="0" step="0.01" bind:value={eingabe.geschossflaeche_m2} /></label>
  <label>Baumasse (m³, optional)<input type="number" min="0" step="0.01" bind:value={eingabe.baumasse_m3} /></label>
  <div><button type="submit">Berechnen</button></div>
</form>

{#if ergebnis}
  <table class="ergebnistabelle">
    <thead><tr><th>Kennzahl</th><th>Ist</th><th>Obergrenze</th><th></th></tr></thead>
    <tbody>
      <tr><td>GRZ (§ 19 BauNVO)</td><td>{z(ergebnis.kennzahlen.grz)}</td><td>{z(ergebnis.grenzwerte.grz)}</td><td class:verstoss={!ergebnis.grz_eingehalten}>{ok(ergebnis.grz_eingehalten)}</td></tr>
      <tr><td>GRZ inkl. § 19 Abs. 4</td><td>{z(ergebnis.kennzahlen.grz_gesamt)}</td><td>{z(ergebnis.grz_gesamt_max)}</td><td class:verstoss={!ergebnis.grz_gesamt_eingehalten}>{ok(ergebnis.grz_gesamt_eingehalten)}</td></tr>
      <tr><td>GFZ (§ 20 BauNVO)</td><td>{z(ergebnis.kennzahlen.gfz)}</td><td>{ergebnis.grenzwerte.gfz === null ? "–" : z(ergebnis.grenzwerte.gfz)}</td><td class:verstoss={ergebnis.gfz_eingehalten === false}>{ok(ergebnis.gfz_eingehalten)}</td></tr>
      <tr><td>BMZ (§ 21 BauNVO)</td><td>{ergebnis.kennzahlen.bmz === null ? "–" : z(ergebnis.kennzahlen.bmz)}</td><td>{ergebnis.grenzwerte.bmz === null ? "–" : z(ergebnis.grenzwerte.bmz)}</td><td class:verstoss={ergebnis.bmz_eingehalten === false}>{ok(ergebnis.bmz_eingehalten)}</td></tr>
    </tbody>
  </table>
  <p class="meta">Orientierungswerte nach § 17 BauNVO. Festsetzungen eines Bebauungsplans gehen vor.</p>
{/if}
