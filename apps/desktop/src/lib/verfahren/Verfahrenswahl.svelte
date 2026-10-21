<script lang="ts">
  import type {
    Pruefverfahren,
    Verfahrensart,
    Verfahrenswahl,
  } from "../generated/plattform";
  import { fehlertext } from "../api";
  import { verfahrenApi } from "./api";
  let {
    projektId,
    verfahrenId,
    onuebernehmen,
  }: {
    projektId: string;
    verfahrenId: string | null;
    onuebernehmen: (art: Verfahrensart, pruef: Pruefverfahren | null) => void;
  } = $props();
  let wohngebaeude = $state(""),
    grosserSonderbau = $state(""),
    erschliessung = $state(""),
    gemeinde = $state(""),
    stichtag = $state(""),
    wahl = $state<Verfahrenswahl | null>(null),
    fehler = $state(""),
    laedt = $state(false);
  const jaNein = (v: string) => (v === "" ? null : v === "true");
  async function pruefen() {
    laedt = true;
    fehler = "";
    try {
      wahl = await verfahrenApi.verfahrenswahl(projektId, verfahrenId, {
        wohngebaeude: jaNein(wohngebaeude),
        grosser_sonderbau: jaNein(grosserSonderbau),
        erschliessung: jaNein(erschliessung),
        gemeinde_verzicht: jaNein(gemeinde),
        stichtag: stichtag || null,
      });
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laedt = false;
    }
  }
  const EIGNUNG = {
    moeglich: "Bedingungen belegt",
    ausgeschlossen: "Ausgeschlossen",
    pruefen: "Offen – prüfen",
  } as const;
</script>

<h3>Verfahrenswahl</h3>
<p class="meta">
  Bedingungen je Verfahren mit Fundstelle. Was das Projekt (Vorhabenprofil,
  Bebauungsplan, Abweichungen) nicht belegt, bleibt offen. Die Wahl trifft die
  Entwurfsverfasserin oder der Entwurfsverfasser.
</p>
<div class="raster">
  {#snippet frage(titel: string, wert: string, setzen: (v: string) => void)}<label
      >{titel}<select
        value={wert}
        onchange={(e) => setzen(e.currentTarget.value)}
        ><option value="">Nicht angegeben</option><option value="true"
          >Ja</option
        ><option value="false">Nein</option></select
      ></label
    >{/snippet}
  {@render frage("Wohngebäude", wohngebaeude, (v) => (wohngebaeude = v))}
  {@render frage(
    "Großer Sonderbau",
    grosserSonderbau,
    (v) => (grosserSonderbau = v),
  )}
  {@render frage("Erschließung gesichert", erschliessung, (v) => (erschliessung = v))}
  {@render frage(
    "Gemeinde verlangt kein Verfahren",
    gemeinde,
    (v) => (gemeinde = v),
  )}
  <label
    >Stichtag (Antragseingang)<input type="date" bind:value={stichtag} /></label
  >
</div>
<button type="button" class="sekundaer" disabled={laedt} onclick={pruefen}
  >Verfahren prüfen</button
>
{#if fehler}<p class="fehler" role="alert">{fehler}</p>{/if}
{#if wahl}<section aria-label="Verfahrenswahl">
    <p>
      {wahl.fassung ?? "Kein Regelpaket"} · Stichtag {wahl.stichtag}{wahl.geprueft
        ? ""
        : " · nicht fachlich abgenommen"}
    </p>
    {#each wahl.hinweise as h}<p class="meta">{h}</p>{/each}
    {#each wahl.vorschlaege as v}<fieldset class="vorschlag {v.eignung}">
        <legend>{v.titel} · {v.rechtsgrundlage} · {EIGNUNG[v.eignung]}</legend>
        <ul>
          {#each v.bedingungen as b}<li>
              {b.erfuellt === true ? "✓" : b.erfuellt === false ? "✗" : "?"}
              {b.text} <span class="meta">({b.quelle})</span>
            </li>{/each}
        </ul>
        {#if v.eignung !== "ausgeschlossen"}<button
            type="button"
            class="link"
            onclick={() => onuebernehmen(v.art, v.pruefverfahren)}
            >Als Verfahrensart der Akte übernehmen</button
          >{/if}
      </fieldset>{/each}
    {#if wahl.fiktion}<p class="hinweisbox">
        Genehmigungsfiktion ({wahl.fiktion.rechtsgrundlage}): {wahl.fiktion
          .regulaer_monate} Monate im Baugenehmigungsverfahren, {wahl.fiktion
          .vereinfacht_wochen} Wochen im vereinfachten Verfahren. {wahl.fiktion
          .hinweis}
      </p>{/if}
  </section>{/if}
