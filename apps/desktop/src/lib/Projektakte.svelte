<script lang="ts">
  import Arbeitsbereich from "./Arbeitsbereich.svelte";
  import Quellenweg from "./arbeitsplatz/Quellenweg.svelte";
  import EinreichenStart from "./EinreichenStart.svelte";
  import Projektorganisation from "./Projektorganisation.svelte";
  import Projektuebersicht from "./Projektuebersicht.svelte";
  import { projektNavigationAnlegen } from "./projektNavigation.svelte";
  import { MODULE, voraussetzung } from "./module";
  import {
    navigationMitModulen,
    moduleVon,
    registerVon,
    type NavZweig,
  } from "./navigation";
  import Sidebar from "./Sidebar.svelte";
  import Symbolknopf from "./Symbolknopf.svelte";
  import NavIcon from "./NavIcon.svelte";
  import type { UebersichtsZiel } from "./generated/plattform";
  import Fachbereich from "./fachmodule/Fachbereich.svelte";
  import { FACHMODULE } from "./fachmodule/register";
  import { projektEditoren } from "./editoren";
  import { untrack } from "svelte";
  import { melden } from "./projektEreignisse";
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
    type GrundstueckKontext,
    type Project,
  } from "./api";
  import {
    berechnungsAnsichten,
    berechnungsTitel,
    type BerechnungsBereich,
    type Ansicht,
  } from "./projektSitzung";
  import Abstandsflaeche from "./Abstandsflaeche.svelte";
  import AbstandsProjekt from "./abstandsflaechen/AbstandsProjekt.svelte";
  import Bauantrag from "./Bauantrag.svelte";
  import XbauPostfach from "./xbau/Postfach.svelte";
  import Portalbereich from "./portale/Portalbereich.svelte";
  import ProjektFormulare from "./ProjektFormulare.svelte";
  import Einreichungen from "./Einreichungen.svelte";
  import Druckausgabe from "./Druckausgabe.svelte";
  import Druckknopf from "./Druckknopf.svelte";
  import Dokumente from "./Dokumente.svelte";
  import Grundstuecksinformationen from "./Grundstuecksinformationen.svelte";
  import MassDerNutzung from "./MassDerNutzung.svelte";
  import Protokoll from "./Protokoll.svelte";
  import Rechtsgrundlagen from "./Rechtsgrundlagen.svelte";
  let {
    projekt,
    ansicht = $bindable("projektuebersicht"),
    planDokument = $bindable(null),
    aktiv,
    info,
    ich,
    darf,
    darfMitglieder = false,
    importAusloeser,
    onprojekt,
    ondialog,
    onhilfe,
    onbibliothek,
  }: {
    projekt: Project;
    ansicht?: Ansicht;
    planDokument?: string | null;
    aktiv: boolean;
    info: AppInfo | null;
    ich: Benutzer | null;
    darf: (min: "bearbeitung" | "projektleitung") => boolean;
    /** Projektmitglieder verwalten (Projektrecht „Leitung“). */
    darfMitglieder?: boolean;
    importAusloeser: number;
    onprojekt: (p: Project) => void;
    onhilfe?: () => void;
    onbibliothek?: () => void;
    ondialog: (
      art: "projektdaten" | "bauherren" | "beteiligte" | "buero" | "mitglieder",
    ) => void;
  } = $props();
  const editoren = projektEditoren();
  const navigation = projektNavigationAnlegen();
  $effect(() => {
    navigation.benutzerId = ich?.id ?? null;
    navigation.projektId = projekt.id;
  });
  // Aktenkopf: Im Aufmaß standardmäßig eingeklappt, damit die Zeichenfläche
  // Platz bekommt; die Wahl wird je Arbeitsplatz für Aufmaß und übrige
  // Bereiche getrennt gemerkt.
  const KOPF_SCHLUESSEL = "bauprojekt.aktenkopf";
  function kopfLesen(): { aufmass: boolean; sonst: boolean } {
    try {
      const v = JSON.parse(localStorage.getItem(KOPF_SCHLUESSEL) ?? "null");
      if (v && typeof v.aufmass === "boolean" && typeof v.sonst === "boolean")
        return v;
    } catch {}
    return { aufmass: true, sonst: false };
  }
  let kopfWahl = $state(kopfLesen());
  const kopfKompakt = $derived(
    ansicht === "planaufmass" ? kopfWahl.aufmass : kopfWahl.sonst,
  );
  function kopfUmschalten() {
    kopfWahl =
      ansicht === "planaufmass"
        ? { ...kopfWahl, aufmass: !kopfKompakt }
        : { ...kopfWahl, sonst: !kopfKompakt };
    try {
      localStorage.setItem(KOPF_SCHLUESSEL, JSON.stringify(kopfWahl));
    } catch {}
  }
  let navigationsNummer = 0;
  $effect(() => {
    const a = navigation.auftrag;
    if (!a || a.nummer === navigationsNummer) return;
    navigationsNummer = a.nummer;
    if (MODULE.some((m) => m.id === a.ziel.bereich))
      ansicht = a.ziel.bereich as Ansicht;
    // Sprung ins Aufmaß einer bestimmten Unterlage (z. B. Flurkarte):
    // Register „unterlage“ mit der Dokument-ID als Eintrag.
    if (
      a.ziel.bereich === "planaufmass" &&
      a.ziel.register === "unterlage" &&
      a.ziel.eintrag
    )
      unterlageOeffnen(a.ziel.eintrag);
  });
  // Wunsch an das Aufmaß, eine Unterlage zu öffnen; die Nummer macht jeden
  // Wunsch unterscheidbar, auch für dieselbe Datei.
  let unterlageWunsch = $state<{ dokumentId: string; nummer: number } | null>(
    null,
  );
  let wunschNummer = 0;
  function unterlageOeffnen(dokumentId: string) {
    planDokument = dokumentId;
    unterlageWunsch = { dokumentId, nummer: ++wunschNummer };
    ansicht = "planaufmass";
  }
  // Von außen gesetzte Unterlage (z. B. Projektstart mit Flurkarte).
  $effect(() => {
    const id = planDokument;
    untrack(() => {
      if (id && unterlageWunsch?.dokumentId !== id)
        unterlageWunsch = { dokumentId: id, nummer: ++wunschNummer };
    });
  });
  function oeffnen(ziel: UebersichtsZiel) {
    if (!MODULE.some((m) => m.id === ziel.bereich)) return;
    navigation.springen(ziel);
    ansicht = ziel.bereich as Ansicht;
  }
  let fachBesucht = $state<Record<string, boolean>>({});
  $effect(() => {
    if (
      ansicht === "xbau" ||
      ansicht === "portale" ||
      FACHMODULE.some((m) => m.id === ansicht)
    )
      fachBesucht[ansicht] = true;
  });
  let nutzungsEditor = $state<MassDerNutzung>();
  let formularEditor = $state<ProjektFormulare>();
  let festsetzungsEditor = $state<Rechtsgrundlagen>();
  let stammdatenEditor = $state<Rechtsgrundlagen>();
  let einreichenBesucht = $state(false);
  $effect(() => {
    if (ansicht === "einreichen") einreichenBesucht = true;
  });
  let druckBesucht = $state(false);
  $effect(() => {
    if (ansicht === "druck") druckBesucht = true;
  });
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
  let grundstuecke = $state<GrundstueckKontext[]>([]);
  let flaechenFehler = $state<string | null>(null);
  $effect(() => {
    const id = projekt.id;
    const version = projekt.version;
    grundstuecke = [];
    flaechenFehler = null;
    void api
      .grundstueckskontext(id)
      .then((k) => {
        if (projekt.id === id && projekt.version === version) {
          grundstuecke = k;
          flaechenFehler = null;
        }
      })
      .catch((e) => {
        if (projekt.id === id && projekt.version === version) {
          grundstuecke = [];
          flaechenFehler = fehlertext(e);
        }
      });
  });
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
  /** Gespeicherte Formulare: alle Bereiche dieser Akte laden ihre Dokumente neu. */
  export async function dokumenteAktualisieren() {
    melden({ projektId: projekt.id, bereich: "dokumente" });
  }
  /** Sprung von außen, z. B. aus dem Büro-Cockpit. */
  export function springen(ziel: UebersichtsZiel) {
    oeffnen(ziel);
  }
  export function hatAenderungen() {
    return editoren.hatAenderungen();
  }
  export function istBeschaeftigt() {
    return editoren.istBeschaeftigt();
  }
  export async function speichern(): Promise<boolean> {
    if (istBeschaeftigt()) return false;
    const blockiert = await editoren.speichern();
    if (blockiert) {
      ansicht = blockiert.bereich;
      fehler =
        blockiert.hinweis ??
        "Eingaben konnten nicht vollständig gespeichert werden. Bitte den markierten Bereich prüfen.";
      return false;
    }
    return !hatAenderungen();
  }
  const REGISTER = navigationMitModulen(MODULE);
  const aktuellesRegister = $derived(registerVon(REGISTER, ansicht));
  // Jedes Register merkt sich seinen zuletzt geöffneten Bereich.
  let zuletzt = $state<Record<string, Ansicht>>({});
  $effect(() => {
    const a = ansicht;
    const r = registerVon(REGISTER, a);
    if (r) untrack(() => (zuletzt[r.id] = a));
  });
  function bereich(id: Ansicht) {
    const modul = MODULE.find((m) => m.id === id);
    ansicht =
      modul?.id === "festsetzungen" && voraussetzung(projekt, modul)
        ? "rechtsgrundlagen"
        : id;
  }
  function registerOeffnen(r: NavZweig) {
    const ziel =
      zuletzt[r.id] ??
      moduleVon(r).find((id) => {
        const m = MODULE.find((m) => m.id === id);
        return m && !voraussetzung(projekt, m);
      });
    if (ziel) bereich(ziel);
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
<header class="projektkopf" aria-label="Aktenkopf">
  <div class="aktenkopf-titel">
    <h1>{projekt.name}</h1>
    <div class="kopfaktionen">
      <Symbolknopf
        titel={kopfKompakt ? "Projektdetails anzeigen" : "Projektdetails einklappen"}
        tip="links"
        aria-expanded={!kopfKompakt}
        onclick={kopfUmschalten}
        ><svg viewBox="0 0 16 16" aria-hidden="true"
          ><path d={kopfKompakt ? "M4 6l4 4 4-4" : "M4 10l4-4 4 4"} /></svg
        ></Symbolknopf
      >
      {#if onhilfe}<Symbolknopf titel="Hilfe zu diesem Bereich" tip="links" onclick={onhilfe}
          ><NavIcon art="hilfe" /></Symbolknopf
        >{/if}
    </div>
  </div>
  <dl class="aktendaten" hidden={kopfKompakt}>
    <div>
      <dt>Bundesland</dt>
      <dd>{landName(projekt.bundesland)}</dd>
    </div>
    <div>
      <dt>{bauherren.length > 1 ? "Bauherren" : "Bauherr"}</dt>
      <dd>{bauherren.map(kundenname).join(", ") || "–"}</dd>
    </div>
    <div>
      <dt>Baugrundstück</dt>
      <dd>
        {grundstuecke.map((g) => g.bezeichnung).join(", ") ||
          "Noch nicht ermittelt"}
      </dd>
    </div>
    <div>
      <dt>Rechtsgrundlage</dt>
      <dd>{rechtsgrundlageName(projekt.planungsrecht.rechtsgrundlage)}</dd>
    </div>
    <div>
      <dt>Stand</dt>
      <dd>
        Version {projekt.version}, {new Date(
          projekt.updated_at,
        ).toLocaleDateString("de-DE", { day: "2-digit", month: "2-digit", year: "numeric" })}
      </dd>
    </div>
  </dl>
</header>
<div class="register" role="tablist" aria-label="Register der Projektakte">
  {#each REGISTER as r (r.id)}
    <button
      role="tab"
      aria-selected={aktuellesRegister?.id === r.id}
      class:aktiv={aktuellesRegister?.id === r.id}
      aria-label={r.titel}
      title={r.untertitel}
      onclick={() => registerOeffnen(r)}
      ><span class="registertitel"
        >{#if r.ziffer}<small class="ziffer">{r.ziffer}</small>{/if}{r.titel}</span
      ></button
    >
  {/each}
</div>

<div class="aktenrumpf">
  <Sidebar {projekt} {aktiv} {ansicht} onbereich={bereich} />
  <main>
    {#if navigation.rueckweg && navigation.rueckweg.bereich !== ansicht}<button
        class="link"
        onclick={() => {
          const z = navigation.rueckweg;
          navigation.rueckweg = null;
          if (z) navigation.springen(z);
        }}>Zurück zum vorherigen Arbeitsbereich →</button
      >{/if}
    {#if ["grundstueck", "grz", "gfz", "geschosse", "volumen", "abstand", "ueberbaubarkeit", "stellplaetze", "wohnflaechen", "din277", "vollgeschosse", "hoehen", "freiflaechen", "entwaesserung", "dokumente", "formulare", "druck", "unterlagen"].includes(ansicht)}<Quellenweg
        bereich={ansicht}
      />{/if}
    {#if einreichenBesucht}<section hidden={ansicht !== "einreichen"}>
        <Arbeitsbereich aktiv={aktiv && !(ansicht !== "einreichen")}
          ><EinreichenStart
            projektId={projekt.id}
            version={projekt.version}
            entwuerfe={editoren.hatAenderungen()}
            aktiv={aktiv && ansicht === "einreichen"}
          /></Arbeitsbereich
        >
      </section>{/if}
    {#each FACHMODULE as modul (modul.id)}
      {#if fachBesucht[modul.id]}<section hidden={ansicht !== modul.id}>
          <Arbeitsbereich aktiv={aktiv && !(ansicht !== modul.id)}>
            <Fachbereich
              laden={modul.laden}
              aktiv={aktiv && ansicht === modul.id}
              {projekt}
              {onprojekt}
              benutzerId={ich?.id ?? ""}
              darf={darf("bearbeitung")}
              darfFreigeben={darf("projektleitung")}
              unterlage={modul.id === "planaufmass" ? unterlageWunsch : null}
            />
          </Arbeitsbereich>
        </section>{/if}
    {/each}
    {#if ansicht === "projektuebersicht"}<Projektuebersicht
        projektId={projekt.id}
        version={projekt.version}
        sichtbar={aktiv}
        benutzerId={ich?.id ?? ""}
        entwuerfe={editoren.hatAenderungen()}
        onoeffnen={oeffnen}
      />{/if}
    <section hidden={ansicht !== "uebersicht"}>
      <Arbeitsbereich aktiv={aktiv && !(ansicht !== "uebersicht")}>
        <h2>Stammdaten</h2>
        <dl class="daten">
          <dt>Name</dt>
          <dd>{projekt.name}</dd>
          <dt>Bundesland</dt>
          <dd>{landName(projekt.bundesland)}</dd>
          <dt>Grundstücksfläche</dt>
          <dd>
            {#each grundstuecke as g (g.schluessel)}<div>
                {g.bezeichnung}: {g.bezugsflaeche_m2?.toLocaleString("de-DE") ??
                  "noch zu prüfen"}
                {g.bezugsflaeche_m2 === null ? "" : "m²"}<span class="meta">
                  · {g.quelle}</span
                >
              </div>{:else}{flaechenFehler ??
                "Noch keine Grundstücksfläche ermittelt"}{/each}
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
          <button class="sekundaer" onclick={() => ondialog("beteiligte")}
            >Weitere Beteiligte …</button
          >
          {#if darfMitglieder}
            <button class="sekundaer" onclick={() => ondialog("mitglieder")}
              >Projektmitglieder …</button
            >
          {/if}
          <button
            class="sekundaer"
            onclick={() => (ansicht = "rechtsgrundlagen")}
            >Rechtsgrundlagen …</button
          >
        </div>
        <Projektorganisation
          projektId={projekt.id}
          benutzerId={ich?.id ?? ""}
          darf={darf("bearbeitung")}
        />
        <Rechtsgrundlagen
          bind:this={stammdatenEditor}
          {projekt}
          darf={darf("bearbeitung")}
          {onprojekt}
          bereich="stammdaten"
        />
      </Arbeitsbereich>
    </section>
    <section hidden={ansicht !== "abstand"}>
      <Arbeitsbereich aktiv={aktiv && !(ansicht !== "abstand")}>
        <h2>Abstandsflächen</h2>
        {#key projekt.id}
          <AbstandsProjekt
            {projekt}
            benutzerId={ich?.id ?? ""}
            darf={darf("bearbeitung")}
            darfFreigeben={darf("projektleitung")}
          />
          <details>
            <summary>Einfache Tiefenvorprüfung (bisheriger Rechner)</summary
            ><Abstandsflaeche
              startLand={projekt.bundesland}
              projektId={projekt.id}
              darfSpeichern={darf("bearbeitung")}
              regelpakete={info?.regelpakete ?? []}
              onfehler={melde}
            />
          </details>
        {/key}
      </Arbeitsbereich>
    </section>
    <section
      hidden={ansicht !== "flurstuecke"}
      aria-label="Grundstücksinformationen"
    >
      <Arbeitsbereich aktiv={aktiv && !(ansicht !== "flurstuecke")}>
        <h2>Flurstücke und Grundbuch</h2>
        {#key projekt.id}<Grundstuecksinformationen
            aktiv={aktiv && ansicht === "flurstuecke"}
            {projekt}
            darf={darf("bearbeitung")}
            {onprojekt}
          />{/key}
      </Arbeitsbereich>
    </section>
    <section hidden={!berechnungsAnsichten.includes(ansicht)}>
      <Arbeitsbereich aktiv={aktiv && !!berechnungsAnsichten.includes(ansicht)}>
        <div class="maskenkopf">
          <h2>
            {berechnungsTitel[
              (ansicht === "mass"
                ? "grundstueck"
                : ansicht) as BerechnungsBereich
            ] ?? "Berechnungen"}
          </h2>
          <Druckknopf
            projektId={projekt.id}
            quelle={ansicht === "grundstueck"
              ? { art: "baugrundstueck" }
              : undefined}
            disabled={!!nutzungsEditor?.hatAenderungen()}
            text="Druckausgabe zusammenstellen"
          />
        </div>
        {#key projekt.id}
          <MassDerNutzung
            benutzerId={ich?.id ?? ""}
            bind:this={nutzungsEditor}
            bezugsflaechenAktuell={grundstuecke.length > 0 && !flaechenFehler}
            {projekt}
            darf={darf("bearbeitung")}
            {onprojekt}
            bereich={(berechnungsAnsichten.includes(ansicht) &&
            ansicht !== "mass"
              ? ansicht
              : "grundstueck") as BerechnungsBereich}
          />
        {/key}
      </Arbeitsbereich>
    </section>
    <section
      aria-label="Rechtsgrundlagen"
      hidden={ansicht !== "rechtsgrundlagen"}
    >
      <Arbeitsbereich aktiv={aktiv && !(ansicht !== "rechtsgrundlagen")}>
        <h2>Rechtsgrundlagen</h2>
        {#key projekt.id}
          <Rechtsgrundlagen
            bind:this={rechtsEditor}
            {projekt}
            darf={darf("bearbeitung")}
            {onprojekt}
          />
        {/key}
      </Arbeitsbereich>
    </section>
    <section hidden={ansicht !== "festsetzungen"}>
      <Arbeitsbereich aktiv={aktiv && !(ansicht !== "festsetzungen")}>
        <h2>Bebauungsplanfestsetzungen</h2>
        <Rechtsgrundlagen
          bind:this={festsetzungsEditor}
          {projekt}
          darf={darf("bearbeitung")}
          {onprojekt}
          bereich="festsetzungen"
        />
      </Arbeitsbereich>
    </section>
    {#if druckBesucht}<section hidden={ansicht !== "druck"}>
        <Arbeitsbereich aktiv={aktiv && !(ansicht !== "druck")}>
          <Druckausgabe
            projektId={projekt.id}
            darfAblegen={darf("bearbeitung")}
          />
        </Arbeitsbereich>
      </section>{/if}
    {#if formulareBesucht}<section hidden={ansicht !== "formulare"}>
        <Arbeitsbereich aktiv={aktiv && !(ansicht !== "formulare")}>
          <ProjektFormulare
            bind:this={formularEditor}
            {projekt}
            darf={darf("bearbeitung")}
            {onbibliothek}
            aktiv={aktiv && ansicht === "formulare"}
            ongespeichert={dokumenteAktualisieren}
            ondateien={() => (ansicht = "dokumente")}
          />
        </Arbeitsbereich>
      </section>{/if}
    {#if fachBesucht.portale && ich}<section hidden={ansicht !== "portale"}>
        <Arbeitsbereich aktiv={aktiv && !(ansicht !== "portale")}>
          <h2>Einreichen / Portal</h2>
          <Portalbereich {projekt} benutzer={ich} />
        </Arbeitsbereich>
      </section>{/if}
    {#if fachBesucht.xbau && ich}<section hidden={ansicht !== "xbau"}>
        <Arbeitsbereich aktiv={aktiv && !(ansicht !== "xbau")}>
          <h2>XBau-Nachrichten</h2>
          <XbauPostfach {projekt} benutzer={ich} />
        </Arbeitsbereich>
      </section>{/if}
    <!-- Bleibt eingehängt, damit ungespeicherte Antragsänderungen beim Bereichswechsel erhalten bleiben. -->
    <section hidden={ansicht !== "bauantrag"}>
      <Arbeitsbereich aktiv={aktiv && !(ansicht !== "bauantrag")}>
        <h2>Bauantrag (XBau)</h2>
        {#if ich}
          {#key projekt.id}
            <Bauantrag
              bind:this={antragEditor}
              {projekt}
              benutzer={ich}
              onbuero={() => ondialog("buero")}
              onbauherren={() => ondialog("bauherren")}
              onbeteiligte={() => ondialog("beteiligte")}
              onfehler={melde}
            />
          {/key}
        {/if}
      </Arbeitsbereich>
    </section>
    {#if ansicht === "protokoll"}
      <section>
        <h2>Chronik</h2>
        <p class="meta">
          Wer hat wann was im Projekt geändert. Die neuesten Einträge stehen
          oben.
        </p>
        <!-- Wird bei jedem Wechsel auf den Reiter neu eingehängt und lädt dabei frisch. -->
        <Protokoll projektId={projekt.id} />
      </section>
    {/if}
    {#if einreichungenBesucht}<section hidden={ansicht !== "einreichungen"}>
        <Arbeitsbereich aktiv={aktiv && !(ansicht !== "einreichungen")}>
          <Einreichungen
            bind:this={einreichungsEditor}
            projektId={projekt.id}
            darfAblegen={darf("bearbeitung")}
            onfehler={melde}
          />
        </Arbeitsbereich>
      </section>{/if}
    <!-- Dokumente bleiben eingehängt, damit "Datei → Dokument importieren" aus jeder Ansicht funktioniert. -->
    <section hidden={ansicht !== "dokumente"}>
      <Arbeitsbereich aktiv={aktiv && !(ansicht !== "dokumente")}>
        <h2>Projektdateien</h2>
        {#if ich}<Dokumente
            bind:this={dokumentEditor}
            projektId={projekt.id}
            benutzer={ich}
            aktiv={aktiv && ansicht === "dokumente"}
            {importAusloeser}
            onplan={unterlageOeffnen}
            onfehler={melde}
          />{/if}
      </Arbeitsbereich>
    </section>
  </main>
</div>

<style>
  .kopfaktionen {
    display: flex;
    gap: 2px;
  }
  .aktenkopf-titel {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
  }
</style>
