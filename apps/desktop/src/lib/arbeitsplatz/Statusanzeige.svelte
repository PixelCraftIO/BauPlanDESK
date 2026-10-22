<script lang="ts">
  import { STATUS, type Arbeitsstatus } from "./status";
  let { status }: { status: Arbeitsstatus } = $props();
  const s = $derived(STATUS[status]);
</script>

<span
  class="arbeitsstatus"
  class:warnung={s.art === "warnung"}
  class:erfolg={s.art === "erfolg"}
  title={s.text}
>
  <span aria-hidden="true">{s.symbol}</span>
  {s.titel}
</span>

<style>
  .arbeitsstatus {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.2rem 0.55rem;
    border: 1px solid var(--border);
    background: var(--leiste);
    color: var(--fg);
    font-size: 0.92rem;
    font-weight: 600;
  }
  .warnung {
    background: var(--warn-bg);
    color: var(--warning-text);
    border-color: var(--warning-text);
  }
  .erfolg {
    background: var(--ok-bg);
    color: var(--ok-text);
  }
</style>
