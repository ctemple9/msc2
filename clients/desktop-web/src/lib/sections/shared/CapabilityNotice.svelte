<script lang="ts">
  export let title = 'This capability is unavailable';
  export let message = 'The connected agent has not advertised this operation.';
  export let helpId: string | null | undefined = undefined;
</script>

<aside class="capability-notice" role="status">
  <strong>{title}</strong>
  <p>{message}</p>
  {#if helpId}<small>Learn more: {helpId}</small>{/if}
</aside>
