<script lang="ts">
  import { untrack } from "svelte";
  import { plattformApi, type Datensatz } from "./plattformApi";
  import { projektNavigation } from "./projektNavigation.svelte";
  import { fehlertext } from "./api";
  import type { Verfahrensakte } from "./generated/plattform";
  let { projektId, aktiv = true }: { projektId: string; aktiv?: boolean } =
    $props();
  const navigation = projektNavigation();
  let akten = $state<Datensatz<Verfahrensakte>[]>([]),
    vid = $state(""),
    fehler = $state(""),
    laeuft = $state(false);
  async function laden() {
    laeuft = true;
    try {
      akten = await plattformApi.liste(projektId, "verfahren");
      if (!akten.some((a) => a.objekt === vid)) vid = akten[0]?.objekt ?? "";
      fehler = "";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  $effect(() => {
    if (aktiv) untrack(laden);
  });
  function oeffnen(bereich: string, register: string | null = null) {
    navigation?.springen({
      bereich,
      objekt: ["verfahren", "portale"].includes(bereich) ? vid || null : null,
      register,
      eintrag: null,
    });
  }
</script>

<section aria-label="Einreichen und Nachreichen" class="einreichen-start">
  <h2>Einreichen / Nachreichen</h2>
  <p>
    Vom Antrag bis zum belegten Eingang: Die Schritte öffnen die vorhandenen
    Projektbereiche. Unterlagen, Dateiversionen und Versandbelege bleiben in
    ihren Fachmodulen erhalten.
  </p>
  <div class="werkzeugleiste">
    <label
      >Verfahrensakte<select bind:value={vid}
        ><option value="">Verfahren auswählen oder neu anlegen</option
        >{#each akten as a}<option value={a.objekt}
            >{a.daten.bezeichnung} · {a.daten.behoerde.name}</option
          >{/each}</select
      ></label
    ><button class="sekundaer" onclick={laden} disabled={laeuft}
      >Verfahren aktualisieren</button
    >
  </div>
  {#if !vid && !laeuft}<p class="hinweisbox">
      Zuerst eine Verfahrensakte anlegen oder auswählen. Antragsdaten, Formulare
      und Projektdateien können unabhängig davon vorbereitet werden.
    </p>{/if}
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  <ol class="einreichschritte">
    <li>
      <div>
        <h3>1 · Zuständigkeit und Anforderungen klären</h3>
        <p>
          Behörde, Vorhabenprofil und benötigte Unterlagen in der Verfahrensakte
          festlegen.
        </p>
      </div>
      <div class="knoepfe links">
        <button onclick={() => oeffnen("verfahren", "stamm")}
          >Verfahrensakte öffnen</button
        ><button
          class="sekundaer"
          onclick={() => oeffnen("verfahren", "anforderungen")}
          disabled={!vid || laeuft}>Anforderungen prüfen</button
        >
      </div>
    </li>
    <li>
      <div>
        <h3>2 · Antragsdaten und Formulare vorbereiten</h3>
        <p>
          Antragsdaten pflegen und benötigte Vorlagen im Projekt ausfüllen.
          XBau-Dateiaustausch ist ein eigener technischer Schritt.
        </p>
      </div>
      <div class="knoepfe links">
        <button onclick={() => oeffnen("bauantrag")}>Antragsdaten öffnen</button
        ><button class="sekundaer" onclick={() => oeffnen("formulare")}
          >Formulare ausfüllen</button
        >
      </div>
    </li>
    <li>
      <div>
        <h3>3 · Unterlagen und Nachweise zusammenstellen</h3>
        <p>
          Konkrete Dateiversionen, Kategorien und Druckausgabe prüfen.
          Abgabestände werden unveränderlich festgehalten.
        </p>
      </div>
      <div class="knoepfe links">
        <button onclick={() => oeffnen("dokumente")}
          >Projektdateien prüfen</button
        ><button class="sekundaer" onclick={() => oeffnen("druck")}
          >Druckausgabe zusammenstellen</button
        >
      </div>
    </li>
    <li>
      <div>
        <h3>4 · Einreichen oder nachreichen</h3>
        <p>
          Portalziel bestätigen, Dateistand einfrieren und bereitstellen. Bei
          Nachreichungen den belegten Vorgänger auswählen. Die verbindliche
          Abgabe erfolgt im zuständigen Portal.
        </p>
      </div>
      <div class="knoepfe links">
        <button onclick={() => oeffnen("portale")} disabled={!vid || laeuft}
          >Portalübergabe vorbereiten</button
        >
      </div>
    </li>
    <li>
      <div>
        <h3>5 · Eingang und Rückmeldungen dokumentieren</h3>
        <p>
          Originalbeleg und tatsächlich eingereichte Dateiversionen abgleichen.
          Nachforderungen, Fristen und Bescheide anhand der Originale in der
          Verfahrensakte bearbeiten.
        </p>
      </div>
      <div class="knoepfe links">
        <button onclick={() => oeffnen("portale")} disabled={!vid || laeuft}
          >Portalbelege erfassen</button
        ><button
          class="sekundaer"
          onclick={() => oeffnen("verfahren", "kommunikation")}
          disabled={!vid || laeuft}>Kommunikation öffnen</button
        ><button
          class="sekundaer"
          onclick={() => oeffnen("verfahren", "nachforderungen")}
          disabled={!vid || laeuft}>Nachforderungen bearbeiten</button
        >
      </div>
    </li>
  </ol>
  <p class="hinweisbox">
    Export und Datei-Upload bestätigen keinen Eingang. Erst Beleg und
    Dateiabgleich dokumentieren die tatsächliche Abgabe.
  </p>
</section>

<style>
  .einreichen-start {
    max-width: 85rem;
  }
  .einreichschritte {
    list-style: none;
    margin: 1.5rem 0;
    padding: 0;
    display: grid;
    gap: 1.2rem;
  }
  .einreichschritte li {
    display: block;
    border: 1px solid var(--border);
    padding: 1.2rem;
    border-radius: 4px;
  }
  .einreichschritte h3 {
    margin: 0 0 0.5rem;
  }
  .einreichschritte p {
    margin: 0.5rem 0 1rem;
    max-width: 75ch;
  }
  .werkzeugleiste label {
    flex: 1;
    max-width: 45rem;
  }
</style>
