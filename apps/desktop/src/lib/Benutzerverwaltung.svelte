<script lang="ts">
  import { KENNWORTSTATUS, ROLLEN, api, fehlertext, rollenName, type Anmeldestatus, type Benutzer, type Rolle } from "./api";
  import { untrack } from "svelte";
  import KennwortAendern from "./KennwortAendern.svelte";

  let {
    ich,
    onich,
    onschliessen,
  }: {
    ich: Benutzer;
    /** Das eigene Konto wurde geändert (z. B. Anzeigename). */
    onich: (b: Benutzer) => void;
    onschliessen: () => void;
  } = $props();

  const istAdmin = $derived(ich.aktiv && ich.rolle === "administrator");

  let liste = $state<Benutzer[]>([]);
  let fehler = $state<string | null>(null);
  let neu = $state({ anmeldename: "", anzeigename: "", rolle: "bearbeitung" as Rolle, erstkennwort: "" });
  /** Betriebsart und Kennwortregeln aus dem Backend. */
  let anmeldung = $state<Anmeldestatus | null>(null);
  const mitKennwort = $derived(!!anmeldung?.kennwortanmeldung);
  /** Benutzer, für den gerade ein Erstkennwort eingegeben wird. */
  let erstFuer = $state<Benutzer | null>(null);
  let erstkennwort = $state("");
  let eigenesAendern = $state(false);
  let meldung = $state<string | null>(null);

  const zeit = (t: string) => new Date(t).toLocaleString("de-DE");

  async function laden() {
    try {
      anmeldung ??= await api.anmeldestatus();
      liste = (await api.benutzerAuflisten()).filter((b) => b.anmeldename !== "system");
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
  $effect(() => {
    // Nur beim Öffnen laden; `anmeldung` ist keine Abhängigkeit.
    untrack(() => laden());
  });

  async function aendern(b: Benutzer, teil: Partial<Pick<Benutzer, "anzeigename" | "rolle" | "aktiv">>) {
    fehler = null;
    try {
      const neuB = await api.benutzerAendern({ id: b.id, anzeigename: b.anzeigename, rolle: b.rolle, aktiv: b.aktiv, ...teil });
      if (neuB.id === ich.id) onich(neuB);
    } catch (e) {
      fehler = fehlertext(e);
    }
    await laden();
  }

  async function anlegen(ev: SubmitEvent) {
    ev.preventDefault();
    fehler = null;
    meldung = null;
    try {
      const { erstkennwort: erst, ...daten } = neu;
      const b = await api.benutzerAnlegen({ ...daten, anzeigename: daten.anzeigename || daten.anmeldename });
      neu = { anmeldename: "", anzeigename: "", rolle: "bearbeitung", erstkennwort: "" };
      if (mitKennwort && erst) {
        try {
          await api.erstkennwortVergeben(b.id, erst);
          meldung = `${b.anzeigename} angelegt. Bitte das Erstkennwort persönlich weitergeben; es muss bei der ersten Anmeldung geändert werden.`;
        } catch (e) {
          fehler = `${b.anzeigename} wurde angelegt, das Erstkennwort aber nicht gespeichert: ${fehlertext(e)}`;
        }
      }
    } catch (e) {
      fehler = fehlertext(e);
    }
    await laden();
  }

  async function erstkennwortSpeichern(ev: SubmitEvent) {
    ev.preventDefault();
    if (!erstFuer) return;
    fehler = null;
    meldung = null;
    try {
      const b = await api.erstkennwortVergeben(erstFuer.id, erstkennwort);
      meldung = `Erstkennwort für ${b.anzeigename} gespeichert. Eine Anmeldesperre ist aufgehoben; das Kennwort muss bei der nächsten Anmeldung geändert werden.`;
      erstFuer = null;
      erstkennwort = "";
    } catch (e) {
      fehler = fehlertext(e);
    }
    await laden();
  }
</script>

<p class="meta">
  Angemeldet als <strong>{ich.anzeigename}</strong> ({ich.anmeldename}, {rollenName(ich.rolle)}).
  {#if mitKennwort}
    Die Anmeldung erfolgt mit Anmeldename und Kennwort; Sie können an jedem Arbeitsplatz arbeiten
    (Benutzer wechseln: <em>Anwendung → Abmelden / Benutzer wechseln …</em>).
  {:else}
    Einzelplatzbetrieb: Die Anmeldung erfolgt über das Benutzerkonto des Betriebssystems, ohne Kennwort.
  {/if}
  {#if !istAdmin}Nur Administratoren können Benutzer anlegen und Rollen ändern.{/if}
</p>

{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
{#if meldung}<p role="status">{meldung}</p>{/if}

{#if mitKennwort && anmeldung}
  {#if eigenesAendern}
    <h3>Eigenes Kennwort ändern</h3>
    <KennwortAendern
      bisherNoetig={ich.kennwort !== "keines"}
      mindestlaenge={anmeldung.kennwort_mindestlaenge}
      ongespeichert={() => {
        eigenesAendern = false;
        meldung = "Ihr Kennwort wurde geändert.";
        laden();
      }}
      onabbrechen={() => (eigenesAendern = false)}
    />
  {:else}
    <div class="knoepfe links">
      <button class="sekundaer" onclick={() => { meldung = null; eigenesAendern = true; }}>Eigenes Kennwort ändern …</button>
    </div>
  {/if}
{/if}

<table class="ergebnistabelle">
  <thead><tr><th>Anmeldename</th><th>Name</th><th>Rolle</th><th>Aktiv</th>{#if mitKennwort}<th>Kennwort</th>{/if}</tr></thead>
  <tbody>
    {#each liste as b (b.id)}
      <tr>
        <td>{b.anmeldename}</td>
        <td>
          {#if istAdmin}
            <input value={b.anzeigename} aria-label="Anzeigename" onchange={(e) => aendern(b, { anzeigename: e.currentTarget.value })} />
          {:else}{b.anzeigename}{/if}
        </td>
        <td>
          {#if istAdmin}
            <select value={b.rolle} aria-label="Rolle" onchange={(e) => aendern(b, { rolle: e.currentTarget.value as Rolle })}>
              {#each ROLLEN as r}<option value={r.code} title={r.beschreibung}>{r.name}</option>{/each}
            </select>
          {:else}{rollenName(b.rolle)}{/if}
        </td>
        <td>
          <input type="checkbox" checked={b.aktiv} disabled={!istAdmin} aria-label="Aktiv" onchange={(e) => aendern(b, { aktiv: e.currentTarget.checked })} />
        </td>
        {#if mitKennwort}
          <td>
            {KENNWORTSTATUS[b.kennwort ?? "keines"]}
            {#if b.anmeldesperre_bis && new Date(b.anmeldesperre_bis) > new Date()}
              <br /><span class="fehlertext">Anmeldung gesperrt bis {zeit(b.anmeldesperre_bis)}</span>
            {/if}
            {#if b.letzte_anmeldung}<br /><span class="meta">Zuletzt angemeldet {zeit(b.letzte_anmeldung)}</span>{/if}
            {#if istAdmin}
              <br /><button class="link" onclick={() => { erstFuer = b; erstkennwort = ""; meldung = null; }}
                >{b.kennwort === "keines" ? "Erstkennwort vergeben …" : "Kennwort zurücksetzen …"}</button
              >
            {/if}
          </td>
        {/if}
      </tr>
    {/each}
  </tbody>
</table>

{#if istAdmin && erstFuer}
  <h3>Erstkennwort für {erstFuer.anzeigename} ({erstFuer.anmeldename})</h3>
  <p class="meta">
    Das Erstkennwort muss bei der nächsten Anmeldung durch ein eigenes ersetzt werden. Eine Sperre
    nach Fehlversuchen wird aufgehoben. Mindestens {anmeldung?.kennwort_mindestlaenge} Zeichen.
  </p>
  <form onsubmit={erstkennwortSpeichern}>
    <input type="password" autocomplete="new-password" bind:value={erstkennwort} required aria-label="Erstkennwort" />
    <button type="submit">Erstkennwort speichern</button>
    <button type="button" class="sekundaer" onclick={() => (erstFuer = null)}>Abbrechen</button>
  </form>
{/if}

{#if istAdmin}
  <h3>Benutzer anlegen</h3>
  {#if mitKennwort}
    <p class="meta">Neue Kollegen erhalten ein Erstkennwort, das sie bei der ersten Anmeldung ändern. Ohne Kennwort ist keine Anmeldung möglich.</p>
  {:else}
    <p class="meta">Wer sich zum ersten Mal anmeldet, wird automatisch mit der Rolle „Lesen“ angelegt. Hier können Sie Kollegen vorab mit einer anderen Rolle eintragen.</p>
  {/if}
  <form onsubmit={anlegen}>
    <input placeholder={mitKennwort ? "Anmeldename" : "Anmeldename (Windows/macOS)"} bind:value={neu.anmeldename} required aria-label="Anmeldename" />
    <input placeholder="Anzeigename" bind:value={neu.anzeigename} aria-label="Anzeigename" />
    <select bind:value={neu.rolle} aria-label="Rolle">
      {#each ROLLEN as r}<option value={r.code}>{r.name}</option>{/each}
    </select>
    {#if mitKennwort}
      <input type="password" autocomplete="new-password" placeholder="Erstkennwort" bind:value={neu.erstkennwort} required aria-label="Erstkennwort" />
    {/if}
    <button type="submit">Anlegen</button>
  </form>
{/if}

<div class="knoepfe">
  <button class="sekundaer" onclick={onschliessen}>Schließen</button>
</div>
