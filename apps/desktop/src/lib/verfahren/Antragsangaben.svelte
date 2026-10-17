<script lang="ts">
  import type {
    Antragsangaben,
    Verfahrensart,
  } from "../generated/plattform";
  let {
    art,
    angaben = $bindable(),
  }: { art: Verfahrensart; angaben?: Antragsangaben } = $props();
  // Lokale Entwürfe älterer Fassungen kennen die Angaben noch nicht.
  $effect.pre(() => {
    angaben ??= { vorbescheidfragen: [], als_bauantrag_weiterfuehren: false };
  });
</script>

{#if angaben && art === "vorbescheid"}<section aria-label="Fragen zum Vorbescheid">
    <h3>Fragen zum Vorbescheid</h3>
    <p class="meta">
      Jede Frage wird in der XBau-Nachricht ein eigener beantragter Vorbescheid.
      Absätze durch Zeilenumbrüche trennen.
    </p>
    {#each angaben.vorbescheidfragen as f, i (f.id)}<div class="frage">
        <label
          >Frage {i + 1}<textarea
            rows="3"
            maxlength="16000"
            bind:value={f.frage}
          ></textarea></label
        ><button
          type="button"
          class="link"
          onclick={() => angaben!.vorbescheidfragen.splice(i, 1)}
          >Frage entfernen</button
        >
      </div>{/each}
    <button
      type="button"
      class="sekundaer"
      onclick={() =>
        angaben!.vorbescheidfragen.push({
          id: crypto.randomUUID(),
          frage: "",
        })}>Frage ergänzen</button
    >
  </section>{:else if angaben && art === "freistellung"}<label
    ><input
      type="checkbox"
      bind:checked={angaben.als_bauantrag_weiterfuehren}
    />Bei abschlägiger Entscheidung der Gemeinde als Bauantrag weiterführen</label
  >{/if}

<style>
  .frage {
    display: flex;
    align-items: flex-end;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
  }
  .frage label {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 0.25rem;
  }
</style>
