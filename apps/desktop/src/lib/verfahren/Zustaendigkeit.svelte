<script lang="ts">
  import type { Benutzer } from "../api";
  let {
    benutzer,
    wert = $bindable(null),
  }: { benutzer: Benutzer[]; wert?: string | null } = $props();
</script>

<label
  >Verantwortlich<select bind:value={wert}
    ><option value={null}>Noch nicht zugeordnet</option
    >{#each benutzer.filter((b) => b.aktiv) as b}<option value={b.id}
        >{b.anzeigename}</option
      >{/each}</select
  ></label
>
