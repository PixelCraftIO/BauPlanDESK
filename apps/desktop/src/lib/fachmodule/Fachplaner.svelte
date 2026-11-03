<script lang="ts">
  import { onMount, untrack } from "svelte";
  import {
    api,
    fehlertext,
    type Project,
    type Benutzer,
    type DocumentVersion,
  } from "../api";
  import type {
    Fachdisziplin,
    Fachplanerlieferung,
    Lieferstatus,
    Verfahrensakte,
  } from "../generated/plattform";
  import { FachSitzung } from "../FachSitzung.svelte";
  import Fachliste from "../Fachliste.svelte";
  import Dokumentliste from "../verfahren/Dokumentliste.svelte";
  import { plattformApi } from "../plattformApi";
  import { ansichtsAbfrage } from "../ansichtsAbfrage.svelte";
  let {
    projekt,
    benutzerId,
    darf,
  }: { projekt: Project; benutzerId: string; darf: boolean } = $props();
  const DISZIPLINEN: [Fachdisziplin, string, string[]][] = [
    [
      "tragwerksplanung",
      "Tragwerksplanung / Statik",
      ["Standsicherheitsnachweis", "Positionsplan", "Prüfstatik (falls gefordert)"],
    ],
    ["brandschutz", "Brandschutz", ["Brandschutznachweis / -konzept"]],
    ["waermeschutz", "Wärmeschutz / GEG", ["GEG-Nachweis", "Energieausweis (Bedarf)"]],
    ["schallschutz", "Schallschutz", ["Schallschutznachweis"]],
    [
      "vermessung",
      "Vermessung / Lageplan",
      ["Amtlicher Lageplan", "Absteckung / Höhenaufnahme"],
    ],
    ["entwaesserung", "Entwässerung", ["Entwässerungsplan", "Überflutungsnachweis"]],
    ["baugrund", "Baugrund / Bodengutachten", ["Baugrundgutachten"]],
    ["haustechnik", "Haustechnik", ["Lüftungskonzept"]],
    ["freianlagen", "Freianlagen", ["Freiflächengestaltungsplan"]],
    ["sonstiges", "Sonstiges", []],
  ];
  const STATUS: [Lieferstatus, string][] = [
    ["offen", "Offen"],
    ["angefragt", "Angefragt"],
    ["teilweise", "Teilweise geliefert"],
    ["geliefert", "Geliefert, Prüfung offen"],
    ["geprueft", "Geprüft"],
    ["entfallen", "Entfallen"],
  ];
  const disziplinText = (d: Fachdisziplin) =>
    DISZIPLINEN.find(([k]) => k === d)?.[1] ?? d;
  const statusText = (s: Lieferstatus) => STATUS.find(([k]) => k === s)?.[1] ?? s;
  const datum = (d: string) =>
    new Date(d + "T12:00:00").toLocaleDateString("de-DE");
  const heute = () => {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  };
  const s = untrack(
    () =>
      new FachSitzung<Fachplanerlieferung>(
        projekt.id,
        "fachplanerlieferung",
        benutzerId,
        () => ({
          disziplin: "tragwerksplanung",
          fachplaner: "",
          kontakt: "",
          positionen: [],
          angefragt_am: heute(),
          faellig: null,
          status: "angefragt",
          verantwortlich: benutzerId || null,
          dokumente: [],
          verfahren: null,
          bemerkung: "",
        }),
        (l) =>
          `${disziplinText(l.disziplin)} · ${l.fachplaner || "ohne Namen"} · ${statusText(l.status)}${l.faellig && ["offen", "angefragt", "teilweise"].includes(l.status) ? ` · bis ${datum(l.faellig)}` : ""}`,
      ),
  );
  let benutzer = $state<Benutzer[]>([]),
    dateien = $state<DocumentVersion[]>([]),
    verfahren = $state<{ id: string; bezeichnung: string }[]>([]),
    neuePosition = $state("");
  async function dateienLaden() {
    try {
      dateien = await api.dokumente(projekt.id);
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  }
  ansichtsAbfrage(() => projekt.id, ["dokumente"], dateienLaden);
  onMount(async () => {
    try {
      const [b, v] = await Promise.all([
        api.benutzerAuflisten(),
        plattformApi.liste<Verfahrensakte>(projekt.id, "verfahren"),
      ]);
      benutzer = b;
      verfahren = v.map((f) => ({ id: f.objekt, bezeichnung: f.daten.bezeichnung }));
    } catch (e) {
      s.fehler = fehlertext(e);
    }
  });
  function vorschlaege(l: Fachplanerlieferung) {
    const liste = DISZIPLINEN.find(([k]) => k === l.disziplin)?.[2] ?? [];
    for (const b of liste)
      if (!l.positionen.some((p) => p.bezeichnung === b))
        l.positionen.push({ bezeichnung: b, geliefert: false });
  }
  function hinzufuegen(l: Fachplanerlieferung) {
    const b = neuePosition.trim();
    if (!b) return;
    l.positionen.push({ bezeichnung: b, geliefert: false });
    neuePosition = "";
  }
</script>

<Fachliste titel="Fachplaner und Lieferungen" {s} bereich="fachplaner" {darf}>
  {#snippet children(l)}<p class="meta">
      Was bei Statik, Brandschutz, Wärmeschutz, Vermessung und weiteren
      Fachplanern angefordert ist. Ausstehende Lieferungen mit Termin erscheinen
      in der Projektübersicht, im Büro-Cockpit, im Kalender-Abo und in der
      Einreichungsprüfung.
    </p>
    <div class="raster">
      <label
        >Disziplin<select bind:value={l.disziplin}
          >{#each DISZIPLINEN as [k, t]}<option value={k}>{t}</option>{/each}</select
        ></label
      ><label
        >Fachplaner<span class="pflicht" aria-hidden="true"> *</span><input
          aria-label="Fachplaner"
          bind:value={l.fachplaner}
          required
          maxlength="300"
          placeholder="z. B. Ingenieurbüro Schmidt"
        /></label
      ><label
        >Kontakt<input
          bind:value={l.kontakt}
          maxlength="1000"
          placeholder="Ansprechpartner, Telefon, E-Mail"
        /></label
      ><label
        >Stand<select bind:value={l.status}
          >{#each STATUS as [k, t]}<option value={k}>{t}</option>{/each}</select
        ></label
      ><label>Angefragt am<input type="date" bind:value={l.angefragt_am} /></label
      ><label
        >Liefertermin<input
          type="date"
          bind:value={l.faellig}
          min={l.angefragt_am ?? undefined}
        /></label
      ><label
        >Nachhalten durch<select bind:value={l.verantwortlich}
          ><option value={null}>Noch nicht zugeordnet</option
          >{#each benutzer.filter((b) => b.aktiv) as b (b.id)}<option value={b.id}
              >{b.anzeigename}</option
            >{/each}</select
        ></label
      ><label
        >Für Verfahren<select bind:value={l.verfahren}
          ><option value={null}>Alle Verfahren</option
          >{#each verfahren as v (v.id)}<option value={v.id}>{v.bezeichnung}</option
            >{/each}</select
        ></label
      >
    </div>
    <fieldset>
      <legend>Angeforderte Unterlagen</legend>
      {#if l.positionen.length}
        <ul class="positionen">
          {#each l.positionen as p, i (i)}
            <li>
              <label class="checkbox"
                ><input type="checkbox" bind:checked={p.geliefert} />{p.bezeichnung}</label
              >
              <button
                type="button"
                class="link"
                aria-label="{p.bezeichnung} entfernen"
                onclick={() => l.positionen.splice(i, 1)}>Entfernen</button
              >
            </li>
          {/each}
        </ul>
      {:else}<p class="meta">Noch keine Unterlagen angefordert.</p>{/if}
      <div class="zeile">
        <input
          aria-label="Weitere Unterlage"
          bind:value={neuePosition}
          maxlength="300"
          placeholder="Weitere Unterlage"
          onkeydown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              hinzufuegen(l);
            }
          }}
        /><button type="button" class="sekundaer" onclick={() => hinzufuegen(l)}
          >Hinzufügen</button
        ><button type="button" class="sekundaer" onclick={() => vorschlaege(l)}
          >Übliche Unterlagen vorschlagen</button
        >
      </div>
    </fieldset>
    <Dokumentliste
      {dateien}
      bind:werte={l.dokumente}
      label="Gelieferte Unterlagen (Projektdateien)"
    />
    <label
      >Bemerkung<textarea bind:value={l.bemerkung} maxlength="10000" rows="3"
      ></textarea></label
    >{/snippet}
</Fachliste>

<style>
  .positionen {
    list-style: none;
    padding: 0;
    margin: 0 0 var(--space-2);
  }
  .positionen li {
    display: flex;
    gap: var(--space-2);
    align-items: baseline;
  }
  .zeile {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
</style>
