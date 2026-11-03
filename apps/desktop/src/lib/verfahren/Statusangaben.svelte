<script lang="ts">
  import { datumDe } from "../api";
  import type {
    Bauausfuehrung,
    Berichtigungsgegenstand,
    Kommunikation,
    Ruecknahme,
    Verfahrensart,
    Verfahrensstatus,
    Vollstaendigkeit,
  } from "../generated/plattform";
  import Berichtigen from "./Berichtigen.svelte";
  import { heute } from "./api";
  let {
    status,
    art,
    kommunikation,
    vollstaendigkeit = $bindable(),
    ruecknahme = $bindable(),
    ausfuehrung = $bindable(),
    gespeichert = {},
    onberichtigen,
  }: {
    status: Verfahrensstatus;
    art: Verfahrensart;
    kommunikation: Kommunikation[];
    vollstaendigkeit?: Vollstaendigkeit;
    ruecknahme?: Ruecknahme;
    ausfuehrung?: Bauausfuehrung;
    /** Gespeicherte Werte: Sie ändern sich nur mit begründeter Berichtigung. */
    gespeichert?: {
      vollstaendigkeit?: Vollstaendigkeit;
      ruecknahme?: Ruecknahme;
      ausfuehrung?: Bauausfuehrung;
    };
    onberichtigen: (g: Berichtigungsgegenstand, grund: string) => void;
  } = $props();
  const eingehend = $derived(kommunikation.filter((k) => k.eingehend));
  let vOffen = $state(false),
    rOffen = $state(false),
    bOffen = $state(false),
    fOffen = $state(false);
  const vFest = $derived(!!gespeichert.vollstaendigkeit && !vOffen);
  const rFest = $derived(!!gespeichert.ruecknahme && !rOffen);
  const bFest = $derived(!!gespeichert.ausfuehrung && !bOffen);
  const fFest = $derived(!!gespeichert.ausfuehrung?.fertigstellung && !fOffen);
  /** Vorschlag: Datum des gewählten Schreibens; der Core prüft die Angaben. */
  const datumVon = (id: string) =>
    kommunikation.find((k) => k.id === id)?.datum ?? heute();
</script>

{#if status === "vollstaendig" || vollstaendigkeit}<fieldset>
    <legend>Vollständigkeit</legend>
    {#if vollstaendigkeit}<div class="raster">
        <label
          >Bestätigt am<span class="pflicht" aria-hidden="true"> *</span><input
            aria-label="Vollständigkeit bestätigt am"
            type="date"
            required
            max={heute()}
            disabled={vFest}
            bind:value={vollstaendigkeit.datum}
          /></label
        ><label
          >Bestätigungsschreiben<select
            aria-label="Bestätigungsschreiben"
            disabled={vFest}
            value={vollstaendigkeit.kommunikation}
            onchange={(e) => {
              vollstaendigkeit!.kommunikation = e.currentTarget.value;
              if (!gespeichert.vollstaendigkeit)
                vollstaendigkeit!.datum = datumVon(e.currentTarget.value);
            }}
            >{#each eingehend as k (k.id)}<option value={k.id}
                >{datumDe(k.datum)} · {k.betreff}</option
              >{/each}</select
          ></label
        >
      </div>
      {#if gespeichert.vollstaendigkeit}<Berichtigen
          bezeichnung="Vollständigkeitsbestätigung"
          bind:offen={vOffen}
          onvormerken={(g) => onberichtigen({ art: "vollstaendigkeit" }, g)}
        />{/if}
      <p class="meta">
        Behördliche Bearbeitungsfristen knüpfen je nach Landesrecht an dieses
        Datum an. Nach dem Speichern ändert es sich nur mit begründeter
        Berichtigung.
      </p>{:else if eingehend.length}<button
        type="button"
        class="sekundaer"
        onclick={() =>
          (vollstaendigkeit = {
            datum: eingehend[0].datum,
            kommunikation: eingehend[0].id,
          })}>Bestätigung der Behörde erfassen</button
      >{:else}<p class="meta">
        Zuerst das Bestätigungsschreiben der Behörde im Register Kommunikation
        erfassen.
      </p>{/if}
  </fieldset>{/if}
{#if status === "zurueckgenommen" || ruecknahme}<fieldset>
    <legend>Rücknahme</legend>
    {#if ruecknahme}<div class="raster">
        <label
          >Zurückgenommen am<span class="pflicht" aria-hidden="true"> *</span
          ><input
            aria-label="Zurückgenommen am"
            type="date"
            required
            max={heute()}
            disabled={rFest}
            bind:value={ruecknahme.datum}
          /></label
        ><label
          >Rücknahmeschreiben<select
            aria-label="Rücknahmeschreiben"
            disabled={rFest}
            value={ruecknahme.kommunikation}
            onchange={(e) => {
              ruecknahme!.kommunikation = e.currentTarget.value;
              if (!gespeichert.ruecknahme)
                ruecknahme!.datum = datumVon(e.currentTarget.value);
            }}
            >{#each kommunikation as k (k.id)}<option value={k.id}
                >{datumDe(k.datum)} · {k.betreff}</option
              >{/each}</select
          ></label
        >
      </div>
      {#if gespeichert.ruecknahme}<Berichtigen
          bezeichnung="Rücknahme"
          bind:offen={rOffen}
          onvormerken={(g) => onberichtigen({ art: "ruecknahme" }, g)}
        />{:else if status !== "zurueckgenommen"}<p class="meta">
          Die Rücknahme wird zusammen mit dem Bearbeitungsstand
          „Zurückgenommen“ gespeichert.
        </p>{/if}{:else if kommunikation.length}<button
        type="button"
        class="sekundaer"
        onclick={() =>
          (ruecknahme = {
            datum: kommunikation[0].datum,
            kommunikation: kommunikation[0].id,
          })}>Rücknahme erfassen</button
      >{:else}<p class="meta">
        Zuerst das Rücknahmeschreiben im Register Kommunikation erfassen.
      </p>{/if}
  </fieldset>{/if}
{#if status === "entschieden" || art === "freistellung" || ausfuehrung}<fieldset>
    <legend>Bauausführung</legend>
    {#if ausfuehrung}<div class="raster">
        <label
          >Baubeginn<span class="pflicht" aria-hidden="true"> *</span><input
            aria-label="Baubeginn"
            type="date"
            required
            disabled={bFest}
            bind:value={ausfuehrung.baubeginn}
          /></label
        ><label
          >Fertigstellung<input
            aria-label="Fertigstellung"
            type="date"
            disabled={fFest}
            value={ausfuehrung.fertigstellung ?? ""}
            onchange={(e) =>
              (ausfuehrung!.fertigstellung =
                e.currentTarget.value || undefined)}
          /></label
        >
      </div>
      {#if gespeichert.ausfuehrung}<Berichtigen
          bezeichnung="Baubeginn"
          bind:offen={bOffen}
          onvormerken={(g) => onberichtigen({ art: "baubeginn" }, g)}
        />{/if}{#if gespeichert.ausfuehrung?.fertigstellung}<Berichtigen
          bezeichnung="Fertigstellung"
          bind:offen={fOffen}
          onvormerken={(g) => onberichtigen({ art: "fertigstellung" }, g)}
        />{/if}
      <p class="meta">
        Der Baubeginn beendet die Wiedervorlage der Geltungsdauer, die
        Fertigstellung die einer Fertigstellungsfrist. Ggf. Baubeginnanzeige an
        die Behörde gesondert senden.
      </p>{:else}<button
        type="button"
        class="sekundaer"
        onclick={() => (ausfuehrung = { baubeginn: heute() })}
        >Baubeginn erfassen</button
      >{/if}
  </fieldset>{/if}
