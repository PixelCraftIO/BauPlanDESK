<script lang="ts">
  import type { Dokumentkategorie } from "./api";
  import { kategorienOptionen } from "./kategorien";
  let {
    kategorien,
    wert = $bindable(""),
    disabled = false,
    label = "Kategorie",
    onchange,
  }: {
    kategorien: Dokumentkategorie[];
    wert?: string;
    disabled?: boolean;
    label?: string;
    onchange?: (id: string) => void;
  } = $props();
  const optionen = $derived(kategorienOptionen(kategorien));
</script>

<label
  >{label}<select
    aria-label={label}
    bind:value={wert}
    {disabled}
    onchange={() => onchange?.(wert)}
  >
    <option value="">Ohne Kategorie</option>
    {#each optionen as k (k.id)}
      {#if k.aktiv || wert === k.id}<option value={k.id} disabled={!k.aktiv}
          >{k.bezeichnung}{!k.aktiv ? " (archiviert)" : ""}</option
        >{/if}
    {/each}
  </select></label
>
