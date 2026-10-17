<script lang="ts">
  import type { Feld, Auswahl } from "../generated/plattform";
  import { mehrzeilig, feldFehler } from "./feldDarstellung";
  let {
    feld,
    pfad,
    wert,
    optionen = [],
    disabled = false,
    quelle = "",
    pruefversuch = false,
    onwert,
  }: {
    feld: Feld;
    pfad?: string;
    wert: unknown;
    optionen?: Auswahl[];
    disabled?: boolean;
    quelle?: string;
    pruefversuch?: boolean;
    onwert: (v: unknown) => void;
  } = $props();
  let zahlInput = $state<HTMLInputElement>();
  let anzeigeZahl = $state<number | undefined>();
  // Native Zwischeneingaben wie „-“ und „1.“ während des Tippens erhalten.
  // Der Datenwert wird geprüft, die sichtbare Eingabe erst beim Verlassen normiert.
  $effect(()=>{const v=wert;if(gebunden || !zahlInput?.matches(':focus'))anzeigeZahl=typeof v==='number'?v:undefined;});
  const id = $props.id();
  let beruehrt = $state(false);
  const opts = $derived(optionen.length ? optionen : feld.optionen);
  const gebunden = $derived(disabled || feld.readonly);
  const erforderlich = $derived(feld.typ === "number");
  const fehler = $derived(
    beruehrt || pruefversuch ? feldFehler(feld, wert) : "",
  );
  const hilfe = $derived(
    gebunden
      ? quelle ||
          (feld.readonly
            ? "Schreibgeschützter Wert"
            : "Aus einer Quelle übernommen")
      : "",
  );
  function aendern(v: unknown) {
    beruehrt = true;
    onwert(v);
  }
</script>

{#if feld.typ !== "hidden"}
  <div
    class="fachfeld"
    class:checkboxfeld={feld.typ === "checkbox"}
    data-feldpfad={pfad ?? feld.name}
    data-feldzustand={fehler
      ? "fehler"
      : feld.readonly
        ? "schreibgeschuetzt"
        : gebunden
          ? "uebernommen"
          : "eingabe"}
  >
    {#if feld.typ === "checkbox"}
      <label for={id} class="checkboxzeile"
        ><input
          {id}
          aria-label={feld.titel}
          type="checkbox"
          checked={wert === true}
          disabled={gebunden}
          onchange={(e) => aendern(e.currentTarget.checked)}
        /><span>{feld.titel}</span></label
      >
    {:else if feld.typ === "multi_select"}
      <fieldset class="mehrfach">
        <legend>{feld.titel}</legend>{#each opts as o}<label
            class="checkboxzeile"
            ><input
              type="checkbox"
              checked={Array.isArray(wert) && wert.includes(o.wert)}
              disabled={gebunden}
              onchange={(e) =>
                aendern(
                  e.currentTarget.checked
                    ? [...(Array.isArray(wert) ? wert : []), o.wert]
                    : (Array.isArray(wert) ? wert : []).filter(
                        (v) => v !== o.wert,
                      ),
                )}
            />{o.titel}</label
          >{/each}
      </fieldset>
    {:else}
      <label for={id}
        >{feld.titel}{#if erforderlich}<span
            class="pflicht"
            aria-hidden="true"
            title="Erforderliche Zahl"
          >
            *</span
          >{/if}</label
      >
      {#if feld.typ === "optional_bool"}
        <select
          {id}
          aria-label={feld.titel}
          value={wert === null ? "" : String(wert)}
          disabled={gebunden}
          aria-describedby={hilfe ? `${id}-hilfe` : undefined}
          onchange={(e) =>
            aendern(
              e.currentTarget.value === ""
                ? null
                : e.currentTarget.value === "true",
            )}
          ><option value="">Keine besondere Entscheidung</option><option
            value="true">Als Vollgeschoss</option
          ><option value="false">Als Nichtvollgeschoss</option></select
        >
      {:else if feld.typ === "select" || feld.typ === "optional_select" || feld.referenz}
        <select
          {id}
          aria-label={feld.titel}
          value={String(wert ?? "")}
          disabled={gebunden}
          aria-describedby={hilfe ? `${id}-hilfe` : undefined}
          onchange={(e) =>
            aendern(
              feld.typ === "optional_select" && e.currentTarget.value === ""
                ? null
                : e.currentTarget.value,
            )}
          ><option value="">Bitte auswählen</option>{#each opts as o}<option
              value={o.wert}>{o.titel}</option
            >{/each}</select
        >
      {:else if feld.typ === "number" || feld.typ === "optional_number"}
        <input
          {id}
          aria-label={feld.titel}
          type="number"
          step="any"
          value={typeof wert === "number" ? wert : undefined}
          readonly={gebunden}
          required={erforderlich}
          aria-required={erforderlich}
          aria-invalid={fehler ? "true" : undefined}
          aria-describedby={[
            hilfe ? `${id}-hilfe` : "",
            fehler ? `${id}-fehler` : "",
          ]
            .filter(Boolean)
            .join(" ") || undefined}
          placeholder={erforderlich ? "Zahl eingeben" : "Noch unbekannt"}
          oninput={(e) =>
            aendern(
              e.currentTarget.value === ""
                ? null
                : e.currentTarget.valueAsNumber,
            )}
        />
      {:else if mehrzeilig(feld)}
        <textarea
          {id}
          aria-label={feld.titel}
          rows="3"
          maxlength="4000"
          value={String(wert ?? "")}
          readonly={gebunden}
          aria-describedby={hilfe ? `${id}-hilfe` : undefined}
          oninput={(e) => aendern(e.currentTarget.value)}></textarea>
      {:else}
        <input
          {id}
          aria-label={feld.titel}
          type="text"
          maxlength="4000"
          value={String(wert ?? "")}
          readonly={gebunden}
          aria-describedby={hilfe ? `${id}-hilfe` : undefined}
          oninput={(e) => aendern(e.currentTarget.value)}
        />
      {/if}
    {/if}
    {#if hilfe}<span class="feldhilfe" id={`${id}-hilfe`}>{hilfe}</span>{/if}
    {#if fehler}<span class="feldfehler" id={`${id}-fehler`}>{fehler}</span
      >{/if}
  </div>
{/if}

<style>
  .fachfeld {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    min-width: 0;
  }
  .fachfeld > label {
    font-size: 0.9rem;
    margin: 0;
  }
  .checkboxfeld {
    justify-content: start;
    align-self: start;
  }
  label.checkboxzeile {
    display: flex;
    flex-direction: row;
    align-items: flex-start;
    gap: 0.6rem;
    margin: 0;
    line-height: 1.5;
  }
  .checkboxzeile input {
    margin-top: 0.2rem;
  }
  .mehrfach {
    margin: 0;
    padding: 0.6rem;
  }
  .mehrfach label {
    margin: 0.4rem 0;
  }
  .fachfeld input:not([type="checkbox"]),
  .fachfeld select,
  .fachfeld textarea {
    width: 100%;
    max-width: 100%;
    box-sizing: border-box;
  }
  input[type="checkbox"] {
    width: 1.125rem;
    flex-shrink: 0;
  }
  textarea {
    resize: vertical;
  }
</style>
