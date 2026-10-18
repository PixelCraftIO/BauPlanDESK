<script lang="ts">
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import { EDITIONEN, api, datumDe, fehlertext, type Benutzer, type Lizenzstatus } from "./api";

  let {
    status,
    ich,
    onstatus,
    onschliessen,
  }: {
    status: Lizenzstatus;
    ich: Benutzer;
    onstatus: (s: Lizenzstatus) => void;
    onschliessen: () => void;
  } = $props();

  let fehler = $state<string | null>(null);
  let hinweis = $state<string | null>(null);
  let laeuft = $state(false);
  let schluessel = $state("");

  const zeit = (t: string) => new Date(t).toLocaleString("de-DE", { dateStyle: "medium", timeStyle: "short" });

  // Beim Öffnen frisch laden: Kollegen können inzwischen gekommen oder gegangen sein.
  $effect(() => {
    api.lizenz().then(onstatus).catch((e) => (fehler = fehlertext(e)));
  });

  const istAdmin = $derived(ich.aktiv && ich.rolle === "administrator");
  const l = $derived(status.lizenz);

  async function einspielen() {
    const pfad = await open({
      title: "Lizenzdatei einspielen",
      filters: [
        { name: "BauProjektIO-Lizenz", extensions: ["bplizenz"] },
        { name: "Alle Dateien", extensions: ["*"] },
      ],
    });
    if (!pfad || Array.isArray(pfad)) return;
    laeuft = true;
    fehler = hinweis = null;
    try {
      const s = await api.lizenzEinspielen(pfad);
      onstatus(s);
      hinweis = "Die Lizenz wurde eingespielt und gilt jetzt für alle Arbeitsplätze an dieser Datenbank.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }

  /** Führt eine Lizenzaktion aus und zeigt bei Erfolg `erfolg` an. */
  async function ausfuehren(aktion: () => Promise<Lizenzstatus>, erfolg: string) {
    laeuft = true;
    fehler = hinweis = null;
    try {
      onstatus(await aktion());
      hinweis = erfolg;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }

  async function aktivieren(ev: SubmitEvent) {
    ev.preventDefault();
    await ausfuehren(
      () => api.lizenzAktivieren(schluessel),
      "Die Abo-Lizenz ist aktiviert. Die übrigen Rechner an dieser Datenbank werden beim nächsten Start automatisch freigeschaltet.",
    );
    if (!fehler) schluessel = "";
  }

  async function abmelden() {
    if (!(await ask("Diesen Rechner beim Lizenzserver abmelden? Er kann danach nur noch lesen, bis er wieder bestätigt wird.", { title: "Lizenz", kind: "warning", okLabel: "Abmelden", cancelLabel: "Abbrechen" }))) return;
    await ausfuehren(api.lizenzAbmelden, "Dieser Rechner ist abgemeldet; sein Platz ist frei.");
  }

  async function erneutVersuchen() {
    fehler = hinweis = null;
    try {
      const s = await api.arbeitsplatzBelegen();
      onstatus(s);
      if (!s.schreiben_erlaubt) hinweis = "Es ist noch kein Arbeitsplatz frei.";
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
</script>

<p class:fehlertext={!status.schreiben_erlaubt}>{status.meldung}</p>

{#if l}
  <dl class="daten">
    <dt>Lizenznehmer</dt><dd>{l.lizenznehmer}</dd>
    <dt>Edition</dt><dd>{EDITIONEN[l.edition]}</dd>
    <dt>Arbeitsplätze</dt><dd>{l.arbeitsplaetze}</dd>
    <dt>Gültig bis</dt><dd>{l.gueltig_bis ? datumDe(l.gueltig_bis) : "unbefristet"}</dd>
    <dt>Updates bis</dt><dd>{l.wartung_bis ? datumDe(l.wartung_bis) : "–"}</dd>
    <dt>Lizenznummer</dt><dd class="meta">{l.lizenz_id}</dd>
  </dl>
{/if}

{#if status.abo}
  <h3>Abo-Lizenz</h3>
  <dl class="daten">
    <dt>Lizenzschlüssel</dt><dd class="meta">{status.abo.lizenzschluessel}</dd>
    <dt>Zuletzt bestätigt</dt><dd>{status.abo.bestaetigt_um ? zeit(status.abo.bestaetigt_um) : "noch nie (dieser Rechner)"}</dd>
    <dt>Ohne Internet bis</dt><dd>{status.abo.offline_gueltig_bis ? zeit(status.abo.offline_gueltig_bis) : "–"}</dd>
  </dl>
  <p class="meta">BauProjektIO bestätigt die Lizenz automatisch im Hintergrund, sobald eine Internetverbindung besteht.</p>
  <div class="knoepfe links">
    <button class="sekundaer" onclick={() => ausfuehren(api.lizenzBestaetigen, "Die Lizenz wurde bestätigt.")} disabled={laeuft}>Jetzt bestätigen</button>
    {#if istAdmin && status.abo.bestaetigt_um}
      <button class="sekundaer" onclick={abmelden} disabled={laeuft}>Diesen Rechner abmelden</button>
    {/if}
  </div>
{/if}

<h3>Belegte Arbeitsplätze ({status.arbeitsplaetze.length} von {status.arbeitsplaetze_max})</h3>
<ul class="auswahlliste">
  {#each status.arbeitsplaetze as a (a.arbeitsplatz)}
    <li>
      <strong>{a.arbeitsplatz}{a.arbeitsplatz === status.eigener_arbeitsplatz ? " (dieser Rechner)" : ""}</strong>
      <span class="meta">{a.benutzer_name} · seit {new Date(a.belegt_seit).toLocaleString("de-DE")}</span>
    </li>
  {:else}
    <li class="meta">Keiner</li>
  {/each}
</ul>
<p class="meta">Ein Arbeitsplatz wird beim Beenden von BauProjektIO frei, nach einem Absturz spätestens nach 15 Minuten.</p>

{#if istAdmin}
  <form class="knoepfe links" onsubmit={aktivieren}>
    <input
      placeholder="Lizenzschlüssel, z. B. BPIO-XXXXX-XXXXX-…"
      bind:value={schluessel}
      aria-label="Lizenzschlüssel"
      spellcheck="false"
      autocomplete="off"
      style="flex: 1"
    />
    <button type="submit" disabled={laeuft || !schluessel.trim()}>{laeuft ? "Wird geprüft …" : "Abo aktivieren"}</button>
  </form>
{/if}

{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
{#if hinweis}<p class="meta" role="status">{hinweis}</p>{/if}

<div class="knoepfe">
  {#if status.zustand.art === "kein_arbeitsplatz"}
    <button class="sekundaer" onclick={erneutVersuchen}>Erneut versuchen</button>
  {/if}
  {#if istAdmin}
    <button class="sekundaer" onclick={einspielen} disabled={laeuft}>{laeuft ? "Wird geprüft …" : "Lizenzdatei einspielen …"}</button>
  {:else}
    <span class="meta">Lizenzen spielt ein Administrator ein.</span>
  {/if}
  <button class="sekundaer" onclick={onschliessen}>Schließen</button>
</div>
