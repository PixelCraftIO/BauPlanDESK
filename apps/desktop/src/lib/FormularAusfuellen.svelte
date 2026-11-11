<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    api,
    fehlertext,
    type DocumentVersion,
    type Feldwert,
    type FormularAnalyse,
    type PdfVorschauDaten,
    type BibliothekQuelle,
    type FormularBezug,
  } from "./api";

  import { druckPdf } from "./druckPdf";
  import { abonnieren } from "./projektEreignisse";
  import { LivePdfVorschau } from "./LivePdfVorschau";
  import PdfVorschau from "./PdfVorschau.svelte";
  const vorgang = crypto.randomUUID();
  let {
    dokumentId,
    quelle,
    onfertig,
    onabbrechen,
    onstatus,
  }: {
    dokumentId?: string;
    quelle?: BibliothekQuelle;
    onfertig: (ergebnis: DocumentVersion) => void;
    onabbrechen: () => void;
    onstatus?: (dirty: boolean, busy: boolean) => void;
  } = $props();

  let analyse = $state<
    (Omit<FormularAnalyse, "dokument"> & { dokument?: DocumentVersion }) | null
  >(null);
  let werte = $state<Record<string, Feldwert>>({});
  let fehler = $state<string | null>(null);
  let laeuft = $state(false);
  let nurLeere = $state(false);
  let vorschau = $state<Pick<PdfVorschauDaten, "pdf_base64" | "seiten"> | null>(
    null,
  );
  let seite = $state(1);
  let vorschauPdf = $state(""),
    vorschauSoll = $state(""),
    vorschauStand = $state(""),
    gezeichnetStand = $state("");
  let vorschauFehler = $state<string | null>(null),
    pdfBereit = $state(false),
    quellstand = $state(0),
    beendet = false,
    ladeNummer = 0;
  const live = new LivePdfVorschau(
    api.formularLiveVorschau,
    (pdf, stand) => {
      if (pdf === vorschauPdf && pdfBereit) gezeichnetStand = stand;
      else pdfBereit = false;
      vorschauPdf = pdf;
      vorschauStand = stand;
    },
    (e) => (vorschauFehler = fehlertext(e)),
  );
  onDestroy(() => {
    beendet = true;
    ladeNummer++;
    live.beenden();
  });
  async function vorschauDrucken() {
    if (!vorschauPdf || vorschauSoll !== gezeichnetStand || laeuft) return;
    laeuft = true;
    try {
      await druckPdf(vorschauPdf);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
  function geaenderteWerte(): Record<string, Feldwert> {
    const result: Record<string, Feldwert> = {};
    for (const f of analyse?.felder ?? []) {
      const w = werte[f.name];
      if (
        !f.schreibgeschuetzt &&
        f.art !== "sonstiges" &&
        w !== undefined &&
        w !== (f.wert ?? (f.art === "ankreuzfeld" ? false : ""))
      )
        result[f.name] = w;
    }
    return result;
  }
  $effect(() => {
    if (!analyse || !vorschau) return;
    const w = geaenderteWerte();
    const stand = JSON.stringify([quellstand, w]);
    vorschauSoll = stand;
    vorschauFehler = null;
    live.planen(
      vorschau.pdf_base64,
      structuredClone($state.snapshot(w)),
      stand,
    );
  });
  function quelleSetzen(p: Pick<PdfVorschauDaten, "pdf_base64" | "seiten">) {
    vorschau = p;
    vorschauPdf = p.pdf_base64;
    vorschauStand = "";
    pdfBereit = false;
    quellstand++;
  }

  const dirty = $derived(
    !!analyse &&
      analyse.felder.some(
        (f) =>
          !f.schreibgeschuetzt &&
          werte[f.name] !== undefined &&
          werte[f.name] !== (f.wert ?? (f.art === "ankreuzfeld" ? false : "")),
      ),
  );
  $effect(() => onstatus?.(dirty, laeuft));

  function leererBezug(): FormularBezug {
    return {
      grundstueck_id: null,
      bauherr_id: null,
      verfahren_id: null,
      entwurfsverfasser_id: null,
      feldquellen: {},
    };
  }
  let auswahl = $state<FormularBezug>(leererBezug());
  /** Der Teil des Bezugs, der die Projektdaten bestimmt (ohne Feldquellen). */
  function datenbezug(b: FormularBezug) {
    return JSON.stringify([
      b.grundstueck_id ?? null,
      b.bauherr_id ?? null,
      b.verfahren_id ?? null,
      b.entwurfsverfasser_id ?? null,
    ]);
  }
  /** Geladener Datenstand mit den aktuell gewählten Feldquellen. */
  function datenstandZumSpeichern() {
    const d = analyse?.quellen?.datenstand;
    if (!d) return undefined;
    return {
      ...d,
      bezug: { ...d.bezug, feldquellen: { ...(auswahl.feldquellen ?? {}) } },
    };
  }

  // Anzeige der Projektdatenquellen; die Fachlogik (Vorschlag, Abgleich)
  // bleibt im Core, hier werden nur Schlüssel beschriftet und gewählt.
  const GRUPPEN: Record<string, string> = {
    projekt: "Projekt",
    planung: "Planung",
    vorhaben: "Vorhaben",
    datum: "Datum",
    grundstueck: "Baugrundstück",
    kennzahlen: "Kennzahlen",
    wohnflaeche: "Wohnfläche (Berechnung)",
    din277: "DIN 277 (Berechnung)",
    stellplaetze: "Stellplätze (Berechnung)",
    freiflaechen: "Freiflächen (Berechnung)",
    entwaesserung: "Entwässerung (Berechnung)",
    hoehen: "Höhen (Berechnung)",
    flaechen: "Flächen",
    gebaeude: "Gebäude",
    bauherr: "Bauherrschaft",
    eigentuemer: "Grundstückseigentümer",
    erbbauberechtigter: "Erbbauberechtigte",
    bevollmaechtigter: "Bevollmächtigte",
    fachplaner: "Fachplaner",
    entwurfsverfasser: "Entwurfsverfasser/in",
    buero: "Büro",
    verfahren: "Verfahren",
    behoerde: "Behörde",
  };
  const SCHLUESSEL: Record<string, string> = {
    gesamt_m2: "Wohnfläche gesamt (m²)",
    wohneinheiten: "Wohneinheiten",
    bgf_m2: "Brutto-Grundfläche (m²)",
    kgf_m2: "Konstruktions-Grundfläche (m²)",
    bri_m3: "Brutto-Rauminhalt (m³)",
    kfz_bedarf: "Kfz-Stellplatzbedarf",
    fahrrad_bedarf: "Fahrrad-Stellplatzbedarf",
    versiegelt_m2: "Versiegelte Fläche (m²)",
    gruen_m2: "Anrechenbare Grünfläche (m²)",
    abflusswirksame_flaeche_m2: "Abflusswirksame Fläche (m²)",
    rueckhaltevolumen_m3: "Rückhaltevolumen (m³)",
    name: "Name",
    vorname: "Vorname",
    familienname: "Familienname",
    firma: "Firma",
    name_vorname: "Name, Vorname",
    name_buero: "Name, Vorname, Büro",
    anschrift: "Anschrift (eine Zeile)",
    name_anschrift: "Name und Anschrift (zwei Zeilen)",
    name_anschrift_zeile: "Name und Anschrift (eine Zeile)",
    strasse: "Straße",
    hausnummer: "Hausnummer",
    strasse_hausnummer: "Straße, Hausnummer",
    plz: "PLZ",
    ort: "Ort",
    plz_ort: "PLZ, Ort",
    telefon: "Telefon",
    mobil: "Mobil",
    email: "E-Mail",
    berufsbezeichnung: "Berufsbezeichnung",
    mitgliedsnummer: "Mitgliedsnummer",
    kammerland: "Land der Kammer",
    bezeichnung: "Bezeichnung",
    aktenzeichen: "Aktenzeichen",
    heute: "Heute",
  };
  function quellenName(schluessel: string) {
    const [gruppe, ...rest] = schluessel.split(".");
    const feld = rest.join(".");
    return `${GRUPPEN[gruppe] ?? gruppe} · ${SCHLUESSEL[feld] ?? feld}`;
  }
  /** Kontextwerte nach Gruppe; die Gruppe der aktuellen Quelle zuerst. */
  function quellenGruppen(aktuell: string | undefined, mehrzeilig: boolean) {
    const gruppen = new Map<string, { schluessel: string; wert: string }[]>();
    for (const [schluessel, wert] of Object.entries(
      analyse?.quellen?.datenstand.werte ?? {},
    )) {
      if (!mehrzeilig && wert.includes("\n")) continue;
      const g = schluessel.split(".")[0];
      if (!gruppen.has(g)) gruppen.set(g, []);
      gruppen.get(g)!.push({ schluessel, wert });
    }
    const eigene = aktuell?.split(".")[0];
    const alle = [...gruppen.entries()];
    return [
      ...alle.filter(([g]) => g === eigene),
      ...alle.filter(([g]) => g !== eigene),
    ].map(([g, eintraege]) => ({ gruppe: GRUPPEN[g] ?? g, eintraege }));
  }
  let quellenwahlOffen = $state<Record<string, boolean>>({});
  function wirksameQuelle(name: string) {
    return auswahl.feldquellen?.[name] ?? analyse?.feldquellen?.[name];
  }
  function quelleWaehlen(name: string, schluessel: string) {
    const feldquellen = { ...(auswahl.feldquellen ?? {}) };
    if (schluessel) feldquellen[name] = schluessel;
    else delete feldquellen[name];
    auswahl.feldquellen = feldquellen;
    const wert = schluessel
      ? analyse?.quellen?.datenstand.werte[schluessel]
      : undefined;
    if (wert !== undefined) werte[name] = wert;
  }
  async function laden(auffrischen = false) {
    const nummer = ++ladeNummer;
    laeuft = true;
    fehler = null;
    try {
      const bibliothek = quelle
        ? await api.bibliothekFormular(
            quelle,
            auffrischen ? auswahl : undefined,
          )
        : null;
      const a: Omit<FormularAnalyse, "dokument"> & {
        dokument?: DocumentVersion;
      } = bibliothek
        ? bibliothek.schema
        : await api.formularAnalysieren(
            dokumentId!,
            auffrischen ? auswahl : undefined,
          );
      if (beendet || nummer !== ladeNummer) return;
      if (bibliothek) quelleSetzen(bibliothek);
      const start: Record<string, Feldwert> = {};
      for (const f of a.felder) {
        if (f.art === "sonstiges") continue;
        start[f.name] =
          a.vorschlag[f.name] ??
          f.wert ??
          (f.art === "ankreuzfeld" ? false : "");
      }
      if (auffrischen && analyse) {
        // Nur unveränderte Vorschläge ersetzen; manuelle Eingaben bleiben bestehen.
        for (const f of analyse.felder) {
          const alt =
            analyse.vorschlag[f.name] ??
            f.wert ??
            (f.art === "ankreuzfeld" ? false : "");
          if (werte[f.name] !== alt) start[f.name] = werte[f.name];
        }
      }
      werte = start;
      analyse = a;
      if (a.quellen)
        auswahl = {
          ...leererBezug(),
          ...a.quellen.datenstand.bezug,
          feldquellen: { ...(a.quellen.datenstand.bezug.feldquellen ?? {}) },
        };
      if (a.dokument) {
        try {
          const p = await api.pdfVorschau(dokumentId!, a.dokument.version);
          if (!beendet && nummer === ladeNummer) quelleSetzen(p);
        } catch (e) {
          if (!beendet && nummer === ladeNummer) vorschauFehler = fehlertext(e);
        }
      }
    } catch (e) {
      if (!beendet && nummer === ladeNummer) fehler = fehlertext(e);
    } finally {
      if (!beendet && nummer === ladeNummer) laeuft = false;
    }
  }
  onMount(() => {
    void laden();
  });
  // Berechnungen, Gebäude oder Nachweise derselben Akte geändert: Vorschläge
  // können veraltet sein. Nicht automatisch ersetzen, nur auf das Neuladen hinweisen.
  let projektdatenGeaendert = $state(false);
  $effect(() => {
    const id = analyse?.quellen?.datenstand.projekt_id;
    if (!id) return;
    return abonnieren(id, ["fachobjekte", "nachweise"], () => { projektdatenGeaendert = true; });
  });

  // Abweichungen ermittelt der Core; hier nur Anzeige und Übernahme.
  const veraltet = $derived(
    new Map((analyse?.veraltet ?? []).map((v) => [v.feld, v])),
  );
  /** Noch nicht übernommen: Eingabe entspricht weiter dem alten Wert. */
  function offenVeraltet(name: string) {
    const v = veraltet.get(name);
    return v && werte[name] === v.wert ? v : undefined;
  }
  const offeneVeraltete = $derived(
    [...veraltet.values()].filter((v) => werte[v.feld] === v.wert),
  );
  function anzeige(w: Feldwert) {
    return typeof w === "boolean" ? (w ? "angekreuzt" : "nicht angekreuzt") : w;
  }
  function uebernehmen(name: string) {
    const v = veraltet.get(name);
    if (v) werte[name] = v.aktuell;
  }
  function alleUebernehmen() {
    for (const v of offeneVeraltete) werte[v.feld] = v.aktuell;
  }

  const sichtbar = $derived(
    (analyse?.felder ?? []).filter(
      (f) =>
        f.art !== "sonstiges" &&
        (!nurLeere || werte[f.name] === "" || werte[f.name] === false),
    ),
  );

  async function absenden(event: SubmitEvent) {
    event.preventDefault();
    if (!analyse) return;
    if (
      analyse.quellen &&
      datenbezug(auswahl) !== datenbezug(analyse.quellen.datenstand.bezug)
    ) {
      fehler =
        "Bitte die gewählten Projektdaten zuerst laden und die Vorschläge prüfen.";
      return;
    }
    const geaendert = geaenderteWerte();
    laeuft = true;
    try {
      onfertig(
        quelle
          ? await api.bibliothekFormularSpeichern(
              quelle,
              geaendert,
              vorgang,
              datenstandZumSpeichern(),
            )
          : await api.formularImProjektSpeichern(
              dokumentId!,
              analyse.dokument!.version,
              geaendert,
              datenstandZumSpeichern(),
            ),
      );
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
</script>

{#if fehler}<p class="fehlertext">{fehler}</p>{/if}

{#if analyse}
  <p class="meta">
    {analyse.dokument?.file_name ?? quelle?.vorlage.name} · {analyse.felder
      .length} Felder
    {#if analyse.vorlage}· erkannt als <strong>{analyse.vorlage.titel}</strong>
      ({analyse.vorlage.version}){/if}
    · {Object.keys(analyse.vorschlag).length} Werte aus den Projektdaten übernommen
  </p>
  {#if analyse.quellen}
    <fieldset disabled={laeuft}>
      <legend>Quellen für Formularvorschläge</legend>
      <div class="zeile">
        <label
          >Baugrundstück<select bind:value={auswahl.grundstueck_id}
            ><option value={null}>Keine Auswahl</option
            >{#each analyse.quellen.grundstuecke as g}<option value={g.id}
                >{g.bezeichnung}</option
              >{/each}</select
          ></label
        >
        <label
          >Bauherr<select bind:value={auswahl.bauherr_id}
            ><option value={null}>Keine Auswahl</option
            >{#each analyse.quellen.bauherren as b}<option value={b.id}
                >{b.bezeichnung}</option
              >{/each}</select
          ></label
        >
        {#if analyse.quellen.entwurfsverfasser?.length}<label
            >Entwurfsverfasser/in<select bind:value={auswahl.entwurfsverfasser_id}
              ><option value={null}>Laut Bauantrag</option
              >{#each analyse.quellen.entwurfsverfasser as e}<option value={e.id}
                  >{e.bezeichnung}</option
                >{/each}</select
            ></label
          >{/if}
        {#if analyse.quellen.verfahren.length}<label
            >Verfahren<select bind:value={auswahl.verfahren_id}
              ><option value={null}>Keine Auswahl</option
              >{#each analyse.quellen.verfahren as v}<option value={v.id}
                  >{v.bezeichnung}</option
                >{/each}</select
            ></label
          >{/if}
        <button type="button" onclick={() => { projektdatenGeaendert = false; void laden(true); }}
          >Projektdaten neu laden</button
        >
        {#if projektdatenGeaendert}<p class="hinweisbox" role="status">Berechnungen oder Gebäudedaten wurden inzwischen geändert. „Projektdaten neu laden“ übernimmt die neuen Werte; eigene Eingaben bleiben erhalten.</p>{/if}
      </div>
      <p class="meta">
        Manuell geänderte Werte bleiben beim Laden erhalten. Die verwendeten
        Quellen werden mit der PDF-Version gespeichert. Je Textfeld lässt sich
        unter „Quelle“ ein anderer Projektwert wählen, etwa Name und Anschrift
        der Bauherrschaft gesammelt in einem Feld.
      </p>
      {#each analyse.quellen.hinweise as hinweis}<p class="hinweis">
          {hinweis}
        </p>{/each}
    </fieldset>
  {/if}
  {#if offeneVeraltete.length}
    <div class="hinweisbox" role="status">
      {offeneVeraltete.length === 1
        ? "Ein Feld weicht"
        : `${offeneVeraltete.length} Felder weichen`} von den aktuellen Projektdaten
      ab und ist als veraltet markiert.
      <button type="button" class="sekundaer" onclick={alleUebernehmen}
        >Alle aktuellen Werte übernehmen</button
      >
    </div>
  {/if}
  <label class="zeile"
    ><input type="checkbox" bind:checked={nurLeere} /> Nur leere Felder anzeigen</label
  >

  <div class="pdf-arbeitsplatz">
    <div class="pdf-vorschau-spalte">
      <button
        type="button"
        class="sekundaer"
        disabled={laeuft ||
          !pdfBereit ||
          !!vorschauFehler ||
          vorschauSoll !== gezeichnetStand}
        onclick={vorschauDrucken}>Aktuelle Eingaben drucken …</button
      >
      {#if vorschau && vorschauPdf}<label
          >Seite<select aria-label="Seite" bind:value={seite}
            >{#each vorschau.seiten as p}<option value={p.nummer}
                >{p.nummer}</option
              >{/each}</select
          ></label
        ><PdfVorschau
          pdfBase64={vorschauPdf}
          {seite}
          onbereit={(pdf) => {
            if (pdf === vorschauPdf) {
              pdfBereit = true;
              gezeichnetStand = vorschauStand;
            }
          }}
          onfehler={(m) => {
            pdfBereit = false;
            vorschauFehler = m;
          }}
        />
        <p class="meta">
          {vorschauFehler
            ? "Die letzte Vorschau konnte nicht aktualisiert werden."
            : vorschauSoll === gezeichnetStand
              ? "Vorschau mit Ihren aktuellen Eingaben."
              : "Vorschau wird aktualisiert …"}
        </p>{:else}<p class="meta">
          PDF-Vorschau wird geladen. Die Eingabemaske kann bereits verwendet
          werden.
        </p>{/if}
      {#if vorschauFehler}<p class="hinweisbox" role="status">
          {vorschauFehler} Ihre Eingaben bleiben erhalten.
        </p>{/if}
    </div>
    <form class="spalte pdf-eingabemaske" onsubmit={absenden}>
      <div class="formularfelder">
        {#each sichtbar as f (f.name)}
          {@const alt = offenVeraltet(f.name)}
          <label
            class:vorschlag={f.name in analyse.vorschlag}
            class:veraltet={!!alt}
            title={f.name}
          >
            <span
              >{f.beschriftung ?? f.name}{f.name in analyse.vorschlag
                ? " · aus Projektdaten"
                : ""}{alt ? " · veraltet" : ""}</span
            >
            {#if f.art === "ankreuzfeld"}
              <input
                type="checkbox"
                bind:checked={werte[f.name] as boolean}
                disabled={f.schreibgeschuetzt}
              />
            {:else if f.art === "optionsfeld" || f.art === "auswahl"}
              <select bind:value={werte[f.name]} disabled={f.schreibgeschuetzt}>
                <option value="">–</option>
                {#each f.optionen as o}<option value={o}>{o}</option>{/each}
              </select>
            {:else if f.mehrzeilig}
              <textarea
                rows="4"
                bind:value={werte[f.name] as string}
                maxlength={f.max_laenge ?? undefined}
                disabled={f.schreibgeschuetzt}
              ></textarea>
            {:else}
              <input
                bind:value={werte[f.name]}
                maxlength={f.max_laenge ?? undefined}
                disabled={f.schreibgeschuetzt}
              />
            {/if}
            {#if f.art === "text" && !f.schreibgeschuetzt && analyse.quellen}
              {@const quelleFeld = wirksameQuelle(f.name)}
              <span class="quellenwahl">
                {#if quellenwahlOffen[f.name]}
                  <select
                    aria-label={`Projektdatenquelle für ${f.beschriftung ?? f.name}`}
                    value={quelleFeld ?? ""}
                    onchange={(e) => quelleWaehlen(f.name, e.currentTarget.value)}
                  >
                    <option value="">– manuell –</option>
                    {#each quellenGruppen(quelleFeld, !!f.mehrzeilig) as g (g.gruppe)}
                      <optgroup label={g.gruppe}>
                        {#each g.eintraege as q (q.schluessel)}
                          <option value={q.schluessel}
                            >{quellenName(q.schluessel)} · {q.wert
                              .replace(/\n/g, " / ")
                              .slice(0, 60)}</option
                          >
                        {/each}
                      </optgroup>
                    {/each}
                  </select>
                {:else}
                  <button
                    type="button"
                    class="sekundaer"
                    onclick={() => (quellenwahlOffen[f.name] = true)}
                    >{quelleFeld
                      ? `Quelle: ${quellenName(quelleFeld)}`
                      : "Quelle: Projektwert wählen …"}</button
                  >
                {/if}
              </span>
            {/if}
            {#if alt}<span class="hinweis"
                >Aktuell in den Projektdaten: „{anzeige(alt.aktuell)}“
                <button
                  type="button"
                  class="sekundaer"
                  disabled={f.schreibgeschuetzt}
                  onclick={() => uebernehmen(f.name)}>Übernehmen</button
                ></span
              >{/if}
          </label>
        {:else}
          <p class="meta">Alle Felder sind ausgefüllt.</p>
        {/each}
      </div>
      <p class="meta">
        {quelle
          ? "Die Bibliotheksvorlage bleibt unverändert. Im Projekt wird nur die ausgefüllte Datei gespeichert."
          : "Die Angaben werden als neue Version derselben Projektdatei gespeichert."}
      </p>
      <div class="knoepfe">
        <button type="button" class="sekundaer" onclick={onabbrechen}
          >Abbrechen</button
        >
        <button
          type="submit"
          disabled={laeuft ||
            !analyse.felder.some(
              (f) => f.art !== "sonstiges" && !f.schreibgeschuetzt,
            )}>{laeuft ? "Speichere …" : "Im Projekt speichern"}</button
        >
      </div>
    </form>
  </div>
{:else if !fehler}
  <p class="meta">Formular wird gelesen …</p>
{/if}

{#if fehler && !analyse}<button class="sekundaer" onclick={onabbrechen}
    >Schließen</button
  >{/if}

<style>
  .quellenwahl {
    display: block;
    font-size: 0.85em;
    margin-top: 0.2rem;
  }
  .quellenwahl select {
    max-width: 100%;
  }
</style>
