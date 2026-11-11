<script lang="ts" module>
  import type {
    AbstandsPlan,
    Regelwerk,
    Regelwerkbezug,
  } from "../generated/plattform";
  export const STICHTAGARTEN = [
    "Eingang Bauantrag",
    "Geplanter Antragseingang",
    "Heute (noch kein Antrag)",
  ];
  export function heute(): string {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }
  /** Gilt die Fassung am Stichtag (ISO-Datum)? Pilotansätze gelten nie. */
  export function giltAm(r: Regelwerk, tag: string): boolean {
    return (
      !!r.fassung.gueltig_ab &&
      r.fassung.gueltig_ab <= tag &&
      (!r.fassung.gueltig_bis || tag <= r.fassung.gueltig_bis)
    );
  }
  /** Am Stichtag geltende Fassung; ohne datierte Fassung der Pilotansatz. */
  export function vorschlag(
    werke: Regelwerk[],
    land: string,
    tag: string,
  ): Regelwerk | undefined {
    const des = werke.filter((r) => r.regeln.land === land);
    return des.some((r) => r.fassung.gueltig_ab)
      ? des.find((r) => giltAm(r, tag))
      : des[0];
  }
  /** Übernimmt Regeln und Bezug; Begründung und Prüfpunkte nur bei derselben Datei. */
  export function uebernehmen(
    p: AbstandsPlan,
    r: Regelwerk,
    stichtag: string,
    stichtag_art: string,
  ) {
    const alt =
      p.regelwerk?.datei === r.datei ? p.regelwerk : undefined;
    // Regelwerke liegen als reaktive Proxys vor; structuredClone scheitert daran.
    p.regeln = $state.snapshot(r.regeln);
    p.regelwerk = {
      datei: r.datei,
      pruefsumme: r.pruefsumme,
      stichtag,
      stichtag_art,
      abweichung: alt?.abweichung ?? "",
      pruefpunkte: alt?.pruefpunkte ?? [],
    } satisfies Regelwerkbezug;
    p.recht.geprueft = false;
  }
  export function datum(t: string | null | undefined): string {
    if (!t) return "offen";
    const [j, m, d] = t.split("-");
    return `${d}.${m}.${j}`;
  }
  export function gueltigkeit(r: Regelwerk): string {
    if (!r.fassung.gueltig_ab) return "Pilotansatz ohne belegte Fassung";
    return r.fassung.gueltig_bis
      ? `gültig ${datum(r.fassung.gueltig_ab)} bis ${datum(r.fassung.gueltig_bis)}`
      : `gültig ab ${datum(r.fassung.gueltig_ab)}`;
  }
</script>

<script lang="ts">
  import { untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { fehlertext } from "../api";
  import { plattformApi } from "../plattformApi";
  import type { AbstandsErgebnis } from "../generated/plattform";
  let {
    plan,
    werke,
    projektId,
    darf,
  }: {
    plan: AbstandsPlan;
    werke: Regelwerk[];
    projektId: string;
    darf: boolean;
  } = $props();
  const land = $derived(plan.regeln.land);
  const desLandes = $derived(werke.filter((r) => r.regeln.land === land));
  let stichtag = $state(""),
    stichtagArt = $state(STICHTAGARTEN[2]),
    wahl = $state(""),
    vergleich = $state<
      { wand: string; alt: number | null; neu: number | null }[] | null
    >(null),
    busy = $state(false),
    fehler = $state("");
  // Eingaben aus dem Plan übernehmen, sobald ein anderer Plan geöffnet wird.
  $effect(() => {
    const b = plan.regelwerk;
    void b?.datei;
    void werke.length;
    untrack(() => {
      stichtag = b?.stichtag ?? heute();
      stichtagArt = b?.stichtag_art ?? STICHTAGARTEN[2];
      wahl = b?.datei ?? vorschlag(werke, land, stichtag)?.datei ?? "";
      vergleich = null;
    });
  });
  // Ist der Antrag bereits eingereicht, richtet sich die Fassung nach dem
  // Eingang; die Akte kennt das Datum, es wird hier nicht erneut gefragt.
  let eingereichtAm = $state<string | null>(null);
  $effect(() => {
    const id = projektId;
    plattformApi
      .uebersicht(id)
      .then((u) => {
        const tage = u.empfaenger
          .flatMap((e) => e.dokumente.map((d) => d.eingereicht_am))
          .sort();
        if (id === projektId) eingereichtAm = tage[0] ?? null;
      })
      .catch(() => (eingereichtAm = null));
  });
  const stichtagWeichtAb = $derived(
    !!eingereichtAm &&
      (stichtag !== eingereichtAm || stichtagArt !== STICHTAGARTEN[0]),
  );
  function stichtagAusAkte() {
    if (!eingereichtAm) return;
    stichtag = eingereichtAm;
    stichtagArt = STICHTAGARTEN[0];
    vergleich = null;
    if (plan.regelwerk) {
      plan.regelwerk.stichtag = stichtag;
      plan.regelwerk.stichtag_art = stichtagArt;
    } else wahl = vorschlag(werke, land, stichtag)?.datei ?? wahl;
  }
  const gewaehlt = $derived(desLandes.find((r) => r.datei === wahl));
  const aktiv = $derived(
    desLandes.find((r) => r.datei === plan.regelwerk?.datei),
  );
  const geltend = $derived(vorschlag(werke, land, stichtag));
  const regelnAktuell = $derived(
    !!aktiv &&
      JSON.stringify(aktiv.regeln) === JSON.stringify(plan.regeln),
  );
  const offenePunkte = $derived(
    aktiv
      ? aktiv.pruefpunkte.filter(
          (q) =>
            !plan.regelwerk?.pruefpunkte?.some(
              (s) => s.absatz === q.absatz && s.erledigt,
            ),
        ).length
      : 0,
  );
  function stand(absatz: number) {
    const b = plan.regelwerk!;
    b.pruefpunkte ??= [];
    let s = b.pruefpunkte.find((s) => s.absatz === absatz);
    if (!s) {
      s = { absatz, erledigt: false, vermerk: "" };
      b.pruefpunkte.push(s);
      s = b.pruefpunkte.at(-1)!;
    }
    return s;
  }
  async function tiefen(p: AbstandsPlan) {
    const r = await invoke<AbstandsErgebnis>("abstandsplan_pruefen", {
      projektId,
      plan: p,
    });
    return new Map(r.waende.map((w) => [w.id, w.tiefe_m ?? null]));
  }
  /** Wechsel bei vorhandenen Wänden nur mit Vorher-Nachher-Vergleich. */
  async function wechseln() {
    if (!gewaehlt) return;
    fehler = "";
    if (!plan.regelwerk || !plan.waende.length || vergleich) {
      uebernehmen(plan, gewaehlt, stichtag, stichtagArt);
      vergleich = null;
      return;
    }
    busy = true;
    try {
      const alt = $state.snapshot(plan) as AbstandsPlan;
      const neu = structuredClone(alt);
      uebernehmen(neu, gewaehlt, stichtag, stichtagArt);
      const [a, n] = await Promise.all([tiefen(alt), tiefen(neu)]);
      vergleich = plan.waende.map((w) => ({
        wand: w.bezeichnung,
        alt: a.get(w.id) ?? null,
        neu: n.get(w.id) ?? null,
      }));
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      busy = false;
    }
  }
  const m = (n: number | null) =>
    n == null
      ? "—"
      : `${n.toLocaleString("de-DE", { maximumFractionDigits: 3 })} m`;
</script>

<section class="fassung" aria-labelledby="fassung-titel">
  <h4 id="fassung-titel">Rechtsgrundlage: Landesbauordnung und Fassung</h4>
  {#if !desLandes.length}
    <p class="hinweisbox">
      Für dieses Bundesland ist keine Regeldatei hinterlegt. Gerechnet wird mit
      dem Regelpaket {plan.regeln.id} · {plan.regeln.version}.
    </p>
  {:else}
    {#if stichtagWeichtAb}<p class="hinweisbox" data-stichtag-hinweis>
        Der Bauantrag ist laut Verfahrensakte am {datum(eingereichtAm)} eingereicht.
        Als Stichtag gilt hier noch „{stichtagArt}“ ({datum(stichtag)}).
        <button
          type="button"
          class="link"
          disabled={!darf}
          onclick={stichtagAusAkte}>Stichtag auf den Antragseingang setzen</button
        >
      </p>{/if}
    <div class="raster">
      <label
        >Stichtag<input
          type="date"
          bind:value={stichtag}
          disabled={!darf}
          onchange={() => {
            vergleich = null;
            if (plan.regelwerk) plan.regelwerk.stichtag = stichtag;
            else wahl = vorschlag(werke, land, stichtag)?.datei ?? wahl;
          }}
        /></label
      ><label
        >Stichtag ist<select
          bind:value={stichtagArt}
          disabled={!darf}
          onchange={() => {
            if (plan.regelwerk) plan.regelwerk.stichtag_art = stichtagArt;
          }}
          >{#each STICHTAGARTEN as a}<option>{a}</option>{/each}</select
        ></label
      ><label
        >Fassung<select
          bind:value={wahl}
          disabled={!darf}
          onchange={() => (vergleich = null)}
          >{#each desLandes as r (r.datei)}<option value={r.datei}
              >{r.fassung.paragraf} · {gueltigkeit(r)}{geltend?.datei ===
              r.datei
                ? " · gilt am Stichtag"
                : ""}</option
            >{/each}</select
        ></label
      >
    </div>
    {#if gewaehlt && (gewaehlt.datei !== plan.regelwerk?.datei || !regelnAktuell)}
      <div class="knoepfe links">
        <button
          type="button"
          disabled={!darf || busy}
          onclick={wechseln}
          >{!plan.regelwerk
            ? "Fassung übernehmen"
            : gewaehlt.datei === plan.regelwerk.datei
              ? "Regeln aus der Regeldatei neu übernehmen"
              : vergleich
                ? "Wechsel bestätigen"
                : "Fassung wechseln (mit Vergleich)"}</button
        >
        {#if vergleich}<button
            type="button"
            class="sekundaer"
            onclick={() => (vergleich = null)}>Abbrechen</button
          >{/if}
      </div>
    {/if}
    {#if !plan.regelwerk}
      <p class="hinweisbox">
        Dieser Plan rechnet noch ohne Regeldatei mit dem Regelpaket {plan.regeln
          .id} · {plan.regeln.version}. Mit „Fassung übernehmen“ werden Fassung,
        Fundstelle, Wortlaut und Prüfpunkte Teil des Nachweises.
      </p>
    {/if}
    {#if vergleich}
      <table>
        <caption>Tiefe je Wand: bisherige und neue Fassung</caption>
        <thead
          ><tr><th>Wand</th><th>Bisher</th><th>Neu</th></tr></thead
        >
        <tbody>
          {#each vergleich as v}<tr class:geaendert={v.alt !== v.neu}
              ><td>{v.wand}</td><td>{m(v.alt)}</td><td>{m(v.neu)}</td></tr
            >{/each}
        </tbody>
      </table>
    {/if}
    {#if fehler}<p role="alert">{fehler}</p>{/if}
    {#if aktiv && plan.regelwerk}
      {@const b = plan.regelwerk}
      {#if !regelnAktuell}<p class="hinweisbox">
          Die Rechenregeln des Plans weichen von der Regeldatei ab (ältere
          Übernahme oder eigenes Regelpaket). „Regeln aus der Regeldatei neu
          übernehmen“ gleicht sie an.
        </p>{/if}
      <dl class="angaben">
        <dt>Gesetz</dt>
        <dd>{aktiv.fassung.gesetz}, {aktiv.fassung.paragraf}</dd>
        <dt>Fassung</dt>
        <dd>{gueltigkeit(aktiv)}</dd>
        <dt>Fundstelle</dt>
        <dd>{aktiv.fassung.fundstelle || "nicht angegeben"}</dd>
        <dt>Quellen</dt>
        <dd>
          {#each aktiv.quellen as q}<div>
              {q.titel} · abgerufen {datum(q.abgerufen)}
            </div>{:else}keine{/each}
        </dd>
        <dt>Fachliche Freigabe</dt>
        <dd>
          {#if aktiv.pruefung.geprueft}Freigegeben von {aktiv.pruefung
              .geprueft_von} am {datum(aktiv.pruefung.geprueft_am)}{:else}<span
              class="warnung">Regelwerk fachlich nicht freigegeben</span
            >{/if}
        </dd>
      </dl>
      <p class="meta">{aktiv.pruefung.vermerk}</p>
      {#if geltend?.datei !== b.datei}
        <label
          >Begründung, warum diese Fassung gilt, obwohl am Stichtag {geltend
            ? `${geltend.fassung.paragraf} · ${gueltigkeit(geltend)}`
            : "keine Fassung"} gilt<span class="pflicht" aria-hidden="true">
            *</span
          ><textarea
            rows="2"
            disabled={!darf}
            bind:value={b.abweichung}
            placeholder="z. B. Antrag vor dem Inkrafttreten gestellt, Übergangsrecht"
          ></textarea></label
        >
      {/if}
      {#if aktiv.fassung.uebergang}<p class="meta">
          Übergangsrecht: {aktiv.fassung.uebergang}
        </p>{/if}
      <details>
        <summary>Wortlaut {aktiv.fassung.paragraf}</summary>
        {#each aktiv.wortlaut as a}<p class="wortlaut">
            ({a.absatz}) {a.text}
          </p>{:else}<p class="meta">Kein Wortlaut hinterlegt.</p>{/each}
      </details>
      {#if aktiv.pruefpunkte.length}
        <h5>
          Nicht automatisch geprüfte Tatbestände ({offenePunkte} von {aktiv
            .pruefpunkte.length} offen)
        </h5>
        <ul class="pruefpunkte">
          {#each aktiv.pruefpunkte as q (q.absatz)}
            {@const st = b.pruefpunkte?.find((s) => s.absatz === q.absatz)}
            <li>
              <label class="haken"
                ><input
                  type="checkbox"
                  disabled={!darf}
                  checked={!!st?.erledigt}
                  onchange={(e) =>
                    (stand(q.absatz).erledigt = e.currentTarget.checked)}
                />Abs. {q.absatz}: {q.frage}</label
              ><input
                aria-label={`Vermerk zu Abs. ${q.absatz}`}
                placeholder="Vermerk (Ergebnis, Beleg)"
                disabled={!darf}
                value={st?.vermerk ?? ""}
                oninput={(e) =>
                  (stand(q.absatz).vermerk = e.currentTarget.value)}
              />
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  {/if}
</section>

<style>
  .fassung {
    margin-block: 1rem;
  }
  .angaben {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 0.25rem 1rem;
  }
  .angaben dt {
    font-weight: 600;
  }
  .angaben dd {
    margin: 0;
  }
  .warnung {
    color: var(--warning-text);
    font-weight: 600;
  }
  .wortlaut {
    max-width: 75ch;
  }
  .pruefpunkte {
    list-style: none;
    padding: 0;
    display: grid;
    gap: 0.5rem;
  }
  .pruefpunkte li {
    display: grid;
    gap: 0.25rem;
  }
  .haken {
    display: flex;
    gap: 0.5rem;
    align-items: flex-start;
  }
  tr.geaendert td {
    font-weight: 600;
  }
</style>
