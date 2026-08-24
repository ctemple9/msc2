<script lang="ts">
  import ActionButton from '../../components/ActionButton.svelte';
  import StatusBadge from '../../components/StatusBadge.svelte';

  export let eyebrow = 'Selected server';
  export let title = '';
  export let description = '';
  export let status = 'Agent disconnected';
  export let statusTone: 'positive' | 'neutral' | 'warning' | 'danger' = 'warning';
  export let actionLabel = '';
  export let onAction: (() => void) | undefined = undefined;
</script>

<div class="screen-header">
  <div>
    <p class="eyebrow">{eyebrow}</p>
    <h2>{title}</h2>
    {#if description}<p class="screen-description">{description}</p>{/if}
  </div>
  <div class="screen-header-actions">
    <StatusBadge {status} tone={statusTone} />
    {#if actionLabel}<ActionButton label={actionLabel} onclick={onAction}
        >{actionLabel}</ActionButton
      >{/if}
  </div>
</div>
