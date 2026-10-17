<script lang="ts">
  import { MODULE, voraussetzung } from "./module";
  import {
    navigationMitModulen,
    KURZTITEL,
    enthaelt,
    type NavZweig,
  } from "./navigation";
  import { untrack } from "svelte";
  import NavIcon from "./NavIcon.svelte";
  import type { Project } from "./api";
  import type { Ansicht } from "./projektSitzung";
  let {
    projekt,
    ansicht,
    globalAnsicht,
    onbereich,
    onglobal,
  }: {
    projekt: Project | null;
    ansicht: Ansicht;
    globalAnsicht: string | null;
    onbereich: (a: Ansicht) => void;
    onglobal: (a: "projekte" | "kunden" | "formulare" | "verwaltung") => void;
  } = $props();
  const NAVIGATION = navigationMitModulen(MODULE);
  let offen = $state<Record<string, boolean>>({});
  let projektId = "";
  let letzteAnsicht = "";
  function lesen(id: string) {
    try {
      const v = JSON.parse(
        localStorage.getItem(`bauprojekt.navigation.v1.${id}`) ?? "{}",
      );
      return v && typeof v === "object" && !Array.isArray(v)
        ? (Object.fromEntries(
            Object.entries(v).filter(([, b]) => typeof b === "boolean"),
          ) as Record<string, boolean>)
        : {};
    } catch {
      return {};
    }
  }
  $effect(() => {
    const id = projekt?.id ?? "",
      a = ansicht;
    untrack(() => {
      if (id !== projektId) {
        projektId = id;
        offen = { grundlagen: true, ...lesen(id) };
        letzteAnsicht = "";
      }
      if (a !== letzteAnsicht) {
        letzteAnsicht = a;
        for (const z of NAVIGATION) {
          if (enthaelt(z, a)) {
            offen[z.id] = true;
            for (const k of z.kinder ?? [])
              if (enthaelt(k, a)) offen[k.id] = true;
          }
        }
      }
    });
  });
  function merken(pid: string, id: string, wert: boolean) {
    if (projektId !== pid) return;
    offen[id] = wert;
    if (projektId)
      try {
        localStorage.setItem(
          `bauprojekt.navigation.v1.${projektId}`,
          JSON.stringify(offen),
        );
      } catch {
        /* Aufklappzustand bleibt in der Sitzung erhalten. */
      }
  }
  function aktiv(id: string) {
    return (
      !globalAnsicht &&
      (ansicht === id || (ansicht === "mass" && id === "grundstueck"))
    );
  }
</script>

{#snippet eintraege(ids: import("./module").ModulId[])}
  {#each ids as id}
    {@const modul = MODULE.find((m) => m.id === id)!}
    {@const grund = voraussetzung(projekt!, modul)}
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
{#snippet gruppe(z: NavZweig, pid: string)}
  <details
    class="nav-zweig"
    open={offen[z.id] ?? false}
    ontoggle={(e) => merken(pid, z.id, e.currentTarget.open)}
  >
    <summary>{z.titel}</summary>
    <div>
      {@render eintraege(z.module)}{#each z.kinder ?? [] as k}{@render gruppe(
          k,
          pid,
        )}{/each}
    </div>
  </details>
{/snippet}

<aside class="aktennavigation" aria-label="Arbeitsbereiche">
  <div class="sidebar-scroll">
    <div class="nav-gruppe">Arbeitsplatz</div>
    <button
      class:aktiv={globalAnsicht === "projekte"}
      onclick={() => onglobal("projekte")}
      ><span class="nav-symbol"><NavIcon art="projekte" /></span
      >Projekte</button
    >
    <button
      class:aktiv={globalAnsicht === "kunden"}
      onclick={() => onglobal("kunden")}
      ><span class="nav-symbol"><NavIcon art="kunden" /></span>Kunden /
      Bauherren</button
    >
    <button
      class:aktiv={globalAnsicht === "formulare"}
      onclick={() => onglobal("formulare")}
      ><span class="nav-symbol" aria-hidden="true">▤</span
      >Formularbibliothek</button
    >
    {#if projekt}<div class="nav-gruppe projekt-nav-name" title={projekt.name}>
        {projekt.name}
      </div>
      <button
        class:aktiv={aktiv("projektuebersicht")}
        aria-current={aktiv("projektuebersicht") ? "page" : undefined}
        onclick={() => onbereich("projektuebersicht")}>Projektübersicht</button
      >
      {#each NAVIGATION as zweig}{@render gruppe(zweig, projekt.id)}{/each}{/if}
  </div>
  <div class="sidebar-system">
    <button
      class:aktiv={globalAnsicht === "verwaltung"}
      onclick={() => onglobal("verwaltung")}
      ><span class="nav-symbol" aria-hidden="true">⚙</span>Einstellungen</button
    >
  </div>
</aside>

<style>
  .aktennavigation {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .sidebar-scroll {
    overflow-y: auto;
    min-height: 0;
    flex: 1;
  }
  .sidebar-system {
    flex-shrink: 0;
  }
  .sidebar-scroll > button,
  .nav-zweig > div > button,
  .sidebar-system > button {
    display: block;
    width: 100%;
    text-align: left;
  }
  .nav-symbol {
    vertical-align: middle;
  }
  .nav-zweig {
    margin: 0.15rem 0;
  }
  .nav-zweig summary {
    padding: 0.55rem 0.7rem;
    font-weight: 650;
    font-size: 0.83rem;
    cursor: pointer;
    color: var(--muted);
  }
  .nav-zweig > div {
    margin-left: 0.8rem;
    padding-left: 0.4rem;
    border-left: 1px solid var(--border);
  }
  .nav-symbol {
    width: 1.4rem;
    display: inline-block;
    color: var(--muted);
  }
  .sidebar-system {
    border-top: 1px solid var(--border);
    margin-top: 1.2rem;
    padding-top: 0.65rem;
  }
  .nav-zweig button {
    white-space: normal;
    overflow-wrap: anywhere;
    line-height: 1.4;
  }
  .nav-zweig .nav-zweig > div {
    margin-left: 0.2rem;
  }
  .projekt-nav-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
