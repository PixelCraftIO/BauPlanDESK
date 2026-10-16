<script lang="ts">
  import { onMount } from "svelte";
  import { confirm } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext, istKonflikt, rollenName, type Benutzer } from "./api";
  import { plattformApi, type Datensatz } from "./plattformApi";
  import type { ProjektMitglied, ProjektRecht } from "./generated/plattform";
  let { projektId, onschliessen }: { projektId: string; onschliessen: () => void } = $props();
  let benutzer = $state<Benutzer[]>([]);
  let mitglieder = $state<Datensatz<ProjektMitglied>[]>([]);
  let rechte = $state<Record<string, ProjektRecht>>({});
  let laeuft = $state(false), bereit = $state(false);
  let fehler = $state<string | null>(null), meldung = $state<string | null>(null);
  const gespeichert = (id: string) => mitglieder.find((m) => m.daten.benutzer_id === id);
  const geaendert = $derived(benutzer.some((b) => (rechte[b.id] ?? "kein") !== (gespeichert(b.id)?.daten.recht ?? "kein")));
  async function laden() {
    laeuft = true;
    fehler = null;
    try {
      const [b, m] = await Promise.all([api.benutzerAuflisten(), plattformApi.mitglieder(projektId)]);
      benutzer = b;
      mitglieder = m;
      rechte = Object.fromEntries(m.map((f) => [f.daten.benutzer_id, f.daten.recht]));
      bereit = true;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  onMount(laden);
  async function speichern(b: Benutzer) {
    laeuft = true;
    fehler = null;
    meldung = null;
    try {
      const m = await plattformApi.mitgliedSetzen(projektId, { benutzer_id: b.id, recht: rechte[b.id] ?? "kein" }, gespeichert(b.id)?.revision ?? 0);
      mitglieder = [...mitglieder.filter((f) => f.objekt !== m.objekt), m];
      meldung = `Projektberechtigung für ${b.anzeigename} gespeichert.`;
    } catch (e) {
      fehler = istKonflikt(e) ? "Die Mitgliedschaft wurde inzwischen geändert. Ihre Auswahl bleibt erhalten. Bitte die aktuelle Liste laden und erneut prüfen." : fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  export function istBeschaeftigt() { return laeuft; }
  export async function schliessenErlaubt() {
    return !laeuft && (!geaendert || await confirm("Ungespeicherte Projektberechtigungen verwerfen?", { title: "Projektmitglieder", kind: "warning" }));
  }
</script>

<p>Projektberechtigungen ergänzen die globale Benutzerrolle. Administratoren haben Zugriff auf alle Projekte. Schreibrechte benötigen außerdem eine gültige Lizenz.</p>
{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
{#if meldung}<p role="status">{meldung}</p>{/if}
{#if bereit}
  <table>
    <thead><tr><th>Benutzer</th><th>Globale Rolle</th><th>Projektberechtigung</th><th>Aktion</th></tr></thead>
    <tbody>{#each benutzer as b (b.id)}
      <tr><td>{b.anzeigename}{b.aktiv ? "" : " (inaktiv)"}</td><td>{rollenName(b.rolle)}</td><td>
        <select aria-label={`Projektberechtigung für ${b.anzeigename}`} value={rechte[b.id] ?? "kein"}
          disabled={laeuft || !b.aktiv || b.rolle === "administrator"}
          onchange={(e) => (rechte[b.id] = e.currentTarget.value as ProjektRecht)}>
          <option value="kein">Kein Zugriff</option><option value="lesen">Lesen</option><option value="bearbeiten">Bearbeiten</option><option value="leitung">Leitung</option>
        </select>
      </td><td><button aria-label={`Berechtigung für ${b.anzeigename} speichern`} disabled={laeuft || !b.aktiv || b.rolle === "administrator" || (rechte[b.id] ?? "kein") === (gespeichert(b.id)?.daten.recht ?? "kein")} onclick={() => speichern(b)}>Speichern</button></td></tr>
    {/each}</tbody>
  </table>
{/if}
<div class="knoepfe">
  <button class="sekundaer" disabled={laeuft} onclick={async () => { if (await schliessenErlaubt()) await laden(); }}>Aktuelle Liste laden</button>
  <button disabled={laeuft} onclick={async () => { if (await schliessenErlaubt()) onschliessen(); }}>Schließen</button>
</div>
