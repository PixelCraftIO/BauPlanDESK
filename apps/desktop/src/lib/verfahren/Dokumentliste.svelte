<script lang="ts">
  import type { DocumentVersion } from "../api";
  import type { Dokumentbezug } from "../generated/plattform";
  import { projektNavigation } from "../projektNavigation.svelte";
  let {
    dateien,
    werte = $bindable(),
    label = "Zugeordnete Unterlagen",
  }: {
    dateien: DocumentVersion[];
    werte: Dokumentbezug[];
    label?: string;
  } = $props();
  const navigation = projektNavigation();
  // Die Auswahl klappt erst auf Wunsch auf; mehrere Dateien lassen sich in
  // einem Schritt zuordnen.
  let offen = $state(false),
    suche = $state(""),
    kategorie = $state(""),
    historisch = $state(false),
    gewaehlt = $state<string[]>([]);
  const schluessel = (id: string, version: number) => `${id}:${version}`;
  const kategorieVon = (d: DocumentVersion) =>
    navigation?.kategorien[d.document_id] ?? "Ohne Kategorie";
  const letzte = $derived.by(() => {
    const m = new Map<string, number>();
    for (const d of dateien)
      m.set(d.document_id, Math.max(m.get(d.document_id) ?? 0, d.version));
    return m;
  });
  const kategorien = $derived([...new Set(dateien.map(kategorieVon))].sort());
  const angebot = $derived(
    dateien
      .filter(
        (d) =>
          (historisch || d.version === letzte.get(d.document_id)) &&
          !werte.some(
            (w) => w.dokument_id === d.document_id && w.version === d.version,
          ) &&
          d.file_name
            .toLocaleLowerCase("de")
            .includes(suche.toLocaleLowerCase("de")) &&
          (!kategorie || kategorieVon(d) === kategorie),
      )
      .sort(
        (a, b) =>
          a.file_name.localeCompare(b.file_name, "de") || b.version - a.version,
      ),
  );
  async function oeffnen() {
    offen = !offen;
    gewaehlt = [];
    if (offen) {
      try {
        await navigation?.kategorienLaden();
      } catch {
        /* Auswahl über den Dateinamen bleibt möglich */
      }
    }
  }
  function zuordnen() {
    const neu = dateien
      .filter((d) => gewaehlt.includes(schluessel(d.document_id, d.version)))
      .map((d) => ({ dokument_id: d.document_id, version: d.version }));
    werte = [...werte, ...neu];
    gewaehlt = [];
    offen = false;
  }
  const name = (w: Dokumentbezug) =>
    `${dateien.find((d) => d.document_id === w.dokument_id && d.version === w.version)?.file_name ?? "Dokument nicht auflösbar"} · Version ${w.version}`;
</script>

<div class="dokumentliste">
  <div class="dl-kopf">
    <strong>{label}</strong>
    <span class="meta"
      >{werte.length
        ? `${werte.length} zugeordnet`
        : "noch nichts zugeordnet"}</span
    >
    <button
      type="button"
      class="sekundaer"
      aria-expanded={offen}
      onclick={oeffnen}
      >{offen ? "Auswahl schließen" : "Unterlagen hinzufügen …"}</button
    >
  </div>
  {#if werte.length}<ul class="dl-werte">
      {#each werte as w, i (schluessel(w.dokument_id, w.version))}<li>
          <span>{name(w)}</span>
          {#if navigation}<button
              type="button"
              class="link"
              onclick={() =>
                navigation.springen({
                  bereich: "dokumente",
                  objekt: w.dokument_id,
                  register: "historie",
                  eintrag: String(w.version),
                })}>Ansehen</button
            >{/if}
          <button
            type="button"
            class="link"
            onclick={() => (werte = werte.filter((_, n) => n !== i))}
            >Zuordnung entfernen</button
          >
        </li>{/each}
    </ul>{/if}
  {#if offen}<div class="dl-auswahl">
      <div class="dl-filter">
        <label
          >Dateiname suchen<input
            type="search"
            aria-label={`${label}: Dateiname suchen`}
            bind:value={suche}
          /></label
        ><label
          >Kategorie<select
            aria-label={`${label}: Kategorie`}
            bind:value={kategorie}
            ><option value="">Alle Kategorien</option
            >{#each kategorien as k}<option>{k}</option>{/each}</select
          ></label
        ><label class="dl-check"
          ><input type="checkbox" bind:checked={historisch} />Auch ältere
          Versionen</label
        >
      </div>
      <ul class="dl-angebot" aria-label={`${label}: Projektdateien`}>
        {#each angebot as d (schluessel(d.document_id, d.version))}<li>
            <label
              ><input
                type="checkbox"
                value={schluessel(d.document_id, d.version)}
                bind:group={gewaehlt}
              />{d.file_name} · Version {d.version}
              <span class="meta">{kategorieVon(d)}</span></label
            >
          </li>{:else}<li class="meta">
            Keine weitere Projektdatei passt zu dieser Auswahl.
          </li>{/each}
      </ul>
      <button type="button" disabled={!gewaehlt.length} onclick={zuordnen}
        >{gewaehlt.length === 1
          ? "1 Unterlage zuordnen"
          : `${gewaehlt.length} Unterlagen zuordnen`}</button
      >
    </div>{/if}
</div>

<style>
  .dokumentliste {
    min-width: 0;
    margin: 0.5rem 0;
  }
  .dl-kopf {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem 0.75rem;
  }
  .dl-kopf .sekundaer {
    margin-left: auto;
  }
  .dl-werte,
  .dl-angebot {
    list-style: none;
    margin: 0.35rem 0;
    padding: 0;
  }
  .dl-werte li {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem 0.75rem;
    padding: 0.2rem 0;
    border-bottom: 1px solid var(--border-innen);
  }
  .dl-werte li span {
    flex: 1 1 16rem;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .dl-auswahl {
    border: 1px solid var(--border-innen);
    padding: 0.5rem 0.75rem;
    margin-top: 0.35rem;
  }
  .dl-filter {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1rem;
    align-items: end;
  }
  .dl-filter label {
    flex: 1 1 12rem;
  }
  .dl-filter .dl-check {
    flex: 0 0 auto;
  }
  .dl-angebot {
    max-height: 14rem;
    overflow: auto;
  }
  .dl-angebot label {
    display: flex;
    gap: 0.5rem;
    align-items: baseline;
    font-weight: normal;
    padding: 0.15rem 0;
  }
</style>
