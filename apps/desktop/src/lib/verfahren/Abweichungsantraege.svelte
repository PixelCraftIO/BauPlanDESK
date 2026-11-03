<script lang="ts">
  import { datumDe } from "../api";
  import type {
    Abweichung,
    Abweichungsabgleich,
    Abweichungsart,
    Abweichungsbedarf,
    Bescheid,
  } from "../generated/plattform";
  import { fehlertext, type DocumentVersion } from "../api";
  import Dokumentfeld from "./Dokumentfeld.svelte";
  import Dokumentliste from "./Dokumentliste.svelte";
  import { ABWEICHUNGSART, verfahrenApi } from "./api";
  let {
    eintraege = $bindable(),
    bescheide,
    dateien,
    projektId,
    verfahrenId,
    alt = [],
  }: {
    eintraege: Abweichung[];
    bescheide: Bescheid[];
    dateien: DocumentVersion[];
    projektId: string;
    verfahrenId: string | null;
    alt?: string[];
  } = $props();
  // Ältere Akten führen noch keine Abweichungen.
  eintraege ??= [];
  let abgleich = $state<Abweichungsabgleich | null>(null),
    fehler = $state(""),
    laedt = $state(false),
    verzicht = $state<Record<string, string>>({});
  async function pruefen() {
    if (!verfahrenId) return;
    laedt = true;
    fehler = "";
    try {
      abgleich = await verfahrenApi.abweichungen(projektId, verfahrenId);
    } catch (e) {
      fehler = fehlertext(e);
    } finally {
      laedt = false;
    }
  }
  $effect(() => {
    if (verfahrenId) void pruefen();
  });
  const offen = $derived(
    (abgleich?.bedarf ?? []).filter(
      (b) => !eintraege.some((e) => e.herkunft === b.schluessel),
    ),
  );
  function aus(b: Abweichungsbedarf, entfallen?: string): Abweichung {
    return {
      id: crypto.randomUUID(),
      art: b.art,
      vorschrift: b.vorschrift,
      rechtsgrundlage: b.rechtsgrundlage,
      gegenstand: b.gegenstand,
      soll: b.soll,
      ist: b.ist,
      begruendung: "",
      herkunft: b.schluessel,
      entfallen,
    };
  }
  function beantragen(b: Abweichungsbedarf) {
    eintraege = [...eintraege, aus(b)];
  }
  function nichtErforderlich(b: Abweichungsbedarf) {
    const grund = verzicht[b.schluessel]?.trim();
    if (!grund) return;
    eintraege = [...eintraege, aus(b, grund)];
    delete verzicht[b.schluessel];
  }
  function manuell() {
    eintraege = [
      ...eintraege,
      {
        id: crypto.randomUUID(),
        art: "abweichung",
        vorschrift: "",
        rechtsgrundlage: "",
        gegenstand: "",
        soll: "",
        ist: "",
        begruendung: "",
      },
    ];
  }
  function nachbar(e: Abweichung) {
    e.nachbarn = [
      ...(e.nachbarn ?? []),
      {
        id: crypto.randomUUID(),
        name: "",
        flurstueck: "",
        zugestimmt: null,
        datum: null,
        beleg: null,
      },
    ];
  }
  const entscheidungen = (id: string) =>
    bescheide.flatMap((b) =>
      (b.abweichungsentscheidungen ?? [])
        .filter((d) => d.abweichung === id)
        .map((d) => ({ ...d, datum: b.datum })),
    );
  const zustand = (e: Abweichung) =>
    abgleich?.ohne_bedarf.includes(e.id)
      ? "Im aktuellen Planungsstand besteht keine Unterschreitung mehr."
      : abgleich?.geaendert.includes(e.id)
        ? "Soll oder Ist hat sich geändert; Angaben und Begründung prüfen."
        : "";
  function bedarfUebernehmen(e: Abweichung) {
    const b = abgleich?.bedarf.find((b) => b.schluessel === e.herkunft);
    if (!b) return;
    e.soll = b.soll;
    e.ist = b.ist;
  }
</script>

<h3>Abweichungen, Ausnahmen und Befreiungen</h3>
<p class="meta">
  Jede Unterschreitung im Planungsstand braucht vor der Abgabe einen Eintrag:
  beantragt mit Vorschrift, Rechtsgrundlage und Begründung oder begründet als
  nicht erforderlich. Ohne Eintrag hält die Anwendung keinen Abgabestand fest.
  Ob eine Abweichung zugelassen werden kann, entscheidet die Behörde.
</p>
<button
  type="button"
  class="sekundaer"
  disabled={laedt || !verfahrenId}
  onclick={pruefen}>Bedarf aus dem Planungsstand prüfen</button
>
{#if !verfahrenId}<p class="meta">Die Akte zuerst speichern.</p>{/if}
{#if fehler}<p class="fehler" role="alert">{fehler}</p>{/if}
{#each abgleich?.hinweise ?? [] as h}<p class="meta">{h}</p>{/each}
{#if offen.length}<h4>Ohne Eintrag ({offen.length})</h4>
  <ul class="abweichungsbedarf">
    {#each offen as b (b.schluessel)}<li data-bedarf={b.schluessel}>
        <strong>{ABWEICHUNGSART[b.art]}: {b.gegenstand}</strong>
        <span class="meta"
          >{b.vorschrift} · Soll {b.soll} · Ist {b.ist} · {b.quelle}</span
        >
        <button type="button" class="sekundaer" onclick={() => beantragen(b)}
          >Als Antrag aufnehmen</button
        >
        <label
          >Begründung, warum nicht erforderlich<input
            bind:value={verzicht[b.schluessel]}
            maxlength="2000"
          /></label
        ><button
          type="button"
          class="link"
          disabled={!verzicht[b.schluessel]?.trim()}
          onclick={() => nichtErforderlich(b)}
          >Als nicht erforderlich führen</button
        >
      </li>{/each}
  </ul>
{:else if abgleich}<p class="meta">
    Jeder ermittelte Bedarf hat einen Eintrag.
  </p>{/if}
<button type="button" class="sekundaer" onclick={manuell}
  >Weitere Abweichung erfassen</button
>
{#each eintraege as e (e.id)}<fieldset
    data-uebersicht-eintrag={e.id}
    class:entfallen={!!e.entfallen}
  >
    <legend
      >{ABWEICHUNGSART[e.art]} · {e.gegenstand || "ohne Gegenstand"}{e.entfallen
        ? " · nicht erforderlich"
        : ""}</legend
    >
    {#if !alt.includes(e.id)}<button
        type="button"
        class="link"
        onclick={() => (eintraege = eintraege.filter((n) => n.id !== e.id))}
        >Noch nicht gespeicherten Eintrag verwerfen</button
      >{/if}
    {#if zustand(e)}<p class="hinweis">
        {zustand(e)}
        {#if abgleich?.geaendert.includes(e.id)}<button
            type="button"
            class="link"
            onclick={() => bedarfUebernehmen(e)}
            >Soll und Ist aus dem Planungsstand übernehmen</button
          >{/if}
      </p>{/if}
    <div class="raster">
      <label
        >Art<select bind:value={e.art}
          >{#each Object.entries(ABWEICHUNGSART) as [wert, titel]}<option
              value={wert as Abweichungsart}>{titel}</option
            >{/each}</select
        ></label
      ><label
        >Gegenstand<input bind:value={e.gegenstand} maxlength="1000" /></label
      ><label
        >Betroffene Vorschrift<input
          bind:value={e.vorschrift}
          maxlength="1000"
          placeholder="z. B. § 6 Abs. 5 BauO NRW"
        /></label
      ><label
        >Rechtsgrundlage der Zulassung<input
          bind:value={e.rechtsgrundlage}
          maxlength="1000"
          placeholder="z. B. § 69 BauO NRW, § 31 Abs. 2 BauGB"
        /></label
      ><label>Soll<input bind:value={e.soll} maxlength="1000" /></label><label
        >Ist<input bind:value={e.ist} maxlength="1000" /></label
      ><label
        >XBau-Code (optional)<input
          value={e.xbau_code ?? ""}
          maxlength="100"
          oninput={(ev) => (e.xbau_code = ev.currentTarget.value || undefined)}
        /></label
      >
    </div>
    <label
      >Begründung<textarea
        bind:value={e.begruendung}
        rows="5"
        maxlength="32000"
        placeholder="Zweck der Anforderung, Schutzziel, öffentliche und nachbarliche Belange"
      ></textarea></label
    >
    <Dokumentliste
      {dateien}
      bind:werte={() => e.dokumente ?? [], (w) => (e.dokumente = w)}
      label="Begründende Unterlagen"
    />
    <h4>Angrenzer</h4>
    {#each e.nachbarn ?? [] as n (n.id)}<div class="raster">
        <label>Name<input bind:value={n.name} maxlength="500" /></label><label
          >Flurstück<input bind:value={n.flurstueck} maxlength="500" /></label
        ><label
          >Erklärung<select
            value={n.zugestimmt === null ? "" : String(n.zugestimmt)}
            onchange={(ev) =>
              (n.zugestimmt =
                ev.currentTarget.value === ""
                  ? null
                  : ev.currentTarget.value === "true")}
            ><option value="">Angefragt</option><option value="true"
              >Zugestimmt</option
            ><option value="false">Nicht zugestimmt</option></select
          ></label
        ><label
          >Datum<input
            type="date"
            value={n.datum ?? ""}
            onchange={(ev) => (n.datum = ev.currentTarget.value || null)}
          /></label
        ><Dokumentfeld {dateien} bind:wert={n.beleg} label="Unterschrift" />
      </div>{/each}
    <button type="button" class="link" onclick={() => nachbar(e)}
      >Angrenzer ergänzen</button
    >
    <label class="haken"
      ><input
        type="checkbox"
        checked={!!e.entfallen}
        onchange={(ev) =>
          (e.entfallen = ev.currentTarget.checked ? " " : undefined)}
      />Nicht (mehr) erforderlich</label
    >
    {#if e.entfallen !== undefined && e.entfallen !== null}<label
        >Begründung<input
          value={e.entfallen.trim()}
          oninput={(ev) => (e.entfallen = ev.currentTarget.value || " ")}
          maxlength="2000"
        /></label
      >{/if}
    {#each entscheidungen(e.id) as d}<p class="meta">
        Bescheid vom {datumDe(d.datum)}: {d.zugelassen ? "zugelassen" : "nicht zugelassen"}{d.nebenbestimmung
          ? ` · Nebenbestimmung: ${d.nebenbestimmung}`
          : ""}
      </p>{/each}
  </fieldset>{/each}

<style>
  .abweichungsbedarf li {
    display: grid;
    gap: 0.25rem;
    margin-bottom: 0.75rem;
  }
  fieldset.entfallen {
    opacity: 0.75;
  }
</style>
