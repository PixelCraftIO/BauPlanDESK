<script lang="ts">
  import type { Snippet } from "svelte";
  import { bereichAktivSetzen } from "./ansichtsAbfrage.svelte";
  let { aktiv, children }: { aktiv: boolean; children: Snippet } = $props();
  bereichAktivSetzen(() => aktiv);
</script>

{@render children()}
