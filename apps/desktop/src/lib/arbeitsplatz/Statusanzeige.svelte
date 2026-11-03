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
    /* Status, kein Knopf: ohne Rahmen, damit es nicht wie ein
       anklickbares Element aussieht. */
    padding: 0.15rem 0.45rem;
    border: 0;
    border-radius: 2px;
    background: transparent;
    color: var(--fg);
    cursor: default;
    font-size: 0.92rem;
    font-weight: 600;
  }
  .warnung {
    background: var(--warn-bg);
    color: var(--warning-text);
  }
  .erfolg {
    background: var(--ok-bg);
    color: var(--ok-text);
  }
</style>
