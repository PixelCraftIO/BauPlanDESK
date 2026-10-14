<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    titel,
    breit = false,
    schliessenErlaubt = true,
    onclose,
    children,
  }: { titel: string; breit?: boolean; schliessenErlaubt?: boolean; onclose: () => void; children: Snippet } = $props();

  let dialog: HTMLDialogElement;

  $effect(() => {
    dialog.showModal();
    return () => dialog.close();
  });
</script>

<!-- Natives modales <dialog>: Escape schließt, Fokus bleibt im Dialog. -->
<dialog bind:this={dialog} class:breit onclose={onclose} oncancel={(e)=>{if(!schliessenErlaubt)e.preventDefault();}} aria-labelledby="dialog-titel">
  <h2 id="dialog-titel">{titel}</h2>
  {@render children()}
</dialog>
