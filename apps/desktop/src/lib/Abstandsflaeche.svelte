<script lang="ts">
  import Druckknopf from "./Druckknopf.svelte";
  import { plattformApi } from "./plattformApi";
  import { editorAnmelden } from "./editoren";
  import {
    api,
    fehlertext,
    GEBIETE,
    LAENDER,
    type AbstandsflaecheErgebnis,
    type Baugebiet,
    type Berechnung,
    type Bundesland,
    type RulePackInfo,
  } from "./api";

  let {
    startLand,
    projektId,
    darfSpeichern = false,
    regelpakete,
    onfehler,
  }: {
    startLand: Bundesland;
    projektId?: string;
    darfSpeichern?: boolean;
    regelpakete: RulePackInfo[];
    onfehler: (meldung: string) => void;
  } = $props();

  const gebiete = GEBIETE.map((g) => g.code);
  // svelte-ignore state_referenced_locally
  const land = $derived(startLand);
  let gebiet = $state<Baugebiet>("WA");
  let wandhoehe = $state(6.5);
  let ergebnis = $state<Berechnung<AbstandsflaecheErgebnis> | null>(null);
  let ergebnisStand = $state("");
  let laeuft = $state(false);
  let anfrage = 0;
  let speichert = $state(false),
    speicherHinweis = $state<string | null>(null),
    vorgangStand = "",
    vorgang = "";
  async function nachweisSpeichern() {
    if (!projektId || !ergebnis || !aktuell) return;
    speichert = true;
    try {
      if (vorgangStand !== stand) {
        vorgang = crypto.randomUUID();
        vorgangStand = stand;
      }
      const nachweis = await plattformApi.nachweisErstellen(
        projektId,
        vorgang,
        `Abstandsfläche – H ${wandhoehe} m`,
        {
          art: "abstand",
          gegenstand: "Wand",
          eingabe: { wandhoehe_m: wandhoehe, gebiet },
        },
      );
      speicherHinweis =
        "Nachweisstand mit Eingaben und Regelpaket gespeichert.";
      return nachweis;
    } catch (e) {
      onfehler(fehlertext(e));
    } finally {
      speichert = false;
    }
  }
  editorAnmelden("abstand", {
    hatAenderungen: () => false,
    istBeschaeftigt: () => laeuft || speichert,
    speichern: async () => true,
  });
  const stand = $derived(
    JSON.stringify([land, gebiet, wandhoehe, regelpakete]),
  );
  const aktuell = $derived(ergebnisStand === stand);
  const verfuegbar = $derived(new Set(regelpakete.map((r) => r.land)));

  async function berechnen(event: SubmitEvent) {
    event.preventDefault();
    const eingabestand = stand;
    const nummer = ++anfrage;
    laeuft = true;
    try {
      const neu = await api.abstandsflaeche(land, {
        wandhoehe_m: wandhoehe,
        gebiet,
      });
      if (nummer === anfrage && stand === eingabestand) {
        ergebnis = neu;
        ergebnisStand = eingabestand;
      }
    } catch (e) {
      if (nummer === anfrage && stand === eingabestand) onfehler(fehlertext(e));
    } finally {
      if (nummer === anfrage) laeuft = false;
    }
  }
</script>

<form onsubmit={berechnen}>
  <p class="meta">
    Landesbauordnung: {LAENDER.find((l) => l.code === land)?.name}. Auswahl
    unter Rechtsgrundlagen.
  </p>
  <label
    >Baugebiet <select bind:value={gebiet}>
      {#each gebiete as g}<option value={g}>{g}</option>{/each}
    </select></label
  >
  <label
    >H <input type="number" min="0" step="0.01" bind:value={wandhoehe} /> m</label
  >
  <button type="submit" disabled={!verfuegbar.has(land) || laeuft}
    >{laeuft ? "Wird berechnet …" : "Berechnen"}</button
  >
</form>
{#if !verfuegbar.has(land)}<p role="status">
    Für die ausgewählte Landesbauordnung ist noch kein Regelpaket installiert.
  </p>{/if}

{#if ergebnis}
  {#if !aktuell}<p role="status" class="hinweisbox">
      Ergebnis veraltet. Die Eingaben oder Grundlagen wurden geändert; bitte neu
      berechnen.
    </p>{/if}
  <p class="ergebnis">
    T = <strong>{ergebnis.ergebnis.tiefe_m.toFixed(2)} m</strong>
    ({ergebnis.ergebnis.faktor} · H, mind. {ergebnis.ergebnis.mindesttiefe_m} m)
  </p>
  <p class="meta">
    {ergebnis.ergebnis.rechtsgrundlage} · Regelpaket {ergebnis.regelpaket.land}
    {ergebnis.regelpaket.version}
  </p>
  {#if projektId}<button
      disabled={!darfSpeichern || !aktuell || speichert}
      onclick={nachweisSpeichern}>Nachweisstand speichern</button
    ><Druckknopf
      {projektId}
      disabled={!darfSpeichern || !aktuell || speichert}
      text="Speichern und drucken …"
      vorbereiten={async () => {
        const n = await nachweisSpeichern();
        return n ? { art: "nachweis", id: n.id } : null;
      }}
    />{/if}
{/if}
{#if speicherHinweis}<p role="status">{speicherHinweis}</p>{/if}
<p class="meta">
  Die Tiefe wird aus Wandhöhe und Baugebiet ermittelt. Weitere Wand-, Dach- und
  Sonderfälle benötigen einen gesonderten fachlichen Nachweis.
</p>
