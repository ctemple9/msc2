<script lang="ts">
  // MSC 1 OverviewChatCardView — the in-game chat feed a player sees with
  // "T", derived from console output via chatFeed.ts (a TypeScript port of
  // MSC 1's ChatFeedParser). This reads a point-in-time /v1/console/tail
  // snapshot rather than the live WebSocket stream the Console tab (P12.10)
  // will own, so it refreshes on the same cadence as the rest of Overview.
  import Card from '../../components/base/Card.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import type { ChatFeedMessage } from './chatFeed';

  export let messages: readonly ChatFeedMessage[] = [];
  export let serverRunning = false;

  let scroller: HTMLDivElement | undefined;

  $: if (scroller && messages) {
    requestAnimationFrame(() => {
      if (scroller) scroller.scrollTop = scroller.scrollHeight;
    });
  }
</script>

<Card padding="14px 16px">
  <div class="overline">
    <span class="msc2-type-overline">Chat</span>
  </div>

  {#if messages.length === 0}
    <div class="empty">
      <Icon name="chat" size={18} />
      <p class="title">No chat yet</p>
      <p class="hint">
        {serverRunning
          ? 'Player chat and advancements will appear here.'
          : 'Start the server to see live chat.'}
      </p>
    </div>
  {:else}
    <div class="feed" bind:this={scroller}>
      {#each messages as message (message.id)}
        {#if message.kind === 'chat'}
          <p class="line">
            <span class="player">{message.player}</span>
            <span class="text">{message.text}</span>
          </p>
        {:else if message.kind === 'advancement'}
          <p class="line advancement">
            <span class="player">{message.player}</span>
            <span class="muted"> earned </span>
            <span class="gold">{message.text}</span>
          </p>
        {:else}
          <p class="line event">
            <span class="event-dot" class:leave={message.kind === 'leave'}></span>
            <span class="player">{message.player}</span>
            <span class="muted"> {message.text}</span>
          </p>
        {/if}
      {/each}
    </div>
  {/if}
</Card>

<style>
  .overline {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 12px;
    color: var(--msc2-text-tertiary);
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    color: var(--msc2-text-tertiary);
    padding: 20px 8px;
    text-align: center;
  }
  .empty .title {
    margin: 6px 0 0;
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-text-secondary);
  }
  .empty .hint {
    margin: 0;
    font-size: 10px;
  }
  .feed {
    height: 190px;
    overflow-y: auto;
    scrollbar-width: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .feed::-webkit-scrollbar {
    display: none;
  }
  .line {
    margin: 0;
    font-size: 11px;
    line-height: 1.5;
  }
  .player {
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .text {
    color: var(--msc2-text-secondary);
    margin-left: 4px;
  }
  .muted {
    color: var(--msc2-text-tertiary);
  }
  .gold {
    color: #d9b34a;
  }
  .event {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 10px;
  }
  .event-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--msc2-status-ok);
    flex-shrink: 0;
  }
  .event-dot.leave {
    background: var(--msc2-neutral-muted);
  }
</style>
