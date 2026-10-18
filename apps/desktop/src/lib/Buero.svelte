<script lang="ts">
  import {
    LAENDER,
    api,
    fehlertext,
    leereAnschrift,
    mindestens,
    type Benutzer,
    type Buero,
  } from "./api";
  import AnschriftFelder from "./AnschriftFelder.svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { bueroGeaendert } from "./projektEreignisse";

  let { ich, onschliessen }: { ich: Benutzer; onschliessen: () => void } = $props();

  const darf = $derived(ich.aktiv && mindestens(ich.rolle, "projektleitung"));
  let buero = $state<Buero>({
    name: "",
    anschrift: leereAnschrift(),
    kennung: "",
    entwurfsverfasser: [],
  });
  let fehler = $state<string | null>(null);
  let laeuft = $state(false);
  /** Geladener Stand für die Erkennung ungespeicherter Änderungen. */
  let geladen = $state<string | null>(null);

  $effect(() => {
    api
      .buero()
      .then((b) => {
        buero = { ...b, entwurfsverfasser: b.entwurfsverfasser ?? [] };
        geladen = JSON.stringify(buero);
      })
      .catch((e) => (fehler = fehlertext(e)));
  });

  export function hatAenderungen() {
    return darf && geladen !== null && JSON.stringify(buero) !== geladen;
  }
  export function istBeschaeftigt() {
    return laeuft;
  }
  /** Rückfrage vor dem Verwerfen (Escape, Abbrechen, Menübefehle). */
  export async function schliessenErlaubt() {
    return (
      !laeuft &&
      (!hatAenderungen() ||
        (await ask("Die Büroangaben wurden noch nicht gespeichert. Verwerfen?", {
          title: "Büroangaben",
          kind: "warning",
          okLabel: "Verwerfen",
          cancelLabel: "Weiter bearbeiten",
        })))
    );
  }

  function personHinzufuegen() {
    buero.entwurfsverfasser.push({
      id: crypto.randomUUID(),
      vorname: "",
      familienname: "",
      berufsbezeichnung: "",
      mitgliedsnummer: "",
      kammerland: null,
      nachweis: "",
    });
  }

  async function speichern(e: SubmitEvent) {
    e.preventDefault();
    laeuft = true;
    fehler = null;
    try {
      await api.bueroSpeichern(buero);
      geladen = JSON.stringify(buero);
      bueroGeaendert();
      onschliessen();
    } catch (err) {
      fehler = fehlertext(err);
    } finally {
      laeuft = false;
    }
  }
</script>

<form class="spalte" onsubmit={speichern}>
  <p class="meta">
    Das Büro ist Absender jedes XBau-Bauantrags. Als Entwurfsverfasser zeichnet
    eine der bauvorlageberechtigten Personen unten; der Bauantrag wählt sie aus.
  </p>
  <label>Name des Büros
    <span class="pflicht" aria-hidden="true"> *</span><input aria-label="Name des Büros" bind:value={buero.name} disabled={!darf} required />
  </label>
  <AnschriftFelder bind:anschrift={buero.anschrift} gesperrt={!darf} />
  <label>Absenderkennung
    <input bind:value={buero.kennung} disabled={!darf} placeholder="z. B. vom Antragsportal vergeben" />
  </label>

  <fieldset disabled={!darf}>
    <legend>Bauvorlageberechtigte Personen</legend>
    <p class="meta">
      Die Berechtigung belegt die Mitgliedsnummer der Architekten- oder
      Ingenieurkammer oder ein Nachweis nach Landesrecht.
    </p>
    {#each buero.entwurfsverfasser as p, i (p.id)}
      <div class="spalte person" role="group" aria-label={`Person ${i + 1}`}>
        <div class="feldzeile">
          <label>Vorname <input bind:value={p.vorname} /></label>
          <label>Familienname <input bind:value={p.familienname} /></label>
          <label>Berufsbezeichnung
            <input bind:value={p.berufsbezeichnung} placeholder="z. B. Architektin" />
          </label>
        </div>
        <div class="feldzeile">
          <label>Kammer-Mitgliedsnummer <input bind:value={p.mitgliedsnummer} /></label>
          <label>Land der Kammer
            <select bind:value={p.kammerland}>
              <option value={null}>–</option>
              {#each LAENDER as l (l.code)}<option value={l.code}>{l.name}</option>{/each}
            </select>
          </label>
        </div>
        <label>Nachweis ohne Kammermitgliedschaft
          <textarea rows="2" bind:value={p.nachweis}></textarea>
        </label>
        <button
          type="button"
          class="sekundaer"
          onclick={() => buero.entwurfsverfasser.splice(i, 1)}>Person entfernen</button
        >
      </div>
    {:else}
      <p class="meta">Noch keine Person eingetragen.</p>
    {/each}
    <button type="button" class="sekundaer" onclick={personHinzufuegen}>Person hinzufügen</button>
  </fieldset>

  {#if !darf}<p class="meta">Büroangaben ändern kann die Projektleitung.</p>{/if}
  {#if fehler}<p class="fehlertext">{fehler}</p>{/if}
  <div class="knoepfe">
    <button
      type="button"
      class="sekundaer"
      onclick={async () => {
        if (await schliessenErlaubt()) onschliessen();
      }}>{darf ? "Abbrechen" : "Schließen"}</button
    >
    {#if darf}<button type="submit" disabled={laeuft}>Speichern</button>{/if}
  </div>
</form>

<style>
  .person {
    border-top: 1px solid var(--border);
    padding-top: 0.6rem;
    margin-bottom: 0.6rem;
  }
  .feldzeile {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
  }
  .feldzeile > label {
    flex: 1 1 10rem;
  }
</style>
