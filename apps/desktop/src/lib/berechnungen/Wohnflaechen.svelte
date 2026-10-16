<script lang="ts">
  import FachBerechnung from "./FachBerechnung.svelte";
  import type { FachProps } from "../fachmodule/register";
  let props: FachProps = $props();
</script>

<FachBerechnung {...props} modul="wohnflaechen" />
