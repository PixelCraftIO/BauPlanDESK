<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
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

{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
{#if hinweis}<p class="meta" role="status">{hinweis}</p>{/if}

<div class="knoepfe">
  {#if status.zustand.art === "kein_arbeitsplatz"}
    <button class="sekundaer" onclick={erneutVersuchen}>Erneut versuchen</button>
  {/if}
  {#if istAdmin}
    <button onclick={einspielen} disabled={laeuft}>{laeuft ? "Wird geprüft …" : "Lizenzdatei einspielen …"}</button>
  {:else}
    <span class="meta">Lizenzen spielt ein Administrator ein.</span>
  {/if}
  <button class="sekundaer" onclick={onschliessen}>Schließen</button>
</div>
