<script lang="ts">
  // Ports DetailsComponentsTabView.swift to the S0 disciplined system
  // (docs/msc2/antiAIslop.md): Server JAR/Loader row, Plugins/Mods list,
  // Crossplay row, and (Bedrock) the Runtime + Broadcast cards. Same
  // shared-component pattern HomeSection/WorldsSection use (D-003).
  //
  // MSC 1's per-row icon-in-tinted-box (ZStack { RoundedRectangle.fill(color
  // .opacity(0.12)); Image(...).foregroundStyle(color) }) is rule #6's exact
  // tell -- a colored icon on an informational element -- so it's dropped
  // here, not ported: rows carry name + badge + StatusDot(+label) only, the
  // same vocabulary HealthGrid/WorldSlotCard already established.
  //
  // Two real, pre-existing backend gaps found while wiring this, left alone
  // rather than expanded on (crates/ wasn't in this step's scope) but worth
  // recording plainly:
  //  1. GET /v1/components (crates/msc-agent/src/routes/components.rs
  //     component_rows) hardcodes is_up_to_date=true/updatable=true for the
  //     primary server-jar row -- there is no real online-build check behind
  //     it yet (unlike GET /v1/versions, which is real). The Server JAR row
  //     below renders whatever the agent reports, honestly; it will always
  //     read "Up to date" until that route gets a real PaperMC-style build
  //     check, which is a backend step of its own.
  //  2. AddonItemDTO has no MSC-1-style tier (managed/userSourced/unmanaged)
  //     -- only `bucket`, the resolver's update-resolution bucket. Geyser/
  //     Floodgate show in this list like any other installed plugin, with no
  //     "Managed" badge; the contract has nothing to badge them with.
  //
  // Two MSC 1 affordances deliberately left out of this step's scope, not
  // half-built: "Export for clients" (ClientExportResponseDTO.stagedDownloadId
  // needs a save-to-disk platform primitive this client doesn't have yet --
  // no section has ever downloaded a file to disk) and per-plugin manual
  // source-linking (the PluginSourcePopover flow) -- neither is named in this
  // step's own line ("list, Add Plugin, Reveal folder, empty state").
  import { onDestroy, onMount } from 'svelte';
  import Icon from '../../components/base/Icon.svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import Card from '../../components/base/Card.svelte';
  import Toggle from '../../components/base/Toggle.svelte';
  import Badge from '../../components/base/Badge.svelte';
  import StatusDot from '../../components/base/StatusDot.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import Menu from '../../components/base/Menu.svelte';
  import VersionPickerSheet from './VersionPickerSheet.svelte';
  import PluginBrowserSheet from './PluginBrowserSheet.svelte';
  import PlayitSetupSheet from '../server-editor/PlayitSetupSheet.svelte';
  import ProjectDetailSheet from './ProjectDetailSheet.svelte';
  import ImportModpackSheet from './ImportModpackSheet.svelte';
  import CurseForgeManualDownloadSheet from './CurseForgeManualDownloadSheet.svelte';
  import { getPlatform } from '../../platform';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, mutate } from '../shared/types';
  import {
    addOnKind,
    addonPaths,
    addonStatusLabel,
    broadcastPaths,
    componentPaths,
    componentStatusLabel,
    componentTone,
    demoAddons,
    demoBroadcastAutostart,
    demoBroadcastStatus,
    demoComponentsStatus,
    demoJarStatus,
    flavorDisplayName,
    healthPath,
    pollOperation,
    serversPath,
    supportsCrossplay,
    isSimpleVoiceChatAddon,
    type ProjectDetailItem,
  } from './model';

  export let api: ScreenProps['api'] = undefined;
  // The voice prompt is keyed by serverId below; keep the host prop shape
  // uniform with the other section-registry components.
  export let hostId = 'local-agent';
  export let serverId = 'survival';

  let components: Schema['ComponentsStatusDTO'] = demoComponentsStatus;
  let addons: Schema['AddonItemDTO'][] = demoAddons;
  let servers: Schema['ServerDTO'][] = [];
  let health: Schema['HealthResponseDTO'] = {
    cards: [],
    overallSeverity: 'gray',
    serverName: '',
    serverRunning: false,
    serverType: '',
  };
  let broadcastStatus: Schema['BroadcastStatusDTO'] = demoBroadcastStatus;
  let broadcastAutostart: Schema['BroadcastAutoStartDTO'] = demoBroadcastAutostart;
  let jarStatus: Schema['BroadcastJarStatusDTO'] = demoJarStatus;
  let playit: Schema['PlayitStatusResponseDTO'] | undefined;

  $: activeServer = servers.find((server) => server.id === serverId);
  $: isBedrock = activeServer?.serverType === 'bedrock';
  $: kind = addOnKind(activeServer?.javaFlavor);
  $: isModded = kind === 'mod';
  $: crossplayEligible = supportsCrossplay(activeServer);
  $: serverRunning = health.serverRunning;
  $: primaryComponent = components.components.find(
    (component) => component.name === activeServer?.javaFlavor,
  );
  $: addonFolderName = isModded ? 'mods' : 'plugins';
  $: anyAddonUpdatable = addons.some((addon) => addon.bucket === 'updateAvailable');
  $: svcAddon = addons.find(isSimpleVoiceChatAddon);

  let notice = '';
  let downloadingBroadcastJar = false;
  let updatingAll = false;
  let togglingStems: Set<string> = new Set();
  let updatingStems: Set<string> = new Set();
  let confirmingRemove: string | undefined;
  let addonMenu: { addon: Schema['AddonItemDTO']; x: number; y: number } | undefined;
  let detailAddon: Schema['AddonItemDTO'] | undefined;
  let showVoicePrompt = false;
  let showPlayitSetup = false;
  let playitSetupVoiceOnly = false;

  let showVersionPicker = false;
  let showBrowser = false;
  let showImportModpack = false;
  let pendingManualFiles:
    { operationId: string; files: Schema['ModpackManualFileEntryDTO'][] } | undefined;

  let fileInput: HTMLInputElement;

  function flash(message: string): void {
    notice = message;
  }

  async function loadComponents(): Promise<void> {
    components = await call(api, components, componentPaths.status);
  }
  async function loadAddons(): Promise<void> {
    const response = await call<Schema['AddonsResponseDTO']>(
      api,
      { addons, isResolving: false, serverSupportsAddons: !!kind, packManaged: false },
      addonPaths.list,
    );
    addons = response.addons;
    checkVoiceTunnelPrompt(playit, response.addons);
  }
  async function loadPlayit(): Promise<void> {
    const next = await call(api, playit, '/v1/playit');
    playit = next;
    checkVoiceTunnelPrompt(next, addons);
  }
  async function loadServers(): Promise<void> {
    servers = await call(api, servers, serversPath);
  }
  async function loadHealth(): Promise<void> {
    health = await call(api, health, healthPath);
  }
  async function loadBroadcast(): Promise<void> {
    // Always fetched (cheap: three small GETs) rather than gated on
    // crossplayEligible/isBedrock -- those are reactive values derived from
    // `servers`, which this same loadAll() pass may not have flushed yet.
    // The template's own {#if crossplayEligible}/{#if isBedrock} blocks
    // decide whether this data ever renders.
    [broadcastStatus, broadcastAutostart, jarStatus] = await Promise.all([
      call(api, broadcastStatus, broadcastPaths.status),
      call(api, broadcastAutostart, broadcastPaths.autostart),
      call(api, jarStatus, broadcastPaths.jarStatus),
    ]);
  }
  async function loadAll(): Promise<void> {
    await Promise.all([
      loadComponents(),
      loadAddons(),
      loadServers(),
      loadHealth(),
      loadBroadcast(),
      loadPlayit(),
    ]);
    checkVoiceTunnelPrompt();
  }

  function voicePromptKey(): string {
    return `msc2.svc-tunnel-prompt.${hostId}.${serverId}`;
  }

  function voicePromptWasDismissed(): boolean {
    return (
      typeof localStorage !== 'undefined' && localStorage.getItem(voicePromptKey()) === 'dismissed'
    );
  }

  function clearVoicePromptState(): void {
    if (typeof localStorage !== 'undefined') localStorage.removeItem(voicePromptKey());
  }

  function checkVoiceTunnelPrompt(
    currentPlayit = playit,
    currentAddons = addons,
  ): void {
    const currentSvcAddon = currentAddons.find(isSimpleVoiceChatAddon);
    if (!currentSvcAddon || !currentPlayit?.playitEnabled) {
      showVoicePrompt = false;
      clearVoicePromptState();
      return;
    }
    // An address means the agent already provisioned the named voice tunnel.
    // GET /v1/playit also synchronizes its host into voicechat-server.properties.
    if (currentPlayit.voiceAddress || voicePromptWasDismissed()) {
      showVoicePrompt = false;
      return;
    }
    showVoicePrompt = true;
  }

  function dismissVoicePrompt(): void {
    if (typeof localStorage !== 'undefined') localStorage.setItem(voicePromptKey(), 'dismissed');
    showVoicePrompt = false;
  }

  function openVoiceSetup(): void {
    showVoicePrompt = false;
    playitSetupVoiceOnly = Boolean(playit?.hasSecretKey);
    showPlayitSetup = true;
  }

  function disableVoiceChat(): void {
    showVoicePrompt = false;
    if (svcAddon) void toggleAddon(svcAddon);
  }

  async function refreshPlayit(): Promise<void> {
    await loadPlayit();
    checkVoiceTunnelPrompt();
  }

  async function toggleAddon(addon: Schema['AddonItemDTO']): Promise<void> {
    togglingStems = new Set(togglingStems).add(addon.jarStem);
    try {
      await mutate(api, addonPaths.update, { jarStem: addon.jarStem, enabled: !addon.isEnabled });
      await loadAddons();
    } catch (error) {
      flash(error instanceof Error ? error.message : `Failed to toggle ${addon.displayName}.`);
    } finally {
      const next = new Set(togglingStems);
      next.delete(addon.jarStem);
      togglingStems = next;
    }
  }

  async function updateAddon(addon: Schema['AddonItemDTO']): Promise<void> {
    updatingStems = new Set(updatingStems).add(addon.jarStem);
    try {
      const result = await mutate<Schema['AddonUpdateResultDTO']>(api, addonPaths.update, {
        jarStem: addon.jarStem,
      });
      if (result.operationId) await pollOperation(api, result.operationId);
      flash(`${addon.displayName} updated — restart the server to apply.`);
      await loadAddons();
    } catch (error) {
      flash(error instanceof Error ? error.message : `Failed to update ${addon.displayName}.`);
    } finally {
      const next = new Set(updatingStems);
      next.delete(addon.jarStem);
      updatingStems = next;
    }
  }

  async function updateAllAddons(): Promise<void> {
    updatingAll = true;
    try {
      const result = await mutate<Schema['AddonUpdateResultDTO']>(api, addonPaths.update, {
        updateAll: true,
      });
      if (result.operationId) await pollOperation(api, result.operationId);
      flash(`Updated ${result.count} ${addonFolderName} — restart the server to apply.`);
      await loadAddons();
    } catch (error) {
      flash(error instanceof Error ? error.message : 'Failed to update all.');
    } finally {
      updatingAll = false;
    }
  }

  async function removeAddon(addon: Schema['AddonItemDTO']): Promise<void> {
    confirmingRemove = undefined;
    try {
      const result = await mutate<Schema['AddonRemoveResultDTO']>(api, addonPaths.remove, {
        jarStem: addon.jarStem,
      });
      flash(result.message);
      addons = addons.filter((item) => item.jarStem !== addon.jarStem);
      checkVoiceTunnelPrompt();
    } catch (error) {
      flash(error instanceof Error ? error.message : `Failed to remove ${addon.displayName}.`);
    }
  }

  function openAddonMenu(event: MouseEvent, addon: Schema['AddonItemDTO']): void {
    addonMenu = { addon, x: event.clientX, y: event.clientY };
  }

  function addonDetailItem(addon: Schema['AddonItemDTO']): ProjectDetailItem {
    return {
      projectId: addon.projectId as string,
      title: addon.displayName,
      iconURL: addon.iconURL,
    };
  }

  function pickBrowserFile(): Promise<{ name: string; bytes: Uint8Array } | null> {
    return new Promise((resolve) => {
      fileInput.addEventListener(
        'change',
        async () => {
          const browserFile = fileInput.files?.[0];
          resolve(
            browserFile
              ? { name: browserFile.name, bytes: new Uint8Array(await browserFile.arrayBuffer()) }
              : null,
          );
        },
        { once: true },
      );
      fileInput.click();
    });
  }

  async function addLocalAddon(): Promise<void> {
    if (!api?.upload) return;
    const picked = await (
      await getPlatform()
    ).pickFile({ label: `Choose a ${isModded ? 'mod' : 'plugin'} JAR`, extensions: ['jar'] }, () =>
      pickBrowserFile(),
    );
    if (!picked) return;
    try {
      const staged = await api.upload('addon-local-file', picked.bytes);
      const result = await mutate<Schema['CatalogInstallResultDTO']>(api, addonPaths.install, {
        stagedUploadId: staged.stagedUploadId,
      });
      if (result.operationId) await pollOperation(api, result.operationId);
      flash(result.message);
      await loadAddons();
    } catch (error) {
      flash(error instanceof Error ? error.message : 'Failed to add that file.');
    }
  }

  function onVersionChanged(): void {
    flash('Version change started.');
    void Promise.all([loadComponents(), loadServers()]);
  }

  async function toggleAutostart(): Promise<void> {
    try {
      broadcastAutostart = await mutate<Schema['BroadcastAutoStartDTO']>(
        api,
        broadcastPaths.autostart,
        { enabled: !broadcastAutostart.enabled },
      );
    } catch (error) {
      flash(error instanceof Error ? error.message : 'Failed to change the broadcast setting.');
    }
  }

  async function downloadBroadcastJar(): Promise<void> {
    downloadingBroadcastJar = true;
    try {
      const result = await mutate<Schema['BroadcastJarDownloadResultDTO']>(
        api,
        broadcastPaths.downloadJar,
      );
      if (result.operationId) await pollOperation(api, result.operationId);
      flash(result.message);
      jarStatus = await call(api, jarStatus, broadcastPaths.jarStatus);
    } catch (error) {
      flash(error instanceof Error ? error.message : 'Failed to download the broadcast JAR.');
    } finally {
      downloadingBroadcastJar = false;
    }
  }

  function onManualFilesPending(
    operationId: string,
    files: Schema['ModpackManualFileEntryDTO'][],
  ): void {
    pendingManualFiles = { operationId, files };
  }

  let refreshTimer: ReturnType<typeof setInterval> | undefined;
  onMount(() => {
    void loadAll();
    refreshTimer = setInterval(() => void loadAll(), 10000);
  });
  onDestroy(() => {
    if (refreshTimer) clearInterval(refreshTimer);
  });
</script>

<div class="components">
  <div class="section-header">
    <div class="overline">
      <Icon name="chip" size={13} />
      <span class="msc2-type-overline">Components</span>
    </div>
    <div class="header-actions">
      {#if isModded}
        <Button size="sm" variant="secondary" onclick={() => (showImportModpack = true)}>
          Import Modpack
        </Button>
      {/if}
      {#if kind}
        <Button size="sm" variant="secondary" onclick={() => (showBrowser = true)}>
          Browse {isModded ? 'mods' : 'plugins'}
        </Button>
      {/if}
      <Button size="sm" variant="secondary" onclick={() => void loadAll()}>Refresh</Button>
    </div>
  </div>

  {#if notice}<p class="notice" role="status">{notice}</p>{/if}

  {#if isBedrock}
    <section class="zone">
      <p class="msc2-type-overline">Bedrock Server</p>
      <Card padding="0">
        <div class="row">
          <div class="info">
            <span class="name">Bedrock Dedicated Server</span>
            <span class="subtitle">{primaryComponent?.installedLabel ?? 'Not installed'}</span>
          </div>
          <Button
            size="sm"
            variant="secondary"
            disabled={serverRunning}
            title={serverRunning ? 'Stop the server before changing its version' : undefined}
            onclick={() => (showVersionPicker = true)}
          >
            Change Version
          </Button>
        </div>
      </Card>
    </section>
  {:else}
    <section class="zone">
      <p class="msc2-type-overline">Server</p>
      <Card padding="0">
        <div class="row">
          <div class="info">
            <div class="title-row">
              <span class="name">{flavorDisplayName(activeServer?.javaFlavor)}</span>
              <Badge variant="category">{isModded ? 'Loader' : 'Server JAR'}</Badge>
            </div>
            <span class="subtitle" class:error={!primaryComponent}>
              {primaryComponent?.installedLabel ?? 'Not installed'}
            </span>
          </div>
          {#if primaryComponent && !isModded}
            <StatusDot
              tone={componentTone(primaryComponent)}
              label={componentStatusLabel(primaryComponent)}
            />
          {/if}
          <Button
            size="sm"
            variant="secondary"
            disabled={serverRunning}
            title={serverRunning ? 'Stop the server before changing its version' : undefined}
            onclick={() => (showVersionPicker = true)}
          >
            Change Version
          </Button>
        </div>
      </Card>
    </section>

    {#if kind}
      <section class="zone">
        <div class="section-header">
          <p class="msc2-type-overline">{isModded ? 'Mods' : 'Plugins'}</p>
          {#if anyAddonUpdatable}
            <Button
              size="sm"
              variant="secondary"
              disabled={updatingAll}
              onclick={() => void updateAllAddons()}
            >
              {updatingAll ? 'Updating…' : 'Update All'}
            </Button>
          {/if}
        </div>
        {#if addons.length === 0}
          <EmptyState title={`No ${isModded ? 'mods' : 'plugins'} installed`}>
            <Icon name="box" size={26} slot="icon" />
          </EmptyState>
        {:else}
          <Card padding="0">
            {#each addons as addon, index (addon.jarStem)}
              <div
                class="addon-row"
                class:bordered={index > 0}
                class:disabled={!addon.isEnabled}
                class:selected={addonMenu?.addon.jarStem === addon.jarStem}
              >
                {#if confirmingRemove === addon.jarStem}
                  <div class="info">
                    <span class="name">{addon.displayName}</span>
                  </div>
                  <span class="confirm">Uninstall?</span>
                  <Button
                    size="sm"
                    variant="secondary"
                    onclick={() => (confirmingRemove = undefined)}
                  >
                    Cancel
                  </Button>
                  <Button size="sm" variant="destructive" onclick={() => void removeAddon(addon)}>
                    Uninstall
                  </Button>
                {:else}
                  <button
                    type="button"
                    class="addon-row-link"
                    onclick={(event) => openAddonMenu(event, addon)}
                  >
                    <div class="info">
                      <span class="title-row">
                        <span class="name">{addon.displayName}</span>
                        <span class="row-affordance"><Icon name="chevron" size={10} /></span>
                      </span>
                      <span class="subtitle">
                        {addon.currentVersion ?? 'Unknown version'}
                        {#if addon.bucket === 'updateAvailable' && addon.availableVersion}
                          → {addon.availableVersion}
                        {/if}
                      </span>
                    </div>
                    {#if addonStatusLabel(addon)}
                      <StatusDot tone="warn" label={addonStatusLabel(addon) ?? ''} />
                    {/if}
                  </button>
                  {#if addon.bucket === 'updateAvailable'}
                    <Button
                      size="sm"
                      variant="secondary"
                      disabled={updatingStems.has(addon.jarStem)}
                      onclick={() => void updateAddon(addon)}
                    >
                      {updatingStems.has(addon.jarStem) ? 'Updating…' : 'Update'}
                    </Button>
                  {/if}
                {/if}
              </div>
            {/each}
          </Card>
        {/if}
        <div class="footer-actions">
          <span title="File access lands with the Files tab (P12.9) — not available yet">
            <Button size="sm" variant="secondary" disabled>
              Reveal {addonFolderName} folder
            </Button>
          </span>
          <Button size="sm" variant="primary" onclick={() => void addLocalAddon()}>
            Add {isModded ? 'Mod' : 'Plugin'}
          </Button>
        </div>
      </section>
    {/if}

    {#if crossplayEligible}
      <section class="zone">
        <p class="msc2-type-overline">Crossplay</p>
        <Card padding="0">
          <div class="row">
            <Toggle
              checked={broadcastAutostart.enabled}
              label="Enable MCXboxBroadcast"
              onchange={() => void toggleAutostart()}
            />
            <div class="info">
              <div class="title-row">
                <span class="name">MCXboxBroadcast</span>
                <Badge variant="category">Broadcast</Badge>
              </div>
              <span class="subtitle">{jarStatus.filename ?? 'Not downloaded'}</span>
            </div>
            <StatusDot
              tone={jarStatus.installed ? 'ok' : 'error'}
              label={jarStatus.installed ? 'Installed' : 'Missing'}
            />
            <Button
              size="sm"
              variant="secondary"
              disabled={downloadingBroadcastJar || jarStatus.downloading}
              onclick={() => void downloadBroadcastJar()}
            >
              {downloadingBroadcastJar || jarStatus.downloading ? 'Downloading…' : 'Download'}
            </Button>
          </div>
        </Card>
      </section>
    {/if}
  {/if}

  {#if isBedrock}
    <section class="zone">
      <p class="msc2-type-overline">Crossplay</p>
      <Card padding="0">
        <div class="row">
          <div class="info">
            <div class="title-row">
              <span class="name">MCXboxBroadcast</span>
              <Badge variant="category">Broadcast</Badge>
            </div>
            <span class="subtitle">{jarStatus.filename ?? 'Not downloaded'}</span>
          </div>
          <StatusDot
            tone={broadcastStatus.bedrockBroadcastRunning ? 'ok' : 'error'}
            label={broadcastStatus.bedrockBroadcastRunning ? 'Running' : 'Stopped'}
          />
          <Button
            size="sm"
            variant="secondary"
            disabled={downloadingBroadcastJar || jarStatus.downloading}
            onclick={() => void downloadBroadcastJar()}
          >
            {downloadingBroadcastJar || jarStatus.downloading ? 'Downloading…' : 'Download JAR'}
          </Button>
        </div>
      </Card>
    </section>
  {/if}

  <input bind:this={fileInput} type="file" accept=".jar" class="hidden-input" />
</div>

{#if showVersionPicker}
  <VersionPickerSheet
    {api}
    title={isBedrock
      ? 'Change Bedrock Version'
      : isModded
        ? 'Change Loader Version'
        : 'Change Server Version'}
    {serverRunning}
    onClose={() => (showVersionPicker = false)}
    onChanged={onVersionChanged}
  />
{/if}

{#if addonMenu}
  {@const menuAddon = addonMenu.addon}
  <Menu
    x={addonMenu.x}
    y={addonMenu.y}
    onClose={() => (addonMenu = undefined)}
    items={[
      {
        label: menuAddon.isEnabled ? 'Disable' : 'Enable',
        onSelect: () => void toggleAddon(menuAddon),
      },
      {
        label: 'View',
        disabled: !menuAddon.projectId,
        onSelect: () => (detailAddon = menuAddon),
      },
      {
        label: 'Uninstall',
        tone: 'destructive',
        onSelect: () => (confirmingRemove = menuAddon.jarStem),
      },
    ]}
  />
{/if}

{#if detailAddon}
  <ProjectDetailSheet
    {api}
    item={addonDetailItem(detailAddon)}
    javaFlavor={activeServer?.javaFlavor}
    serverMinecraftVersion={primaryComponent?.installedVersion}
    onClose={() => (detailAddon = undefined)}
    onInstalled={() => void loadAddons()}
  />
{/if}

{#if showBrowser && kind}
  <PluginBrowserSheet
    {api}
    addOnKind={kind}
    javaFlavor={activeServer?.javaFlavor}
    serverMinecraftVersion={primaryComponent?.installedVersion}
    onClose={() => (showBrowser = false)}
    onInstalled={() => void loadAddons()}
  />
{/if}

{#if showImportModpack}
  <ImportModpackSheet
    {api}
    onClose={() => (showImportModpack = false)}
    onImported={() => void loadAddons()}
    {onManualFilesPending}
  />
{/if}

{#if pendingManualFiles}
  <CurseForgeManualDownloadSheet
    {api}
    operationId={pendingManualFiles.operationId}
    files={pendingManualFiles.files}
    onClose={() => (pendingManualFiles = undefined)}
    onAllResolved={() => {
      pendingManualFiles = undefined;
      flash('All files resolved — import finishing in the background.');
      void loadAddons();
    }}
  />
{/if}

{#if showVoicePrompt && svcAddon}
  <Sheet title="Simple Voice Chat needs a tunnel" size="sm" onClose={() => (showVoicePrompt = false)}>
    <div class="voice-prompt">
      <p>
        Simple Voice Chat is enabled, but this server has no MSC Voice Playit tunnel. Voice chat
        will not work for players connecting through Playit.
      </p>
      {#if serverRunning}
        <p class="hint">
          The server is running. Simple Voice Chat reads <code>voice_host</code> when it starts, so
          restart the server after the tunnel is configured.
        </p>
      {/if}
      <div class="prompt-actions">
        <Button variant="primary" onclick={openVoiceSetup}>Set up voice tunnel</Button>
        <Button variant="secondary" onclick={disableVoiceChat}>Disable Voice Chat</Button>
        <Button variant="secondary" onclick={dismissVoicePrompt}>Don't Ask Again</Button>
      </div>
    </div>
  </Sheet>
{/if}

{#if showPlayitSetup}
  <PlayitSetupSheet
    {api}
    {playit}
    context="settings"
    voiceOnly={playitSetupVoiceOnly}
    onClose={() => (showPlayitSetup = false)}
    onComplete={() => void refreshPlayit()}
    onReset={() => void refreshPlayit()}
  />
{/if}

<style>
  .components {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
  }
  .overline {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--msc2-text-tertiary);
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
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
  .row,
  .addon-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
  }
  .addon-row.bordered {
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .addon-row.disabled .name {
    color: var(--msc2-text-secondary);
  }
  .addon-row.selected {
    border-radius: var(--msc2-radius-2);
    background: rgba(59, 130, 246, 0.06);
    box-shadow: inset 0 0 0 1.5px var(--msc2-selection);
  }
  .row-affordance {
    display: inline-flex;
    color: var(--msc2-text-tertiary);
  }
  .addon-row-link {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
    min-width: 0;
    background: none;
    border: none;
    padding: 0;
    text-align: left;
    cursor: pointer;
    color: inherit;
    font: inherit;
  }
  .info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }
  .title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .name {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .subtitle {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .subtitle.error {
    color: var(--msc2-status-error);
  }
  .confirm {
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .footer-actions {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
  .hidden-input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    overflow: hidden;
  }
  .voice-prompt {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .voice-prompt p {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--msc2-text-secondary);
  }
  .voice-prompt .hint {
    color: var(--msc2-text-tertiary);
  }
  .voice-prompt code {
    font-family: var(--msc2-font-mono);
    font-size: 11px;
  }
  .prompt-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding-top: 4px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
</style>
