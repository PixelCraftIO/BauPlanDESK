<script lang="ts">
  import Arbeitscheck from "./arbeitsplatz/Arbeitscheck.svelte";
  import Statushilfe from "./arbeitsplatz/Statushilfe.svelte";
  import { abonnieren } from "./projektEreignisse";
  import type { Projektuebersicht } from "./generated/plattform";
  import { untrack } from "svelte";
  import { plattformApi, type Datensatz } from "./plattformApi";
  import { projektNavigation } from "./projektNavigation.svelte";
  import { fehlertext } from "./api";
  import type { Verfahrensakte } from "./generated/plattform";
  let { projektId, aktiv = true, version = 0, entwuerfe = false }: { projektId: string; aktiv?: boolean; version?: number; entwuerfe?: boolean } =
    $props();
  const navigation = projektNavigation();
  let schritteElement = $state<HTMLOListElement>();
  function schrittOeffnen(index: number) {
    const h = schritteElement?.querySelectorAll<HTMLHeadingElement>("h3")[index];
    h?.setAttribute("tabindex", "-1"); h?.focus(); h?.scrollIntoView({block:"start"});
  }
  let akten = $state<Datensatz<Verfahrensakte>[]>([]),
    vid = $state(""),
    fehler = $state(""),
    laeuft = $state(false);
  let pruefstand = $state<Projektuebersicht | null>(null);
  let anfrage = 0;
  async function laden() {
    const nr = ++anfrage, id = projektId;
    laeuft = true;
    fehler = "";
    pruefstand = null;
    try {
      const [aktenNeu, stand] = await Promise.all([plattformApi.liste<Verfahrensakte>(id, "verfahren"), plattformApi.uebersicht(id)]);
      if (nr !== anfrage || id !== projektId) return;
      if (stand.projekt_id !== id) throw Error("Prüfstand gehört zu einem anderen Projekt");
      akten = aktenNeu;
      pruefstand = stand;
      if (!akten.some((a) => a.objekt === vid)) vid = akten[0]?.objekt ?? "";
    } catch (e) {
      if (nr === anfrage) fehler = fehlertext(e);
    } finally {
      if (nr === anfrage) laeuft = false;
    }
  }
  $effect(() => {
    void projektId; void version;
    if (aktiv) untrack(laden);
    return () => { anfrage++; laeuft = false; };
  });
  $effect(() => abonnieren(projektId, ["dokumente", "fachobjekte", "nachweise"], () => { if (aktiv) void laden(); }));
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
  {#if laeuft}<p role="status">Arbeitscheck wird aktualisiert …</p>{:else if pruefstand}<Arbeitscheck daten={pruefstand} {entwuerfe} verfahren={vid} onoeffnen={z=>navigation?.springen(z)}/>{:else}<p class="hinweisbox">Arbeitscheck nicht verfügbar. Die Vollständigkeit ist derzeit nicht auswertbar.</p>{/if}
  <nav class="schrittauswahl" aria-label="Abgabeschritte">
    {#each ["Anforderungen", "Antragsdaten", "Unterlagen", "Abgabe", "Eingangsbeleg"] as titel, i}<button type="button" class="sekundaer" onclick={()=>schrittOeffnen(i)}>{i+1} · {titel}</button>{/each}
  </nav>
  <ol class="einreichschritte" bind:this={schritteElement}>
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
  <Statushilfe/>
  <p class="hinweisbox">
    Export und Datei-Upload bestätigen keinen Eingang. Erst Beleg und
    Dateiabgleich dokumentieren die tatsächliche Abgabe.
  </p>
</section>

<style>
  .schrittauswahl { display:flex; flex-wrap:wrap; gap:.6rem; margin-top:1rem; }
  .einreichschritte h3 { scroll-margin-block:2rem; }
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
    border-radius: 0;
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
