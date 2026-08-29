<script lang="ts">
  // Ports MSC 1 SidebarView.swift's HowToConnectSidebarSection -- a plain
  // list (icon+label was tried and dropped per Cameron's own visual review;
  // the eye toggle moved up into the disclosure header, see ControlSidebar)
  // of one pill-shaped value row per connection method, every method with
  // data shown at once (no Local/Public switch). Keeps ConnectionCard's
  // exact data and derivation (/v1/connectivity, /v1/config/geyser, the
  // same honest "not reported yet" fallback for Java's LAN address -- the
  // agent still doesn't report one, ConnectionCard.svelte's own finding
  // still holds).
  //
  // Not ported: the oracle's fourth row, "Xbox · add friend" (the broadcast
  // alt account's gamertag) -- P12.12 found /v1/broadcast/credentials is
  // POST-only in the frozen contract, so there is no real value to show.
  import type { Schema, ScreenApi } from '../../../sections/shared/types';
  import { call } from '../../../sections/shared/types';

  export let api: ScreenApi | undefined = undefined;
  export let serverType: string | undefined = undefined;
  export let activeServerId: string | undefined = undefined;
  // Owned by ControlSidebar -- shared with the eye toggle it renders in the
  // disclosure header row, above this component's own content.
  export let showAddresses = false;

  let connectivity: Schema['ConnectivityResponseDTO'] | undefined;
  let geyser: Schema['GeyserConfigResponseDTO'] | undefined;
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
    tone: 'warn' | 'ok' | 'bedrock';
    value: string | undefined;
    fallback: string;
  }

  $: rows = ((): ConnectRow[] => {
    const out: ConnectRow[] = [
      {
        key: 'java-lan',
        label: 'Java · same Wi-Fi',
        tone: 'warn',
        value: undefined,
        fallback: 'Not reported by this host yet',
      },
      {
        key: 'java-public',
        label: `Java · ${publicSuffix}`,
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
          tone: 'bedrock',
          value: geyser?.address ? `${geyser.address}:${geyser.port}` : undefined,
          fallback: '0.0.0.0',
        },
        {
          key: 'bedrock-public',
          label: `Bedrock · ${publicSuffix}`,
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
  {#each rows as row (row.key)}
    <div class="row">
      <span class="row-label">{row.label}</span>
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
  .row {
    display: flex;
    flex-direction: column;
    gap: 5px;
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
