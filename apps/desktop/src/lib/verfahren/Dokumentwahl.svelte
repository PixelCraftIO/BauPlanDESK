<script lang="ts">
  import type { DocumentVersion } from "../api";
  import type { Dokumentbezug } from "../generated/plattform";
  import { projektNavigation } from "../projektNavigation.svelte";
  import { onMount } from "svelte";
  let {
    dateien,
    wert = $bindable(null),
    label = "Dokumentversion",
    required = false,
  }: {
    dateien: DocumentVersion[];
    wert?: Dokumentbezug | null;
    label?: string;
    required?: boolean;
  } = $props();
  const navigation = projektNavigation();
  let suche = $state(""),
    historisch = $state(false),
    typ = $state(""),
    kategorie = $state(""),
    fehler = $state("");
  async function kategorienLaden() {
    try {
      await navigation?.kategorienLaden();
      fehler = "";
    } catch {
      fehler =
        "Kategorien nicht verfügbar. Auswahl über Dateiname und genaue Version bleibt möglich.";
    }
  }
  onMount(kategorienLaden);
  const kategorien = $derived(
    [
      ...new Set(
        dateien.map(
          (d) => navigation?.kategorien[d.document_id] ?? "Ohne Kategorie",
        ),
      ),
    ].sort(),
  );
  const letzte = $derived.by(() => {
    const m = new Map<string, number>();
    for (const d of dateien)
      m.set(d.document_id, Math.max(m.get(d.document_id) ?? 0, d.version));
    return m;
  });
  const ausgewaehlt = $derived(
    dateien.find(
      (d) => d.document_id === wert?.dokument_id && d.version === wert?.version,
    ),
  );
  const gefiltert = $derived(
    dateien
      .filter(
        (d) =>
          (historisch || d.version === letzte.get(d.document_id)) &&
          d.file_name
            .toLocaleLowerCase("de")
            .includes(suche.toLocaleLowerCase("de")) &&
          (!typ || d.file_name.toLowerCase().endsWith(typ)) &&
          (!kategorie ||
            (navigation?.kategorien[d.document_id] ?? "Ohne Kategorie") ===
              kategorie),
      )
      .sort(
        (a, b) =>
          a.file_name.localeCompare(b.file_name, "de") || b.version - a.version,
      ),
  );
  const key = (d: Dokumentbezug) => `${d.dokument_id}:${d.version}`;
  const docKey = (d: DocumentVersion) => `${d.document_id}:${d.version}`;
</script>

<div class="dokumentwahl">
  <details>
    <summary>Dokumentauswahl durchsuchen und filtern</summary>
    <label
      >Dateiname suchen<input
        type="search"
        aria-label={`${label}: Dateiname suchen`}
        bind:value={suche}
      /></label
    >
    <label
      >Kategorie<select
        aria-label={`${label}: Kategorie`}
        bind:value={kategorie}
        ><option value="">Alle Kategorien</option>{#each kategorien as k}<option
            >{k}</option
          >{/each}</select
      ></label
    ><button type="button" class="link" onclick={kategorienLaden}
      >Kategorien aktualisieren</button
    >{#if fehler}<p role="status">{fehler}</p>{/if}
    <label
      >Dateityp<select aria-label={`${label}: Dateityp`} bind:value={typ}
        ><option value="">Alle Dateitypen</option><option value=".pdf"
          >PDF</option
        ><option value=".dwg">DWG</option><option value=".eml">E-Mail</option
        ></select
      ></label
    >
    <label
      ><input type="checkbox" bind:checked={historisch} />Auch historische
      Versionen anbieten</label
    >
  </details>
  <label
    >{label}<select
      {required}
      value={wert ? key(wert) : ""}
      onchange={(e) => {
        const d = dateien.find((d) => docKey(d) === e.currentTarget.value);
        wert = d ? { dokument_id: d.document_id, version: d.version } : null;
      }}
    >
      <option value="">Kein Dokument zugeordnet</option>
      {#if wert && !gefiltert.some((d) => docKey(d) === key(wert!))}<option
          value={key(wert)}
          >{ausgewaehlt?.file_name ?? "Dokument nicht auflösbar"} · Version {wert.version}
          (zugeordnet)</option
        >{/if}
      {#each gefiltert as d (docKey(d))}<option value={docKey(d)}
          >{d.file_name} · Version {d.version}</option
        >{/each}
    </select></label
  >
  {#if wert}<div class="meta">
      Zugeordneter Stand: {ausgewaehlt?.file_name ?? "Dokument nicht auflösbar"}
      · Version {wert.version}
      {#if !ausgewaehlt}<span role="status"
          >Metadaten fehlen. Projektdateien prüfen; die Referenz wurde
          beibehalten.</span
        >{/if}
      {#if navigation}<button
          type="button"
          class="link"
          onclick={() =>
            navigation.springen({
              bereich: "dokumente",
              objekt: wert!.dokument_id,
              register: "historie",
              eintrag: String(wert!.version),
            })}>Genau diese Version ansehen</button
        >{/if}
    </div>{/if}
</div>

<style>
  .dokumentwahl {
    min-width: 0;
  }
  details {
    margin-bottom: 0.5rem;
  }
  select,
  input[type="search"] {
    width: 100%;
  }
  details > label {
    display: block;
    margin: 0.5rem 0;
  }
</style>
