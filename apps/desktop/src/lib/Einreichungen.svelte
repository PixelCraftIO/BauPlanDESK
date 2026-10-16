<script lang="ts">
  import { kategorieStandText, kategorieGeaendert } from "./kategorien";
  import PaketPruefstand from "./PaketPruefstand.svelte";
  import { editorAnmelden } from "./editoren";
  import { projektNavigation } from "./projektNavigation.svelte";
  import { onMount, tick } from "svelte";
  import { ask, save } from "@tauri-apps/plugin-dialog";
  import {
    api,
    fehlertext,
    type Einreichung,
    type DokumentVergleich,
    type NeuesPaket,
  } from "./api";
  let {
    projektId,
    darfAblegen,
    onfehler,
  }: {
    projektId: string;
    darfAblegen: boolean;
    onfehler: (m: string) => void;
  } = $props();
  let chronik = $state<Einreichung[]>([]),
    vergleich = $state<DokumentVergleich[]>([]),
    auswahl = $state<string[]>([]);
  let titel = $state("Bauantrag"),
    empfaenger = $state("Bauamt"),
    vergleichEmpfaenger = $state(""),
    kategorieFilter = $state(""),
    laeuft = $state(false),
    hinweis = $state<string | null>(null);
  let bestaetigung = $state<Einreichung | null>(null),
    datum = $state(""),
    notiz = $state("");
  const statusText: Record<DokumentVergleich["status"], string> = {
    neu: "Noch nicht eingereicht",
    bereits_eingereicht: "Diese Version bereits eingereicht",
    gleicher_inhalt: "Neue Version, gleicher Dateiinhalt",
    geaendert: "Seit Einreichung geändert",
    aelter: "Älter als eingereichter Stand",
  };
  const vergleichAktuell = $derived(
    vergleichEmpfaenger.trim().toLowerCase() ===
      empfaenger.trim().toLowerCase(),
  );
  const kategorien = $derived(
    [
      ...new Map(
        vergleich
          .filter((v) => v.kategorie)
          .map((v) => [v.kategorie!.id, v.kategorie!]),
      ).values(),
    ].sort((a, b) =>
      kategorieStandText(a).localeCompare(kategorieStandText(b), "de"),
    ),
  );
  const gefiltert = $derived(
    vergleich.filter(
      (v) =>
        !kategorieFilter ||
        (kategorieFilter === ":ohne"
          ? !v.kategorie
          : v.kategorie?.id === kategorieFilter),
    ),
  );
  export function istBeschaeftigt() {
    return laeuft;
  }
  export function hatAenderungen() {
    return auswahl.length > 0 || titel !== "Bauantrag" || !!bestaetigung;
  }
  export function verwerfen() {
    auswahl = [];
    titel = "Bauantrag";
    bestaetigung = null;
  }
  async function laden() {
    if (laeuft) return;
    laeuft = true;
    const ziel = empfaenger;
    try {
      const [e, v] = await Promise.all([
        api.einreichungen(projektId),
        api.einreichungsVergleich(projektId, ziel),
      ]);
      chronik = e;
      vergleich = v;
      vergleichEmpfaenger = ziel;
      auswahl = auswahl.filter((id) =>
        v.some((row) => row.aktuell.document_id === id),
      );
    } catch (e) {
      onfehler(fehlertext(e));
    } finally {
      laeuft = false;
    }
  }
  const navigation = projektNavigation();
  let letzterSprung = 0;
  $effect(() => {
    const a = navigation?.auftrag;
    if (
      !a ||
      a.nummer === letzterSprung ||
      a.ziel.bereich !== "einreichungen" ||
      !a.ziel.eintrag ||
      !chronik.length
    )
      return;
    letzterSprung = a.nummer;
    void tick().then(() => {
      const e = document.querySelector(
        `[data-projekt-id="${projektId}"] [data-paket="${a.ziel.eintrag}"]`,
      );
      if (e instanceof HTMLDetailsElement) e.open = true;
      e?.scrollIntoView({ block: "center" });
    });
  });
  onMount(laden);
  function umschalten(id: string, checked: boolean) {
    auswahl = checked ? [...auswahl, id] : auswahl.filter((x) => x !== id);
  }
  async function exportieren(e: Einreichung) {
    laeuft = true;
    try {
      const ziel = await save({
        title: "Einreichungspaket exportieren",
        defaultPath: `Einreichung-${e.stand.id}.zip`,
        filters: [{ name: "ZIP", extensions: ["zip"] }],
      });
      if (ziel) {
        await api.einreichungExportieren(e.stand.id, ziel);
        hinweis = "Unverändertes Einreichungspaket exportiert.";
      }
    } catch (err) {
      onfehler(fehlertext(err));
    } finally {
      laeuft = false;
    }
  }
  let pruefvorschau = $state<import("./api").PaketPruefstand | null>(null),
    pruefFingerprint = $state("");
  const auswahlFingerprint = $derived(
    JSON.stringify([
      projektId,
      vergleich
        .filter((v) => auswahl.includes(v.aktuell.document_id))
        .map((v) => [v.aktuell.document_id, v.aktuell.version]),
    ]),
  );
  async function pruefen() {
    laeuft = true;
    try {
      const fingerprint = auswahlFingerprint;
      pruefvorschau = await api.paketPruefstand(
        projektId,
        vergleich
          .filter((v) => auswahl.includes(v.aktuell.document_id))
          .map((v) => ({
            dokument_id: v.aktuell.document_id,
            version: v.aktuell.version,
          })),
      );
      pruefFingerprint = fingerprint;
    } catch (e) {
      onfehler(fehlertext(e));
    } finally {
      laeuft = false;
    }
  }
  let paketVorgang: { fingerprint: string; id: string } | null = null;
  async function erstellen(event: SubmitEvent) {
    event.preventDefault();
    if (!auswahl.length || !vergleichAktuell) return;
    const paket: NeuesPaket = {
      titel,
      empfaenger,
      dokumente: vergleich
        .filter((v) => auswahl.includes(v.aktuell.document_id))
        .map((v) => ({
          dokument_id: v.aktuell.document_id,
          version: v.aktuell.version,
        })),
    };
    laeuft = true;
    try {
      const ziel = await save({
        title: "Einreichungspaket erstellen und exportieren",
        defaultPath: "Einreichung.zip",
        filters: [{ name: "ZIP", extensions: ["zip"] }],
      });
      if (!ziel) return;
      const fingerprint = JSON.stringify([projektId, paket]);
      if (paketVorgang?.fingerprint !== fingerprint)
        paketVorgang = { fingerprint, id: crypto.randomUUID() };
      const e = await api.einreichungErstellen(
        projektId,
        paket,
        paketVorgang.id,
      );
      paketVorgang = null;
      chronik = [e, ...chronik];
      auswahl = [];
      titel = "Bauantrag";
      hinweis =
        "Paketstand gespeichert. Nach tatsächlicher Übermittlung das Einreichungsdatum bestätigen.";
      await api.einreichungExportieren(e.stand.id, ziel);
    } catch (e) {
      onfehler(fehlertext(e));
    } finally {
      laeuft = false;
    }
  }
  function zurBestaetigung(e: Einreichung) {
    bestaetigung = e;
    datum = new Date().toLocaleDateString("sv-SE");
    notiz = "";
  }
  async function bestaetigen(event: SubmitEvent) {
    event.preventDefault();
    if (!bestaetigung) return;
    const e = bestaetigung;
    if (
      !(await ask(
        `Die ${e.stand.dokumente.length} Unterlagen dieses Paketstands wurden am ${datum} an ${e.stand.empfaenger} übermittelt? Die Bestätigung hält diesen Einreichungsstand fest.`,
        { title: "Einreichung bestätigen", kind: "info" },
      ))
    )
      return;
    laeuft = true;
    try {
      await api.einreichungBestaetigen(e.stand.id, e.version, datum, notiz);
      bestaetigung = null;
      hinweis = "Einreichung mit Datum und Dateiversionen bestätigt.";
    } catch (err) {
      onfehler(fehlertext(err));
    } finally {
      laeuft = false;
      await laden();
    }
  }
  editorAnmelden("einreichungen", {
    hatAenderungen: hatAenderungen,
    istBeschaeftigt: istBeschaeftigt,
    speichern: async () => false,
    hinweis:
      "Einreichungsauswahl zuerst als Paket festhalten oder ausdrücklich verwerfen.",
  });
</script>

<h2>Einreichungen und Nachreichungen</h2>
<p class="meta">
  Paketstände halten ausgewählte Dokumentversionen fest. Erst nach Ihrer
  Bestätigung gelten die Unterlagen als eingereicht. Die Anzeige beruht auf
  Ihren Einträgen und ist keine Eingangsbestätigung der Behörde.
</p>
{#if hinweis}<p role="status">{hinweis}</p>{/if}
<form onsubmit={erstellen}>
  <fieldset disabled={laeuft || !darfAblegen}>
    <legend>Neues Einreichungspaket</legend>
    <div class="raster">
      <label
        >Bezeichnung<input bind:value={titel} required maxlength="300" /></label
      ><label
        >Empfänger<input
          bind:value={empfaenger}
          required
          maxlength="300"
        /></label
      >
    </div>
    <div class="knoepfe links">
      <button type="button" class="sekundaer" onclick={laden}
        >Vergleich aktualisieren</button
      ><button
        type="button"
        class="sekundaer"
        disabled={!vergleichAktuell}
        onclick={() =>
          (auswahl = vergleich
            .filter((v) => v.status === "neu" || v.status === "geaendert")
            .map((v) => v.aktuell.document_id))}
        >Neue und geänderte auswählen</button
      ><button
        type="button"
        class="sekundaer"
        disabled={!vergleichAktuell}
        onclick={() => (auswahl = vergleich.map((v) => v.aktuell.document_id))}
        >Alle auswählen</button
      >
    </div>
  </fieldset>
  {#if !vergleichAktuell}<p class="hinweisbox">
      Empfänger geändert. Vergleich aktualisieren, bevor ein Paket erstellt
      wird.
    </p>{/if}
  <div class="knoepfe links">
    <label
      >Kategorie filtern<select
        aria-label="Kategorie filtern"
        bind:value={kategorieFilter}
        ><option value="">Alle Kategorien</option><option value=":ohne"
          >Ohne Kategorie</option
        >{#each kategorien as k}<option value={k.id}
            >{kategorieStandText(k)}</option
          >{/each}</select
      ></label
    >
    <button
      type="button"
      class="sekundaer"
      disabled={laeuft ||
        !darfAblegen ||
        !vergleichAktuell ||
        !gefiltert.length}
      onclick={() =>
        (auswahl = [
          ...new Set([
            ...auswahl,
            ...gefiltert.map((v) => v.aktuell.document_id),
          ]),
        ])}>Gefilterte Unterlagen auswählen</button
    ><span class="meta"
      >{auswahl.length} Unterlage(n) ausgewählt, auch außerhalb des Filters</span
    >
  </div>
  <table>
    <thead
      ><tr
        ><th>Auswahl</th><th>Unterlage</th><th>Kategorie aktuell</th><th
          >Kategorie zuletzt eingereicht</th
        ><th>Aktuelle Version</th><th>Zuletzt eingereicht</th><th>Vergleich</th
        ></tr
      ></thead
    ><tbody>
      {#each gefiltert as v (v.aktuell.document_id)}
        {@const kategorieErfasst =
          v.eingereichte_kategorie_erfasst ?? v.eingereichte_kategorie != null}
        <tr
          ><td
            ><input
              type="checkbox"
              aria-label={`${v.aktuell.file_name} auswählen`}
              checked={auswahl.includes(v.aktuell.document_id)}
              disabled={laeuft || !darfAblegen || !vergleichAktuell}
              onchange={(e) =>
                umschalten(v.aktuell.document_id, e.currentTarget.checked)}
            /></td
          ><td>{v.aktuell.file_name}</td><td
            >{kategorieStandText(
              v.kategorie,
            )}{#if kategorieErfasst && kategorieGeaendert(v.kategorie, v.eingereichte_kategorie)}<span
                class="meta"
              >
                · Kategorie geändert</span
              >{/if}</td
          ><td
            >{!v.zuletzt_eingereicht
              ? "—"
              : kategorieErfasst
                ? kategorieStandText(v.eingereichte_kategorie)
                : "Damals nicht erfasst"}</td
          ><td>v{v.aktuell.version}</td><td
            >{#if v.zuletzt_eingereicht}v{v.zuletzt_eingereicht.version} · {v.eingereicht_am}{:else}—{/if}</td
          ><td>{statusText[v.status]}</td></tr
        >{/each}
    </tbody>
  </table>
  {#if !vergleich.length}<p class="meta">
      Keine Projektdateien für ein Einreichungspaket vorhanden.
    </p>{/if}
  <div class="knoepfe">
    <button
      disabled={laeuft || !darfAblegen || !auswahl.length || !vergleichAktuell}
      type="submit"
      >{laeuft
        ? "Wird verarbeitet …"
        : "Paket festhalten und ZIP exportieren …"}</button
    >
  </div>
</form>
{#if bestaetigung}<form onsubmit={bestaetigen} class="hinweisbox">
    <h3>Einreichung bestätigen: {bestaetigung.stand.titel}</h3>
    <fieldset disabled={laeuft}>
      <div class="raster">
        <label
          >Einreichungsdatum<input
            type="date"
            bind:value={datum}
            required
            max={new Date().toLocaleDateString("sv-SE")}
          /></label
        ><label
          >Übermittlungsweg / Aktenzeichen / Notiz<input
            bind:value={notiz}
            maxlength="4000"
            placeholder="z. B. Bauportal, Aktenzeichen …"
          /></label
        >
      </div>
      <div class="knoepfe">
        <button
          class="sekundaer"
          type="button"
          onclick={() => (bestaetigung = null)}>Abbrechen</button
        ><button type="submit">Einreichung bestätigen</button>
      </div>
    </fieldset>
  </form>{/if}
<div class="knoepfe links">
  <button
    class="sekundaer"
    disabled={laeuft || !auswahl.length || !vergleichAktuell}
    onclick={pruefen}>Auswahl gegen Anforderungen prüfen</button
  >
</div>
{#if pruefvorschau}{#if pruefFingerprint !== auswahlFingerprint}<p
      class="hinweis"
    >
      Auswahl geändert. Die Vorschau bitte erneut prüfen.
    </p>{/if}<PaketPruefstand stand={pruefvorschau} />{/if}
<h3>Chronik der Paketstände</h3>
{#if !chronik.length}<p class="meta">
    Noch keine Einreichungspakete erstellt.
  </p>{/if}
{#each chronik as e (e.stand.id)}<details class="paket" data-paket={e.stand.id}>
    <summary
      >{e.stand.titel} · {e.stand.empfaenger} · {e.einreichung
        ? `Eingereicht am ${e.einreichung.datum}`
        : "Vorbereitet, noch nicht eingereicht"} · {e.stand.dokumente.length} Unterlagen</summary
    >
    <p class="meta">
      Erstellt von {e.stand.erstellt_von} am {new Date(
        e.stand.erstellt_am,
      ).toLocaleString("de-DE")}. Paket-ID: {e.stand.id}
    </p>
    {#if e.einreichung}<p>
        Bestätigt von {e.einreichung.bestaetigt_von}. {e.einreichung.notiz}
      </p>{/if}
    <PaketPruefstand stand={e.stand.pruefstand} />
    <table>
      <thead
        ><tr
          ><th>Unterlage</th><th>Kategorie im Paket</th><th
            >Festgehaltene Version</th
          ><th>SHA-256</th></tr
        ></thead
      ><tbody
        >{#each e.stand.dokumente as d}<tr
            ><td>{d.file_name}</td><td
              >{e.stand.kategorien && d.document_id in e.stand.kategorien
                ? kategorieStandText(e.stand.kategorien[d.document_id])
                : "Damals nicht erfasst"}</td
            ><td>v{d.version}</td><td class="hash">{d.sha256}</td></tr
          >{/each}</tbody
      >
    </table>
    <div class="knoepfe links">
      <button class="sekundaer" disabled={laeuft} onclick={() => exportieren(e)}
        >Dieses ZIP erneut exportieren …</button
      >{#if !e.einreichung}<button
          disabled={laeuft || !darfAblegen}
          onclick={() => zurBestaetigung(e)}
          >Einreichungsdatum erfassen …</button
        >{/if}
    </div>
  </details>{/each}

<style>
  table {
    width: 100%;
    margin: 0.8rem 0;
  }
  th {
    text-align: left;
  }
  .hash {
    font-family: monospace;
    overflow-wrap: anywhere;
    max-width: 24rem;
  }
  .paket {
    border-top: 1px solid var(--rand);
    padding: 0.8rem 0;
  }
  summary {
    cursor: pointer;
  }
</style>
