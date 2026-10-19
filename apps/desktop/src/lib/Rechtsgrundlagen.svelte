<script lang="ts">
  import { projektNavigation } from "./projektNavigation.svelte";
  import { editorAnmelden } from "./editoren";
  import { untrack } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import {
    api,
    fehlertext,
    istKonflikt,
    GEBIETE,
    LAENDER,
    type Bundesland,
    type Planungsrecht,
    type Project,
  } from "./api";

  let {
    projekt,
    darf,
    onprojekt,
    bereich = "grundlagen",
  }: {
    projekt: Project;
    darf: boolean;
    onprojekt: (p: Project) => void;
    bereich?: "grundlagen" | "festsetzungen" | "stammdaten";
  } = $props();
  const navigation = projektNavigation();
  let land = $state<Bundesland>(untrack(() => projekt.bundesland));
  let gespeichertLand = $state<Bundesland>(untrack(() => projekt.bundesland));
  function grundlagen(p: Project): Planungsrecht {
    const wert = structuredClone($state.snapshot(p.planungsrecht));
    wert.bau_nvo ??= null;
    wert.umgebung ??= null;
    return wert;
  }
  let gespeichert = $state(untrack(() => grundlagen(projekt)));
  let version = $state(untrack(() => projekt.version));
  let eingabe = $state<Planungsrecht>(
    untrack(() => structuredClone($state.snapshot(gespeichert))),
  );
  let laeuft = $state(false);
  let fehler = $state<string | null>(null);
  let hinweis = $state<string | null>(null);
  const geaendert = $derived(
    JSON.stringify(eingabe) !== JSON.stringify(gespeichert) ||
      land !== gespeichertLand,
  );
  const mitPlan = $derived(
    !["kein", "ungeklaert"].includes(eingabe.bebauungsplan_art),
  );
  // Einfacher Plan mit ergänzendem § 34 BauGB: nicht festgesetzte Maße nach der Umgebung.
  const umgebungErfassen = $derived(
    (!mitPlan && eingabe.rechtsgrundlage === "par34") ||
      (eingabe.bebauungsplan_art === "einfach" &&
        eingabe.ergaenzende_grundlage === "par34"),
  );
  const aussenbereich = $derived(
    eingabe.rechtsgrundlage === "par35" ||
      (eingabe.bebauungsplan_art === "einfach" &&
        eingabe.ergaenzende_grundlage === "par35"),
  );

  // Änderungen in anderen Projektbereichen übernehmen, solange hier kein Entwurf offen ist.
  $effect(() => {
    const p = projekt;
    untrack(() => {
      if (!geaendert) {
        gespeichert = grundlagen(p);
        eingabe = structuredClone($state.snapshot(gespeichert));
        version = p.version;
        land = p.bundesland;
        gespeichertLand = land;
      }
    });
  });

  export async function verwerfenErlaubt(): Promise<boolean> {
    return (
      !geaendert ||
      (await ask(
        "Die Rechtsgrundlagen wurden noch nicht gespeichert. Änderungen verwerfen?",
        {
          title: "Rechtsgrundlagen",
          kind: "warning",
          okLabel: "Verwerfen",
          cancelLabel: "Abbrechen",
        },
      ))
    );
  }

  async function neuLaden() {
    if (!(await verwerfenErlaubt())) return;
    try {
      const p = await api.projektOeffnen(projekt.id);
      gespeichert = grundlagen(p);
      eingabe = structuredClone($state.snapshot(gespeichert));
      version = p.version;
      land = p.bundesland;
      gespeichertLand = land;
      fehler = null;
      hinweis = null;
      onprojekt(p);
    } catch (e) {
      fehler = fehlertext(e);
    }
  }

  async function speichern(e: SubmitEvent) {
    e.preventDefault();
    laeuft = true;
    fehler = null;
    hinweis = null;
    try {
      const p = await api.projektBereichSpeichern(
        {
          ...projekt,
          version,
          bundesland: gespeichertLand,
          planungsrecht: structuredClone($state.snapshot(gespeichert)),
        },
        bereich === "grundlagen" ? "grundlagen" : bereich,
        {
          id: projekt.id,
          expected_version: version,
          name: projekt.name,
          bundesland: land,
          grundstuecksflaeche_m2: projekt.grundstuecksflaeche_m2,
          planungsrecht: {
            ...structuredClone($state.snapshot(eingabe)),
            // Angezeigt wird bei einem Plan „§ 30 BauGB“; ältere Stände übernehmen das.
            ...(bereich === "grundlagen" && mitPlan
              ? { rechtsgrundlage: "par30" as const }
              : {}),
          },
        },
      );
      gespeichert = grundlagen(p);
      eingabe = structuredClone($state.snapshot(gespeichert));
      version = p.version;
      land = p.bundesland;
      gespeichertLand = land;
      onprojekt(p);
      hinweis = "Rechtsgrundlagen und Eckdaten gespeichert.";
    } catch (e) {
      fehler = istKonflikt(e)
        ? "Das Projekt wurde inzwischen geändert. Ihre Eingaben bleiben erhalten. Laden Sie den aktuellen Stand, bevor Sie die Angaben erneut bearbeiten."
        : fehlertext(e);
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
  export async function speichernBeimSchliessen(): Promise<boolean> {
    if (istBeschaeftigt()) return false;
    await speichern(new Event("submit") as SubmitEvent);
    return !hatAenderungen();
  }
  // Bereich ist für diese Editorinstanz fest; Projektwechsel erzeugt einen eigenen Kontext.
  // svelte-ignore state_referenced_locally
  editorAnmelden(
    bereich === "grundlagen"
      ? "rechtsgrundlagen"
      : bereich === "stammdaten"
        ? "uebersicht"
        : "festsetzungen",
    { hatAenderungen, istBeschaeftigt, speichern: speichernBeimSchliessen },
  );
</script>

<p class="meta">
  {bereich === "grundlagen"
    ? "Die gespeicherte Einordnung steuert die Projektbereiche und die Berechnungsgrundlagen."
    : bereich === "festsetzungen"
      ? "Festgesetzte Werte des Bebauungsplans. Die Berechnungen übernehmen diese Vorgaben."
      : "Standort und Beschreibung des Vorhabens."}
</p>
<form class="rechtsgrundlagen" onsubmit={speichern}>
  {#if !darf}<p class="lesemodus">Nur Leserecht · Die Angaben können hier nicht geändert werden.</p>{/if}
  <div class="knoepfe links formularaktionen">
    <button type="submit" disabled={!darf || !geaendert || laeuft}
      >{laeuft ? "Wird gespeichert …" : "Speichern"}</button
    >
    <button type="button" class="sekundaer" onclick={neuLaden} disabled={laeuft}
      >Aktuellen Stand laden</button
    >
    {#if geaendert}<span class="meta" role="status"
        >Ungespeicherte Änderungen</span
      >{/if}
  </div>
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  {#if hinweis}<p role="status">{hinweis}</p>{/if}
  {#if bereich === "grundlagen"}<fieldset disabled={!darf || laeuft}>
      <legend>Planungsrechtliche Grundlage</legend>
      <div class="raster">
        <label
          >Bebauungsplan
          <select
            aria-label="Bebauungsplan"
            bind:value={eingabe.bebauungsplan_art}
            onchange={(e) => {
              eingabe.bebauungsplan_art = e.currentTarget
                .value as Planungsrecht["bebauungsplan_art"];
              if (!["kein", "ungeklaert"].includes(eingabe.bebauungsplan_art))
                eingabe.rechtsgrundlage = "par30";
              else if (eingabe.rechtsgrundlage === "par30")
                eingabe.rechtsgrundlage = null;
              // Die Kopplung prüft der Core beim Speichern; hier nur die Auswahl leeren.
              if (eingabe.bebauungsplan_art !== "einfach")
                eingabe.ergaenzende_grundlage = null;
            }}
          >
            <option value="ungeklaert">Noch ungeklärt</option><option
              value="kein">Kein Bebauungsplan</option
            ><option value="qualifiziert">Qualifizierter Bebauungsplan</option
            ><option value="vorhabenbezogen"
              >Vorhabenbezogener Bebauungsplan</option
            ><option value="einfach">Einfacher Bebauungsplan</option>
          </select>
        </label>
        {#if mitPlan}<p>Rechtsgrundlage: § 30 BauGB</p>{:else}<label
            >Rechtsgrundlage<select
              aria-label="Rechtsgrundlage"
              bind:value={eingabe.rechtsgrundlage}
              ><option value={null}>Noch nicht festgelegt</option><option
                value="par34">§ 34 BauGB – Innenbereich</option
              ><option value="par35">§ 35 BauGB – Außenbereich</option></select
            ></label
          >{/if}
        <label
          >{mitPlan
            ? "BauNVO des Bebauungsplans"
            : "BauNVO als Berechnungsgrundlage"}<select
            aria-label="BauNVO"
            bind:value={eingabe.bau_nvo}
            ><option value={null}>Bitte festlegen</option
            >{#each ["1962", "1968", "1977", "1990", "2017", "2021"] as jahr}<option
                value={jahr}>{jahr}</option
              >{/each}</select
          ></label
        >
        <label
          >Anzuwendende Landesbauordnung<select bind:value={land}
            >{#each LAENDER as l}<option value={l.code}>{l.name}</option
              >{/each}</select
          ></label
        >
        {#if eingabe.bebauungsplan_art === "einfach" || eingabe.ergaenzende_grundlage}
          <label
            >Ergänzende Beurteilung (§ 30 Abs. 3 BauGB)
            <select bind:value={eingabe.ergaenzende_grundlage}>
              <option value={null}>Noch nicht festgelegt</option>
              <option value="par34">§ 34 BauGB – Innenbereich</option>
              <option value="par35">§ 35 BauGB – Außenbereich</option>
            </select>
          </label>
        {/if}
        {#if aussenbereich}
          <label
            >Vorhaben im Außenbereich
            <select bind:value={eingabe.aussenbereich_vorhaben}>
              <option value={null}>Noch nicht festgelegt</option>
              <option value="privilegiert"
                >Privilegiertes Vorhaben (§ 35 Abs. 1)</option
              >
              <option value="sonstiges">Sonstiges Vorhaben (§ 35 Abs. 2)</option
              >
            </select>
          </label>
        {/if}
      </div>
      {#if eingabe.rechtsgrundlage === "par30" && !mitPlan}
        <p class="hinweisbox">
          Für die Einordnung nach § 30 BauGB bitte den maßgeblichen
          Bebauungsplan klären und dessen Art angeben.
        </p>
      {/if}
      {#if umgebungErfassen}<p class="meta">
          Einfügen in die Eigenart der näheren Umgebung, Erschließung und
          weitere Voraussetzungen unter „Beurteilung“ dokumentieren.
        </p>
        {#if umgebungErfassen}
          <h3>Vergleichswerte der näheren Umgebung</h3>
          <p class="meta">
            Die Berechnungen stellen GRZ, GFZ und Vollgeschosse diesen Werten
            gegenüber{mitPlan ? ", soweit der Plan sie nicht festsetzt" : ""}.
            Die Orientierungswerte des § 17 BauNVO gelten hier nicht. Ob sich
            das Vorhaben einfügt, bleibt Ihre fachliche Beurteilung.
          </p>
          {#if eingabe.umgebung}
            <div class="raster">
              <label
                >GRZ der Umgebung<input
                  type="number"
                  min="0"
                  max="1"
                  step="any"
                  bind:value={eingabe.umgebung.grz}
                /></label
              >
              <label
                >GFZ der Umgebung<input
                  type="number"
                  min="0"
                  step="any"
                  bind:value={eingabe.umgebung.gfz}
                /></label
              >
              <label
                >Vollgeschosse der Umgebung<input
                  type="number"
                  min="0"
                  step="1"
                  bind:value={eingabe.umgebung.vollgeschosse}
                /></label
              >
            </div>
            <label
              >Referenzbebauung und Quelle<textarea
                bind:value={eingabe.umgebung.grundlage}
                maxlength="4000"
                required={eingabe.umgebung.grz != null ||
                  eingabe.umgebung.gfz != null ||
                  eingabe.umgebung.vollgeschosse != null}
                placeholder="z. B. Musterweg 3 bis 9, Lageplan und Ortsbesichtigung"
              ></textarea></label
            >
            <button
              type="button"
              class="sekundaer"
              onclick={() => (eingabe.umgebung = null)}
              >Vergleichswerte entfernen</button
            >
          {:else}
            <button
              type="button"
              class="sekundaer"
              onclick={() =>
                (eingabe.umgebung = {
                  grz: null,
                  gfz: null,
                  vollgeschosse: null,
                  grundlage: "",
                })}>Vergleichswerte erfassen</button
            >
          {/if}
        {/if}
      {/if}
    </fieldset>
  {/if}
  {#if mitPlan && bereich !== "stammdaten"}
    {#if bereich === 'festsetzungen'}<div class="planbezug"><strong>{projekt.planungsrecht.plan_bezeichnung || 'Bebauungsplan noch nicht bezeichnet'}</strong><p class="meta">{projekt.planungsrecht.plan_nummer || 'Fassung noch nicht angegeben'} · Rechtskraft: {projekt.planungsrecht.plan_rechtskraft || 'noch nicht angegeben'}</p><button type="button" class="link" onclick={()=>navigation?.springen({bereich:'rechtsgrundlagen',objekt:null,register:null,eintrag:null})}>Rechtsgrundlagen bearbeiten</button></div>{/if}
    <fieldset disabled={!darf || laeuft}>
      <legend>Angaben zum Bebauungsplan</legend>
      {#if bereich === 'grundlagen'}
      <div class="raster">
        <label
          >Planbezeichnung<input
            bind:value={eingabe.plan_bezeichnung}
            maxlength="500"
            placeholder="z. B. Wohngebiet Lindenweg"
          /></label
        >
        <label
          >Plannummer / Fassung<input
            bind:value={eingabe.plan_nummer}
            maxlength="100"
          /></label
        >
        <label
          >Rechtskraft seit<input
            type="date"
            bind:value={eingabe.plan_rechtskraft}
          /></label
        >
      </div>
      {/if}
      {#if bereich === "festsetzungen"}
        <h3>Festgesetzte Werte</h3>
        <div class="raster">
          <label
            >Baugebiet
            <select bind:value={eingabe.festgesetzte_werte.baugebiet}>
              <option value={null}>Nicht festgelegt</option>
              {#each GEBIETE as g}<option value={g.code}
                  >{g.code} – {g.name}</option
                >{/each}
            </select>
          </label>
          <label
            >GRZ – Hauptanlagen<input
              type="number"
              min="0"
              max="1"
              step="any"
              bind:value={eingabe.festgesetzte_werte.grz}
            /></label
          >
          <label
            >GRZ einschließlich Nebenanlagen<input
              type="number"
              min="0"
              max="1"
              step="any"
              bind:value={eingabe.festgesetzte_werte.grz_gesamt}
              placeholder="Leer: Regel nach § 19 Abs. 4"
            /></label
          >
          <label
            >GFZ<input
              type="number"
              min="0"
              step="any"
              bind:value={eingabe.festgesetzte_werte.gfz}
            /></label
          >
          <label
            >BMZ<input
              type="number"
              min="0"
              step="any"
              bind:value={eingabe.festgesetzte_werte.bmz}
            /></label
          >
          <label
            >Festgesetzte Grundfläche (GR, m²)<input
              type="number"
              min="0"
              step="any"
              bind:value={eingabe.festgesetzte_werte.grundflaeche_m2}
            /></label
          >
          <label
            >Festgesetzte Geschossfläche (GF, m²)<input
              type="number"
              min="0"
              step="any"
              bind:value={eingabe.festgesetzte_werte.geschossflaeche_m2}
            /></label
          >
          <label
            >Baufensterfläche (m², nur Information)<input
              type="number"
              min="0"
              step="any"
              bind:value={eingabe.festgesetzte_werte.ueberbaubare_flaeche_m2}
            /></label
          >
          <label
            >Festgesetzte Vollgeschosse (Z)<input
              type="number"
              min="0"
              step="1"
              bind:value={eingabe.festgesetzte_werte.vollgeschosse}
            /></label
          >
          <label
            >Anforderung an die Vollgeschosse
            <select bind:value={eingabe.festgesetzte_werte.geschossigkeit}
              ><option value="maximal">Höchstzahl</option><option
                value="zwingend">Zwingende Zahl</option
              ></select
            >
          </label>
        </div>
        <p class="meta">
          Die gespeicherten Werte werden unter „Maß der Nutzung“ übernommen.
          Leere Felder bleiben ungeprüft. Die Baufensterfläche allein prüft die
          Lage des Gebäudes nicht; dafür Baufenster und Vorhaben unter „Pläne“
          zuordnen.
        </p>
        <fieldset>
          <legend>Besondere GFZ-Festsetzung für Nichtvollgeschosse</legend>
          <p class="meta">
            Leer lassen: automatisch nach gewählter BauNVO. Hier nur eine
            ausdrücklich abweichende Festsetzung des Bebauungsplans hinterlegen.
          </p>
          <div class="raster">
            <label
              >Anrechnungsanteil (0–1, optional)<input
                type="number"
                min="0"
                max="1"
                step="any"
                bind:value={eingabe.festgesetzte_werte.nichtvollgeschoss_faktor}
                placeholder="Automatisch nach BauNVO"
              /></label
            >
            <label
              >Wortlaut / Fundstelle<textarea
                bind:value={
                  eingabe.festgesetzte_werte.nichtvollgeschoss_festsetzung
                }
                required={(eingabe.festgesetzte_werte
                  .nichtvollgeschoss_faktor ?? 0) > 0}
              ></textarea></label
            >
          </div>
        </fieldset>
        <label
          >Weitere Festsetzungen und Abweichungen<textarea
            bind:value={eingabe.festsetzungen}
            maxlength="10000"
            rows="3"
            placeholder="Baugebiet, GRZ / GFZ, Vollgeschosse, Bauweise, Baugrenzen, örtliche Vorschriften, Ausnahmen oder Befreiungen …"
          ></textarea></label
        >
      {/if}
    </fieldset>
  {/if}
  {#if bereich === "stammdaten"}<fieldset disabled={!darf || laeuft}>
      <legend>Eckdaten des Bauvorhabens</legend>
      <div class="raster">
        <label
          >Gemeinde / Stadt<input
            bind:value={eingabe.gemeinde}
            maxlength="500"
          /></label
        >
        <label
          >Standort / Grundstück<input
            bind:value={eingabe.standort}
            maxlength="1000"
            placeholder="Adresse, Gemarkung, Flurstück …"
          /></label
        >
        <label
          >Geplante Nutzung<input
            bind:value={eingabe.nutzung}
            maxlength="1000"
            placeholder="z. B. Wohngebäude mit zwei Wohneinheiten"
          /></label
        >
      </div>
      <label
        >Beschreibung des Vorhabens<textarea
          bind:value={eingabe.vorhaben}
          maxlength="10000"
          rows="3"
          placeholder="Neubau, Anbau, Umbau, Nutzungsänderung; wesentliche Eckdaten …"
        ></textarea></label
      >
      <label
        >Beurteilung / offene Fragen<textarea
          bind:value={eingabe.begruendung}
          maxlength="10000"
          rows="3"
          placeholder="Einordnung, Erschließung, Abstimmung mit der Behörde, offene Prüfpunkte …"
        ></textarea></label
      >
    </fieldset>{/if}
</form>
