<script lang="ts">
  // Ports MSC 1 SidebarView.swift's HowToConnectSidebarSection -- a plain
  // list (icon+label was tried and dropped per Cameron's own visual review;
  // addresses are always shown here so connection values are immediately usable)
  // of one pill-shaped value row per connection method, every method with
  // data shown at once (no Local/Public switch). Keeps ConnectionCard's
  // exact data and derivation (/v1/connectivity, /v1/config/geyser, and the
  // host address reported by /v1/servers). If address discovery is unavailable,
  // the same honest "not reported yet" fallback remains in place.
  //
  // Xbox's fourth row uses the authenticated gamertag from the existing
  // /v1/broadcast/status route. Credentials remain write-only; the gamertag is
  // the safe, displayable value the agent already exposes.
  import { onMount } from 'svelte';
  import type { Schema, ScreenApi } from '../../../sections/shared/types';
  import { call } from '../../../sections/shared/types';

  export let api: ScreenApi | undefined = undefined;
  export let serverType: string | undefined = undefined;
  export let activeServerId: string | undefined = undefined;
  export let gamePort: number | undefined = undefined;
  export let bedrockPort: number | undefined = undefined;
  export let hostAddress: string | undefined = undefined;
  export let showXboxBroadcast = false;
  export let xboxBroadcastEnabled = false;
  export let addressesVisible = false;

  let connectivity: Schema['ConnectivityResponseDTO'] | undefined;
  let geyser: Schema['GeyserConfigResponseDTO'] | undefined;
  let playit: Schema['PlayitStatusResponseDTO'] | undefined;
  let broadcast: Schema['BroadcastStatusDTO'] = {
    xboxBroadcastRunning: false,
    bedrockBroadcastRunning: false,
  };
  let copiedRow = '';
  let loadedForServerId: string | undefined;
  let refreshTimer: ReturnType<typeof setInterval> | undefined;

  $: isBedrockServer = serverType === 'bedrock';
  $: bedrockEndpointPort = isBedrockServer ? gamePort : (bedrockPort ?? geyser?.port);
  $: hasBedrockEndpoint = isBedrockServer || geyser?.isGeyserInstalled === true;

  // /v1/connectivity's joinAddress and Playit's protocol-specific addresses
  // carry "host:port" combined. Local endpoints use the detected host from
  // /v1/servers plus the relevant local port.
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

  function endpointText(
    value: string | undefined,
    fallbackPort: number | undefined,
  ): string | undefined {
    const endpoint = splitEndpoint(value);
    if (!endpoint) return undefined;
    const port = fallbackPort ?? endpoint.port;
    const host = endpoint.host.includes(':') ? `[${endpoint.host}]` : endpoint.host;
    return port !== undefined ? `${host}:${port}` : host;
  }

  $: playitSelected = playit?.playitEnabled === true;
  $: playitJavaAddress = playitSelected ? playit?.javaAddress : undefined;
  $: playitBedrockAddress = playitSelected ? playit?.bedrockAddress : undefined;

  function publicSuffix(playitSelected: boolean, addressSource: string | undefined): string {
    return playitSelected || addressSource === 'playit' ? 'anywhere' : 'public';
  }

  $: publicJavaAddress = endpointText(
    playit === undefined
      ? undefined
      : playitSelected
        ? playitJavaAddress
        : connectivity?.joinAddress,
    playit === undefined || playitSelected ? undefined : gamePort,
  );
  $: publicBedrockAddress = endpointText(
    playit === undefined
      ? undefined
      : playitSelected
        ? playitBedrockAddress
        : connectivity?.joinAddress,
    playit === undefined || playitSelected ? undefined : bedrockEndpointPort,
  );
  $: localJavaAddress = hostAddress ? endpointText(hostAddress, gamePort) : undefined;
  $: localBedrockAddress = hostAddress ? endpointText(hostAddress, bedrockEndpointPort) : undefined;

  interface ConnectRow {
    key: string;
    label: string;
    value: string | undefined;
    fallback: string;
  }

  $: rows = ((): ConnectRow[] => {
    const out: ConnectRow[] = [];
    if (isBedrockServer) {
      out.push(
        {
          key: 'bedrock-lan',
          label: 'Bedrock · same Wi-Fi',
          value: localBedrockAddress,
          fallback: 'Not reported by this host yet',
        },
        {
          key: 'bedrock-public',
          label: `Bedrock · ${publicSuffix(playitSelected, connectivity?.joinAddressSource)}`,
          value: publicBedrockAddress,
          fallback: 'Not available yet',
        },
      );
    } else {
      out.push(
        {
          key: 'java-lan',
          label: 'Java · same Wi-Fi',
          value: localJavaAddress,
          fallback: 'Not reported by this host yet',
        },
        {
          key: 'java-public',
          label: `Java · ${publicSuffix(playitSelected, connectivity?.joinAddressSource)}`,
          value: publicJavaAddress,
          fallback: 'Not available yet',
        },
      );
      if (hasBedrockEndpoint) {
        out.push(
          {
            key: 'bedrock-lan',
            label: 'Bedrock · same Wi-Fi',
            value: localBedrockAddress,
            fallback: 'Not reported by this host yet',
          },
          {
            key: 'bedrock-public',
            label: `Bedrock · ${publicSuffix(playitSelected, connectivity?.joinAddressSource)}`,
            value: publicBedrockAddress,
            fallback: 'Not available yet',
          },
        );
      }
    }
    if (showXboxBroadcast && xboxBroadcastEnabled) {
      out.push({
        key: 'xbox-friend',
        label: 'Console · add friend',
        value: broadcast.gamertag,
        fallback: 'Not signed in yet',
      });
    }
    return out;
  })();

  $: if (activeServerId !== loadedForServerId) {
    loadedForServerId = activeServerId;
    void load();
  }

  async function load(): Promise<void> {
    [connectivity, geyser, playit, broadcast] = await Promise.all([
      call(api, connectivity, '/v1/connectivity'),
      call(api, geyser, '/v1/config/geyser'),
      call(api, playit, '/v1/playit'),
      call<Schema['BroadcastStatusDTO']>(
        api,
        { xboxBroadcastRunning: false, bedrockBroadcastRunning: false },
        '/v1/broadcast/status',
      ),
    ]);
  }

  // Playit can save its public endpoint after this sidebar has already been
  // mounted, so refresh the read-only connection facts while the section is
  // visible instead of waiting for another server selection.
  onMount(() => {
    refreshTimer = setInterval(() => void load(), 8000);
    return () => {
      if (refreshTimer) clearInterval(refreshTimer);
    };
  });

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
      {#if row.value && addressesVisible}
        <button type="button" class="pill" onclick={() => copy(row.key, row.value ?? '')}>
          <span
            class="pill-value mono"
            class:scrollable={playitSelected &&
              (row.key === 'java-public' || row.key === 'bedrock-public')}>{row.value}</span
          >
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
      {:else if row.value}
        <p class="unavailable hidden-value">Hidden</p>
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
    border-radius: 8px;
    color: var(--msc2-text-primary);
    background: var(--msc2-tier-surface);
    border: 1px solid var(--msc2-hairline-subtle);
    cursor: pointer;
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
    font-size: 10px;
    line-height: 1.2;
  }
  .pill-value.scrollable {
    overflow-x: auto;
    overflow-y: hidden;
    text-overflow: clip;
    scrollbar-width: none;
    overscroll-behavior-x: contain;
  }
  .pill-value.scrollable::-webkit-scrollbar {
    display: none;
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
