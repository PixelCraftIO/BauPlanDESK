<script lang="ts">
  import { onMount } from "svelte";
  import {
    api,
    fehlertext,
    mindestens,
    type Benutzer,
    type KategorienKatalog,
  } from "./api";
  import { kategorienOptionen } from "./kategorien";
  let { ich }: { ich: Benutzer | null } = $props();
  let katalog = $state<KategorienKatalog | null>(null),
    laeuft = $state(false),
    fehler = $state(""),
    meldung = $state("");
  let bearbeitung = $state<string | null>(null),
    name = $state(""),
    eltern = $state(""),
    aktiv = $state(true);
  const darf = $derived(
    !!ich?.aktiv && mindestens(ich.rolle, "projektleitung"),
  );
  async function laden() {
    laeuft = true;
    try {
      katalog = await api.kategorienkatalog();
      fehler = "";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  onMount(laden);
  export function istBeschaeftigt() {
    return laeuft || bearbeitung !== null;
  }
  function bearbeiten(id: string) {
    const k = katalog?.kategorien.find((k) => k.id === id);
    bearbeitung = id;
    name = k?.name ?? "";
    eltern = k?.eltern_id ?? "";
    aktiv = k?.aktiv ?? true;
    fehler = "";
    meldung = "";
  }
  async function speichern(e: SubmitEvent) {
    e.preventDefault();
    if (!katalog || bearbeitung === null) return;
    laeuft = true;
    fehler = "";
    const id = bearbeitung || `eigen_${crypto.randomUUID()}`;
    const k = { id, name: name.trim(), eltern_id: eltern || null, aktiv };
    const kategorien = katalog.kategorien.some((v) => v.id === id)
      ? katalog.kategorien.map((v) => (v.id === id ? k : v))
      : [...katalog.kategorien, k];
    try {
      katalog = await api.kategorienkatalogSpeichern({
        ...katalog,
        kategorien,
      });
      bearbeitung = null;
      meldung = "Kategorienkatalog gespeichert.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
</script>

<details class="kategorien-einstellungen">
  <summary>Dokumentkategorien</summary>
  <p class="meta">
    Gemeinsamer Katalog für alle Projekte und Formularvorlagen. Archivierte
    Kategorien bleiben an bestehenden Unterlagen erhalten. Einreichungspakete
    bewahren ihre damaligen Bezeichnungen.
  </p>
  {#if fehler}<p role="alert" class="fehlertext">{fehler}</p>{/if}
  {#if meldung}<p role="status">{meldung}</p>{/if}
  <div class="knoepfe links">
    <button
      class="sekundaer"
      disabled={!darf || laeuft || bearbeitung !== null}
      onclick={() => bearbeiten("")}>Kategorie hinzufügen</button
    ><button
      class="sekundaer"
      disabled={laeuft || bearbeitung !== null}
      onclick={laden}>Katalog aktualisieren</button
    >
  </div>
  {#if bearbeitung !== null}<form onsubmit={speichern}>
      <fieldset disabled={laeuft || !darf}>
        <legend
          >{bearbeitung ? "Kategorie bearbeiten" : "Neue Kategorie"}</legend
        >
        <div class="raster">
          <label>Name<input bind:value={name} required maxlength="120" /></label
          >
          <label
            >Hauptkategorie<select
              aria-label="Hauptkategorie"
              bind:value={eltern}
              ><option value="">Eigenständige Hauptkategorie</option
              >{#each katalog?.kategorien.filter((k) => !k.eltern_id && k.id !== bearbeitung && k.aktiv) ?? [] as k}<option
                  value={k.id}>{k.name}</option
                >{/each}</select
            ></label
          >
          <label class="zeile"
            ><input type="checkbox" bind:checked={aktiv} />Aktiv</label
          >
        </div>
        <p class="meta">
          Vor dem Archivieren einer Hauptkategorie deren Unterkategorien
          archivieren oder anders zuordnen.
        </p>
        <div class="knoepfe">
          <button type="submit">Kategorie speichern</button><button
            class="sekundaer"
            type="button"
            onclick={() => (bearbeitung = null)}>Abbrechen</button
          >
        </div>
      </fieldset>
    </form>{/if}
  <table class="ergebnistabelle" aria-label="Dokumentkategorien">
    <thead><tr><th>Kategorie</th><th>Status</th><th></th></tr></thead><tbody>
      {#each kategorienOptionen(katalog?.kategorien ?? []) as k}<tr
          ><td>{k.bezeichnung}</td><td>{k.aktiv ? "Aktiv" : "Archiviert"}</td
          ><td
            ><button
              class="link"
              disabled={!darf || laeuft || bearbeitung !== null}
              onclick={() => bearbeiten(k.id)}>Bearbeiten</button
            ></td
          ></tr
        >{/each}
    </tbody>
  </table>
</details>

<style>
  .kategorien-einstellungen {
    padding: 1rem 0;
    border-bottom: 1px solid var(--border);
  }
  summary {
    cursor: pointer;
    font-weight: 600;
  }
</style>
