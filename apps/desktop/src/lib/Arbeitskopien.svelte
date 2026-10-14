<script lang="ts">
  import { onMount } from "svelte";
  import { api, fehlertext, type Arbeitskopie } from "./api";
  let {
    projektId,
    darfAblegen,
    onaktualisiert,
    onfehler,
  }: {
    projektId: string;
    darfAblegen: boolean;
    onaktualisiert: () => Promise<void>;
    onfehler: (m: string) => void;
  } = $props();
  let kopien = $state<Arbeitskopie[]>([]),
    laeuft = $state(false),
    hinweis = $state<string | null>(null);
  export function istBeschaeftigt() {
    return laeuft;
  }
  export async function aktualisieren() {
    try {
      kopien = await api.arbeitskopien(projektId);
    } catch (e) {
      onfehler(fehlertext(e));
    }
  }
  onMount(aktualisieren);
  async function oeffnen(k: Arbeitskopie) {
    laeuft = true;
    try {
      await api.arbeitskopieOeffnen(k.stand.id);
    } catch (e) {
      onfehler(fehlertext(e));
    } finally {
      laeuft = false;
    }
  }
  async function uebernehmen(k: Arbeitskopie) {
    laeuft = true;
    try {
      const v = await api.arbeitskopieUebernehmen(k.stand.id);
      hinweis = `${v.file_name} als Version ${v.version} übernommen.`;
      await onaktualisiert();
    } catch (e) {
      onfehler(fehlertext(e));
    } finally {
      laeuft = false;
      await aktualisieren();
    }
  }
</script>

<section>
  <div class="knoepfe links">
    <h3>Externe Arbeitskopien</h3>
    <button class="sekundaer" disabled={laeuft} onclick={aktualisieren}
      >Änderungen prüfen</button
    >
  </div>
  <p class="meta">
    Im zugeordneten Programm speichern, anschließend Änderungen prüfen und als
    neue Dokumentversion übernehmen. Die bisherigen Versionen bleiben erhalten.
    Arbeitskopien bleiben auch nach einem Neustart auf diesem Gerät verfügbar.
  </p>
  {#if hinweis}<p role="status">{hinweis}</p>{/if}
  {#if !kopien.length}<p class="meta">
      Noch keine externen Arbeitskopien für dieses Projekt.
    </p>{:else}
    <table>
      <thead
        ><tr
          ><th>Datei / Arbeitskopie</th><th>Basisversion</th><th>Stand</th><th
          ></th></tr
        ></thead
      ><tbody>
        {#each kopien as k (k.stand.id)}<tr
            ><td
              >{k.stand.dokument.file_name}<small class="meta">{k.pfad}</small
              ></td
            ><td>{k.stand.dokument.version}</td><td
              >{k.geaendert ? "Geändert" : "Unverändert"}</td
            ><td
              ><button class="link" disabled={laeuft} onclick={() => oeffnen(k)}
                >Arbeitskopie öffnen</button
              ><button
                class="link"
                disabled={laeuft || !darfAblegen || !k.geaendert}
                onclick={() => uebernehmen(k)}
                >Als neue Version übernehmen</button
              ></td
            ></tr
          >{/each}
      </tbody>
    </table>{/if}
</section>

<style>
  table {
    width: 100%;
  }
  th {
    text-align: left;
  }
  small {
    display: block;
    overflow-wrap: anywhere;
  }
  section {
    margin-top: 1.5rem;
  }
</style>
