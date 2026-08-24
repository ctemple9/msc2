<script lang="ts">
  export let title = '';
  export let eyebrow = '';
  export let tone: 'normal' | 'accent' | 'warning' = 'normal';
</script>

<article class="surface-card {tone}">
  {#if eyebrow}<p class="eyebrow">{eyebrow}</p>{/if}
  {#if title}<h2>{title}</h2>{/if}
  <div class="card-content"><slot /></div>
</article>

<style>
  .surface-card {
    min-width: 0;
    padding: 1.15rem;
    border: 1px solid var(--msc-border);
    border-radius: var(--msc-radius-md);
    background: var(--msc-surface);
    box-shadow: 0 0.7rem 2rem rgba(0, 0, 0, 0.12);
  }

  .surface-card.accent {
    border-color: rgba(143, 227, 207, 0.42);
  }

  .surface-card.warning {
    border-color: rgba(244, 213, 141, 0.42);
  }

  h2 {
    margin: 0;
    font-size: 1.05rem;
  }

  .eyebrow {
    margin: 0 0 0.5rem;
    color: var(--msc-accent);
    font-size: 0.68rem;
    font-weight: 800;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .card-content {
    margin-top: 0.8rem;
  }
</style>
