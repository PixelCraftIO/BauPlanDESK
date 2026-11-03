<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { fehlertext, type Project } from "../api";
  import type {
    Ampel,
    Einreichungspruefung,
    Pruefbereich,
    Verfahrensakte,
  } from "../generated/plattform";
  import { plattformApi } from "../plattformApi";
  import { projektNavigation } from "../projektNavigation.svelte";
  import { ansichtsAbfrage } from "../ansichtsAbfrage.svelte";
  let { projekt }: { projekt: Project; benutzerId: string; darf: boolean } =
    $props();
  const navigation = projektNavigation();
  const AMPEL: Record<Ampel, string> = {
    rot: "Vor dem Einreichen erledigen",
    gelb: "Prüfen",
    gruen: "In Ordnung",
  };
  let stand = $state<Einreichungspruefung | null>(null),
    verfahren = $state<{ id: string; bezeichnung: string }[]>([]),
    auswahl = $state<string | null>(null),
    filter = $state<Pruefbereich | null>(null),
    fehler = $state<string | null>(null),
    laeuft = $state(false);
  const punkte = $derived(
    (stand?.punkte ?? []).filter((p) => !filter || p.bereich === filter),
  );
  const datum = (d: string) =>
    new Date(d + "T12:00:00").toLocaleDateString("de-DE");
  async function laden() {
    laeuft = true;
    fehler = null;
    try {
      stand = await invoke<Einreichungspruefung>("einreichungspruefung", {
        projektId: projekt.id,
        verfahren: auswahl,
      });
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  ansichtsAbfrage(() => projekt.id, ["fachobjekte", "dokumente"], laden);
  onMount(async () => {
    try {
      const v = await plattformApi.liste<Verfahrensakte>(projekt.id, "verfahren");
      verfahren = v.map((f) => ({ id: f.objekt, bezeichnung: f.daten.bezeichnung }));
    } catch (e) {
      fehler = fehlertext(e);
    }
  });
</script>

<section class="einreichungspruefung" aria-busy={laeuft}>
  <h2>Einreichungsprüfung</h2>
  <p class="meta">
    Eine Liste vor dem Absenden: Bauvorlagen, Nachweise, Pläne mit Index,
    Formulare, Lieferungen der Fachplaner und offene Nachforderungen. Rot ist vor
    dem Einreichen zu erledigen, Gelb bewusst zu prüfen. Ein Klick auf einen
    Punkt öffnet die Stelle in der Akte.
  </p>
  <div class="kopf">
    <label
      >Verfahren<select bind:value={auswahl} onchange={laden}
        ><option value={null}>Alle Verfahren des Projekts</option
        >{#each verfahren as v (v.id)}<option value={v.id}>{v.bezeichnung}</option
          >{/each}</select
      ></label
    >
    <button type="button" class="sekundaer" onclick={laden} disabled={laeuft}
      >Neu prüfen</button
    >
    {#if stand}<span class="gesamt ampel-{stand.gesamt}" role="status"
        >Gesamt: {AMPEL[stand.gesamt]}</span
      >{/if}
  </div>
  {#if fehler}<p role="alert">{fehler}</p>{/if}
  {#if stand}
    <ul class="bereiche" aria-label="Bereiche">
      {#each stand.bereiche as b (b.bereich)}
        <li>
          <button
            type="button"
            class="bereich ampel-{b.ampel}"
            aria-pressed={filter === b.bereich}
            onclick={() => (filter = filter === b.bereich ? null : b.bereich)}
          >
            <span class="punkt" aria-hidden="true"></span>
            <span>{b.bezeichnung}</span>
            <small
              >{b.rot || b.gelb
                ? `${b.rot ? `${b.rot} rot` : ""}${b.rot && b.gelb ? " · " : ""}${b.gelb ? `${b.gelb} gelb` : ""}`
                : "in Ordnung"}</small
            >
          </button>
        </li>
      {/each}
    </ul>
    {#if punkte.length}
      <table>
        <thead
          ><tr><th>Ampel</th><th>Bereich</th><th>Punkt</th><th>Hinweis</th><th
              >Termin</th
            ></tr
          ></thead
        >
        <tbody>
          {#each punkte as p (p.id + p.hinweis)}
            <tr class="ampel-{p.ampel}">
              <td><span class="punkt" title={AMPEL[p.ampel]}></span><span class="sr"
                  >{AMPEL[p.ampel]}</span
                ></td>
              <td>{stand.bereiche.find((b) => b.bereich === p.bereich)?.bezeichnung}</td>
              <td
                ><button
                  type="button"
                  class="link"
                  onclick={() => navigation?.springen(p.ziel)}>{p.titel}</button
                ></td
              >
              <td>{p.hinweis}</td>
              <td>{p.faellig ? datum(p.faellig) : ""}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}<p class="meta">Keine offenen Punkte{filter ? " in diesem Bereich" : ""}.</p>{/if}
  {/if}
</section>

<style>
  .kopf {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }
  .gesamt {
    padding: 0.2rem 0.6rem;
    border: 1px solid var(--border);
    font-weight: 600;
  }
  .bereiche {
    list-style: none;
    padding: 0;
    margin: 0 0 var(--space-3);
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(12rem, 1fr));
    gap: var(--space-2);
  }
  .bereich {
    width: 100%;
    display: grid;
    grid-template-columns: auto 1fr;
    column-gap: var(--space-2);
    text-align: left;
    padding: var(--space-2);
    border: 1px solid var(--border);
    background: var(--panel);
    color: var(--fg);
  }
  .bereich small {
    grid-column: 2;
  }
  .bereich[aria-pressed="true"] {
    outline: 2px solid var(--accent);
  }
  .punkt {
    display: inline-block;
    width: 0.8rem;
    height: 0.8rem;
    border-radius: 50%;
    border: 1px solid var(--border);
    align-self: center;
  }
  .ampel-rot .punkt,
  .gesamt.ampel-rot {
    background: var(--error-text);
  }
  .ampel-gelb .punkt {
    background: #e0b000;
  }
  .ampel-gruen .punkt {
    background: var(--ok-text);
  }
  .gesamt.ampel-rot {
    color: #fff;
  }
  .gesamt.ampel-gelb {
    background: var(--warn-bg);
    color: var(--warning-text);
  }
  .gesamt.ampel-gruen {
    background: var(--ok-bg);
    color: var(--ok-text);
  }
  tr.ampel-rot td {
    background: var(--err-bg);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
