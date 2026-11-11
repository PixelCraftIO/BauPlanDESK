<script module lang="ts">
  /** Beim Anlegen gewählte Unterlage, die mit Kategorie abgelegt wird. */
  export type Startplan = { kategorie: "flurkarte" | "lageplan"; pfad: string };
  /** Beim Anlegen erfasste Eckdaten: Baugrundstück, Vorhaben und Verfahren. */
  export type Startangaben = {
    anschrift: Anschrift;
    gemeinde: string;
    vorhaben: string;
    nutzung: string;
    wohngebaeude: boolean | null;
    /** Angestrebtes Verfahren; `null` = später festlegen. */
    verfahren: Verfahrensart | null;
  };
</script>

<script lang="ts">
  import { untrack } from "svelte";
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import {
    LAENDER,
    api,
    fehlertext,
    landName,
    kundenname,
    leereKundendaten,
    type Anschrift,
    type Kunde,
    type Kundendaten,
    type Projektwerte as Werte,
  } from "./api";
  import type { Verfahrensart } from "./generated/plattform";
  import AnschriftFelder from "./AnschriftFelder.svelte";
  import KundeFelder from "./KundeFelder.svelte";
  import Abweichungen from "./Abweichungen.svelte";
  import { abgleichen, pfadSetzen, type Abweichung } from "./abgleich";

  let {
    start,
    kunden = null,
    kundenLadefehler = null,
    absenden,
    knopf,
    fassungVerfolgen = false,
    plaene = false,
    onabbrechen,
  }: {
    start: Werte;
    /** Kunden zur Auswahl des Bauherrn (nur beim Anlegen). */
    kunden?: Kunde[] | null;
    kundenLadefehler?: string | null;
    absenden: (
      w: Werte,
      bauherr: string | null,
      plaene: Startplan[],
      start?: Startangaben,
    ) => Promise<void>;
    knopf: string;
    /** Gespeichertes Projekt: neue Fassungen von `start` als Grundlage abgleichen. */
    fassungVerfolgen?: boolean;
    /** Beim Anlegen: Flurkarte / Lageplan zur Ablage mit Kategorie auswählen; die Akte öffnet danach im Aufmaß. */
    plaene?: boolean;
    onabbrechen: () => void;
  } = $props();

  // svelte-ignore state_referenced_locally
  let werte = $state<Werte>({ ...start });
  /** Fassung, auf der die Eingaben beruhen. */
  // svelte-ignore state_referenced_locally
  let basis = $state<Werte>({ ...start });
  let bauherr = $state<string | null>(null);
  /** Bauherr, der mit dem Projekt neu angelegt wird. */
  let neuerBauherr = $state<Kundendaten | null>(null);
  let fehler = $state<string | null>(null);
  const leererStart = (): Startangaben => ({
    anschrift: {
      strasse: "",
      hausnummer: "",
      postleitzahl: "",
      ort: "",
    },
    gemeinde: "",
    vorhaben: "",
    nutzung: "",
    wohngebaeude: null,
    verfahren: null,
  });
  let startangaben = $state<Startangaben>(leererStart());
  /** Verfahren, die ein neues Vorhaben einleiten können. */
  const STARTVERFAHREN: [Verfahrensart, string][] = [
    ["bauantrag", "Bauantrag (Baugenehmigung)"],
    ["freistellung", "Genehmigungsfreistellung"],
    ["vorbescheid", "Vorbescheid (Bauvoranfrage)"],
    ["nutzungsaenderung", "Nutzungsänderung"],
    ["beseitigung", "Beseitigung (Abbruch)"],
  ];
  let startplaene = $state<Startplan[]>([]);
  let laeuft = $state(false);
  async function planWaehlen(kategorie: Startplan["kategorie"]) {
    const auswahl = await open({
      multiple: true,
      title:
        kategorie === "flurkarte"
          ? "Flurkarte / Liegenschaftskarte auswählen"
          : "Lageplan auswählen",
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    const pfade = Array.isArray(auswahl) ? auswahl : auswahl ? [auswahl] : [];
    for (const pfad of pfade)
      if (!startplaene.some((p) => p.pfad === pfad))
        startplaene.push({ kategorie, pfad });
  }
  const dateiname = (pfad: string) => pfad.split(/[\\/]/).pop() ?? pfad;
  let abweichungen = $state<Abweichung[]>([]);
  let neueFassung = $state(false);
  const FELDNAMEN: Record<string, string> = {
    name: "Projektname",
    bundesland: "Bundesland",
    grundstuecksflaeche_m2: "Grundstücksfläche",
  };

  // Kommt eine neue Fassung (z. B. nach einem Versionskonflikt geladen), wird
  // sie Grundlage. Eigene Eingaben bleiben stehen; nur fremd geänderte Felder
  // werden übernommen, auf beiden Seiten geänderte Felder werden angezeigt.
  $effect(() => {
    if (!fassungVerfolgen) return;
    const neu: Werte = { ...start };
    untrack(() => {
      const alt = $state.snapshot(basis) as Werte;
      if (JSON.stringify(neu) === JSON.stringify(alt)) return;
      const ergebnis = abgleichen(alt, $state.snapshot(werte) as Werte, neu);
      basis = neu;
      werte = ergebnis.werte;
      abweichungen = ergebnis.abweichungen;
      neueFassung = true;
    });
  });

  function aktuellUebernehmen(pfad: string) {
    const a = abweichungen.find((x) => x.pfad === pfad);
    if (!a) return;
    pfadSetzen(werte, pfad, a.aktuell);
    abweichungen = abweichungen.filter((x) => x.pfad !== pfad);
  }

  export function hatAenderungen() {
    return (
      JSON.stringify($state.snapshot(werte)) !==
        JSON.stringify($state.snapshot(basis)) ||
      bauherr !== null ||
      neuerBauherr !== null ||
      startplaene.length > 0 ||
      JSON.stringify($state.snapshot(startangaben)) !==
        JSON.stringify(leererStart())
    );
  }
  export function istBeschaeftigt() {
    return laeuft;
  }
  /** Rückfrage vor dem Verwerfen ungespeicherter Eingaben. */
  export async function schliessenErlaubt() {
    return (
      !laeuft &&
      (!hatAenderungen() ||
        (await ask("Die Eingaben wurden noch nicht gespeichert. Verwerfen?", {
          title: knopf === "Anlegen" ? "Neues Projekt" : "Projektdaten",
          kind: "warning",
          okLabel: "Verwerfen",
          cancelLabel: "Weiter bearbeiten",
        })))
    );
  }

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (laeuft) return;
    laeuft = true;
    fehler = null;
    try {
      // Ein neuer Bauherr wird zuerst gespeichert: Scheitert das, entsteht
      // kein Projekt ohne ihn, und die Eingaben bleiben stehen.
      if (neuerBauherr) {
        try {
          const k = await api.kundeAnlegen(
            $state.snapshot(neuerBauherr) as Kundendaten,
          );
          bauherr = k.id;
          neuerBauherr = null;
          kunden = [...(kunden ?? []), k];
        } catch (e) {
          fehler = `Der Bauherr konnte nicht angelegt werden: ${fehlertext(e)}`;
          return;
        }
      }
      await absenden(
        {
          ...werte,
          name: werte.name.trim(),
          grundstuecksflaeche_m2: werte.grundstuecksflaeche_m2 || null,
        },
        bauherr,
        $state.snapshot(startplaene) as Startplan[],
        plaene ? ($state.snapshot(startangaben) as Startangaben) : undefined,
      );
    } finally {
      laeuft = false;
    }
  }
</script>

<form class="spalte" onsubmit={submit}>
  <label>
    Projektname
    <!-- svelte-ignore a11y_autofocus -->
    <input bind:value={werte.name} required autofocus />
  </label>
  <label>
    Bundesland
    <select bind:value={werte.bundesland}>
      {#each LAENDER as l}<option value={l.code}>{l.name}</option>{/each}
    </select>
  </label>
  {#if plaene}
    <fieldset>
      <legend>Baugrundstück</legend>
      <AnschriftFelder bind:anschrift={startangaben.anschrift} gesperrt={laeuft} />
      <label>
        Gemeinde / Stadt
        <input bind:value={startangaben.gemeinde} maxlength="200" />
      </label>
      <p class="meta">
        Flurstücke mit Flächen und Grundbuchangaben folgen in der Projektakte
        unter Grundlagen → Flurstücke; die Fläche des Baugrundstücks ergibt
        sich daraus.
      </p>
    </fieldset>
    <fieldset>
      <legend>Vorhaben und Verfahren</legend>
      <label>
        Vorhaben
        <input
          bind:value={startangaben.vorhaben}
          maxlength="500"
          placeholder="z. B. Neubau eines Einfamilienhauses mit Garage"
        />
      </label>
      <div class="zeile">
        <label>
          Geplante Nutzung
          <input
            bind:value={startangaben.nutzung}
            maxlength="200"
            placeholder="z. B. Wohnen"
          />
        </label>
        <label>
          Wohngebäude
          <select bind:value={startangaben.wohngebaeude}>
            <option value={null}>Noch offen</option>
            <option value={true}>Ja</option>
            <option value={false}>Nein</option>
          </select>
        </label>
      </div>
      <label>
        Angestrebtes Verfahren
        <select bind:value={startangaben.verfahren}>
          <option value={null}>– später festlegen –</option>
          {#each STARTVERFAHREN as [art, name]}<option value={art}>{name}</option
            >{/each}
        </select>
      </label>
      <p class="meta">
        Mit dem Verfahren legt die Anwendung die Verfahrensakte an und leitet
        daraus ab, welche Unterlagen und Nachweise das Vorhaben braucht. Der
        Projektbaum zeigt sie dann als offen. Alles lässt sich später ändern.
      </p>
    </fieldset>
  {:else}
    <p class="meta">
      Flurstücke mit ihren Flächen und optionalen Grundbuchangaben werden in der
      Projektakte unter Grundlagen → Flurstücke erfasst. Die Baugrundstücksfläche
      wird separat berechnet.
    </p>
  {/if}
  {#if neueFassung}
    <p class="hinweisbox" role="status">
      Das Projekt wurde inzwischen an einem anderen Arbeitsplatz geändert. Die
      aktuelle Fassung ({landName(basis.bundesland)}, „{basis.name}“) ist jetzt
      Grundlage; Ihre Eingaben bleiben im Formular. Bitte prüfen und erneut
      speichern.
    </p>
    <Abweichungen
      {abweichungen}
      titel={(pfad) => FELDNAMEN[pfad] ?? pfad}
      gesperrt={laeuft}
      onuebernehmen={aktuellUebernehmen}
    />
  {/if}
  {#if kundenLadefehler}
    <p class="fehlertext" role="alert">Bauherren konnten nicht geladen werden: {kundenLadefehler} Sie können das Projekt anlegen und den Bauherrn später zuordnen.</p>
  {/if}
  {#if kunden}
    {#if neuerBauherr}
      <fieldset>
        <legend>Neuer Bauherr</legend>
        <KundeFelder bind:daten={neuerBauherr} gesperrt={laeuft} />
        <button
          type="button"
          class="link"
          disabled={laeuft}
          onclick={() => (neuerBauherr = null)}
          >Doch einen vorhandenen Bauherrn wählen</button
        >
      </fieldset>
    {:else}
      <label>
        Bauherr
        <select bind:value={bauherr}>
          <option value={null}>– später zuordnen –</option>
          {#each kunden as k (k.id)}<option value={k.id}
              >{kundenname(k)}{k.kundennummer
                ? ` (${k.kundennummer})`
                : ""}</option
            >{/each}
        </select>
      </label>
      <button
        type="button"
        class="link"
        disabled={laeuft}
        onclick={() => {
          bauherr = null;
          neuerBauherr = leereKundendaten();
        }}>Neuen Bauherrn anlegen …</button
      >
    {/if}
  {/if}
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  {#if plaene}
    <fieldset>
      <legend>Pläne zum Start (optional)</legend>
      <p class="meta">
        Flurkarte / Liegenschaftskarte und Lageplan werden nach dem Anlegen mit
        Kategorie in den Projektdateien abgelegt; die Akte öffnet sich im
        Aufmaß, in dem Flurstücksgrenzen, Abzugsflächen und Gebäude
        gezeichnet werden. Flurstücksnummern und Katasterflächen bleiben
        Eingaben unter Grundlagen → Flurstücke.
      </p>
      <div class="knoepfe">
        <button
          type="button"
          class="sekundaer"
          disabled={laeuft}
          onclick={() => void planWaehlen("flurkarte")}>Flurkarte wählen …</button
        >
        <button
          type="button"
          class="sekundaer"
          disabled={laeuft}
          onclick={() => void planWaehlen("lageplan")}>Lageplan wählen …</button
        >
      </div>
      {#if startplaene.length}<ul>
          {#each startplaene as s, i (s.pfad)}<li>
              {s.kategorie === "flurkarte" ? "Flurkarte" : "Lageplan"}: {dateiname(
                s.pfad,
              )}
              <button
                type="button"
                class="link"
                disabled={laeuft}
                onclick={() => startplaene.splice(i, 1)}>Entfernen</button
              >
            </li>{/each}
        </ul>{/if}
    </fieldset>
  {/if}
  <div class="knoepfe">
    <button
      type="button"
      class="sekundaer"
      onclick={async () => {
        if (await schliessenErlaubt()) onabbrechen();
      }}>Abbrechen</button
    >
    <button type="submit" disabled={laeuft}>{knopf}</button>
  </div>
</form>
