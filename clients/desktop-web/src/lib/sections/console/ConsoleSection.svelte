<script lang="ts">
  import { onMount } from 'svelte';
  import ActionButton from '../../components/ActionButton.svelte';
  import StatePanel from '../../components/StatePanel.svelte';
  import OperationQueue from '../shared/OperationQueue.svelte';
  import NotificationFeed from '../shared/NotificationFeed.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, errorMessage, mutate } from '../shared/types';
  import { demoConsole, filterLines, livePaths, rememberCommand } from './model';

  export let api: ScreenProps['api'] = undefined;
  export let operations: readonly Schema['OperationDTO'][] = [];
  export let notifications: readonly Schema['NotificationEventDTO'][] = [];
  let lines = demoConsole;
  let search = '';
  let level = '';
  let paused = false;
  let command = '';
  let history: string[] = [];
  let favorites: string[] = ['/list', 'save-all'];
  let notice = '';

  onMount(async () => {
    lines = await call(api, lines, livePaths.tail);
  });
  $: visibleLines = paused ? lines : filterLines(lines, search, level);

  async function send(value = command): Promise<void> {
    const next = value.trim();
    if (!next) return;
    history = rememberCommand(history, next);
    try {
      notice = (await mutate<Schema['CommandResult']>(api, livePaths.command, { command: next }))
        .result;
      command = '';
    } catch (error) {
      notice = errorMessage(error);
    }
  }

  function toggleFavorite(): void {
    const value = command.trim();
    if (value && !favorites.includes(value)) favorites = [...favorites, value];
  }

  async function copyVisible(): Promise<void> {
    try {
      await navigator.clipboard?.writeText(visibleLines.map((line) => line.text).join('\n'));
      notice = 'Visible console lines copied.';
    } catch {
      notice = 'Copy is unavailable in this browser context.';
    }
  }
</script>

<div class="screen">
  <ScreenHeader
    eyebrow="Live server"
    title="Console"
    description="The console is bounded per host, searchable locally, and reconnects from the agent's tail without losing the selected-server context."
    status={paused ? 'Paused' : 'Live'}
    statusTone={paused ? 'warning' : 'positive'}
    actionLabel="Clear local view"
    onAction={() => (lines = [])}
  />
  <div class="screen-card">
    <div class="form-grid">
      <div class="field">
        <label for="console-search">Search history</label><input
          id="console-search"
          bind:value={search}
          placeholder="Find a line"
        />
      </div>
      <div class="field">
        <label for="console-level">Filter level</label><select id="console-level" bind:value={level}
          ><option value="">All levels</option><option value="info">Info</option><option
            value="warn">Warnings</option
          ><option value="error">Errors</option></select
        >
      </div>
    </div>
    <div class="screen-actions" style="margin-top: .7rem">
      <ActionButton
        kind="quiet"
        label={paused ? 'Resume console' : 'Pause console'}
        onclick={() => (paused = !paused)}>{paused ? 'Resume' : 'Pause'}</ActionButton
      ><ActionButton kind="quiet" label="Copy visible lines" onclick={copyVisible}
        >Copy</ActionButton
      >
    </div>
    <div class="console-window" aria-live="polite" aria-label="Server console">
      {#if visibleLines.length}{#each visibleLines as line}<div class="console-line">
            <time>{new Date(line.ts).toLocaleTimeString()}</time><span class="console-level"
              >{line.level ?? 'info'}</span
            ><span>{line.text}</span>
          </div>{/each}{:else}<StatePanel
          kind="empty"
          title="No matching lines"
          message="The local view is empty; reconnecting will request a bounded tail from the agent."
        />{/if}
    </div>
    <div class="inline-form" style="margin-top: .8rem">
      <div class="field">
        <label for="console-command">Send command</label><input
          id="console-command"
          bind:value={command}
          onkeydown={(event) => event.key === 'Enter' && send()}
          placeholder="say hello"
        />
      </div>
      <ActionButton label="Send" onclick={() => send()}>Send</ActionButton><ActionButton
        kind="quiet"
        label="Favorite command"
        onclick={toggleFavorite}>☆</ActionButton
      >
    </div>
    {#if history.length}<div
        class="tag-list"
        style="margin-top: .7rem"
        aria-label="Command history"
      >
        {#each history.slice(0, 8) as item}<button
            class="tag tag-button"
            type="button"
            onclick={() => send(item)}>{item}</button
          >{/each}
      </div>{/if}
    {#if favorites.length}<p class="metric-label" style="margin-top: 1rem">Favorites</p>
      <div class="tag-list">
        {#each favorites as item}<button
            class="tag tag-button"
            type="button"
            onclick={() => send(item)}>{item}</button
          >{/each}
      </div>{/if}
  </div>
  {#if notice}<p class="muted" role="status">{notice}</p>{/if}
  <div class="screen-grid">
    <section class="screen-card">
      <div class="screen-card-header">
        <h3>Operations</h3>
        <span class="metric-label">Reconnect-safe</span>
      </div>
      <OperationQueue
        {operations}
        onCancel={async (id) => {
          try {
            await api?.post(`/v1/operations/${id}/cancel`);
          } catch (error) {
            notice = errorMessage(error);
          }
        }}
      />
    </section>
    <section class="screen-card">
      <h3>Notifications</h3>
      <NotificationFeed {notifications} />
    </section>
  </div>
</div>

<style>
  .console-window {
    min-height: 15rem;
    max-height: 32rem;
    overflow: auto;
    margin-top: 1rem;
    padding: 0.8rem;
    border: 1px solid var(--msc-border);
    border-radius: var(--msc-radius-sm);
    color: #d7f5e8;
    background: #0a1014;
    font:
      0.82rem/1.55 ui-monospace,
      SFMono-Regular,
      Menlo,
      monospace;
  }
  .console-line {
    display: grid;
    grid-template-columns: 5.5rem 4rem 1fr;
    gap: 0.55rem;
    padding: 0.12rem 0;
  }
  .console-line time,
  .console-level {
    color: var(--msc-subtle);
  }
  .tag-button {
    border: 0;
    cursor: pointer;
    font: inherit;
    background: transparent;
  }
  @media (max-width: 600px) {
    .console-line {
      grid-template-columns: 1fr;
      gap: 0.05rem;
      padding: 0.35rem 0;
    }
  }
</style>
