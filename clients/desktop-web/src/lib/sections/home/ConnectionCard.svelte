<script lang="ts">
  // MSC 1 OverviewConnectionCardView, ported to the S0 disciplined card —
  // no gradient rails, status carried by dot + label only. The agent does
  // not yet report a local LAN IP (ServerDTO.hostAddress is always null —
  // crates/msc-api/src/dto/lifecycle.rs), so the Local column shows the
  // real port and states the address honestly rather than fabricating one.
  import Card from '../../components/base/Card.svelte';
  import SegmentedControl from '../../components/base/SegmentedControl.svelte';
  import Button from '../../components/base/Button.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import type { Schema } from '../shared/types';

  export let serverType: string | undefined = undefined;
  export let gamePort: number | undefined = undefined;
  export let geyser: Schema['GeyserConfigResponseDTO'] | undefined = undefined;
  export let connectivity: Schema['ConnectivityResponseDTO'] | undefined = undefined;

  let showPublic = false;
  let showAddresses = true;
  let copiedLabel = '';

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

  // Local shows the real port and states the address honestly (the agent
  // doesn't report a LAN IP yet). Public swaps in the resolved join
  // address but keeps the same port field — a tunnel/DNS name generally
  // still forwards to the same numeric port, and the contract has no
  // separate "public port" concept for the plain case.
  $: javaIp = showPublic ? (connectivity?.joinAddress ?? null) : null;
  $: javaIpFallback = showPublic ? 'Not available yet' : 'Not reported by this host yet';
  $: geyserIp = showPublic ? (connectivity?.joinAddress ?? null) : (geyser?.address ?? null);
  $: geyserIpFallback = showPublic ? 'Not available yet' : '0.0.0.0';

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

<Card padding="14px 16px">
  <div class="header">
    <div class="overline">
      <Icon name="network" size={12} />
      <span class="msc2-type-overline">Connection Info</span>
    </div>
    <div class="header-actions">
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
  </div>

  <div class="columns" class:single={isBedrockServer}>
    <div class="cell">
      <div class="cell-header">
        <span class="dot" class:online={!showPublic || !!javaIp}></span>
        <span class="platform-label"
          >{isBedrockServer ? 'Bedrock · Dedicated' : 'Java · PC / Mac'}</span
        >
        <span class="source-tag">{sourceTag}</span>
      </div>
      <span class="label">IP</span>
      {#if javaIp}
        <p class="value mono">{mask(javaIp)}</p>
      {:else}
        <p class="value mono muted-value">{javaIpFallback}</p>
      {/if}
      <span class="label">Port</span>
      <p class="value mono">{gamePort !== undefined ? mask(String(gamePort)) : '—'}</p>
      <Button
        variant="secondary"
        size="sm"
        disabled={gamePort === undefined}
        onclick={() => gamePort !== undefined && copy('Java', String(gamePort))}
      >
        {copiedLabel === 'Java' ? 'Copied' : 'Copy port'}
      </Button>
    </div>

    {#if !isBedrockServer}
      {#if hasGeyser}
        <div class="cell">
          <div class="cell-header">
            <span class="dot" class:online={!showPublic || !!geyserIp}></span>
            <span class="platform-label">Bedrock (Geyser)</span>
            <span class="source-tag">{sourceTag}</span>
          </div>
          <span class="label">IP</span>
          {#if geyserIp}
            <p class="value mono">{mask(geyserIp)}</p>
          {:else}
            <p class="value mono muted-value">{geyserIpFallback}</p>
          {/if}
          <span class="label">Port</span>
          <p class="value mono">{mask(String(geyser?.port))}</p>
          <Button
            variant="secondary"
            size="sm"
            disabled={!geyserIp}
            onclick={() => geyserIp && copy('Bedrock', `${geyserIp}:${geyser?.port}`)}
          >
            {copiedLabel === 'Bedrock' ? 'Copied' : 'Copy address'}
          </Button>
        </div>
      {:else}
        <div class="cell ghost">
          <div class="cell-header">
            <span class="dot"></span>
            <span class="platform-label muted">Bedrock (Geyser)</span>
          </div>
          <p class="unavailable">Not configured</p>
          <p class="unavailable-hint">Enable in Settings</p>
        </div>
      {/if}
    {/if}
  </div>
</Card>

<style>
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 12px;
    color: var(--msc2-text-tertiary);
  }
  .overline {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .eye {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
  }
  .eye:hover {
    color: var(--msc2-text-secondary);
  }
  .columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .columns.single {
    grid-template-columns: 1fr;
  }
  .cell {
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .cell.ghost {
    background: rgba(255, 255, 255, 0.02);
    border-style: dashed;
    opacity: 0.6;
  }
  .cell-header {
    display: flex;
    align-items: center;
    gap: 5px;
    margin-bottom: 6px;
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
    letter-spacing: 0.6px;
    text-transform: uppercase;
    color: var(--msc2-text-tertiary);
  }
  .platform-label.muted {
    opacity: 0.6;
  }
  .source-tag {
    margin-left: auto;
    font-size: 9px;
    font-weight: 600;
    color: var(--msc2-text-secondary);
    background: rgba(255, 255, 255, 0.08);
    padding: 2px 6px;
    border-radius: 20px;
  }
  .label {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.6px;
    text-transform: uppercase;
    color: var(--msc2-text-tertiary);
    margin-top: 4px;
  }
  .value {
    margin: 2px 0 6px;
    font-size: 13px;
    color: var(--msc2-text-primary);
  }
  .value.mono {
    font-family: var(--msc2-font-mono);
  }
  .muted-value {
    color: var(--msc2-text-tertiary);
    font-size: 11px;
  }
  .unavailable {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .unavailable-hint {
    margin: 2px 0 0;
    font-size: 10px;
    color: var(--msc2-text-tertiary);
  }
</style>
