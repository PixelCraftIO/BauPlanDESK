<script lang="ts">
  import { heuteDeutsch } from "./kalender";
  import { kategorieStandText } from "./kategorien";
  import PaketPruefstand from "./PaketPruefstand.svelte";
  import { editorAnmelden } from "./editoren";
  import { projektNavigation } from "./projektNavigation.svelte";
  import { onMount, tick } from "svelte";
  import { ask, save } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext, type Einreichung } from "./api";
  let {
    projektId,
    darfAblegen,
    onfehler,
  }: {
    projektId: string;
    darfAblegen: boolean;
    onfehler: (m: string) => void;
  } = $props();
  let chronik = $state<Einreichung[]>([]);
  let laeuft = $state(false),
    hinweis = $state<string | null>(null);
  let bestaetigung = $state<Einreichung | null>(null),
    datum = $state(""),
    notiz = $state("");
  export function istBeschaeftigt() {
    return laeuft;
  }
  export function hatAenderungen() {
    return !!bestaetigung;
  }
  export function verwerfen() {
    bestaetigung = null;
  }
  async function laden() {
    if (laeuft) return;
    laeuft = true;
    try {
      chronik = await api.einreichungen(projektId);
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
  function zurBestaetigung(e: Einreichung) {
    bestaetigung = e;
    datum = heuteDeutsch();
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
    hinweis: "Einreichungsbestätigung zuerst abschließen oder abbrechen.",
  });
</script>

<h2>Frühere Einreichungspakete</h2>
<p class="meta">
  Neue Einreichungen und Nachreichungen entstehen als Abgabestand der
  Verfahrensakte, mit Behörde, Aktenzeichen und eingefrorenen Unterlagen. Die
  Pakete hier bleiben unverändert lesbar und exportierbar; ihr
  Einreichungsdatum lässt sich weiterhin bestätigen.
</p>
{#if navigation}<div class="knoepfe links">
    <button
      class="sekundaer"
      onclick={() =>
        navigation.springen({
          bereich: "verfahren",
          objekt: null,
          register: null,
          eintrag: null,
        })}>Zur Abgabe in der Verfahrensakte …</button
    >
  </div>{/if}
{#if hinweis}<p role="status">{hinweis}</p>{/if}
{#if bestaetigung}<form onsubmit={bestaetigen} class="hinweisbox">
    <h3>Einreichung bestätigen: {bestaetigung.stand.titel}</h3>
    <fieldset disabled={laeuft}>
      <div class="raster">
        <label
          >Einreichungsdatum<span class="pflicht" aria-hidden="true"> *</span><input aria-label="Einreichungsdatum"
            type="date"
            bind:value={datum}
            required
            max={heuteDeutsch()}
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
<h3>Chronik der Paketstände</h3>
{#if !chronik.length}<p class="meta">
    In diesem Projekt gibt es keine früheren Einreichungspakete.
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
