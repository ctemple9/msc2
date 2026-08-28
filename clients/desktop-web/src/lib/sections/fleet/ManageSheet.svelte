<script lang="ts">
  // Ports MSC 1 ManageServersView.swift: a flat, card-per-server list with a
  // header count, per-card context menu (Set as Active / Edit... / Remove),
  // and a footer for Import.../Add Server... -- adapted for D-013 multi-host
  // (docs/msc2/msc2-decisions.md#D-013, 2026-08-27 design discussion).
  // "Edit..." (P12.12) opens ServerEditorSheet, the port of
  // ServerEditorView.swift's General/Broadcast tabs -- renaming now lives
  // there rather than as its own menu item, matching the oracle (MSC 1 has
  // no separate Rename action either; it's a field inside the editor).
  //
  // Multi-host chrome (host-group headers, Add Host) is Tauri-only: a browser
  // tab can only ever reach the single agent that served it
  // (src/lib/platform/index.ts's createAgentTransport always uses
  // window.location.origin off Tauri, with no per-host baseUrl), so a browser
  // never has more than the one host to show. With exactly one host on either
  // platform, this renders with zero host-group chrome -- pixel-equivalent to
  // MSC 1's own flat list.
  //
  // Known, deliberate gaps against the oracle (not silently glossed over):
  // - MSC 1's delete alert offers "Delete from Disk" vs "Remove Only";
  //   ServerDeleteRequestDTO (openapi.json) carries only `serverId`, no
  //   disk-delete flag, so only "Remove from Controller" is offered here.
  // - MSC 1's footer has Export.../Import.../Add Server...; there is no
  //   `/v1/servers/export` route in the contract at all, so Export is
  //   omitted rather than wired to nothing.
  // - "Add Server..." opens AddServerWizard.svelte (P12.18a-i), a real port
  //   of MSC 1's multi-step AddServerWizardView. The Fresh path (shell,
  //   Choose Path, Configure, Network, World, Add-ons, and the real Confirm
  //   + create) is real end to end as of P12.18g; the Import path is still a
  //   placeholder until P12.18h. `refreshServers` (already used after
  //   Import) also runs once the wizard's own create succeeds, so a newly
  //   created server shows up here without closing and reopening this sheet.
  import Sheet from '../../components/base/Sheet.svelte';
  import Card from '../../components/base/Card.svelte';
  import Button from '../../components/base/Button.svelte';
  import Badge from '../../components/base/Badge.svelte';
  import StatusDot from '../../components/base/StatusDot.svelte';
  import Field from '../../components/base/Field.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import Menu from '../../components/base/Menu.svelte';
  import ServerEditorSheet from '../server-editor/ServerEditorSheet.svelte';
  import AddServerWizard from './wizard/AddServerWizard.svelte';
  import type { HostId, HostRecord } from '../../hosts/types';
  import type { Schema, ScreenApi } from '../shared/types';
  import { errorMessage, mutate } from '../shared/types';
  import { fleetMutationPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let servers: readonly Schema['ServerDTO'][] = [];
  export let status: Schema['RemoteAPIStatus'] = { running: false };
  export let permissions: readonly string[] = [];
  export let hosts: readonly HostRecord[] = [];
  export let hostSummaries: ReadonlyMap<HostId, { connection: string; serverCount: number }> =
    new Map();
  export let activeHostId: HostId = '';
  export let isDesktopShell = false;
  export let onClose: () => void;
  export let onSwitchHost: (id: HostId) => void;
  export let onAddHost: (label: string, baseUrl: string, pairingCode: string) => Promise<string>;
  export let onRemoveHost: (id: HostId) => void;
  export let onServersChanged: (servers: readonly Schema['ServerDTO'][]) => void;
  /** Called after `setActive`'s own `POST /v1/active-server` succeeds, so the
   *  caller can sync its *local* active-server state (sidebar dropdown,
   *  header, section routing) -- `App.svelte`'s own `selectServer` already
   *  does this for the sidebar's server picker, but this sheet posted the
   *  same mutation directly and never told the parent, so "Set Active" here
   *  changed the agent's active server without the client ever finding out.
   *  Real bug Cameron hit verifying P12.18g: a just-created server's own
   *  "Set Active" button did nothing visible until picked again from the
   *  sidebar dropdown, which goes through `selectServer` instead. */
  export let onActivated: (serverId: string) => void = () => {};

  const canControl =
    permissions.length === 0 ||
    permissions.includes('serverControl') ||
    permissions.includes('admin');

  let importPath = '';
  let showImport = false;
  let showWizard = false;
  let showAddHost = false;
  let notice = '';

  let openMenuFor: string | undefined;
  let menuPos = { x: 0, y: 0 };
  let confirmingRemoveId: string | undefined;
  let editingServer: Schema['ServerDTO'] | undefined;

  let addHostLabel = '';
  let addHostUrl = '';
  let addHostCode = '';
  let addHostBusy = false;
  let addHostError = '';

  $: multiHost = isDesktopShell && hosts.length > 1;

  async function refreshServers(): Promise<void> {
    if (!api) return;
    onServersChanged(await api.get<Schema['ServerDTO'][]>('/v1/servers'));
  }

  function openMenu(serverId: string, event: MouseEvent): void {
    menuPos = { x: event.clientX, y: event.clientY };
    openMenuFor = serverId;
  }

  async function setActive(serverId: string): Promise<void> {
    try {
      await mutate(api, fleetMutationPaths.active, { serverId });
      onActivated(serverId);
    } catch (error) {
      notice = errorMessage(error);
    }
  }

  async function removeServer(serverId: string): Promise<void> {
    confirmingRemoveId = undefined;
    try {
      const result = await mutate<Schema['ServerDeleteResultDTO']>(api, fleetMutationPaths.delete, {
        serverId,
      });
      notice = result.message;
      await refreshServers();
    } catch (error) {
      notice = errorMessage(error);
    }
  }

  async function importServer(): Promise<void> {
    const sourcePath = importPath.trim();
    if (!sourcePath) return;
    try {
      const result = await mutate<Schema['ServerImportResultDTO']>(api, fleetMutationPaths.import, {
        action: 'importExisting',
        sourcePath,
        acceptEula: false,
      });
      notice = result.message;
      importPath = '';
      showImport = false;
      await refreshServers();
    } catch (error) {
      notice = errorMessage(error);
    }
  }

  async function submitAddHost(): Promise<void> {
    const label = addHostLabel.trim();
    const baseUrl = addHostUrl.trim();
    const pairingCode = addHostCode.trim();
    if (!label || !baseUrl || !pairingCode) return;
    addHostBusy = true;
    addHostError = '';
    try {
      await onAddHost(label, baseUrl, pairingCode);
      addHostLabel = '';
      addHostUrl = '';
      addHostCode = '';
      showAddHost = false;
    } catch (error) {
      addHostError = errorMessage(error);
    } finally {
      addHostBusy = false;
    }
  }

  function badgeTone(server: Schema['ServerDTO']): 'ok' | 'bedrock' {
    return server.serverType === 'bedrock' ? 'bedrock' : 'ok';
  }
</script>

{#snippet serverCard(server: Schema['ServerDTO'])}
  {@const isActive = server.id === status.activeServerId}
  <Card padding="0">
    <div class="server-row" class:active={isActive}>
      <div class="server-icon" class:active={isActive}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path
            d="M4 6h16v4H4zM4 14h16v4H4z"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linejoin="round"
          />
        </svg>
      </div>
      <div class="server-info">
        <div class="server-title">
          <span class="server-name">{server.name || '(no name)'}</span>
          <Badge variant="status" tone={badgeTone(server)}
            >{server.serverType === 'bedrock' ? 'BEDROCK' : 'JAVA'}</Badge
          >
          {#if isActive}<Badge variant="status" tone="ok">ACTIVE</Badge>{/if}
        </div>
        <p class="server-dir">{server.directory}</p>
      </div>
      <Button variant="secondary" size="sm" onclick={() => setActive(server.id)}>Set Active</Button>
      <Button
        variant="ghost-icon"
        size="sm"
        label="More actions"
        onclick={(event) => openMenu(server.id, event)}
      >
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <circle cx="12" cy="5" r="1.6" fill="currentColor" />
          <circle cx="12" cy="12" r="1.6" fill="currentColor" />
          <circle cx="12" cy="19" r="1.6" fill="currentColor" />
        </svg>
      </Button>
    </div>
    {#if confirmingRemoveId === server.id}
      <div class="confirm-row">
        <span
          >Remove "{server.name}" from this controller? The agent refuses this while running.</span
        >
        <div class="confirm-actions">
          <Button variant="secondary" size="sm" onclick={() => (confirmingRemoveId = undefined)}
            >Cancel</Button
          >
          <Button variant="destructive" size="sm" onclick={() => removeServer(server.id)}
            >Remove from Controller</Button
          >
        </div>
      </div>
    {/if}
  </Card>
  {#if openMenuFor === server.id}
    <Menu
      x={menuPos.x}
      y={menuPos.y}
      onClose={() => (openMenuFor = undefined)}
      items={[
        { label: 'Set as Active', onSelect: () => setActive(server.id) },
        { label: 'Edit…', onSelect: () => (editingServer = server) },
        {
          label: 'Remove…',
          tone: 'destructive',
          disabled: !canControl,
          onSelect: () => (confirmingRemoveId = server.id),
        },
      ]}
    />
  {/if}
{/snippet}

<Sheet title="Manage Servers" size="md" {onClose} closeAnchorId="ob_manage_done">
  <div class="manage">
    <p class="count">
      {servers.length} server{servers.length === 1 ? '' : 's'} configured
    </p>
    {#if notice}<p class="notice" role="status">{notice}</p>{/if}

    {#if multiHost}
      {#each hosts as host (host.id)}
        {@const summary = hostSummaries.get(host.id)}
        <div class="host-group">
          <div class="host-header">
            <StatusDot
              tone={summary?.connection === 'connected' ? 'ok' : 'warn'}
              label={host.label}
            />
            <span class="host-count">{summary?.serverCount ?? 0} servers</span>
            {#if host.id !== activeHostId}
              <Button variant="secondary" size="sm" onclick={() => onSwitchHost(host.id)}
                >Switch</Button
              >
            {/if}
            {#if host.id !== 'local-agent'}
              <Button
                variant="ghost-icon"
                size="sm"
                label={`Remove ${host.label}`}
                onclick={() => onRemoveHost(host.id)}
              >
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true">
                  <path
                    d="M6 6l12 12M18 6L6 18"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                  />
                </svg>
              </Button>
            {/if}
          </div>
          {#if host.id === activeHostId}
            <div class="server-list">
              {#each servers as server (server.id)}{@render serverCard(server)}{/each}
            </div>
          {:else}
            <p class="host-inactive">Switch to this host to see its servers.</p>
          {/if}
        </div>
      {/each}
    {:else if servers.length === 0}
      <EmptyState
        title="No servers yet"
        message="Create a new server or import an existing folder."
      />
    {:else}
      <div class="server-list">
        {#each servers as server (server.id)}{@render serverCard(server)}{/each}
      </div>
    {/if}

    <div class="footer">
      <Button variant="secondary" onclick={() => (showImport = !showImport)} disabled={!canControl}
        >Import…</Button
      >
      {#if isDesktopShell}
        <Button variant="secondary" onclick={() => (showAddHost = !showAddHost)}>Add Host…</Button>
      {/if}
      <Button
        variant="primary"
        onclick={() => (showWizard = true)}
        disabled={!canControl}
        anchorId="ob_create_server">Add Server…</Button
      >
    </div>

    {#if showImport}
      <Card>
        <div class="inline-form">
          <Field bind:value={importPath} placeholder="/path/to/existing/server" />
          <Button variant="primary" onclick={importServer} disabled={!importPath.trim()}
            >Import</Button
          >
        </div>
      </Card>
    {/if}

    {#if showAddHost}
      <Card>
        <div class="add-host-form">
          <Field bind:value={addHostLabel} placeholder="Label, e.g. Garage Mini PC" />
          <Field bind:value={addHostUrl} placeholder="https://host-address:port" />
          <Field bind:value={addHostCode} placeholder="Pairing code from that host" />
          {#if addHostError}<p class="error">{addHostError}</p>{/if}
          <Button variant="primary" onclick={submitAddHost} disabled={addHostBusy}
            >Redeem pairing code</Button
          >
        </div>
      </Card>
    {/if}
  </div>
</Sheet>

{#if editingServer}
  <ServerEditorSheet
    {api}
    server={editingServer}
    {canControl}
    onClose={() => (editingServer = undefined)}
    onServersChanged={refreshServers}
    onSetActive={setActive}
  />
{/if}

{#if showWizard}
  <AddServerWizard {api} onClose={() => (showWizard = false)} onCreated={refreshServers} />
{/if}

<style>
  .manage {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .count {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .notice {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .server-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .server-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 11px 14px;
  }
  .server-row.active {
    box-shadow: inset 3px 0 0 var(--msc2-status-ok);
  }
  .server-icon {
    width: 32px;
    height: 32px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 8px;
    background: var(--msc2-neutral-elevated);
    color: rgba(255, 255, 255, 0.6);
  }
  .server-icon.active {
    color: var(--msc2-status-ok);
  }
  .server-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .server-title {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .server-name {
    font-size: 13px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .server-dir {
    margin: 0;
    font-size: 11px;
    font-family: var(--msc2-font-mono, monospace);
    color: var(--msc2-text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .confirm-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 14px 14px;
    border-top: 1px solid var(--msc2-hairline-subtle);
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .confirm-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }
  .host-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .host-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 2px 2px;
  }
  .host-count {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
    flex: 1;
  }
  .host-inactive {
    margin: 0 0 4px;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .inline-form,
  .add-host-form {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .add-host-form {
    flex-direction: column;
    align-items: stretch;
  }
  .error {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-status-error);
  }
</style>
