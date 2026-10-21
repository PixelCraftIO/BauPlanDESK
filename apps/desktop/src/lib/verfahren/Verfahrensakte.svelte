<script lang="ts">
  import { projektNavigation } from "../projektNavigation.svelte";
  import { onMount, untrack, tick } from "svelte";
  import {
    api,
    fehlertext,
    type Project,
    type DocumentVersion,
    type Benutzer,
  } from "../api";
  import type {
    Verfahrensakte,
    Abgabewunsch,
    Vorhabenprofil,
    Anforderungskatalog,
    Fachstand,
    Dokumentbezug,
    Berichtigungsgegenstand,
    Ereignisart,
    ExterneEinreichungswunsch,
  } from "../generated/plattform";
  import { FachSitzung } from "../FachSitzung.svelte";
  import Fachliste from "../Fachliste.svelte";
  import {
    leer,
    verfahrenApi,
    heute,
    STATUS,
    EREIGNISART,
    VERFAHRENSART,
    baugenehmigung,
    berichtigungsereignis,
  } from "./api";
  import Statusangaben from "./Statusangaben.svelte";
  import Schritte from "./Schritte.svelte";
  import Kommunikation from "./Kommunikation.svelte";
  import Nachforderungen from "./Nachforderungen.svelte";
  import Bescheide from "./Bescheide.svelte";
  import Abweichungsantraege from "./Abweichungsantraege.svelte";
  import Gebuehren from "./Gebuehren.svelte";
  import Verfahrenswahl from "./Verfahrenswahl.svelte";
  import Unterschriften from "./Unterschriften.svelte";
  import Schreiben from "./Schreiben.svelte";
  import Anforderungen from "./Anforderungen.svelte";
  import Abgaben from "./Abgaben.svelte";
  import ExterneEinreichungen from "./ExterneEinreichungen.svelte";
  import Profil from "./Profil.svelte";
  import Zustaendigkeit from "./Zustaendigkeit.svelte";
  import Fristfeld from "./Fristfeld.svelte";
  import Antragsangaben from "./Antragsangaben.svelte";
  import Dokumentfeld from "./Dokumentfeld.svelte";
  import Dokumentliste from "./Dokumentliste.svelte";
  let {
    projekt,
    benutzerId,
    darf,
    darfFreigeben = false,
  }: {
    projekt: Project;
    benutzerId: string;
    darf: boolean;
    darfFreigeben?: boolean;
  } = $props();
  const s = untrack(
    () =>
      new FachSitzung<Verfahrensakte>(
        projekt.id,
        "verfahren",
        benutzerId,
        leer,
        (a) =>
          `${a.bezeichnung} · ${a.aktenzeichen || "ohne Aktenzeichen"} · ${STATUS[a.status]}`,
      ),
  );
  let dateien = $state<DocumentVersion[]>([]),
    benutzer = $state<Benutzer[]>([]),
    tab = $state("stamm"),
    ersatzVor = $state<Dokumentbezug | null>(null),
    ersatzNach = $state<Dokumentbezug | null>(null),
    ersatzGrund = $state(""),
    notiz = $state(""),
    notizDatum = $state(heute()),
    notizBeleg = $state<Dokumentbezug | null>(null),
    notizBerichtigt = $state<string | null>(null),
    chronikArt = $state<Ereignisart | "">("");
  /**
   * Merkt eine begründete Berichtigung für die nächste Speicherung vor; eine
   * frühere, noch ungespeicherte Berichtigung desselben Werts wird ersetzt.
   */
  function berichtigen(
    a: Verfahrensakte,
    g: Berichtigungsgegenstand,
    grund: string,
  ) {
    const gespeichert = s.original?.ereignisse.length ?? 0;
    const gleich = (x?: Berichtigungsgegenstand) =>
      !!x && JSON.stringify(x) === JSON.stringify(g);
    a.ereignisse = [
      ...a.ereignisse.slice(0, gespeichert),
      ...a.ereignisse
        .slice(gespeichert)
        .filter((e) => !gleich(e.berichtigung?.gegenstand)),
      berichtigungsereignis(g, grund, benutzerId),
    ];
  }
  /** Nur Unterlagen aus versendeten Abgaben und externen Einreichungen dürfen
   * als genehmigt gelten; die Auswahl ist eine Vorschau, der Core prüft
   * verbindlich (auch Datum und Berichtigungen). */
  function abgegeben(a: Verfahrensakte) {
    const extern = a.externe_einreichungen ?? [];
    const enthalten = a.abgaben
      .filter(
        (b) =>
          b.versandstatus === "versendet" || b.versandstatus === "eingegangen",
      )
      .flatMap((b) => b.dokumente.map((d) => d.dokument))
      .concat(
        extern
          .filter((e) => !extern.some((n) => n.berichtigt === e.id))
          .flatMap((e) => e.dokumente),
      );
    return dateien.filter((d) =>
      enthalten.some(
        (e) => e.dokument_id === d.document_id && e.version === d.version,
      ),
    );
  }
  const navigation = projektNavigation();
  let letzterSprung = 0;
  $effect(() => {
    const a = navigation?.auftrag;
    if (
      !a ||
      a.nummer === letzterSprung ||
      a.ziel.bereich !== "verfahren" ||
      s.id !== a.ziel.objekt
    )
      return;
    letzterSprung = a.nummer;
    if (a.ziel.register) tab = a.ziel.register;
    if (a.ziel.eintrag)
      void tick().then(() => {
        const e = document.querySelector(
          `[data-projekt-id="${projekt.id}"] [data-uebersicht-eintrag="${a.ziel.eintrag}"]`,
        );
        if (e instanceof HTMLDetailsElement) e.open = true;
        e?.scrollIntoView({ block: "center" });
      });
  });
  async function dateienLaden() {
    try {
      [dateien, benutzer] = await Promise.all([
        api.dokumente(projekt.id),
        api.benutzerAuflisten(),
      ]);
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
  onMount(dateienLaden);
  function uebernehmen(f: Fachstand) {
    const a = f.daten as unknown as Verfahrensakte;
    s.revision = f.revision;
    s.original = structuredClone(a);
    s.eingabe = structuredClone(a);
    s.liste = [
      ...s.liste.filter((n) => n.objekt !== f.objekt),
      { ...f, daten: a },
    ];
    s.meldung = "Gespeichert.";
  }
  async function aktion(fn: (basis: number) => Promise<Fachstand>) {
    if (!(await s.speichern())) return;
    s.laeuft = true;
    s.fehler = null;
    try {
      uebernehmen(await fn(s.revision));
    } catch (e) {
      s.fehler = fehlertext(e);
    } finally {
      s.laeuft = false;
    }
  }
  const festhalten = (w: Abgabewunsch) =>
    aktion((b) => verfahrenApi.abgabe(projekt.id, s.id!, b, w));
  async function extern(w: ExterneEinreichungswunsch) {
    await aktion((b) =>
      verfahrenApi.externeEinreichung(projekt.id, s.id!, b, w),
    );
    return !s.fehler;
  }
  const profil = (p: Vorhabenprofil, k: Anforderungskatalog) =>
    aktion((b) => verfahrenApi.profilAnwenden(projekt.id, s.id!, b, p, k));
  const tabs = [
    ["stamm", "Verfahren"],
    ["profil", "Vorhabenprofil"],
    ["anforderungen", "Anforderungen"],
    ["abweichungen", "Abweichungen / Befreiungen"],
    ["unterschriften", "Unterschriften"],
    ["kommunikation", "Kommunikation"],
    ["nachforderungen", "Nachforderungen"],
    ["schritte", "Behördliche Schritte"],
    ["schreiben", "Schreiben"],
    ["abgaben", "Abgabestände"],
    ["bescheide", "Bescheide / Auflagen"],
    ["beziehungen", "Korrekturen"],
    ["chronik", "Verfahrenschronik"],
  ];
  // Die Akte ist maßgebend für Behörde und Aktenzeichen; ältere Projekte
  // führen die Bauaufsicht noch im Bauantrag.
  async function bauaufsichtUebernehmen(a: Verfahrensakte) {
    try {
      const b = (await api.bauantrag(projekt.id)).antrag.bauaufsicht;
      if (!b.kennung.trim()) {
        s.fehler = "Der Bauantrag enthält keine Behördenkennung.";
        return;
      }
      a.behoerde.kennung = b.kennung;
      if (!a.behoerde.name.trim()) a.behoerde.name = b.name;
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
</script>

<Fachliste
  titel="Verfahrensakte"
  {s}
  bereich="verfahren"
  {darf}
  lesendBedienbar
>
  {#snippet children(a)}
    {#key s.id}
      <nav class="verfahren-tabs" aria-label="Verfahrensbereiche">
        {#each tabs as [id, titel]}<button
            type="button"
            class="sekundaer"
            class:aktiv={tab === id}
            aria-pressed={tab === id}
            onclick={() => (tab = id)}>{titel}</button
          >{/each}
      </nav>
      <button type="button" class="link" onclick={dateienLaden}
        >Dokumentversionen aktualisieren</button
      >
      {#if tab === "stamm"}<fieldset disabled={!darf}>
          <div class="raster">
            <label
              >Verfahrensbezeichnung<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Verfahrensbezeichnung"
                required
                maxlength="500"
                bind:value={a.bezeichnung}
              /></label
            ><label
              >Verfahrensart<select bind:value={a.art}
                >{#each Object.entries(VERFAHRENSART) as [wert, titel]}<option
                    value={wert}>{titel}</option
                  >{/each}</select
              ></label
            >{#if baugenehmigung(a.art)}<label
                >Prüfverfahren<select
                  value={a.pruefverfahren ?? ""}
                  onchange={(e) =>
                    (a.pruefverfahren = (e.currentTarget.value ||
                      undefined) as Verfahrensakte["pruefverfahren"])}
                  ><option value="">Noch nicht festgelegt</option
                  ><option value="regulaer">Reguläres Verfahren</option
                  ><option value="vereinfacht">Vereinfachtes Verfahren</option
                  ></select
                ></label
              >{/if}<label
              >Bearbeitungsstand<select bind:value={a.status}
                >{#each Object.entries(STATUS) as [wert, titel]}<option
                    value={wert}>{titel}</option
                  >{/each}</select
              ></label
            ><label
              >Aktenzeichen<input
                maxlength="500"
                bind:value={a.aktenzeichen}
              /></label
            ><label
              >Behörde<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Behörde"
                required
                maxlength="500"
                bind:value={a.behoerde.name}
              /></label
            ><label
              >Behördenkennung<input bind:value={a.behoerde.kennung} /></label
            >{#if !a.behoerde.kennung.trim() && darf}<button
                type="button"
                class="sekundaer"
                onclick={() => bauaufsichtUebernehmen(a)}
                >Behörde aus dem Bauantrag übernehmen</button
              >{/if}<label>Kontakt<input bind:value={a.behoerde.kontakt} /></label
            ><Zustaendigkeit {benutzer} bind:wert={a.verantwortlich} /><label
              >Vorgängerverfahren<select bind:value={a.vorgaenger}
                ><option value={null}>Keines</option
                >{#each s.liste.filter((v) => v.objekt !== s.id) as v}<option
                    value={v.objekt}>{v.daten.bezeichnung}</option
                  >{/each}</select
              ></label
            ><label
              >Bereits erfasste Behörde übernehmen<select
                value=""
                onchange={(e) => {
                  const b = s.liste.find(
                    (v) => v.daten.behoerde.id === e.currentTarget.value,
                  )?.daten.behoerde;
                  if (b) a.behoerde = structuredClone($state.snapshot(b));
                }}
                ><option value="">Behörde auswählen</option
                >{#each [...new Map(s.liste.map( (v) => [v.daten.behoerde.id, v.daten.behoerde], )).values()] as b}<option
                    value={b.id}>{b.name}</option
                  >{/each}</select
              ></label
            >
          </div>
          {#if a.art === "widerspruch"}<p class="meta">
              Ein Widerspruch benötigt das Vorgängerverfahren mit dem
              angefochtenen Bescheid.
            </p>{/if}
          <fieldset>
            <legend>Bearbeitungsfrist der Behörde</legend>
            <p class="meta">
              Frist oder Eintritt einer Genehmigungsfiktion, wie sie die Behörde
              etwa in der Eingangs- oder Vollständigkeitsbestätigung nennt. Die
              Wiedervorlage läuft ab der Einreichung bis zur Entscheidung.
            </p>
            <Fristfeld {dateien} bind:wert={a.bearbeitungsfrist} />
          </fieldset>
          <Statusangaben
            status={a.status}
            art={a.art}
            kommunikation={a.kommunikation}
            bind:vollstaendigkeit={a.vollstaendigkeit}
            bind:ruecknahme={a.ruecknahme}
            bind:ausfuehrung={a.ausfuehrung}
            gespeichert={{
              vollstaendigkeit: s.original?.vollstaendigkeit,
              ruecknahme: s.original?.ruecknahme,
              ausfuehrung: s.original?.ausfuehrung,
            }}
            onberichtigen={(g, grund) => berichtigen(a, g, grund)}
          />
          <Antragsangaben art={a.art} bind:angaben={a.antragsangaben} />
        </fieldset>
      {:else if tab === "profil"}<fieldset disabled={!darf}>
          <Profil
            {projekt}
            art={a.art}
            stand={a.profil}
            onprofil={profil}
          />{#if a.profil}<label
              ><input
                type="checkbox"
                checked={!!a.profil.bestaetigt_von}
                disabled={!darfFreigeben}
                onchange={(e) => {
                  a.profil!.bestaetigt_von = e.currentTarget.checked
                    ? benutzerId
                    : null;
                  a.profil!.bestaetigt_am = null;
                }}
              />Vorhabenprofil fachlich bestätigt</label
            >{/if}
        <Verfahrenswahl
            projektId={projekt.id}
            verfahrenId={s.id}
            onuebernehmen={(art, pruef) => {
              a.art = art;
              a.pruefverfahren = pruef ?? undefined;
            }}
          />
        </fieldset>
      {:else if tab === "anforderungen"}<fieldset disabled={!darf}>
          <Anforderungen
            bind:eintraege={a.anforderungen}
            {dateien}
            {benutzer}
            {darfFreigeben}
          />
        </fieldset>
      {:else if tab === "schreiben"}<Schreiben
          akte={a}
          projektId={projekt.id}
          verfahrenId={s.id}
          {darf}
          ondokument={dateienLaden}
        />
      {:else if tab === "unterschriften"}<fieldset disabled={!darf}>
          <Unterschriften
            bind:eintraege={
              () => a.unterschriften ?? [], (w) => (a.unterschriften = w)
            }
            projektId={projekt.id}
            verfahrenId={s.id}
          />
        </fieldset>
      {:else if tab === "abweichungen"}<fieldset disabled={!darf}>
          <Abweichungsantraege
            bind:eintraege={
              () => a.abweichungen ?? [], (w) => (a.abweichungen = w)
            }
            bescheide={a.bescheide}
            {dateien}
            projektId={projekt.id}
            verfahrenId={s.id}
            alt={s.original?.abweichungen?.map((v) => v.id) ?? []}
          />
        </fieldset>
      {:else if tab === "kommunikation"}<fieldset disabled={!darf}>
          <Kommunikation
            bind:eintraege={a.kommunikation}
            akte={a}
            {dateien}
            alt={s.original?.kommunikation.map((v) => v.id) ?? []}
          />
        </fieldset>
      {:else if tab === "nachforderungen"}<fieldset disabled={!darf}>
          <Nachforderungen
            bind:eintraege={a.nachforderungen}
            akte={a}
            {dateien}
            {benutzer}
            alt={s.original?.nachforderungen.flatMap((n) =>
              n.punkte.map((p) => p.id),
            ) ?? []}
          />
        </fieldset>
      {:else if tab === "schritte"}<fieldset disabled={!darf}>
          <Schritte
            bind:eintraege={a.schritte}
            kommunikation={a.kommunikation}
            abweichungen={a.abweichungen ?? []}
            gruendeAlt={s.original?.schritte?.flatMap((v) =>
              (v.gruende ?? []).map((g) => g.id),
            ) ?? []}
            {dateien}
            {benutzer}
            alt={s.original?.schritte?.map((v) => v.id) ?? []}
          />
        </fieldset>
      {:else if tab === "abgaben"}<Abgaben
          bind:eintraege={a.abgaben}
          {dateien}
          projektId={projekt.id}
          verfahrenId={s.id!}
          {darf}
          behoerde={a.behoerde}
          aktenzeichen={a.aktenzeichen}
          gespeichert={s.original?.abgaben ?? []}
          portalZiele={Object.fromEntries(
            a.portal?.vorgaenge.flatMap((v) =>
              v.abgaben.map((b) => [b.id, v.id]),
            ) ?? [],
          )}
          onportal={(id) =>
            navigation?.springen({
              bereich: "portale",
              objekt: s.id,
              register:
                a.portal?.vorgaenge.find((v) =>
                  v.abgaben.some((b) => b.id === id),
                )?.id ?? null,
              eintrag: id,
            })}
          onfesthalten={festhalten}
          onbusy={(b) => (s.laeuft = b)}
          schritte={a.schritte}
        /><ExterneEinreichungen
          eintraege={a.externe_einreichungen ?? []}
          {dateien}
          {darf}
          onnachtragen={extern}
        />
      {:else if tab === "bescheide"}<fieldset disabled={!darf}>
          <Bescheide
            bind:eintraege={a.bescheide}
            {dateien}
            abgegeben={abgegeben(a)}
            {benutzer}
            projektId={projekt.id}
            art={a.art}
            abweichungen={a.abweichungen ?? []}
            alt={s.original?.bescheide.map((v) => v.id) ?? []}
            auflagenAlt={s.original?.bescheide.flatMap((b) =>
              b.auflagen.map((p) => p.id),
            ) ?? []}
            ohneZustellung={s.original?.bescheide
              .filter((v) => !v.zugestellt)
              .map((v) => v.id) ?? []}
            onberichtigen={(g, grund) => berichtigen(a, g, grund)}
          />
          <Gebuehren
            bind:eintraege={() => a.gebuehren ?? [], (w) => (a.gebuehren = w)}
            kommunikation={a.kommunikation}
            {dateien}
            {benutzer}
            alt={s.original?.gebuehren?.map((v) => v.id) ?? []}
          />
        </fieldset>
      {:else if tab === "beziehungen"}<h3>Korrektur- und Ersatzbeziehungen</h3>
        <fieldset disabled={!darf}>
          <Dokumentfeld
            {dateien}
            bind:wert={ersatzVor}
            label="Bisherige Unterlage"
          /><Dokumentfeld
            {dateien}
            bind:wert={ersatzNach}
            label="Ersetzende Unterlage"
          /><label
            >Änderungsgrund<input
              maxlength="4000"
              bind:value={ersatzGrund}
            /></label
          ><button
            type="button"
            class="sekundaer"
            disabled={!ersatzVor || !ersatzNach || !ersatzGrund.trim()}
            onclick={() => {
              a.dokumentbeziehungen.push({
                id: crypto.randomUUID(),
                vorher: ersatzVor!,
                nachher: ersatzNach!,
                grund: ersatzGrund,
                bezug: null,
              });
              ersatzGrund = "";
            }}>Ersatzbeziehung erfassen</button
          >
        </fieldset>
        <ul>
          {#each a.dokumentbeziehungen as b}<li>
              Version {b.vorher.version} → Version {b.nachher.version}: {b.grund}
            </li>{/each}
        </ul>
      {:else if tab === "chronik"}<h3>Verfahrenschronik</h3>
        <fieldset disabled={!darf}>
          <label
            >Ereignisdatum<input
              type="date"
              max={heute()}
              bind:value={notizDatum}
            /></label
          ><label
            >Ereignis / Berichtigung<textarea
              maxlength="4000"
              bind:value={notiz}
            ></textarea></label
          ><Dokumentfeld
            {dateien}
            bind:wert={notizBeleg}
            label="Ereignisbeleg"
          /><label
            >Berichtigt früheren Eintrag<select bind:value={notizBerichtigt}
              ><option value={null}>Keinen</option
              >{#each a.ereignisse as e}<option value={e.id}
                  >{e.datum} · {e.text}</option
                >{/each}</select
            ></label
          ><button
            type="button"
            class="sekundaer"
            disabled={!notiz.trim()}
            onclick={() => {
              a.ereignisse.push({
                id: crypto.randomUUID(),
                datum: notizDatum,
                erfasst_am: new Date().toISOString(),
                erfasst_von: benutzerId,
                text: notiz,
                beleg: notizBeleg,
                berichtigt: notizBerichtigt,
              });
              notiz = "";
            }}>Ereignis erfassen</button
          >
        </fieldset>
        <h4>Zeitleiste</h4>
        <label
          >Art der Änderung<select bind:value={chronikArt}
            ><option value="">Alle</option
            >{#each Object.entries(EREIGNISART) as [wert, titel]}<option
                value={wert}>{titel}</option
              >{/each}</select
          ></label
        >
        <table class="ergebnistabelle">
          <thead
            ><tr
              ><th>Ereignisdatum</th><th>Erfasst</th><th>Art</th><th
                >Ereignis</th
              ></tr
            ></thead
          ><tbody
            >{#each [...a.ereignisse]
              .reverse()
              .filter((e) => !chronikArt || (e.arten ?? []).includes(chronikArt)) as e (e.id)}<tr
                ><td>{e.datum}</td><td
                  >{new Date(e.erfasst_am).toLocaleString("de-DE")}</td
                ><td
                  >{(e.arten ?? []).length
                    ? (e.arten ?? []).map((x) => EREIGNISART[x]).join(", ")
                    : "Notiz / älterer Eintrag"}</td
                ><td
                  >{e.text}{a.ereignisse.some((n) => n.berichtigt === e.id)
                    ? " (berichtigt)"
                    : ""}</td
                ></tr
              >{:else}<tr><td colspan="4">Keine Ereignisse dieser Art.</td></tr
              >{/each}</tbody
          >
        </table>{/if}
    {/key}
  {/snippet}
</Fachliste>

<style>
  .verfahren-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-bottom: 1rem;
  }
  .verfahren-tabs .aktiv {
    background: var(--action-bg);
    color: var(--on-action);
  }
  fieldset {
    border: 0;
    padding: 0.6rem 0;
  }
  nav + button {
    margin-bottom: 1rem;
  }
</style>
