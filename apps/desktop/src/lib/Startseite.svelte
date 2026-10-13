<script lang="ts">
  import type { ZuletztEintrag } from "./api";

  let {
    zuletzt,
    darfAnlegen = true,
    onneu,
    onoeffnen,
    onzuletzt,
  }: {
    zuletzt: ZuletztEintrag[];
    darfAnlegen?: boolean;
    onneu: () => void;
    onoeffnen: () => void;
    onzuletzt: (id: string) => void;
  } = $props();
</script>

<div class="start">
  <section>
    <h2>Willkommen</h2>
    <p class="meta">Legen Sie ein neues Bauvorhaben an oder öffnen Sie ein bestehendes.</p>
    <div class="knoepfe links">
      <button onclick={onneu} disabled={!darfAnlegen} title={darfAnlegen ? "" : "Projekte anlegen darf die Projektleitung."}>Neues Projekt …</button>
      <button class="sekundaer" onclick={onoeffnen}>Projekt öffnen …</button>
    </div>
  </section>
  <section>
    <h2>Zuletzt geöffnet</h2>
    <ul>
      {#each zuletzt as z (z.id)}
        <li><button class="link" onclick={() => onzuletzt(z.id)}>{z.name}</button></li>
      {:else}
        <li class="meta">Noch keine Projekte geöffnet.</li>
      {/each}
    </ul>
  </section>
</div>
