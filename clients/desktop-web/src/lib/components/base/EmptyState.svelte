<script lang="ts">
  // Centered muted icon, title, one line, optional action. No decoration.
  // docs/msc2/renderings/primitives.html
  export let title: string;
  export let message = '';
</script>

<div class="empty">
  <span class="icon"><slot name="icon" /></span>
  <div class="title">{title}</div>
  {#if message}<div class="message">{message}</div>{/if}
  <slot name="action" />
</div>

<style>
  .empty {
    background: var(--msc2-tier-content);
    border-radius: 12px;
    padding: 26px 16px;
    text-align: center;
  }

  .icon {
    font-size: 30px;
    color: rgba(255, 255, 255, 0.28);
    display: inline-flex;
  }

  .title {
    font-size: 14px;
    font-weight: 500;
    margin-top: 10px;
    color: var(--msc2-text-primary);
  }

  .message {
    font-size: 12px;
    color: rgba(255, 255, 255, 0.45);
    margin-top: 4px;
    line-height: 1.5;
  }
</style>
