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
    wohngebaeude = $state("ungeklaert"),
    nutzung = $state(""),
    katalog = $state<Anforderungskatalog | null>(null),
    kataloge = $state<Anforderungskatalog[]>([]),
    fehler = $state("");
  onMount(async () => {
    try {
      kataloge = (await verfahrenApi.kataloge().catch(() => [])).filter(
        (k) =>
          (!k.bundesland || k.bundesland === projekt.bundesland) &&
          (!k.art || k.art === art),
      );
      // Ein Landespaket für das Bundesland des Projekts geht vor.
      katalog =
        kataloge.find((k) => k.bundesland === projekt.bundesland) ??
        kataloge[0] ??
        (await verfahrenApi.profil());
      if (stand) {
        klasse = stand.vorhaben.gebaeudeklasse ?? undefined;
        sonderbau =
          stand.vorhaben.sonderbau === null
            ? "ungeklaert"
            : String(stand.vorhaben.sonderbau);
        nutzung = stand.vorhaben.nutzung;
        wohngebaeude =
          stand.vorhaben.wohngebaeude == null
            ? "ungeklaert"
            : String(stand.vorhaben.wohngebaeude);
      } else {
        // Die geplante Nutzung steht bereits in den Stammdaten des Projekts.
        nutzung = projekt.planungsrecht.nutzung.trim().slice(0, 500);
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
  ><label
    >Wohngebäude<select bind:value={wohngebaeude}
      ><option value="ungeklaert">Noch zu klären</option><option value="true"
        >Ja</option
      ><option value="false">Nein</option></select
    ></label
  ><label>Nutzung<input maxlength="500" bind:value={nutzung} /></label
  >{#if kataloge.length > 1}<label
      >Katalog<select
        value={katalog?.id ?? ""}
        onchange={(e) =>
          (katalog =
            kataloge.find((k) => k.id === e.currentTarget.value) ?? katalog)}
        >{#each kataloge as k (k.id)}<option value={k.id}>{k.titel}</option
          >{/each}</select
      ></label
    >{/if}<label
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
          ...(wohngebaeude === "ungeklaert"
            ? {}
            : { wohngebaeude: wohngebaeude === "true" }),
        },
        katalog!,
      )}>Anforderungen aus Profil vorbereiten</button
  >
  <p class="meta">
    Die Erforderlichkeit wird nach Verfahrensart, Wohngebäude, Gebäudeklasse,
    Sonderbau und planungsrechtlicher Grundlage vorbelegt. Eigene
    Entscheidungen bleiben beim erneuten Vorbereiten erhalten. Die fachliche
    Freigabe bleibt bei der Projektleitung.
  </p>{/if}
