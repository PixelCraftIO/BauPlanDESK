<script lang="ts">
  import { tick, untrack, onDestroy } from "svelte";
  import { handbuch, sucheKapitel } from "./handbuch";
  import { handbuchMarkdown } from "./export";
  import { handbuchDrucken } from "./druck";
  import { MODULE } from "../module";
  let { anfrage, projektOffen = false, onbereich, onzurueck }: {
    anfrage: { id: string; nummer: number };
    projektOffen?: boolean;
    onbereich: (id: string) => void;
    onzurueck: () => void;
  } = $props();
  let suche = $state("");
  let auswahl = $state("start");
  let ueberschrift = $state<HTMLHeadingElement>();
  let fehler = $state("");
  let aufraeumen: (() => void) | undefined;
  let downloadUrl: string | undefined;
  let zuletzt = -1;
  const treffer = $derived(sucheKapitel(suche));
  const kapitel = $derived(handbuch.kapitel.find((k) => k.id === auswahl) ?? handbuch.kapitel[0]);
  const gruppen = [...new Set(handbuch.kapitel.map((k) => k.gruppe))];
  const index = $derived(handbuch.kapitel.indexOf(kapitel));
  $effect(() => {
    if (anfrage.nummer === zuletzt) return;
    zuletzt = anfrage.nummer;
    untrack(() => { suche = ""; auswahl = anfrage.id; });
    void tick().then(() => ueberschrift?.focus({ preventScroll: true }));
  });
  function oeffnen(id: string) {
    auswahl = id;
    void tick().then(() => {
      ueberschrift?.focus({ preventScroll: true });
      ueberschrift?.scrollIntoView({ block: "start" });
    });
  }
  function suchen(s: string) {
    suche = s;
    const hits = sucheKapitel(s);
    if (hits.length && !hits.some((k) => k.id === auswahl)) auswahl = hits[0].id;
  }
  function drucken() {
    fehler = "";
    try { aufraeumen?.(); aufraeumen = handbuchDrucken(); }
    catch (e) { fehler = e instanceof Error ? e.message : "Handbuch konnte nicht gedruckt werden."; }
  }
  function herunterladen() {
    if (downloadUrl) URL.revokeObjectURL(downloadUrl);
    downloadUrl = URL.createObjectURL(new Blob([handbuchMarkdown()], { type: "text/markdown;charset=utf-8" }));
    const link = document.createElement("a");
    link.href = downloadUrl; link.download = "BauPlanDESK-Bedienhandbuch.md";
    document.body.append(link); link.click(); link.remove();
  }
  onDestroy(() => { aufraeumen?.(); if (downloadUrl) URL.revokeObjectURL(downloadUrl); });
</script>

<main class="hilfe" aria-label="Bedienhandbuch">
  <header class="hilfe-kopf">
    <div><h1>Hilfe / Handbuch</h1><p>Für Architekten und Planer · Stand {handbuch.stand} · offline verfügbar</p></div>
    <div class="knoepfe links">
      {#if projektOffen}<button type="button" class="sekundaer" onclick={onzurueck}>Zurück zum Projekt</button>{/if}
      <button type="button" class="sekundaer" onclick={herunterladen}>Handbuch herunterladen (.md)</button>
      <button type="button" onclick={drucken}>Gesamtes Handbuch drucken</button>
    </div>
  </header>
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  <div class="hilfe-layout">
    <aside class="hilfe-gliederung" aria-label="Handbuchnavigation">
      <label for="handbuch-suche">Handbuch durchsuchen</label>
      <input id="handbuch-suche" type="search" value={suche} oninput={(e) => suchen(e.currentTarget.value)} placeholder="z. B. Nachforderung, GRZ oder Formular" />
      <p class="meta" role="status" aria-live="polite">{treffer.length} von {handbuch.kapitel.length} Kapiteln{suche.trim() ? " gefunden" : ""}</p>
      {#if suche}<button class="link" onclick={() => suchen("")}>Suche zurücksetzen</button>{/if}
      <nav aria-label="Inhaltsverzeichnis">
        {#each gruppen as gruppe}
          {@const inGruppe = treffer.filter((k) => k.gruppe === gruppe)}
          {#if inGruppe.length}<h2>{gruppe}</h2>
            {#each inGruppe as k (k.id)}<button type="button" aria-current={auswahl === k.id ? "page" : undefined} onclick={() => oeffnen(k.id)} title={k.kurztext}>{k.titel}</button>{/each}
          {/if}
        {/each}
      </nav>
    </aside>
    <div class="hilfe-lesebereich">
      {#if !treffer.length}<section class="hilfe-leer" aria-label="Keine Suchtreffer"><h2>Keine passenden Kapitel</h2><p>Andere Suchbegriffe verwenden oder die Suche zurücksetzen. Die Suche prüft auch Schritte, Tabellen und Fragen im Handbuch.</p></section>
      {:else}
        <article aria-labelledby="handbuch-kapitel">
          <p class="hilfe-meta">{kapitel.gruppe} · Kapitel {index + 1} von {handbuch.kapitel.length}</p>
          <h2 id="handbuch-kapitel" tabindex="-1" bind:this={ueberschrift}>{kapitel.titel}</h2>
          <p class="hilfe-kurztext">{kapitel.kurztext}</p>
          {#each kapitel.abschnitte as a}
            <section>
              {#if a.titel}<h3>{a.titel}</h3>{/if}
              {#each a.text ?? [] as p}<p>{p}</p>{/each}
              {#if a.schritte}<ol>{#each a.schritte as p}<li>{p}</li>{/each}</ol>{/if}
              {#if a.liste}<ul>{#each a.liste as p}<li>{p}</li>{/each}</ul>{/if}
              {#if a.tabelle}<div class="hilfe-tabelle"><table><thead><tr>{#each a.tabelle.spalten as spalte}<th scope="col">{spalte}</th>{/each}</tr></thead><tbody>{#each a.tabelle.zeilen as zeile}<tr>{#each zeile as zelle}<td>{zelle}</td>{/each}</tr>{/each}</tbody></table></div>{/if}
              {#if a.hinweis}<aside class="hilfe-hinweis">{a.hinweis}</aside>{/if}
            </section>
          {/each}
          {#if kapitel.bereiche.length && projektOffen}
            <section class="hilfe-weiter"><h3>Im geöffneten Projekt</h3><div class="knoepfe links">
              {#each kapitel.bereiche as id}
                {@const m = MODULE.find((v) => v.id === id)}
                {#if m}<button type="button" class="sekundaer" onclick={() => onbereich(id)}>{m.titel} öffnen</button>{/if}
              {/each}
            </div></section>
          {/if}
          <nav class="hilfe-weiter" aria-label="Weiterführende Kapitel"><h3>Weiterlesen</h3>
            {#each kapitel.siehe as id}<button type="button" class="link" onclick={() => { suche = ""; oeffnen(id); }}>{handbuch.kapitel.find((k) => k.id === id)?.titel}</button>{/each}
          </nav>
          <nav class="hilfe-blaettern" aria-label="Kapitel wechseln">
            <button class="sekundaer" disabled={index === 0} onclick={() => { suche = ""; oeffnen(handbuch.kapitel[index - 1].id); }}>← Voriges Kapitel</button>
            <button class="sekundaer" disabled={index === handbuch.kapitel.length - 1} onclick={() => { suche = ""; oeffnen(handbuch.kapitel[index + 1].id); }}>Nächstes Kapitel →</button>
          </nav>
        </article>
      {/if}
    </div>
  </div>
</main>

<style>
  .hilfe { display:block; max-width:1440px; margin:0 auto; min-width:0; }
  .hilfe-kopf { display:flex; flex-wrap:wrap; gap:1.5rem; justify-content:space-between; margin-bottom:1.5rem; }
  .hilfe-kopf h1 { margin-bottom:.5rem; }
  .hilfe-kopf p { color:var(--muted); }
  .hilfe-layout { display:grid; grid-template-columns:minmax(250px, 300px) minmax(0,1fr); gap:2.5rem; align-items:start; }
  .hilfe-gliederung { position:sticky; top:1rem; max-height:calc(100dvh - 150px); overflow:auto; padding:.5rem 1rem 1rem 0; scrollbar-gutter:stable; }
  .hilfe-gliederung input { width:100%; min-width:0; }
  .hilfe-gliederung label { display:block; font-weight:600; margin-bottom:.5rem; }
  .hilfe-gliederung h2 { margin:1.5rem 0 .5rem; font-size:.9rem; color:var(--muted); }
  .hilfe-gliederung nav button { display:block; width:100%; text-align:left; white-space:normal; overflow-wrap:anywhere; background:transparent; color:var(--fg); border:1px solid transparent; border-radius:4px; margin:.15rem 0; padding:.6rem .7rem; line-height:1.4; }
  .hilfe-gliederung nav button:hover { background:var(--panel); border-color:var(--border); }
  .hilfe-gliederung nav button[aria-current="page"] { background:var(--panel); border-color:var(--accent); font-weight:600; color:var(--accent); }
  .hilfe-lesebereich { min-width:0; }
  article { max-width:880px; font-size:1rem; line-height:1.75; padding-bottom:3rem; }
  article > section { background:transparent; border:0; padding:0; }
  article h2 { font-size:1.8rem; line-height:1.3; scroll-margin-top:1rem; }
  article h3 { margin:1.8rem 0 .65rem; font-size:1.15rem; line-height:1.4; }
  article p { margin:.85rem 0; }
  .hilfe-meta { color:var(--muted); font-size:.9rem; }
  .hilfe-kurztext { font-size:1.1rem; color:var(--muted); }
  article ol,article ul { padding-left:1.6rem; list-style-position:outside; }
  article ol { list-style-type:decimal; }
  article ul { list-style-type:disc; }
  article li { display:list-item; border:0; padding:0 0 0 .3rem; margin-bottom:.8rem; }
  .hilfe-hinweis { padding:1rem 1.2rem; border-left:3px solid var(--accent); background:var(--panel); margin:1.4rem 0; }
  .hilfe-tabelle { max-width:100%; overflow:auto; margin:1rem 0; }
  table { border-collapse:collapse; width:100%; line-height:1.5; }
  th,td { padding:.85rem; border:1px solid var(--border); text-align:left; vertical-align:top; min-width:150px; }
  th { background:var(--panel); }
  .hilfe-weiter { border-top:1px solid var(--border); margin-top:2rem; padding-top:.3rem; }
  .hilfe-weiter .link { display:block; text-align:left; white-space:normal; margin:.4rem 0; }
  .hilfe-blaettern { display:flex; flex-wrap:wrap; gap:1rem; justify-content:space-between; border-top:1px solid var(--border); margin-top:2rem; padding-top:1rem; }
  .hilfe-leer { padding:2rem; background:var(--panel); border:1px solid var(--border); }
  @media (max-width:800px) {
    .hilfe-layout { grid-template-columns:minmax(0,1fr); gap:1.5rem; }
    .hilfe-gliederung { position:static; max-height:300px; border:1px solid var(--border); padding:1rem; }
    article h2 { font-size:1.5rem; }
  }
</style>
