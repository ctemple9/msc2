<script lang="ts">
  export let helpId: string;
  export let hostId = 'local-agent';
  export let serverId = 'survival';
</script>

<a
  class="help-link"
  href={`/hosts/${encodeURIComponent(hostId)}/servers/${encodeURIComponent(serverId)}/handbook?topic=${encodeURIComponent(helpId)}`}
>
  Learn more
</a>

<style>
  .help-link {
    color: var(--msc2-text-secondary);
    font-size: 12px;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .help-link:hover {
    color: var(--msc2-text-primary);
  }
  .help-link:focus-visible {
    outline: 2px solid var(--msc2-hairline-field-focus);
    outline-offset: 2px;
  }
</style>
