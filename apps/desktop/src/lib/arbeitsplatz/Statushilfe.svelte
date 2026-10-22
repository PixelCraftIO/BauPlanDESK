<script lang="ts">
  import { STATUS, type Arbeitsstatus } from "./status";
  import Statusanzeige from "./Statusanzeige.svelte";
</script>

<details class="statushilfe">
  <summary>Was bedeuten die Bearbeitungsstände?</summary>
  <dl>
    {#each Object.entries(STATUS) as [id, s]}<div>
        <dt><Statusanzeige status={id as Arbeitsstatus} /></dt>
        <dd>{s.text}</dd>
      </div>{/each}
  </dl>
</details>

<style>
  .statushilfe {
    margin-block: 1rem;
    border: 1px solid var(--border);
    padding: 0.5rem 1rem;
  }
  dl {
    display: grid;
    gap: 0.75rem;
  }
  dl div {
    display: grid;
    grid-template-columns: minmax(12rem, 1fr) 3fr;
    gap: 1rem;
  }
  dd {
    margin: 0;
  }
  @media (max-width: 800px) {
    dl div {
      grid-template-columns: 1fr;
      gap: 0.4rem;
    }
  }
</style>
