<script lang="ts">
  import Druckknopf from "./Druckknopf.svelte";
  import { berechnungsplan, berechnungsansaetze } from "./grundstuecksdaten";
  import type { Gebaeude } from "./generated/plattform";
  import type { Datensatz } from "./plattformApi";
  import { plattformApi } from "./plattformApi";
  import { editorAnmelden } from "./editoren";
  import { onMount, untrack, tick } from "svelte";
  import {
    api,
    fehlertext,
    type Project,
    type BaugrundstuecksPlan,
    type BaugrundstueckGruppe,
    type BaugrundstuecksPruefung,
    type BaugrundstueckBerechnung,
  } from "./api";
  import type { BerechnungsBereich } from "./projektSitzung";
  import { regelwerk } from "./baunvo";
  import ABZUEGE from "../../../../regelwerke/grundstueck-abzuege.json";
  import GrundstueckFormular from "./GrundstueckFormular.svelte";
  import FlaechenFormular from "./FlaechenFormular.svelte";
  let {
    projekt,
    darf,
    onprojekt,
    bereich = "grundstueck",
    bezugsflaechenAktuell = false,
  }: {
    projekt: Project;
    darf: boolean;
    onprojekt: (p: Project) => void;
    bereich?: BerechnungsBereich;
    bezugsflaechenAktuell?: boolean;
  } = $props();
  function initial(): BaugrundstuecksPlan {
    const plan = structuredClone(
      berechnungsplan($state.snapshot(projekt.planungsrecht)),
    );
    const f = projekt.planungsrecht.bau_nvo;
    if (f) {
      for (const b of plan.berechnungen) {
        const a = b.eingabe.aufstellung;
        if (a && a.bau_nvo !== f) {
          a.bau_nvo = f;
          if (!a.nichtvollgeschoss_festsetzung.trim())
            a.nichtvollgeschoss_faktor = regelwerk(f).nichtvollgeschoss_faktor;
        }
      }
    }
    return plan;
  }
  let basisProjekt = $state(
    untrack(() => structuredClone($state.snapshot(projekt))),
  );
  let eingabe = $state(untrack(initial)),
    gespeichert = $state(untrack(initial)),
    basis = $state(untrack(() => projekt.version));
  let gruppen = $state<BaugrundstueckGruppe[]>([]),
    gruppenStand = $state(""),
    ergebnis = $state<BaugrundstuecksPruefung | null>(null),
    ergebnisStand = $state(""),
    auswahl = $state(""),
    laeuft = $state(false),
    fehler = $state<string | null>(null),
    hinweis = $state<string | null>(null);
  const mitPlan = $derived(
    !["kein", "ungeklaert"].includes(projekt.planungsrecht.bebauungsplan_art),
  );
  const gruppenFingerprint = $derived(
    JSON.stringify([eingabe.flurstuecke, projekt.version]),
  );
  const fingerprint = $derived(JSON.stringify([eingabe, projekt.version]));
  const gruppenAktuell = $derived(gruppenStand === gruppenFingerprint),
    aktuell = $derived(ergebnisStand === fingerprint);
  const quellenGeaendert = $derived(
    JSON.stringify(
      eingabe.flurstuecke.map(
        ({
          id,
          gemarkung,
          flur,
          nummer,
          grundbuchbezirk,
          grundbuchblatt,
          laufende_nummer,
          flaeche_m2,
        }) => ({
          id,
          gemarkung,
          flur,
          nummer,
          grundbuchbezirk,
          grundbuchblatt,
          laufende_nummer,
          flaeche_m2,
        }),
      ),
    ) !==
      JSON.stringify(
        berechnungsplan($state.snapshot(projekt.planungsrecht)).flurstuecke.map(
          ({
            id,
            gemarkung,
            flur,
            nummer,
            grundbuchbezirk,
            grundbuchblatt,
            laufende_nummer,
            flaeche_m2,
          }) => ({
            id,
            gemarkung,
            flur,
            nummer,
            grundbuchbezirk,
            grundbuchblatt,
            laufende_nummer,
            flaeche_m2,
          }),
        ),
      ),
  );
  const geaendert = $derived(
    JSON.stringify(eingabe) !== JSON.stringify(gespeichert),
  );
  const gruppe = $derived(
    gruppen.find((g) => g.schluessel === auswahl) ?? gruppen[0],
  );
  const berechnung = $derived(
    eingabe.berechnungen.find((b) => b.schluessel === gruppe?.schluessel),
  );
  const nachweis = $derived(
    aktuell
      ? ergebnis?.baugrundstuecke.find(
          (g) => g.grundstueck.schluessel === gruppe?.schluessel,
        )?.pruefung
      : null,
  );
  const z = (n: number | null | undefined) =>
    n == null ? "—" : n.toLocaleString("de-DE", { maximumFractionDigits: 3 });
  const nachweisTitel: Record<string, string> = {
    grz: "GRZ I / GRZ",
    grz_gesamt: "GRZ II",
    gfz: "GFZ",
    bmz: "BMZ",
    grundflaeche: "Grundfläche",
    geschossflaeche: "Geschossfläche",
    baufensterflaeche: "Baufensterfläche (Information)",
    baufenster_lage: "Lage im Baufenster (§ 23 BauNVO, m² außerhalb)",
    vollgeschosse: "Vollgeschosse",
  };
  // Orientierungs- und Umgebungswerte zeigen nur die Abweichung; der Core wertet sie nicht als Verstoß.
  const quellenTitel: Record<string, string> = {
    festsetzung: "Festsetzung",
    orientierungswert: "Orientierungswert § 17 BauNVO",
    bau_nvo19: "§ 19 Abs. 4 BauNVO",
    umgebung: "Nähere Umgebung (§ 34 BauGB)",
    keine: "Keine Vorgabe",
  };
  const relevanteNachweise = $derived(
    (nachweis?.nachweise ?? []).filter((n) =>
      bereich === "grz"
        ? ["grz", "grz_gesamt", "grundflaeche", "baufenster_lage"].includes(
            n.art,
          )
        : bereich === "gfz"
          ? ["gfz", "geschossflaeche"].includes(n.art)
          : bereich === "geschosse"
            ? n.art === "vollgeschosse"
            : bereich === "volumen"
              ? n.art === "bmz"
              : false,
    ),
  );
  $effect(() => {
    const p = projekt;
    untrack(() => {
      if (!geaendert) {
        eingabe = initial();
        basisProjekt = structuredClone($state.snapshot(p));
        gespeichert = structuredClone($state.snapshot(eingabe));
        basis = p.version;
      }
    });
  });
  function anlegen(g: BaugrundstueckGruppe): BaugrundstueckBerechnung {
    const f = projekt.planungsrecht.bau_nvo ?? "2017";
    return {
      schluessel: g.schluessel,
      eigene_festsetzungen: null,
      eingabe: {
        gebiet: projekt.planungsrecht.festgesetzte_werte.baugebiet ?? "WA",
        vollgeschosse: null,
        mass: {
          grundstuecksflaeche_m2: g.bauland_m2,
          grundflaeche_m2: 0,
          grundflaeche_nebenanlagen_m2: 0,
          geschossflaeche_m2: 0,
          baumasse_m3: null,
        },
        aufstellung: {
          bau_nvo: f,
          bauteile: [],
          geschosse: [],
          nichtvollgeschoss_faktor: regelwerk(f).nichtvollgeschoss_faktor,
          nichtvollgeschoss_festsetzung: "",
          grundstueck: {
            basisflaeche_m2: g.eigentumsflaeche_m2,
            nicht_massgebend_m2: g.abzuege_m2,
            abzug_begruendung: "Flurstücksabzüge",
            gemeinschaftsanteile: [],
          },
        },
      },
    };
  }
  async function ermitteln(initialisierung = false) {
    const stand = gruppenFingerprint;
    const eingabestand = fingerprint;
    fehler = null;
    try {
      const g = await api.baugrundstueckeErmitteln(
        projekt.id,
        structuredClone($state.snapshot(eingabe)),
      );
      if (stand !== gruppenFingerprint) return false;
      const nurVorgaben =
        initialisierung && !geaendert && eingabestand === fingerprint;
      gruppen = g;
      gruppenStand = stand;
      if (!g.some((x) => x.schluessel === auswahl))
        auswahl = g[0]?.schluessel ?? "";
      for (const gruppe of g)
        if (
          !eingabe.berechnungen.some((b) => b.schluessel === gruppe.schluessel)
        )
          eingabe.berechnungen.push(anlegen(gruppe));
      if (nurVorgaben) gespeichert = structuredClone($state.snapshot(eingabe));
      return true;
    } catch (e) {
      fehler = fehlertext(e);
      return false;
    }
  }
  onMount(async () => {
    if (eingabe.flurstuecke.length) await ermitteln(true);
  });
  async function neuErmitteln() {
    laeuft = true;
    try {
      await ermitteln();
    } finally {
      laeuft = false;
    }
  }
  function request() {
    const e = structuredClone($state.snapshot(eingabe));
    for (const b of e.berechnungen) {
      if (b.eingabe.aufstellung && projekt.planungsrecht.bau_nvo)
        b.eingabe.aufstellung.bau_nvo = projekt.planungsrecht.bau_nvo;
    }
    return e;
  }
  let nachweisHinweis = $state<string | null>(null),
    nachweisVorgang = "",
    nachweisStand = "";
  let gebaeude = $state<Datensatz<Gebaeude>[]>([]);
  let grundkontexte = $state<import("./api").GrundstueckKontext[]>([]);
  onMount(() => { api.grundstueckskontext(projekt.id).then((g) => { grundkontexte = g; }).catch((e) => { fehler = fehlertext(e); }); });
  $effect(() => {
    void bereich;
    untrack(() => {
      void plattformApi
        .liste<Gebaeude>(projekt.id, "gebaeude")
        .then((g) => (gebaeude = g))
        .catch((e) => (fehler = fehlertext(e)));
    });
  });
  async function nachweisSpeichern() {
    if (!ergebnis || !aktuell) return;
    laeuft = true;
    try {
      if (nachweisStand !== fingerprint) {
        nachweisVorgang = crypto.randomUUID();
        nachweisStand = fingerprint;
      }
      const fach =
        bereich === "geschosse"
          ? "geschossigkeit"
          : bereich === "volumen"
            ? "bmz"
            : bereich === "gfz"
              ? "gfz"
              : "grz";
      const nachweis = await plattformApi.nachweisErstellen(
        projekt.id,
        nachweisVorgang,
        `${fach.toLocaleUpperCase("de-DE")} – ${projekt.name}`,
        { art: "nutzung", fach, plan: request() },
      );
      nachweisHinweis =
        "Nachweisstand mit Eingaben und Regelpaketen gespeichert.";
      return nachweis;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function berechnen() {
    laeuft = true;
    fehler = null;
    try {
      if (!(await ermitteln())) return;
      const stand = fingerprint;
      const r = await api.projektBaugrundstueckePruefen(projekt.id, request());
      if (stand === fingerprint) {
        ergebnis = r;
        ergebnisStand = stand;
      }
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  async function speichern() {
    laeuft = true;
    fehler = null;
    try {
      if (!(await ermitteln())) return;
      const plan = request();
      await api.projektBaugrundstueckePruefen(projekt.id, plan);
      const p = await api.projektBereichSpeichern(
        structuredClone($state.snapshot(basisProjekt)),
        "grundstuecke",
        {
          id: projekt.id,
          expected_version: basis,
          name: projekt.name,
          bundesland: projekt.bundesland,
          grundstuecksflaeche_m2: projekt.grundstuecksflaeche_m2,
          planungsrecht: {
            ...structuredClone($state.snapshot(projekt.planungsrecht)),
            baugrundstuecksermittlung: berechnungsansaetze(plan),
          },
        },
      );
      eingabe = structuredClone(
        berechnungsplan($state.snapshot(p.planungsrecht)),
      );
      basisProjekt = structuredClone($state.snapshot(p));
      gespeichert = structuredClone($state.snapshot(eingabe));
      basis = p.version;
      onprojekt(p);
      await tick();
      await ermitteln();
      hinweis = "Baugrundstücke und Berechnungseingaben gespeichert.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  export function hatAenderungen() {
    return geaendert;
  }
  export function istBeschaeftigt() {
    return laeuft;
  }
  export async function speichernBeimSchliessen() {
    if (laeuft) return false;
    if (geaendert) await speichern();
    return !geaendert;
  }
  editorAnmelden("grundstueck", {
    hatAenderungen,
    istBeschaeftigt,
    speichern: speichernBeimSchliessen,
  });
</script>

{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}{#if hinweis}<p
    role="status"
  >
    {hinweis}
  </p>{/if}
{#if bereich !== "grundstueck" && !bezugsflaechenAktuell && eingabe.flurstuecke.length}<p
    class="hinweisbox"
  >
    Vor einem Nachweisstand die Baugrundstücksermittlung prüfen und speichern.
    Die Berechnung ist bis dahin eine Vorschau.
  </p>{/if}
{#if quellenGeaendert}<p class="hinweisbox" role="status">
    Die gespeicherten Flurstücksdaten wurden geändert. Der Berechnungsentwurf
    bleibt erhalten. Für eine neue Berechnung bitte die aktuellen Daten laden.
  </p>
  <button
    class="sekundaer"
    disabled={laeuft}
    onclick={async () => {
      eingabe = initial();
      gespeichert = initial();
      basisProjekt = structuredClone($state.snapshot(projekt));
      basis = projekt.version;
      ergebnis = null;
      await neuErmitteln();
    }}>Berechnungsentwurf verwerfen und aktuelle Daten laden</button
  >{/if}
{#if bereich === "grundstueck"}
  <p class="meta">
    Die gespeicherten Flurstücke sind die unveränderte Datengrundlage. Hier
    werden Zuordnung, Abzüge und Zuschläge für die Baugrundstücke ermittelt.
  </p>
  {#if !bezugsflaechenAktuell && eingabe.flurstuecke.length}<p
      class="hinweisbox"
    >
      Es liegen noch keine bestätigten aktuellen Bezugsflächen vor. Die
      Ermittlung prüfen und anschließend „Eingaben speichern“ wählen.
    </p>{/if}
  <h3>Datengrundlage: gespeicherte Flurstücke</h3>
  <div class="tabellen-scroll">
    <table class="ergebnistabelle">
      <thead
        ><tr
          ><th>Gemarkung / Flur</th><th>Flurstück</th><th>Flurstücksfläche</th
          ><th>Grundbuchzuordnung</th></tr
        ></thead
      ><tbody>
        {#each eingabe.flurstuecke as f (f.id)}<tr
            ><td>{f.gemarkung} / {f.flur || "—"}</td><td>{f.nummer}</td><td
              >{z(f.flaeche_m2)} m²</td
            ><td
              >{f.grundbuchbezirk && f.grundbuchblatt && f.laufende_nummer
                ? `${f.grundbuchbezirk} · Blatt ${f.grundbuchblatt} · lfd. Nr. ${f.laufende_nummer}`
                : "Optional – Flurstück wird einzeln angesetzt"}</td
            ></tr
          >{/each}
      </tbody>
    </table>
  </div>
  <p class="meta">
    Kataster- und Grundbuchangaben werden unter Grundstücksinformationen →
    Flurstücke und Grundbuch gepflegt. Ohne vollständige Grundbuchangaben wird jedes
    Flurstück einzeln berechnet. Für ein gemeinsames Baugrundstück können Sie
    vollständige Grundbuchangaben ergänzen oder den Flurstücken dieselbe
    besondere Zuordnungskennung mit Grundlage geben.
  </p>
  <fieldset disabled={!darf || laeuft}>
    <legend>1 · Flächenabzüge und Zuordnungen</legend>
    {#each eingabe.flurstuecke as f (f.id)}<details>
        <summary
          >{f.gemarkung || "Gemarkung"} · {f.nummer || "Flurstück"} · {f.abzuege
            .length} Abzüge</summary
        >
        <table class="ergebnistabelle">
          <thead
            ><tr><th>Abzugsart</th><th>m²</th><th>Grundlage</th><th></th></tr
            ></thead
          ><tbody
            >{#each f.abzuege as a, i}<tr
                ><td
                  ><select aria-label="Abzugsart" bind:value={a.art}
                    >{#each Object.entries(ABZUEGE).filter(([, r]) => mitPlan || !r.bebauungsplan_erforderlich) as [code, r]}<option
                        value={code}>{r.bezeichnung}</option
                      >{/each}</select
                  ></td
                ><td
                  ><input
                    aria-label="Abzugsfläche"
                    type="number"
                    min="0"
                    step="any"
                    bind:value={a.flaeche_m2}
                  /></td
                ><td
                  ><input
                    aria-label="Abzugsgrundlage"
                    bind:value={a.grundlage}
                  /></td
                ><td
                  ><button class="link" onclick={() => f.abzuege.splice(i, 1)}
                    >Entfernen</button
                  ></td
                ></tr
              >{/each}</tbody
          >
        </table>
        <button
          class="sekundaer"
          onclick={() =>
            f.abzuege.push({
              art: "sonstiger_abzug",
              flaeche_m2: 0,
              grundlage: "",
            })}>Abzug hinzufügen</button
        >
        <details>
          <summary>Abweichende rechtliche Grundstückszuordnung</summary
          >{#if f.abweichende_zuordnung}<div class="raster">
              <label
                >Zuordnungskennung<input
                  bind:value={f.abweichende_zuordnung.kennung}
                /></label
              ><label
                >Rechtliche Grundlage<input
                  bind:value={f.abweichende_zuordnung.grundlage}
                /></label
              >
            </div>
            <button
              class="link"
              onclick={() => (f.abweichende_zuordnung = null)}
              >Besondere Zuordnung entfernen</button
            >{:else}<button
              class="sekundaer"
              onclick={() =>
                (f.abweichende_zuordnung = { kennung: "", grundlage: "" })}
              >Besondere Zuordnung erfassen</button
            >{/if}
        </details>
      </details>{/each}
  </fieldset>
  <h3>2 · Baugrundstücksermittlung</h3>
  <button
    class="sekundaer"
    disabled={laeuft || !eingabe.flurstuecke.length}
    onclick={neuErmitteln}>Baugrundstücke ermitteln</button
  >
  {#if gruppen.length}<div class="tabellen-scroll">
      <table class="ergebnistabelle">
        <thead
          ><tr
            ><th>Baugrundstück</th><th>Flurstücke</th><th>Eigentumsfläche</th
            ><th>Abzüge</th><th>Bauland</th></tr
          ></thead
        ><tbody
          >{#each gruppen as g}<tr
              ><td>{g.bezeichnung}</td><td>{g.flurstueck_ids.length}</td><td
                >{z(g.eigentumsflaeche_m2)} m²</td
              ><td>− {z(g.abzuege_m2)} m²</td><td
                ><strong>{z(g.bauland_m2)} m²</strong></td
              ></tr
            >{/each}</tbody
        >
      </table>
    </div>{/if}
{/if}
{#if gruppen.length && !gruppenAktuell}<p class="hinweisbox">
    Flurstücke oder Grundlagen geändert. Die angezeigte Ermittlung ist veraltet;
    bitte neu ermitteln.
  </p>{/if}
{#if !eingabe.flurstuecke.length}<p class="meta">
    Zuerst unter Grundstücksinformationen → Flurstücke und Grundbuch die
    Flurstücke erfassen und speichern.
  </p>{/if}
{#if gruppe && berechnung?.eingabe.aufstellung}
  <label
    >Baugrundstück<select aria-label="Baugrundstück" bind:value={auswahl}
      >{#each gruppen as g}<option value={g.schluessel}>{g.bezeichnung}</option
        >{/each}</select
    ></label
  >
  <fieldset disabled={!darf || laeuft || !gruppenAktuell}>
    {#if bereich === "grundstueck"}<GrundstueckFormular
        aufstellung={berechnung.eingabe.aufstellung}
        basisFallback={gruppe.eigentumsflaeche_m2}
        ergebnis={nachweis?.aufstellung?.grundstueck ?? null}
        {mitPlan}
        flurstuecksBasis={{
          basis: gruppe.eigentumsflaeche_m2,
          abzug: gruppe.abzuege_m2,
        }}
      />
    {:else if bereich === "volumen"}<label
        >Baumasse / Bauvolumen (m³)<input
          type="number"
          min="0"
          step="any"
          bind:value={berechnung.eingabe.mass.baumasse_m3}
        /></label
      >
    {:else}<FlaechenFormular
        {gebaeude}
        projektId={projekt.id}
        grundstueckId={grundkontexte.find((g) => g.schluessel === gruppe.schluessel)?.id}
        bereich={bereich === "grz" ? "grz" : "gfz"}
        aufstellung={berechnung.eingabe.aufstellung}
        festsetzungen={projekt.planungsrecht.festgesetzte_werte}
        ergebnis={nachweis?.aufstellung ?? null}
      />{/if}
  </fieldset>
{/if}
{#if eingabe.berechnungen.some((b) => !gruppen.some((g) => g.schluessel === b.schluessel)) && gruppenAktuell}<p
    class="hinweisbox"
  >
    Die Grundstückszuordnung hat sich geändert. Verwaiste Berechnungen müssen
    vor dem Speichern geprüft werden.
  </p>
  <button
    class="sekundaer"
    disabled={!darf || laeuft}
    onclick={() => {
      eingabe.berechnungen = eingabe.berechnungen.filter((b) =>
        gruppen.some((g) => g.schluessel === b.schluessel),
      );
    }}>Nicht zugeordnete Berechnungen entfernen</button
  >{/if}
{#each nachweis?.hinweise ?? [] as h}<p class="hinweisbox" role="status">
    {h}
  </p>{/each}
{#if relevanteNachweise.length}<table class="ergebnistabelle">
    <thead
      ><tr
        ><th>Nachweis</th><th>Ist</th><th>Vorgabe</th><th>Grundlage</th><th
          >Abweichung</th
        ></tr
      ></thead
    ><tbody
      >{#each relevanteNachweise as n}<tr
          ><td
            >{nachweisTitel[n.art]}{#if n.gegenstand}<span class="meta">
                · {n.gegenstand}</span
              >{/if}</td
          ><td>{z(n.ist)}</td><td>{z(n.soll)}</td><td
            >{quellenTitel[n.quelle]}</td
          ><td class:verstoss={n.eingehalten === false}
            >{n.art === "baufenster_lage" && n.ist == null
              ? "Nicht geprüft"
              : z(n.abweichung ?? n.ueberschreitung)}</td
          ></tr
        >{/each}</tbody
    >
  </table>{/if}
{#if ergebnis && !aktuell}<p class="meta">
    Eingaben geändert. Ergebnisse bitte neu berechnen.
  </p>{/if}
<div class="knoepfe links">
  <button disabled={laeuft || !eingabe.flurstuecke.length} onclick={berechnen}
    >{bereich === "grundstueck"
      ? "Bezugsflächen berechnen"
      : "Berechnen"}</button
  ><button
    class="sekundaer"
    disabled={!darf || laeuft || (!geaendert && bezugsflaechenAktuell)}
    onclick={speichern}>Eingaben speichern</button
  >{#if geaendert}<span class="meta">Ungespeicherte Eingaben</span>{/if}
</div>

{#if ergebnis}<button
    disabled={!darf || !aktuell || laeuft || !bezugsflaechenAktuell}
    onclick={nachweisSpeichern}>Nachweisstand speichern</button
  ><Druckknopf
    projektId={projekt.id}
    disabled={!darf || !aktuell || laeuft || !bezugsflaechenAktuell}
    text="Speichern und drucken …"
    vorbereiten={async () => {
      const n = await nachweisSpeichern();
      return n ? { art: "nachweis", id: n.id } : null;
    }}
  />{/if}
{#if nachweisHinweis}<p role="status">{nachweisHinweis}</p>{/if}

<style>
  table {
    margin: 1rem 0;
  }
  details {
    padding: 0.6rem 0;
  }
  fieldset {
    margin: 1rem 0;
  }
  th {
    text-align: left;
  }
</style>
