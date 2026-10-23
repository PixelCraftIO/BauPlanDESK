<script lang="ts">
  import Dokumentwahl from "./Dokumentwahl.svelte";
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

<Dokumentwahl {dateien} bind:wert {label} {required} />
