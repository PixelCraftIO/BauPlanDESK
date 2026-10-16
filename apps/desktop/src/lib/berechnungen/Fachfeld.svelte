<script lang="ts">
  import type { Feld, Auswahl } from "../generated/plattform";
  let {
    feld,
    wert,
    optionen = [],
    disabled = false,
    onwert,
  }: {
    feld: Feld;
    wert: unknown;
    optionen?: Auswahl[];
    disabled?: boolean;
    onwert: (v: unknown) => void;
  } = $props();
  const opts = $derived(optionen.length ? optionen : feld.optionen);
</script>

{#if feld.typ !== "hidden"}
  <label class="fachfeld">
    <span>{feld.titel}</span>
    {#if feld.typ === "checkbox"}<input
        type="checkbox"
        checked={wert === true}
        disabled={disabled || feld.readonly}
        onchange={(e) => onwert(e.currentTarget.checked)}
      />
    {:else if feld.typ === "multi_select"}<span class="mehrfach"
        >{#each opts as o}<label
            ><input
              type="checkbox"
              checked={Array.isArray(wert) && wert.includes(o.wert)}
              {disabled}
              onchange={(e) =>
                onwert(
                  e.currentTarget.checked
                    ? [...(Array.isArray(wert) ? wert : []), o.wert]
                    : (Array.isArray(wert) ? wert : []).filter(
                        (v) => v !== o.wert,
                      ),
                )}
            />{o.titel}</label
          >{/each}</span
      >
    {:else if feld.typ === "optional_bool"}<select
        value={wert === null ? "" : String(wert)}
        {disabled}
        onchange={(e) =>
          onwert(
            e.currentTarget.value === ""
              ? null
              : e.currentTarget.value === "true",
          )}
        ><option value="">Keine besondere Entscheidung</option><option
          value="true">Als Vollgeschoss</option
        ><option value="false">Als Nichtvollgeschoss</option></select
      >
    {:else if feld.typ === "select" || feld.typ === "optional_select" || feld.referenz}<select
        value={String(wert ?? "")}
        disabled={disabled || feld.readonly}
        onchange={(e) =>
          onwert(
            feld.typ === "optional_select" && e.currentTarget.value === ""
              ? null
              : e.currentTarget.value,
          )}
        ><option value="">Bitte auswählen</option>{#each opts as o}<option
            value={o.wert}>{o.titel}</option
          >{/each}</select
      >
    {:else if feld.typ === "number" || feld.typ === "optional_number"}<input
        type="number"
        step="any"
        value={typeof wert === "number" ? wert : undefined}
        disabled={disabled || feld.readonly}
        oninput={(e) =>
          onwert(
            e.currentTarget.value === ""
              ? feld.typ === "optional_number"
                ? null
                : 0
              : e.currentTarget.valueAsNumber,
          )}
      />
    {:else}<textarea
        rows="2"
        maxlength="4000"
        value={String(wert ?? "")}
        disabled={disabled || feld.readonly}
        oninput={(e) => onwert(e.currentTarget.value)}
      ></textarea>{/if}
  </label>
{/if}

<style>
  .fachfeld {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    min-width: 0;
  }
  .fachfeld > span {
    font-size: 0.9rem;
  }
  input,
  select,
  textarea {
    max-width: 100%;
    box-sizing: border-box;
  }
  input[type="checkbox"] {
    align-self: flex-start;
  }
  .mehrfach {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
  }
  .mehrfach label {
    display: flex;
    gap: 0.4rem;
  }
  textarea {
    resize: vertical;
  }
</style>
