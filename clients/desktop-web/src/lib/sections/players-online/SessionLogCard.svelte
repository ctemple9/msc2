<script lang="ts">
  // Ports DetailsPlayersTabView's sessionLogCard: filter, day-grouped
  // join/leave timeline, show-more/less past a recent-event limit, clear.
  import Card from '../../components/base/Card.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import Field from '../../components/base/Field.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import {
    filterSessionEvents,
    groupSessionEventsByDay,
    sessionDurationLabel,
    type SessionEvent,
  } from './model';

  export let events: readonly SessionEvent[] = [];
  export let onlineNames: ReadonlySet<string> = new Set();
  export let onClear: (() => void) | undefined = undefined;

  const RECENT_LIMIT = 10;

  let filterText = '';
  let showAll = false;

  $: filtered = filterSessionEvents(events, filterText);
  $: days = groupSessionEventsByDay(filtered);
  $: totalFiltered = filtered.length;
  $: visibleDays = showAll ? days : takeRecent(days, RECENT_LIMIT);

  function takeRecent(allDays: ReturnType<typeof groupSessionEventsByDay>, limit: number) {
    let remaining = limit;
    const result: typeof allDays = [];
    for (const day of allDays) {
      if (remaining <= 0) break;
      const slice = day.events.slice(-remaining);
      result.push({ day: day.day, events: slice });
      remaining -= slice.length;
    }
    return result;
  }

  function formatDay(day: string): string {
    return new Date(day).toLocaleDateString(undefined, {
      weekday: 'long',
      month: 'short',
      day: 'numeric',
    });
  }
  function formatTime(ts: string): string {
    const date = new Date(ts);
    return Number.isNaN(date.getTime())
      ? ''
      : date.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
  }
</script>

<Card>
  <div class="header">
    <div class="overline">
      <Icon name="clock" size={13} />
      <span class="msc2-type-overline">Session Log</span>
    </div>
    {#if onClear}
      <button type="button" class="clear" disabled={events.length === 0} onclick={onClear}
        >Clear Log</button
      >
    {/if}
  </div>

  <Field bind:value={filterText} placeholder="Filter by player name" />

  <div class="body">
    {#if events.length === 0}
      <EmptyState
        title="No session events recorded yet."
        message="Player join and leave events will appear here once the server has run."
      >
        <Icon name="clock" size={26} slot="icon" />
      </EmptyState>
    {:else if totalFiltered === 0}
      <p class="empty-filtered">No events match &quot;{filterText}&quot;.</p>
    {:else}
      <div class="days">
        {#each visibleDays as day (day.day)}
          <div class="day">
            <div class="day-header">{formatDay(day.day)}</div>
            {#each day.events as event (event.id)}
              <div class="event-row">
                <Icon name={event.kind === 'join' ? 'download' : 'clock'} size={12} />
                <span class="player">{event.player}</span>
                <span class="kind" class:leave={event.kind === 'leave'}
                  >{event.kind === 'join' ? 'joined' : 'left'}</span
                >
                <span class="spacer"></span>
                {#if sessionDurationLabel(event, filtered, onlineNames.has(event.player))}
                  <span class="duration"
                    >{sessionDurationLabel(event, filtered, onlineNames.has(event.player))}</span
                  >
                {/if}
                <span class="time">{formatTime(event.ts)}</span>
              </div>
            {/each}
          </div>
        {/each}
      </div>
      {#if totalFiltered > RECENT_LIMIT}
        <button type="button" class="toggle" onclick={() => (showAll = !showAll)}>
          {showAll ? 'Show recent only' : `Show all ${totalFiltered} events`}
        </button>
      {/if}
    {/if}
  </div>
</Card>

<style>
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }
  .overline {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--msc2-text-tertiary);
  }
  .clear {
    font-size: 12px;
    color: var(--msc2-status-error);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
  }
  .clear:disabled {
    color: var(--msc2-text-tertiary);
    cursor: not-allowed;
  }
  .body {
    margin-top: 10px;
  }
  .empty-filtered {
    margin: 0;
    padding: 12px 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .days {
    display: flex;
    flex-direction: column;
  }
  .day-header {
    font-size: 11px;
    font-weight: 500;
    color: var(--msc2-text-tertiary);
    padding: 6px 0;
  }
  .event-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
    color: var(--msc2-text-tertiary);
  }
  .player {
    font-size: 12px;
    color: var(--msc2-text-primary);
  }
  .kind {
    font-size: 12px;
    color: var(--msc2-status-ok);
  }
  .kind.leave {
    color: var(--msc2-text-tertiary);
  }
  .spacer {
    flex: 1;
  }
  .duration {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
    background: rgba(255, 255, 255, 0.05);
    border-radius: 4px;
    padding: 1px 6px;
  }
  .time {
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--msc2-text-tertiary);
  }
  .toggle {
    width: 100%;
    margin-top: 6px;
    padding: 8px 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
    background: none;
    border: none;
    border-top: 1px solid var(--msc2-hairline-subtle);
    cursor: pointer;
  }
</style>
