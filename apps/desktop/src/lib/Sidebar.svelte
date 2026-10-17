<script lang="ts">
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
    ansicht,
    onbereich,
  }: {
    projekt: Project;
    ansicht: Ansicht;
    onbereich: (a: Ansicht) => void;
  } = $props();
  const NAVIGATION = navigationMitModulen(MODULE);
  const register = $derived(registerVon(NAVIGATION, ansicht) ?? NAVIGATION[0]);
  function aktiv(id: string) {
    return ansicht === id || (ansicht === "mass" && id === "grundstueck");
  }
</script>

{#snippet eintraege(ids: import("./module").ModulId[])}
  {#each ids as id}
    {@const modul = MODULE.find((m) => m.id === id)!}
    {@const grund = voraussetzung(projekt, modul)}
    {#if !(id === "festsetzungen" && grund)}<button
        class:aktiv={aktiv(id)}
        aria-current={aktiv(id) ? "page" : undefined}
        aria-label={modul.titel}
        title={grund ?? modul.titel}
        disabled={!!grund}
        onclick={() => onbereich(id)}>{KURZTITEL[id] ?? modul.titel}</button
      >{/if}
  {/each}
{/snippet}
{#snippet gruppe(z: NavZweig)}
  <div class="nav-gruppe">{z.titel}</div>
  {@render eintraege(z.module)}
  {#each z.kinder ?? [] as k}{@render gruppe(k)}{/each}
{/snippet}

<aside class="aktennavigation" aria-label="Arbeitsbereiche">
  <nav aria-label={`Gliederung ${register.titel}`}>
    {#if register.module.length || !register.kinder?.length}
      {@render gruppe(register)}
    {:else}
      {#each register.kinder ?? [] as k}{@render gruppe(k)}{/each}
    {/if}
  </nav>
</aside>

<style>
  nav {
    display: flex;
    flex-direction: column;
  }
  nav > button {
    display: block;
    width: 100%;
    text-align: left;
    white-space: normal;
    overflow-wrap: anywhere;
    line-height: 1.35;
    padding-left: 18px;
  }
</style>
