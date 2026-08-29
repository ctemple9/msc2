<script lang="ts">
  // Ports MSC 1 SidebarView.swift's HowToConnectSidebarSection directly --
  // Cameron flagged that the first build of this reused Overview's boxed,
  // two-column Connection Info card (ConnectionCard.svelte), when the
  // oracle's own sidebar treatment is plainer: one eye toggle, then a flat
  // list of icon + label + a single pill-shaped value row per method, all
  // shown at once (no Local/Public switch -- every method that has a value
  // just appears). This rebuild keeps ConnectionCard's exact data and
  // derivation (/v1/connectivity, /v1/config/geyser, the same honest
  // "not reported yet" fallback for Java's LAN address -- the agent still
  // doesn't report one, ConnectionCard.svelte's own finding still holds)
  // but restyles it to that plain row shape instead of borrowing the card.
  //
  // Not ported: the oracle's fourth row, "Xbox · add friend" (the broadcast
  // alt account's gamertag) -- P12.12 found /v1/broadcast/credentials is
  // POST-only in the frozen contract, so there is no real value to show.
  import type { Schema, ScreenApi } from '../../../sections/shared/types';
  import { call } from '../../../sections/shared/types';

  export let api: ScreenApi | undefined = undefined;
  export let serverType: string | undefined = undefined;
  export let activeServerId: string | undefined = undefined;

  let connectivity: Schema['ConnectivityResponseDTO'] | undefined;
  let geyser: Schema['GeyserConfigResponseDTO'] | undefined;
  let showAddresses = false;
  let copiedRow = '';
  let loadedForServerId: string | undefined;

  $: isBedrockServer = serverType === 'bedrock';
  $: hasGeyser = !isBedrockServer && geyser?.isGeyserInstalled && geyser?.port !== undefined;

  // /v1/connectivity's joinAddress already carries "host:port" combined
  // (crates/msc-application/src/network_diagnostics.rs), so it's used as a
  // full value directly, with no separate port field the way the LAN row
  // would need one if the agent ever starts reporting a LAN address.
  $: publicSuffix = connectivity?.joinAddressSource === 'playit' ? 'anywhere' : 'public';

  interface ConnectRow {
    key: string;
    label: string;
    icon: 'cup' | 'box' | 'globe';
    tone: 'warn' | 'ok' | 'bedrock';
    value: string | undefined;
    fallback: string;
  }

  $: rows = ((): ConnectRow[] => {
    const out: ConnectRow[] = [
      {
        key: 'java-lan',
        label: 'Java · same Wi-Fi',
        icon: 'cup',
        tone: 'warn',
        value: undefined,
        fallback: 'Not reported by this host yet',
      },
      {
        key: 'java-public',
        label: `Java · ${publicSuffix}`,
        icon: 'globe',
        tone: 'ok',
        value: connectivity?.joinAddress,
        fallback: 'Not available yet',
      },
    ];
    if (isBedrockServer || hasGeyser) {
      out.push(
        {
          key: 'bedrock-lan',
          label: 'Bedrock · same Wi-Fi',
          icon: 'box',
          tone: 'bedrock',
          value: geyser?.address ? `${geyser.address}:${geyser.port}` : undefined,
          fallback: '0.0.0.0',
        },
        {
          key: 'bedrock-public',
          label: `Bedrock · ${publicSuffix}`,
          icon: 'globe',
          tone: 'ok',
          value: connectivity?.joinAddress,
          fallback: 'Not available yet',
        },
      );
    }
    return out;
  })();

  $: if (activeServerId !== loadedForServerId) {
    loadedForServerId = activeServerId;
    void load();
  }

  async function load(): Promise<void> {
    connectivity = await call(api, connectivity, '/v1/connectivity');
    geyser = await call(api, geyser, '/v1/config/geyser');
  }

  function mask(value: string): string {
    return showAddresses ? value : '•'.repeat(Math.min(value.length, 15));
  }

  async function copy(key: string, value: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(value);
      copiedRow = key;
      setTimeout(() => {
        if (copiedRow === key) copiedRow = '';
      }, 1500);
    } catch {
      // Clipboard access denied — the value is still visible to select manually.
    }
  }
</script>

<div class="how-to-connect">
  <button type="button" class="eye" onclick={() => (showAddresses = !showAddresses)}>
    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      {#if showAddresses}
        <path
          d="M3 3l18 18M10.6 10.6a3 3 0 0 0 4.2 4.2M6.6 6.6C4.3 8.1 2.7 10 2 12c1.5 3.8 5.5 7 10 7 1.6 0 3.1-.4 4.5-1.1M17.4 17.4C19.5 15.9 21.1 14 22 12c-1.1-2.8-3.4-5.2-6.3-6.5"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      {:else}
        <path
          d="M2 12c1.5-3.8 5.5-7 10-7s8.5 3.2 10 7c-1.5 3.8-5.5 7-10 7s-8.5-3.2-10-7z"
          stroke="currentColor"
          stroke-width="1.8"
          stroke-linejoin="round"
        />
        <circle cx="12" cy="12" r="3" stroke="currentColor" stroke-width="1.8" />
      {/if}
    </svg>
    <span>{showAddresses ? 'Hide' : 'Show'}</span>
  </button>

  {#each rows as row (row.key)}
    <div class="row">
      <div class="row-header">
        <span class="row-icon tone-{row.tone}">
          {#if row.icon === 'cup'}
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true">
              <path
                d="M5 4h10v9a5 5 0 0 1-10 0zM15 7h2a2 2 0 0 1 0 4h-2"
                stroke="currentColor"
                stroke-width="1.8"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          {:else if row.icon === 'box'}
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true">
              <path
                d="M12 3l8 4.5v9L12 21l-8-4.5v-9zM12 3v18M4 7.5l8 4.5 8-4.5"
                stroke="currentColor"
                stroke-width="1.8"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          {:else}
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true">
              <path
                d="M3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18"
                stroke="currentColor"
                stroke-width="1.8"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          {/if}
        </span>
        <span class="row-label">{row.label}</span>
      </div>
      {#if row.value}
        <button
          type="button"
          class="pill tone-{row.tone}"
          onclick={() => copy(row.key, row.value ?? '')}
        >
          <span class="pill-value mono">{mask(row.value)}</span>
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path
              d="M4 4h12v12H4z"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linejoin="round"
            />
            <path
              d="M8 8h12v12H8z"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linejoin="round"
            />
          </svg>
          <span class="sr-only">{copiedRow === row.key ? 'Copied' : `Copy ${row.label}`}</span>
        </button>
      {:else}
        <p class="unavailable">{row.fallback}</p>
      {/if}
    </div>
  {/each}
</div>

<style>
  .how-to-connect {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .eye {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    align-self: flex-start;
    padding: 0;
    font: inherit;
    font-size: 11px;
    font-weight: 500;
    color: var(--msc2-text-tertiary);
    background: transparent;
    border: none;
    cursor: pointer;
  }
  .eye:hover {
    color: var(--msc2-text-secondary);
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .row-header {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .row-icon {
    display: inline-flex;
    flex-shrink: 0;
  }
  .row-icon.tone-warn {
    color: var(--msc2-status-warn);
  }
  .row-icon.tone-ok {
    color: var(--msc2-status-ok);
  }
  .row-icon.tone-bedrock {
    color: var(--msc2-status-bedrock);
  }
  .row-label {
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .unavailable {
    margin: 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .pill {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    padding: 8px 10px;
    font: inherit;
    text-align: left;
    border: none;
    border-radius: 8px;
    color: var(--msc2-text-primary);
    cursor: pointer;
  }
  .pill.tone-warn {
    background: var(--msc2-status-warn-tint);
  }
  .pill.tone-ok {
    background: var(--msc2-status-ok-tint);
  }
  .pill.tone-bedrock {
    background: var(--msc2-status-bedrock-tint);
  }
  .pill svg {
    flex-shrink: 0;
    opacity: 0.7;
  }
  .pill-value {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mono {
    font-family: var(--msc2-font-mono);
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
