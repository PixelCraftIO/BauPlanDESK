<script lang="ts">
  import { ask } from "@tauri-apps/plugin-dialog";
  import {
    KUNDENKATEGORIEN,
    anschriftZeile,
    api,
    fehlertext,
    istKonflikt,
    kategorieName,
    kundendaten,
    kundenname,
    kundePasst,
    landName,
    leereKundendaten,
    mindestens,
    type Benutzer,
    type Kunde,
    type Kundendaten,
    type Kundenkategorie,
    type Project,
  } from "./api";
  import KundeFelder from "./KundeFelder.svelte";

  let {
    ich,
    onoeffnen,
    onschliessen,
  }: {
    ich: Benutzer;
    /** Projekt eines Kunden öffnen. */
    onoeffnen: (projektId: string) => void;
    onschliessen: () => void;
  } = $props();

  const darf = $derived(ich.aktiv && mindestens(ich.rolle, "bearbeitung"));
  const darfLoeschen = $derived(ich.aktiv && mindestens(ich.rolle, "projektleitung"));

  let liste = $state<Kunde[]>([]);
  let suche = $state("");
  let kategorie = $state<Kundenkategorie | "">("");
  /** Gewählter Kunde; `null` mit `bearbeitung` = neuer Kunde. */
  let gewaehlt = $state<Kunde | null>(null);
  let bearbeitung = $state<Kundendaten | null>(null);
  let projekte = $state<Project[]>([]);
  let fehler = $state<string | null>(null);
  let laeuft = $state(false);

  const gefiltert = $derived(liste.filter((k) => (!kategorie || k.kategorie === kategorie) && kundePasst(k, suche)));
  const geaendert = $derived(
    !!bearbeitung && JSON.stringify(bearbeitung) !== JSON.stringify(gewaehlt ? kundendaten(gewaehlt) : leereKundendaten()),
  );

  async function laden() {
    try {
      liste = await api.kunden();
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
  $effect(() => {
    laden();
  });

  async function verwerfenErlaubt(): Promise<boolean> {
    return !geaendert || (await ask("Die Änderungen am Kunden wurden nicht gespeichert. Verwerfen?", { title: "Kunden", kind: "warning", okLabel: "Verwerfen", cancelLabel: "Abbrechen" }));
  }

  async function waehlen(k: Kunde) {
    if (gewaehlt?.id === k.id || !(await verwerfenErlaubt())) return;
    zeigen(k);
  }

  async function zeigen(k: Kunde) {
    fehler = null;
    gewaehlt = k;
    bearbeitung = kundendaten(k);
    projekte = [];
    try {
      projekte = await api.kundeProjekte(k.id);
    } catch (e) {
      fehler = fehlertext(e);
    }
  }

  async function neu() {
    if (!(await verwerfenErlaubt())) return;
    fehler = null;
    gewaehlt = null;
    projekte = [];
    bearbeitung = { ...leereKundendaten(), kategorie: kategorie || "privat" };
  }

  async function speichern(e: SubmitEvent) {
    e.preventDefault();
    if (!bearbeitung) return;
    laeuft = true;
    fehler = null;
    try {
      const k = gewaehlt
        ? await api.kundeAendern(gewaehlt.id, bearbeitung, gewaehlt.version)
        : await api.kundeAnlegen(bearbeitung);
      await laden();
      await zeigen(k);
    } catch (err) {
      if (istKonflikt(err) && gewaehlt) {
        await laden();
        const aktuell = liste.find((k) => k.id === gewaehlt?.id);
        if (aktuell) await zeigen(aktuell);
        fehler = "Der Kunde wurde inzwischen an einem anderen Arbeitsplatz geändert. Die aktuelle Fassung wurde geladen; bitte Änderungen erneut eintragen.";
      } else {
        fehler = fehlertext(err);
      }
    } finally {
      laeuft = false;
    }
  }

  async function loeschen() {
    if (!gewaehlt) return;
    const name = kundenname(gewaehlt);
    if (!(await ask(`${name} endgültig aus der Kundendatenbank löschen?`, { title: "Kunden löschen", kind: "warning", okLabel: "Löschen", cancelLabel: "Abbrechen" }))) return;
    laeuft = true;
    fehler = null;
    try {
      await api.kundeLoeschen(gewaehlt.id, gewaehlt.version);
      gewaehlt = null;
      bearbeitung = null;
      projekte = [];
      await laden();
    } catch (err) {
      fehler = fehlertext(err);
    } finally {
      laeuft = false;
    }
  }

  async function projektOeffnen(id: string) {
    if (await verwerfenErlaubt()) onoeffnen(id);
  }

  async function schliessen() {
    if (await verwerfenErlaubt()) onschliessen();
  }
  export function hatAenderungen() {return geaendert;}
  export function istBeschaeftigt() {return laeuft;}
  export async function speichernBeimSchliessen() {if(laeuft)return false;await speichern(new Event("submit") as SubmitEvent);return !geaendert;}
</script>

<div class="kunden">
  <div class="kundenliste">
    <div class="knoepfe links">
      <!-- svelte-ignore a11y_autofocus -->
      <input placeholder="Suchen …" bind:value={suche} aria-label="Kunden suchen" autofocus />
      <select bind:value={kategorie} aria-label="Kategorie">
        <option value="">Alle Kategorien</option>
        {#each KUNDENKATEGORIEN as k}<option value={k.code}>{k.name}</option>{/each}
      </select>
    </div>
    <ul class="auswahlliste" role="listbox" aria-label="Kunden">
      {#each gefiltert as k (k.id)}
        <li
          role="option"
          aria-selected={gewaehlt?.id === k.id}
          class:aktiv={gewaehlt?.id === k.id}
          onclick={() => waehlen(k)}
          onkeydown={(e) => e.key === "Enter" && waehlen(k)}
          tabindex="0"
        >
          <strong>{kundenname(k)}</strong>
          <span class="meta">{[kategorieName(k.kategorie), k.kundennummer, anschriftZeile(k.anschrift)].filter(Boolean).join(" · ")}</span>
        </li>
      {:else}
        <li class="meta">{liste.length ? "Kein Kunde passt zur Suche." : "Noch keine Kunden angelegt."}</li>
      {/each}
    </ul>
    <p class="meta">{liste.length} {liste.length === 1 ? "Kunde" : "Kunden"}</p>
    <button onclick={neu} disabled={!darf}>Neuer Kunde</button>
  </div>

  <div class="kundendetail">
    {#if bearbeitung}
      <form class="spalte" onsubmit={speichern}>
        <h3>{gewaehlt ? kundenname(gewaehlt) : "Neuer Kunde"}</h3>
        <KundeFelder bind:daten={bearbeitung} gesperrt={!darf} />
        {#if gewaehlt}
          <p class="meta">Angelegt {new Date(gewaehlt.created_at).toLocaleDateString("de-DE")}, zuletzt geändert {new Date(gewaehlt.updated_at).toLocaleString("de-DE")}.</p>
        {/if}
        <div class="knoepfe links">
          {#if darf}<button type="submit" disabled={laeuft || !geaendert}>{gewaehlt ? "Speichern" : "Anlegen"}</button>{/if}
          {#if gewaehlt && darfLoeschen}
            <button type="button" class="sekundaer" onclick={loeschen} disabled={laeuft || projekte.length > 0} title={projekte.length > 0 ? "Nur Kunden ohne Projekte können gelöscht werden" : ""}>Löschen</button>
          {/if}
        </div>
      </form>
      {#if gewaehlt}
        <h3>Projekte</h3>
        {#if projekte.length === 0}
          <p class="meta">Noch keinem Projekt als Bauherr zugeordnet.</p>
        {:else}
          <ul class="projekte">
            {#each projekte as p (p.id)}
              <li>
                <button class="link" onclick={() => projektOeffnen(p.id)}>{p.name}</button>
                <span class="meta">{landName(p.bundesland)} · geändert {new Date(p.updated_at).toLocaleDateString("de-DE")}</span>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    {:else}
      <p class="meta">Kunden wie Bauherren, Bauträger, Investoren oder Baufirmen werden hier einmal erfasst und dann Projekten als Bauherren zugeordnet. Änderungen gelten sofort in allen Projekten.</p>
    {/if}
    {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  </div>
</div>

<div class="knoepfe">
  <button class="sekundaer" onclick={schliessen}>Schließen</button>
</div>
