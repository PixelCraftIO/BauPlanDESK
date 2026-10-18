<script lang="ts">
  import type { DocumentVersion } from "../api";
  import type {
    Dokumentbezug,
    Einreichungsweg,
    ExterneEinreichung,
    ExterneEinreichungswunsch,
  } from "../generated/plattform";
  import Dokumentfeld from "./Dokumentfeld.svelte";
  import Dokumentliste from "./Dokumentliste.svelte";
  import { heute, EINREICHUNGSWEG } from "./api";
  let {
    eintraege,
    dateien,
    darf,
    onnachtragen,
  }: {
    eintraege: ExterneEinreichung[];
    dateien: DocumentVersion[];
    darf: boolean;
    /** Speichert über den Core-Befehl; die allgemeine Speicherung nimmt keine neuen Einträge an. */
    onnachtragen: (w: ExterneEinreichungswunsch) => Promise<boolean>;
  } = $props();
  let offen = $state(false),
    datum = $state(heute()),
    weg = $state<Einreichungsweg>("post"),
    beleg = $state<Dokumentbezug | null>(null),
    bemerkung = $state(""),
    dokumente = $state<Dokumentbezug[]>([]),
    berichtigt = $state<string | null>(null),
    vorgang = "",
    fingerprint = "";
  const name = (d: Dokumentbezug | null) =>
    d
      ? (dateien.find(
          (v) => v.document_id === d.dokument_id && v.version === d.version,
        )?.file_name ?? `Version ${d.version}`)
      : "Kein Beleg";
  const berichtigtVon = (id: string) =>
    eintraege.find((n) => n.berichtigt === id);
  const unvollstaendig = $derived(
    !datum ||
      datum > heute() ||
      ((weg === "sonstiges" || !!berichtigt) && !bemerkung.trim()),
  );
  async function nachtragen() {
    const w: ExterneEinreichungswunsch = {
      id: "",
      datum,
      weg,
      beleg: beleg ? structuredClone($state.snapshot(beleg)) : null,
      bemerkung,
      dokumente: structuredClone($state.snapshot(dokumente)),
      berichtigt,
    };
    // Gleiche Angaben, gleiche Kennung: eine Wiederholung bleibt wirkungslos.
    const q = JSON.stringify({ ...w, id: undefined });
    if (q !== fingerprint) {
      fingerprint = q;
      vorgang = crypto.randomUUID();
    }
    w.id = vorgang;
    if (await onnachtragen(w)) {
      offen = false;
      bemerkung = "";
      dokumente = [];
      beleg = null;
      berichtigt = null;
      fingerprint = "";
    }
  }
</script>

<h3>Externe Einreichungen</h3>
<p class="meta">
  Für Anträge, die auf Papier, persönlich oder im Portal ohne Abgabestand der
  App eingereicht wurden. Eine externe Einreichung zählt wie ein versendeter
  Abgabestand als eingereicht, friert aber keine Projektdaten ein und erzeugt
  keine Ausgabedateien. Gespeicherte Einträge bleiben unverändert; ein Fehler
  wird durch eine begründete Berichtigung behoben.
</p>
{#if eintraege.length}<table class="ergebnistabelle">
    <thead
      ><tr
        ><th>Eingereicht am</th><th>Weg</th><th>Beleg</th><th>Unterlagen</th><th
          >Bemerkung</th
        ></tr
      ></thead
    ><tbody
      >{#each eintraege as e (e.id)}<tr data-uebersicht-eintrag={e.id}
          ><td
            >{e.datum}{berichtigtVon(e.id) ? " (berichtigt)" : ""}<br /><span
              class="meta">extern</span
            ></td
          ><td>{EINREICHUNGSWEG[e.weg]}</td><td>{name(e.beleg)}</td><td
            >{e.dokumente.length
              ? e.dokumente.map((d) => name(d)).join(", ")
              : "Nicht in der Ablage"}</td
          ><td>{e.bemerkung}</td></tr
        >{/each}</tbody
    >
  </table>{/if}
{#if darf}{#if !offen}<button
      type="button"
      class="sekundaer"
      onclick={() => {
        offen = true;
        datum = heute();
      }}>Externe Einreichung nachtragen …</button
    >{:else}<fieldset class="hinweisbox">
      <legend>Externe Einreichung nachtragen</legend>
      <div class="raster">
        <label
          >Eingereicht am<span class="pflicht" aria-hidden="true"> *</span><input
            aria-label="Eingereicht am"
            type="date"
            required
            max={heute()}
            bind:value={datum}
          /></label
        ><label
          >Einreichungsweg<select aria-label="Einreichungsweg" bind:value={weg}
            >{#each Object.entries(EINREICHUNGSWEG) as [wert, titel]}<option
                value={wert}>{titel}</option
              >{/each}</select
          ></label
        ><Dokumentfeld
          {dateien}
          bind:wert={beleg}
          label="Beleg (optional)"
        /><label
          >Berichtigt früheren Eintrag<select
            aria-label="Berichtigt früheren Eintrag"
            bind:value={berichtigt}
            ><option value={null}>Keinen</option
            >{#each eintraege.filter((e) => !berichtigtVon(e.id)) as e}<option
                value={e.id}>{e.datum} · {EINREICHUNGSWEG[e.weg]}</option
              >{/each}</select
          ></label
        >
      </div>
      <label
        >Bemerkung{weg === "sonstiges" || berichtigt
          ? " (erforderlich)"
          : ""}<textarea
          aria-label="Bemerkung"
          rows="3"
          maxlength="4000"
          bind:value={bemerkung}
        ></textarea></label
      ><Dokumentliste
        {dateien}
        bind:werte={dokumente}
        label="Eingereichte Unterlagen in der Ablage (optional, Voraussetzung für genehmigte Unterlagen)"
      />
      <div class="knoepfe">
        <button type="button" class="sekundaer" onclick={() => (offen = false)}
          >Abbrechen</button
        ><button type="button" disabled={unvollstaendig} onclick={nachtragen}
          >Einreichung nachtragen</button
        >
      </div>
    </fieldset>{/if}{/if}
