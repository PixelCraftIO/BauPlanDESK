<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    titel,
    onclose,
    children,
  }: { titel: string; onclose: () => void; children: Snippet } = $props();

  let dialog: HTMLDialogElement;

  $effect(() => {
    dialog.showModal();
    return () => dialog.close();
  });
</script>

<!-- Natives modales <dialog>: Escape schließt, Fokus bleibt im Dialog. -->
<dialog bind:this={dialog} onclose={onclose} aria-labelledby="dialog-titel">
  <h2 id="dialog-titel">{titel}</h2>
  {@render children()}
</dialog>
