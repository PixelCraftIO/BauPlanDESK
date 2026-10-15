<script lang="ts">
  import Sicherungen from "./Sicherungen.svelte";
  let sicherungen = $state<Sicherungen>();
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext, type Benutzer } from "./api";
  let { ich }: { ich: Benutzer | null } = $props();
  let formularRoot = $state(""),
    projektRoot = $state(""),
    laeuft = $state(false),
    fehler = $state<string | null>(null),
    hinweis = $state<string | null>(null);
  export function istBeschaeftigt() {
    return laeuft || !!sicherungen?.istBeschaeftigt();
  }
  async function laden() {
    try {
      const [f, p] = await Promise.all([
        api.formularbibliothek(),
        api.projektordnerStatus(),
      ]);
      formularRoot = f.root;
      projektRoot = p;
    } catch (e) {
      fehler = fehlertext(e);
    }
  }
  onMount(laden);
  async function waehlen(art: "formulare" | "projekte") {
    laeuft = true;
    fehler = null;
    try {
      const pfad = await open({
        directory: true,
        multiple: false,
        title:
          art === "formulare"
            ? "Formularordner auswählen"
            : "Projektordner auswählen",
        defaultPath: art === "formulare" ? formularRoot : projektRoot,
      });
      if (typeof pfad !== "string") return;
      if (art === "formulare") await api.formularordnerWaehlen(pfad);
      else await api.projektordnerWaehlen(pfad);
      await laden();
      hinweis =
        "Speicherort gespeichert. Bestehende Dateien bleiben am bisherigen Ort; neue Ausgaben verwenden die neue Auswahl.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
</script>

<section class="einstellungsabschnitt">
  <h2>Speicherorte</h2>
  {#if fehler}<p class="fehlertext" role="alert">
      {fehler}
    </p>{/if}{#if hinweis}<p role="status">{hinweis}</p>{/if}
  <div class="speicherort">
    <div>
      <h3>PDF-Formularvorlagen</h3>
      <p>{formularRoot || "Wird geladen …"}</p>
      <p class="meta">
        Projektunabhängige Bibliothek, lokal oder auf einer Serverfreigabe.
      </p>
    </div>
    <button
      class="sekundaer"
      disabled={laeuft || ich?.rolle !== "administrator"}
      onclick={() => waehlen("formulare")}>Formularordner auswählen …</button
    >
  </div>
  <div class="speicherort">
    <div>
      <h3>Lesbare Projektordner</h3>
      <p>{projektRoot || "Wird geladen …"}</p>
      <p class="meta">
        Importierte Dateien und gespeicherte Formulare werden automatisch
        projektweise unter Dokumente beziehungsweise Pläne ausgegeben. Versionen
        bleiben getrennt erhalten.
      </p>
    </div>
    <button
      class="sekundaer"
      disabled={laeuft || ich?.rolle !== "administrator"}
      onclick={() => waehlen("projekte")}>Projektordner auswählen …</button
    >
  </div>
  <p class="meta">
    Die verwaltete Ablage darunter enthält die verbindlichen Dokumentversionen.
    Projektordner sind lesbare Ausgaben; Änderungen daran werden ausdrücklich
    als neue Version importiert.
  </p>
</section>

<Sicherungen bind:this={sicherungen} {ich} />

<style>
  .speicherort {
    display: flex;
    gap: 1.2rem;
    align-items: center;
    justify-content: space-between;
    margin: 1.2rem 0;
  }
  .speicherort > button {
    flex-shrink: 0;
  }
  .speicherort > div {
    min-width: 0;
  }
  .speicherort p {
    overflow-wrap: anywhere;
  }
</style>
