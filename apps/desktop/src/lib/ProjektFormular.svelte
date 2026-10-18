<script lang="ts">
  import { untrack } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import {
    LAENDER,
    landName,
    kundenname,
    type Kunde,
    type Projektwerte as Werte,
  } from "./api";
  import Abweichungen from "./Abweichungen.svelte";
  import { abgleichen, pfadSetzen, type Abweichung } from "./abgleich";

  let {
    start,
    kunden = null,
    kundenLadefehler = null,
    absenden,
    knopf,
    fassungVerfolgen = false,
    onabbrechen,
  }: {
    start: Werte;
    /** Kunden zur Auswahl des Bauherrn (nur beim Anlegen). */
    kunden?: Kunde[] | null;
    kundenLadefehler?: string | null;
    absenden: (w: Werte, bauherr: string | null) => Promise<void>;
    knopf: string;
    /** Gespeichertes Projekt: neue Fassungen von `start` als Grundlage abgleichen. */
    fassungVerfolgen?: boolean;
    onabbrechen: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  let werte = $state<Werte>({ ...start });
  /** Fassung, auf der die Eingaben beruhen. */
  // svelte-ignore state_referenced_locally
  let basis = $state<Werte>({ ...start });
  let bauherr = $state<string | null>(null);
  let laeuft = $state(false);
  let abweichungen = $state<Abweichung[]>([]);
  let neueFassung = $state(false);
  const FELDNAMEN: Record<string, string> = {
    name: "Projektname",
    bundesland: "Bundesland",
    grundstuecksflaeche_m2: "Grundstücksfläche",
  };

  // Kommt eine neue Fassung (z. B. nach einem Versionskonflikt geladen), wird
  // sie Grundlage. Eigene Eingaben bleiben stehen; nur fremd geänderte Felder
  // werden übernommen, auf beiden Seiten geänderte Felder werden angezeigt.
  $effect(() => {
    if (!fassungVerfolgen) return;
    const neu: Werte = { ...start };
    untrack(() => {
      const alt = $state.snapshot(basis) as Werte;
      if (JSON.stringify(neu) === JSON.stringify(alt)) return;
      const ergebnis = abgleichen(alt, $state.snapshot(werte) as Werte, neu);
      basis = neu;
      werte = ergebnis.werte;
      abweichungen = ergebnis.abweichungen;
      neueFassung = true;
    });
  });

  function aktuellUebernehmen(pfad: string) {
    const a = abweichungen.find((x) => x.pfad === pfad);
    if (!a) return;
    pfadSetzen(werte, pfad, a.aktuell);
    abweichungen = abweichungen.filter((x) => x.pfad !== pfad);
  }

  export function hatAenderungen() {
    return (
      JSON.stringify($state.snapshot(werte)) !==
        JSON.stringify($state.snapshot(basis)) || bauherr !== null
    );
  }
  export function istBeschaeftigt() {
    return laeuft;
  }
  /** Rückfrage vor dem Verwerfen ungespeicherter Eingaben. */
  export async function schliessenErlaubt() {
    return (
      !laeuft &&
      (!hatAenderungen() ||
        (await ask("Die Eingaben wurden noch nicht gespeichert. Verwerfen?", {
          title: knopf === "Anlegen" ? "Neues Projekt" : "Projektdaten",
          kind: "warning",
          okLabel: "Verwerfen",
          cancelLabel: "Weiter bearbeiten",
        })))
    );
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (laeuft) return;
    laeuft = true;
    try {
      await absenden(
        {
          ...werte,
          name: werte.name.trim(),
          grundstuecksflaeche_m2: werte.grundstuecksflaeche_m2 || null,
        },
        bauherr,
      );
    } finally {
      laeuft = false;
    }
  }
</script>

<form class="spalte" onsubmit={submit}>
  <label>
    Projektname
    <!-- svelte-ignore a11y_autofocus -->
    <input bind:value={werte.name} required autofocus />
  </label>
  <label>
    Bundesland
    <select bind:value={werte.bundesland}>
      {#each LAENDER as l}<option value={l.code}>{l.name}</option>{/each}
    </select>
  </label>
  <p class="meta">
    Flurstücke mit ihren Flächen und optionalen Grundbuchangaben werden in der
    Projektakte unter Grundstücksinformationen erfasst. Die Baugrundstücksfläche
    wird separat berechnet.
  </p>
  {#if neueFassung}
    <p class="hinweisbox" role="status">
      Das Projekt wurde inzwischen an einem anderen Arbeitsplatz geändert. Die
      aktuelle Fassung ({landName(basis.bundesland)}, „{basis.name}“) ist jetzt
      Grundlage; Ihre Eingaben bleiben im Formular. Bitte prüfen und erneut
      speichern.
    </p>
    <Abweichungen
      {abweichungen}
      titel={(pfad) => FELDNAMEN[pfad] ?? pfad}
      gesperrt={laeuft}
      onuebernehmen={aktuellUebernehmen}
    />
  {/if}
  {#if kundenLadefehler}
    <p class="fehlertext" role="alert">Bauherren konnten nicht geladen werden: {kundenLadefehler} Sie können das Projekt anlegen und den Bauherrn später zuordnen.</p>
  {/if}
  {#if kunden}
    <label>
      Bauherr
      <select bind:value={bauherr}>
        <option value={null}>– später zuordnen –</option>
        {#each kunden as k (k.id)}<option value={k.id}
            >{kundenname(k)}{k.kundennummer
              ? ` (${k.kundennummer})`
              : ""}</option
          >{/each}
      </select>
    </label>
  {/if}
  <div class="knoepfe">
    <button
      type="button"
      class="sekundaer"
      onclick={async () => {
        if (await schliessenErlaubt()) onabbrechen();
      }}>Abbrechen</button
    >
    <button type="submit" disabled={laeuft}>{knopf}</button>
  </div>
</form>
