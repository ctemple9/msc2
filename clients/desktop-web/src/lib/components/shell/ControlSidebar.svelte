<script lang="ts">
  // Server-control rail: host-aware picker, Start/Stop, Manage…, and MSC 1's
  // four collapsible sections, all real now (P12.21, P12.22).
  // docs/msc2/renderings/shell.html, MSC 1 SidebarView.swift.
  import Button from '../base/Button.svelte';
  import Menu from '../base/Menu.svelte';
  import Icon from '../base/Icon.svelte';
  import ShellIcon from './ShellIcon.svelte';
  import PlayerAvatar from './PlayerAvatar.svelte';
  import HowToConnectSection from './sidebar/HowToConnectSection.svelte';
  import ConsoleAccessSection from './sidebar/ConsoleAccessSection.svelte';
  import QuickCommandsSection from './sidebar/QuickCommandsSection.svelte';
  import { bannerColorAccent } from '../../styles/bannerColor';
  import { getPlatform } from '../../platform';
  import { JAVA_FLAVOR_CATALOG, crossPlayUnavailable } from '../../sections/fleet/wizard/model';
  import type { JavaCategory, JavaFlavor } from '../../sections/fleet/wizard/model';
  import type { HostId, HostRecord } from '../../hosts/types';
  import type { Schema, ScreenApi } from '../../sections/shared/types';

  // JAVA_FLAVOR_CATALOG only covers the 6 flavors the creation wizard offers
  // (isAvailableInCreateFlow) -- an imported or pre-existing server can carry
  // a javaFlavor outside that set (pufferfish, spigot, quilt). The oracle's
  // full JavaServerFlavor enum (JavaServerFlavor.swift) fixes every flavor's
  // category regardless of Create-flow availability: pufferfish/spigot are
  // standard (plugin) like Paper/Purpur; quilt is modded (loader) like
  // Fabric/NeoForge/Forge. This fills that gap rather than mis-hiding
  // Xbox Broadcast for a flavor the wizard's own catalog doesn't list.
  const MODDED_FLAVORS = new Set(['fabric', 'neoforge', 'forge', 'quilt']);

  function javaFlavorCategory(flavor: string): JavaCategory {
    const known = JAVA_FLAVOR_CATALOG.find((entry) => entry.id === flavor);
    if (known) return known.category;
    return MODDED_FLAVORS.has(flavor) ? 'modded' : 'standard';
  }

  export let hostLabel: string;
  export let hosts: readonly HostRecord[] = [];
  export let activeHostId: HostId = '';
  export let isDesktopShell = false;
  // Threaded through for the sections that call the agent directly
  // (How to Connect, Services, and Quick Commands) — the same `api` every
  // section and ConsoleDock (P12.10) already receive.
  export let api: ScreenApi | undefined = undefined;
  export let servers: readonly Schema['ServerDTO'][] = [];
  export let activeServerId: string | undefined = undefined;
  export let running = false;
  export let connected = false;
  export let canControl = true;
  export let bannerColor: string;
  export let onSelectServer: (id: string) => void;
  export let onSwitchHost: (id: HostId) => void = () => undefined;
  export let onLifecycle: (action: 'start' | 'stop') => void;
  export let onOpenAgentSetup: () => void;
  export let onManage: () => void;

  let pickerOpen = false;
  let pickerPos = { x: 0, y: 0 };

  function openPicker(event: MouseEvent): void {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    pickerPos = { x: rect.left, y: rect.bottom + 4 };
    pickerOpen = true;
  }

  $: multiHost = isDesktopShell && hosts.length > 1;

  type PickerItem = { label: string; onSelect: () => void; disabled?: boolean };

  function buildPickerItems(): PickerItem[] {
    if (!multiHost) {
      return [
        ...servers.map((server): PickerItem => ({
          label: server.name,
          onSelect: () => onSelectServer(server.id),
        })),
        { label: 'Agent…', onSelect: onOpenAgentSetup },
        { label: 'Manage…', onSelect: onManage },
      ];
    }
    const items: PickerItem[] = [];
    for (const host of hosts) {
      items.push({ label: `— ${host.label} —`, onSelect: () => {}, disabled: true });
      if (host.id === activeHostId) {
        for (const server of servers) {
          items.push({ label: server.name, onSelect: () => onSelectServer(server.id) });
        }
      } else {
        items.push({ label: 'Switch to this host…', onSelect: () => onSwitchHost(host.id) });
      }
    }
    items.push({ label: 'Agent…', onSelect: onOpenAgentSetup });
    items.push({ label: 'Manage…', onSelect: onManage });
    return items;
  }

  // Referenced directly (not just through buildPickerItems' internals) so
  // Svelte re-runs this whenever any of them changes.
  $: pickerItems = (() => {
    void multiHost;
    void hosts;
    void servers;
    void activeHostId;
    return buildPickerItems();
  })();

  const DISCLOSURE_SECTIONS = [
    'Services',
    'How to connect',
    'Maintenance',
    'Quick commands',
  ] as const;
  let expanded: Record<(typeof DISCLOSURE_SECTIONS)[number], boolean> = {
    Services: false,
    'How to connect': false,
    Maintenance: false,
    'Quick commands': false,
  };

  function toggle(section: (typeof DISCLOSURE_SECTIONS)[number]): void {
    expanded = { ...expanded, [section]: !expanded[section] };
  }

  // Owns the eye toggle rendered in How to Connect's own header row (moved
  // there per Cameron's own visual review -- the oracle keeps it inside the
  // section's content, but this reads cleaner one row up, next to the
  // section title itself, with no "Show"/"Hide" text needed alongside it).
  // Only rendered while the section is expanded, and always resets to
  // hidden (masked) when the section collapses -- Cameron's own call: never
  // remember a "shown" state across a closed disclosure.
  let showAddresses = false;
  $: if (!expanded['How to connect']) showAddresses = false;

  $: activeServer = servers.find((server) => server.id === activeServerId);

  // Services is MSC2's own external-services panel (playit.gg + Xbox
  // Broadcast, not an oracle 1:1 port; MSC 1 calls its Xbox-Broadcast-only
  // equivalent "Console Access"), so it shows whenever a server is
  // selected -- playit applies to any server type/flavor. Xbox Broadcast's
  // own crossplay rule (MSC 1 SidebarView.swift's `showCrossPlatform`:
  // always for Bedrock; for Java, only flavors that can host Geyser/Xbox
  // Broadcast, not Vanilla, not a modded loader) only gates its own
  // sub-block within the section, passed down separately.
  $: showXboxBroadcast =
    activeServer !== undefined &&
    (activeServer.serverType === 'bedrock' ||
      (!!activeServer.javaFlavor &&
        !crossPlayUnavailable(
          javaFlavorCategory(activeServer.javaFlavor),
          activeServer.javaFlavor as JavaFlavor,
        )));

  // MSC 1's SidebarView.swift hides How to Connect entirely with no server
  // selected (`if viewModel.selectedServer != nil`); Maintenance always
  // shows, its two buttons disabled instead.
  $: visibleSections = DISCLOSURE_SECTIONS.filter((section) => {
    if (section === 'How to connect') return !!activeServer;
    if (section === 'Services') return !!activeServer;
    return true;
  });

  let maintenanceNotice = '';

  async function revealFolder(path: string): Promise<void> {
    maintenanceNotice = '';
    await (
      await getPlatform()
    ).revealInFileManager(path, async () => {
      maintenanceNotice = 'This needs the desktop app.';
    });
  }

  function openServerDirectory(): void {
    if (!activeServer) return;
    void revealFolder(activeServer.directory);
  }

  function openLogsDirectory(): void {
    if (!activeServer) return;
    void revealFolder(`${activeServer.directory}/logs`);
  }
</script>

<aside class="sidebar" aria-label="Server controls">
  <div class="scroll">
    <div class="block">
      <p class="overline">Server controls</p>

      <button
        type="button"
        class="picker"
        style="background: {bannerColorAccent(bannerColor, 0.12)};"
        aria-haspopup="menu"
        onclick={openPicker}
      >
        <span class="sr-only">{connected ? 'Connected' : 'Disconnected'}</span>
        <span class="picker-label">{hostLabel} ▸ {activeServer?.name ?? 'No server'}</span>
        <ShellIcon name="selector" size={14} />
      </button>
      {#if pickerOpen}
        <Menu
          x={pickerPos.x}
          y={pickerPos.y}
          onClose={() => (pickerOpen = false)}
          items={pickerItems}
        />
      {/if}

      <div class="control-row">
        <Button
          variant={running ? 'stop' : 'start'}
          size="sm"
          disabled={!canControl || !activeServer}
          onclick={() => onLifecycle(running ? 'stop' : 'start')}
          anchorId="ob_start_button"
        >
          <ShellIcon name="play" size={13} />
          {running ? 'Stop' : 'Start'}
        </Button>
        <Button variant="secondary" size="sm" onclick={onManage} anchorId="ob_manage_servers"
          >Manage…</Button
        >
      </div>
    </div>

    {#each visibleSections as section (section)}
      <div class="disclosure">
        <div class="disclosure-header">
          <button
            type="button"
            class="disclosure-toggle"
            aria-expanded={expanded[section]}
            onclick={() => toggle(section)}
          >
            <ShellIcon name={expanded[section] ? 'chevron-down' : 'chevron-right'} size={11} />
            <span class="overline">{section}</span>
          </button>
          {#if section === 'How to connect' && expanded[section]}
            <button
              type="button"
              class="eye-toggle"
              aria-label={showAddresses ? 'Hide addresses' : 'Show addresses'}
              onclick={() => (showAddresses = !showAddresses)}
            >
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
            </button>
          {/if}
        </div>
        {#if expanded[section]}
          <div class="disclosure-content">
            {#if section === 'Services'}
              <ConsoleAccessSection
                {api}
                serverType={activeServer?.serverType}
                {activeServerId}
                {canControl}
                {showXboxBroadcast}
              />
            {:else if section === 'How to connect'}
              <HowToConnectSection
                {api}
                serverType={activeServer?.serverType}
                {activeServerId}
                {showAddresses}
              />
            {:else if section === 'Maintenance'}
              <div class="maintenance-row">
                <button
                  type="button"
                  class="maintenance-button"
                  disabled={!activeServer}
                  onclick={openServerDirectory}
                >
                  <Icon name="folder" size={13} />
                  <span>Directory</span>
                </button>
                <button
                  type="button"
                  class="maintenance-button"
                  disabled={!activeServer}
                  onclick={openLogsDirectory}
                >
                  <Icon name="note" size={13} />
                  <span>Logs</span>
                </button>
              </div>
              {#if maintenanceNotice}
                <p class="maintenance-notice">{maintenanceNotice}</p>
              {/if}
            {:else}
              <QuickCommandsSection
                {api}
                {activeServerId}
                {running}
                isBedrock={activeServer?.serverType === 'bedrock'}
                {canControl}
              />
            {/if}
          </div>
        {/if}
      </div>
    {/each}

    <div class="block actions-block">
      <p class="overline">Actions</p>
      <p class="avatar-title">Your Avatar</p>
      <PlayerAvatar />
    </div>
  </div>
</aside>

<style>
  .sidebar {
    width: 240px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: var(--msc2-tier-chrome);
    border-right: 1px solid var(--msc2-hairline-faint);
    min-height: 0;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    padding: 14px 12px 8px;
  }
  .overline {
    margin: 0;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.8px;
    text-transform: uppercase;
    color: var(--msc2-text-tertiary);
  }
  .block {
    margin-bottom: 14px;
  }
  .picker {
    position: relative;
    display: flex;
    width: 100%;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
    padding: 7px 9px;
    font: inherit;
    text-align: left;
    border: 1px solid var(--msc2-hairline-field);
    border-radius: 8px;
    color: var(--msc2-text-secondary);
    cursor: pointer;
    box-sizing: border-box;
  }
  .picker:focus-visible {
    border-color: var(--msc2-hairline-field-focus);
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .picker-label {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    color: var(--msc2-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .control-row {
    display: flex;
    gap: 6px;
    margin-top: 8px;
  }
  .control-row :global(.btn.secondary) {
    flex-shrink: 0;
  }
  .disclosure {
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .disclosure-header {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 8px 2px;
  }
  .disclosure-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
    padding: 0;
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.5);
    cursor: pointer;
    text-align: left;
  }
  .disclosure-toggle:hover {
    color: rgba(255, 255, 255, 0.8);
  }
  .disclosure-toggle:focus-visible {
    outline: 2px solid rgba(255, 255, 255, 0.4);
  }
  .eye-toggle {
    display: flex;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    padding: 0;
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.4);
    cursor: pointer;
  }
  .eye-toggle:hover {
    color: rgba(255, 255, 255, 0.75);
  }
  .eye-toggle:focus-visible {
    outline: 2px solid rgba(255, 255, 255, 0.4);
  }
  .disclosure-content {
    margin: 0 0 10px;
  }
  .maintenance-row {
    display: flex;
    gap: 6px;
  }
  .maintenance-button {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 8px 4px;
    font: inherit;
    color: var(--msc2-text-secondary);
    background: rgba(255, 255, 255, 0.04);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    cursor: pointer;
  }
  .maintenance-button:hover:not(:disabled) {
    color: var(--msc2-text-primary);
    background: rgba(255, 255, 255, 0.07);
  }
  .maintenance-button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .maintenance-button span {
    font-size: 10px;
    font-weight: 500;
  }
  .maintenance-notice {
    margin: 6px 0 0;
    font-size: 10px;
    color: var(--msc2-text-tertiary);
  }
  .actions-block {
    flex-shrink: 0;
    padding-top: 12px;
    margin-top: auto;
    margin-bottom: 4px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .avatar-title {
    margin: 4px 0 8px;
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
</style>
