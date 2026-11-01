<script lang="ts">
  import { api, fehlertext, landName, type Benutzer } from "./api";
  import type {
    Buerocockpit,
    CockpitTermin,
    Friststufe,
    UebersichtsZiel,
  } from "./generated/plattform";
  let {
    ich,
    aktualisieren = 0,
    onoeffnen,
  }: {
    ich: Benutzer | null;
    /** Neu laden, wenn sich der Wert ändert (z. B. nach dem Schließen einer Akte). */
    aktualisieren?: number;
    onoeffnen: (projektId: string, ziel?: UebersichtsZiel) => void;
  } = $props();
  let daten = $state<Buerocockpit | null>(null),
    benutzer = $state<Benutzer[]>([]),
    fehler = $state<string | null>(null),
    laedt = $state(false),
    nurMeine = $state(false),
    zeitraum = $state<"dringend" | "monat" | "alle">("monat"),
    suche = $state(""),
    offen = $state(true);
  const STUFEN: Record<Friststufe, string> = {
    ueberfaellig: "Überfällig",
    diese_woche: "Diese Woche",
    monat: "30 Tage",
    spaeter: "Später",
  };
  const ENTSCHEIDUNG: Record<string, string> = {
    genehmigt: "Genehmigt",
    teilweise_genehmigt: "Teilweise genehmigt",
    abgelehnt: "Abgelehnt",
    sonstiges: "Sonstige Entscheidung",
  };
  const datum = (d: string | null) =>
    d ? new Date(d + "T12:00:00").toLocaleDateString("de-DE") : "—";
  const name = (id: string | null) =>
    id ? (benutzer.find((b) => b.id === id)?.anzeigename ?? "unbekannt") : "—";
  const imZeitraum = (t: CockpitTermin) =>
    zeitraum === "alle" ||
    t.dringlichkeit === "ueberfaellig" ||
    t.dringlichkeit === "diese_woche" ||
    (zeitraum === "monat" && t.dringlichkeit === "monat");
  const passt = (t: CockpitTermin) =>
    (!nurMeine || t.verantwortlich === ich?.id) &&
    imZeitraum(t) &&
    (t.projekt + " " + t.titel + " " + t.verfahrensbezeichnung + " " + t.art_bezeichnung)
      .toLowerCase()
      .includes(suche.trim().toLowerCase());
  const termine = $derived(daten?.termine.filter(passt) ?? []);
  const zaehler = $derived(
    (["ueberfaellig", "diese_woche", "monat"] as Friststufe[]).map((d) => ({
      d,
      n:
        daten?.termine.filter(
          (t) =>
            t.dringlichkeit === d &&
            (!nurMeine || t.verantwortlich === ich?.id),
        ).length ?? 0,
    })),
  );
  const projekte = $derived(
    (daten?.projekte ?? []).filter(
      (p) =>
        p.verfahren.length > 0 &&
        (!nurMeine ||
          p.verfahren.some((v) => v.verantwortlich === ich?.id) ||
          daten?.termine.some(
            (t) => t.projekt_id === p.projekt_id && t.verantwortlich === ich?.id,
          )) &&
        (p.name + " " + p.verfahren.map((v) => v.aktenzeichen).join(" "))
          .toLowerCase()
          .includes(suche.trim().toLowerCase()),
    ),
  );
  const ablehnungen = $derived(
    (daten?.projekte ?? []).flatMap((p) =>
      p.verfahren
        .filter((v) => v.ablehnung_offen)
        .map((v) => ({ p, v })),
    ),
  );
  async function laden() {
    laedt = true;
    fehler = null;
    try {
      const [c, b] = await Promise.all([
        api.bueroCockpit(),
        api.benutzerAuflisten().catch(() => [] as Benutzer[]),
      ]);
      daten = c;
      benutzer = b;
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laedt = false;
    }
  }
  function zuTermin(t: CockpitTermin) {
    onoeffnen(t.projekt_id, {
      bereich: "verfahren",
      objekt: t.verfahren,
      register: t.register,
      eintrag: t.objekt,
    });
  }
  $effect(() => {
    void aktualisieren;
    void laden();
  });
</script>

<details class="cockpit" bind:open={offen}>
  <summary>
    <h2>Büro-Cockpit</h2>
    <span class="meta"
      >Fristen und Verfahrensstände aller Projekte{daten
        ? ` · Stand ${datum(daten.stichtag)}`
        : ""}</span
    >
  </summary>
  <div class="cockpit-filter">
    {#each zaehler as z}<button
        type="button"
        class="kennzahl"
        class:ueberfaellig={z.d === "ueberfaellig" && z.n > 0}
        onclick={() => (zeitraum = z.d === "monat" ? "monat" : "dringend")}
        ><strong>{z.n}</strong> {STUFEN[z.d]}</button
      >{/each}
    <label
      >Zuständigkeit<select bind:value={nurMeine}
        ><option value={false}>Alle</option><option value={true}
          >Nur meine</option
        ></select
      ></label
    >
    <label
      >Zeitraum<select bind:value={zeitraum}
        ><option value="dringend">Überfällig und diese Woche</option><option
          value="monat">Nächste 30 Tage</option
        ><option value="alle">Alle laufenden</option></select
      ></label
    >
    <label
      >Suchen<input
        type="search"
        bind:value={suche}
        placeholder="Projekt, Frist, Aktenzeichen"
      /></label
    >
    <button class="sekundaer" onclick={laden} disabled={laedt}
      >{laedt ? "Lädt …" : "Aktualisieren"}</button
    >
  </div>
  {#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
  {#each daten?.fehler ?? [] as f}<p class="fehlertext" role="alert">
      {f.projekt}: Verfahrensakten nicht lesbar ({f.meldung})
    </p>{/each}
  {#if ablehnungen.length}<div class="hinweisbox" role="status">
      <strong>Ablehnung ohne Folgeverfahren:</strong>
      {#each ablehnungen as { p, v }, i}{i ? ", " : " "}<button
          type="button"
          class="link"
          onclick={() =>
            onoeffnen(p.projekt_id, {
              bereich: "verfahren",
              objekt: v.id,
              register: "bescheide",
              eintrag: null,
            })}>{p.name} · {v.bezeichnung}</button
        >{/each}. In der Verfahrensakte stehen Widerspruch, Klage oder geänderter
      Antrag mit Fristen zur Auswahl.
    </div>{/if}
  <h3>Fristen und Wiedervorlagen</h3>
  <div class="tabellen-scroll">
    <table class="ergebnistabelle cockpit-termine">
      <thead
        ><tr
          ><th>Datum</th><th>Stand</th><th>Projekt</th><th>Verfahren</th><th
            >Art</th
          ><th>Betreff</th><th>Zuständig</th></tr
        ></thead
      >
      <tbody>
        {#each termine as t (t.projekt_id + t.verfahren + t.objekt + t.art + t.datum)}<tr
            class:zeile-ueberfaellig={t.dringlichkeit === "ueberfaellig"}
            ><td class="zahl">{datum(t.datum)}</td><td
              ><span class="stufe stufe-{t.dringlichkeit}"
                >{STUFEN[t.dringlichkeit]}</span
              ></td
            ><td>{t.projekt}</td><td>{t.verfahrensbezeichnung}</td><td
              >{t.art_bezeichnung}</td
            ><td
              ><button
                type="button"
                class="link"
                title={t.hinweis || "In der Verfahrensakte öffnen"}
                onclick={() => zuTermin(t)}>{t.titel}</button
              ></td
            ><td>{name(t.verantwortlich)}</td></tr
          >{:else}<tr
            ><td colspan="7" class="meta"
              >{laedt
                ? "Wird geladen …"
                : "Keine laufenden Fristen in diesem Zeitraum."}</td
            ></tr
          >{/each}
      </tbody>
    </table>
  </div>
  <h3>Laufende Verfahren</h3>
  <div class="tabellen-scroll">
    <table class="ergebnistabelle">
      <thead
        ><tr
          ><th>Projekt</th><th>Verfahren</th><th>Stand</th><th
            >Letzte Abgabe</th
          ><th>Offene Nachforderungen</th><th>Letzter Bescheid</th><th
            >Nächste Frist</th
          ></tr
        ></thead
      >
      <tbody>
        {#each projekte as p (p.projekt_id)}
          {#each p.verfahren as v, i (v.id)}<tr
              >{#if i === 0}<td rowspan={p.verfahren.length}
                  ><button
                    type="button"
                    class="link"
                    onclick={() => onoeffnen(p.projekt_id)}>{p.name}</button
                  ><br /><span class="meta">{landName(p.bundesland)}</span
                  >{#if p.ueberfaellig}<br /><span class="stufe stufe-ueberfaellig"
                      >{p.ueberfaellig} überfällig</span
                    >{/if}</td
                >{/if}<td
                ><button
                  type="button"
                  class="link"
                  onclick={() =>
                    onoeffnen(p.projekt_id, {
                      bereich: "verfahren",
                      objekt: v.id,
                      register: "stamm",
                      eintrag: null,
                    })}>{v.bezeichnung}</button
                >{#if v.aktenzeichen}<br /><span class="meta"
                    >Az. {v.aktenzeichen}{v.behoerde
                      ? ` · ${v.behoerde}`
                      : ""}</span
                  >{/if}</td
              ><td>{v.status_bezeichnung}</td><td class="zahl"
                >{datum(v.letzte_abgabe)}</td
              ><td class="zahl">{v.offene_punkte || "—"}</td><td
                >{v.letzter_bescheid
                  ? `${ENTSCHEIDUNG[v.letzter_bescheid.entscheidung]} · ${datum(v.letzter_bescheid.datum)}`
                  : "—"}</td
              >{#if i === 0}<td class="zahl" rowspan={p.verfahren.length}
                  >{datum(p.naechste_frist)}</td
                >{/if}</tr
            >{/each}
        {:else}<tr
            ><td colspan="7" class="meta"
              >{laedt ? "Wird geladen …" : "Keine Projekte mit Verfahrensakte."}</td
            ></tr
          >{/each}
      </tbody>
    </table>
  </div>
</details>

<style>
  .cockpit {
    margin-bottom: var(--space-4);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    background: var(--panel);
  }
  .cockpit summary {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    cursor: pointer;
  }
  .cockpit summary h2 {
    margin: 0;
    display: inline;
  }
  .cockpit h3 {
    margin: var(--space-3) 0 var(--space-1);
    font-size: 0.95rem;
  }
  .cockpit-filter {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .kennzahl {
    background: var(--leiste);
    color: var(--fg);
    border: 1px solid var(--border);
  }
  .kennzahl.ueberfaellig {
    background: var(--err-bg);
    color: var(--error-text);
    border-color: var(--error-text);
  }
  .stufe {
    display: inline-block;
    padding: 0 0.4rem;
    border: 1px solid var(--border);
    font-size: 0.9em;
    white-space: nowrap;
  }
  .stufe-ueberfaellig {
    background: var(--err-bg);
    color: var(--error-text);
    border-color: var(--error-text);
  }
  .stufe-diese_woche {
    background: var(--warn-bg);
    color: var(--warning-text);
    border-color: var(--warning-text);
  }
  .stufe-monat {
    background: var(--accent-soft);
  }
  .zeile-ueberfaellig td {
    background: var(--err-bg);
  }
</style>
