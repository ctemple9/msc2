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
    color: var(--msc-accent);
    font-size: 0.78rem;
  }
  .help-link:focus-visible {
    outline: none;
    box-shadow: var(--msc-focus);
  }
</style>
