<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    titel,
    breit = false,
    schliessenErlaubt = true,
    onclose,
    onrequestclose,
    children,
  }: {
    titel: string;
    breit?: boolean;
    schliessenErlaubt?: boolean;
    onclose: () => void;
    onrequestclose?: () => Promise<boolean>;
    children: Snippet;
  } = $props();

  let dialog: HTMLDialogElement;

  $effect(() => {
    dialog.showModal();
    return () => dialog.close();
  });
  function cancel(event: Event) {
    if (!schliessenErlaubt) {
      event.preventDefault();
      return;
    }
    if (onrequestclose) {
      event.preventDefault();
      void onrequestclose().then((ok) => {
        if (ok) onclose();
      });
    }
  }
</script>

<!-- Natives modales <dialog>: Escape schließt, Fokus bleibt im Dialog. -->
<dialog
  bind:this={dialog}
  class:breit
  {onclose}
  oncancel={cancel}
  aria-labelledby="dialog-titel"
>
  <h2 id="dialog-titel">{titel}</h2>
  {@render children()}
</dialog>
