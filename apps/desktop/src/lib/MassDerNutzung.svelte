<script lang="ts">
  import Baugrundstuecke from "./Baugrundstuecke.svelte";
  import type { Project } from "./api";
  import type { BerechnungsBereich } from "./projektSitzung";
  let {
    projekt,
    benutzerId,
    darf,
    onprojekt,
    bereich = "grundstueck",
    bezugsflaechenAktuell = false,
  }: {
    projekt: Project;
    benutzerId: string;
    darf: boolean;
    onprojekt: (p: Project) => void;
    bereich?: BerechnungsBereich;
    bezugsflaechenAktuell?: boolean;
  } = $props();
  let editor = $state<Baugrundstuecke>();
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

<Baugrundstuecke
  bind:this={editor}
  {projekt}
  {benutzerId}
  {darf}
  {onprojekt}
  {bereich}
  {bezugsflaechenAktuell}
/>
