<script lang="ts">
  import type { DocumentVersion } from "../api";
  import type { Dokumentbezug } from "../generated/plattform";
  let {
    dateien,
    wert = $bindable(null),
    label = "Dokumentversion",
    required = false,
  }: {
    dateien: DocumentVersion[];
    wert?: Dokumentbezug | null;
    label?: string;
    required?: boolean;
  } = $props();
</script>

<label
  >{label}<select
    {required}
    value={wert ? `${wert.dokument_id}:${wert.version}` : ""}
    onchange={(e) => {
      const d = dateien.find(
        (d) => `${d.document_id}:${d.version}` === e.currentTarget.value,
      );
      wert = d ? { dokument_id: d.document_id, version: d.version } : null;
    }}
    ><option value="">Kein Dokument zugeordnet</option
    >{#each dateien as d (`${d.document_id}:${d.version}`)}<option
        value={`${d.document_id}:${d.version}`}
        >{d.file_name} · Version {d.version}</option
      >{/each}</select
  ></label
>
