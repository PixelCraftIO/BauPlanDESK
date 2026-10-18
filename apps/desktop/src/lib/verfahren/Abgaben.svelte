<script lang="ts">
  import { save } from "@tauri-apps/plugin-dialog";
  import { fehlertext, type DocumentVersion } from "../api";
  import type {
    Abgabestand,
    Abgabewunsch,
    Behoerde,
    Dokumentbezug,
    Nachrichtenstand,
  } from "../generated/plattform";
  import { xbau as xbauApi } from "../xbau/api";
  import Dokumentliste from "./Dokumentliste.svelte";
  import Dokumentfeld from "./Dokumentfeld.svelte";
  import { verfahrenApi, heute } from "./api";
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
  } = $props();
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
    const bezug = xbau && xbauBezug ? xbauBezug : undefined;
    const q = JSON.stringify([titel, dokumente, xbau, bezug, druck, anschreiben]);
    if (q !== fingerprint) {
      fingerprint = q;
      vorgang = crypto.randomUUID();
    }
    await onfesthalten({
      xbau_bezug: bezug,
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
  const status = (b: Abgabestand) => b.versandstatus ?? "Vorbereitet";
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
          >{/each}</select
      ></label
    >{/if}<label
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
{#each eintraege as b (b.id)}<details data-uebersicht-eintrag={b.id} open>
    <summary
      >{b.titel} · {status(b)} · {new Date(b.erstellt_am).toLocaleString(
        "de-DE",
      )}</summary
    >
    <ul>
      {#each b.dokumente as d}<li>
          {d.name} · Version {d.dokument.version}
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
            ><td>{e.datum}</td><td
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
            >{e.datum} · {e.status} · Ereignis bearbeiten / ansehen</summary
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
                      value={alt.id}>{alt.datum} · {alt.status}</option
                    >{/each}</select
                ></label
              >
            </div>
          </fieldset>
        </details>{/each}
    </fieldset>
  </details>{/each}
