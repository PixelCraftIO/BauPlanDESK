<script lang="ts">
  // Stand des Projekts von der Vorbereitung bis zum Abschluss. Die Stufen bis
  // „Fertiggestellt“ ergeben sich aus den Verfahrensakten; den Abschluss
  // setzt das Büro.
  import { ask } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext } from "./api";
  import type { Projektstand, Projektstufe } from "./generated/plattform";
  let {
    projektId,
    aktualisieren = 0,
    darf = true,
  }: {
    projektId: string;
    /** Neu laden, wenn sich der Wert ändert. */
    aktualisieren?: number | string;
    darf?: boolean;
  } = $props();
  const STUFEN: [Projektstufe, string][] = [
    ["vorbereitung", "Vorbereitung"],
    ["eingereicht", "Eingereicht"],
    ["genehmigt", "Genehmigt"],
    ["im_bau", "Im Bau"],
    ["fertiggestellt", "Fertiggestellt"],
    ["abgeschlossen", "Abgeschlossen"],
  ];
  let stand = $state<Projektstand | null>(null),
    fehler = $state<string | null>(null),
    laeuft = $state(false);
  let anfrage = 0;
  async function laden() {
    const nr = ++anfrage;
    try {
      const neu = await api.projektstand(projektId);
      if (nr === anfrage) stand = neu;
    } catch {
      // Ohne Auskunft bleibt die Leiste ausgeblendet; die Übersicht selbst
      // meldet Ladefehler.
      if (nr === anfrage) stand = null;
    }
  }
  $effect(() => {
    void aktualisieren;
    void projektId;
    void laden();
  });
  /** Position der Stufe in der Leiste; „abgelehnt“ steht auf „Eingereicht“. */
  const position = (s: Projektstufe) =>
    STUFEN.findIndex(([id]) => id === (s === "abgelehnt" ? "eingereicht" : s));
  const erreicht = $derived(stand ? position(stand.stufe) : -1);
  const datum = (s: string | null) =>
    s
      ? new Date(`${s}T12:00:00`).toLocaleDateString("de-DE", {
          day: "2-digit",
          month: "2-digit",
          year: "numeric",
        })
      : "";
  async function abschliessen(abgeschlossen: boolean) {
    if (
      abgeschlossen &&
      !(await ask(
        "Das Projekt wird als abgeschlossen geführt und lässt sich im Projektverzeichnis ausblenden. Alle Daten bleiben erhalten und bearbeitbar; der Abschluss lässt sich zurücknehmen.",
        {
          title: "Projekt abschließen",
          okLabel: "Abschließen",
          cancelLabel: "Abbrechen",
        },
      ))
    )
      return;
    laeuft = true;
    fehler = null;
    try {
      stand = await api.projektAbschliessen(projektId, abgeschlossen);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
</script>

{#if stand}
  <section class="projektstand" aria-label="Projektstand">
    <ol>
      {#each STUFEN as [id, name], i (id)}
        <li
          class:erreicht={i < erreicht}
          class:aktuell={i === erreicht}
          aria-current={i === erreicht ? "step" : undefined}
        >
          <span class="marke" aria-hidden="true"
            >{i < erreicht ? "✓" : i + 1}</span
          >
          <span
            >{i === erreicht && stand.stufe === "abgelehnt"
              ? "Abgelehnt"
              : name}{#if i === erreicht && stand.seit}<span class="meta"
                >{" "}· seit {datum(stand.seit)}</span
              >{/if}</span
          >
        </li>
      {/each}
    </ol>
    {#if stand.stufe === "abgeschlossen"}
      <button
        class="sekundaer"
        disabled={!darf || laeuft}
        onclick={() => abschliessen(false)}>Projekt wieder öffnen</button
      >
    {:else}
      <button
        class="sekundaer"
        disabled={!darf || laeuft}
        title="Das Projekt als abgeschlossen führen; alle Daten bleiben erhalten."
        onclick={() => abschliessen(true)}>Projekt abschließen …</button
      >
    {/if}
    {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  </section>
{/if}

<style>
  .projektstand {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    padding: 8px 0;
    border: 0;
    margin: 0;
  }
  .projektstand > button {
    margin-left: auto;
  }
  ol {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 0;
    list-style: none;
    margin: 0;
    padding: 0;
    flex: 0 1 auto;
  }
  li {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 14px 4px 0;
    color: var(--muted);
  }
  li + li::before {
    content: "";
    width: 18px;
    border-top: 1px solid var(--border);
    margin-right: 8px;
  }
  .marke {
    display: inline-grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border: 1px solid var(--border);
    border-radius: 50%;
    font-size: 0.8rem;
    background: var(--panel);
  }
  li.erreicht {
    color: var(--fg);
  }
  li.erreicht .marke {
    background: var(--ok-bg);
    color: var(--ok-text);
    border-color: var(--ok-text);
  }
  li.aktuell {
    color: var(--fg);
    font-weight: 600;
  }
  li.aktuell .marke {
    background: var(--action-bg);
    color: var(--on-action);
    border-color: var(--action-bg);
  }
  li .meta {
    font-weight: 400;
  }
  .fehlertext {
    flex-basis: 100%;
    margin: 0;
  }
</style>
