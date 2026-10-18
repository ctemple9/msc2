<script lang="ts">
  // Ports MSCSettingsView.swift's MSC Settings sheet, reduced to what the
  // frozen contract actually backs -- see this step's rolling-plan.md entry
  // for the full accounting. Four MSC 1 cards are gone because MSC 2
  // replaced the thing they managed, not because they were never ported:
  // Remote Access -> named tokens + Tailscale (AccessSection, D-012);
  // Bedrock Runtime toggle -> automatic per-host runtime detection
  // (Phase 10); Process Management (orphan scan, relaunch-on-crash) -> the
  // agent is itself the OS-managed persistent service, so MSC 1's "orphaned
  // by a crashed app" problem doesn't exist here. Four more -- Config
  // Recovery and Archives were dropped at the owner's direction. Storage
  // now has host-scoped preview/cleanup routes for abandoned map data;
  // Network Ports remains a future contract-amendment step. "Testing reset" and "Open App Support Folder" are dropped
  // outright -- the former has no backend and reads as a dev-only escape
  // hatch, the latter has no meaning for a possibly-remote agent host.
  //
  // What's left and real: Open Server Folder (GET /v1/config/servers-root +
  // the revealInFileManager seam P12.9 built for the Files tab), plus
  // host-wide service configuration below.
  import { onMount } from 'svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import SegmentedControl from '../../components/base/SegmentedControl.svelte';
  import StorageView from './StorageView.svelte';
  import Card from '../../components/base/Card.svelte';
  import Button from '../../components/base/Button.svelte';
  import Toggle from '../../components/base/Toggle.svelte';
  import Field from '../../components/base/Field.svelte';
  import VisibilityIcon from '../../components/base/VisibilityIcon.svelte';
  import PlayitSetupSheet from '../server-editor/PlayitSetupSheet.svelte';
  import { getPlatform } from '../../platform';
  import type { UpdateCheckResult } from '../../platform';
  import { updateErrorMessage, type UpdateWorkflowState } from '../../updates/coordinated';
  import { bundleIdentity } from '../../bundle-identity';
  import type { Schema, ScreenApi } from '../shared/types';
  import { call, errorMessage, mutate } from '../shared/types';

  type SettingsTab = 'general' | 'services' | 'storage';
  const settingsTabs = [
    { value: 'general', label: 'General' },
    { value: 'services', label: 'Services' },
    { value: 'storage', label: 'Storage' },
  ] as const;
  let activeTab: SettingsTab = 'general';
  let storageOpened = false;
  let storageAtOverview = true;

  export let api: ScreenApi | undefined = undefined;
  export let hostId: string;
  export let hostName = 'Selected host';
  export let serverUsesPlayit: boolean | undefined = undefined;
  export let onClose: () => void;
  export let preloadTabs = true;
  export let onPreloadTabsChanged: (enabled: boolean) => void = () => {};
  export let onOpenReset: () => void = () => {};
  export let onOpenUninstall: () => void = () => {};

  let serversRootPath = '';
  let revealBusy = false;
  let revealNotice = '';

  let curseforge: Schema['CurseForgeApiKeyStatusDTO'] = { configured: false };
  let curseforgeApiKey = '';
  let curseforgeApiKeyVisible = false;
  let curseforgeSaving = false;
  let curseforgeNotice = '';

  let playit: Schema['PlayitStatusResponseDTO'] = {
    serverName: '',
    serverType: 'java',
    playitEnabled: false,
    isRunning: false,
    hasSecretKey: false,
    voiceChatEnabled: false,
  };
  let showPlayitSetup = false;
  let duckdns: Schema['DuckDNSStatusResponseDTO'] = { isConfigured: false };
  let duckHost = '';
  let duckBusy = false;
  let duckdnsNotice = '';

  let updateState: UpdateWorkflowState = 'idle';
  let updateResult: UpdateCheckResult | undefined;
  let updateNotice = '';
  let confirmUpdateInstall = false;

  $: showDuckDns = serverUsesPlayit === false;
  onMount(async () => {
    const [
      root,
      playitStatus,
      duckdnsStatus,
      curseforgeStatus,
    ] = await Promise.all([
      call<Schema['ServersRootResponseDTO']>(api, { path: '' }, '/v1/config/servers-root'),
      call<Schema['PlayitStatusResponseDTO']>(api, playit, '/v1/playit'),
      call<Schema['DuckDNSStatusResponseDTO']>(api, duckdns, '/v1/duckdns'),
      call<Schema['CurseForgeApiKeyStatusDTO']>(api, curseforge, '/v1/config/curseforge'),
    ]);
    serversRootPath = root.path;
    playit = playitStatus;
    duckdns = duckdnsStatus;
    duckHost = duckdnsStatus.hostname ?? '';
    curseforge = curseforgeStatus;
  });

  function updatePreloadTabs(enabled: boolean): void {
    preloadTabs = enabled;
    onPreloadTabsChanged(enabled);
  }

  function selectTab(value: string): void {
    activeTab = value as SettingsTab;
    if (activeTab === 'storage') storageOpened = true;
  }

  async function openServersFolder(): Promise<void> {
    if (revealBusy || !serversRootPath) return;
    revealBusy = true;
    revealNotice = '';
    try {
      await (await getPlatform()).revealInFileManager(serversRootPath);
    } catch (error) {
      revealNotice = errorMessage(error);
    } finally {
      revealBusy = false;
    }
  }

  async function saveCurseForgeApiKey(value = curseforgeApiKey): Promise<void> {
    if (curseforgeSaving) return;
    curseforgeSaving = true;
    curseforgeNotice = '';
    try {
      curseforge = await mutate<Schema['CurseForgeApiKeyStatusDTO']>(api, '/v1/config/curseforge', {
        apiKey: value.trim(),
      });
      curseforgeApiKey = '';
      curseforgeNotice = curseforge.configured
        ? 'CurseForge API key saved for this agent.'
        : 'CurseForge API key cleared.';
    } catch (error) {
      curseforgeNotice = errorMessage(error);
    } finally {
      curseforgeSaving = false;
    }
  }

  async function saveDuckDns(): Promise<void> {
    await saveDuckDnsValue(duckHost.trim());
  }

  async function saveDuckDnsValue(hostname: string): Promise<void> {
    if (duckBusy) return;
    duckBusy = true;
    duckdnsNotice = '';
    try {
      const result = await mutate<Schema['DuckDNSUpdateResultDTO']>(api, '/v1/duckdns', {
        hostname,
      });
      duckdns = { isConfigured: !!result.hostname, hostname: result.hostname };
      duckHost = result.hostname ?? '';
      duckdnsNotice = result.hostname ? 'DuckDNS hostname saved.' : 'DuckDNS hostname removed.';
    } catch (error) {
      duckdnsNotice = errorMessage(error);
    } finally {
      duckBusy = false;
    }
  }

  function removeDuckDns(): void {
    void saveDuckDnsValue('');
  }

  function refreshPlayit(): void {
    void (async () => {
      playit = await call(api, playit, '/v1/playit');
    })();
  }

  async function checkForUpdates(): Promise<void> {
    if (updateState === 'checking' || updateState === 'installing') return;
    updateState = 'checking';
    updateResult = undefined;
    updateNotice = '';
    confirmUpdateInstall = false;
    try {
      updateResult = await (await getPlatform()).checkForUpdates();
      updateState =
        updateResult.state === 'current'
          ? 'current'
          : updateResult.state === 'staged'
            ? 'available'
            : 'unavailable';
    } catch (error) {
      updateState = 'error';
      updateNotice = updateErrorMessage(error);
    }
  }

  async function installAvailableUpdate(): Promise<void> {
    if (!updateResult?.releaseId || updateState !== 'available') return;
    const releaseId = updateResult.releaseId;
    updateState = 'installing';
    confirmUpdateInstall = false;
    updateNotice = '';
    try {
      const result = await (await getPlatform()).installUpdate(releaseId);
      updateState = 'installed';
      updateNotice = result.detail;
    } catch (error) {
      updateState = 'error';
      updateNotice = updateErrorMessage(error);
    }
  }
</script>

<Sheet title="MSC Settings" size="md" {onClose}>
  <div class="settings">
    <nav aria-label="MSC Settings sections">
      <SegmentedControl options={settingsTabs} value={activeTab} onchange={selectTab} />
    </nav>

    {#if activeTab === 'general'}
      <section class="zone">
        <p class="msc2-type-overline">Updates</p>
        <Card padding="0">
          <div class="update-block">
            <div class="row update-summary">
              <div class="row-text">
                <span class="name">MSC 2</span>
                <span class="hint">Installed bundle v{bundleIdentity.version}</span>
              </div>
              <Button
                variant="secondary"
                size="sm"
                disabled={updateState === 'checking' || updateState === 'installing'}
                onclick={() => void checkForUpdates()}
                >{updateState === 'checking'
                  ? 'Checking…'
                  : updateState === 'installing'
                    ? 'Installing…'
                    : 'Check for updates'}</Button
              >
            </div>

            {#if hostId !== 'local-agent'}
              <p class="update-boundary">
                The selected host is remote. This checks or installs MSC on this desktop only; it
                does not update the remote host's service.
              </p>
            {/if}

            {#if updateState === 'current'}
              <p class="update-status" role="status">
                You are up to date{updateResult?.releaseId ? ` · ${updateResult.releaseId}` : ''}.
              </p>
            {:else if updateState === 'unavailable'}
              <p class="update-status" role="status">
                {updateResult?.detail ||
                  'Native installation is local to the desktop or headless host.'}
              </p>
            {:else if updateState === 'available' && updateResult}
              <div class="update-details" aria-live="polite">
                <div class="update-detail-row">
                  <span class="name">Available release</span>
                  <span class="release-id mono">{updateResult.releaseId}</span>
                </div>
                {#if updateResult.installMode}
                  <p class="hint">Installation: {updateResult.installMode}</p>
                {/if}
                <p class="hint">{updateResult.detail}</p>
                {#if updateResult.releaseNotes}
                  <div class="release-notes">
                    <span class="notes-label">Release notes</span>
                    <pre>{updateResult.releaseNotes}</pre>
                  </div>
                {/if}
                {#if !confirmUpdateInstall}
                  <div class="update-actions">
                    <Button
                      variant="primary"
                      size="sm"
                      onclick={() => (confirmUpdateInstall = true)}
                      >Install {updateResult.releaseId}…</Button
                    >
                  </div>
                {:else}
                  <div class="update-confirmation" role="alert">
                    <div class="row-text">
                      <span class="name">Install this release?</span>
                      <span class="hint"
                        >MSC will close, install the already verified release, and relaunch. A
                        failed replacement can be rolled back.</span
                      >
                    </div>
                    <div class="update-actions">
                      <Button
                        variant="secondary"
                        size="sm"
                        onclick={() => (confirmUpdateInstall = false)}>Cancel</Button
                      >
                      <Button
                        variant="primary"
                        size="sm"
                        onclick={() => void installAvailableUpdate()}
                        >Install {updateResult.releaseId}</Button
                      >
                    </div>
                  </div>
                {/if}
              </div>
            {:else if updateState === 'installed'}
              <p class="update-status" role="status">{updateNotice}</p>
            {:else if updateState === 'installing'}
              <p class="update-status" role="status">
                Installing the verified release locally… MSC 2 will relaunch when the installer
                finishes.
              </p>
            {:else if updateState === 'error'}
              <p class="update-status error" role="alert">{updateNotice}</p>
            {:else if updateState === 'idle'}
              <p class="update-status">
                Check for a signed release when you are ready. Nothing installs automatically.
              </p>
            {/if}
          </div>
        </Card>
      </section>

      <section class="zone">
        <p class="msc2-type-overline">Performance</p>
        <Card padding="0">
          <div class="row">
            <div class="row-text">
              <span class="name">Preload tabs</span>
              <span class="hint">Load available tab code in the background after MSC opens.</span>
            </div>
            <Toggle checked={preloadTabs} label="Preload tabs" onchange={updatePreloadTabs} />
          </div>
        </Card>
      </section>

      <section class="zone">
        <p class="msc2-type-overline">Reset and uninstall</p>
        <Card padding="0">
          <div class="row">
            <div class="row-text">
              <span class="name">Reset MSC</span>
              <span class="hint"
                >Choose whether to reset this device or the selected host, then confirm.</span
              >
            </div>
            <Button variant="destructive" size="sm" onclick={onOpenReset}>Reset…</Button>
          </div>
          <div class="row">
            <div class="row-text">
              <span class="name">Uninstall MSC 2</span>
              <span class="hint"
                >Permanently remove this computer's MSC installation, servers, worlds, and backups.</span
              >
            </div>
            <Button variant="destructive" size="sm" onclick={onOpenUninstall}>Uninstall…</Button>
          </div>
        </Card>
      </section>
    {/if}

    {#if activeTab === 'services'}
      <section class="zone">
        <p class="msc2-type-overline">Modpack Imports</p>
        <p class="hint">
          CurseForge modpacks need an API key to resolve their files. You can create one in the
          <a href="https://console.curseforge.com/" target="_blank" rel="noreferrer"
            >CurseForge API Console</a
          >, or enter it when an import asks for it. The key is stored securely on this agent and is
          never shown again.
        </p>
        <Card padding="0">
          <div class="row">
            <div class="row-text">
              <span class="name">CurseForge API key</span>
              <span class="hint">{curseforge.configured ? 'Configured' : 'Not configured'}</span>
            </div>
            <div class="control">
              <div class="password-control">
                <Field
                  bind:value={curseforgeApiKey}
                  type={curseforgeApiKeyVisible ? 'text' : 'password'}
                  placeholder={curseforge.configured ? 'Saved — enter to replace' : 'Paste API key'}
                  width="220px"
                />
                <button
                  type="button"
                  class="visibility-toggle"
                  aria-label={curseforgeApiKeyVisible ? 'Hide API key' : 'Show API key'}
                  aria-pressed={curseforgeApiKeyVisible}
                  title={curseforgeApiKeyVisible ? 'Hide API key' : 'Show API key'}
                  onclick={() => (curseforgeApiKeyVisible = !curseforgeApiKeyVisible)}
                >
                  <VisibilityIcon visible={curseforgeApiKeyVisible} />
                </button>
              </div>
              <Button
                variant="secondary"
                size="sm"
                disabled={curseforgeSaving || !curseforgeApiKey.trim()}
                onclick={() => void saveCurseForgeApiKey()}
                >{curseforgeSaving ? 'Saving…' : 'Save key'}</Button
              >
              {#if curseforge.configured}
                <Button
                  variant="secondary"
                  size="sm"
                  disabled={curseforgeSaving}
                  onclick={() => void saveCurseForgeApiKey('')}>Clear key</Button
                >
              {/if}
            </div>
          </div>
        </Card>
        {#if curseforgeNotice}<p class="hint" role="status">{curseforgeNotice}</p>{/if}
      </section>

      <section class="zone">
        <p class="msc2-type-overline">Playit</p>
        <p class="hint">
          The Playit account and shared tunnels belong to this agent. Per-server tunnel
          participation stays under Manage → Services.
        </p>
        <Card padding="0">
          <div class="row">
            <div class="row-text">
              <span class="name">Playit account</span>
              <span class="hint">{playit.hasSecretKey ? 'Configured' : 'Setup required'}</span>
            </div>
            <Button
              variant="secondary"
              size="sm"
              onclick={() => (showPlayitSetup = true)}
              >{playit.hasSecretKey ? 'Manage setup…' : 'Set up…'}</Button
            >
          </div>
          {#if playit.agentId}
            <div class="row bordered">
              <span class="name">Agent</span>
              <span class="hint mono playit-value">{playit.agentId}</span>
            </div>
          {/if}
          {#if playit.javaAddress}
            <div class="row bordered">
              <span class="name">Java tunnel</span>
              <span class="hint mono playit-value">{playit.javaAddress}</span>
            </div>
          {/if}
          {#if playit.bedrockAddress}
            <div class="row bordered">
              <span class="name">Bedrock tunnel</span>
              <span class="hint mono playit-value">{playit.bedrockAddress}</span>
            </div>
          {/if}
          {#if playit.voiceAddress}
            <div class="row bordered">
              <span class="name">Voice tunnel</span>
              <span class="hint mono playit-value">{playit.voiceAddress}</span>
            </div>
          {/if}
        </Card>
      </section>

      {#if showDuckDns}
        <section class="zone">
          <p class="msc2-type-overline">DuckDNS</p>
          <p class="hint">
            This hostname is used for port-forwarded servers that are not using Playit.
          </p>
          <Card padding="0">
            <div class="row">
              <span class="name">Hostname</span>
              <div class="control">
                <Field bind:value={duckHost} placeholder="example.duckdns.org" width="220px" />
                <Button variant="secondary" size="sm" disabled={duckBusy} onclick={saveDuckDns}>
                  {duckBusy ? 'Saving…' : 'Save'}
                </Button>
                {#if duckdns.isConfigured}
                  <Button variant="secondary" size="sm" disabled={duckBusy} onclick={removeDuckDns}>
                    Remove
                  </Button>
                {/if}
              </div>
            </div>
          </Card>
          {#if duckdnsNotice}<p class="hint" role="status">{duckdnsNotice}</p>{/if}
        </section>
      {/if}
    {/if}

    {#if storageOpened}
      <div class="storage-tab" class:inactive={activeTab !== 'storage'}>
        {#if storageAtOverview}
          <section class="zone">
            <p class="msc2-type-overline">Server folder</p>
            <Card padding="0">
              <div class="row">
                <div class="row-text">
                  <span class="name">On {hostName}</span>
                  <span class="hint dir">{serversRootPath || 'Loading…'}</span>
                </div>
                <Button
                  variant="secondary"
                  size="sm"
                  disabled={revealBusy || !serversRootPath}
                  onclick={openServersFolder}>Open Server Folder…</Button
                >
              </div>
            </Card>
            {#if revealNotice}<p class="hint">{revealNotice}</p>{/if}
          </section>
        {/if}
        <StorageView
          {api}
          {hostName}
          showRootBack={false}
          onOverviewChange={(overview) => (storageAtOverview = overview)}
          onBack={() => selectTab('general')}
        />
      </div>
    {/if}
  </div>
</Sheet>

{#if showPlayitSetup}
  <PlayitSetupSheet
    {api}
    {playit}
    context="settings"
    onClose={() => (showPlayitSetup = false)}
    onComplete={refreshPlayit}
    onReset={refreshPlayit}
  />
{/if}

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .storage-tab {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .storage-tab.inactive {
    display: none;
  }
  .zone {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 11px 14px;
  }
  .update-block {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .update-summary {
    padding-bottom: 2px;
  }
  .update-boundary,
  .update-status,
  .update-details,
  .update-confirmation {
    margin: 0 14px;
  }
  .update-boundary,
  .update-status {
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .update-status {
    padding-bottom: 12px;
  }
  .update-status.error {
    color: var(--msc2-status-error);
  }
  .update-details {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 0 12px;
  }
  .update-detail-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
  }
  .release-id {
    color: var(--msc2-text-secondary);
    font-size: 12px;
  }
  .release-notes {
    padding-top: 4px;
  }
  .notes-label {
    display: block;
    margin-bottom: 4px;
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-text-secondary);
  }
  .release-notes pre {
    max-height: 160px;
    margin: 0;
    overflow: auto;
    white-space: pre-wrap;
    font-family: inherit;
    font-size: 12px;
    line-height: 1.45;
    color: var(--msc2-text-tertiary);
  }
  .update-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 3px;
  }
  .update-confirmation {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 0 2px;
    border-top: 1px solid var(--msc2-hairline-faint);
  }
  .row-text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .name {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .control {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }
  .password-control {
    position: relative;
    width: 220px;
    flex-shrink: 0;
  }
  .password-control :global(.field) {
    width: 100% !important;
    padding-right: 34px;
  }
  .visibility-toggle {
    position: absolute;
    top: 50%;
    right: 8px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    padding: 0;
    transform: translateY(-50%);
    color: var(--msc2-text-tertiary);
    background: transparent;
    border: none;
    cursor: pointer;
  }
  .visibility-toggle:hover {
    color: var(--msc2-text-primary);
  }
  .visibility-toggle:focus-visible {
    outline: 2px solid var(--msc2-hairline);
    outline-offset: 2px;
    border-radius: 3px;
  }
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .hint a {
    color: var(--msc2-text-secondary);
  }
  .mono {
    font-family: var(--msc2-font-mono, monospace);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .playit-value {
    overflow: visible;
    overflow-wrap: anywhere;
    text-align: right;
    white-space: normal;
  }
  .hint.dir {
    overflow: hidden;
    font-family: var(--msc2-font-mono, monospace);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
