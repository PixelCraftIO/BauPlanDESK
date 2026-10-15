<script lang="ts">
  import { plattformApi } from "./plattformApi";
  import { fehlertext } from "./api";
  import type { Projektstatus } from "./generated/plattform";
  import type { Ansicht } from "./projektSitzung";
  import { MODULE } from "./module";
  let {
    projektId,
    version,
    sichtbar,
    onbereich,
  }: {
    projektId: string;
    version: number;
    sichtbar: boolean;
    onbereich: (id: Ansicht) => void;
  } = $props();
  let status = $state<Projektstatus | null>(null),
    fehler = $state<string | null>(null),
    laeuft = $state(false);
  let anfrage = 0;
  async function laden() {
    const nummer = ++anfrage;
    laeuft = true;
    fehler = null;
    try {
      const neu = await plattformApi.status(projektId);
      if (nummer === anfrage) status = neu;
    } catch (e) {
      if (nummer === anfrage) fehler = fehlertext(e);
    } finally {
      if (nummer === anfrage) laeuft = false;
    }
  }
  $effect(() => {
    void version;
    if (sichtbar) void laden();
  });
  function oeffnen(id: string) {
    if (MODULE.some((m) => m.id === id)) onbereich(id as Ansicht);
  }
</script>

<section aria-label="Projektstatus">
  <div class="zeile">
    <h3>Arbeitsstand</h3>
    <button type="button" class="link" disabled={laeuft} onclick={laden}
      >Aktualisieren</button
    >
  </div>
  {#if fehler}<p class="fehlertext" role="alert">
      Arbeitsstand konnte nicht geladen werden: {fehler}
    </p>{/if}
  {#if status}
    <dl class="daten">
      <dt>Aufgaben</dt>
      <dd>
        {status.offene_aufgaben} offen · {status.ueberfaellige_aufgaben} überfällig
      </dd>
      <dt>Erforderliche Unterlagen</dt>
      <dd>
        {status.erforderliche_unterlagen} erfasst · {status.fehlende_unterlagen}
        fehlen · {status.ungepruefte_unterlagen} zu prüfen
      </dd>
      <dt>Nachweisstände</dt>
      <dd>
        {status.nachweise} gespeichert · {status.veraltete_nachweise} veraltet ·
        {status.offene_freigaben} ohne Freigabe
      </dd>
      <dt>Einreichungen</dt>
      <dd>
        {status.exportierte_pakete} Pakete · {status.bestaetigte_einreichungen} als
        eingereicht bestätigt
      </dd>
    </dl>
    {#if status.hinweise.length}<ul>
        {#each status.hinweise as h}<li>
            <button
              type="button"
              class="link"
              onclick={() => oeffnen(h.bereich)}>{h.text}</button
            >
          </li>{/each}
      </ul>{/if}
    <p class="meta">
      Stand {new Date(status.ermittelt_am).toLocaleString("de-DE")}. Die
      Übersicht bewertet erfasste Daten; nicht angelegte Anforderungen sind
      dadurch nicht geprüft.
    </p>
  {/if}
</section>
