<script lang="ts">
  export let status = 'Unknown';
  export let tone: 'positive' | 'neutral' | 'warning' | 'danger' = 'neutral';
</script>

<span class="status-badge {tone}" role="status"><span aria-hidden="true"></span>{status}</span>

<style>
  .status-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.45rem;
    width: fit-content;
    padding: 0.3rem 0.55rem;
    border-radius: 99rem;
    color: var(--msc-muted);
    background: rgba(232, 238, 242, 0.08);
    font-size: 0.76rem;
    font-weight: 700;
  }

  .status-badge span {
    width: 0.42rem;
    height: 0.42rem;
    border-radius: 50%;
    background: currentColor;
  }

  .positive {
    color: var(--msc-accent);
  }
  .warning {
    color: var(--msc-warning);
  }
  .danger {
    color: var(--msc-danger);
  }
</style>
