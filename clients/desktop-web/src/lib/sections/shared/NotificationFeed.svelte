<script lang="ts">
  import type { Schema } from './types';
  import { dateLabel } from './types';

  export let notifications: readonly Schema['NotificationEventDTO'][] = [];
</script>

<div class="notification-feed" aria-label="Notifications">
  {#if notifications.length === 0}
    <p class="muted">No new notifications for this host.</p>
  {:else}
    {#each notifications.slice(-6).reverse() as notification (notification.id)}
      <div class="notification-row">
        <strong>{notification.title}</strong>
        <span>{dateLabel(notification.occurredAtISO8601)}</span>
        <p>{notification.body}</p>
      </div>
    {/each}
  {/if}
</div>
