<script lang="ts">
  import { editorRegister } from "./editoren";
  import Navigationseintrag from "./Navigationseintrag.svelte";
  import { onMount, untrack } from "svelte";
  import { MODULE, voraussetzung } from "./module";
  import {
    navigationMitModulen,
    pfadZu,
    moduleVon,
    KURZTITEL,
    type NavZweig,
  } from "./navigation";
  import type { Project } from "./api";
  import type { Ansicht } from "./projektSitzung";
  import type { Bereichsstatus } from "./generated/plattform";
  import StatusSymbol, {
    type Statusart,
    RANG,
    REIHENFOLGE,
    STATUSWORT,
    STATUSERKLAERUNG,
    statusartVon,
  } from "./StatusSymbol.svelte";
  let {
    projekt,
    aktiv = true,
    ansicht,
    onbereich,
    staende = {},
    onbenoetigt,
  }: {
    projekt: Project;
    aktiv?: boolean;
    ansicht: Ansicht;
    onbereich: (a: Ansicht) => void;
    /** Bereichsstatus aus dem Core, je Bereichs-ID. */
    staende?: Record<string, Bereichsstatus>;
    onbenoetigt?: (bereich: string, benoetigt: boolean) => void;
  } = $props();
  type Symbol = { art: Statusart; text: string } | null;
  /**
   * Blau aus ungespeicherten Änderungen (nur die Oberfläche kennt sie), sonst
   * der Zustand aus dem Core. Ein Problem bleibt auch ungespeichert rot.
   */
  function symbolVon(id: string): Symbol {
    const modul = MODULE.find((m) => m.id === id);
    if (modul && voraussetzung(projekt, modul)) return null;
    const s = staende[id];
    const art = statusartVon(s?.zustand);
    const text = s?.gruende.join("\n") ?? "";
    if (art === "rot") return { art, text };
    if (editoren?.hatAenderungenIn(id as Ansicht)) return { art: "blau", text: "" };
    return art && { art, text };
  }
  /** Ein Zweig zeigt das dringendste Symbol seiner Bereiche. */
  function zweigSymbol(z: NavZweig): Symbol {
    let best: Symbol = null;
    const offen: string[] = [];
    for (const id of moduleVon(z)) {
      const s = symbolVon(id);
      if (!s) continue;
      if (s.art !== "gruen")
        offen.push(MODULE.find((m) => m.id === id)?.titel ?? id);
      if (!best || RANG[s.art] > RANG[best.art]) best = s;
    }
    return best && { art: best.art, text: offen.join("\n") };
  }
  // Legende des Statusschemas („?“ im Baumkopf).
  let legendeOffen = $state(false);
  let legende = $state<HTMLElement>();
  $effect(() => {
    if (!legendeOffen) return;
    const klick = (e: PointerEvent) => {
      if (legende && !legende.contains(e.target as Node)) legendeOffen = false;
    };
    const taste = (e: KeyboardEvent) => {
      if (e.key === "Escape") legendeOffen = false;
    };
    window.addEventListener("pointerdown", klick, true);
    window.addEventListener("keydown", taste);
    return () => {
      window.removeEventListener("pointerdown", klick, true);
      window.removeEventListener("keydown", taste);
    };
  });
  // „Nur offene“: Geprüftes und Leeres ausblenden (lokal je Benutzer).
  let nurOffen = $state(false);
  try {
    nurOffen = localStorage.getItem("bauplan.baum.nurOffen") === "1";
  } catch {
    /* Standard: alles zeigen */
  }
  function nurOffenUmschalten() {
    nurOffen = !nurOffen;
    try {
      localStorage.setItem("bauplan.baum.nurOffen", nurOffen ? "1" : "0");
    } catch {
      /* Navigation bleibt nutzbar */
    }
  }
  function sichtbar(id: string) {
    if (!nurOffen || istAktiv(id)) return true;
    const s = symbolVon(id);
    return !!s && s.art !== "gruen";
  }
  const NAVIGATION = navigationMitModulen(MODULE);
  // Zugeklappte Zweige des Projektbaums, je Projekt lokal gemerkt.
  const baumSchluessel = () => `bauplan.baum.${projekt.id}`;
  const ZWEIGE = (function sammeln(zs: NavZweig[]): string[] {
    return zs.flatMap((z) => (z.kinder ? [z.id, ...sammeln(z.kinder)] : [z.id]));
  })(NAVIGATION);
  let zu = $state(new Set<string>());
  try {
    const gemerkt = JSON.parse(localStorage.getItem(baumSchluessel()) ?? "[]");
    if (Array.isArray(gemerkt))
      zu = new Set(gemerkt.filter((id: string) => ZWEIGE.includes(id)));
  } catch {
    /* Baum startet aufgeklappt */
  }
  const alleZu = $derived(ZWEIGE.every((id) => zu.has(id)));
  function zuSetzen(neu: Set<string>) {
    zu = neu;
    try {
      localStorage.setItem(baumSchluessel(), JSON.stringify([...neu]));
    } catch {
      /* Navigation bleibt nutzbar */
    }
  }
  function umschalten(id: string) {
    const neu = new Set(zu);
    if (!neu.delete(id)) neu.add(id);
    zuSetzen(neu);
  }
  function alleZuklappen() {
    zuSetzen(new Set(ZWEIGE));
  }
  function alleAufklappen() {
    zuSetzen(new Set());
  }
  // Der Zweig des geöffneten Bereichs bleibt sichtbar.
  $effect(() => {
    const id = ansicht === "mass" ? "grundstueck" : ansicht;
    untrack(() => {
      const pfad = pfadZu(NAVIGATION, id);
      if (pfad.some((z) => zu.has(z))) {
        const neu = new Set(zu);
        for (const z of pfad) neu.delete(z);
        zuSetzen(neu);
      }
    });
  });
  const editoren = editorRegister();
  let suchfeld = $state<HTMLInputElement>();
  let aktionsmeldung = $state("");
  let aktionLaeuft = $state(false);
  async function speichern() {
    if (aktionLaeuft) return;
    aktionLaeuft = true;
    aktionsmeldung = "";
    try {
      const id = ["grz", "gfz", "geschosse", "volumen", "mass"].includes(
        ansicht,
      )
        ? "grundstueck"
        : ansicht;
      const dirty = editoren?.hatAenderungenIn(id);
      const ok = await editoren?.speichernIn(id);
      aktionsmeldung = ok
        ? dirty
          ? "Bereich gespeichert bzw. vorbereitenden Entwurf lokal gesichert."
          : "Keine ungespeicherten Änderungen in diesem Bereich."
        : "Bereich konnte nicht gespeichert werden. Feldfehler und offene Bearbeitungen prüfen.";
    } catch {
      aktionsmeldung = "Speichern fehlgeschlagen; Eingaben bleiben erhalten.";
    } finally {
      aktionLaeuft = false;
    }
  }
  let suche = $state("");
  let favoriten = $state<string[]>([]);
  let zuletzt = $state<string[]>([]);
  onMount(() => {
    try {
      const d = JSON.parse(
        localStorage.getItem(`bauplan.wegen.${projekt.id}`) ?? "{}",
      );
      favoriten = Array.isArray(d.favoriten)
        ? d.favoriten.filter((id: string) => MODULE.some((m) => m.id === id))
        : [];
      zuletzt = Array.isArray(d.zuletzt)
        ? d.zuletzt.filter((id: string) => MODULE.some((m) => m.id === id))
        : [];
    } catch {
      /* optionale lokale Präferenzen */
    }
  });
  function merken(id: string, favorit = false) {
    if (favorit)
      favoriten = favoriten.includes(id)
        ? favoriten.filter((x) => x !== id)
        : [...favoriten, id];
    else zuletzt = [id, ...zuletzt.filter((x) => x !== id)].slice(0, 5);
    try {
      localStorage.setItem(
        `bauplan.wegen.${projekt.id}`,
        JSON.stringify({ favoriten, zuletzt }),
      );
    } catch {
      /* Navigation bleibt nutzbar */
    }
  }
  function oeffnen(id: import("./module").ModulId) {
    merken(id);
    suche = "";
    onbereich(id);
  }
  function istAktiv(id: string) {
    return ansicht === id || (ansicht === "mass" && id === "grundstueck");
  }
</script>

{#snippet eintraege(ids: import("./module").ModulId[], baum = false)}
  {#each baum ? ids.filter(sichtbar) : ids as id}
    {@const modul = MODULE.find((m) => m.id === id)!}
    {@const grund = voraussetzung(projekt, modul)}
    {@const stand = staende[id]}
    <Navigationseintrag
      status={symbolVon(id)}
      nichtBenoetigt={stand?.zustand === "nicht_benoetigt"}
      kennzeichenbar={!!stand?.kennzeichenbar}
      onbenoetigt={onbenoetigt && ((b) => onbenoetigt(id, b))}
      {id}
      titel={modul.titel}
      kurz={KURZTITEL[id] ?? modul.titel}
      {grund}
      entwurf={["grz", "gfz", "geschosse", "volumen"].includes(id)}
      aktiv={istAktiv(id)}
      favorit={favoriten.includes(id)}
      onoeffnen={oeffnen}
      onfavorit={() => merken(id, true)}
    />
  {/each}
{/snippet}
{#snippet zweig(z: NavZweig, ebene: number)}
  {@const offen = !zu.has(z.id)}
  {@const symbol = zweigSymbol(z)}
  {#if !nurOffen || moduleVon(z).some(sichtbar)}<li class="baumzweig" data-ebene={ebene}>
    <button
      type="button"
      class="zweigkopf"
      class:wurzel={ebene === 0}
      aria-expanded={offen}
      aria-label={z.titel}
      title={z.untertitel ?? z.titel}
      onclick={() => umschalten(z.id)}
      ><span class="pfeil" class:offen aria-hidden="true"></span
      ><span class="zweigtitel"
        >{#if z.ziffer}<small class="ziffer">{z.ziffer}</small>{/if}{z.titel}</span
      >{#if symbol}<span class="zweigsymbol"><StatusSymbol art={symbol.art} text={symbol.text} /></span>{/if}</button
    >
    {#if offen}<ul role="group" aria-label={z.titel}>
        {#if z.module.length}<li class="blaetter">{@render eintraege(z.module, true)}</li>{/if}
        {#each z.kinder ?? [] as k (k.id)}{@render zweig(k, ebene + 1)}{/each}
      </ul>{/if}
  </li>{/if}
{/snippet}

<svelte:window
  onkeydown={(e) => {
    if (aktiv && (e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      suchfeld?.focus();
    }
  }}
/>
<aside class="aktennavigation" aria-label="Arbeitsbereiche">
  <label class="nav-suche"
    >Bereich suchen<input
      type="search"
      aria-label="Bereich oder Aktion suchen"
      title="Bereiche und Aktionen suchen (Strg+K)"
      bind:this={suchfeld}
      bind:value={suche}
      placeholder="z. B. GRZ, Formulare"
    /></label
  >
  {#if aktionsmeldung}<p role="status" class="nav-meldung">
      {aktionsmeldung}
    </p>{/if}
  {#if suche}<nav aria-label="Suchergebnisse">
      {#if "aktuellen bereich speichern".includes(suche.toLocaleLowerCase("de"))}<button
          type="button"
          disabled={aktionLaeuft}
          onclick={speichern}>Aktuellen Bereich speichern</button
        >{/if}
      {#if "drucken pdf ausgeben".includes(suche.toLocaleLowerCase("de"))}<button
          type="button"
          onclick={() => oeffnen("druck")}>Druckausgabe öffnen</button
        >{/if}
      {@render eintraege(
        MODULE.filter((m) =>
          m.titel
            .toLocaleLowerCase("de")
            .includes(suche.toLocaleLowerCase("de")),
        ).map((m) => m.id),
      )}
    </nav>{:else}
    {#if favoriten.length}<details>
        <summary>Favoriten</summary>{@render eintraege(
          favoriten as import("./module").ModulId[],
        )}
      </details>{/if}
    {#if zuletzt.length}<details>
        <summary>Zuletzt geöffnet</summary>{@render eintraege(
          zuletzt as import("./module").ModulId[],
        )}
      </details>{/if}
    <div class="baumkopf">
      <span>Projekt</span>
      <button
        type="button"
        class="link"
        aria-pressed={nurOffen}
        title="Nur Bereiche mit Problem, offener Arbeit oder ungespeicherten Änderungen zeigen (Geprüftes und Leeres ausblenden)"
        onclick={nurOffenUmschalten}>Nur offene</button
      >
      <button
        type="button"
        class="link"
        onclick={() => (alleZu ? alleAufklappen() : alleZuklappen())}
        >{alleZu ? "Alle aufklappen" : "Alle zuklappen"}</button
      >
      <span class="legende" bind:this={legende}>
        <button
          type="button"
          class="link legendeknopf"
          aria-expanded={legendeOffen}
          aria-controls="statuslegende"
          aria-label="Legende der Statussymbole"
          title="Legende der Statussymbole"
          onclick={() => (legendeOffen = !legendeOffen)}>?</button
        >
        {#if legendeOffen}<div id="statuslegende" class="legendefeld" role="dialog" aria-label="Legende der Statussymbole">
            <strong>Statussymbole</strong>
            <ul>
              {#each REIHENFOLGE as art (art)}<li data-art={art}>
                  <StatusSymbol {art} /><span
                    ><b>{STATUSWORT[art]}</b> {STATUSERKLAERUNG[art]}</span
                  >
                </li>{/each}
              <li><span class="ohne" aria-hidden="true"></span><span
                  ><b>Ohne Zeichen</b> Nicht benötigt, leer oder noch gesperrt (ⓘ zeigt die Voraussetzung).</span
                ></li>
            </ul>
            <p>Ein Zweig zeigt das dringendste Symbol seiner Bereiche. Listen wie Aufgaben werden nie grün.</p>
          </div>{/if}
      </span>
    </div>
    <nav aria-label="Gliederung der Projektakte">
      <ul class="projektbaum">
        {#each NAVIGATION as z (z.id)}{@render zweig(z, 0)}{/each}
      </ul>
    </nav>{/if}
</aside>

<style>
  nav {
    display: flex;
    flex-direction: column;
  }
  .nav-meldung {
    font-size: 0.8rem;
    padding: 0.5rem;
  }
  .nav-suche {
    display: block;
    padding: 0.5rem;
    font-size: 0.8rem;
  }
  .nav-suche input {
    width: 100%;
    min-width: 0;
  }
  details {
    padding: 0.4rem;
  }
  /* Projektbaum: Register → Gruppe → Bereich, wie der Ebenenbaum im Zeicheneditor. */
  .baumkopf {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 2px 10px 4px;
    font-weight: 600;
    border-bottom: 1px solid var(--border-innen);
    /* Bezugsrahmen der Legende: so breit wie die Navigation. */
    position: relative;
  }
  .baumkopf > span:first-child {
    flex: 1;
  }
  .legende {
    display: inline-flex;
  }
  .baumkopf .legendeknopf {
    width: 1.15rem;
    height: 1.15rem;
    border: 1px solid var(--border);
    border-radius: 50%;
    font-weight: 600;
    text-decoration: none;
    line-height: 1;
  }
  .legendefeld {
    position: absolute;
    top: calc(100% + 2px);
    left: 4px;
    right: 4px;
    z-index: 30;
    padding: 0.6rem 0.7rem;
    background: var(--panel);
    color: var(--fg);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 14px rgb(0 0 0 / 18%);
    font-weight: 400;
    font-size: 0.8rem;
  }
  .legendefeld ul {
    list-style: none;
    margin: 0.4rem 0;
    padding: 0;
  }
  .legendefeld li {
    border: none;
    background: none;
    display: flex;
    gap: 0.45rem;
    align-items: flex-start;
    padding: 0.2rem 0;
  }
  .legendefeld li :global(.statussymbol),
  .legendefeld .ohne {
    margin-top: 2px;
    width: 14px;
    flex: none;
  }
  .legendefeld p {
    margin: 0;
    color: var(--muted);
  }
  .baumkopf .link[aria-pressed="true"] {
    font-weight: 600;
    text-decoration: underline;
  }
  .zweigsymbol {
    margin-left: auto;
    padding-right: 4px;
    display: inline-flex;
  }
  .baumkopf .link {
    white-space: nowrap;
    font-size: 0.8rem;
    font-weight: 400;
    min-height: 0;
    padding: 0;
  }
  .projektbaum,
  .projektbaum ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .projektbaum ul {
    padding-left: 12px;
  }
  /* Globale Listenzeilen (flex, Rahmen) gelten im Baum nicht. */
  .projektbaum li {
    display: block;
    padding: 0;
    border-top: 0;
  }
  .projektbaum > .baumzweig + .baumzweig {
    border-top: 1px solid var(--border-innen);
  }
  .projektbaum .zweigkopf {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    padding-left: 4px;
    color: var(--muted);
    font-weight: 600;
  }
  .projektbaum .zweigkopf.wurzel {
    color: var(--fg);
    padding-block: 5px;
  }
  .zweigtitel {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .ziffer {
    font-size: 0.68rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .pfeil {
    flex: none;
    width: 14px;
    height: 14px;
    position: relative;
  }
  .pfeil::before {
    content: "";
    position: absolute;
    left: 4px;
    top: 3px;
    border-style: solid;
    border-width: 4px 0 4px 6px;
    border-color: transparent transparent transparent currentColor;
    opacity: 0.7;
  }
  .pfeil.offen::before {
    transform: rotate(90deg);
  }
  .blaetter :global(.navigationseintrag > button:not(.favorit)),
  .blaetter :global(.sperrhinweis > button:not(.info)) {
    padding-left: 18px;
  }
</style>
