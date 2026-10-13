<script lang="ts">
  import {
    anschriftZeile,
    api,
    fehlertext,
    istKonflikt,
    kategorieName,
    kundenname,
    kundePasst,
    leereKundendaten,
    mindestens,
    type Benutzer,
    type Kunde,
    type Kundendaten,
    type Project,
  } from "./api";
  import KundeFelder from "./KundeFelder.svelte";

  let {
    projekt,
    ich,
    onprojekt,
    onschliessen,
  }: {
    projekt: Project;
    ich: Benutzer;
    /** Gespeichert: das Projekt mit neuer Version. */
    onprojekt: (p: Project) => void;
    onschliessen: () => void;
  } = $props();

  const darf = $derived(ich.aktiv && mindestens(ich.rolle, "bearbeitung"));

  let alle = $state<Kunde[]>([]);
  let gespeichert = $state<string[]>([]);
  let gewaehlt = $state<Kunde[]>([]);
  let suche = $state("");
  let neu = $state<Kundendaten | null>(null);
  let fehler = $state<string | null>(null);
  let laeuft = $state(false);

  const vorschlaege = $derived(
    alle.filter((k) => !gewaehlt.some((g) => g.id === k.id) && kundePasst(k, suche)).slice(0, 50),
  );
  const geaendert = $derived(gewaehlt.map((k) => k.id).join() !== gespeichert.join());

  async function laden() {
    try {
      const [kunden, bauherren] = await Promise.all([api.kunden(), api.bauherren(projekt.id)]);
      alle = kunden;
      gewaehlt = bauherren;
      gespeichert = bauherren.map((k) => k.id);
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
  $effect(() => {
    laden();
  });

  function verschieben(i: number, um: number) {
    const j = i + um;
    if (j < 0 || j >= gewaehlt.length) return;
    [gewaehlt[i], gewaehlt[j]] = [gewaehlt[j], gewaehlt[i]];
  }

  async function anlegen(e: SubmitEvent) {
    e.preventDefault();
    if (!neu) return;
    laeuft = true;
    fehler = null;
    try {
      const k = await api.kundeAnlegen(neu);
      alle = [...alle, k];
      gewaehlt = [...gewaehlt, k];
      neu = null;
    } catch (err) {
      fehler = fehlertext(err);
    } finally {
      laeuft = false;
    }
  }

  async function speichern() {
    laeuft = true;
    fehler = null;
    try {
      onprojekt(await api.bauherrenSetzen(projekt.id, gewaehlt.map((k) => k.id), projekt.version));
      onschliessen();
    } catch (e) {
      if (istKonflikt(e)) {
        fehler = "Das Projekt wurde inzwischen an einem anderen Arbeitsplatz geändert. Bitte den Dialog schließen und erneut öffnen.";
        onprojekt(await api.projektOeffnen(projekt.id));
      } else {
        fehler = fehlertext(e);
      }
    } finally {
      laeuft = false;
    }
  }
</script>

<div class="spalte">
  <p class="meta">Bauherren kommen aus der Kundendatenbank. Der erste Bauherr wird beim Bauantrag als Gebührenpflichtiger eingesetzt, wenn dort niemand anderes eingetragen ist.</p>

  <ul class="bauherren">
    {#each gewaehlt as k, i (k.id)}
      <li>
        <div>
          <strong>{i + 1}. {kundenname(k)}</strong>
          <span class="meta">{[kategorieName(k.kategorie), anschriftZeile(k.anschrift)].filter(Boolean).join(" · ")}</span>
        </div>
        {#if darf}
          <span class="werkzeuge">
            <button type="button" class="sekundaer" onclick={() => verschieben(i, -1)} disabled={i === 0} aria-label="Nach oben">↑</button>
            <button type="button" class="sekundaer" onclick={() => verschieben(i, 1)} disabled={i === gewaehlt.length - 1} aria-label="Nach unten">↓</button>
            <button type="button" class="sekundaer" onclick={() => gewaehlt.splice(i, 1)}>Entfernen</button>
          </span>
        {/if}
      </li>
    {:else}
      <li class="meta">Noch kein Bauherr zugeordnet.</li>
    {/each}
  </ul>

  {#if darf}
    {#if neu}
      <form class="spalte" onsubmit={anlegen}>
        <h3>Neuer Kunde</h3>
        <KundeFelder bind:daten={neu} />
        <div class="knoepfe links">
          <button type="submit" disabled={laeuft}>Anlegen und hinzufügen</button>
          <button type="button" class="sekundaer" onclick={() => (neu = null)}>Abbrechen</button>
        </div>
      </form>
    {:else}
      <!-- svelte-ignore a11y_autofocus -->
      <input placeholder="Kunden suchen und hinzufügen …" bind:value={suche} aria-label="Kunden suchen" autofocus />
      <ul class="auswahlliste" aria-label="Kunden">
        {#each vorschlaege as k (k.id)}
          <li>
            <button type="button" class="link" onclick={() => (gewaehlt = [...gewaehlt, k])}>{kundenname(k)}</button>
            <span class="meta">{[kategorieName(k.kategorie), k.kundennummer, anschriftZeile(k.anschrift)].filter(Boolean).join(" · ")}</span>
          </li>
        {:else}
          <li class="meta">{alle.length ? "Kein weiterer Kunde passt zur Suche." : "Die Kundendatenbank ist noch leer."}</li>
        {/each}
      </ul>
      <div>
        <button type="button" class="sekundaer" onclick={() => (neu = leereKundendaten())}>Neuen Kunden anlegen …</button>
      </div>
    {/if}
  {/if}

  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  <div class="knoepfe">
    <button type="button" class="sekundaer" onclick={onschliessen}>{darf ? "Abbrechen" : "Schließen"}</button>
    {#if darf}<button type="button" onclick={speichern} disabled={laeuft || !geaendert || !!neu}>Speichern</button>{/if}
  </div>
</div>
