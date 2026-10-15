<script lang="ts">
  import { api, fehlertext } from "./api";
  import type { Ausgabeauftrag } from "./generated/plattform";
  import { editorAnmelden } from "./editoren";
  let {
    projektId,
    darf,
    stand,
  }: { projektId: string; darf: boolean; stand: number } = $props();
  let liste = $state<Ausgabeauftrag[]>([]),
    laeuft = $state(false),
    fehler = $state<string | null>(null),
    meldung = $state<string | null>(null),
    ziel = $state(""),
    erledigte = $state(false);
  async function laden() {
    laeuft = true;
    try {
      [liste, ziel] = await Promise.all([
        api.ausgabeauftraege(projektId),
        api.projektordnerStatus(),
      ]);
      fehler = null;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  $effect(() => {
    void stand;
    void laden();
  });
  async function wiederholen(a: Ausgabeauftrag) {
    laeuft = true;
    try {
      const d = await api.ausgabeWiederholen(projektId, a.id);
      meldung = d.ablage_hinweis ?? `Datei ausgegeben: ${d.projektordner_pfad}`;
      await laden();
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  editorAnmelden("dokumente", {
    hatAenderungen: () => false,
    istBeschaeftigt: () => laeuft,
    speichern: async () => !laeuft,
  });
</script>

<details>
  <summary
    >Ordnerausgaben · {liste.filter((a) => a.status !== "fertig").length} offen /
    fehlgeschlagen</summary
  >
  <p class="meta">
    Bestätigte Dokumentversionen bleiben im DMS erhalten. Ausgaben werden nach
    einem Neustart fortgesetzt; fremd geänderte Dateien werden nicht
    überschrieben.
  </p>
  <div class="zeile">
    <button class="link" disabled={laeuft} onclick={laden}
      >Aufträge aktualisieren</button
    ><label
      ><input type="checkbox" bind:checked={erledigte} />Erledigte Ausgaben
      anzeigen</label
    >
  </div>
  {#if fehler}<p class="fehlertext" role="alert">
      {fehler}
    </p>{/if}{#if meldung}<p role="status">{meldung}</p>{/if}
  <div class="tabellen-scroll">
    <table class="ergebnistabelle">
      <thead
        ><tr><th>Dateiausgabe</th><th>Status</th><th>Versuche</th><th></th></tr
        ></thead
      ><tbody
        >{#each liste.filter((a) => erledigte || a.status !== "fertig") as a (a.id)}<tr
            ><td class="pfad"
              >{a.relativer_pfad}
              <p class="meta pfad">
                {a.ziel_root}{a.ziel_root !== ziel
                  ? " · früheres Ausgabeziel"
                  : ""}
              </p></td
            ><td
              >{a.status === "fertig"
                ? "Ausgegeben"
                : a.status === "laeuft"
                  ? "In Bearbeitung"
                  : "Erneuter Versuch vorgesehen"}{#if a.fehler}<p
                  class="fehlertext"
                >
                  {a.fehler}
                </p>{/if}</td
            ><td>{a.versuche}</td><td
              ><button
                class="link"
                disabled={!darf || laeuft}
                onclick={() => wiederholen(a)}
                >Am aktuellen Ziel ausgeben</button
              ></td
            ></tr
          >{:else}<tr><td colspan="4">Keine offenen Ausgabeaufträge.</td></tr
          >{/each}</tbody
      >
    </table>
  </div>
</details>

<style>
  .pfad {
    overflow-wrap: anywhere;
    max-width: 36rem;
  }
</style>
