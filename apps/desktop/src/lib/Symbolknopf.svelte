<script lang="ts">
  import type { Snippet } from "svelte";
  /**
   * Knopf, der nur ein Symbol zeigt. Der Name erscheint beim Überfahren oder
   * Fokussieren als Tooltip und ist über aria-label auch für Vorlesehilfen da.
   */
  let {
    titel,
    kuerzel = "",
    tip = "unten",
    aktiv = undefined,
    disabled = false,
    type = "button",
    klasse = "",
    onclick,
    children,
    ...rest
  }: {
    titel: string;
    kuerzel?: string;
    tip?: "unten" | "rechts" | "links";
    aktiv?: boolean;
    disabled?: boolean;
    type?: "button" | "submit";
    klasse?: string;
    onclick?: (e: MouseEvent) => void;
    children: Snippet;
    [key: string]: unknown;
  } = $props();
  const tooltip = $derived(kuerzel ? `${titel} ${kuerzel}` : titel);
</script>

<button
  {type}
  class={`symbolknopf tip-${tip} ${klasse}`}
  class:aktiv
  aria-label={titel}
  aria-pressed={aktiv}
  aria-keyshortcuts={kuerzel || undefined}
  data-tip={tooltip}
  {disabled}
  {onclick}
  {...rest}
>
  {@render children()}
</button>
