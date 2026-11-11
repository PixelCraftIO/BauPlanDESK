<script lang="ts">
  import { datumDe } from "../api";
  import { save } from "@tauri-apps/plugin-dialog";
  import type {
    Schreibenart,
    Schreibenergebnis,
    Verfahrensakte,
  } from "../generated/plattform";
  import { fehlertext } from "../api";
  import { heute, verfahrenApi, SCHRITTART } from "./api";
  let {
    akte,
    projektId,
    verfahrenId,
    darf,
    ondokument,
    vorgabe = null,
  }: {
    /** Vorgewählte Art und Nachforderung (aus Lage oder Nachforderung). */
    vorgabe?: { art: string; nachforderung?: string } | null;
    akte: Verfahrensakte;
    projektId: string;
    verfahrenId: string | null;
    darf: boolean;
    /** Nach der Ablage einer Projektunterlage (Dokumentliste aktualisieren). */
    ondokument?: () => void;
  } = $props();
  let art = $state<Schreibenart["art"]>("nachforderungsantwort"),
    nachforderung = $state(""),
    schritt = $state(""),
    fristbezug = $state(""),
    bis = $state(""),
    grund = $state(""),
    datum = $state(heute()),
    zusatz = $state(""),
    ablegen = $state(true),
    ergebnis = $state<Schreibenergebnis | null>(null),
    fehler = $state(""),
    laeuft = $state(false);
  $effect(() => {
    if (!vorgabe) return;
    art = vorgabe.art as Schreibenart["art"];
    if (vorgabe.nachforderung) nachforderung = vorgabe.nachforderung;
  });
  const anhoerungen = $derived(
    akte.schritte.filter((s) => s.art === "anhoerung"),
  );
  // Fristen, die verlängert werden können: Forderungspunkte und Schritte.
  const fristen = $derived([
    ...akte.nachforderungen.flatMap((n) =>
      n.punkte
        .filter((p) => p.frist)
        .map((p) => ({ id: p.id, titel: `${p.titel} (bis ${datumDe(p.frist!.datum)})` })),
    ),
    ...akte.schritte
      .filter((s) => s.frist)
      .map((s) => ({
        id: s.id,
        titel: `${SCHRITTART[s.art]} (bis ${datumDe(s.frist!.datum)})`,
      })),
  ]);
  function auftrag(): Schreibenart | null {
    switch (art) {
      case "nachforderungsantwort":
        return nachforderung ? { art, nachforderung } : null;
      case "fristverlaengerung":
        return fristbezug && bis ? { art, bezug: fristbezug, bis, grund } : null;
      case "stellungnahme":
        return schritt ? { art, schritt } : null;
      default:
        return { art } as Schreibenart;
    }
  }
  async function erzeugen() {
    const a = auftrag();
    if (!a || !verfahrenId) return;
    laeuft = true;
    fehler = "";
    try {
      const ziel = await save({
        title: "Schreiben speichern",
        defaultPath: `${datum} Schreiben.pdf`,
        filters: [{ name: "PDF", extensions: ["pdf"] }],
      });
      ergebnis = await verfahrenApi.schreiben(
        projektId,
        verfahrenId,
        { art: a, datum, zusatz, ablegen: ablegen && darf },
        ziel,
      );
      if (ergebnis.dokument) ondokument?.();
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
  }
</script>

<h3>Schreiben an die Behörde</h3>
<p class="meta">
  Das Schreiben entsteht aus den Einträgen der Akte im Briefbogen des Büros.
  Abgelegt wird es als Projektunterlage (Schriftverkehr Behörde) und kann so
  einem Abgabestand beigefügt werden. Der Versand wird im Register
  Abgabestände oder Kommunikation erfasst.
</p>
<div class="raster">
  <label
    >Art<select bind:value={art}
      ><option value="nachforderungsantwort">Antwort auf Nachforderung</option
      ><option value="fristverlaengerung">Antrag auf Fristverlängerung</option
      ><option value="stellungnahme">Stellungnahme zur Anhörung</option><option
        value="abweichungsantrag"
        >Antrag auf Abweichung, Ausnahme, Befreiung</option
      ><option value="sachstandsanfrage">Sachstandsanfrage</option><option
        value="ruecknahme">Rücknahme des Antrags</option
      ></select
    ></label
  >
  {#if art === "nachforderungsantwort"}<label
      >Nachforderung<select bind:value={nachforderung}
        ><option value="">Auswählen</option
        >{#each akte.nachforderungen as n (n.id)}<option value={n.id}
            >{akte.kommunikation.find((k) => k.id === n.kommunikation)
              ?.datum ?? ""} · {n.punkte.length} Punkte</option
          >{/each}</select
      ></label
    >{:else if art === "fristverlaengerung"}<label
      >Frist<select bind:value={fristbezug}
        ><option value="">Auswählen</option
        >{#each fristen as f (f.id)}<option value={f.id}>{f.titel}</option
          >{/each}</select
      ></label
    ><label>Verlängern bis<input type="date" bind:value={bis} /></label><label
      >Begründung<input maxlength="4000" bind:value={grund} /></label
    >{:else if art === "stellungnahme"}<label
      >Anhörung<select bind:value={schritt}
        ><option value="">Auswählen</option
        >{#each anhoerungen as s (s.id)}<option value={s.id}
            >{s.frist ? `Frist ${datumDe(s.frist.datum)}` : "ohne Frist"} · {s.gruende
              ?.length ?? 0} Beanstandungen</option
          >{/each}</select
      ></label
    >{/if}
  <label>Datum<input type="date" bind:value={datum} /></label>
</div>
<label
  >Zusätzlicher Text<textarea rows="3" maxlength="16000" bind:value={zusatz}
  ></textarea></label
>
{#if darf}<label class="haken"
    ><input type="checkbox" bind:checked={ablegen} />Als Projektunterlage
    ablegen</label
  >{/if}
<button
  type="button"
  disabled={laeuft || !verfahrenId || !auftrag()}
  onclick={erzeugen}>Schreiben erzeugen …</button
>
{#if fehler}<p class="fehler" role="alert">{fehler}</p>{/if}
{#if ergebnis}<section aria-label="Erzeugtes Schreiben">
    <p>
      {ergebnis.brief.betreff}{ergebnis.dokument
        ? " · als Projektunterlage abgelegt"
        : ""}
    </p>
    {#each ergebnis.hinweise as h}<p class="hinweis">{h}</p>{/each}
  </section>{/if}
