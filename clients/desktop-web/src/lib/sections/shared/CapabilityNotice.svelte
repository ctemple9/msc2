<script lang="ts">
  import HelpLink from '../../help/HelpLink.svelte';
  export let title = 'This capability is unavailable';
  export let message = 'The connected agent has not advertised this operation.';
  export let helpId: string | null | undefined = undefined;
  export let hostId = 'local-agent';
  export let serverId = 'survival';
</script>

<aside class="capability-notice" role="status">
  <strong>{title}</strong>
  <p>{message}</p>
  {#if helpId}<HelpLink {helpId} {hostId} {serverId} />{/if}
</aside>
