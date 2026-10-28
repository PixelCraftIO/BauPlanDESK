<script lang="ts">
  /** Nacharbeit des Modells: Änderungen über die GlobalId, gespeichert in der Modellakte. */
  import { untrack } from "svelte";
  import { fehlertext } from "../api";
  import { plattformApi, type Datensatz } from "../plattformApi";
  import type { Befund, Modellaenderung, Modellakte, Modellauszug, Planpolygon, Raumvorschlag } from "../generated/plattform";
  import { ifcApi } from "./api";
  import { ARTEN, NACHARBEIT_KLASSEN, elementtitel, flaeche, klassentitel, laenge } from "./format";
  let { projektId, akte, auszug, auswahl, markiert, geschoss, darf, vorgabe = null, punkte = [], zeichnen = false, onzeichnen, onakte, onvorschau, onauswahl, onmarkieren }: {
    projektId: string;
    akte: Datensatz<Modellakte>;
    auszug: Modellauszug;
    auswahl: string;
    markiert: string[];
    geschoss: string;
    darf: boolean;
    /** Befund der Prüfung, dessen Behebung vorbelegt wird. */
    vorgabe?: Befund | null;
    /** Im Grundriss gesetzte Punkte für einen neuen Umriss (Modellkoordinaten). */
    punkte?: [number, number][];
    zeichnen?: boolean;
    onzeichnen?: (an: boolean) => void;
    onakte: (a: Datensatz<Modellakte>) => void;
    onvorschau: (p: Planpolygon[]) => void;
    onauswahl: (gid: string) => void;
    onmarkieren: (gids: string[]) => void;
  } = $props();
  const liste = $derived(akte.daten.nacharbeit ?? []);
  const ziele = $derived(markiert.length ? markiert : auswahl ? [auswahl] : []);
  const einzel = $derived(ziele.length === 1 ? auszug.elemente.find(e => e.global_id === ziele[0]) ?? null : null);
  let klasse = $state("IFCWALL"), art = $state(""), zielGeschoss = $state("");
  let satz = $state("Pset_WallCommon"), name = $state("IsExternal"), typ = $state<"text" | "zahl" | "logisch">("logisch"), wert = $state("true");
  let neuName = $state(""), neuLang = $state("");
  let uk = $state(0), ok = $state(0);
  let laeuft = $state(false), fehler = $state(""), meldung = $state("");
  let vorschlaege = $state<Raumvorschlag[]>([]), raumNamen = $state<Record<string, string>>({});
  $effect(() => {
    const v = vorgabe; if (!v) return;
    if (v.eigenschaft) { const [s, n] = v.eigenschaft.split("."); satz = s; name = n ?? ""; }
  });
  $effect(() => { if (einzel) { neuName = einzel.name; neuLang = einzel.langname ?? ""; } });
  $effect(() => { const g = einzel?.geometrie; if (g) { uk = g.unten_m; ok = g.oben_m; } });
  // Zeichnen endet, wenn ein anderes Element gewählt wird.
  $effect(() => { void einzel?.global_id; untrack(() => { if (zeichnen) onzeichnen?.(false); }); });
  $effect(() => { if (!zielGeschoss || !auszug.geschosse.some(g => g.global_id === zielGeschoss)) zielGeschoss = geschoss || auszug.geschosse[0]?.global_id || ""; });
  $effect(() => { void geschoss; vorschlaege = []; onvorschau([]); });

  async function speichern(neu: Modellaenderung[], text: string) {
    if (!darf) return;
    laeuft = true; fehler = ""; meldung = "";
    try {
      const f = await plattformApi.speichern<Modellakte>(projektId, "ifcmodell", akte.objekt, akte.revision, { ...$state.snapshot(akte.daten), nacharbeit: neu });
      onakte(f); meldung = text;
    } catch (e) { fehler = fehlertext(e); } finally { laeuft = false; }
  }
  const anhaengen = (c: Modellaenderung[], text: string) => speichern([...$state.snapshot(liste), ...c], text);
  function klassifizieren() { void anhaengen(ziele.map(g => ({ art: "klassifizieren", global_id: g, klasse, vordefiniert: art || null })), `${ziele.length} Element(e) als ${klassentitel(klasse)} klassifiziert.`); }
  function zuordnen() { void anhaengen(ziele.map(g => ({ art: "geschoss_zuordnen", global_id: g, geschoss: zielGeschoss })), `${ziele.length} Element(e) dem Geschoss zugeordnet.`); }
  function eigenschaft() {
    const w = typ === "zahl" ? { typ: "zahl" as const, wert: Number(String(wert).replace(",", ".")) } : typ === "logisch" ? { typ: "logisch" as const, wert: wert === "true" } : { typ: "text" as const, wert };
    if (w.typ === "zahl" && !Number.isFinite(w.wert)) { fehler = "Zahl eingeben."; return; }
    void anhaengen(ziele.map(g => ({ art: "eigenschaft_setzen", global_id: g, satz: satz.trim(), name: name.trim(), wert: w })), `${satz}.${name} an ${ziele.length} Element(en) gesetzt.`);
  }
  function benennen() { if (einzel) void anhaengen([{ art: "benennen", global_id: einzel.global_id, name: neuName.trim(), langname: neuLang.trim() || null }], "Name geändert."); }
  function hoehe() {
    const g = einzel?.geometrie; if (!einzel || !g) return;
    const u = Number(uk), o = Number(ok);
    if (!Number.isFinite(u) || !Number.isFinite(o) || o - u < 0.01) { fehler = "Oberkante über der Unterkante angeben."; return; }
    const unten_m = Math.abs(u - g.unten_m) > 1e-4 ? u : null, oben_m = Math.abs(o - g.oben_m) > 1e-4 ? o : null;
    if (unten_m == null && oben_m == null) { meldung = "Höhe unverändert."; return; }
    void anhaengen([{ art: "hoehe_setzen", global_id: einzel.global_id, unten_m, oben_m }], `Höhe von ${elementtitel(einzel)} geändert.`);
  }
  function umriss() {
    if (!einzel || punkte.length < 3) return;
    void anhaengen([{ art: "umriss_setzen", global_id: einzel.global_id, umriss: $state.snapshot(punkte) as [number, number][] }], `Umriss von ${elementtitel(einzel)} ersetzt.`).then(() => onzeichnen?.(false));
  }
  function ausblenden() { void anhaengen(ziele.map(g => ({ art: "ausblenden", global_id: g })), `${ziele.length} Element(e) ausgeblendet.`); }
  async function vorschlagen() {
    fehler = "";
    try {
      vorschlaege = await ifcApi.raumvorschlaege(projektId, akte.objekt, geschoss);
      onvorschau(vorschlaege.filter(v => !v.vorhanden).map(v => ({ aussen: v.umriss })));
      if (!vorschlaege.length) meldung = "Keine geschlossenen Flächen zwischen den Wänden gefunden.";
    } catch (e) { fehler = fehlertext(e); }
  }
  function raumBilden(v: Raumvorschlag, i: number) {
    const n = (raumNamen[v.global_id] ?? "").trim() || `Raum ${i + 1}`;
    const [nummer, ...rest] = n.split(" ");
    void anhaengen([{ art: "raum_bilden", global_id: v.global_id, geschoss, name: rest.length ? nummer : n, langname: rest.length ? rest.join(" ") : null, umriss: v.umriss, hoehe_m: null }], `Raum „${n}“ gebildet.`).then(() => {
      vorschlaege = vorschlaege.filter(x => x.global_id !== v.global_id);
      onvorschau(vorschlaege.filter(v => !v.vorhanden).map(v => ({ aussen: v.umriss })));
    });
  }
  function entfernen(i: number) { void speichern($state.snapshot(liste).filter((_, j) => j !== i), "Änderung zurückgenommen."); }
  const beschreibung = (c: Modellaenderung): string => {
    const t = auszug.elemente.find(e => e.global_id === c.global_id);
    const wer = t ? elementtitel(t) : c.art === "ausblenden" ? "ausgeblendetes Element" : c.global_id;
    switch (c.art) {
      case "klassifizieren": return `${wer}: Klasse ${klassentitel(c.klasse)}${c.vordefiniert ? ` (${c.vordefiniert})` : ""}`;
      case "geschoss_zuordnen": return `${wer}: Geschoss ${auszug.geschosse.find(g => g.global_id === c.geschoss)?.name ?? "?"}`;
      case "eigenschaft_setzen": return `${wer}: ${c.satz}.${c.name} = ${c.wert.typ === "logisch" ? (c.wert.wert ? "Ja" : "Nein") : c.wert.wert}`;
      case "benennen": return `${wer}: Name „${c.name}${c.langname ? ` ${c.langname}` : ""}“`;
      case "raum_bilden": return `Neuer Raum „${c.name}${c.langname ? ` ${c.langname}` : ""}“`;
      case "ausblenden": return `${wer}: ausgeblendet`;
      case "hoehe_setzen": return `${wer}: ${c.unten_m != null ? `Unterkante ${laenge(c.unten_m)}` : ""}${c.unten_m != null && c.oben_m != null ? ", " : ""}${c.oben_m != null ? `Oberkante ${laenge(c.oben_m)}` : ""}`;
      case "umriss_setzen": return `${wer}: neuer Umriss (${c.umriss.length} Punkte)`;
    }
  };
</script>

<section class="nacharbeit" aria-label="Nacharbeit">
  <p class="meta">Änderungen gelten für BauPlanIO (Prüfung, Übernahme, Auswertung, Export); die IFC-Datei bleibt unverändert. Sie hängen an der GlobalId und überstehen einen Reimport.</p>
  <h4>Auswahl: {ziele.length ? (einzel ? elementtitel(einzel) : `${ziele.length} Elemente`) : "keine"}</h4>
  {#if markiert.length}<button type="button" class="link" onclick={() => onmarkieren([])}>Hervorhebung aufheben</button>{/if}
  {#if !ziele.length}<p class="meta">Ein Element wählen oder in der Prüfung „Im Modell zeigen“ für mehrere.</p>{/if}
  <fieldset disabled={!darf || laeuft || !ziele.length}>
    <div class="zeile"><label>Klasse<select aria-label="Neue Klasse" bind:value={klasse}>{#each NACHARBEIT_KLASSEN as k}<option value={k}>{klassentitel(k)}</option>{/each}</select></label>
      <label>Art<input aria-label="Art (PredefinedType)" list="ifc-arten" bind:value={art} placeholder="optional"/></label>
      <datalist id="ifc-arten">{#each ARTEN[klasse] ?? [] as a}<option value={a}></option>{/each}</datalist>
      <button type="button" class="sekundaer" onclick={klassifizieren}>Klassifizieren</button></div>
    <div class="zeile"><label>Geschoss<select aria-label="Zielgeschoss" bind:value={zielGeschoss}>{#each auszug.geschosse as g (g.global_id)}<option value={g.global_id}>{g.name}</option>{/each}</select></label>
      <button type="button" class="sekundaer" onclick={zuordnen}>Geschoss zuordnen</button></div>
    <div class="zeile"><label>Eigenschaftssatz<input aria-label="Eigenschaftssatz" bind:value={satz}/></label><label>Name<input aria-label="Eigenschaftsname" bind:value={name}/></label></div>
    <div class="zeile"><label>Typ<select aria-label="Werttyp" bind:value={typ}><option value="logisch">Ja/Nein</option><option value="zahl">Zahl</option><option value="text">Text</option></select></label>
      {#if typ === "logisch"}<label>Wert<select aria-label="Eigenschaftswert" bind:value={wert}><option value="true">Ja</option><option value="false">Nein</option></select></label>
      {:else}<label>Wert<input aria-label="Eigenschaftswert" bind:value={wert}/></label>{/if}
      <button type="button" class="sekundaer" onclick={eigenschaft}>Eigenschaft setzen</button></div>
    {#if einzel}
      <div class="zeile"><label>Name<input aria-label="Neuer Name" bind:value={neuName}/></label><label>Langname / Nutzung<input aria-label="Neuer Langname" bind:value={neuLang}/></label>
        <button type="button" class="sekundaer" onclick={benennen}>Benennen</button></div>
    {/if}
    <button type="button" class="sekundaer" onclick={ausblenden}>Ausblenden</button>
  </fieldset>

  {#if einzel?.geometrie}
    <h4>Geometrie</h4>
    <fieldset disabled={!darf || laeuft}>
      <div class="zeile"><label>Unterkante (m)<input aria-label="Unterkante" type="number" step=".01" bind:value={uk}/></label>
        <label>Oberkante (m)<input aria-label="Oberkante" type="number" step=".01" bind:value={ok}/></label>
        <button type="button" class="sekundaer" onclick={hoehe}>Höhe ändern</button></div>
      <p class="meta">{einzel.klasse.startsWith("IFCSLAB") ? "Dicke" : "Höhe"} {laenge(Number(ok) - Number(uk))} · Wandhöhe, Deckendicke oder Brüstung; der Körper wird zur senkrechten Extrusion seines Grundrisses.</p>
      <div class="zeile">
        <button type="button" class="sekundaer" onclick={() => onzeichnen?.(!zeichnen)}>{zeichnen ? "Zeichnen beenden" : "Neuen Umriss zeichnen"}</button>
        {#if zeichnen}
          <button type="button" class="sekundaer" disabled={punkte.length < 3} onclick={umriss}>Umriss übernehmen ({punkte.length} Punkte)</button>
          <button type="button" class="link" onclick={() => onzeichnen?.(true)}>Neu beginnen</button>
        {/if}
      </div>
      {#if zeichnen}<p class="meta" role="status">Eckpunkte nacheinander im Grundriss setzen; der Umriss schließt sich selbst.</p>{/if}
    </fieldset>
  {/if}

  <h4>Räume aus Wänden ({auszug.geschosse.find(g => g.global_id === geschoss)?.name ?? "Geschoss"})</h4>
  <button type="button" class="sekundaer" disabled={!geschoss || laeuft} onclick={() => void vorschlagen()}>Geschlossene Flächen suchen</button>
  {#if vorschlaege.length}
    <ul class="vorschlaege" aria-label="Raumvorschläge">
      {#each vorschlaege as v, i (v.global_id)}
        <li>{flaeche(v.flaeche_m2)}
          {#if v.vorhanden}<span class="meta"> · vorhanden: <button type="button" class="link" onclick={() => onauswahl(v.vorhanden!)}>{elementtitel(auszug.elemente.find(e => e.global_id === v.vorhanden) ?? { name: "Raum", langname: null, klasse: "IFCSPACE", global_id: v.vorhanden! })}</button></span>
          {:else if darf}<input aria-label={`Name des Raums ${i + 1}`} placeholder="z. B. 0.03 Bad" bind:value={raumNamen[v.global_id]}/><button type="button" class="sekundaer" disabled={laeuft} onclick={() => raumBilden(v, i)}>Raum bilden</button>{/if}
        </li>
      {/each}
    </ul>
  {/if}

  <h4>Änderungen ({liste.length})</h4>
  {#if liste.length}
    <ol class="aenderungen" aria-label="Änderungen">
      {#each liste as c, i (i)}<li><span>{beschreibung(c)}</span>{#if darf}<button type="button" class="link" disabled={laeuft} onclick={() => entfernen(i)}>Zurücknehmen</button>{/if}</li>{/each}
    </ol>
  {:else}<p class="meta">Noch keine Nacharbeit.</p>{/if}
  {#if laeuft}<p class="meta" role="status">Wird gespeichert …</p>{/if}
  {#if meldung}<p class="meta" role="status">{meldung}</p>{/if}
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
</section>

<style>
  .nacharbeit fieldset{border:0;padding:0;margin:0;display:flex;flex-direction:column;gap:4px}
  .zeile{display:flex;gap:4px;align-items:flex-end;flex-wrap:wrap}
  .zeile label{display:flex;flex-direction:column;font-size:.8rem;flex:1;min-width:6rem}
  .zeile button{min-height:26px}
  h4{margin:10px 0 4px;font-size:.85rem}
  .vorschlaege,.aenderungen{margin:4px 0;padding-left:18px;font-size:.85rem}
  .vorschlaege li{display:flex;gap:4px;align-items:center;flex-wrap:wrap;margin-bottom:2px}
  .vorschlaege input{width:9rem}
  .aenderungen li{margin-bottom:2px}
  .aenderungen li span{margin-right:6px}
</style>
