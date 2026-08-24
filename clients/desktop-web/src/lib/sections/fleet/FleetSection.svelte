<script lang="ts">
  import { onMount } from 'svelte';
  import ActionButton from '../../components/ActionButton.svelte';
  import ConfirmDialog from '../../components/ConfirmDialog.svelte';
  import StatusBadge from '../../components/StatusBadge.svelte';
  import CapabilityNotice from '../shared/CapabilityNotice.svelte';
  import OperationQueue from '../shared/OperationQueue.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, errorMessage, mutate } from '../shared/types';
  import { demoServers, demoStatus, fleetMutationPaths, selectedServer } from './model';

  export let api: ScreenProps['api'] = undefined;
  export let hostId = 'selected host';
  export let permissions: readonly string[] = [];
  export let onServerSelected: ((id: string) => void) | undefined = undefined;

  let servers = demoServers;
  let status = demoStatus;
  let runtimes: Schema['JavaRuntimeDTO'][] = [];
  let versions: Schema['VersionsResponseDTO'] = {
    flavorName: 'Paper',
    isBedrock: false,
    supportsVersions: true,
    versions: [],
  };
  let templates: Schema['TemplatesResponseDTO'] = {
    paperTemplates: [],
    pluginTemplates: [],
    serverRunning: false,
  };
  let name = '';
  let importPath = '';
  let rename = '';
  let selectedRuntime = 21;
  let selectedVersion = '';
  let notice = '';
  let pendingDelete: string | null = null;

  const canControl =
    permissions.length === 0 ||
    permissions.includes('serverControl') ||
    permissions.includes('admin');
  const active = () => selectedServer(servers, status.activeServerId);

  onMount(async () => {
    if (!api) return;
    servers = await call(api, servers, '/v1/servers');
    status = await call(api, status, '/v1/status');
    const runtimeResult = await call<Schema['JavaRuntimesResponseDTO']>(
      api,
      { runtimes: [] },
      '/v1/java-runtimes',
    );
    runtimes = runtimeResult.runtimes;
    versions = await call(api, versions, '/v1/versions');
    templates = await call(api, templates, fleetMutationPaths.templates);
    selectedVersion = versions.versions[0]?.id ?? '';
  });

  async function choose(id: string): Promise<void> {
    try {
      await mutate(api, fleetMutationPaths.active, { serverId: id });
      status = { ...status, activeServerId: id };
      onServerSelected?.(id);
      notice = `Selected ${servers.find((server) => server.id === id)?.name ?? id}.`;
    } catch (error) {
      notice = errorMessage(error);
    }
  }

  async function lifecycle(path: string): Promise<void> {
    try {
      notice = (await mutate<Schema['SimpleResult']>(api, path)).result;
    } catch (error) {
      notice = errorMessage(error);
    }
  }

  async function createServer(): Promise<void> {
    if (!name.trim()) return;
    try {
      const result = await mutate<Schema['ServerCreateResultDTO']>(api, fleetMutationPaths.create, {
        name,
        serverType: 'paper',
        versionId: versions.versions[0]?.id,
      });
      notice = result.message;
      name = '';
      if (api) servers = await call(api, servers, '/v1/servers');
    } catch (error) {
      notice = errorMessage(error);
    }
  }

  async function importServer(): Promise<void> {
    if (!importPath.trim()) return;
    try {
      notice = (
        await mutate<Schema['ServerImportResultDTO']>(api, fleetMutationPaths.import, {
          action: 'importExisting',
          sourcePath: importPath,
          acceptEula: false,
        })
      ).message;
    } catch (error) {
      notice = errorMessage(error);
    }
  }

  async function deleteServer(): Promise<void> {
    if (!pendingDelete) return;
    try {
      notice = (
        await mutate<Schema['ServerDeleteResultDTO']>(api, fleetMutationPaths.delete, {
          serverId: pendingDelete,
        })
      ).message;
    } catch (error) {
      notice = errorMessage(error);
    }
    pendingDelete = null;
  }

  async function renameServer(): Promise<void> {
    const current = active();
    if (!current || !rename.trim()) return;
    try {
      notice = (
        await mutate<Schema['ServerRenameResultDTO']>(api, fleetMutationPaths.rename, {
          serverId: current.id,
          name: rename,
        })
      ).message;
    } catch (error) {
      notice = errorMessage(error);
    }
  }

  async function installJava(): Promise<void> {
    try {
      notice = (
        await mutate<Schema['JavaRuntimeInstallResultDTO']>(
          api,
          fleetMutationPaths.installRuntime,
          { major: selectedRuntime },
        )
      ).message;
    } catch (error) {
      notice = errorMessage(error);
    }
  }

  async function acceptEula(): Promise<void> {
    try {
      notice = (
        await mutate<Schema['ServerEULAResultDTO']>(api, fleetMutationPaths.eula, {
          serverId: active()?.id,
        })
      ).message;
    } catch (error) {
      notice = errorMessage(error);
    }
  }

  async function changeVersion(): Promise<void> {
    if (!selectedVersion) return;
    try {
      notice = (
        await mutate<Schema['VersionChangeResultDTO']>(api, '/v1/components/version', {
          versionId: selectedVersion,
        })
      ).message;
    } catch (error) {
      notice = errorMessage(error);
    }
  }
</script>

<div class="screen">
  <ScreenHeader
    eyebrow="Fleet and lifecycle"
    title="Servers"
    description="Switch the active server, provision Java, and keep lifecycle work explicit and recoverable."
    status={status.running ? 'Running' : 'Stopped'}
    statusTone={status.running ? 'positive' : 'neutral'}
  />
  {#if !canControl}<CapabilityNotice
      title="Server controls hidden"
      message="This token can inspect the fleet but cannot start, stop, create, or delete servers."
    />{/if}
  {#if notice}<p class="muted" role="status">{notice}</p>{/if}

  <section class="screen-card">
    <div class="screen-card-header">
      <h3>Registered servers</h3>
      <span class="metric-label">{servers.length} servers</span>
    </div>
    <table class="data-table">
      <thead
        ><tr
          ><th>Name</th><th>Type</th><th>Port</th><th>State</th><th class="actions">Actions</th></tr
        ></thead
      >
      <tbody>
        {#each servers as server (server.id)}
          <tr>
            <td><strong>{server.name}</strong><br /><small>{server.directory}</small></td>
            <td
              >{server.serverType}{#if server.javaFlavor}<br /><small>{server.javaFlavor}</small
                >{/if}</td
            >
            <td>{server.gamePort ?? '—'}</td>
            <td
              >{#if server.id === status.activeServerId}<StatusBadge
                  status={status.running ? 'Active · running' : 'Active · stopped'}
                  tone={status.running ? 'positive' : 'neutral'}
                />{:else}<span class="muted">Available</span>{/if}</td
            >
            <td class="actions"
              ><div class="screen-actions">
                <ActionButton kind="quiet" label="Select server" onclick={() => choose(server.id)}
                  >Select</ActionButton
                ><ActionButton
                  kind="danger"
                  label="Delete server"
                  disabled={!canControl}
                  onclick={() => (pendingDelete = server.id)}>Delete</ActionButton
                >
              </div></td
            >
          </tr>
        {:else}<tr><td class="empty-row" colspan="5">No servers registered on this host.</td></tr
          >{/each}
      </tbody>
    </table>
  </section>

  <div class="screen-grid">
    <section class="screen-card accent">
      <h3>Active lifecycle</h3>
      <p>Actions target <strong>{active()?.name ?? 'no server'}</strong> on the selected host.</p>
      <div class="screen-actions">
        <ActionButton
          label="Start"
          disabled={!canControl || status.running}
          onclick={() => lifecycle(fleetMutationPaths.start)}>Start</ActionButton
        ><ActionButton
          kind="quiet"
          label="Stop"
          disabled={!canControl || !status.running}
          onclick={() => lifecycle(fleetMutationPaths.stop)}>Stop</ActionButton
        ><ActionButton
          kind="quiet"
          label="Restart"
          disabled={!canControl}
          onclick={() =>
            lifecycle(fleetMutationPaths.stop).then(() => lifecycle(fleetMutationPaths.start))}
          >Restart</ActionButton
        ><ActionButton
          kind="quiet"
          label="Accept Minecraft EULA"
          disabled={!canControl}
          onclick={acceptEula}>Accept EULA</ActionButton
        >
      </div>
      <div class="inline-form" style="margin-top: .8rem">
        <div class="field">
          <label for="rename-server">Rename active server</label><input
            id="rename-server"
            bind:value={rename}
            placeholder="New display name"
          />
        </div>
        <ActionButton label="Rename" disabled={!canControl} onclick={renameServer}
          >Rename</ActionButton
        >
      </div>
    </section>
    <section class="screen-card">
      <h3>Create or import</h3>
      <div class="form-grid" style="margin-top: .7rem">
        <div class="field">
          <label for="new-server">New server name</label><input
            id="new-server"
            bind:value={name}
            placeholder="My server"
          />
        </div>
        <div class="field">
          <label for="server-template">Template</label><select id="server-template"
            ><option>Paper · latest stable</option><option>Vanilla</option><option>Fabric</option
            ></select
          >
        </div>
      </div>
      <div class="screen-actions" style="margin-top: .7rem">
        <ActionButton label="Create server" disabled={!canControl} onclick={createServer}
          >Create</ActionButton
        >
      </div>
      <div class="inline-form" style="margin-top: .8rem">
        <div class="field">
          <label for="import-path">Import folder or archive</label><input
            id="import-path"
            bind:value={importPath}
            placeholder="/path/to/server"
          />
        </div>
        <ActionButton
          kind="quiet"
          label="Import server"
          disabled={!canControl}
          onclick={importServer}>Import</ActionButton
        >
      </div>
    </section>
  </div>

  <div class="screen-grid">
    <section class="screen-card">
      <h3>Java runtimes</h3>
      <p>Choose a host-installed runtime or request a bounded agent-side install.</p>
      <div class="inline-form">
        <div class="field">
          <label for="java-runtime">Install Java major</label><select
            id="java-runtime"
            bind:value={selectedRuntime}
            ><option value={8}>Java 8</option><option value={17}>Java 17</option><option value={21}
              >Java 21</option
            ><option value={25}>Java 25</option></select
          >
        </div>
        <ActionButton label="Install" disabled={!canControl} onclick={installJava}
          >Install</ActionButton
        >
      </div>
      {#if runtimes.length}<div class="tag-list" style="margin-top: .7rem">
          {#each runtimes as runtime}<span class="tag"
              >{runtime.name} · {runtime.majorVersion ?? 'unknown'}</span
            >{/each}
        </div>{:else}<p class="muted">No runtime inventory loaded yet.</p>{/if}
    </section>
    <section class="screen-card">
      <h3>Version and templates</h3>
      <p>
        {versions.supportsVersions
          ? `${versions.versions.length} compatible versions advertised.`
          : 'Version changes are unavailable for this server.'}
      </p>
      <div class="inline-form">
        <div class="field">
          <label for="server-version">Version</label><select
            id="server-version"
            bind:value={selectedVersion}
            disabled={!versions.supportsVersions}
            >{#each versions.versions as version}<option value={version.id}
                >{version.displayLabel}</option
              >{/each}</select
          >
        </div>
        <ActionButton
          kind="quiet"
          label="Change version"
          disabled={!canControl || !selectedVersion}
          onclick={changeVersion}>Change</ActionButton
        >
      </div>
      <div class="tag-list" style="margin-top: .7rem">
        {#each [...templates.paperTemplates, ...templates.pluginTemplates].slice(0, 6) as template}<span
            class="tag">{template.displayName}</span
          >{/each}
      </div>
      <p class="field-help">
        EULA acceptance, version changes, and template creation remain separate confirmations.
      </p>
    </section>
  </div>

  <section class="screen-card">
    <div class="screen-card-header">
      <h3>Durable fleet work</h3>
      <span class="metric-label">Reconnect-safe</span>
    </div>
    <OperationQueue operations={[]} />
  </section>

  <ConfirmDialog
    open={pendingDelete !== null}
    title="Delete this server?"
    message="The selected host will remove the server record. The agent refuses this while it is running."
    context={`Host: ${hostId} · Server: ${servers.find((server) => server.id === pendingDelete)?.name ?? pendingDelete ?? 'unknown'}`}
    confirmLabel="Delete server"
    onConfirm={deleteServer}
    onClose={() => (pendingDelete = null)}
  />
</div>
