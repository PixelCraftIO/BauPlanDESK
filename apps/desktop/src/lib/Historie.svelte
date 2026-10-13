<script lang="ts">
  import { ask, save } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext, type DocumentVersion } from "./api";

  let {
    dokument,
    darfWiederherstellen,
    onwiederhergestellt,
    onfehler,
    onschliessen,
  }: {
    dokument: DocumentVersion;
    darfWiederherstellen: boolean;
    onwiederhergestellt: (v: DocumentVersion) => void;
    onfehler: (e: unknown) => void;
    onschliessen: () => void;
  } = $props();

  let versionen = $state<DocumentVersion[]>([]);
  let fehler = $state<string | null>(null);
  let laeuft = $state(false);

  $effect(() => {
    api
      .historie(dokument.document_id)
      .then((v) => (versionen = [...v].sort((a, b) => b.version - a.version)))
      .catch((e) => (fehler = fehlertext(e)));
  });

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

<ul class="auswahlliste">
  {#each versionen as v (v.version)}
    <li>
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

<div class="knoepfe">
  <button class="sekundaer" onclick={onschliessen}>Schließen</button>
</div>
