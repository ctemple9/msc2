<script lang="ts">
  // The docked console — real behavior (P12.10): filter chips, search, a live
  // buffered stream, command input + Send, copy/clear. docs/msc2/renderings/shell.html,
  // MSC 1 ConsoleView/ConsoleManager, ~/Documents/MSCSS/Main View.
  import { onDestroy, onMount } from 'svelte';
  import Button from '../base/Button.svelte';
  import Toggle from '../base/Toggle.svelte';
  import CommandPaletteSheet from '../../sections/console/CommandPaletteSheet.svelte';
  import { onboardingAnchor } from '../../help/tourAnchors';
  import {
    BrowserWebSocketConnector,
    ReconnectingStream,
    type StreamState,
  } from '../../streams/reconnecting';
  import type { ScreenApi, Schema } from '../../sections/shared/types';
  import {
    CONSOLE_CHIPS,
    type ConsoleChipId,
    type ConsoleLine,
    type ConsoleOrigin,
    type ConsoleLevel,
    type CustomFilter,
    EMPTY_CUSTOM_FILTER,
    commandEchoLine,
    commandSuggestions,
    consoleLineKey,
    consoleLinesAfterClear,
    consoleLineTone,
    livePaths,
    rememberCommand,
    visibleConsoleLines,
  } from '../../sections/console/model';

  export let collapsed = false;
  export let onToggle: () => void;
  // Explicit pixel height while expanded — undefined lets the collapsed
  // header-only row size itself naturally.
  export let height: number | undefined = undefined;
  export let api: ScreenApi | undefined = undefined;
  // Threaded from ApplicationShell's own `servers`/`activeServerId` (P12.10b)
  // -- both were already props there for DetailsHeader, so no App.svelte
  // change was needed to get this here.
  export let serverType: string | undefined = undefined;

  const FALLBACK_POLL_INTERVAL_MS = 2000;
  const PLAYER_POLL_INTERVAL_MS = 5000;

  let lines: ConsoleLine[] = [];
  let chip: ConsoleChipId = 'all';
  let custom: CustomFilter = EMPTY_CUSTOM_FILTER;
  let showCustomPanel = false;
  let search = '';
  let command = '';
  let history: string[] = [];
  let sendError = '';
  let logEl: HTMLDivElement | undefined;
  let followTail = true;
  let fallbackPollTimer: ReturnType<typeof setInterval> | undefined;
  let playersPollTimer: ReturnType<typeof setInterval> | undefined;
  let consoleStream: ReconnectingStream<ConsoleLine> | undefined;
  let streamState: StreamState = 'idle';
  let connectedApi: ScreenApi | undefined;
  let componentMounted = false;
  let stoppingStream = false;
  let feedGeneration = 0;
  let clearVersion = 0;
  let clearedAt: number | undefined;
  const clearedLineKeys = new Set<string>();
  let playersResponse: Schema['PlayersResponseDTO'] = { count: 0, players: [] };
  let showPalette = false;
  let hideAuto = false;
  let showFilters = false;

  $: visible = visibleConsoleLines(lines, chip, custom, search, hideAuto);
  $: onlinePlayers = playersResponse.players;
  $: suggestions = command ? commandSuggestions(command, serverType, onlinePlayers) : [];
  $: activeFilterCount =
    Number(chip !== 'all' && chip !== 'custom') +
    Number(hideAuto) +
    Number(custom.origins.size > 0 || custom.levels.size > 0);

  // The shell keeps this dock mounted while the selected host changes. Rebuild
  // the stream at that boundary so one host's console history cannot bleed into
  // another host's view.
  $: if (componentMounted && api !== connectedApi) {
    stopConsoleFeed();
    connectedApi = api;
    if (api) {
      resetConsoleBoundary();
      startConsoleFeed();
      void refreshPlayers();
    }
  }

  $: if (logEl && visible.length && followTail) {
    const target = logEl;
    requestAnimationFrame(() => {
      target.scrollTop = target.scrollHeight;
    });
  }

  async function pollTail(): Promise<void> {
    const currentApi = api;
    if (!currentApi) return;
    const version = clearVersion;
    try {
      const fetchedLines = await currentApi.get<ConsoleLine[]>(livePaths.tail);
      if (version === clearVersion) {
        lines = consoleLinesAfterClear(fetchedLines, clearedAt, clearedLineKeys);
      }
    } catch {
      // Agent unreachable this cycle — keep showing the last known buffer.
    }
  }

  async function refreshPlayers(): Promise<void> {
    const currentApi = api;
    if (!currentApi) return;
    try {
      playersResponse = await currentApi.get<Schema['PlayersResponseDTO']>(livePaths.players);
    } catch {
      // Same as above — keep the last known roster.
    }
  }

  async function streamUrl(currentApi: ScreenApi): Promise<string | undefined> {
    if (!currentApi.resourceUrl || typeof WebSocket === 'undefined') return undefined;
    try {
      const url = new URL(currentApi.resourceUrl(livePaths.stream), window.location.href);
      url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
      try {
        const result = await currentApi.post<{ ticket?: unknown }>(livePaths.streamTicket);
        if (typeof result.ticket === 'string' && result.ticket) {
          url.searchParams.set('ticket', result.ticket);
        }
      } catch {
        // Older agents and browser sessions can still authenticate the socket
        // directly; the ticket is an additive desktop-auth capability.
      }
      return url.toString();
    } catch {
      return undefined;
    }
  }

  async function startConsoleFeed(): Promise<void> {
    const currentApi = api;
    const generation = ++feedGeneration;
    if (!currentApi) {
      streamState = 'closed';
      return;
    }
    streamState = 'connecting';
    const url = await streamUrl(currentApi);
    if (!componentMounted || generation !== feedGeneration || api !== currentApi) return;
    if (!url) {
      streamState = 'closed';
      startFallbackPolling();
      void pollTail();
      return;
    }

    consoleStream = new ReconnectingStream<ConsoleLine>({
      connector: new BrowserWebSocketConnector<ConsoleLine>(url),
      maxHistory: 200,
      dedupeKey: consoleLineKey,
      onUpdate: (history) => {
        lines = consoleLinesAfterClear([...history], clearedAt, clearedLineKeys);
      },
      onState: (state) => {
        streamState = state;
        if (state === 'live') stopFallbackPolling();
        if (state === 'closed' && !stoppingStream) {
          startFallbackPolling();
          void pollTail();
        }
      },
    });
    consoleStream.connect();
  }

  function stopConsoleFeed(): void {
    feedGeneration += 1;
    stoppingStream = true;
    consoleStream?.close();
    consoleStream = undefined;
    stoppingStream = false;
    stopFallbackPolling();
  }

  function startFallbackPolling(): void {
    if (fallbackPollTimer) return;
    fallbackPollTimer = setInterval(() => void pollTail(), FALLBACK_POLL_INTERVAL_MS);
  }

  function stopFallbackPolling(): void {
    if (!fallbackPollTimer) return;
    clearInterval(fallbackPollTimer);
    fallbackPollTimer = undefined;
  }

  function resetConsoleBoundary(): void {
    clearVersion += 1;
    clearedAt = undefined;
    clearedLineKeys.clear();
    lines = [];
    playersResponse = { count: 0, players: [] };
  }

  onMount(() => {
    componentMounted = true;
    connectedApi = api;
    if (api) {
      startConsoleFeed();
      void refreshPlayers();
    }
    playersPollTimer = setInterval(() => void refreshPlayers(), PLAYER_POLL_INTERVAL_MS);
  });

  onDestroy(() => {
    componentMounted = false;
    stopConsoleFeed();
    if (playersPollTimer) clearInterval(playersPollTimer);
  });

  function onLogScroll(): void {
    if (!logEl) return;
    followTail = logEl.scrollHeight - logEl.scrollTop - logEl.clientHeight < 24;
  }

  function selectChip(id: ConsoleChipId): void {
    if (id === 'custom') {
      chip = 'custom';
      showCustomPanel = true;
      return;
    }
    chip = id;
    showCustomPanel = false;
  }

  function closeFilters(): void {
    showFilters = false;
    showCustomPanel = false;
  }

  function setOrigin(origin: ConsoleOrigin, enabled: boolean): void {
    const next = new Set(custom.origins);
    if (enabled) next.add(origin);
    else next.delete(origin);
    custom = { ...custom, origins: next };
    chip = 'custom';
  }

  function setLevel(level: ConsoleLevel, enabled: boolean): void {
    const next = new Set(custom.levels);
    if (enabled) next.add(level);
    else next.delete(level);
    custom = { ...custom, levels: next };
    chip = 'custom';
  }

  function resetCustom(): void {
    custom = EMPTY_CUSTOM_FILTER;
    chip = 'all';
    showCustomPanel = false;
  }

  async function send(value = command): Promise<void> {
    const trimmed = value.trim();
    if (!trimmed || !api) return;
    history = rememberCommand(history, trimmed);
    lines = [...lines, commandEchoLine(trimmed)];
    command = '';
    sendError = '';
    followTail = true;
    try {
      await api.post(livePaths.command, { command: trimmed });
      if (streamState !== 'live') void pollTail();
      void refreshPlayers();
    } catch (error) {
      sendError = error instanceof Error ? error.message : 'The agent did not run that command.';
    }
  }

  async function copyVisible(): Promise<void> {
    try {
      await navigator.clipboard?.writeText(visible.map((line) => line.text).join('\n'));
    } catch {
      // Clipboard access can be unavailable outside a secure context.
    }
  }

  function clearConsole(): void {
    for (const line of lines) clearedLineKeys.add(consoleLineKey(line));
    clearedAt = Date.now();
    clearVersion += 1;
    lines = [];
  }
</script>

<div
  class="dock"
  class:expanded={!collapsed}
  style={!collapsed && height !== undefined ? `height: ${height}px;` : ''}
  use:onboardingAnchor={'ob_console_panel'}
>
  <div class="dock-header">
    <button
      type="button"
      class="collapse"
      aria-label={collapsed ? 'Show console' : 'Hide console'}
      onclick={onToggle}
    >
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true">
        <path
          d={collapsed ? 'M6 9l6 6 6-6' : 'M6 15l6-6 6 6'}
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
    <span class="label">Console</span>
    {#if !collapsed}
      <div class="search-field">
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <circle cx="11" cy="11" r="7" stroke="currentColor" stroke-width="1.8" />
          <path
            d="M20 20l-4.3-4.3"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
          />
        </svg>
        <input type="text" bind:value={search} placeholder="Search…" aria-label="Search console" />
      </div>
      <button
        type="button"
        class="filter-trigger"
        class:active={activeFilterCount > 0}
        aria-expanded={showFilters}
        aria-haspopup="dialog"
        onclick={() => (showFilters = !showFilters)}
      >
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path
            d="M4 7h16M7 12h10M10 17h4"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
          />
        </svg>
        <span>Filters</span>
        {#if activeFilterCount > 0}<span class="filter-count">{activeFilterCount}</span>{/if}
      </button>
    {/if}
  </div>

  {#if !collapsed && showFilters}
    <div class="scrim" role="presentation" onclick={closeFilters}></div>
    <div class="filter-panel" role="dialog" aria-label="Console filters">
      <div class="filter-panel-header">
        <span>Console filters</span>
        <button type="button" class="reset" onclick={resetCustom}>Reset</button>
      </div>
      <div class="filters">
        {#each CONSOLE_CHIPS as item (item.id)}
          {#if item.id === 'custom'}
            <button
              type="button"
              class="chip custom"
              class:active={chip === 'custom'}
              aria-expanded={showCustomPanel}
              onclick={() => selectChip(item.id)}
            >
              <svg width="10" height="10" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                <path
                  d="M4 7h16M4 12h10M4 17h6"
                  stroke="currentColor"
                  stroke-width="1.8"
                  stroke-linecap="round"
                />
              </svg>
              {item.label}
            </button>
          {:else}
            <button
              type="button"
              class="chip"
              class:active={chip === item.id}
              onclick={() => selectChip(item.id)}
            >
              {item.label}
            </button>
          {/if}
        {/each}
      </div>
      <div class="auto-filter">
        <Toggle
          checked={hideAuto}
          label="Hide automatic output"
          onchange={(checked) => (hideAuto = checked)}
        />
        <span>Hide Auto</span>
      </div>

      {#if showCustomPanel}
        <div class="custom-options">
          <p class="group-label">Sources</p>
          <div class="check-row">
            <Toggle
              checked={custom.origins.has('server')}
              label="Server process"
              onchange={(checked) => setOrigin('server', checked)}
            />
            <span>Server process</span>
          </div>
          <div class="check-row">
            <Toggle
              checked={custom.origins.has('controller')}
              label="Controller"
              onchange={(checked) => setOrigin('controller', checked)}
            />
            <span>Controller</span>
          </div>
          <p class="group-label">Levels</p>
          <div class="check-row">
            <Toggle
              checked={custom.levels.has('info')}
              label="Info"
              onchange={(checked) => setLevel('info', checked)}
            />
            <span>Info</span>
          </div>
          <div class="check-row">
            <Toggle
              checked={custom.levels.has('warn')}
              label="Warn"
              onchange={(checked) => setLevel('warn', checked)}
            />
            <span>Warn</span>
          </div>
          <div class="check-row">
            <Toggle
              checked={custom.levels.has('error')}
              label="Error"
              onchange={(checked) => setLevel('error', checked)}
            />
            <span>Error</span>
          </div>
        </div>
      {/if}
    </div>
  {/if}

  {#if !collapsed}
    <div
      class="body"
      bind:this={logEl}
      onscroll={onLogScroll}
      aria-live="polite"
      aria-label="Server console"
    >
      {#if visible.length}
        {#each visible as line, index (line.ts + '-' + index)}
          <p
            class="line"
            class:alt={index % 2 === 1}
            class:tone-error={consoleLineTone(line) === 'error'}
            class:tone-warn={consoleLineTone(line) === 'warn'}
            class:tone-muted={consoleLineTone(line) === 'muted'}
          >
            {line.text}
          </p>
        {/each}
      {:else}
        <p class="empty">
          {lines.length
            ? 'No console lines match this filter.'
            : 'Connect to a running server to see console output here.'}
        </p>
      {/if}
    </div>

    {#if suggestions.length}
      <div class="autocomplete-strip" role="listbox" aria-label="Command suggestions">
        {#each suggestions as suggestion (suggestion)}
          <button type="button" class="suggestion" onclick={() => (command = suggestion)}>
            {suggestion}
          </button>
        {/each}
      </div>
    {/if}

    <div class="input-row">
      <span class="prompt" aria-hidden="true">›</span>
      <input
        type="text"
        class="command"
        bind:value={command}
        onkeydown={(event) => event.key === 'Enter' && send()}
        placeholder="Enter command…"
        disabled={!api}
        aria-label="Console command"
      />
      <button
        type="button"
        class="icon-action"
        title="Command palette"
        aria-label="Command palette"
        onclick={() => (showPalette = true)}
      >
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <rect
            x="5"
            y="4"
            width="14"
            height="16"
            rx="2"
            stroke="currentColor"
            stroke-width="1.6"
          />
          <path
            d="M8 9h8M8 13h8M8 17h5"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
          />
        </svg>
      </button>
      <button
        type="button"
        class="icon-action"
        title="Copy visible lines"
        aria-label="Copy visible lines"
        onclick={copyVisible}
      >
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <rect
            x="9"
            y="9"
            width="11"
            height="11"
            rx="1.5"
            stroke="currentColor"
            stroke-width="1.6"
          />
          <path
            d="M6 15H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h9a1 1 0 0 1 1 1v1"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
      <button
        type="button"
        class="icon-action"
        title="Clear console"
        aria-label="Clear console"
        onclick={clearConsole}
      >
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path
            d="M4 7h16M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2M7 7l1 12a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1l1-12M10 11v6M14 11v6"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
      <Button variant="primary" size="sm" label="Send" disabled={!api} onclick={() => send()}
        >Send</Button
      >
    </div>
    {#if sendError}<p class="send-error" role="status">{sendError}</p>{/if}
  {/if}
</div>

{#if showPalette}
  <CommandPaletteSheet
    {serverType}
    {onlinePlayers}
    onClose={() => (showPalette = false)}
    onUse={(value) => (command = value)}
  />
{/if}

<style>
  .dock {
    flex-shrink: 0;
    background: var(--msc2-tier-terminal);
    border-top: 1px solid var(--msc2-hairline-subtle);
    padding: 8px 12px;
    box-sizing: border-box;
    position: relative;
  }
  .dock.expanded {
    display: flex;
    flex-direction: column;
  }
  .dock-header {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .collapse {
    display: inline-flex;
    color: rgba(255, 255, 255, 0.5);
    background: transparent;
    border: none;
    padding: 2px;
    cursor: pointer;
  }
  .collapse:hover {
    color: rgba(255, 255, 255, 0.85);
  }
  .collapse:focus-visible {
    outline: 2px solid rgba(255, 255, 255, 0.4);
  }
  .label {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.8px;
    text-transform: uppercase;
    color: var(--msc2-text-tertiary);
    flex-shrink: 0;
  }
  .search-field {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 200px;
    color: var(--msc2-text-tertiary);
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-field);
    border-radius: 6px;
    padding: 4px 8px;
    margin-left: 8px;
  }
  .search-field input {
    flex: 1;
    min-width: 0;
    font: inherit;
    font-size: 11px;
    color: var(--msc2-text-primary);
    background: transparent;
    border: none;
  }
  .search-field input:focus {
    outline: none;
  }
  .filter-trigger {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-left: auto;
    padding: 5px 8px;
    color: var(--msc2-text-secondary);
    background: transparent;
    border: none;
    border-radius: 6px;
    font: inherit;
    font-size: 11px;
    cursor: pointer;
  }
  .filter-trigger:hover,
  .filter-trigger[aria-expanded='true'] {
    background: var(--msc2-neutral-elevated);
    color: var(--msc2-text-primary);
  }
  .filter-trigger.active {
    color: var(--msc2-text-primary);
  }
  .filter-trigger:focus-visible {
    outline: 2px solid rgba(255, 255, 255, 0.4);
    outline-offset: 1px;
  }
  .filter-count {
    min-width: 14px;
    text-align: center;
    color: var(--msc2-text-tertiary);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }
  .filters {
    flex-shrink: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    font-weight: 500;
    color: var(--msc2-text-secondary);
    background: transparent;
    border: none;
    border-radius: 5px;
    padding: 3px 8px;
    cursor: pointer;
    white-space: nowrap;
  }
  .chip.active {
    color: rgba(255, 255, 255, 0.9);
    background: var(--msc2-neutral-elevated);
    font-weight: 600;
  }
  .filter-panel {
    position: absolute;
    top: 38px;
    right: 12px;
    z-index: 101;
    width: min(520px, calc(100% - 24px));
    box-sizing: border-box;
    padding: 10px 12px;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline);
    border-radius: 8px;
    box-shadow: var(--msc2-shadow-float);
  }
  .filter-panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 6px;
    color: var(--msc2-text-primary);
    font-size: 12px;
    font-weight: 600;
  }
  .auto-filter {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
    padding: 8px 4px 0;
    color: var(--msc2-text-secondary);
    font-size: 10px;
    white-space: nowrap;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .auto-filter :global(.track) {
    transform: scale(0.68);
    transform-origin: left center;
    margin-right: -10px;
  }
  .scrim {
    position: fixed;
    inset: 0;
    z-index: 100;
  }
  .custom-options {
    margin-top: 8px;
    padding: 4px 4px 0;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .reset {
    font-size: 11px;
    color: var(--msc2-text-secondary);
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
  }
  .reset:hover {
    color: var(--msc2-text-primary);
  }
  .group-label {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.6px;
    text-transform: uppercase;
    color: var(--msc2-text-tertiary);
    margin: 8px 0 4px;
  }
  .check-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--msc2-text-secondary);
    padding: 3px 0;
  }
  .body {
    flex: 1;
    min-height: 0;
    margin-top: 7px;
    font-family: var(--msc2-font-mono);
    font-size: 11px;
    color: var(--msc2-text-secondary);
    line-height: 1.5;
    overflow-y: auto;
  }
  .line {
    margin: 0;
    padding: 0.5px 2px;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .line.alt {
    background: rgba(255, 255, 255, 0.018);
  }
  .line.tone-error {
    color: var(--msc2-status-error);
  }
  .line.tone-warn {
    color: var(--msc2-status-warn);
  }
  .line.tone-muted {
    color: var(--msc2-text-tertiary);
  }
  .empty {
    margin: 0;
    color: var(--msc2-text-tertiary);
  }
  .autocomplete-strip {
    flex-shrink: 0;
    display: flex;
    gap: 6px;
    overflow-x: auto;
    margin-top: 7px;
  }
  .suggestion {
    flex-shrink: 0;
    font-family: var(--msc2-font-mono);
    font-size: 11px;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.85);
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--msc2-hairline-field);
    border-radius: 6px;
    padding: 4px 9px;
    cursor: pointer;
    white-space: nowrap;
  }
  .suggestion:hover {
    background: rgba(255, 255, 255, 0.1);
  }
  .input-row {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 7px;
    margin-top: 7px;
  }
  .prompt {
    color: var(--msc2-text-tertiary);
    font-family: var(--msc2-font-mono);
    font-size: 12px;
  }
  .command {
    flex: 1;
    box-sizing: border-box;
    font-family: var(--msc2-font-mono);
    font-size: 11px;
    color: var(--msc2-text-primary);
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-field);
    border-radius: 7px;
    padding: 5px 9px;
  }
  .command:focus {
    outline: none;
    border-color: var(--msc2-hairline-field-focus);
  }
  .command:disabled {
    cursor: not-allowed;
    color: var(--msc2-text-tertiary);
  }
  .icon-action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    flex-shrink: 0;
    color: rgba(255, 255, 255, 0.6);
    background: transparent;
    border: 1px solid var(--msc2-hairline-field);
    border-radius: 7px;
    cursor: pointer;
  }
  .icon-action:hover {
    color: rgba(255, 255, 255, 0.9);
    background: rgba(255, 255, 255, 0.06);
  }
  .send-error {
    margin: 6px 0 0;
    font-size: 11px;
    color: var(--msc2-status-error);
  }
</style>
