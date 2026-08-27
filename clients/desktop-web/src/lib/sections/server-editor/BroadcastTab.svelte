<script lang="ts">
  // Ports ServerEditorBroadcastTab.swift (Java) -- Xbox broadcast, alt-account
  // credential notes, helper JAR install -- plus the Playit/DuckDNS/resource
  // pack panels P12.11 flagged as per-server and assigned here rather than
  // the host-level Manage sheet. Every route this tab calls
  // (crates/msc-agent/src/routes/networking.rs) resolves a single
  // agent-wide "active server", so -- like GeneralTab's Memory block -- the
  // whole tab is gated on `isActive` rather than risking a mutation landing
  // on the wrong server.
  //
  // Left out, not silently dropped: IP Mode (auto/public/private) and the
  // computed "transfers to host:port" preview have no backing route or field
  // at all (no XboxBroadcastIPMode get/set anywhere in the contract); "Reset
  // Xbox Sign-In" has no dedicated route either -- /v1/broadcast/restart
  // restarts the helper but doesn't clear cached credentials the way MSC 1's
  // reset does, so mapping "Reset Sign-In" to it would be dishonest. Host/
  // Port below are ServerDTO's own real hostAddress/gamePort fields instead
  // of the IP-mode-aware preview. Alt-account credentials
  // (/v1/broadcast/credentials) is POST-only in the contract -- nothing
  // reads them back -- so the fields start blank every time this tab opens,
  // matching xbox_broadcast_alt_password's own "Keychain-only; never decoded
  // from JSON" comment in crates/msc-domain/src/app_config_schema.rs.
  import Card from '../../components/base/Card.svelte';
  import Button from '../../components/base/Button.svelte';
  import Field from '../../components/base/Field.svelte';
  import Toggle from '../../components/base/Toggle.svelte';
  import StatusDot from '../../components/base/StatusDot.svelte';
  import ListRow from '../../components/base/ListRow.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { call, errorMessage, mutate } from '../shared/types';
  import { pollOperation, serverEditorPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let server: Schema['ServerDTO'];
  export let isActive = false;
  export let canControl = true;
  export let onRequestActivate: () => void;

  $: isJava = server.serverType !== 'bedrock';

  let status: Schema['BroadcastStatusDTO'] | undefined;
  let autostart: Schema['BroadcastAutoStartDTO'] | undefined;
  let jarStatus: Schema['BroadcastJarStatusDTO'] | undefined;
  let playit: Schema['PlayitStatusResponseDTO'] | undefined;
  let duckdns: Schema['DuckDNSStatusResponseDTO'] | undefined;
  let resourcePacks: Schema['ResourcePacksResponseDTO'] | undefined;

  let credEmail = '';
  let credGamertag = '';
  let credPassword = '';
  let showPassword = false;
  let credBusy = false;

  let duckHost = '';
  let duckBusy = false;

  let broadcastBusy = false;
  let jarBusy = false;
  let playitBusy = false;

  let notice = '';
  let loaded = false;

  $: broadcastRunning = isJava
    ? (status?.xboxBroadcastRunning ?? false)
    : (status?.bedrockBroadcastRunning ?? false);

  $: if (isActive && !loaded) {
    loaded = true;
    void loadAll();
  }
  $: if (!isActive) loaded = false;

  async function loadAll(): Promise<void> {
    [status, autostart, jarStatus, playit, duckdns, resourcePacks] = await Promise.all([
      call(api, status, serverEditorPaths.broadcastStatus),
      call(api, autostart, serverEditorPaths.broadcastAutostart),
      call(api, jarStatus, serverEditorPaths.broadcastJarStatus),
      call(api, playit, serverEditorPaths.playit),
      call(api, duckdns, serverEditorPaths.duckdns),
      call(api, resourcePacks, serverEditorPaths.resourcePacks),
    ]);
    duckHost = duckdns?.hostname ?? '';
  }

  async function toggleAutostart(enabled: boolean): Promise<void> {
    try {
      autostart = await mutate<Schema['BroadcastAutoStartDTO']>(
        api,
        serverEditorPaths.broadcastAutostart,
        { enabled },
      );
    } catch (error) {
      notice = errorMessage(error);
    }
  }

  async function toggleBroadcast(): Promise<void> {
    if (broadcastBusy) return;
    broadcastBusy = true;
    try {
      const path = broadcastRunning
        ? serverEditorPaths.broadcastStop
        : serverEditorPaths.broadcastStart;
      const result = await mutate<Schema['BroadcastSimpleResultDTO']>(api, path);
      if (result.operationId) await pollOperation(api, result.operationId);
      status = await call(api, status, serverEditorPaths.broadcastStatus);
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      broadcastBusy = false;
    }
  }

  async function downloadJar(): Promise<void> {
    if (jarBusy) return;
    jarBusy = true;
    try {
      const result = await mutate<Schema['BroadcastJarDownloadResultDTO']>(
        api,
        serverEditorPaths.broadcastDownloadJar,
      );
      if (result.operationId) await pollOperation(api, result.operationId);
      notice = result.message;
      jarStatus = await call(api, jarStatus, serverEditorPaths.broadcastJarStatus);
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      jarBusy = false;
    }
  }

  async function saveCredentials(): Promise<void> {
    if (credBusy || !credEmail.trim() || !credGamertag.trim() || !credPassword) return;
    credBusy = true;
    try {
      await mutate(api, serverEditorPaths.broadcastCredentials, {
        email: credEmail.trim(),
        gamertag: credGamertag.trim(),
        password: credPassword,
      });
      notice = 'Broadcast credentials saved.';
      credEmail = '';
      credGamertag = '';
      credPassword = '';
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      credBusy = false;
    }
  }

  async function togglePlayit(): Promise<void> {
    if (playitBusy || !playit) return;
    playitBusy = true;
    try {
      const path = playit.isRunning ? serverEditorPaths.playitStop : serverEditorPaths.playitStart;
      await mutate(api, path);
      playit = { ...playit, isRunning: !playit.isRunning };
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      playitBusy = false;
    }
  }

  async function saveDuckDns(): Promise<void> {
    if (duckBusy) return;
    duckBusy = true;
    try {
      const result = await mutate<Schema['DuckDNSUpdateResultDTO']>(
        api,
        serverEditorPaths.duckdns,
        {
          hostname: duckHost.trim(),
        },
      );
      duckdns = { ...duckdns, isConfigured: true, hostname: result.hostname };
      notice = result.message ?? 'DuckDNS label saved.';
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      duckBusy = false;
    }
  }

  async function togglePack(pack: Schema['ResourcePackItemDTO']): Promise<void> {
    try {
      const result = await mutate<Schema['ResourcePackMutationResultDTO']>(
        api,
        serverEditorPaths.resourcePacksToggle,
        { packId: pack.id, enabled: !pack.isActive },
      );
      if (result.updated) resourcePacks = result.updated;
    } catch (error) {
      notice = errorMessage(error);
    }
  }
</script>

<div class="tab">
  {#if notice}<p class="notice" role="status">{notice}</p>{/if}

  {#if !isActive}
    <Card>
      <div class="notice-row">
        <div class="notice-text">
          <span class="name">Set as active to configure broadcast</span>
          <p class="hint">
            Xbox broadcast, Playit, DuckDNS, and resource packs are only editable for the currently
            active server.
          </p>
        </div>
        <Button variant="secondary" size="sm" onclick={onRequestActivate}>Set as Active</Button>
      </div>
    </Card>
  {:else}
    <section class="zone">
      <p class="msc2-type-overline">Xbox Broadcast</p>
      <Card padding="0">
        <div class="row">
          <div class="toggle-info">
            <Toggle
              checked={autostart?.enabled ?? false}
              label="Start Xbox broadcast automatically"
              onchange={toggleAutostart}
            />
            <span class="name">Start automatically with this server</span>
          </div>
        </div>
        <div class="row bordered">
          <span class="name">Join address</span>
          <span class="mono"
            >{server.hostAddress ?? '—'}{server.gamePort ? `:${server.gamePort}` : ''}</span
          >
        </div>
        <div class="row bordered">
          <StatusDot
            tone={broadcastRunning ? 'ok' : 'warn'}
            label={broadcastRunning ? 'Running' : 'Stopped'}
          />
          <Button
            variant="secondary"
            size="sm"
            disabled={broadcastBusy || !canControl}
            onclick={toggleBroadcast}>{broadcastRunning ? 'Stop' : 'Start'}</Button
          >
        </div>
        <div class="row bordered">
          <StatusDot
            tone={jarStatus?.installed ? 'ok' : 'warn'}
            label={jarStatus?.installed
              ? (jarStatus.filename ?? 'Helper installed')
              : 'Helper JAR not installed'}
          />
          <Button
            variant="secondary"
            size="sm"
            disabled={jarBusy || !canControl}
            onclick={downloadJar}>{jarBusy ? 'Downloading…' : 'Download…'}</Button
          >
        </div>
      </Card>
    </section>

    {#if isJava}
      <section class="zone">
        <p class="msc2-type-overline">Alt Account Profile</p>
        <p class="hint">
          Local reference only -- which Microsoft/Xbox account is used as the broadcast alt. The
          agent stores this and does not sign you in. Existing values aren't shown here; saving
          replaces them.
        </p>
        <Card padding="0">
          <div class="row">
            <span class="name">Email</span>
            <Field bind:value={credEmail} type="email" width="220px" />
          </div>
          <div class="row bordered">
            <span class="name">Gamertag</span>
            <Field bind:value={credGamertag} width="220px" />
          </div>
          <div class="row bordered">
            <span class="name">Password</span>
            <div class="password-control">
              <Field
                bind:value={credPassword}
                type={showPassword ? 'text' : 'password'}
                width="184px"
              />
              <button
                type="button"
                class="reveal"
                aria-label={showPassword ? 'Hide password' : 'Show password'}
                onclick={() => (showPassword = !showPassword)}
              >
                {#if showPassword}
                  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                    <path
                      d="M3 3l18 18M10.6 10.6a3 3 0 0 0 4.2 4.2M6.6 6.6C4.3 8.1 2.7 10 2 12c1.5 3.8 5.5 7 10 7 1.6 0 3.1-.4 4.5-1.1M17.4 17.4C19.5 15.9 21.1 14 22 12c-1.1-2.8-3.4-5.2-6.3-6.5"
                      stroke="currentColor"
                      stroke-width="1.8"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    />
                  </svg>
                {:else}
                  <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                    <path
                      d="M2 12c1.5-3.8 5.5-7 10-7s8.5 3.2 10 7c-1.5 3.8-5.5 7-10 7s-8.5-3.2-10-7z"
                      stroke="currentColor"
                      stroke-width="1.8"
                      stroke-linejoin="round"
                    />
                    <circle cx="12" cy="12" r="3" stroke="currentColor" stroke-width="1.8" />
                  </svg>
                {/if}
              </button>
            </div>
          </div>
        </Card>
        <div class="footer-actions">
          <Button
            variant="secondary"
            size="sm"
            disabled={credBusy ||
              !credEmail.trim() ||
              !credGamertag.trim() ||
              !credPassword ||
              !canControl}
            onclick={saveCredentials}>{credBusy ? 'Saving…' : 'Save Credentials'}</Button
          >
        </div>
      </section>
    {/if}

    <section class="zone">
      <p class="msc2-type-overline">Playit</p>
      <Card padding="0">
        <div class="row">
          <StatusDot
            tone={playit?.isRunning ? 'ok' : 'warn'}
            label={playit?.isRunning ? 'Running' : 'Stopped'}
          />
          <Button
            variant="secondary"
            size="sm"
            disabled={playitBusy || !playit?.playitEnabled || !canControl}
            onclick={togglePlayit}>{playit?.isRunning ? 'Stop' : 'Start'}</Button
          >
        </div>
      </Card>
      <p class="hint">{playit?.note ?? 'Managed helper state is reported by the agent.'}</p>
    </section>

    <section class="zone">
      <p class="msc2-type-overline">DuckDNS</p>
      <Card padding="0">
        <div class="row">
          <span class="name">Hostname</span>
          <div class="control">
            <Field bind:value={duckHost} placeholder="example.duckdns.org" width="200px" />
            <Button
              variant="secondary"
              size="sm"
              disabled={duckBusy || !canControl}
              onclick={saveDuckDns}>{duckBusy ? 'Saving…' : 'Save'}</Button
            >
          </div>
        </div>
      </Card>
      <p class="hint">
        DuckDNS supplies a name; it does not replace authentication or the loopback/Tailscale
        management boundary.
      </p>
    </section>

    <section class="zone">
      <p class="msc2-type-overline">Resource Packs</p>
      {#if !resourcePacks?.packs.length}
        <EmptyState
          title="No resource packs"
          message="Resource packs installed on this server appear here."
        />
      {:else}
        <Card padding="0">
          {#each resourcePacks.packs as pack, index (pack.id)}
            <ListRow
              title={pack.name}
              subtitle={`${pack.typeLabel} · ${pack.fileSizeDisplay}`}
              last={index === resourcePacks.packs.length - 1}
            >
              <svelte:fragment slot="trailing">
                <Button
                  variant="secondary"
                  size="sm"
                  disabled={!canControl}
                  onclick={() => togglePack(pack)}>{pack.isActive ? 'Disable' : 'Enable'}</Button
                >
              </svelte:fragment>
            </ListRow>
          {/each}
        </Card>
      {/if}
    </section>
  {/if}
</div>

<style>
  .tab {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .notice {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .zone {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .notice-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
  }
  .notice-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 11px 14px;
  }
  .row.bordered {
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .name {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .mono {
    font-size: 12px;
    font-family: var(--msc2-font-mono, monospace);
    color: var(--msc2-text-tertiary);
  }
  .toggle-info {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .control {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
    line-height: 1.5;
  }
  .footer-actions {
    display: flex;
    justify-content: flex-end;
  }
  .password-control {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .reveal {
    width: 26px;
    height: 26px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: rgba(255, 255, 255, 0.55);
    background: transparent;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    flex-shrink: 0;
  }
  .reveal:hover {
    background: rgba(255, 255, 255, 0.08);
  }
</style>
