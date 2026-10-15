<script lang="ts">
  import BueroSicherung from "./verfahren/BueroSicherung.svelte";
  import { onMount } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext, type Benutzer } from "./api";
  let { ich }: { ich: Benutzer | null } = $props();
  let status = $state<{
      lokal: boolean;
      profil: string;
      hinweis: string;
    } | null>(null),
    laeuft = $state(false),
    fehler = $state<string | null>(null),
    meldung = $state<string | null>(null),
    wiederhergestellt = $state<string | null>(null);
  const darf = $derived(ich?.aktiv && ich.rolle === "administrator");
  export function istBeschaeftigt() {
    return laeuft;
  }
  onMount(async () => {
    try {
      status = await api.sicherungsstatus();
    } catch (e) {
      fehler = fehlertext(e);
    }
  });
  async function erstellen() {
    laeuft = true;
    fehler = null;
    try {
      const ziel = await save({
        title: "Vollständige Einzelplatzsicherung",
        defaultPath: `BauProjektIO-Sicherung-${new Date().toISOString().replaceAll(":", "-")}.zip`,
        filters: [{ name: "Sicherungsarchiv", extensions: ["zip"] }],
      });
      if (!ziel) return;
      const m = await api.sicherungErstellen(ziel);
      meldung = `Sicherung erstellt und geprüft: ${m.dateien.length} Dateien. ${ziel}`;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function auswahl(): Promise<string | null> {
    const p = await open({
      title: "BauProjektIO-Sicherungsarchiv wählen",
      multiple: false,
      filters: [{ name: "Sicherungsarchiv", extensions: ["zip"] }],
    });
    return typeof p === "string" ? p : null;
  }
  async function pruefen() {
    laeuft = true;
    fehler = null;
    try {
      const p = await auswahl();
      if (!p) return;
      const m = await api.sicherungPruefen(p);
      meldung = `Integrität von ${m.dateien.length} Dateien geprüft. Datenbanklesbarkeit und Profilstart werden bei der Wiederherstellung geprüft.`;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function wiederherstellen() {
    laeuft = true;
    fehler = null;
    try {
      const p = await auswahl();
      if (!p) return;
      const parent = await open({
        title: "Verzeichnis für ein neues, getrenntes Profil",
        directory: true,
        multiple: false,
      });
      if (typeof parent !== "string") return;
      const ziel = parent + `/BauProjektIO-Wiederhergestellt-${Date.now()}`;
      wiederhergestellt = await api.sicherungWiederherstellen(p, ziel);
      meldung = `Wiederhergestellt und geprüft: ${wiederhergestellt}. Das aktuelle Profil bleibt geöffnet.`;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function aktivieren() {
    if (!wiederhergestellt) return;
    laeuft = true;
    try {
      await api.profilAktivieren(wiederhergestellt);
      meldung =
        "Das wiederhergestellte Profil wird beim nächsten Start geöffnet. Die bisherigen Daten bleiben erhalten.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function standard() {
    laeuft = true;
    try {
      await api.standardprofilAktivieren();
      meldung =
        "Beim nächsten Start wird wieder das bisherige Standardprofil geöffnet.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
</script>

<section class="einstellungsabschnitt">
  <h2>Sicherung und Wiederherstellung</h2>
  {#if fehler}<p class="fehlertext" role="alert">
      {fehler}
    </p>{/if}{#if meldung}<p role="status">{meldung}</p>{/if}
  {#if status}<p class="meta">{status.hinweis}</p>
    <p class="meta pfad">Aktives Datenprofil: {status.profil}</p>
    <div class="knoepfe links">
      <button disabled={!darf || !status.lokal || laeuft} onclick={erstellen}
        >Sicherung erstellen …</button
      ><button class="sekundaer" disabled={!darf || laeuft} onclick={pruefen}
        >Archiv prüfen …</button
      ><button
        class="sekundaer"
        disabled={!darf || laeuft}
        onclick={wiederherstellen}>In neues Profil wiederherstellen …</button
      >
    </div>{/if}
  {#if status && !status.lokal}<BueroSicherung darf={!!darf} />{/if}
  {#if wiederhergestellt}<button disabled={!darf || laeuft} onclick={aktivieren}
      >Wiederhergestelltes Profil nach Neustart verwenden</button
    >{/if}
  <button class="link" disabled={laeuft} onclick={standard}
    >Nach Neustart zum Standardprofil zurückkehren</button
  >
  <p class="meta">
    Sicherungen enthalten Projekt- und Kontodaten. Sie sollten im geschützten
    Büro-Backup aufbewahrt werden. Die Wiederherstellung überschreibt keine
    bestehenden Projekte.
  </p>
</section>

<style>
  .pfad {
    overflow-wrap: anywhere;
  }
</style>
