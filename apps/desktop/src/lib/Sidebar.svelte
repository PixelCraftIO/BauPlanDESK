<script lang="ts">
  import { editorRegister } from "./editoren";
  import Navigationseintrag from "./Navigationseintrag.svelte";
  import { onMount } from "svelte";
  import { MODULE, voraussetzung } from "./module";
  import {
    navigationMitModulen,
    registerVon,
    KURZTITEL,
    type NavZweig,
  } from "./navigation";
  import type { Project } from "./api";
  import type { Ansicht } from "./projektSitzung";
  let {
    projekt,
    aktiv = true,
    ansicht,
    onbereich,
  }: {
    projekt: Project;
    aktiv?: boolean;
    ansicht: Ansicht;
    onbereich: (a: Ansicht) => void;
  } = $props();
  const NAVIGATION = navigationMitModulen(MODULE);
  const register = $derived(registerVon(NAVIGATION, ansicht) ?? NAVIGATION[0]);
  const editoren = editorRegister();
  let suchfeld = $state<HTMLInputElement>();
  let aktionsmeldung = $state("");
  let aktionLaeuft = $state(false);
  async function speichern() {
    if (aktionLaeuft) return;
    aktionLaeuft = true;
    aktionsmeldung = "";
    try {
      const id = ["grz", "gfz", "geschosse", "volumen", "mass"].includes(
        ansicht,
      )
        ? "grundstueck"
        : ansicht;
      const dirty = editoren?.hatAenderungenIn(id);
      const ok = await editoren?.speichernIn(id);
      aktionsmeldung = ok
        ? dirty
          ? "Bereich gespeichert bzw. vorbereitenden Entwurf lokal gesichert."
          : "Keine ungespeicherten Änderungen in diesem Bereich."
        : "Bereich konnte nicht gespeichert werden. Feldfehler und offene Bearbeitungen prüfen.";
    } catch {
      aktionsmeldung = "Speichern fehlgeschlagen; Eingaben bleiben erhalten.";
    } finally {
      aktionLaeuft = false;
    }
  }
  let suche = $state("");
  let favoriten = $state<string[]>([]);
  let zuletzt = $state<string[]>([]);
  onMount(() => {
    try {
      const d = JSON.parse(
        localStorage.getItem(`bauplan.wegen.${projekt.id}`) ?? "{}",
      );
      favoriten = Array.isArray(d.favoriten)
        ? d.favoriten.filter((id: string) => MODULE.some((m) => m.id === id))
        : [];
      zuletzt = Array.isArray(d.zuletzt)
        ? d.zuletzt.filter((id: string) => MODULE.some((m) => m.id === id))
        : [];
    } catch {
      /* optionale lokale Präferenzen */
    }
  });
  function merken(id: string, favorit = false) {
    if (favorit)
      favoriten = favoriten.includes(id)
        ? favoriten.filter((x) => x !== id)
        : [...favoriten, id];
    else zuletzt = [id, ...zuletzt.filter((x) => x !== id)].slice(0, 5);
    try {
      localStorage.setItem(
        `bauplan.wegen.${projekt.id}`,
        JSON.stringify({ favoriten, zuletzt }),
      );
    } catch {
      /* Navigation bleibt nutzbar */
    }
  }
  function oeffnen(id: import("./module").ModulId) {
    merken(id);
    suche = "";
    onbereich(id);
  }
  function istAktiv(id: string) {
    return ansicht === id || (ansicht === "mass" && id === "grundstueck");
  }
</script>

{#snippet eintraege(ids: import("./module").ModulId[])}
  {#each ids as id}
    {@const modul = MODULE.find((m) => m.id === id)!}
    {@const grund = voraussetzung(projekt, modul)}
    <Navigationseintrag
      {id}
      titel={modul.titel}
      kurz={KURZTITEL[id] ?? modul.titel}
      {grund}
      entwurf={["grz", "gfz", "geschosse", "volumen"].includes(id)}
      aktiv={istAktiv(id)}
      favorit={favoriten.includes(id)}
      onoeffnen={oeffnen}
      onfavorit={() => merken(id, true)}
    />
  {/each}
{/snippet}
{#snippet gruppe(z: NavZweig)}
  <div class="nav-gruppe">{z.titel}</div>
  {@render eintraege(z.module)}
  {#each z.kinder ?? [] as k}{@render gruppe(k)}{/each}
{/snippet}

<svelte:window
  onkeydown={(e) => {
    if (aktiv && (e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      suchfeld?.focus();
    }
  }}
/>
<aside class="aktennavigation" aria-label="Arbeitsbereiche">
  <label class="nav-suche"
    >Bereich suchen<input
      type="search"
      aria-label="Bereich oder Aktion suchen"
      title="Bereiche und Aktionen suchen (Strg+K)"
      bind:this={suchfeld}
      bind:value={suche}
      placeholder="z. B. GRZ, Formulare"
    /></label
  >
  {#if aktionsmeldung}<p role="status" class="nav-meldung">
      {aktionsmeldung}
    </p>{/if}
  {#if suche}<nav aria-label="Suchergebnisse">
      {#if "aktuellen bereich speichern".includes(suche.toLocaleLowerCase("de"))}<button
          type="button"
          disabled={aktionLaeuft}
          onclick={speichern}>Aktuellen Bereich speichern</button
        >{/if}
      {#if "drucken pdf ausgeben".includes(suche.toLocaleLowerCase("de"))}<button
          type="button"
          onclick={() => oeffnen("druck")}>Druckausgabe öffnen</button
        >{/if}
      {@render eintraege(
        MODULE.filter((m) =>
          m.titel
            .toLocaleLowerCase("de")
            .includes(suche.toLocaleLowerCase("de")),
        ).map((m) => m.id),
      )}
    </nav>{:else}
    {#if favoriten.length}<details>
        <summary>Favoriten</summary>{@render eintraege(
          favoriten as import("./module").ModulId[],
        )}
      </details>{/if}
    {#if zuletzt.length}<details>
        <summary>Zuletzt geöffnet</summary>{@render eintraege(
          zuletzt as import("./module").ModulId[],
        )}
      </details>{/if}
    <nav aria-label={`Gliederung ${register.titel}`}>
      {#if register.module.length || !register.kinder?.length}
        {@render gruppe(register)}
      {:else}
        {#each register.kinder ?? [] as k}{@render gruppe(k)}{/each}
      {/if}
    </nav>{/if}
</aside>

<style>
  nav {
    display: flex;
    flex-direction: column;
  }
  .nav-meldung {
    font-size: 0.8rem;
    padding: 0.5rem;
  }
  .nav-suche {
    display: block;
    padding: 0.5rem;
    font-size: 0.8rem;
  }
  .nav-suche input {
    width: 100%;
    min-width: 0;
  }
  details {
    padding: 0.4rem;
  }
</style>
