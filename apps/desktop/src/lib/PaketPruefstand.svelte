<script lang="ts">
  import type { PaketPruefstand } from "./api";
  let { stand }: { stand: PaketPruefstand | null | undefined } = $props();
</script>

{#if stand}<aside class="hinweisbox">
    <h4>Prüfstand bei Paketerstellung</h4>
    <p class="meta">
      {new Date(stand.ermittelt_am).toLocaleString("de-DE")}. Ein
      Nachreichungspaket darf einen Teil der erforderlichen Unterlagen
      enthalten.
    </p>
    {#each stand.hinweise as h}<p>
        {h}
      </p>{/each}{#if stand.nicht_enthaltene_pflichtunterlagen.length}<p>
        Erforderliche Unterlagen, die in diesem Paket nicht enthalten sind:
      </p>
      <ul>
        {#each stand.nicht_enthaltene_pflichtunterlagen as h}<li>{h}</li>{/each}
      </ul>{/if}{#if stand.ungepruefte_unterlagen.length}<p>
        Enthaltene Unterlagen ohne Freigabe: {stand.ungepruefte_unterlagen.join(
          ", ",
        )}
      </p>{/if}{#each stand.nachweise.filter((n) => !n.aktuell || !n.freigabe || !n.stand.vollstaendig) as n}<p
      >
        {n.stand.bezeichnung}: {!n.aktuell
          ? "Grundlagen geändert"
          : !n.stand.vollstaendig
            ? "unvollständig"
            : "ohne Freigabe"}
      </p>{/each}
  </aside>{:else}<p class="meta">
    Für diesen älteren Paketstand wurde noch kein Prüfstand erfasst.
  </p>{/if}
