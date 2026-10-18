<script lang="ts">
  import { kategorieStandText } from "./kategorien";
  import PaketPruefstand from "./PaketPruefstand.svelte";
  import { projektNavigation } from "./projektNavigation.svelte";
  import { onMount, tick } from "svelte";
  import { save } from "@tauri-apps/plugin-dialog";
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
  export function istBeschaeftigt() {
    return laeuft;
  }
  // Pakete sind nur noch lesbar; es gibt keinen Entwurf.
  export function hatAenderungen() {
    return false;
  }
  export function verwerfen() {}
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
</script>

<h2>Frühere Einreichungspakete</h2>
<p class="meta">
  Neue Einreichungen und Nachreichungen entstehen als Abgabestand der
  Verfahrensakte, mit Behörde, Aktenzeichen und eingefrorenen Unterlagen. Die
  Pakete hier bleiben unverändert lesbar und exportierbar. Sie werden nicht
  mehr bestätigt: Eine außerhalb der App erfolgte Einreichung wird in der
  Verfahrensakte unter „Externe Einreichung nachtragen“ erfasst.
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
<h3>Chronik der Paketstände</h3>
{#if !chronik.length}<p class="meta">
    In diesem Projekt gibt es keine früheren Einreichungspakete.
  </p>{/if}
{#each chronik as e (e.stand.id)}<details class="paket" data-paket={e.stand.id}>
    <summary
      >{e.stand.titel} · {e.stand.empfaenger} · {e.einreichung
        ? `Eingereicht am ${e.einreichung.datum}`
        : "Ohne bestätigte Einreichung"} · {e.stand.dokumente.length} Unterlagen</summary
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
      >
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
