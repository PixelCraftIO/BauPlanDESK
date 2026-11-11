<script lang="ts">
  // Zeigt nach dem ersten Start, was dem Büro für den ersten Bauantrag noch
  // fehlt (Befund 3 der Gesamtanalyse). Ist alles eingerichtet, bleibt die
  // Fläche leer.
  import { api, fehlertext, type Buero, type Formularbibliothek } from "./api";
  let {
    aktualisieren = 0,
    darf = true,
    onbuero,
    onformulare,
  }: {
    /** Neu laden, wenn sich der Wert ändert (z. B. nach dem Schließen eines Dialogs). */
    aktualisieren?: number;
    darf?: boolean;
    onbuero: () => void;
    onformulare: () => void;
  } = $props();
  let buero = $state<Buero | null>(null),
    bibliothek = $state<Formularbibliothek | null>(null),
    fehler = $state<string | null>(null),
    laeuft = $state(false);
  async function laden() {
    try {
      [buero, bibliothek] = await Promise.all([
        api.buero(),
        api.formularbibliothek(),
      ]);
    } catch {
      // Ohne Auskunft kein Hinweis: Die Einrichtung ist dann nicht beurteilbar.
      buero = null;
      bibliothek = null;
    }
  }
  $effect(() => {
    void aktualisieren;
    void laden();
  });
  const werksvorlagen = $derived(
    (bibliothek?.werksvorlagen ?? []).filter((w) => !w.vorhanden),
  );
  const schritte = $derived(
    buero && bibliothek
      ? [
          {
            id: "buero",
            titel: "Büroangaben",
            text: "Name und Anschrift des Büros stehen in Formularen, Anschreiben und im Plankopf.",
            fertig: !!buero.name.trim() && !!buero.anschrift.ort.trim(),
          },
          {
            id: "entwurfsverfasser",
            titel: "Entwurfsverfasserin oder Entwurfsverfasser",
            text: "Die bauvorlageberechtigte Person mit Kammer und Mitgliedsnummer wird im Bauantrag gewählt.",
            fertig: buero.entwurfsverfasser.length > 0,
          },
          {
            id: "formulare",
            titel: "Formularvorlagen",
            text: bibliothek.vorlagen.length
              ? `${bibliothek.vorlagen.length} ${bibliothek.vorlagen.length === 1 ? "Vorlage steht" : "Vorlagen stehen"} in der Formularbibliothek.`
              : werksvorlagen.length
              ? `${werksvorlagen.length} amtliche ${werksvorlagen.length === 1 ? "Vordruck liegt" : "Vordrucke liegen"} der Anwendung bei und ${werksvorlagen.length === 1 ? "ist" : "sind"} bereits mit den Projektdaten verknüpft.`
              : "Amtliche Vordrucke als PDF in die Formularbibliothek legen.",
            fertig: bibliothek.vorlagen.length > 0,
          },
        ]
      : [],
  );
  const offen = $derived(schritte.filter((s) => !s.fertig));
  async function vordruckeUebernehmen() {
    laeuft = true;
    fehler = null;
    try {
      await api.werksvorlagenUebernehmen();
      await laden();
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
</script>

{#if offen.length}
  <section class="ersteinrichtung" aria-labelledby="ersteinrichtung-titel">
    <h2 id="ersteinrichtung-titel">Einrichtung abschließen</h2>
    <p class="meta">
      Noch {offen.length} von {schritte.length}
      {offen.length === 1 ? "Schritt" : "Schritten"} offen. Ohne diese Angaben
      bleiben Bauantrag und Formulare später unvollständig.
    </p>
    {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
    <ol>
      {#each schritte as s (s.id)}
        <li class:fertig={s.fertig}>
          <span class="marke" aria-hidden="true">{s.fertig ? "✓" : "○"}</span>
          <div>
            <strong>{s.titel}</strong>
            <span class="sr-only">{s.fertig ? "erledigt" : "offen"}</span>
            <p class="meta">{s.text}</p>
          </div>
          {#if !s.fertig}
            {#if s.id === "formulare" && werksvorlagen.length}
              <button disabled={!darf || laeuft} onclick={vordruckeUebernehmen}
                >Mitgelieferte Vordrucke übernehmen</button
              >
            {:else if s.id === "formulare"}
              <button class="sekundaer" onclick={onformulare}
                >Formularbibliothek öffnen</button
              >
            {:else}
              <button class="sekundaer" onclick={onbuero}
                >Büroangaben öffnen</button
              >
            {/if}
          {/if}
        </li>
      {/each}
    </ol>
  </section>
{/if}

<style>
  .ersteinrichtung {
    border: 1px solid var(--border);
    border-left: 4px solid var(--accent);
    padding: 12px 16px;
    margin-bottom: 16px;
    background: var(--panel);
  }
  h2 {
    margin: 0 0 4px;
    font-size: 1.05rem;
  }
  ol {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
  }
  li {
    display: grid;
    grid-template-columns: 24px 1fr auto;
    gap: 8px;
    align-items: center;
    padding: 8px 0;
    border-top: 1px solid var(--border-innen);
  }
  li p {
    margin: 2px 0 0;
  }
  li.fertig strong {
    font-weight: 500;
  }
  .marke {
    font-size: 1.1rem;
    text-align: center;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
