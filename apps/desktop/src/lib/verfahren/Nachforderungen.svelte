<script lang="ts">
  import { onMount } from "svelte";
  import { datumDe, fehlertext } from "../api";
  import { plattformApi, type Datensatz } from "../plattformApi";
  import { projektNavigation } from "../projektNavigation.svelte";
  import type {
    Aufgabe,
    Forderungspunkt,
    Nachforderung,
    Verfahrensakte,
  } from "../generated/plattform";
  import type { DocumentVersion, Benutzer } from "../api";
  import Dokumentliste from "./Dokumentliste.svelte";
  import Fristfeld from "./Fristfeld.svelte";
  import Zustaendigkeit from "./Zustaendigkeit.svelte";
  let {
    eintraege = $bindable(),
    akte,
    dateien,
    benutzer,
    alt = [],
    projektId,
    darf = true,
    onnachreichen,
    onschreiben,
    onversand,
  }: {
    projektId: string;
    darf?: boolean;
    /** Hält die Antwortunterlagen der offenen Punkte als Nachreichung fest. */
    onnachreichen?: (nachforderung: string) => void;
    /** Öffnet das Antwortschreiben zu dieser Nachforderung. */
    onschreiben?: (nachforderung: string) => void;
    /** Öffnet den Abgabestand, um den Versand zu erfassen. */
    onversand?: (abgabe: string) => void;
    eintraege: Nachforderung[];
    akte: Verfahrensakte;
    dateien: DocumentVersion[];
    benutzer: Benutzer[];
    /** Gespeicherte Forderungspunkte; ihr Wortlaut bleibt laut Core unverändert. */
    alt?: string[];
  } = $props();
  const navigation = projektNavigation();
  let original = $state("");
  const STATUS: Record<string, string> = {
    offen: "Offen",
    in_arbeit: "In Arbeit",
    vorbereitet: "Antwort vorbereitet",
    eingereicht: "Antwort eingereicht",
    bestaetigt: "Behördlich bestätigt",
    entfallen: "Entfallen",
  };
  const unbeantwortet = (p: Forderungspunkt) =>
    ["offen", "in_arbeit", "vorbereitet"].includes(p.status);
  /** Aufgaben des Projekts; ein Punkt führt die an ihn gebundenen mit. */
  let aufgaben = $state<Datensatz<Aufgabe>[]>([]),
    aufgabenFehler = $state<string | null>(null);
  async function aufgabenLaden() {
    try {
      aufgaben = await plattformApi.liste<Aufgabe>(projektId, "aufgaben");
    } catch (e) {
      aufgabenFehler = fehlertext(e);
    }
  }
  onMount(aufgabenLaden);
  async function aufgabeAnlegen(p: Forderungspunkt) {
    aufgabenFehler = null;
    try {
      await plattformApi.speichern<Aufgabe>(
        projektId,
        "aufgaben",
        crypto.randomUUID(),
        0,
        {
          titel: p.titel.slice(0, 500),
          beschreibung: "",
          faellig: p.frist?.datum ?? null,
          verantwortlich: p.verantwortlich,
          status: "offen",
          bezug: p.id,
        },
      );
      await aufgabenLaden();
    } catch (e) {
      aufgabenFehler = fehlertext(e);
    }
  }
  /**
   * Stand der geführten Antwort: Unterlagen zugeordnet, Nachreichung
   * festgehalten, Versand erfasst. Maßgebend sind die noch nicht
   * beantworteten Punkte.
   */
  function antwort(n: Nachforderung) {
    const offen = n.punkte.filter(unbeantwortet);
    const mitUnterlagen = offen.filter((p) => p.dokumente.length);
    const staende = akte.abgaben.filter((b) =>
      n.punkte.some((p) => p.abgaben.includes(b.id)),
    );
    const letzter = staende.at(-1) ?? null;
    const festgehalten =
      mitUnterlagen.length > 0 &&
      mitUnterlagen.every((p) =>
        p.abgaben.some((id) => akte.abgaben.some((b) => b.id === id)),
      );
    const versendet = !!letzter?.lage?.eingereicht;
    const eingereichtFuehren = offen.filter((p) =>
      p.abgaben.some(
        (id) => akte.abgaben.find((b) => b.id === id)?.lage?.eingereicht,
      ),
    );
    return {
      offen,
      mitUnterlagen,
      letzter,
      festgehalten,
      versendet,
      eingereichtFuehren,
    };
  }
  /** Abgabestände, die als Antwort in Frage kommen: ab dem Eingang der Forderung. */
  function antwortstaende(n: Nachforderung, p: Forderungspunkt) {
    const ab = akte.kommunikation.find((c) => c.id === n.kommunikation)?.datum;
    return akte.abgaben.filter(
      (b) =>
        p.abgaben.includes(b.id) || !ab || b.erstellt_am.slice(0, 10) >= ab,
    );
  }
  function neu() {
    if (!original) return;
    eintraege = [
      ...eintraege,
      { id: crypto.randomUUID(), kommunikation: original, punkte: [] },
    ];
  }
  function punkt(n: Nachforderung) {
    n.punkte.push({
      id: crypto.randomUUID(),
      titel: "",
      grundlage: "",
      verantwortlich: null,
      frist: null,
      status: "offen",
      antwort: "",
      dokumente: [],
      abgaben: [],
      bestaetigung: null,
    });
  }
</script>

<h3>Nachforderungen und Antworten</h3>
{#if aufgabenFehler}<p class="fehler" role="alert">{aufgabenFehler}</p>{/if}
<label
  >Eingegangenes Schreiben / Gesprächsnotiz<select bind:value={original}
    ><option value="">Kommunikation auswählen</option
    >{#each akte.kommunikation.filter((c) => c.eingehend) as c}<option
        value={c.id}>{datumDe(c.datum)} · {c.betreff}</option
      >{/each}</select
  ></label
><button type="button" class="sekundaer" disabled={!original} onclick={neu}
  >Nachforderung anlegen</button
>
{#each eintraege as n (n.id)}{@const kommunikation = akte.kommunikation.find(
    (c) => c.id === n.kommunikation,
  )}
  {@const stand = antwort(n)}
  <section>
    <h4>
      {akte.kommunikation.find((c) => c.id === n.kommunikation)?.betreff ??
        "Nachforderung"}
    </h4>
    {#if kommunikation}<p class="meta">
        Originaleingang: {datumDe(kommunikation.datum)} · {kommunikation.betreff}
      </p>
      {#if kommunikation.original}<button
          type="button"
          class="link"
          onclick={() => {
            const d = kommunikation.original;
            if (d)
              navigation?.springen({
                bereich: "dokumente",
                objekt: d.dokument_id,
                register: "historie",
                eintrag: String(d.version),
              });
          }}>Originalschreiben in genau dieser Version ansehen →</button
        >{/if}{/if}
    <p>
      {n.punkte.filter((p) => ["bestaetigt", "entfallen"].includes(p.status))
        .length} von {n.punkte.length} Punkten abgeschlossen
    </p>
    {#if stand.offen.length}<div class="antwortweg" aria-label="Antwort einreichen">
        <strong>Antwort einreichen</strong>
        <ol>
          <li>
            <span class="nr">{stand.mitUnterlagen.length === stand.offen.length ? "✓" : "1."}</span>
            Antwortunterlagen bei den Punkten zuordnen
            <span class="meta"
              >{stand.mitUnterlagen.length} von {stand.offen.length} offenen Punkten
              haben Unterlagen</span
            >
          </li>
          <li>
            <span class="nr">{stand.festgehalten ? "✓" : "2."}</span>
            Unterlagen als Abgabestand zusammenstellen
            <button
              type="button"
              class="sekundaer"
              disabled={!darf || !stand.mitUnterlagen.length}
              onclick={() => onnachreichen?.(n.id)}
              >{stand.festgehalten
                ? "Erneut festhalten"
                : "Nachreichung festhalten"}</button
            >
          </li>
          <li>
            <span class="nr">3.</span>
            Antwortschreiben an die Behörde
            <button
              type="button"
              class="sekundaer"
              onclick={() => onschreiben?.(n.id)}>Anschreiben erstellen</button
            >
          </li>
          <li>
            <span class="nr">{stand.versendet ? "✓" : "4."}</span>
            Versand und Eingang belegen
            {#if stand.letzter}<button
                type="button"
                class="sekundaer"
                onclick={() => onversand?.(stand.letzter!.id)}
                >{stand.versendet
                  ? "Abgabestand ansehen"
                  : "Versand erfassen"}</button
              >{:else}<span class="meta"
                >nach dem Festhalten der Nachreichung</span
              >{/if}
          </li>
        </ol>
        {#if stand.eingereichtFuehren.length}<p>
            Die Nachreichung ist versendet.
            <button
              type="button"
              onclick={() => {
                for (const p of stand.eingereichtFuehren)
                  p.status = "eingereicht";
              }}
              >{stand.eingereichtFuehren.length === 1
                ? "Punkt als eingereicht führen"
                : `${stand.eingereichtFuehren.length} Punkte als eingereicht führen`}</button
            >
          </p>{/if}
      </div>{/if}
    <button type="button" class="sekundaer" onclick={() => punkt(n)}
      >Forderungspunkt hinzufügen</button
    >
    {#each n.punkte as p (p.id)}{@const staende = antwortstaende(n, p)}{@const gebunden =
        aufgaben.filter((a) => a.daten.bezug === p.id)}<details
        data-uebersicht-eintrag={p.id}
        open={p.status !== "bestaetigt"}
      >
        <summary
          >{p.titel || "Neuer Punkt"} · {STATUS[p.status] ?? p.status}{p.frist
            ? ` · Frist ${datumDe(p.frist.datum)}`
            : ""}</summary
        >
        {#if alt.includes(p.id)}<p class="meta">
            Wortlaut und Grundlage aus dem behördlichen Schreiben bleiben nach
            dem Speichern unverändert. Ein abweichender Wortlaut ist ein neuer
            Punkt; der bisherige wird begründet als entfallen geführt.
          </p>{/if}
        <div class="raster">
          <label
            >Forderung<span class="pflicht" aria-hidden="true"> *</span><input
              aria-label="Forderung"
              required
              maxlength="500"
              readonly={alt.includes(p.id)}
              bind:value={p.titel}
            /></label
          ><label
            >Grundlage / Fundstelle<input
              maxlength="4000"
              readonly={alt.includes(p.id)}
              bind:value={p.grundlage}
            /></label
          ><Zustaendigkeit {benutzer} bind:wert={p.verantwortlich} /><label
            >Bearbeitungsstand<select bind:value={p.status}
              ><option value="offen">Offen</option><option value="in_arbeit"
                >In Arbeit</option
              ><option value="vorbereitet">Antwort vorbereitet</option><option
                value="eingereicht">Antwort eingereicht</option
              ><option value="bestaetigt">Behördlich bestätigt</option><option
                value="entfallen">Entfallen (Begründung erforderlich)</option
              ></select
            ></label
          ><label
            >Betrifft Unterlage der Akte<select
              value={p.anforderung ?? ""}
              onchange={(e) => {
                if (e.currentTarget.value)
                  p.anforderung = e.currentTarget.value;
                else delete p.anforderung;
              }}
              ><option value="">Keine bestimmte Unterlage</option
              >{#each akte.anforderungen as f (f.id)}<option value={f.id}
                  >{f.titel}</option
                >{/each}</select
            ></label
          >
        </div>
        {#if p.anforderung}<p class="meta">
            Die Antwortunterlagen dieses Punktes werden beim Speichern auch bei
            der Unterlage eingetragen. Solange der Punkt offen ist, erscheint
            der Vorgang in der Übersicht nur einmal.
          </p>{/if}
        <Fristfeld {dateien} bind:wert={p.frist} /><label
          >Antwort / Begründung<textarea
            rows="4"
            maxlength="16000"
            bind:value={p.antwort}
          ></textarea></label
        ><Dokumentliste
          {dateien}
          bind:werte={p.dokumente}
          label="Antwortunterlagen"
        />
        <fieldset>
          <legend>Antwort-Abgabestände</legend>
          {#if !staende.length}<p class="meta">
              Noch kein Abgabestand seit Eingang der Forderung. „Nachreichung
              festhalten“ legt ihn aus den Antwortunterlagen an.
            </p>{/if}{#each staende as b (b.id)}<label
              ><input
                type="checkbox"
                checked={p.abgaben.includes(b.id)}
                onchange={(e) =>
                  (p.abgaben = e.currentTarget.checked
                    ? [...p.abgaben, b.id]
                    : p.abgaben.filter((id) => id !== b.id))}
              />{b.titel}</label
            >{/each}
        </fieldset>
        <div class="punktaufgaben">
          <strong>Aufgaben zu diesem Punkt</strong>
          {#if gebunden.length}<ul>
              {#each gebunden as a (a.objekt)}<li>
                  {a.daten.titel} · {a.daten.status === "erledigt"
                    ? "Erledigt"
                    : a.daten.faellig
                      ? `bis ${datumDe(a.daten.faellig)}`
                      : "ohne Frist"}
                  <button
                    type="button"
                    class="link"
                    onclick={() =>
                      navigation?.springen({
                        bereich: "aufgaben",
                        objekt: a.objekt,
                        register: null,
                        eintrag: null,
                      })}>Öffnen</button
                  >
                </li>{/each}
            </ul>{:else}<span class="meta">keine</span>{/if}
          {#if alt.includes(p.id) && unbeantwortet(p)}<button
              type="button"
              class="sekundaer"
              disabled={!darf}
              onclick={() => aufgabeAnlegen(p)}>Aufgabe anlegen</button
            >{/if}
          <p class="meta">
            Gebundene Aufgaben zählen nicht als eigener offener Punkt und
            werden erledigt, sobald der Punkt beantwortet ist.
          </p>
        </div>
        <label
          >Behördliche Bestätigung<select bind:value={p.bestaetigung}
            ><option value={null}>Noch keine Rückmeldung</option
            >{#each akte.kommunikation.filter((c) => c.eingehend) as c}<option
                value={c.id}>{datumDe(c.datum)} · {c.betreff}</option
              >{/each}</select
          ></label
        >
      </details>{/each}
  </section>{/each}

<style>
  .antwortweg {
    border: 1px solid var(--border-innen);
    padding: 0.5rem 0.75rem;
    margin: 0.5rem 0;
  }
  .antwortweg ol {
    list-style: none;
    margin: 0.35rem 0 0;
    padding: 0;
  }
  .antwortweg .nr {
    display: inline-block;
    min-width: 1.5rem;
    font-weight: 600;
  }
  .antwortweg li {
    padding: 0.2rem 0;
  }
  .antwortweg li button {
    margin-left: 0.5rem;
  }
  .punktaufgaben {
    margin: 0.5rem 0;
  }
  .punktaufgaben ul {
    margin: 0.25rem 0;
    padding-left: 1.25rem;
  }
</style>
