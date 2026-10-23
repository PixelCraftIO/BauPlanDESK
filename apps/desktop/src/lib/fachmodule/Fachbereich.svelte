<script lang="ts">
  import { untrack, type Component } from "svelte";
  import { bereichAktivSetzen } from "../ansichtsAbfrage.svelte";
  import type { FachProps } from "./register";
  let {
    laden,
    aktiv,
    ...props
  }: FachProps & {
    aktiv: boolean;
    laden: () => Promise<{ default: Component<FachProps> }>;
  } = $props();
  bereichAktivSetzen(() => aktiv);
  let komponente = $state<Component<FachProps>>(),
    fehler = $state("");
  async function starten() {
    fehler = "";
    try {
      komponente = (await laden()).default;
    } catch {
      fehler =
        "Der Arbeitsbereich konnte nicht geladen werden. Bitte erneut versuchen.";
    }
  }
  $effect(() => {
    untrack(() => void starten());
  });
</script>

{#if komponente}{@const Inhalt = komponente}<Inhalt {...props} />
{:else if fehler}<p role="alert">{fehler}</p>
  <button onclick={starten}>Erneut laden</button>
{:else}<p role="status">Arbeitsbereich wird geladen …</p>{/if}
