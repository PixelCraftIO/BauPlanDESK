<script lang="ts">
  import Flurstuecke from "./Flurstuecke.svelte";
  import type { Project } from "./api";
  import type { BerechnungsBereich } from "./projektSitzung";
  let {
    projekt,
    darf,
    onprojekt,
    bereich = "grundstueck",
  }: {
    projekt: Project;
    darf: boolean;
    onprojekt: (p: Project) => void;
    bereich?: BerechnungsBereich;
  } = $props();
  let editor = $state<Flurstuecke>();
  export function hatAenderungen() {
    return !!editor?.hatAenderungen();
  }
  export function istBeschaeftigt() {
    return !!editor?.istBeschaeftigt();
  }
  export async function speichernBeimSchliessen() {
    return (await editor?.speichernBeimSchliessen()) ?? true;
  }
</script>

<Flurstuecke bind:this={editor} {projekt} {darf} {onprojekt} {bereich} />
