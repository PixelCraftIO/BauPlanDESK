<script lang="ts">
  let { titel, wert, min, onwert }: { titel: string; wert?: number; min?: number; onwert: (v: number | undefined) => void } = $props();
</script>
<label>{titel}<input type="number" step="any" {min} value={wert ?? ""} placeholder="Noch unbekannt" oninput={(e) => onwert(e.currentTarget.value === "" ? undefined : e.currentTarget.valueAsNumber)} /></label>
