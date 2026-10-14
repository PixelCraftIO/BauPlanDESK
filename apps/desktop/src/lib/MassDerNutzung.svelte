<script lang="ts">
  import Flurstuecke from "./Flurstuecke.svelte";
  import GrundstueckFormular from "./GrundstueckFormular.svelte";
  import { regelwerk, REGELWERKE } from "./baunvo";
  import FlaechenFormular from "./FlaechenFormular.svelte";
  import { untrack } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { api, fehlertext, GEBIETE, type NutzungsEingabe, type NutzungsPruefung, type Project } from "./api";

  let { projekt, darf, onprojekt, onrechtsgrundlagen }: { projekt: Project; darf: boolean; onprojekt: (p: Project) => void; onrechtsgrundlagen: () => void } = $props();
  const initial = (): NutzungsEingabe => structuredClone($state.snapshot(projekt.planungsrecht.berechnung ?? {
    mass: { grundstuecksflaeche_m2: projekt.grundstuecksflaeche_m2 ?? 0, grundflaeche_m2: 0, grundflaeche_nebenanlagen_m2: 0, geschossflaeche_m2: 0, baumasse_m3: null },
    aufstellung: { bau_nvo: "2017", bauteile: [], geschosse: [], nichtvollgeschoss_faktor: regelwerk("2017").nichtvollgeschoss_faktor, nichtvollgeschoss_festsetzung: "", grundstueck: { basisflaeche_m2: projekt.grundstuecksflaeche_m2 ?? 0, nicht_massgebend_m2: 0, abzug_begruendung: "", gemeinschaftsanteile: [] } },
    vollgeschosse: null, gebiet: projekt.planungsrecht.festgesetzte_werte.baugebiet ?? "WA",
  }));
  let mehrere = $state(untrack(() => !!projekt.planungsrecht.baugrundstuecke || !projekt.planungsrecht.berechnung));
  let flurstueckeEditor: Flurstuecke | undefined;
  let letzteProjektflaeche = $state(untrack(() => projekt.grundstuecksflaeche_m2));
  let gespeichert = $state(untrack(initial));
  let eingabe = $state<NutzungsEingabe>(untrack(initial));
  let ergebnis = $state<NutzungsPruefung | null>(null);
  let ergebnisStand = $state("");
  let formular: HTMLFormElement;
  let laeuft = $state(false);
  let fehler = $state<string | null>(null);
  let hinweis = $state<string | null>(null);
  const geaendert = $derived(JSON.stringify(eingabe) !== JSON.stringify(gespeichert));
  const fingerprint = $derived(JSON.stringify([eingabe, projekt.version]));
  const aktuell = $derived(ergebnis && ergebnisStand === fingerprint);
  const mitPlan = $derived(!["kein", "ungeklaert"].includes(projekt.planungsrecht.bebauungsplan_art));
  const festgelegtesGebiet = $derived(mitPlan ? projekt.planungsrecht.festgesetzte_werte.baugebiet : null);
  const historisch = $derived(!!eingabe.aufstellung && !regelwerk(eingabe.aufstellung.bau_nvo).grz_ii);
  const z = (n: number | null) => n == null ? "–" : n.toLocaleString("de-DE", { maximumFractionDigits: 3 });
  const differenz = (n: number | null) => n == null ? "–" : n.toLocaleString("de-DE", { maximumFractionDigits: 6 });
  const titel: Record<string,string> = { grz: "GRZ – Hauptanlagen", grz_gesamt: "GRZ einschließlich Nebenanlagen", gfz: "GFZ", bmz: "BMZ", grundflaeche: "Grundfläche (GR, m²)", geschossflaeche: "Geschossfläche (GF, m²)", baufensterflaeche: "Flächenvergleich Baufenster (m²)", vollgeschosse: "Vollgeschosse (Z)" };
  const quellen = { festsetzung: "Bebauungsplan", orientierungswert: "Orientierungswert § 17 BauNVO", bau_nvo19: "§ 19 Abs. 4 aus GRZ", keine: "Nicht festgelegt" };

  $effect(() => {
    const p = projekt;
    untrack(() => {
      if (!geaendert && p.planungsrecht.berechnung) {
        gespeichert = structuredClone($state.snapshot(p.planungsrecht.berechnung));
        eingabe = structuredClone($state.snapshot(gespeichert));
      }
    });
  });

  $effect(() => {
    const flaeche = projekt.grundstuecksflaeche_m2;
    untrack(() => {
      if (flaeche !== letzteProjektflaeche) {
        if (eingabe.aufstellung?.grundstueck) {
          if (eingabe.aufstellung.grundstueck.basisflaeche_m2 === (letzteProjektflaeche ?? 0)) eingabe.aufstellung.grundstueck.basisflaeche_m2 = flaeche ?? 0;
        } else if (eingabe.mass.grundstuecksflaeche_m2 === (letzteProjektflaeche ?? 0)) eingabe.mass.grundstuecksflaeche_m2 = flaeche ?? 0;
        letzteProjektflaeche = flaeche;
      }
    });
  });

  export async function verwerfenErlaubt(): Promise<boolean> {
    if (flurstueckeEditor && !(await flurstueckeEditor.verwerfenErlaubt())) return false;
    return !geaendert || await ask("Die Berechnungseingaben wurden nicht gespeichert. Verwerfen?", { title: "Maß der Nutzung", kind: "warning", okLabel: "Verwerfen", cancelLabel: "Abbrechen" });
  }

  async function methodeWechseln(methode: string) {
    if (methode === "aufstellung") { aufstellungAktivieren(); return; }
    if (!await ask("Die Bauteil- und Geschossaufstellung wird beim nächsten Speichern entfernt. Zu direkten Flächensummen wechseln?", { title: "Eingabemethode", kind: "warning", okLabel: "Wechseln", cancelLabel: "Abbrechen" })) return;
    if (aktuell && ergebnis?.aufstellung) {
      eingabe.mass = structuredClone($state.snapshot(ergebnis.aufstellung.mass));
      eingabe.vollgeschosse = ergebnis.aufstellung.vollgeschosse;
    }
    eingabe.aufstellung = null;
  }

  function aufstellungAktivieren() {
    eingabe.aufstellung = { bau_nvo: "2017", bauteile: [], geschosse: [], nichtvollgeschoss_faktor: regelwerk("2017").nichtvollgeschoss_faktor, nichtvollgeschoss_festsetzung: "", grundstueck: { basisflaeche_m2: eingabe.mass.grundstuecksflaeche_m2, nicht_massgebend_m2: 0, abzug_begruendung: "", gemeinschaftsanteile: [] } };
  }

  function request(): NutzungsEingabe {
    const e = structuredClone($state.snapshot(eingabe));
    e.gebiet = festgelegtesGebiet ?? e.gebiet;
    e.mass.baumasse_m3 ??= null;
    e.vollgeschosse ??= null;
    return e;
  }

  async function berechnen(event: SubmitEvent) {
    event.preventDefault();
    laeuft = true; fehler = null; hinweis = null;
    const stand = fingerprint;
    try {
      const r = await api.projektNutzungPruefen(projekt.id, request());
      if (r.projekt_version !== projekt.version) {
        ergebnis = null;
        fehler = "Die Projektfestsetzungen wurden inzwischen geändert. Bitte Projekt erneut öffnen und mit dem aktuellen Stand berechnen.";
      } else { ergebnis = r; ergebnisStand = stand; }
    } catch(e) { ergebnis = null; fehler = fehlertext(e); }
    finally { laeuft = false; }
  }

  async function speichern() {
    if (!formular.reportValidity()) return;
    laeuft = true; fehler = null; hinweis = null;
    try {
      const berechnung = request();
      if (berechnung.aufstellung) {
        const pruefung = await api.projektNutzungPruefen(projekt.id, berechnung);
        if (pruefung.projekt_version !== projekt.version || !pruefung.aufstellung) throw new Error("Projektstand geändert. Bitte Projekt erneut öffnen.");
        berechnung.mass = pruefung.aufstellung.mass;
        berechnung.vollgeschosse = pruefung.aufstellung.vollgeschosse;
      }
      const p = await api.projektAktualisieren({ id: projekt.id, expected_version: projekt.version, name: projekt.name, bundesland: projekt.bundesland, grundstuecksflaeche_m2: projekt.grundstuecksflaeche_m2,
        planungsrecht: { ...structuredClone($state.snapshot(projekt.planungsrecht)), berechnung },
      });
      gespeichert = structuredClone($state.snapshot(p.planungsrecht.berechnung!));
      eingabe = structuredClone($state.snapshot(gespeichert));
      onprojekt(p);
      ergebnis = null;
      hinweis = "Berechnungseingaben gespeichert.";
    } catch(e) { fehler = fehlertext(e); }
    finally { laeuft = false; }
  }
</script>

<label>Grundstücksberechnung<select bind:value={mehrere}><option value={true}>Flurstücke und einzelne Baugrundstücke</option><option value={false}>Bisherige Berechnung mit einer Gesamtfläche</option></select></label>
<div hidden={!mehrere}><Flurstuecke bind:this={flurstueckeEditor} {projekt} {darf} {onprojekt} /></div>
<div hidden={mehrere}>
{#if projekt.planungsrecht.baugrundstuecke}<p class="hinweisbox">Für dieses Projekt sind einzelne Baugrundstücke hinterlegt. Die bisherigen Gesamteingaben sind erhalten; verwenden Sie die Berechnung je Baugrundstück.</p>{/if}
<div class="knoepfe links"><button type="button" class="sekundaer" onclick={onrechtsgrundlagen}>Festsetzungen bearbeiten …</button>{#if geaendert}<span class="meta">Ungespeicherte Eingaben</span>{/if}</div>
<p class="meta">{mitPlan ? "Vergleich mit den gespeicherten Festsetzungen dieses Bebauungsplans. Nicht festgelegte Werte bleiben ungeprüft." : "Ohne Bebauungsplan: Vergleich mit Orientierungswerten nach § 17 BauNVO; keine verbindliche Zulässigkeitsentscheidung."}</p>
{#if fehler}<p class="fehlertext" role="alert">{fehler}</p>{/if}
{#if hinweis}<p role="status">{hinweis}</p>{/if}
<form bind:this={formular} class="raster nutzungsformular" onsubmit={berechnen}>
  <div class="formular-breit">
    <label>Eingabemethode<select value={eingabe.aufstellung ? "aufstellung" : "summen"} onchange={async (e) => { const select = e.currentTarget; await methodeWechseln(select.value); select.value = eingabe.aufstellung ? "aufstellung" : "summen"; }}><option value="summen">Direkte Flächensummen (bisherige Berechnung)</option><option value="aufstellung">Bauteile und Geschosse nach BauNVO erfassen</option></select></label>
    {#if eingabe.aufstellung}
      <label>Maßgebende BauNVO-Fassung<select bind:value={eingabe.aufstellung.bau_nvo}>{#each REGELWERKE as r}<option value={r.fassung}>BauNVO {r.fassung}</option>{/each}</select></label>
      <p class="meta">Die Fassung anhand des Bebauungsplans und seiner Überleitungsvorschriften bestimmen. Das Projektjahr allein ist dafür nicht maßgebend. Die Aufstellung ersetzt die direkten Grund- und Geschossflächensummen bei der Berechnung. Ein Methodenwechsel wird erst beim Speichern übernommen.</p>
    {:else}<p class="hinweisbox">Direkte Summen verwenden die bisherige Berechnung mit GRZ I/II. Für historische Fassungen und eine Aufschlüsselung die Bauteil- und Geschossformulare wählen.</p>{/if}
  </div>
  <label>Baugebiet {#if festgelegtesGebiet}<span class="meta">aus Bebauungsplan</span>{/if}
    <select value={festgelegtesGebiet ?? eingabe.gebiet} disabled={!!festgelegtesGebiet} onchange={(e) => (eingabe.gebiet = e.currentTarget.value as NutzungsEingabe["gebiet"])}>
      {#each GEBIETE as g}<option value={g.code}>{g.code} – {g.name}</option>{/each}
    </select>
  </label>
  {#if eingabe.aufstellung}<div class="formular-breit"><GrundstueckFormular aufstellung={eingabe.aufstellung} basisFallback={eingabe.mass.grundstuecksflaeche_m2} ergebnis={aktuell ? ergebnis?.aufstellung?.grundstueck ?? null : null} mitPlan={mitPlan} /></div>
  {:else}<label>Maßgebende Grundstücksfläche (m²)<input type="number" min="0.01" step="any" bind:value={eingabe.mass.grundstuecksflaeche_m2} required /></label>{/if}
  {#if !eingabe.aufstellung}
  <label>Grundfläche Hauptanlagen (m²)<input type="number" min="0" step="any" bind:value={eingabe.mass.grundflaeche_m2} required /></label>
  <label>Garagen, Stellplätze, Nebenanlagen (m²)<input type="number" min="0" step="any" bind:value={eingabe.mass.grundflaeche_nebenanlagen_m2} required /></label>
  <label>Geschossfläche in Vollgeschossen (m²)<input type="number" min="0" step="any" bind:value={eingabe.mass.geschossflaeche_m2} required /></label>
  {/if}
  <label>Baumasse (m³, optional)<input type="number" min="0" step="any" bind:value={eingabe.mass.baumasse_m3} /></label>
  {#if !eingabe.aufstellung}<label>Geplante Vollgeschosse<input type="number" min="0" step="1" bind:value={eingabe.vollgeschosse} /></label>{/if}
  {#if eingabe.aufstellung}<div class="formular-breit"><FlaechenFormular festsetzungen={mitPlan ? projekt.planungsrecht.festgesetzte_werte : { ...projekt.planungsrecht.festgesetzte_werte, nichtvollgeschoss_faktor: null }} bind:aufstellung={eingabe.aufstellung} ergebnis={aktuell ? ergebnis?.aufstellung ?? null : null} /></div>{/if}
  <div class="knoepfe links"><button type="submit" disabled={laeuft || !!projekt.planungsrecht.baugrundstuecke}>Berechnen</button><button type="button" class="sekundaer" onclick={speichern} disabled={!darf || laeuft || !geaendert || !!projekt.planungsrecht.baugrundstuecke}>Eingaben speichern</button></div>
</form>
{#if ergebnis && !aktuell}<p class="hinweisbox" role="status">Eingaben oder Festsetzungen wurden geändert. Bitte neu berechnen.</p>{/if}
{#if aktuell && ergebnis}
  <div class="tabellen-scroll">
    <table class="ergebnistabelle">
      <thead><tr><th>Nachweis</th><th>Ist</th><th>Soll</th><th>Quelle</th><th>Überschreitung / Abweichung</th><th>Ergebnis</th></tr></thead>
      <tbody>{#each ergebnis.nachweise as n}
        {#if !(historisch && n.art === "grz_gesamt")}
        <tr>
          <td>{n.art === "grz" ? (historisch ? "GRZ" : "GRZ I") : n.art === "grz_gesamt" ? "GRZ II (Gesamtfläche)" : titel[n.art]}{n.zwingend ? " (zwingend)" : ""}</td><td>{z(n.ist)}</td><td>{z(n.soll)}</td><td>{quellen[n.quelle]}</td>
          <td>{n.zwingend ? differenz(n.abweichung) : differenz(n.ueberschreitung)}{n.ueberschreitung_prozent != null && n.ueberschreitung_prozent > 0 ? ` (${z(n.ueberschreitung_prozent)} %)` : ""}{n.flaechenueberschreitung_m2 != null && n.flaechenueberschreitung_m2 > 0 ? ` · ${z(n.flaechenueberschreitung_m2)} m²` : ""}</td>
          <td class:verstoss={n.eingehalten === false}>{n.soll == null ? "Nicht festgelegt" : n.ist == null ? "Eingabe fehlt" : n.eingehalten ? "✓ Vergleichswert eingehalten" : "✗ Abweichung"}</td>
        </tr>
        {/if}
      {/each}</tbody>
    </table>
  </div>
  <p class="meta">Der Flächenvergleich mit dem Baufenster sagt nichts über die Lage innerhalb der Baugrenze aus. Die geometrische Prüfung erfolgt unter „Pläne“. Die Geschossfläche wird aus der Aufstellung berechnet oder als direkte Summe eingegeben.</p>
{/if}

</div>
