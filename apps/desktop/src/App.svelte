<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import {
    api,
    fehlertext,
    type AppInfo,
    type Baugebiet,
    type Bundesland,
    type Project,
    type UpdateInfo,
  } from "./lib/api";
  import Abstandsflaeche from "./lib/Abstandsflaeche.svelte";

  let info = $state<AppInfo | null>(null);
  let projekte = $state<Project[]>([]);
  let fehler = $state<string | null>(null);
  let update = $state<UpdateInfo | null>(null);
  let installiere = $state(false);

  let neuName = $state("");
  let neuLand = $state<Bundesland>("BY");
  let neuFlaeche = $state<number | null>(null);

  async function laden() {
    try {
      [info, projekte] = await Promise.all([api.appInfo(), api.projekte()]);
    } catch (e) {
      fehler = fehlertext(e);
    }
  }

  async function anlegen(event: SubmitEvent) {
    event.preventDefault();
    try {
      await api.projektAnlegen({
        name: neuName,
        bundesland: neuLand,
        grundstuecksflaeche_m2: neuFlaeche,
      });
      neuName = "";
      neuFlaeche = null;
      projekte = await api.projekte();
    } catch (e) {
      fehler = fehlertext(e);
    }
  }

  async function updateInstallieren() {
    installiere = true;
    try {
      await api.updateInstallieren(); // startet die App neu
    } catch (e) {
      fehler = fehlertext(e);
      installiere = false;
    }
  }

  onMount(() => {
    laden();
    const ab = listen<UpdateInfo>("update-verfuegbar", (e) => (update = e.payload));
    return () => {
      ab.then((f) => f());
    };
  });

  const laender: Bundesland[] = ["BW", "BY", "BE", "BB", "HB", "HH", "HE", "MV", "NI", "NW", "RP", "SL", "SN", "ST", "SH", "TH"];
  const gebiete: Baugebiet[] = ["WS", "WR", "WA", "WB", "MD", "MDW", "MI", "MU", "MK", "GE", "GI", "SO"];
</script>

<header>
  <h1>BauProjektIO</h1>
  {#if info}
    <span class="meta">v{info.version} · {info.modus} · Ablage: {info.storage}</span>
  {/if}
</header>

{#if update}
  <div class="banner">
    Version {update.version} ist verfügbar.
    <button onclick={updateInstallieren} disabled={installiere}>
      {installiere ? "Wird installiert …" : "Jetzt aktualisieren"}
    </button>
    <button class="sekundaer" onclick={() => (update = null)}>Später</button>
  </div>
{/if}

{#if fehler}
  <div class="fehler" role="alert">
    {fehler}
    <button class="sekundaer" onclick={() => (fehler = null)}>Schließen</button>
  </div>
{/if}

<main>
  <section>
    <h2>Projekte</h2>
    <form onsubmit={anlegen}>
      <input placeholder="Projektname" bind:value={neuName} required />
      <select bind:value={neuLand}>
        {#each laender as land}<option value={land}>{land}</option>{/each}
      </select>
      <input type="number" min="1" step="0.01" placeholder="Grundstück m²" bind:value={neuFlaeche} />
      <button type="submit">Anlegen</button>
    </form>
    <ul>
      {#each projekte as p (p.id)}
        <li>
          <strong>{p.name}</strong>
          <span class="meta">{p.bundesland}{p.grundstuecksflaeche_m2 ? ` · ${p.grundstuecksflaeche_m2} m²` : ""} · Version {p.version}</span>
        </li>
      {:else}
        <li class="meta">Noch keine Projekte.</li>
      {/each}
    </ul>
  </section>

  <section>
    <h2>Abstandsfläche</h2>
    <Abstandsflaeche {laender} {gebiete} regelpakete={info?.regelpakete ?? []} onfehler={(m) => (fehler = m)} />
  </section>
</main>
