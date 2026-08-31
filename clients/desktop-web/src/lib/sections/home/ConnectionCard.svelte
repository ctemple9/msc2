<script lang="ts">
  // MSC 1 OverviewConnectionCardView, ported to the S0 disciplined card —
  // no gradient rails, status carried by dot + label only. The agent reports
  // the host's best-effort LAN address through
  // ServerDTO.hostAddress. If discovery is unavailable, the Local column
  // keeps the honest fallback rather than fabricating an address.
  import Card from '../../components/base/Card.svelte';
  import SegmentedControl from '../../components/base/SegmentedControl.svelte';
  import Button from '../../components/base/Button.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import type { Schema } from '../shared/types';

  export let serverType: string | undefined = undefined;
  export let gamePort: number | undefined = undefined;
  export let hostAddress: string | undefined = undefined;
  export let geyser: Schema['GeyserConfigResponseDTO'] | undefined = undefined;
  export let connectivity: Schema['ConnectivityResponseDTO'] | undefined = undefined;
  export let playit: Schema['PlayitStatusResponseDTO'] | undefined = undefined;

  let showPublic = false;
  let copiedLabel = '';

  $: isBedrockServer = serverType === 'bedrock';
  $: hasGeyser = !isBedrockServer && geyser?.isGeyserInstalled && geyser?.port !== undefined;

  type Endpoint = { host: string; port?: number };

  function splitEndpoint(value: string | undefined): Endpoint | undefined {
    const trimmed = value?.trim();
    if (!trimmed) return undefined;
    if (trimmed.startsWith('[')) {
      const close = trimmed.indexOf(']');
      if (close > 0) {
        const portText = trimmed.slice(close + 1).replace(/^:/, '');
        return {
          host: trimmed.slice(1, close),
          port: /^\d+$/.test(portText) ? Number(portText) : undefined,
        };
      }
    }
    if ((trimmed.match(/:/g) ?? []).length !== 1) return { host: trimmed };
    const separator = trimmed.lastIndexOf(':');
    const portText = separator > 0 ? trimmed.slice(separator + 1) : '';
    return separator > 0 && /^\d+$/.test(portText)
      ? { host: trimmed.slice(0, separator), port: Number(portText) }
      : { host: trimmed };
  }

  function publicEndpoint(
    value: string | undefined,
    fallbackPort: number | undefined,
  ): Endpoint | undefined {
    const endpoint = splitEndpoint(value);
    return endpoint ? { ...endpoint, port: fallbackPort ?? endpoint.port } : undefined;
  }

  function endpointText(endpoint: Endpoint | undefined): string | undefined {
    if (!endpoint) return undefined;
    const host = endpoint.host.includes(':') ? `[${endpoint.host}]` : endpoint.host;
    return endpoint.port !== undefined ? `${host}:${endpoint.port}` : host;
  }

  $: playitSelected = playit?.playitEnabled === true;
  $: playitJavaAddress = playitSelected ? playit?.javaAddress : undefined;
  $: playitBedrockAddress = playitSelected ? playit?.bedrockAddress : undefined;
  // Once Playit is selected, do not briefly display a forwarding/DuckDNS
  // endpoint while Playit is still starting or has not saved its address.
  $: publicJavaValue =
    playit === undefined
      ? undefined
      : playitSelected
        ? playitJavaAddress
        : connectivity?.joinAddress;
  $: publicBedrockValue =
    playit === undefined
      ? undefined
      : playitSelected
        ? playitBedrockAddress
        : connectivity?.joinAddress;
  // A plain public hostname (DuckDNS/public IP) forwards to the local port;
  // Playit supplies its own endpoint and therefore its own public port.
  $: publicJavaEndpoint = publicEndpoint(publicJavaValue, playitSelected ? undefined : gamePort);
  $: publicBedrockEndpoint = publicEndpoint(
    publicBedrockValue,
    playitSelected ? undefined : isBedrockServer ? gamePort : geyser?.port,
  );

  function sourceTag(
    isPublic: boolean,
    isPlayit: boolean,
    addressSource: string | undefined,
  ): string {
    if (!isPublic) return 'LAN';
    if (isPlayit) return 'playit.gg';
    if (addressSource === 'duckdns') return 'DuckDNS';
    if (addressSource === 'public_ip') return 'Public IP';
    return 'Unavailable';
  }

  $: javaSourceTag = sourceTag(
    showPublic,
    playitSelected,
    playit === undefined ? undefined : connectivity?.joinAddressSource,
  );
  $: bedrockSourceTag = sourceTag(
    showPublic,
    playitSelected,
    playit === undefined ? undefined : connectivity?.joinAddressSource,
  );

  // Local uses the detected host address. Public uses the endpoint belonging
  // to that protocol, including a different tunnel port when one exists.
  $: javaIp = showPublic ? (publicJavaEndpoint?.host ?? null) : (hostAddress ?? null);
  $: javaIpFallback = showPublic ? 'Not available yet' : 'Not reported by this host yet';
  $: javaDisplayPort = showPublic
    ? (publicJavaEndpoint?.port ?? (playit === undefined || playitSelected ? undefined : gamePort))
    : gamePort;
  $: geyserIp = showPublic ? (publicBedrockEndpoint?.host ?? null) : (hostAddress ?? null);
  $: geyserIpFallback = showPublic ? 'Not available yet' : 'Not reported by this host yet';
  $: geyserDisplayPort = showPublic
    ? (publicBedrockEndpoint?.port ??
      (playit === undefined || playitSelected
        ? undefined
        : isBedrockServer
          ? gamePort
          : geyser?.port))
    : geyser?.port;
  $: javaCopyValue = showPublic
    ? endpointText(publicJavaEndpoint)
    : gamePort !== undefined
      ? String(gamePort)
      : undefined;
  $: bedrockCopyValue = endpointText(
    showPublic
      ? publicBedrockEndpoint
      : hostAddress && geyserDisplayPort !== undefined
        ? { host: hostAddress, port: geyserDisplayPort }
        : undefined,
  );

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
        <span class="source-tag">{javaSourceTag}</span>
      </div>
      <span class="label">IP</span>
      {#if javaIp}
        <p class="value mono">{javaIp}</p>
      {:else}
        <p class="value mono muted-value">{javaIpFallback}</p>
      {/if}
      <span class="label">Port</span>
      <p class="value mono">
        {javaDisplayPort !== undefined ? javaDisplayPort : '—'}
      </p>
      <Button
        variant="secondary"
        size="sm"
        disabled={!javaCopyValue}
        onclick={() => javaCopyValue && copy('Java', javaCopyValue)}
      >
        {copiedLabel === 'Java' ? 'Copied' : showPublic ? 'Copy address' : 'Copy port'}
      </Button>
    </div>

    {#if !isBedrockServer}
      {#if hasGeyser}
        <div class="cell">
          <div class="cell-header">
            <span class="dot" class:online={!showPublic || !!geyserIp}></span>
            <span class="platform-label">Bedrock (Geyser)</span>
            <span class="source-tag">{bedrockSourceTag}</span>
          </div>
          <span class="label">IP</span>
          {#if geyserIp}
            <p class="value mono">{geyserIp}</p>
          {:else}
            <p class="value mono muted-value">{geyserIpFallback}</p>
          {/if}
          <span class="label">Port</span>
          <p class="value mono">
            {geyserDisplayPort !== undefined ? geyserDisplayPort : '—'}
          </p>
          <Button
            variant="secondary"
            size="sm"
            disabled={!bedrockCopyValue}
            onclick={() => bedrockCopyValue && copy('Bedrock', bedrockCopyValue)}
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
