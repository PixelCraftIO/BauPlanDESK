<script lang="ts">
  import { onMount, untrack } from "svelte";
  import type {
    Anforderungskatalog,
    Profilstand,
    Vorhabenprofil,
    Verfahrensart,
  } from "../generated/plattform";
  import type { Project } from "../api";
  import { fehlertext } from "../api";
  import { verfahrenApi } from "./api";
  let {
    projekt,
    art,
    stand,
    onprofil,
  }: {
    projekt: Project;
    art: Verfahrensart;
    stand: Profilstand | null;
    onprofil: (p: Vorhabenprofil, k: Anforderungskatalog) => Promise<void>;
  } = $props();
  let klasse = $state<number | undefined>(
      untrack(() => stand?.vorhaben.gebaeudeklasse ?? undefined),
    ),
    sonderbau = $state("ungeklaert"),
    nutzung = $state(""),
    katalog = $state<Anforderungskatalog | null>(null),
    fehler = $state("");
  onMount(async () => {
    try {
      katalog = await verfahrenApi.profil();
      if (stand) {
        klasse = stand.vorhaben.gebaeudeklasse ?? undefined;
        sonderbau =
          stand.vorhaben.sonderbau === null
            ? "ungeklaert"
            : String(stand.vorhaben.sonderbau);
        nutzung = stand.vorhaben.nutzung;
      }
    } catch (e) {
      fehler = fehlertext(e);
    }
  });
  async function laden(e: Event) {
    try {
      const f = (e.currentTarget as HTMLInputElement).files?.[0];
      if (!f) return;
      if (f.size > 256 * 1024) throw Error("Katalog höchstens 256 KB");
      katalog = JSON.parse(await f.text());
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
</script>

<h3>Vorhabenprofil und Anforderungskatalog</h3>
{#if stand}<p>
    Gespeicherter Katalog: {stand.katalog.titel} · Version {stand.katalog
      .version} · {stand.bestaetigt_von
      ? "Profil bestätigt"
      : "Fachliche Prüfung offen"}
  </p>{/if}
<div class="raster">
  <label
    >Gebäudeklasse<input
      type="number"
      min="1"
      max="5"
      bind:value={klasse}
    /></label
  ><label
    >Sonderbau<select bind:value={sonderbau}
      ><option value="ungeklaert">Noch zu klären</option><option value="true"
        >Ja</option
      ><option value="false">Nein</option></select
    ></label
  ><label>Nutzung<input maxlength="500" bind:value={nutzung} /></label><label
    >Eigenen Katalog laden<input
      type="file"
      accept=".json"
      onchange={laden}
    /></label
  >
</div>
{#if fehler}<p role="alert" class="fehlertext">{fehler}</p>{/if}{#if katalog}<p>
    {katalog.titel} · Version {katalog.version}
  </p>
  <p class="hinweisbox">{katalog.hinweis}</p>
  <button
    type="button"
    class="sekundaer"
    onclick={() =>
      onprofil(
        {
          bundesland: projekt.bundesland,
          art,
          gebaeudeklasse: klasse ?? null,
          sonderbau: sonderbau === "ungeklaert" ? null : sonderbau === "true",
          nutzung,
        },
        katalog!,
      )}>Anforderungen aus Profil vorbereiten</button
  >{/if}
