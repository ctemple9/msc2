<script lang="ts">
  // A status dot must map to a defined state and always carry a text label
  // (docs/msc2/antiAIslop.md tell #12). docs/msc2/renderings/status-card.html
  export let tone: 'ok' | 'warn' | 'error' | 'bedrock' = 'ok';
  export let label: string;
  export let showDot = true;
</script>

<span class="status-dot" class:no-dot={!showDot}>
  {#if showDot}<span class="dot {tone}" aria-hidden="true"></span>{/if}
  <span class="label {tone}">{label}</span>
</span>

<style>
  .status-dot {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    display: inline-block;
  }

  .label {
    font-size: 12px;
    font-weight: 500;
  }

  .dot.ok {
    background-color: var(--msc2-status-ok);
  }
  .dot.warn {
    background-color: var(--msc2-status-warn);
  }
  .dot.error {
    background-color: var(--msc2-status-error);
  }
  .dot.bedrock {
    background-color: var(--msc2-status-bedrock);
  }

  .label.ok {
    color: var(--msc2-status-ok);
  }
  .label.warn {
    color: var(--msc2-status-warn);
  }
  .label.error {
    color: var(--msc2-status-error);
  }
  .label.bedrock {
    color: var(--msc2-status-bedrock);
  }
</style>
