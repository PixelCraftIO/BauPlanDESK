<script lang="ts">
  let { titel, wert, min, onwert, readonly=false }: { titel: string; wert?: number; min?: number; readonly?:boolean; onwert: (v: number | undefined) => void } = $props();
</script>
<label>{titel}<input type="number" step="any" {min} {readonly} title={readonly?"Aus Aufmaß verknüpft":undefined} value={wert ?? ""} placeholder="Noch unbekannt" oninput={(e) => onwert(e.currentTarget.value === "" ? undefined : e.currentTarget.valueAsNumber)} /></label>
