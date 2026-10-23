<script lang="ts">
  import { ask, save } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext, type DocumentVersion } from "./api";

  let {
    dokument,
    bezugsVersion = null,
    darfWiederherstellen,
    onwiederhergestellt,
    onfehler,
    onschliessen,
  }: {
    dokument: DocumentVersion;
    bezugsVersion?: number | null;
    darfWiederherstellen: boolean;
    onwiederhergestellt: (v: DocumentVersion) => void;
    onfehler: (e: unknown) => void;
    onschliessen: () => void;
  } = $props();

  let versionen = $state<DocumentVersion[]>([]);
  let fehler = $state<string | null>(null);
  let laeuft = $state(false);
  let mehr = $state(false);

  let generation = 0;
  async function laden(weitere = false) {
    const nr = ++generation, id = dokument.document_id;
    laeuft = true;
    try {
      let v = await api.historienseite(id, weitere ? versionen.at(-1)?.version ?? null : null);
      if (nr !== generation) return;
      v = [...v].sort((a,b) => b.version - a.version);
      // Ein gezielt verknüpfter Altstand bleibt auch außerhalb der ersten Seite erreichbar.
      if (!weitere && bezugsVersion !== null && !v.some(d => d.version === bezugsVersion)) {
        fehler = "Der verknüpfte Altstand liegt außerhalb der ersten Seite. Über ‚Ältere Versionen laden‘ ist er erreichbar.";
      }
      versionen = weitere ? [...versionen,...v] : v;
      mehr = v.length === 50;
    } catch(e) { if (nr === generation) fehler = fehlertext(e); }
    finally { if(nr===generation) laeuft=false; }
  }
  $effect(() => { void dokument.document_id; void bezugsVersion; void laden(); return () => { generation++; }; });

  const neueste = $derived(versionen[0]?.version ?? dokument.version);

  async function exportieren(v: DocumentVersion) {
    const punkt = v.file_name.lastIndexOf(".");
    const vorschlag = punkt > 0 ? `${v.file_name.slice(0, punkt)} (Version ${v.version})${v.file_name.slice(punkt)}` : `${v.file_name} (Version ${v.version})`;
    const ziel = await save({ title: `Version ${v.version} exportieren`, defaultPath: vorschlag });
    if (!ziel) return;
    try {
      await api.dokumentExportieren(v.document_id, ziel, v.version);
    } catch (e) {
      fehler = fehlertext(e);
    }
  }

  async function wiederherstellen(v: DocumentVersion) {
    if (!(await ask(`Version ${v.version} als neue Version ${neueste + 1} ablegen? Die bisherigen Versionen bleiben erhalten.`, { title: "Version wiederherstellen", okLabel: "Wiederherstellen", cancelLabel: "Abbrechen" }))) return;
    laeuft = true;
    try {
      onwiederhergestellt(await api.versionWiederherstellen(v.document_id, v.version));
    } catch (e) {
      onfehler(e);
    } finally {
      laeuft = false;
    }
  }
</script>

{#if fehler}<p class="fehlertext">{fehler}</p>{/if}

{#if bezugsVersion !== null}<p class="hinweisbox">Verknüpfter Dokumentstand: Version {bezugsVersion}. Neuere Versionen ersetzen einen bereits dokumentierten Abgabestand nicht automatisch.</p>{/if}
<ul class="auswahlliste">
  {#each versionen as v (v.version)}
    <li class:aktiv={v.version === bezugsVersion}>
      <strong>Version {v.version}{v.version === neueste ? " (aktuell)" : ""}</strong>
      <span class="meta">{new Date(v.created_at).toLocaleString("de-DE")} · {v.created_by} · {v.file_name}</span>
      <span class="aktionen">
        <button class="link" onclick={() => exportieren(v)}>Exportieren …</button>
        {#if darfWiederherstellen && v.version !== neueste}
          <button class="link" disabled={laeuft} onclick={() => wiederherstellen(v)}>Wiederherstellen</button>
        {/if}
      </span>
    </li>
  {/each}
</ul>

{#if mehr}<button class="sekundaer" disabled={laeuft} onclick={() => laden(true)}>Ältere Versionen laden</button>{/if}
<div class="knoepfe">
  <button class="sekundaer" onclick={onschliessen}>Schließen</button>
</div>
