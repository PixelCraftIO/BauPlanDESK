<script lang="ts">
  import Dokumentwahl from "./Dokumentwahl.svelte";
  import type { DocumentVersion } from "../api";
  import type { Dokumentbezug } from "../generated/plattform";
  let {
    dateien,
    werte = $bindable(),
    label = "Zugeordnete Unterlagen",
  }: {
    dateien: DocumentVersion[];
    werte: Dokumentbezug[];
    label?: string;
  } = $props();
  let auswahl = $state<Dokumentbezug | null>(null);
  function add() {
    const d = dateien.find(
      (d) =>
        d.document_id === auswahl?.dokument_id &&
        d.version === auswahl?.version,
    );
    if (
      d &&
      !werte.some(
        (w) => w.dokument_id === d.document_id && w.version === d.version,
      )
    )
      werte = [...werte, { dokument_id: d.document_id, version: d.version }];
    auswahl = null;
  }
  const name = (w: Dokumentbezug) =>
    `${dateien.find((d) => d.document_id === w.dokument_id && d.version === w.version)?.file_name ?? "Dokument nicht auflösbar"} · Version ${w.version}`;
</script>

<Dokumentwahl {dateien} bind:wert={auswahl} {label} /><button
  type="button"
  class="sekundaer"
  disabled={!auswahl}
  onclick={add}>Unterlage zuordnen</button
>
<ul>
  {#each werte as w, i (`${w.dokument_id}:${w.version}`)}<li>
      {name(w)}
      <button
        type="button"
        class="link"
        onclick={() => (werte = werte.filter((_, n) => n !== i))}
        >Zuordnung entfernen</button
      >
    </li>{/each}
</ul>
