<script lang="ts">
  import { datumDe } from "../api";
  import { save } from "@tauri-apps/plugin-dialog";
  import { fehlertext, type DocumentVersion } from "../api";
  import type {
    Abgabestand,
    Abgabewunsch,
    Behoerde,
    Dokumentbezug,
    Folgewunsch,
    Nachrichtenstand,
    Planunterlage,
    Verfahrensschritt,
  } from "../generated/plattform";
  import { xbau as xbauApi } from "../xbau/api";
  import Dokumentliste from "./Dokumentliste.svelte";
  import Dokumentfeld from "./Dokumentfeld.svelte";
  import { verfahrenApi, heute, EINREICHUNG } from "./api";
  let {
    eintraege = $bindable(),
    dateien,
    projektId,
    verfahrenId,
    darf,
    behoerde,
    aktenzeichen,
    onfesthalten,
    onbusy,
    gespeichert = [],
    portalZiele = {},
    onportal,
    schritte = [],
  }: {
    eintraege: Abgabestand[];
    dateien: DocumentVersion[];
    projektId: string;
    verfahrenId: string;
    darf: boolean;
    behoerde: Behoerde;
    aktenzeichen: string;
    onfesthalten: (w: Abgabewunsch) => Promise<void>;
    onbusy: (b: boolean) => void;
    gespeichert?: Abgabestand[];
    portalZiele?: Record<string, string>;
    onportal?: (abgabe: string) => void;
    /** Verfahrensschritte der Akte; Anhörungen erlauben eine Stellungnahme (0204). */
    schritte?: Verfahrensschritt[];
  } = $props();
  const anhoerungen = $derived(schritte.filter((s) => s.art === "anhoerung"));
  // Folgenachricht statt Antrag: Art und ihre besonderen Angaben.
  let folgeDatum = $state(heute()),
    folgeBis = $state(""),
    pruefungStandsicherheit = $state(false),
    standsicherheit = $state(false),
    brandschutz = $state(false),
    wiederkehrend = $state(false);
  function folgewunsch(wahl: string): Folgewunsch | undefined {
    const [art, schritt] = wahl.split(":").slice(1);
    switch (art) {
      case "stellungnahme":
        return { art: "stellungnahme", schritt };
      case "abweichungsantrag":
        return { art: "abweichungsantrag" };
      case "verlaengerung":
        return { art: "verlaengerung", bis: folgeBis || null };
      case "baubeginn":
        return {
          art: "baubeginn",
          datum: folgeDatum,
          pruefung_standsicherheit: pruefungStandsicherheit,
        };
      case "nutzungsaufnahme":
        return {
          art: "nutzungsaufnahme",
          datum: folgeDatum,
          standsicherheit_bescheinigt: standsicherheit,
          brandschutz_bescheinigt: brandschutz,
          wiederkehrende_pruefungen: wiederkehrend,
        };
      case "ruecknahme":
        return { art: "ruecknahme" };
    }
    return undefined;
  }
  let titel = $state("Bauantragsunterlagen"),
    dokumente = $state<Dokumentbezug[]>([]),
    xbau = $state(false),
    druck = $state(false),
    anschreiben = $state(""),
    xbauBezug = $state(""),
    nachrichten = $state<Nachrichtenstand[]>([]),
    erfasst = $state<Record<string, boolean>>({}),
    fehler = $state("");
  // Nachreichungen beziehen sich auf den eigenen Antrag oder eine Befundliste.
  const bezugsarten = [
    "baugenehmigung.antrag.0200",
    "vorbescheid.antrag.0210",
    "prozessnachrichten.formellePruefungBefundliste.1140",
  ];
  $effect(() => {
    if (!xbau) return;
    xbauApi
      .nachrichten(projektId)
      .then((n) => {
        nachrichten = n.filter((m) =>
          bezugsarten.includes(m.nachricht.pruefung.kopf.element),
        );
      })
      .catch((e) => (fehler = fehlertext(e)));
  });
  async function journal(b: Abgabestand) {
    onbusy(true);
    fehler = "";
    try {
      await verfahrenApi.xbauErfassen(projektId, verfahrenId, b.id);
      erfasst[b.id] = true;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      onbusy(false);
    }
  }
  let fingerprint = "",
    vorgang = "";
  async function festhalten() {
    const folge =
      xbau && xbauBezug.startsWith("folge:") ? folgewunsch(xbauBezug) : undefined;
    const bezug =
      xbau && xbauBezug && !folge ? xbauBezug : undefined;
    const q = JSON.stringify([
      titel,
      dokumente,
      xbau,
      bezug,
      folge,
      druck,
      anschreiben,
    ]);
    if (q !== fingerprint) {
      fingerprint = q;
      vorgang = crypto.randomUUID();
    }
    await onfesthalten({
      xbau_bezug: bezug,
      xbau_folge: folge,
      id: vorgang,
      titel,
      dokumente: structuredClone($state.snapshot(dokumente)),
      xbau,
      druck,
      anschreiben,
    });
  }
  async function exportieren(
    b: Abgabestand,
    format: string,
    dateiname: string,
  ) {
    onbusy(true);
    fehler = "";
    try {
      const ziel = await save({
        title: "Festgehaltene Ausgabe exportieren",
        defaultPath: dateiname,
      });
      if (ziel)
        await verfahrenApi.export(projektId, verfahrenId, b.id, format, ziel);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      onbusy(false);
    }
  }
  // Eine Aussage zum Stand der Einreichung, vom Core abgeleitet (auch für
  // Projektübersicht und Planliste maßgebend).
  const status = (b: Abgabestand) =>
    b.lage
      ? `${b.lage.text}${b.lage.datum ? ` am ${new Date(b.lage.datum + "T12:00:00").toLocaleDateString("de-DE")}` : ""}`
      : (b.versandstatus ?? "Zusammengestellt · Versand unbestätigt");
  // Planindex und Einreichungsstatus leitet der Core aus der gespeicherten Akte ab.
  let planunterlagen = $state<Planunterlage[]>([]);
  $effect(() => {
    // Ohne gespeicherte Abgabestände gibt es weder Index noch Status.
    if (!JSON.stringify(gespeichert) || !gespeichert.length) {
      planunterlagen = [];
      return;
    }
    verfahrenApi
      .planunterlagen(projektId, verfahrenId)
      .then((u) => (planunterlagen = u))
      .catch((e) => (fehler = fehlertext(e)));
  });
</script>

<h3>Abgabestände und Übermittlungsbelege</h3>
<p class="meta">
  Alle Ausgaben verwenden dieselben festgehaltenen Dokumentversionen. Eine
  Ausgabe bestätigt weder Versand noch Eingang. Fehlerhafte Einträge werden
  durch begründete Folgeeinträge berichtigt.
</p>
{#if fehler}<p role="alert" class="fehlertext">{fehler}</p>{/if}
<fieldset disabled={!darf}>
  <legend>Neuen Stand zusammenstellen</legend>
  <p class="meta">
    Empfänger laut Verfahrensakte: {behoerde.name.trim() || "–"}
    ({behoerde.kennung.trim() || "keine DVDV-Kennung"}) · Aktenzeichen: {aktenzeichen.trim() ||
      "–"}. Druck und XBau übernehmen beides aus der Akte.
  </p><label
    >Abgabebezeichnung<input bind:value={titel} maxlength="500" /></label
  ><Dokumentliste {dateien} bind:werte={dokumente} /><label
    ><input type="checkbox" bind:checked={xbau} />XBau-Ausgabe zusätzlich
    erzeugen (Antragsdaten und Anlagenarten erforderlich)</label
  >{#if xbau}<label
      >XBau-Nachricht<select aria-label="XBau-Nachricht" bind:value={xbauBezug}
        ><option value="">Antrag zum Verfahren</option
        >{#each nachrichten as n}<option value={n.id}
            >Nachreichung zu {n.nachricht.pruefung.kopf.titel} · {n.id.slice(
              0,
              8,
            )}</option
          >{/each}{#each anhoerungen as a (a.id)}<option
            value={`folge:stellungnahme:${a.id}`}
            >Stellungnahme zur Anhörung{a.frist ? ` (Frist ${datumDe(a.frist.datum)})` : ""}</option
          >{/each}<option value="folge:abweichungsantrag"
          >Antrag auf Abweichung (isoliert)</option
        ><option value="folge:verlaengerung"
          >Antrag auf Verlängerung der Geltungsdauer</option
        ><option value="folge:baubeginn">Baubeginnsanzeige</option><option
          value="folge:nutzungsaufnahme">Anzeige der Nutzungsaufnahme</option
        ><option value="folge:ruecknahme">Rücknahme des Antrags</option></select
      ></label
    >{#if xbauBezug === "folge:verlaengerung"}<label
        >Verlängerung bis (optional)<input
          type="date"
          bind:value={folgeBis}
        /></label
      >{:else if xbauBezug === "folge:baubeginn" || xbauBezug === "folge:nutzungsaufnahme"}<label
        >{xbauBezug === "folge:baubeginn"
          ? "Baubeginn am"
          : "Nutzungsaufnahme am"}<input
          type="date"
          bind:value={folgeDatum}
        /></label
      >{/if}{#if xbauBezug === "folge:baubeginn"}<label class="haken"
        ><input type="checkbox" bind:checked={pruefungStandsicherheit} />Prüfung
        des Standsicherheitsnachweises erforderlich (Erklärung des
        Tragwerksplaners)</label
      >{:else if xbauBezug === "folge:nutzungsaufnahme"}<label class="haken"
        ><input type="checkbox" bind:checked={standsicherheit} />Standsicherheit
        bescheinigt</label
      ><label class="haken"
        ><input type="checkbox" bind:checked={brandschutz} />Brandschutz
        bescheinigt</label
      ><label class="haken"
        ><input type="checkbox" bind:checked={wiederkehrend} />Wiederkehrende
        Prüfungen erforderlich</label
      >{:else if xbauBezug === "folge:ruecknahme"}<p class="meta">
        Die Erklärung der Rücknahme steht im Anschreiben.
      </p>{:else if xbauBezug.startsWith("folge:stellungnahme")}<p class="meta">
        Die Stellungnahme besteht aus den Reaktionen zu den Beanstandungen der
        Anhörung (Register Behördliche Schritte); ohne Beanstandungen aus dem
        Anschreiben.
      </p>{/if}{/if}<label
    ><input type="checkbox" bind:checked={druck} />Druck-PDF mit
    Inhaltsverzeichnis zusätzlich erzeugen (druckbare Dateien)</label
  ><label
    >Anschreiben für Druck-PDF<textarea
      rows="4"
      maxlength="16000"
      bind:value={anschreiben}
    ></textarea></label
  ><button
    type="button"
    disabled={!titel.trim() || !dokumente.length}
    onclick={festhalten}>Abgabestand verbindlich festhalten</button
  >
</fieldset>
{#if planunterlagen.length}<section aria-label="Planindex und Einreichungsstatus">
    <h4>Planindex und Einreichungsstatus</h4>
    <p class="meta">
      Abgeleitet aus versendeten Abgabeständen und Bescheiden der gespeicherten
      Akte. Der Revisionsindex der Dokumentdaten und der Bürostatus der
      Unterlagen bleiben davon getrennt.
    </p>
    <table class="ergebnistabelle">
      <thead
        ><tr
          ><th>Unterlage</th><th>Planindex</th><th>Status</th><th
            >Versionen</th
          ></tr
        ></thead
      ><tbody
        >{#each planunterlagen as u (u.dokument_id)}<tr
            ><td
              >{u.name || u.dokument_id}{#each u.hinweise as h}<p
                  class="meta"
                  role="status"
                >
                  {h}
                </p>{/each}</td
            ><td>{u.planindex ?? "–"}</td><td
              >{EINREICHUNG[u.status]}{#if u.offene_version != null}<span
                  class="meta"
                >
                  · Version {u.offene_version} noch nicht eingereicht</span
                >{/if}</td
            ><td
              >{u.staende
                .map(
                  (s) =>
                    `V${s.version}${s.planindex ? ` (${s.planindex})` : ""}: ${EINREICHUNG[s.status]}`,
                )
                .join(" · ")}</td
            ></tr
          >{/each}</tbody
      >
    </table>
  </section>{/if}
{#each eintraege as b (b.id)}<details data-uebersicht-eintrag={b.id} open>
    <summary
      >{b.titel} · {status(b)}</summary
    >
    <p class="meta">
      Festgehalten am {new Date(b.erstellt_am).toLocaleString("de-DE")}
    </p>
    <ul>
      {#each b.dokumente as d}<li>
          {d.name} · Version {d.dokument.version}{#if d.planindex}
            · im Anlagenverzeichnis Index {d.planindex}{/if}
        </li>{/each}
    </ul>
    {#if b.hinweise.length}<aside class="hinweisbox">
        <strong>Prüfhinweise des damaligen Stands</strong>
        <ul>
          {#each b.hinweise as h}<li>{h}</li>{/each}
        </ul>
      </aside>{/if}
    <div class="knoepfe links">
      {#each b.artefakte as f}<button
          type="button"
          class="sekundaer"
          onclick={() => exportieren(b, f.format, f.dateiname)}
          >{f.format.toLocaleUpperCase()} exportieren</button
        >{/each}{#if b.artefakte.some((f) => f.format === "xbau")}<button
          type="button"
          class="sekundaer"
          disabled={!darf || erfasst[b.id]}
          onclick={() => journal(b)}
          >{erfasst[b.id]
            ? "Im XBau-Journal erfasst"
            : "Im XBau-Journal erfassen"}</button
        >{/if}
    </div>
    <table class="ergebnistabelle">
      <thead
        ><tr
          ><th>Ereignisdatum</th><th>Status</th><th>Weg / Referenz</th><th
            >Notiz</th
          ><th>Beleg</th></tr
        ></thead
      ><tbody
        >{#each b.versand as e}<tr
            ><td>{datumDe(e.datum)}</td><td
              >{e.status}{b.versand.some((n) => n.berichtigt === e.id)
                ? " (berichtigt)"
                : ""}</td
            ><td>{e.weg} · {e.referenz}</td><td>{e.notiz}</td><td
              >{e.beleg
                ? dateien.find(
                    (d) =>
                      d.document_id === e.beleg?.dokument_id &&
                      d.version === e.beleg?.version,
                  )?.file_name
                : "Kein Beleg"}</td
            ></tr
          >{/each}</tbody
      >
    </table>
    {#if portalZiele[b.id]}<p class="hinweisbox">
        Diese Abgabe wird über den Portalbereich dokumentiert.
      </p>
      <button class="sekundaer" onclick={() => onportal?.(b.id)}
        >Portalbelege bearbeiten</button
      >{/if}
    <fieldset disabled={!darf || !!portalZiele[b.id]}>
      <legend>Neues Versand-/Eingangsereignis (mit Akte speichern)</legend
      ><button
        type="button"
        class="sekundaer"
        onclick={() =>
          b.versand.push({
            id: crypto.randomUUID(),
            status: "versendet",
            datum: heute(),
            weg: "",
            referenz: "",
            notiz: "",
            beleg: null,
            berichtigt: null,
          })}>Ereignis ergänzen</button
      >
      {#each b.versand.filter((e) => !e.notiz) as e (e.id)}<p class="meta">
          Ereignis benötigt eine Notiz / Begründung.
        </p>{/each}
      <!-- Bereits gespeicherte Einträge werden im Elternobjekt anhand ihrer Kennung geschützt. -->
      {#each b.versand as e (e.id)}<details
          open={!gespeichert.some((b) => b.versand.some((v) => v.id === e.id))}
        >
          <summary
            >{datumDe(e.datum)} · {e.status} · Ereignis bearbeiten / ansehen</summary
          >
          <fieldset
            disabled={gespeichert.some((b) =>
              b.versand.some((v) => v.id === e.id),
            )}
          >
            <div class="raster">
              <label
                >Status<select bind:value={e.status}
                  ><option value="versendet">Versendet</option><option
                    value="eingegangen">Eingang belegt</option
                  ><option value="ungeklaert"
                    >Übergabe / Dateiabgleich ungeklärt</option
                  ><option value="zurueckgewiesen">Zurückgewiesen</option
                  ></select
                ></label
              ><label
                >Ereignisdatum<input
                  type="date"
                  max={heute()}
                  bind:value={e.datum}
                /></label
              ><label>Übermittlungsweg<input bind:value={e.weg} /></label><label
                >Übermittlungsreferenz<input bind:value={e.referenz} /></label
              ><label
                >Notiz / Begründung<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Notiz / Begründung"
                  required
                  maxlength="4000"
                  bind:value={e.notiz}
                /></label
              ><Dokumentfeld
                {dateien}
                bind:wert={e.beleg}
                label="Quittung / Eingangsbeleg"
              /><label
                >Berichtigt früheres Ereignis<select bind:value={e.berichtigt}
                  ><option value={null}>Keines</option
                  >{#each b.versand.slice(0, b.versand.indexOf(e)) as alt}<option
                      value={alt.id}>{datumDe(alt.datum)} · {alt.status}</option
                    >{/each}</select
                ></label
              >
            </div>
          </fieldset>
        </details>{/each}
    </fieldset>
  </details>{/each}
