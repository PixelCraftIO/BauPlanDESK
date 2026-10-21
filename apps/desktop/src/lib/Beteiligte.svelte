<script lang="ts">
  import { onMount } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import {
    BETEILIGTENROLLEN,
    FACHRICHTUNGEN,
    anschriftZeile,
    beteiligtenrolleName,
    api,
    fehlertext,
    istKonflikt,
    kategorieName,
    kundenname,
    kundePasst,
    leereKundendaten,
    mindestens,
    type Benutzer,
    type Beteiligtenrolle,
    type Kunde,
    type Kundendaten,
    type Project,
    type Projektbeteiligter,
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
  let gespeichert = $state("");
  let gewaehlt = $state<Projektbeteiligter[]>([]);
  let rolle = $state<Beteiligtenrolle>("grundstueckseigentuemer");
  let suche = $state("");
  let neu = $state<Kundendaten | null>(null);
  let fehler = $state<string | null>(null);
  let laeuft = $state(false);

  const schluessel = (l: Projektbeteiligter[]) =>
    l.map((b) => `${b.rolle}:${b.kunde.id}:${(b.fachrichtung ?? "").trim()}`).join();
  const vorschlaege = $derived(
    alle
      .filter(
        (k) =>
          !gewaehlt.some((b) => b.rolle === rolle && b.kunde.id === k.id) &&
          kundePasst(k, suche),
      )
      .slice(0, 50),
  );
  const geaendert = $derived(schluessel(gewaehlt) !== gespeichert);

  async function laden() {
    try {
      const [kunden, beteiligte] = await Promise.all([
        api.kunden(),
        api.beteiligte(projekt.id),
      ]);
      alle = kunden;
      gewaehlt = beteiligte;
      gespeichert = schluessel(beteiligte);
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
  // Nur beim Öffnen laden: Eine neue Projektversion (z. B. nach einem
  // Konflikt) darf die eigene Auswahl nicht ersetzen.
  onMount(laden);

  function hinzufuegen(k: Kunde) {
    // Je Rolle in Erfassungsreihenfolge, Rollen in fester Reihenfolge.
    const reihenfolge = BETEILIGTENROLLEN.map((r) => r.code);
    gewaehlt = [...gewaehlt, { rolle, kunde: k, fachrichtung: "" }].sort(
      (a, b) => reihenfolge.indexOf(a.rolle) - reihenfolge.indexOf(b.rolle),
    );
  }

  async function anlegen(e: SubmitEvent) {
    e.preventDefault();
    if (!neu) return;
    laeuft = true;
    fehler = null;
    try {
      const k = await api.kundeAnlegen(neu);
      alle = [...alle, k];
      hinzufuegen(k);
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
      onprojekt(
        await api.beteiligteSetzen(
          projekt.id,
          gewaehlt.map((b) => ({
            rolle: b.rolle,
            kunde: b.kunde.id,
            fachrichtung: b.fachrichtung ?? "",
          })),
          projekt.version,
        ),
      );
      gespeichert = schluessel(gewaehlt);
      onschliessen();
    } catch (e) {
      if (istKonflikt(e)) await konfliktNachladen();
      else fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }

  /**
   * Versionskonflikt: Die aktuelle Projektfassung wird Grundlage, die eigene
   * Auswahl bleibt stehen und kann erneut gespeichert werden.
   */
  async function konfliktNachladen() {
    try {
      const [p, aktuell] = await Promise.all([
        api.projektOeffnen(projekt.id),
        api.beteiligte(projekt.id),
      ]);
      onprojekt(p);
      gespeichert = schluessel(aktuell);
      const namen =
        aktuell
          .map((b) => `${kundenname(b.kunde)} (${beteiligtenrolleName(b.rolle)})`)
          .join(", ") || "keine weiteren Beteiligten";
      fehler = geaendert
        ? `Das Projekt wurde inzwischen an einem anderen Arbeitsplatz geändert. Ihre Auswahl bleibt erhalten. Dort gespeichert ist jetzt: ${namen}. „Speichern“ ersetzt diese Zuordnung durch Ihre Auswahl.`
        : `Das Projekt wurde inzwischen an einem anderen Arbeitsplatz geändert. Die dort gespeicherte Zuordnung (${namen}) entspricht bereits Ihrer Auswahl.`;
    } catch (ladeFehler) {
      fehler = `Das Projekt wurde inzwischen an einem anderen Arbeitsplatz geändert. Die aktuelle Fassung konnte nicht geladen werden: ${fehlertext(ladeFehler)} Ihre Auswahl bleibt erhalten; bitte erneut speichern.`;
    }
  }

  const neuerKundeBegonnen = $derived(
    !!neu && JSON.stringify(neu) !== JSON.stringify(leereKundendaten()),
  );
  export function hatAenderungen() {
    return darf && (geaendert || neuerKundeBegonnen);
  }
  export function istBeschaeftigt() {
    return laeuft;
  }
  /** Rückfrage vor dem Verwerfen (Escape, Abbrechen, Menübefehle). */
  export async function schliessenErlaubt() {
    return (
      !laeuft &&
      (!hatAenderungen() ||
        (await ask(
          "Die geänderten Beteiligten wurden noch nicht gespeichert. Verwerfen?",
          {
            title: "Weitere Beteiligte",
            kind: "warning",
            okLabel: "Verwerfen",
            cancelLabel: "Weiter bearbeiten",
          },
        )))
    );
  }
</script>

<div class="spalte">
  <p class="meta">
    Weitere Beteiligte kommen wie die Bauherren aus der Kundendatenbank. Ist ein
    Grundstückseigentümer oder Erbbauberechtigter nicht zugleich Bauherr, weist
    der Bauantrag auf die nötige Zustimmungserklärung hin.
  </p>

  {#each BETEILIGTENROLLEN as r (r.code)}
    {@const inRolle = gewaehlt.filter((b) => b.rolle === r.code)}
    {#if inRolle.length}
      <h3>{r.name}</h3>
      <ul class="bauherren">
        {#each inRolle as b (b.kunde.id)}
          <li>
            <div>
              <strong>{kundenname(b.kunde)}</strong>
              <span class="meta"
                >{[kategorieName(b.kunde.kategorie), anschriftZeile(b.kunde.anschrift)]
                  .filter(Boolean)
                  .join(" · ")}</span
              >
              {#if b.rolle === "fachplaner"}
                <label class="meta"
                  >Fachrichtung
                  <input
                    list="fachrichtungen"
                    bind:value={b.fachrichtung}
                    disabled={!darf}
                    maxlength="100"
                    placeholder="z. B. Tragwerksplanung/Standsicherheit"
                  /></label
                >
              {/if}
            </div>
            {#if darf}
              <button
                type="button"
                class="sekundaer"
                onclick={() => (gewaehlt = gewaehlt.filter((x) => x !== b))}
                >Entfernen</button
              >
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/each}
  {#if !gewaehlt.length}<p class="meta">Noch keine weiteren Beteiligten.</p>{/if}
  <datalist id="fachrichtungen">
    {#each FACHRICHTUNGEN as f (f.code)}<option value={f.name}></option>{/each}
  </datalist>

  {#if darf}
    <label
      >Rolle
      <select bind:value={rolle}>
        {#each BETEILIGTENROLLEN as r (r.code)}<option value={r.code}>{r.name}</option>{/each}
      </select>
    </label>
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
      <input
        placeholder="Kunden suchen und in dieser Rolle hinzufügen …"
        bind:value={suche}
        aria-label="Kunden suchen"
      />
      <ul class="auswahlliste" aria-label="Kunden">
        {#each vorschlaege as k (k.id)}
          <li>
            <button type="button" class="link" onclick={() => hinzufuegen(k)}>{kundenname(k)}</button>
            <span class="meta"
              >{[kategorieName(k.kategorie), k.kundennummer, anschriftZeile(k.anschrift)]
                .filter(Boolean)
                .join(" · ")}</span
            >
          </li>
        {:else}
          <li class="meta">
            {alle.length ? "Kein weiterer Kunde passt zur Suche." : "Die Kundendatenbank ist noch leer."}
          </li>
        {/each}
      </ul>
      <div>
        <button type="button" class="sekundaer" onclick={() => (neu = leereKundendaten())}
          >Neuen Kunden anlegen …</button
        >
      </div>
    {/if}
  {/if}

  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  <div class="knoepfe">
    <button
      type="button"
      class="sekundaer"
      onclick={async () => {
        if (await schliessenErlaubt()) onschliessen();
      }}>{darf ? "Abbrechen" : "Schließen"}</button
    >
    {#if darf}<button type="button" onclick={speichern} disabled={laeuft || !geaendert || !!neu}
        >Speichern</button
      >{/if}
  </div>
</div>
