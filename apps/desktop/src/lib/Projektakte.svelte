<script lang="ts">
  import { tick, untrack } from "svelte";
  import {
    api,
    fehlertext,
    kundenname,
    kategorieName,
    landName,
    rechtsgrundlageName,
    bebauungsplanName,
    type AppInfo,
    type Benutzer,
    type Kunde,
    type Project,
  } from "./api";
  import {
    berechnungsAnsichten,
    berechnungsTitel,
    type BerechnungsBereich,
    type Ansicht,
  } from "./projektSitzung";
  import Abstandsflaeche from "./Abstandsflaeche.svelte";
  import Bauantrag from "./Bauantrag.svelte";
  import ProjektFormulare from "./ProjektFormulare.svelte";
  import Einreichungen from "./Einreichungen.svelte";
  import Dokumente from "./Dokumente.svelte";
  import MassDerNutzung from "./MassDerNutzung.svelte";
  import Plaene from "./Plaene.svelte";
  import Protokoll from "./Protokoll.svelte";
  import Rechtsgrundlagen from "./Rechtsgrundlagen.svelte";
  let {
    projekt,
    ansicht = $bindable("uebersicht"),
    planDokument = $bindable(null),
    aktiv,
    info,
    ich,
    darf,
    importAusloeser,
    exportAusloeser,
    onprojekt,
    ondialog,
  }: {
    projekt: Project;
    ansicht?: Ansicht;
    planDokument?: string | null;
    aktiv: boolean;
    info: AppInfo | null;
    ich: Benutzer | null;
    darf: (min: "bearbeitung" | "projektleitung") => boolean;
    importAusloeser: number;
    exportAusloeser: number;
    onprojekt: (p: Project) => void;
    ondialog: (art: "projektdaten" | "bauherren" | "buero") => void;
  } = $props();
  let planEditor = $state<Plaene>();
  let nutzungsEditor = $state<MassDerNutzung>();
  let formularEditor = $state<ProjektFormulare>();
  let festsetzungsEditor = $state<Rechtsgrundlagen>();
  let stammdatenEditor = $state<Rechtsgrundlagen>();
  let formulareBesucht = $state(false);
  $effect(() => {
    if (ansicht === "formulare") formulareBesucht = true;
  });
  let rechtsEditor = $state<Rechtsgrundlagen>();
  let antragEditor = $state<Bauantrag>();
  let einreichungsEditor = $state<Einreichungen>();
  let einreichungenBesucht = $state(false);
  $effect(() => {
    if (ansicht === "einreichungen") einreichungenBesucht = true;
  });
  let dokumentEditor = $state<Dokumente>();
  let bauherren = $state<Kunde[]>([]);
  let fehler = $state<string | null>(null);
  let hinweis = $state<string | null>(null);
  const melde = (e: unknown) => (fehler = fehlertext(e));
  $effect(() => {
    const id = projekt.id;
    void projekt.version;
    untrack(() =>
      api
        .bauherren(id)
        .then((b) => (bauherren = b))
        .catch(melde),
    );
  });
  export async function dokumenteAktualisieren() {
    await dokumentEditor?.aktualisieren();
  }
  export function hatAenderungen() {
    return !!(
      planEditor?.hatAenderungen() ||
      nutzungsEditor?.hatAenderungen() ||
      rechtsEditor?.hatAenderungen() ||
      festsetzungsEditor?.hatAenderungen() ||
      stammdatenEditor?.hatAenderungen() ||
      einreichungsEditor?.hatAenderungen() ||
      antragEditor?.hatAenderungen()
    );
  }
  export function istBeschaeftigt() {
    return !!(
      planEditor?.istBeschaeftigt() ||
      nutzungsEditor?.istBeschaeftigt() ||
      rechtsEditor?.istBeschaeftigt() ||
      festsetzungsEditor?.istBeschaeftigt() ||
      stammdatenEditor?.istBeschaeftigt() ||
      formularEditor?.istBeschaeftigt() ||
      antragEditor?.istBeschaeftigt() ||
      einreichungsEditor?.istBeschaeftigt() ||
      dokumentEditor?.istBeschaeftigt()
    );
  }
  export async function speichern(): Promise<boolean> {
    if (istBeschaeftigt()) return false;
    if (einreichungsEditor?.hatAenderungen()) {
      ansicht = "einreichungen";
      fehler =
        "Die Einreichungsauswahl zuerst als Paket festhalten oder beim Schließen ausdrücklich verwerfen. Einreichungsdaten werden im eigenen Bereich bestätigt.";
      return false;
    }
    const editoren: {
      editor:
        | typeof rechtsEditor
        | typeof nutzungsEditor
        | typeof planEditor
        | typeof antragEditor;
      bereich: Ansicht;
    }[] = [
      { editor: rechtsEditor, bereich: "rechtsgrundlagen" },
      { editor: festsetzungsEditor, bereich: "festsetzungen" },
      { editor: stammdatenEditor, bereich: "uebersicht" },
      { editor: nutzungsEditor, bereich: "grundstueck" },
      { editor: planEditor, bereich: "plaene" },
      { editor: antragEditor, bereich: "bauantrag" },
    ];
    for (const { editor, bereich } of editoren) {
      if (editor?.hatAenderungen()) {
        if (!(await editor.speichernBeimSchliessen())) {
          ansicht = bereich;
          return false;
        }
        await tick();
      }
    }
    return !hatAenderungen();
  }
  async function grundstuecksflaecheUebernehmen(m2: number) {
    const p = projekt;
    try {
      onprojekt(
        await api.projektAktualisieren({
          id: p.id,
          expected_version: p.version,
          name: p.name,
          bundesland: p.bundesland,
          grundstuecksflaeche_m2: m2,
        }),
      );
      hinweis = `Grundstücksfläche ${m2.toLocaleString("de-DE")} m² übernommen.`;
    } catch (e) {
      melde(e);
    }
  }
</script>

{#if fehler}<div class="fehler" role="alert">
    {fehler}<button class="sekundaer" onclick={() => (fehler = null)}
      >Schließen</button
    >
  </div>{/if}
{#if hinweis}<div class="banner" role="status">
    {hinweis}<button class="sekundaer" onclick={() => (hinweis = null)}
      >OK</button
    >
  </div>{/if}
<header class="projektkopf">
  <div>
    <h1>{projekt.name}</h1>
    <span class="meta">
      {landName(projekt.bundesland)}
      {projekt.grundstuecksflaeche_m2
        ? ` · Grundstück ${projekt.grundstuecksflaeche_m2.toLocaleString("de-DE")} m²`
        : ""}
    </span>
  </div>
</header>

<main>
  <section hidden={ansicht !== "uebersicht"}>
    <h2>Stammdaten</h2>
    <dl class="daten">
      <dt>Name</dt>
      <dd>{projekt.name}</dd>
      <dt>Bundesland</dt>
      <dd>{landName(projekt.bundesland)}</dd>
      <dt>Grundstücksfläche</dt>
      <dd>
        {projekt.grundstuecksflaeche_m2
          ? `${projekt.grundstuecksflaeche_m2.toLocaleString("de-DE")} m²`
          : "–"}
      </dd>
      <dt>{bauherren.length > 1 ? "Bauherren" : "Bauherr"}</dt>
      <dd>
        {#each bauherren as k (k.id)}
          <div>
            {kundenname(k)}
            <span class="meta"
              >({kategorieName(k.kategorie)}{k.telefon
                ? `, ${k.telefon}`
                : ""}{k.email ? `, ${k.email}` : ""})</span
            >
          </div>
        {:else}–{/each}
      </dd>
      <dt>Rechtsgrundlage</dt>
      <dd>{rechtsgrundlageName(projekt.planungsrecht.rechtsgrundlage)}</dd>
      <dt>Bebauungsplan</dt>
      <dd>
        {bebauungsplanName(projekt.planungsrecht.bebauungsplan_art)}{![
          "kein",
          "ungeklaert",
        ].includes(projekt.planungsrecht.bebauungsplan_art) &&
        projekt.planungsrecht.plan_bezeichnung
          ? ` · ${projekt.planungsrecht.plan_bezeichnung}`
          : ""}
      </dd>
      <dt>Angelegt</dt>
      <dd>{new Date(projekt.created_at).toLocaleString("de-DE")}</dd>
      <dt>Zuletzt geändert</dt>
      <dd>
        {new Date(projekt.updated_at).toLocaleString("de-DE")} (Version {projekt.version})
      </dd>
    </dl>
    <div class="knoepfe links">
      <button
        class="sekundaer"
        onclick={() => ondialog("projektdaten")}
        disabled={!darf("bearbeitung")}>Bearbeiten …</button
      >
      <button class="sekundaer" onclick={() => ondialog("bauherren")}
        >Bauherren …</button
      >
      <button class="sekundaer" onclick={() => (ansicht = "rechtsgrundlagen")}
        >Rechtsgrundlagen …</button
      >
    </div>
    <Rechtsgrundlagen
      bind:this={stammdatenEditor}
      {projekt}
      darf={darf("bearbeitung")}
      {onprojekt}
      bereich="stammdaten"
    />
  </section>
  <section hidden={ansicht !== "abstand"}>
    <h2>Abstandsflächen</h2>
    {#key projekt.id}
      <Abstandsflaeche
        startLand={projekt.bundesland}
        regelpakete={info?.regelpakete ?? []}
        onfehler={melde}
      />
    {/key}
  </section>
  <section hidden={ansicht !== "plaene"}>
    <h2>Pläne</h2>
    {#key projekt.id}
      <Plaene
        bind:this={planEditor}
        {projekt}
        aktiv={aktiv && ansicht === "plaene"}
        darf={darf("bearbeitung")}
        {onprojekt}
        bind:dokumentId={planDokument}
        onflaeche={grundstuecksflaecheUebernehmen}
        onfehler={melde}
      />
    {/key}
  </section>
  <section hidden={!berechnungsAnsichten.includes(ansicht)}>
    <h2>
      {berechnungsTitel[
        (ansicht === "mass" ? "grundstueck" : ansicht) as BerechnungsBereich
      ] ?? "Berechnungen"}
    </h2>
    {#key projekt.id}
      <MassDerNutzung
        bind:this={nutzungsEditor}
        {projekt}
        darf={darf("bearbeitung")}
        {onprojekt}
        bereich={(berechnungsAnsichten.includes(ansicht) && ansicht !== "mass"
          ? ansicht
          : "grundstueck") as BerechnungsBereich}
      />
    {/key}
  </section>
  <section
    aria-label="Rechtsgrundlagen"
    hidden={ansicht !== "rechtsgrundlagen"}
  >
    <h2>Rechtsgrundlagen</h2>
    {#key projekt.id}
      <Rechtsgrundlagen
        bind:this={rechtsEditor}
        {projekt}
        darf={darf("bearbeitung")}
        {onprojekt}
      />
    {/key}
  </section>
  <section hidden={ansicht !== "festsetzungen"}>
    <h2>Bebauungsplanfestsetzungen</h2>
    <Rechtsgrundlagen
      bind:this={festsetzungsEditor}
      {projekt}
      darf={darf("bearbeitung")}
      {onprojekt}
      bereich="festsetzungen"
    />
  </section>
  {#if formulareBesucht}<section hidden={ansicht !== "formulare"}>
      <ProjektFormulare
        bind:this={formularEditor}
        {projekt}
        darf={darf("bearbeitung")}
        ongespeichert={dokumenteAktualisieren}
        ondateien={() => (ansicht = "dokumente")}
      />
    </section>{/if}
  <!-- Bleibt eingehängt, damit "Datei → Bauantrag als XBau exportieren" aus jeder Ansicht funktioniert. -->
  <section hidden={ansicht !== "bauantrag"}>
    <h2>Bauantrag (XBau)</h2>
    {#if ich}
      {#key projekt.id}
        <Bauantrag
          bind:this={antragEditor}
          {projekt}
          benutzer={ich}
          {exportAusloeser}
          onbuero={() => ondialog("buero")}
          onbauherren={() => ondialog("bauherren")}
          onfehler={melde}
          onhinweis={(m) => (hinweis = m)}
        />
      {/key}
    {/if}
  </section>
  {#if ansicht === "protokoll"}
    <section>
      <h2>Protokoll</h2>
      <p class="meta">
        Wer hat wann was im Projekt geändert. Die neuesten Einträge stehen oben.
      </p>
      <!-- Wird bei jedem Wechsel auf den Reiter neu eingehängt und lädt dabei frisch. -->
      <Protokoll projektId={projekt.id} />
    </section>
  {/if}
  {#if einreichungenBesucht}<section hidden={ansicht !== "einreichungen"}>
      <Einreichungen
        bind:this={einreichungsEditor}
        projektId={projekt.id}
        darfAblegen={darf("bearbeitung")}
        onfehler={melde}
      />
    </section>{/if}
  <!-- Dokumente bleiben eingehängt, damit "Datei → Dokument importieren" aus jeder Ansicht funktioniert. -->
  <section hidden={ansicht !== "dokumente"}>
    <h2>Projektdateien</h2>
    {#if ich}<Dokumente
        bind:this={dokumentEditor}
        projektId={projekt.id}
        benutzer={ich}
        {importAusloeser}
        onplan={(id) => {
          planDokument = id;
          ansicht = "plaene";
        }}
        onfehler={melde}
      />{/if}
  </section>
</main>
