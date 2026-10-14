<script lang="ts">
  let { art }: { art: "projekte" | "kunden" | "formulare" | "einstellungen" } =
    $props();
</script>

<svg
  width="17"
  height="17"
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.6"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if art === "projekte"}<path d="M3 7h7l2-3h9v16H3z" /><path
      d="M3 8h18"
    />{:else if art === "kunden"}<circle cx="9" cy="8" r="3" /><path
      d="M3 20v-2a6 6 0 0 1 12 0v2M17 5a3 3 0 0 1 0 6M18 14a5 5 0 0 1 3 5v1"
    />{:else if art === "formulare"}<path
      d="M6 3h9l4 4v14H6zM14 3v5h5M9 12h7M9 16h7"
    />{:else}<circle cx="12" cy="12" r="3" /><path
      d="m10 3 4 0 1 3 3 1 3 2-1 4 1 3-3 2-3 1-1 3h-4l-1-3-3-1-3-2 1-3-1-4 3-2 3-1z"
    />{/if}
</svg>
