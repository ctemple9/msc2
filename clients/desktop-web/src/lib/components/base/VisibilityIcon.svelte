<script lang="ts">
  export let visible = false;
</script>

<svg width="14" height="14" viewBox="0 0 24 24" fill="none" aria-hidden="true">
  {#if visible}
    <path
      d="M2.5 12s3.5-5 9.5-5 9.5 5 9.5 5-3.5 5-9.5 5-9.5-5-9.5-5z"
      stroke="currentColor"
      stroke-width="1.8"
      stroke-linejoin="round"
    />
    <circle cx="12" cy="12" r="2.5" stroke="currentColor" stroke-width="1.8" />
  {:else}
    <path
      d="M3 3l18 18M10.6 10.6a2 2 0 0 0 2.8 2.8M6.5 6.5C4.4 8 2.9 10 2.5 12c1.3 3.1 4.8 5 9.5 5 1.4 0 2.7-.2 3.8-.6M17.5 17.5c2.1-1.5 3.6-3.5 4-5.5-1.3-3.1-4.8-5-9.5-5-1.4 0-2.7.2-3.8.6"
      stroke="currentColor"
      stroke-width="1.8"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  {/if}
</svg>
