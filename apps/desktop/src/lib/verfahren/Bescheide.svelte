<script lang="ts">
  import { datumDe } from "../api";
  import type {
    Abweichung,
    Ablehnungsgrund,
    Nachbarbekanntgabe,
    Bescheid,
    Bescheidfristen,
    Berichtigungsgegenstand,
    Fristberechnung,
    Verfahrensart,
  } from "../generated/plattform";
  import Berichtigen from "./Berichtigen.svelte";
  import type { DocumentVersion, Benutzer } from "../api";
  import Dokumentfeld from "./Dokumentfeld.svelte";
  import Dokumentliste from "./Dokumentliste.svelte";
  import Fristfeld from "./Fristfeld.svelte";
  import Zustaendigkeit from "./Zustaendigkeit.svelte";
  import {
    heute,
    verfahrenApi,
    NEBENBESTIMMUNG,
    PFLICHT,
    FAELLIGKEIT,
    baugenehmigung,
  } from "./api";
  let {
    eintraege = $bindable(),
    dateien,
    benutzer,
    abgegeben,
    projektId,
    art,
    abweichungen = [],
    ablehnungsgruende = $bindable([]),
    nachbarbekanntgaben = $bindable([]),
    gruendeAlt = [],
    alt = [],
    auflagenAlt = [],
    ohneZustellung = [],
    onberichtigen,
  }: {
    eintraege: Bescheid[];
    dateien: DocumentVersion[];
    /** Dokumentversionen aus versendeten Abgabeständen und externen Einreichungen. */
    abgegeben: DocumentVersion[];
    benutzer: Benutzer[];
    projektId: string;
    art: Verfahrensart;
    /** Beantragte Abweichungen der Akte; der Bescheid entscheidet je Eintrag. */
    abweichungen?: Abweichung[];
    ablehnungsgruende?: Ablehnungsgrund[];
    nachbarbekanntgaben?: Nachbarbekanntgabe[];
    /** Gespeicherte Ablehnungsgründe; ihr Wortlaut bleibt unverändert. */
    gruendeAlt?: string[];
    alt?: string[];
    /** Gespeicherte Nebenbestimmungen; ihr Wortlaut bleibt laut Core unverändert. */
    auflagenAlt?: string[];
    /** Gespeicherte Bescheide, deren Zustelldatum noch einmal nachgetragen werden darf. */
    ohneZustellung?: string[];
    onberichtigen: (g: Berichtigungsgegenstand, grund: string) => void;
  } = $props();
  let neuerGrund = $state<Record<string, string>>({}),
    nachbar = $state<Record<string, string>>({}),
    nachbarDatum = $state<Record<string, string>>({});
  function grund(b: Bescheid) {
    const text = neuerGrund[b.id]?.trim();
    if (!text) return;
    ablehnungsgruende = [
      ...ablehnungsgruende,
      {
        id: crypto.randomUUID(),
        bescheid: b.id,
        text,
        grundlage: "",
        reaktion: "",
      },
    ];
    neuerGrund[b.id] = "";
  }
  function bekanntgabe(b: Bescheid) {
    const name = nachbar[b.id]?.trim();
    const datum = nachbarDatum[b.id];
    if (!name || !datum) return;
    nachbarbekanntgaben = [
      ...nachbarbekanntgaben,
      { id: crypto.randomUUID(), bescheid: b.id, name, datum, beleg: null },
    ];
    nachbar[b.id] = "";
  }
  /** Bescheide, deren gespeichertes Zustelldatum gerade berichtigt wird. */
  let zustellungOffen = $state<Record<string, boolean>>({});
  const ENTSCHEIDUNG: Record<string, string> = {
    genehmigt: "Genehmigt",
    teilweise_genehmigt: "Teilweise genehmigt",
    abgelehnt: "Abgelehnt",
    sonstiges: "Sonstige Entscheidung",
  };
  const genehmigend = (b: Bescheid) =>
    b.entscheidung === "genehmigt" || b.entscheidung === "teilweise_genehmigt";
  // Fristen berechnet der Core aus Zustellung, Entscheidung und Bundesland,
  // auch für noch nicht gespeicherte Angaben.
  let fristen = $state<Bescheidfristen[]>([]);
  $effect(() => {
    const auftrag = { art, bescheide: $state.snapshot(eintraege) as Bescheid[] };
    let aktuell = true;
    verfahrenApi.bescheidfristen(projektId, auftrag).then(
      (r) => aktuell && (fristen = r),
      () => aktuell && (fristen = []),
    );
    return () => (aktuell = false);
  });
  let original = $state<import("../generated/plattform").Dokumentbezug | null>(
    null,
  );
  function neu() {
    if (!original) return;
    eintraege = [
      ...eintraege,
      {
        id: crypto.randomUUID(),
        entscheidung: "genehmigt",
        datum: heute(),
        zugestellt: null,
        original: structuredClone($state.snapshot(original)),
        genehmigte_unterlagen: [],
        auflagen: [],
        ersetzt: null,
      },
    ];
  }
</script>

<h3>Bescheide und genehmigte Unterlagen</h3>
<Dokumentfeld
  {dateien}
  bind:wert={original}
  label="Neuer Bescheid – Original"
/><button type="button" class="sekundaer" disabled={!original} onclick={neu}
  >Bescheid erfassen</button
>
<p class="meta">
  Eine Genehmigung oder Ablehnung wird zusammen mit dem Bearbeitungsstand
  „Entschieden“ gespeichert. Ein Verlängerungsbescheid verlängert die
  Geltungsdauer eines früheren Bescheids.
</p>
{#snippet fristzeile(titel: string, w: Fristberechnung | null)}
  {#if w}<p>
      <strong>{titel}: bis {datumDe(w.ende)}</strong> · {w.grundlage}
    </p>
    <ul class="meta">
      {#each w.hinweise as h}<li>{h}</li>{/each}
    </ul>{/if}
{/snippet}
{#each eintraege as b (b.id)}{@const fest = alt.includes(b.id)}{@const f =
    fristen.find((f) => f.bescheid === b.id)}<details data-uebersicht-eintrag={b.id} open>
    <summary
      >{datumDe(b.datum)} · {ENTSCHEIDUNG[b.entscheidung] ?? b.entscheidung}{b.teilbaugenehmigung
        ? " · Teilbaugenehmigung"
        : ""}{eintraege.some((n) => n.ersetzt === b.id)
        ? " · durch Folgebescheid ersetzt"
        : ""}</summary
    >
    <div class="raster">
      <label
        >Entscheidung<select disabled={fest} bind:value={b.entscheidung}
            ><option value="genehmigt">Genehmigt</option><option
              value="teilweise_genehmigt">Teilweise genehmigt</option
            ><option value="abgelehnt">Abgelehnt</option><option
              value="sonstiges">Sonstige Entscheidung</option
            ></select
          ></label
        ><label
          >Bescheiddatum<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Bescheiddatum"
            type="date"
            disabled={fest}
            required
            max={heute()}
            bind:value={b.datum}
          /></label
        ><label
          >Zustelldatum<input
            type="date"
            max={heute()}
            disabled={fest &&
              !ohneZustellung.includes(b.id) &&
              !zustellungOffen[b.id]}
            value={b.zugestellt ?? ""}
            onchange={(e) => (b.zugestellt = e.currentTarget.value || null)}
          /></label
        >{#if fest && !ohneZustellung.includes(b.id)}<Berichtigen
            bezeichnung="Zustelldatum"
            bind:offen={
              () => !!zustellungOffen[b.id],
              (v) => (zustellungOffen[b.id] = v)
            }
            onvormerken={(g) =>
              onberichtigen({ art: "zustellung", bescheid: b.id }, g)}
          />{/if}<label
          >Ersetzt früheren Bescheid<select
            disabled={fest}
            bind:value={b.ersetzt}
            ><option value={null}>Keinen</option
            >{#each eintraege.filter((n) => n.id !== b.id) as n}<option
                value={n.id}>{datumDe(n.datum)} · {ENTSCHEIDUNG[n.entscheidung] ?? n.entscheidung}</option
              >{/each}</select
          ></label
        >{#if !fest || b.verlaengerung}<label
            >Verlängert die Geltungsdauer von<select
              disabled={fest}
              value={b.verlaengerung?.bescheid ?? ""}
              onchange={(e) =>
                (b.verlaengerung = e.currentTarget.value
                  ? {
                      bescheid: e.currentTarget.value,
                      bis: b.verlaengerung?.bis ?? heute(),
                    }
                  : undefined)}
              ><option value="">Kein Verlängerungsbescheid</option
              >{#each eintraege.filter((n) => n.id !== b.id && genehmigend(n)) as n}<option
                  value={n.id}>{datumDe(n.datum)} · {ENTSCHEIDUNG[n.entscheidung] ?? n.entscheidung}</option
                >{/each}</select
            ></label
          >{#if b.verlaengerung}<label
              >Verlängert bis<span class="pflicht" aria-hidden="true"> *</span
              ><input
                aria-label="Verlängert bis"
                type="date"
                required
                disabled={fest}
                bind:value={b.verlaengerung.bis}
              /></label
            >{/if}{/if}{#if baugenehmigung(art) && art !== "teilbaugenehmigung" && (!fest || b.teilbaugenehmigung)}<label
            ><input
              type="checkbox"
              disabled={fest}
              checked={!!b.teilbaugenehmigung}
              onchange={(e) =>
                (b.teilbaugenehmigung = e.currentTarget.checked || undefined)}
            /> Teilbaugenehmigung (beendet den Bauantrag nicht)</label
          >{/if}
      </div>
    <fieldset disabled={fest}>
      {#if abgegeben.length || b.genehmigte_unterlagen.length}<Dokumentliste
          dateien={fest ? dateien : abgegeben}
          bind:werte={b.genehmigte_unterlagen}
          label="Ausdrücklich genehmigte Unterlagen"
        />{:else}<p class="hinweis">
          Genehmigte Unterlagen stammen aus einem versendeten Abgabestand oder
          einer externen Einreichung. Beides im Register Abgabestände erfassen.
        </p>{/if}
    </fieldset>
    {#if abweichungen.some((e) => !e.entfallen) || b.abweichungsentscheidungen?.length}<fieldset
        disabled={fest}
      >
        <legend>Entscheidung über Abweichungen, Ausnahmen und Befreiungen</legend>
        {#each abweichungen.filter((e) => !e.entfallen || b.abweichungsentscheidungen?.some((d) => d.abweichung === e.id)) as e (e.id)}{@const d =
            b.abweichungsentscheidungen?.find((d) => d.abweichung === e.id)}<div
            class="raster"
          >
            <label
              >{e.gegenstand || e.vorschrift}<select
                value={d ? String(d.zugelassen) : ""}
                onchange={(ev) => {
                  const v = ev.currentTarget.value;
                  const rest = (b.abweichungsentscheidungen ?? []).filter(
                    (x) => x.abweichung !== e.id,
                  );
                  b.abweichungsentscheidungen = v
                    ? [
                        ...rest,
                        {
                          abweichung: e.id,
                          zugelassen: v === "true",
                          begruendung: d?.begruendung ?? "",
                          nebenbestimmung: d?.nebenbestimmung ?? "",
                        },
                      ]
                    : rest;
                }}
                ><option value="">Im Bescheid nicht entschieden</option
                ><option value="true">Zugelassen</option><option value="false"
                  >Nicht zugelassen</option
                ></select
              ></label
            >{#if d}<label
                >Nebenbestimmung zur Zulassung<input
                  bind:value={d.nebenbestimmung}
                  maxlength="16000"
                /></label
              ><label
                >Begründung der Behörde<input
                  bind:value={d.begruendung}
                  maxlength="16000"
                /></label
              >{/if}
          </div>{/each}
        {#if !fest}<p class="meta">
            Nach dem Speichern bleiben die Entscheidungen wie der Bescheid
            unverändert.
          </p>{/if}
      </fieldset>{/if}
    {#if b.entscheidung === "abgelehnt" || b.entscheidung === "teilweise_genehmigt"}<h4>
        Ablehnungsgründe
      </h4>
      {#each ablehnungsgruende.filter((g) => g.bescheid === b.id) as g (g.id)}<div
          class="raster"
        >
          <p><strong>{g.text}</strong></p>
          <label
            >Reaktion im Folgeantrag<textarea
              rows="2"
              maxlength="16000"
              bind:value={g.reaktion}
            ></textarea></label
          >{#if !gruendeAlt.includes(g.id)}<button
              type="button"
              class="link"
              onclick={() =>
                (ablehnungsgruende = ablehnungsgruende.filter(
                  (n) => n.id !== g.id,
                ))}>Verwerfen</button
            >{/if}
        </div>{/each}
      <label
        >Grund (Wortlaut des Bescheids)<input
          maxlength="4000"
          bind:value={neuerGrund[b.id]}
        /></label
      ><button
        type="button"
        class="sekundaer"
        disabled={!neuerGrund[b.id]?.trim()}
        onclick={() => grund(b)}>Ablehnungsgrund erfassen</button
      >{/if}
    {#if genehmigend(b)}<h4>Bekanntgabe an Nachbarn</h4>
      <p class="meta">
        Die Rechtsbehelfsfrist eines Nachbarn läuft erst ab seiner
        Bekanntgabe; danach ist die Genehmigung ihm gegenüber bestandskräftig.
      </p>
      {#each nachbarbekanntgaben.filter((n) => n.bescheid === b.id) as n (n.id)}<div
          class="raster"
        >
          <span>{n.name} · bekanntgegeben am {datumDe(n.datum)}</span><label
            >Angefochten am<input
              type="date"
              max={heute()}
              value={n.angefochten ?? ""}
              onchange={(e) =>
                (n.angefochten = e.currentTarget.value || undefined)}
            /></label
          ><Dokumentfeld {dateien} bind:wert={n.beleg} label="Beleg" />
        </div>{/each}
      <div class="raster">
        <label>Nachbar<input maxlength="500" bind:value={nachbar[b.id]} /></label
        ><label
          >Bekanntgegeben am<input
            type="date"
            max={heute()}
            bind:value={nachbarDatum[b.id]}
          /></label
        >
      </div>
      <button
        type="button"
        class="sekundaer"
        disabled={!nachbar[b.id]?.trim() || !nachbarDatum[b.id]}
        onclick={() => bekanntgabe(b)}>Bekanntgabe erfassen</button
      >{/if}
    {#if f}<section aria-label="Berechnete Fristen">
        <h4>Berechnete Fristen</h4>
        {@render fristzeile("Rechtsbehelfsfrist", f.rechtsbehelfsfrist)}
        {@render fristzeile("Geltungsdauer der Genehmigung", f.geltungsdauer)}
        {@render fristzeile("Frist zur Fertigstellung", f.fertigstellung)}
        {#each f.hinweise as h}<p class="meta">{h}</p>{/each}
      </section>{/if}
    <h4>Nebenbestimmungen / offene Pflichten</h4>
    <button
      type="button"
      class="sekundaer"
      onclick={() =>
        b.auflagen.push({
          id: crypto.randomUUID(),
          art: "auflage",
          titel: "",
          ausloeser: "",
          verantwortlich: null,
          frist: null,
          erledigt: false,
          nachweise: [],
        })}>Nebenbestimmung hinzufügen</button
    >{#each b.auflagen as p (p.id)}<fieldset data-uebersicht-eintrag={p.id}>
        <legend
          >{NEBENBESTIMMUNG[p.art ?? "auflage"]}: {p.titel || "neu"}</legend
        >
        <div class="raster">
          <label
            >Art<select
              value={p.art ?? "auflage"}
              onchange={(e) =>
                (p.art = e.currentTarget.value as typeof p.art)}
              >{#each Object.entries(NEBENBESTIMMUNG) as [wert, titel]}<option
                  value={wert}>{titel}</option
                >{/each}</select
            ></label
          ><label
            >Nebenbestimmung<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Nebenbestimmung"
              required
              maxlength="500"
              readonly={auflagenAlt.includes(p.id)}
              bind:value={p.titel}
            /></label
          ><label
            >Fälligkeit / Auslöser<input
              maxlength="4000"
              readonly={auflagenAlt.includes(p.id)}
              bind:value={p.ausloeser}
            /></label
          ><Zustaendigkeit {benutzer} bind:wert={p.verantwortlich} />{#if PFLICHT.includes(p.art ?? "auflage")}<label
              >Fällig<select
                value={p.faelligkeit ?? ""}
                onchange={(e) =>
                  (p.faelligkeit = (e.currentTarget.value ||
                    undefined) as typeof p.faelligkeit)}
                ><option value="">Nicht zugeordnet</option
                >{#each Object.entries(FAELLIGKEIT) as [w, t]}<option value={w}
                    >{t}</option
                  >{/each}</select
              ></label
            >{/if}
        </div>
        {#if auflagenAlt.includes(p.id)}<p class="meta">
            Wortlaut aus dem Bescheid, nach dem Speichern unveränderlich.
          </p>{/if}
        <Fristfeld {dateien} bind:wert={p.frist} /><Dokumentliste
          {dateien}
          bind:werte={p.nachweise}
          label="Erfüllungsbelege"
        />{#if p.art === "hinweis"}<p class="meta">
            Ein Hinweis begründet keine Pflicht und wird nicht wiedervorgelegt.
          </p>{:else if PFLICHT.includes(p.art ?? "auflage") || p.erledigt}<label
            ><input type="checkbox" bind:checked={p.erledigt} />Erfüllt (Beleg
            erforderlich)</label
          >{:else}<p class="meta">
            Keine erfüllbare Pflicht: Mit Frist erinnert sie vor dem Ablauf und
            endet danach, ohne Frist wird sie nur zur Kenntnis geführt.
          </p>{/if}{#if p.art === "befristung"}<p class="meta">
            Die Frist ist das Ablaufdatum der Befristung.
          </p>{/if}
      </fieldset>{/each}
  </details>{/each}
