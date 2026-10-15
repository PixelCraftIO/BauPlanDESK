<script lang="ts">
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
  let auswahl = $state("");
  function add() {
    const d = dateien.find((d) => `${d.document_id}:${d.version}` === auswahl);
    if (
      d &&
      !werte.some(
        (w) => w.dokument_id === d.document_id && w.version === d.version,
      )
    )
      werte = [...werte, { dokument_id: d.document_id, version: d.version }];
    auswahl = "";
  }
  const name = (w: Dokumentbezug) =>
    `${dateien.find((d) => d.document_id === w.dokument_id && d.version === w.version)?.file_name ?? w.dokument_id} · Version ${w.version}`;
</script>

<label
  >{label}<select bind:value={auswahl}
    ><option value="">Dokumentversion wählen</option
    >{#each dateien as d (`${d.document_id}:${d.version}`)}<option
        value={`${d.document_id}:${d.version}`}
        >{d.file_name} · Version {d.version}</option
      >{/each}</select
  ></label
><button type="button" class="sekundaer" disabled={!auswahl} onclick={add}
  >Unterlage zuordnen</button
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
