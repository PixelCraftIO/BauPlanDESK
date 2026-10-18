<script lang="ts">
  import { wertText, type Abweichung } from "./abgleich";

  let {
    abweichungen,
    titel = (pfad: string) => pfad,
    gesperrt = false,
    onuebernehmen,
  }: {
    abweichungen: Abweichung[];
    /** Anzeigename eines Feldpfads. */
    titel?: (pfad: string) => string;
    gesperrt?: boolean;
    /** Den Wert der aktuellen Fassung für dieses Feld übernehmen. */
    onuebernehmen: (pfad: string) => void;
  } = $props();
</script>

{#if abweichungen.length}
  <div class="abweichungen" role="group" aria-label="Abweichungen zur aktuellen Fassung">
    <p>
      Diese Felder wurden auch am anderen Arbeitsplatz geändert. Ihre Eingabe
      steht im Formular; beim Speichern ersetzt sie den aktuellen Wert.
    </p>
    <ul>
      {#each abweichungen as a (a.pfad)}
        <li>
          <span><strong>{titel(a.pfad)}</strong>: aktuell {wertText(a.aktuell)}</span>
          <button
            type="button"
            class="sekundaer"
            disabled={gesperrt}
            onclick={() => onuebernehmen(a.pfad)}
            aria-label={`Aktuellen Wert für ${titel(a.pfad)} übernehmen`}
            >Aktuellen Wert übernehmen</button
          >
        </li>
      {/each}
    </ul>
  </div>
{/if}

<style>
  .abweichungen {
    border: 1px solid var(--border);
    border-left: 3px solid var(--warning-text);
    background: var(--warn-bg);
    padding: 0.5rem 0.75rem;
    border-radius: 4px;
  }
  .abweichungen ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .abweichungen li {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
    justify-content: space-between;
  }
</style>
