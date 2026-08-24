<script lang="ts">
  import { onMount } from 'svelte';
  import ActionButton from '../../components/ActionButton.svelte';
  import StatusBadge from '../../components/StatusBadge.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import CapabilityNotice from '../shared/CapabilityNotice.svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, errorMessage, mutate } from '../shared/types';

  export let api: ScreenProps['api'] = undefined;
  let connectivity: Schema['ConnectivityResponseDTO'] = {
    headline: 'No connectivity check yet',
    method: 'unavailable',
    serverName: 'Survival',
    serverRunning: false,
    serverType: 'paper',
    severity: 'unknown',
    status: 'Unknown',
  };
  let playit: Schema['PlayitStatusResponseDTO'] = {
    hasSecretKey: false,
    isRunning: false,
    playitEnabled: false,
    serverName: 'Survival',
    serverType: 'paper',
    voiceChatEnabled: false,
  };
  let duckdns: Schema['DuckDNSStatusResponseDTO'] = { isConfigured: false };
  let resourcePacks: Schema['ResourcePacksResponseDTO'] = {
    geyserPacks: [],
    isGeyserAvailable: false,
    isJava: true,
    packs: [],
    requirePack: false,
    serverType: 'paper',
  };
  let broadcast: Schema['BroadcastStatusDTO'] = {
    bedrockBroadcastRunning: false,
    xboxBroadcastRunning: false,
  };
  let duckHost = '';
  let notice = '';
  onMount(async () => {
    connectivity = await call(api, connectivity, '/v1/connectivity');
    playit = await call(api, playit, '/v1/playit');
    duckdns = await call(api, duckdns, '/v1/duckdns');
    resourcePacks = await call(api, resourcePacks, '/v1/resourcepacks');
    broadcast = await call(api, broadcast, '/v1/broadcast/status');
  });
  async function post<T>(path: string, body?: unknown): Promise<T | undefined> {
    try {
      return await mutate<T>(api, path, body);
    } catch (error) {
      notice = errorMessage(error);
      return undefined;
    }
  }
  async function togglePlayit(): Promise<void> {
    const result = await post<Schema['PlayitActionResultDTO']>(
      playit.isRunning ? '/v1/playit/stop' : '/v1/playit/start',
    );
    if (result) {
      playit = { ...playit, isRunning: !playit.isRunning };
      notice = result.message ?? result.result;
    }
  }
  async function saveDuckDNS(): Promise<void> {
    const result = await post<Schema['DuckDNSUpdateResultDTO']>('/v1/duckdns', {
      hostname: duckHost,
    });
    if (result) {
      duckdns = { ...duckdns, hostname: result.hostname };
      notice = result.message ?? 'DuckDNS label saved.';
    }
  }
  async function togglePack(pack: Schema['ResourcePackItemDTO']): Promise<void> {
    const result = await post<Schema['ResourcePackMutationResultDTO']>('/v1/resourcepacks/toggle', {
      packId: pack.id,
      enabled: !pack.isActive,
    });
    if (result?.updated) resourcePacks = result.updated;
  }
  async function toggleBroadcast(): Promise<void> {
    const result = await post<Schema['BroadcastSimpleResultDTO']>(
      broadcast.xboxBroadcastRunning ? '/v1/broadcast/stop' : '/v1/broadcast/start',
    );
    if (result) {
      broadcast = { ...broadcast, xboxBroadcastRunning: !broadcast.xboxBroadcastRunning };
      notice = result.result;
    }
  }
</script>

<div class="screen">
  <ScreenHeader
    eyebrow="Reachability"
    title="Networking"
    description="Diagnostics distinguish local listening, public reachability, helper state, and the join address instead of promising a connection the agent did not prove."
    status={connectivity.status}
    statusTone={connectivity.severity === 'ok' ? 'positive' : 'warning'}
    actionLabel="Run diagnostics"
    onAction={async () => (connectivity = await call(api, connectivity, '/v1/connectivity'))}
  />
  {#if notice}<p class="muted" role="status">{notice}</p>{/if}
  {#if connectivity.helpId}<CapabilityNotice
      title={connectivity.headline}
      message={connectivity.detail ??
        connectivity.note ??
        'Open the linked help topic for the reason.'}
      helpId={connectivity.helpId}
    />{/if}
  <div class="screen-grid three">
    <section class="screen-card">
      <span class="metric-label">Join address</span>
      <p class="metric-large">{connectivity.joinAddress ?? 'Unavailable'}</p>
      <p class="muted">
        Source: {connectivity.joinAddressSource ?? 'unavailable'} · {connectivity.method}
      </p>
    </section>
    <section class="screen-card">
      <span class="metric-label">Local listener</span>
      <p class="metric-large">{connectivity.localListening ? 'Open' : 'Not proven'}</p>
      <StatusBadge
        status={connectivity.localListening ? 'Listening' : 'Unavailable'}
        tone={connectivity.localListening ? 'positive' : 'warning'}
      />
    </section>
    <section class="screen-card">
      <span class="metric-label">Public reachability</span>
      <p class="metric-large">{connectivity.externallyReachable ? 'Reachable' : 'Not proven'}</p>
      <p class="muted">{connectivity.portDiagnostics?.public.detail ?? 'No public test result.'}</p>
    </section>
  </div>
  <div class="screen-grid">
    <section class="screen-card">
      <h3>Playit</h3>
      <p>{playit.note ?? 'Managed helper state is reported by the agent.'}</p>
      <StatusBadge
        status={playit.isRunning ? 'Running' : 'Stopped'}
        tone={playit.isRunning ? 'positive' : 'neutral'}
      />
      <div class="screen-actions" style="margin-top: .7rem">
        <ActionButton
          label={playit.isRunning ? 'Stop Playit' : 'Start Playit'}
          disabled={!playit.playitEnabled}
          onclick={togglePlayit}>{playit.isRunning ? 'Stop' : 'Start'}</ActionButton
        >
      </div>
    </section>
    <section class="screen-card">
      <h3>DuckDNS label</h3>
      <p>
        DuckDNS supplies a name; it does not replace authentication or the loopback/Tailscale
        management boundary.
      </p>
      <div class="inline-form">
        <div class="field">
          <label for="duckdns-host">Hostname</label><input
            id="duckdns-host"
            bind:value={duckHost}
            placeholder={duckdns.hostname ?? 'example.duckdns.org'}
          />
        </div>
        <ActionButton label="Save DuckDNS" onclick={saveDuckDNS}>Save</ActionButton>
      </div>
    </section>
    <section class="screen-card">
      <h3>Xbox Broadcast</h3>
      <p>Broadcast helper state is explicit and remains separate from server lifecycle.</p>
      <StatusBadge
        status={broadcast.xboxBroadcastRunning ? 'Running' : 'Stopped'}
        tone={broadcast.xboxBroadcastRunning ? 'positive' : 'neutral'}
      />
      <div class="screen-actions" style="margin-top: .7rem">
        <ActionButton label="Toggle Xbox Broadcast" onclick={toggleBroadcast}
          >{broadcast.xboxBroadcastRunning ? 'Stop' : 'Start'}</ActionButton
        >
      </div>
    </section>
  </div>
  <section class="screen-card">
    <div class="screen-card-header">
      <h3>Resource packs</h3>
      <span class="metric-label">{resourcePacks.packs.length} Java packs</span>
    </div>
    {#each resourcePacks.packs as pack (pack.id)}<div class="operation-row">
        <div>
          <strong>{pack.name}</strong>
          <p>{pack.typeLabel} · {pack.fileSizeDisplay}</p>
        </div>
        <ActionButton kind="quiet" label="Toggle resource pack" onclick={() => togglePack(pack)}
          >{pack.isActive ? 'Disable' : 'Enable'}</ActionButton
        >
      </div>{:else}<p class="muted">No resource packs installed.</p>{/each}
  </section>
</div>
