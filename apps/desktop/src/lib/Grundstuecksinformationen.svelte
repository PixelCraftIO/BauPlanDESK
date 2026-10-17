<script lang="ts">
  import { tick, untrack } from "svelte";
  import { api, fehlertext, type Project, type DocumentVersion } from "./api";
  import type {
    FlurstuecksInfo,
    FlurstuecksRolle,
  } from "./generated/plattform";
  import { grundstuecksinformationen } from "./grundstuecksdaten";
  import { editorAnmelden } from "./editoren";
  let {
    projekt,
    darf,
    aktiv,
    onprojekt,
  }: {
    projekt: Project;
    darf: boolean;
    aktiv: boolean;
    onprojekt: (p: Project) => void;
  } = $props();
  const initial = () =>
    grundstuecksinformationen($state.snapshot(projekt.planungsrecht));
  let eingabe = $state(untrack(initial)),
    gespeichert = $state(untrack(initial));
  let basis = $state(untrack(() => structuredClone($state.snapshot(projekt))));
  let laeuft = $state(false),
    fehler = $state<string | null>(null),
    meldung = $state<string | null>(null);
  let dokumente = $state<DocumentVersion[]>([]);
  const geaendert = $derived(
    JSON.stringify(eingabe) !== JSON.stringify(gespeichert),
  );
  $effect(() => {
    const p = projekt;
    untrack(() => {
      if (!geaendert) {
        eingabe = grundstuecksinformationen($state.snapshot(p.planungsrecht));
        gespeichert = structuredClone($state.snapshot(eingabe));
        basis = structuredClone($state.snapshot(p));
      }
    });
  });
  let dokumentenFehler = $state<string | null>(null);
  $effect(() => {
    if (!aktiv) return;
    const id = projekt.id;
    let abgebaut = false;
    dokumentenFehler = null;
    void api
      .dokumente(id)
      .then((d) => {
        if (!abgebaut) dokumente = d;
      })
      .catch((e) => {
        if (!abgebaut) dokumentenFehler = fehlertext(e);
      });
    return () => {
      abgebaut = true;
    };
  });
  function hinzufuegen() {
    eingabe.flurstuecke.push({
      id: crypto.randomUUID(),
      gemarkung: "",
      flur: "",
      nummer: "",
      flaeche_m2: 0,
      grundbuchbezirk: "",
      grundbuchblatt: "",
      laufende_nummer: null,
      grundbuchauszug: null,
      eigentuemer: "",
      bemerkung: "",
    });
  }
  const ROLLEN: { code: FlurstuecksRolle; name: string }[] = [
    { code: "vorhaben", name: "Vorhabengrundstück" },
    { code: "nachbar", name: "Nachbarflurstück" },
    { code: "sonstiges", name: "Sonstiges (Zuwegung, Baulast …)" },
  ];
  function rolleSetzen(f: FlurstuecksInfo, wert: string) {
    // Vorhaben ist der Normalfall und wird wie im Core nicht gespeichert.
    if (wert === "vorhaben") delete f.rolle;
    else f.rolle = wert as FlurstuecksRolle;
  }
  function auszug(f: FlurstuecksInfo, wert: string) {
    const [id, v] = wert.split(":");
    f.grundbuchauszug = wert ? { dokument_id: id, version: Number(v) } : null;
  }
  async function speichern() {
    laeuft = true;
    fehler = null;
    meldung = null;
    try {
      const p = await api.projektBereichSpeichern(
        structuredClone($state.snapshot(basis)),
        "grundstuecksinformationen",
        {
          id: projekt.id,
          expected_version: basis.version,
          name: projekt.name,
          bundesland: projekt.bundesland,
          grundstuecksflaeche_m2: projekt.grundstuecksflaeche_m2,
          planungsrecht: {
            ...structuredClone($state.snapshot(projekt.planungsrecht)),
            grundstuecksinformationen: structuredClone(
              $state.snapshot(eingabe),
            ),
          },
        },
      );
      eingabe = grundstuecksinformationen(p.planungsrecht);
      gespeichert = structuredClone($state.snapshot(eingabe));
      basis = structuredClone(p);
      onprojekt(p);
      await tick();
      meldung =
        "Grundstücksinformationen gespeichert. Die Baugrundstücksermittlung verwendet diese Daten als Grundlage.";
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laeuft = false;
    }
    return !geaendert;
  }
  editorAnmelden("flurstuecke", {
    hatAenderungen: () => geaendert,
    istBeschaeftigt: () => laeuft,
    speichern: async () => !laeuft && (!geaendert || (await speichern())),
  });
</script>

<p class="meta">
  Katasterangaben und tatsächliche Flurstücksflächen. Grundbuchangaben und
  Auszüge können später ergänzt werden. Zuordnungen, Abzüge und Zuschläge werden
  separat unter Berechnungen → Baugrundstück erfasst. Nur Flurstücke des
  Vorhabengrundstücks bilden Baugrundstücke und stehen im Bauantrag; Nachbar-
  und sonstige Flurstücke bleiben als Angaben erhalten.
</p>
{#if gespeichert.flurstuecke.length && projekt.grundstuecksflaeche_m2 != null}<p
    class="meta"
  >
    Grundstücksfläche des Vorhabens (gespeicherter Stand): {projekt.grundstuecksflaeche_m2.toLocaleString(
      "de-DE",
    )} m²
  </p>{/if}
{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
{#if meldung}<p role="status">{meldung}</p>{/if}
{#if dokumentenFehler}<p class="hinweisbox" role="status">
    Projektdateien für die Auszugswahl konnten nicht geladen werden: {dokumentenFehler}
  </p>{/if}
<fieldset disabled={!darf || laeuft}>
  <legend>Flurstücke</legend>
  <div class="tabellen-scroll">
    <table class="ergebnistabelle eingabetabelle">
      <thead
        ><tr
          ><th>Gemarkung</th><th>Flur</th><th>Flurstück</th><th
            >Flurstücksfläche m²</th
          ><th>Bezug</th><th></th></tr
        ></thead
      ><tbody>
        {#each eingabe.flurstuecke as f (f.id)}<tr
            ><td
              ><input
                aria-label="Gemarkung"
                bind:value={f.gemarkung}
                required
                maxlength="500"
              /></td
            ><td
              ><input
                aria-label="Flur"
                bind:value={f.flur}
                maxlength="500"
              /></td
            ><td
              ><input
                aria-label="Flurstücknummer"
                bind:value={f.nummer}
                required
                maxlength="500"
              /></td
            ><td
              ><input
                aria-label="Flurstücksfläche"
                type="number"
                min="0.01"
                step="any"
                bind:value={f.flaeche_m2}
                required
              /></td
            ><td
              ><select
                aria-label="Bezug zum Vorhaben"
                value={f.rolle ?? "vorhaben"}
                onchange={(e) => rolleSetzen(f, e.currentTarget.value)}
              >
                {#each ROLLEN as r (r.code)}<option value={r.code}
                    >{r.name}</option
                  >{/each}
              </select></td
            ><td
              ><button
                class="link"
                onclick={() =>
                  (eingabe.flurstuecke = eingabe.flurstuecke.filter(
                    (x) => x.id !== f.id,
                  ))}>Entfernen</button
              ></td
            ></tr
          >{:else}<tr><td colspan="6">Noch keine Flurstücke erfasst.</td></tr
          >{/each}
      </tbody>
    </table>
  </div>
  <button class="sekundaer" onclick={hinzufuegen}>Flurstück hinzufügen</button>
  {#each eingabe.flurstuecke as f (f.id)}<details>
      <summary
        >{f.gemarkung || "Gemarkung"} · {f.nummer || "Flurstück"} – Grundbuch und
        weitere Angaben (optional)</summary
      >
      <div class="raster">
        <label
          >Grundbuchbezirk<input
            bind:value={f.grundbuchbezirk}
            maxlength="500"
          /></label
        >
        <label
          >Grundbuchblatt<input
            bind:value={f.grundbuchblatt}
            maxlength="500"
          /></label
        >
        <label
          >Laufende Nummer<input
            type="number"
            min="1"
            step="1"
            bind:value={f.laufende_nummer}
            placeholder="Noch nicht bekannt"
          /></label
        >
        <label
          >Eigentümer<input bind:value={f.eigentuemer} maxlength="500" /></label
        >
        <label
          >Grundbuchauszug<select
            aria-label="Grundbuchauszug"
            value={f.grundbuchauszug
              ? `${f.grundbuchauszug.dokument_id}:${f.grundbuchauszug.version}`
              : ""}
            onchange={(e) => auszug(f, e.currentTarget.value)}
          >
            <option value="">Kein Auszug zugeordnet</option>
            {#if f.grundbuchauszug && !dokumente.some((d) => d.document_id === f.grundbuchauszug?.dokument_id && d.version === f.grundbuchauszug.version)}<option
                value={`${f.grundbuchauszug.dokument_id}:${f.grundbuchauszug.version}`}
                >Zugeordnete Dokumentversion {f.grundbuchauszug.version}</option
              >{/if}
            {#each dokumente as d}<option
                value={`${d.document_id}:${d.version}`}
                >{d.file_name} · Version {d.version}</option
              >{/each}
          </select></label
        >
        <label
          >Bemerkung<textarea bind:value={f.bemerkung} maxlength="4000"
          ></textarea></label
        >
      </div>
      <p class="meta">
        Ein Grundbuchauszug kann unter Projektdateien hinzugefügt und hier
        zugeordnet werden. Eine laufende Nummer gilt zusammen mit Bezirk und
        Grundbuchblatt.
      </p>
    </details>{/each}
</fieldset>
<div class="knoepfe links">
  <button disabled={!darf || laeuft || !geaendert} onclick={speichern}
    >Grundstücksinformationen speichern</button
  >{#if geaendert}<span class="meta">Ungespeicherte Änderungen</span>{/if}
</div>

<style>
  .eingabetabelle input {
    min-width: 4rem;
    width: 100%;
  }
  .eingabetabelle td {
    vertical-align: top;
  }
  details {
    margin-top: 1rem;
  }
  summary {
    cursor: pointer;
  }
  .raster {
    margin-top: 1rem;
  }
</style>
