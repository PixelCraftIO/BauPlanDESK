<script lang="ts">
  import type {
    Abweichung,
    Kommunikation,
    Schrittart,
    Verfahrensschritt,
  } from "../generated/plattform";
  import type { DocumentVersion, Benutzer } from "../api";
  import Fristfeld from "./Fristfeld.svelte";
  import Zustaendigkeit from "./Zustaendigkeit.svelte";
  import { SCHRITTART } from "./api";
  let {
    eintraege = $bindable(),
    kommunikation,
    dateien,
    benutzer,
    abweichungen = [],
    alt = [],
    gruendeAlt = [],
  }: {
    eintraege: Verfahrensschritt[];
    kommunikation: Kommunikation[];
    dateien: DocumentVersion[];
    benutzer: Benutzer[];
    abweichungen?: Abweichung[];
    alt?: string[];
    /** Gespeicherte Beanstandungen; ihr Wortlaut bleibt laut Core unverändert. */
    gruendeAlt?: string[];
  } = $props();
  let neuerGrund = $state<Record<string, string>>({});
  function grund(v: Verfahrensschritt) {
    const text = neuerGrund[v.id]?.trim();
    if (!text) return;
    v.gruende = [
      ...(v.gruende ?? []),
      { id: crypto.randomUUID(), text, grundlage: "", reaktion: "" },
    ];
    neuerGrund[v.id] = "";
  }
  // Ältere Akten führen noch keine Verfahrensschritte.
  eintraege ??= [];
  const eingehend = $derived(kommunikation.filter((k) => k.eingehend));
  const ausgehend = $derived(kommunikation.filter((k) => !k.eingehend));
  let art = $state<Schrittart>("anhoerung");
  let schreiben = $state("");
  function neu() {
    if (!schreiben) return;
    eintraege = [
      ...eintraege,
      {
        id: crypto.randomUUID(),
        art,
        kommunikation: schreiben,
        frist: null,
        stellungnahme: null,
        verantwortlich: null,
      },
    ];
    schreiben = "";
  }
  const betreff = (id: string | null) => {
    const k = kommunikation.find((k) => k.id === id);
    return k ? `${k.datum} · ${k.betreff}` : "Schreiben fehlt";
  };
</script>

<h3>Anhörung, Zurückstellung und Beteiligungen</h3>
<p class="meta">
  Vor einer belastenden Entscheidung hört die Behörde an (§ 28 VwVfG); die
  Gemeinde kann ein Baugesuch zurückstellen lassen (§ 15 BauGB). Beides beginnt
  mit einem eingegangenen Schreiben im Register Kommunikation.
</p>
{#if eingehend.length}<div class="raster">
    <label
      >Art<select bind:value={art}
        >{#each Object.entries(SCHRITTART) as [wert, titel]}<option
            value={wert}>{titel}</option
          >{/each}</select
      ></label
    ><label
      >Schreiben der Behörde<select bind:value={schreiben}
        ><option value="">Kommunikation auswählen</option
        >{#each eingehend as k (k.id)}<option value={k.id}
            >{k.datum} · {k.betreff}</option
          >{/each}</select
      ></label
    >
  </div>
  <button type="button" class="sekundaer" disabled={!schreiben} onclick={neu}
    >Verfahrensschritt erfassen</button
  >{:else}<p class="meta">
    Zuerst das Schreiben der Behörde im Register Kommunikation erfassen.
  </p>{/if}
{#each eintraege as v (v.id)}<fieldset data-uebersicht-eintrag={v.id}>
    <legend>{SCHRITTART[v.art]} · {betreff(v.kommunikation)}</legend>
    {#if !alt.includes(v.id)}<button
        type="button"
        class="link"
        onclick={() => (eintraege = eintraege.filter((n) => n.id !== v.id))}
        >Noch nicht gespeicherten Schritt verwerfen</button
      >{/if}
    <p class="meta">
      {v.art === "anhoerung"
        ? "Frist: bis wann die Stellungnahme vorliegen muss."
        : v.art === "zurueckstellung"
          ? "Frist: Ende der Zurückstellung (Pflichtangabe)."
          : v.art === "einvernehmen"
            ? "Frist: Ende der Zwei-Monats-Frist ab Eingang des Ersuchens bei der Gemeinde. Ohne Verweigerung gilt das Einvernehmen danach als erteilt (§ 36 Abs. 2 Satz 2 BauGB)."
            : "Frist: Ende der Beteiligungsfrist laut Schreiben."}
    </p>
    <Fristfeld {dateien} bind:wert={v.frist} />
    <div class="raster">
      {#if v.art === "anhoerung"}<label
          >Stellungnahme<select bind:value={v.stellungnahme}
            ><option value={null}>Noch keine</option
            >{#each ausgehend as k (k.id)}<option value={k.id}
                >{k.datum} · {k.betreff}</option
              >{/each}</select
          ></label
        >{:else if v.art === "zurueckstellung"}<label
          >Beendet durch Folgeschreiben<select
            value={v.folgeschreiben ?? ""}
            onchange={(e) =>
              (v.folgeschreiben = e.currentTarget.value || undefined)}
            ><option value="">Noch keines</option
            >{#each eingehend.filter((k) => k.id !== v.kommunikation) as k (k.id)}<option
                value={k.id}>{k.datum} · {k.betreff}</option
              >{/each}</select
          ></label
        >{/if}<Zustaendigkeit {benutzer} bind:wert={v.verantwortlich} />
    </div>
    {#if v.art === "anhoerung"}<h4>Beanstandungen und Reaktion</h4>
      {#each v.gruende ?? [] as g (g.id)}<div class="beanstandung">
          <p>
            <strong>{g.text}</strong>{#if g.grundlage}<span class="meta">
                · {g.grundlage}</span
              >{/if}
          </p>
          <div class="raster">
            <label
              >Reaktion (Stellungnahme, Planänderung …)<textarea
                rows="2"
                maxlength="16000"
                bind:value={g.reaktion}
              ></textarea></label
            ><label
              >Mit Abweichungsantrag<select
                value={g.abweichung ?? ""}
                onchange={(e) =>
                  (g.abweichung = e.currentTarget.value || undefined)}
                ><option value="">Keiner</option
                >{#each abweichungen as e (e.id)}<option value={e.id}
                    >{e.gegenstand || e.vorschrift}</option
                  >{/each}</select
              ></label
            >
          </div>
          {#if !gruendeAlt.includes(g.id)}<button
              type="button"
              class="link"
              onclick={() =>
                (v.gruende = (v.gruende ?? []).filter((n) => n.id !== g.id))}
              >Noch nicht gespeicherte Beanstandung verwerfen</button
            >{/if}
        </div>{/each}
      <div class="raster">
        <label
          >Weitere Beanstandung (Wortlaut der Behörde)<input
            maxlength="4000"
            bind:value={neuerGrund[v.id]}
          /></label
        >
      </div>
      <button
        type="button"
        class="sekundaer"
        disabled={!neuerGrund[v.id]?.trim()}
        onclick={() => grund(v)}>Beanstandung erfassen</button
      >{/if}
    {#if v.art === "zurueckstellung"}<p class="meta">
        Die Wiedervorlage endet mit dem Fristende oder einem Folgeschreiben der
        Behörde; danach steht ihre Entscheidung aus.
      </p>{/if}
  </fieldset>{/each}
