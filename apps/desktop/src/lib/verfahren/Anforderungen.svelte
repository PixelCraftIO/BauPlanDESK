<script lang="ts">
  import { projektNavigation } from "../projektNavigation.svelte";
  import type { Anforderung } from "../generated/plattform";
  import type { DocumentVersion, Benutzer } from "../api";
  import Dokumentliste from "./Dokumentliste.svelte";
  import Zustaendigkeit from "./Zustaendigkeit.svelte";
  let {
    eintraege = $bindable(),
    dateien,
    benutzer,
    darfFreigeben,
    verfahrenId,
  }: {
    verfahrenId?: string | null;
    eintraege: Anforderung[];
    dateien: DocumentVersion[];
    benutzer: Benutzer[];
    darfFreigeben: boolean;
  } = $props();
  const navigation = projektNavigation();
  /** Bereiche der Projektakte, deren Nachweis eine Anforderung belegt. */
  const NACHWEIS: Record<string, string> = {
    grz: "GRZ",
    gfz: "GFZ",
    geschosse: "Geschossigkeit",
    volumen: "Bauvolumen / BMZ",
    ueberbaubarkeit: "Überbaubare Grundstücksfläche",
    wohnflaechen: "Wohnflächenberechnung",
    din277: "DIN-277-Flächen",
    vollgeschosse: "Vollgeschossprüfung",
    hoehen: "Gebäudehöhen",
    stellplaetze: "Stellplatznachweis",
    abstand: "Abstandsflächen",
    freiflaechen: "Versiegelung / Freiflächen",
    entwaesserung: "Regenwasser / Rückhaltung",
  };
  type Zustand = "klaeren" | "fehlt" | "pruefen" | "erledigt" | "entfaellt";
  const ZUSTAND: Record<Zustand, string> = {
    klaeren: "Zu klären",
    fehlt: "Unterlage fehlt",
    pruefen: "Freigabe offen",
    erledigt: "Erledigt",
    entfaellt: "Nicht erforderlich",
  };
  function zustand(a: Anforderung): Zustand {
    if (a.erforderlich === "ungeklaert") return "klaeren";
    if (a.erforderlich === "nicht_erforderlich") return "entfaellt";
    if (!a.dokumente.length) return "fehlt";
    return a.freigegeben ? "erledigt" : "pruefen";
  }
  type Filter = "offen" | "erledigt" | "entfaellt" | "alle";
  let filter = $state<Filter>("offen");
  /** Aufgeklappte Einträge; alle anderen zeigen nur ihre Kopfzeile. */
  let auf = $state<Record<string, boolean>>({});
  const passt = (a: Anforderung, f: Filter) => {
    const z = zustand(a);
    return f === "alle"
      ? true
      : f === "offen"
        ? z !== "erledigt" && z !== "entfaellt"
        : f === "entfaellt"
          ? a.erforderlich === "nicht_erforderlich"
          : z === "erledigt";
  };
  const zahl = (f: Filter) => eintraege.filter((a) => passt(a, f)).length;
  // Aufgeklappte Einträge bleiben sichtbar, auch wenn sie durch die Bearbeitung
  // den Filter verlassen (sonst verschwände die Zeile unter der Hand).
  const sichtbar = $derived(
    eintraege.filter((a) => passt(a, filter) || auf[a.id]),
  );
  const FILTER: [Filter, string][] = [
    ["offen", "Zu erledigen"],
    ["erledigt", "Erledigt"],
    ["entfaellt", "Nicht erforderlich"],
    ["alle", "Alle"],
  ];
  function ergaenzen() {
    const id = crypto.randomUUID();
    eintraege.push({
      id,
      titel: "",
      erforderlich: "ungeklaert",
      grund: "",
      quelle: "Manuell",
      dokumente: [],
      freigegeben: false,
      verantwortlich: null,
    });
    auf[id] = true;
    if (filter !== "alle") filter = "offen";
  }
  // Sprung aus Übersicht oder Einreichungsprüfung: Eintrag zeigen und öffnen,
  // auch wenn der Filter ihn gerade ausblendet.
  $effect(() => {
    const z = navigation?.auftrag?.ziel;
    if (z?.register === "anforderungen" && z.eintrag) auf[z.eintrag] = true;
  });
  const nachweisOeffnen = (bereich: string) =>
    navigation?.springen({
      bereich,
      objekt: null,
      register: null,
      eintrag: null,
    });
  const auftrag = $derived(navigation?.unterlagenauftrag);
  function erstellen(id: string, titel: string, bereich = "formulare") {
    if (!verfahrenId) return;
    navigation?.unterlageErstellen(
      {
        bereich: "verfahren",
        objekt: verfahrenId,
        register: "anforderungen",
        eintrag: id,
      },
      titel,
      bereich,
    );
  }
</script>

<h3>Anforderungen und Vollständigkeit</h3>
<p class="meta">
  Eine Zeile je Anforderung; ein Klick öffnet die Einzelheiten. Eine Kategorie
  allein erfüllt keine Anforderung: Erforderlichkeit und genaue Unterlagenstände
  werden ausdrücklich zugeordnet und freigegeben.
</p>
<div class="anf-kopf">
  <div class="anf-filter" role="group" aria-label="Anforderungen anzeigen">
    {#each FILTER as [f, name]}<button
        type="button"
        class="sekundaer"
        aria-pressed={filter === f}
        onclick={() => (filter = f)}>{name} ({zahl(f)})</button
      >{/each}
  </div>
  <button type="button" class="sekundaer" onclick={ergaenzen}
    >Anforderung ergänzen</button
  >
</div>
{#if !sichtbar.length}<p class="meta">
    {eintraege.length
      ? "Keine Anforderung in dieser Ansicht."
      : "Noch keine Anforderungen. Unter „Vorhabenprofil“ aus dem Katalog vorbereiten oder hier einzeln ergänzen."}
  </p>{/if}
<div class="anf-liste">
  {#each sichtbar as a (a.id)}{@const z = zustand(a)}<details
      data-uebersicht-eintrag={a.id}
      open={!!auf[a.id]}
      ontoggle={(e) => (auf[a.id] = e.currentTarget.open)}
    >
      <summary>
        <span class="anf-status {z}">{ZUSTAND[z]}</span>
        <span class="anf-titel">{a.titel || "Neue Anforderung"}</span>
        <span class="meta"
          >{a.dokumente.length
            ? `${a.dokumente.length} Unterlage${a.dokumente.length === 1 ? "" : "n"}`
            : "keine Unterlage"}{a.nachweise?.length
            ? ` · Nachweis: ${a.nachweise.map((n) => NACHWEIS[n] ?? n).join(", ")}`
            : ""}</span
        >
      </summary>
      {#if auf[a.id]}
        <div class="anf-inhalt">
          {#if verfahrenId}<div class="knoepfe links">
              {#each a.nachweise ?? [] as n}<button
                  type="button"
                  class="sekundaer"
                  onclick={() => nachweisOeffnen(n)}
                  >{NACHWEIS[n] ?? n} öffnen …</button
                >{/each}
              {#if !a.nachweise?.length}<button
                  type="button"
                  class="sekundaer"
                  onclick={() => erstellen(a.id, a.titel)}
                  >Formular für diese Anforderung erstellen …</button
                >{/if}<button
                type="button"
                class="link"
                onclick={() => erstellen(a.id, a.titel, "dokumente")}
                >Datei importieren oder auswählen …</button
              >
            </div>{/if}
          {#if auftrag?.ziel.objekt === verfahrenId && auftrag?.ziel.eintrag === a.id && auftrag.dokument}<aside
              class="hinweisbox"
            >
              <p>
                Neue Dokumentversion bereit. Die fachliche Freigabe wird nach
                der Zuordnung zurückgesetzt.
              </p>
              <button
                type="button"
                onclick={() => {
                  const d = auftrag.dokument!;
                  if (
                    !a.dokumente.some(
                      (x) =>
                        x.dokument_id === d.dokument_id &&
                        x.version === d.version,
                    )
                  )
                    a.dokumente.push({ ...d });
                  a.freigegeben = false;
                  if (navigation) navigation.unterlagenauftrag = null;
                }}>Version dieser Anforderung zuordnen</button
              ><button
                type="button"
                class="link"
                onclick={() => {
                  if (navigation) navigation.unterlagenauftrag = null;
                }}>Vorschlag verwerfen</button
              >
            </aside>{/if}
          <div class="raster">
            <label
              >Anforderung<span class="pflicht" aria-hidden="true"> *</span
              ><input
                aria-label="Anforderung"
                required
                maxlength="500"
                bind:value={a.titel}
              /></label
            ><label
              >Erforderlichkeit<select
                bind:value={a.erforderlich}
                onchange={() => (a.freigegeben = false)}
                ><option value="ungeklaert">Noch zu klären</option><option
                  value="erforderlich">Erforderlich</option
                ><option value="nicht_erforderlich">Nicht erforderlich</option
                ></select
              ></label
            ><Zustaendigkeit {benutzer} bind:wert={a.verantwortlich} />
          </div>
          <label class="anf-voll"
            >Begründung<textarea rows="2" maxlength="4000" bind:value={a.grund}
            ></textarea></label
          ><label class="anf-voll"
            >Quelle / Katalogstand<input
              maxlength="4000"
              bind:value={a.quelle}
            /></label
          ><Dokumentliste {dateien} bind:werte={a.dokumente} /><label
            class="anf-freigabe"
            ><input
              type="checkbox"
              disabled={!darfFreigeben}
              bind:checked={a.freigegeben}
            />Fachlich freigegeben (Projektleitung)</label
          >
        </div>
      {/if}
    </details>{/each}
</div>

<style>
  .anf-kopf {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1rem;
    align-items: center;
    justify-content: space-between;
    margin: 0.5rem 0;
  }
  .anf-filter {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }
  .anf-filter button[aria-pressed="true"] {
    background: var(--action-bg);
    border-color: var(--action-bg);
    color: var(--on-action);
  }
  .anf-liste {
    border-top: 1px solid var(--border-innen);
  }
  .anf-liste details {
    border-bottom: 1px solid var(--border-innen);
    margin: 0;
    padding: 0;
  }
  .anf-liste summary {
    display: grid;
    grid-template-columns: 9.5rem minmax(0, 1fr) auto;
    gap: 0.75rem;
    align-items: center;
    padding: 0.4rem 0.25rem;
    cursor: pointer;
  }
  .anf-liste summary:hover {
    background: var(--zebra);
  }
  .anf-titel {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }
  .anf-status {
    font-size: 0.85em;
    padding: 0.1rem 0.4rem;
    border-radius: 3px;
    border: 1px solid currentColor;
    text-align: center;
    white-space: nowrap;
  }
  .anf-status.klaeren,
  .anf-status.fehlt {
    color: var(--error-text);
  }
  .anf-status.pruefen {
    color: var(--warning-text);
  }
  .anf-status.erledigt {
    color: var(--ok-text);
  }
  .anf-status.entfaellt {
    color: var(--muted);
  }
  .anf-inhalt {
    padding: 0.25rem 0.25rem 0.9rem 0.25rem;
    display: grid;
    gap: 0.5rem;
  }
  .anf-voll {
    display: grid;
    gap: 0.2rem;
  }
  .anf-voll textarea,
  .anf-voll input {
    width: 100%;
    box-sizing: border-box;
  }
  .anf-freigabe {
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }
</style>
