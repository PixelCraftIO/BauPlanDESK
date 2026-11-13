<script lang="ts">
  import { api, fehlertext } from "./api";
  import type {
    Aboauskunft,
    Aboumfang,
  } from "./generated/plattform";
  let { modus }: { modus: string } = $props();
  const SPEICHER = "bauplanio.serveradresse";
  const gelesen = () => {
    try {
      return localStorage.getItem(SPEICHER) ?? "";
    } catch {
      return "";
    }
  };
  let auskunft = $state<Aboauskunft | null>(null),
    umfang = $state<Aboumfang>("meine"),
    server = $state(gelesen()),
    adresse = $state<string | null>(null),
    fehler = $state<string | null>(null),
    laeuft = $state(false),
    kopiert = $state(false);
  const netzwerk = $derived(modus === "Netzwerk");
  async function laden() {
    try {
      auskunft = await api.kalenderaboAuskunft();
      if (auskunft.umfang) umfang = auskunft.umfang;
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
  function serverMerken() {
    try {
      localStorage.setItem(SPEICHER, server.trim());
    } catch {
      /* nur Komfort */
    }
  }
  async function anlegen() {
    laeuft = true;
    fehler = null;
    kopiert = false;
    try {
      const s = await api.kalenderaboAnlegen(umfang);
      serverMerken();
      adresse = server.trim().replace(/\/+$/, "") + s.pfad;
      await laden();
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function widerrufen() {
    laeuft = true;
    fehler = null;
    try {
      await api.kalenderaboWiderrufen();
      adresse = null;
      await laden();
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function kopieren() {
    if (!adresse) return;
    try {
      await navigator.clipboard.writeText(adresse);
      kopiert = true;
    } catch {
      kopiert = false;
    }
  }
  $effect(() => {
    void laden();
  });
</script>

<section class="einstellungsabschnitt">
  <h2>Kalender-Abo</h2>
  {#if !netzwerk}
    <p class="meta">
      Das Abo braucht den BauPlanDESK-Server im Netzwerkbetrieb. Im Einzelplatz
      lassen sich die Fristen je Projekt in der Verfahrensakte als
      Kalenderdatei (ICS) speichern.
    </p>
  {:else}
    <p class="meta">
      Persönliche Kalenderadresse für Outlook, Apple- oder Google-Kalender.
      Der Kalender enthält die laufenden Fristen aller Projekte, die Sie lesen
      dürfen, und aktualisiert sich bei jedem Abruf.
    </p>
    <p>
      {#if auskunft?.aktiv}Abo aktiv seit {new Date(
          auskunft.erstellt_am ?? "",
        ).toLocaleDateString("de-DE")} ({auskunft.umfang === "meine"
          ? "nur eigene Zuständigkeit"
          : "alle Fristen"}).{:else}Kein Abo angelegt.{/if}
    </p>
    <div class="raster">
      <label
        >Adresse des BauPlanDESK-Servers<input
          type="url"
          bind:value={server}
          placeholder="https://bauplandesk.buero.local:8080"
        /></label
      >
      <label
        >Umfang<select bind:value={umfang}
          ><option value="meine">Nur meine Zuständigkeit</option><option
            value="alle">Alle Fristen</option
          ></select
        ></label
      >
    </div>
    <div class="knoepfe links">
      <button onclick={anlegen} disabled={laeuft || !server.trim()}
        >{auskunft?.aktiv ? "Neue Adresse erzeugen" : "Abo anlegen"}</button
      >
      {#if auskunft?.aktiv}<button
          class="sekundaer"
          onclick={widerrufen}
          disabled={laeuft}>Abo widerrufen</button
        >{/if}
    </div>
    {#if adresse}
      <p class="hinweisbox" role="status">
        Diese Adresse wird nur jetzt angezeigt. Wer sie kennt, sieht Ihre
        Fristen; bei Verlust eine neue Adresse erzeugen (die alte wird damit
        ungültig).<br /><code>{adresse}</code>
        <button class="sekundaer" onclick={kopieren}
          >{kopiert ? "Kopiert" : "Adresse kopieren"}</button
        >
      </p>
    {/if}
  {/if}
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
</section>
