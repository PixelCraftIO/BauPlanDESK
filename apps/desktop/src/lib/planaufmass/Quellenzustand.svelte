<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { Planquellenhinweis } from "../generated/plattform";
  import { ansichtsAbfrage } from "../ansichtsAbfrage.svelte";
  // Dieselben Quellenprüfungen wie beim Speichern, schon beim Öffnen: zeigt,
  // welche verknüpften Werte nicht mehr dem Aufmaß entsprechen. Ohne
  // `bereiche` alle Verbraucher des Projekts (Aufmaß-Editor).
  let { projektId, version = 0, bereiche }: { projektId: string; version?: number; bereiche?: string[] } = $props();
  let hinweise = $state<Planquellenhinweis[]>([]);
  let generation = 0;
  async function laden() {
    const n = ++generation;
    const h = await invoke<Planquellenhinweis[]>("planquellen_zustand", { projektId }).catch(() => [] as Planquellenhinweis[]);
    if (n === generation) hinweise = h;
  }
  const neu = ansichtsAbfrage(() => projektId, ["fachobjekte", "dokumente"], laden);
  $effect(() => {
    void version;
    neu();
  });
  const sichtbar = $derived(bereiche ? hinweise.filter((h) => bereiche.includes(h.bereich)) : hinweise);
</script>
{#if sichtbar.length}<div class="hinweisbox" role="status" aria-label="Aufmaßquellen">
  <strong>{sichtbar.length === 1 ? "Ein verknüpfter Wert entspricht" : `${sichtbar.length} verknüpfte Bereiche entsprechen`} nicht mehr dem Aufmaß.</strong>
  <ul>{#each sichtbar as h (h.bereich + (h.objekt ?? ""))}<li>{bereiche?.length === 1 ? "" : `${h.titel}: `}{h.text}</li>{/each}</ul>
</div>{/if}
