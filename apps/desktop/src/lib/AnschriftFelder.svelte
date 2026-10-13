<script lang="ts">
  import type { Anschrift } from "./api";

  let { anschrift = $bindable(), gesperrt = false }: { anschrift: Anschrift; gesperrt?: boolean } = $props();
</script>

<div class="anschrift">
  <label class="breit">Straße <input bind:value={anschrift.strasse} disabled={gesperrt} /></label>
  <label>Nr. <input bind:value={anschrift.hausnummer} disabled={gesperrt} size="5" /></label>
  <label>PLZ <input bind:value={anschrift.postleitzahl} disabled={gesperrt} size="6" /></label>
  <label class="breit">Ort <input bind:value={anschrift.ort} disabled={gesperrt} /></label>
</div>
