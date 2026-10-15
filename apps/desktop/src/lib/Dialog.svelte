<script lang="ts">
  import { onMount, type Snippet } from "svelte";

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

  // close() stellt ein späteres natives close-Ereignis in die Browser-Warteschlange.
  // Nach dem Abbau darf es keinen inzwischen neu geöffneten Dialog schließen.
  let eingebunden = false;
  onMount(() => {
    eingebunden = true;
    dialog.showModal();
    return () => {
      eingebunden = false;
      dialog.close();
    };
  });
  function geschlossen() {
    if (eingebunden) onclose();
  }
  function cancel(event: Event) {
    if (!schliessenErlaubt) {
      event.preventDefault();
      return;
    }
    event.preventDefault();
    if (onrequestclose) {
      void onrequestclose().then((ok) => {
        if (ok && eingebunden) onclose();
      });
    } else {
      onclose();
    }
  }
</script>

<!-- Natives modales <dialog>: Escape schließt, Fokus bleibt im Dialog. -->
<dialog
  bind:this={dialog}
  class:breit
  onclose={geschlossen}
  oncancel={cancel}
  aria-labelledby="dialog-titel"
>
  <h2 id="dialog-titel">{titel}</h2>
  {@render children()}
</dialog>
