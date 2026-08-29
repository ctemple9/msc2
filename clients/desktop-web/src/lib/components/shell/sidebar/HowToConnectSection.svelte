<script lang="ts">
  // Ports MSC 1 SidebarView.swift's HowToConnectSidebarSection -- a compact,
  // per-method connection-address reference for the active server, condensed
  // into the sidebar's narrow column. Reuses the exact data (/v1/connectivity,
  // /v1/config/geyser) and derivation the Overview tab's Connection Info card
  // already established (ConnectionCard.svelte), so the two surfaces never
  // disagree about what "the join address" is.
  //
  // Not ported: the oracle's fourth row, "Xbox · add friend" (the broadcast
  // alt account's gamertag). P12.12 found /v1/broadcast/credentials is
  // POST-only in the frozen contract -- nothing reads a saved gamertag back
  // -- so there is no real value to show here.
  import Button from '../../base/Button.svelte';
  import SegmentedControl from '../../base/SegmentedControl.svelte';
  import type { Schema, ScreenApi } from '../../../sections/shared/types';
  import { call } from '../../../sections/shared/types';

  export let api: ScreenApi | undefined = undefined;
  export let serverType: string | undefined = undefined;
  export let gamePort: number | undefined = undefined;
  export let activeServerId: string | undefined = undefined;

  let connectivity: Schema['ConnectivityResponseDTO'] | undefined;
  let geyser: Schema['GeyserConfigResponseDTO'] | undefined;
  let showPublic = false;
  let showAddresses = true;
  let copiedLabel = '';
  let loadedForServerId: string | undefined;

  $: isBedrockServer = serverType === 'bedrock';
  $: hasGeyser = !isBedrockServer && geyser?.isGeyserInstalled && geyser?.port !== undefined;

  $: sourceTag = !showPublic
    ? 'LAN'
    : connectivity?.joinAddressSource === 'playit'
      ? 'playit.gg'
      : connectivity?.joinAddressSource === 'duckdns'
        ? 'DuckDNS'
        : connectivity?.joinAddressSource === 'public_ip'
          ? 'Public IP'
          : 'Unavailable';

  // The agent doesn't report a LAN IP yet (ConnectionCard.svelte's own
  // finding still holds), so Local always states that honestly rather than
  // fabricating an address.
  $: javaIp = showPublic ? (connectivity?.joinAddress ?? null) : null;
  $: javaIpFallback = showPublic ? 'Not available yet' : 'Not reported by this host yet';
  $: geyserIp = showPublic ? (connectivity?.joinAddress ?? null) : (geyser?.address ?? null);
  $: geyserIpFallback = showPublic ? 'Not available yet' : '0.0.0.0';

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

  async function copy(label: string, value: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(value);
      copiedLabel = label;
      setTimeout(() => {
        if (copiedLabel === label) copiedLabel = '';
      }, 1500);
    } catch {
      // Clipboard access denied — the value is still visible to select manually.
    }
  }
</script>

<div class="how-to-connect">
  <div class="header-row">
    <button
      type="button"
      class="eye"
      onclick={() => (showAddresses = !showAddresses)}
      aria-label={showAddresses ? 'Hide addresses' : 'Show addresses'}
    >
      {showAddresses ? 'Hide' : 'Show'}
    </button>
    <SegmentedControl
      options={[
        { value: 'local', label: 'Local' },
        { value: 'public', label: 'Public' },
      ]}
      value={showPublic ? 'public' : 'local'}
      onchange={(v) => (showPublic = v === 'public')}
    />
  </div>

  <div class="cell">
    <div class="cell-header">
      <span class="dot" class:online={!showPublic || !!javaIp}></span>
      <span class="platform-label"
        >{isBedrockServer ? 'Bedrock · Dedicated' : 'Java · PC / Mac'}</span
      >
      <span class="source-tag">{sourceTag}</span>
    </div>
    {#if javaIp}
      <p class="value mono">{mask(javaIp)}</p>
    {:else}
      <p class="value mono muted-value">{javaIpFallback}</p>
    {/if}
    <div class="cell-footer">
      <span class="port-label">Port {gamePort !== undefined ? mask(String(gamePort)) : '—'}</span>
      <Button
        variant="secondary"
        size="sm"
        disabled={gamePort === undefined}
        onclick={() => gamePort !== undefined && copy('Java', String(gamePort))}
      >
        {copiedLabel === 'Java' ? 'Copied' : 'Copy port'}
      </Button>
    </div>
  </div>

  {#if !isBedrockServer}
    {#if hasGeyser}
      <div class="cell">
        <div class="cell-header">
          <span class="dot" class:online={!showPublic || !!geyserIp}></span>
          <span class="platform-label">Bedrock (Geyser)</span>
          <span class="source-tag">{sourceTag}</span>
        </div>
        {#if geyserIp}
          <p class="value mono">{mask(`${geyserIp}:${geyser?.port}`)}</p>
        {:else}
          <p class="value mono muted-value">{geyserIpFallback}</p>
        {/if}
        <div class="cell-footer">
          <Button
            variant="secondary"
            size="sm"
            disabled={!geyserIp}
            onclick={() => geyserIp && copy('Bedrock', `${geyserIp}:${geyser?.port}`)}
          >
            {copiedLabel === 'Bedrock' ? 'Copied' : 'Copy address'}
          </Button>
        </div>
      </div>
    {:else}
      <p class="unavailable">Bedrock (Geyser) not configured</p>
    {/if}
  {/if}
</div>

<style>
  .how-to-connect {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .eye {
    font-size: 10px;
    color: var(--msc2-text-tertiary);
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
  }
  .eye:hover {
    color: var(--msc2-text-secondary);
  }
  .cell {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 8px 9px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .cell-header {
    display: flex;
    align-items: center;
    gap: 5px;
    margin-bottom: 4px;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--msc2-neutral-muted);
    flex-shrink: 0;
  }
  .dot.online {
    background: var(--msc2-status-ok);
  }
  .platform-label {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.5px;
    text-transform: uppercase;
    color: var(--msc2-text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .source-tag {
    margin-left: auto;
    flex-shrink: 0;
    font-size: 9px;
    font-weight: 600;
    color: var(--msc2-text-secondary);
    background: rgba(255, 255, 255, 0.08);
    padding: 2px 6px;
    border-radius: 20px;
  }
  .value {
    margin: 0 0 6px;
    font-size: 12px;
    color: var(--msc2-text-primary);
  }
  .value.mono {
    font-family: var(--msc2-font-mono);
    word-break: break-all;
  }
  .muted-value {
    color: var(--msc2-text-tertiary);
    font-size: 11px;
  }
  .cell-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }
  .port-label {
    font-size: 10px;
    color: var(--msc2-text-tertiary);
    font-family: var(--msc2-font-mono);
  }
  .unavailable {
    margin: 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
</style>
