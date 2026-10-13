<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext, type DocumentVersion } from "./api";

  let {
    projektId,
    importAusloeser,
    onfehler,
  }: { projektId: string; importAusloeser: number; onfehler: (m: string) => void } = $props();

  let versionen = $state<DocumentVersion[]>([]);
  let laeuft = $state(false);

  // Je Dokument nur die neueste Version anzeigen, ältere als Historie zählen.
  const dokumente = $derived.by(() => {
    const neueste = new Map<string, { doc: DocumentVersion; anzahl: number }>();
    for (const v of versionen) {
      const bisher = neueste.get(v.document_id);
      if (!bisher || v.version > bisher.doc.version) {
        neueste.set(v.document_id, { doc: v, anzahl: (bisher?.anzahl ?? 0) + 1 });
      } else {
        bisher.anzahl++;
      }
    }
    return [...neueste.values()].sort((a, b) => a.doc.file_name.localeCompare(b.doc.file_name, "de"));
  });

  async function laden() {
    try {
      versionen = await api.dokumente(projektId);
    } catch (e) {
      onfehler(fehlertext(e));
    }
  }

  export async function importieren(dokumentId: string | null = null) {
    const auswahl = await open({
      multiple: dokumentId === null,
      title: dokumentId ? "Neue Version auswählen" : "Dokumente importieren",
      filters: [
        { name: "Pläne und Formulare", extensions: ["pdf", "dxf", "dwg", "xml", "png", "jpg"] },
        { name: "Alle Dateien", extensions: ["*"] },
      ],
    });
    if (!auswahl) return;
    const pfade = Array.isArray(auswahl) ? auswahl : [auswahl];
    laeuft = true;
    try {
      for (const pfad of pfade) await api.dokumentImportieren(projektId, pfad, dokumentId);
    } catch (e) {
      onfehler(fehlertext(e));
    } finally {
      laeuft = false;
      await laden();
    }
  }

  $effect(() => {
    void projektId;
    laden();
  });

  // Menü "Datei → Dokument importieren" zählt diesen Wert hoch.
  let letzterAusloeser = 0;
  $effect(() => {
    if (importAusloeser > letzterAusloeser) {
      letzterAusloeser = importAusloeser;
      importieren();
    }
  });

  const groesse = (b: number) =>
    b < 1024 * 1024 ? `${(b / 1024).toFixed(0)} KB` : `${(b / 1024 / 1024).toFixed(1)} MB`;
</script>

<div class="knoepfe links">
  <button onclick={() => importieren()} disabled={laeuft}>{laeuft ? "Importiere …" : "Dokumente importieren …"}</button>
</div>

<table class="ergebnistabelle">
  <thead><tr><th>Datei</th><th>Version</th><th>Größe</th><th>Geändert</th><th></th></tr></thead>
  <tbody>
    {#each dokumente as { doc, anzahl } (doc.document_id)}
      <tr>
        <td>{doc.file_name}</td>
        <td>{doc.version}{anzahl > 1 ? ` (${anzahl} Versionen)` : ""}</td>
        <td>{groesse(doc.size_bytes)}</td>
        <td>{new Date(doc.created_at).toLocaleString("de-DE")} · {doc.created_by}</td>
        <td><button class="link" onclick={() => importieren(doc.document_id)}>Neue Version …</button></td>
      </tr>
    {:else}
      <tr><td colspan="5" class="meta">Noch keine Dokumente. Ziehen Sie Dateien über „Datei → Dokument importieren“ ins Projekt.</td></tr>
    {/each}
  </tbody>
</table>
