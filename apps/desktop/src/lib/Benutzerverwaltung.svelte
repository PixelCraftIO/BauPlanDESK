<script lang="ts">
  import { ROLLEN, api, fehlertext, rollenName, type Benutzer, type Rolle } from "./api";

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
  let neu = $state({ anmeldename: "", anzeigename: "", rolle: "bearbeitung" as Rolle });

  async function laden() {
    try {
      liste = (await api.benutzerAuflisten()).filter((b) => b.anmeldename !== "system");
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
  $effect(() => {
    laden();
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
    try {
      await api.benutzerAnlegen({ ...neu, anzeigename: neu.anzeigename || neu.anmeldename });
      neu = { anmeldename: "", anzeigename: "", rolle: "bearbeitung" };
    } catch (e) {
      fehler = fehlertext(e);
    }
    await laden();
  }
</script>

<p class="meta">
  Angemeldet als <strong>{ich.anzeigename}</strong> ({ich.anmeldename}, {rollenName(ich.rolle)}).
  Die Anmeldung erfolgt über das Benutzerkonto des Betriebssystems.
  {#if !istAdmin}Nur Administratoren können Benutzer anlegen und Rollen ändern.{/if}
</p>

{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}

<table class="ergebnistabelle">
  <thead><tr><th>Anmeldename</th><th>Name</th><th>Rolle</th><th>Aktiv</th></tr></thead>
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
      </tr>
    {/each}
  </tbody>
</table>

{#if istAdmin}
  <h3>Benutzer anlegen</h3>
  <p class="meta">Wer sich zum ersten Mal anmeldet, wird automatisch mit der Rolle „Lesen“ angelegt. Hier können Sie Kollegen vorab mit einer anderen Rolle eintragen.</p>
  <form onsubmit={anlegen}>
    <input placeholder="Anmeldename (Windows/macOS)" bind:value={neu.anmeldename} required aria-label="Anmeldename" />
    <input placeholder="Anzeigename" bind:value={neu.anzeigename} aria-label="Anzeigename" />
    <select bind:value={neu.rolle} aria-label="Rolle">
      {#each ROLLEN as r}<option value={r.code}>{r.name}</option>{/each}
    </select>
    <button type="submit">Anlegen</button>
  </form>
{/if}

<div class="knoepfe">
  <button class="sekundaer" onclick={onschliessen}>Schließen</button>
</div>
