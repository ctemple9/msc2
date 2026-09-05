<script lang="ts">
  // Ports ServerEditorGeneralTab.swift's Identity/Memory/EULA/Danger Zone
  // blocks. Automation (auto-restart-on-crash), Notes, the four per-server
  // notification toggles, and the headless-script generator are left out --
  // real oracle fields (crates/msc-domain/src/app_config_schema.rs already
  // carries `notes`, `auto_restart_on_crash`, `notification_prefs`) but with
  // no HTTP route anywhere in docs/msc2/api-contract/openapi.json to read or
  // write them, unlike the identity/RAM/EULA/delete fields below which are
  // all real, wired routes. Not silently dropped -- see this step's rolling
  // plan entry.
  //
  // "Server Directory" follows MSC 1's Browse... behavior: it repoints the
  // config record and never moves files. The native picker is supplied by the
  // shared platform adapter, with the browser adapter's manual-path prompt as
  // its fallback.
  //
  // Memory (RAM) is a real route (`/v1/config/ram`) but, like every route
  // this tab touches besides rename/eula/delete, it has no serverId
  // parameter -- crates/msc-agent/src/routes/versions.rs's
  // get_ram_config/set_ram_config always act on whichever server the agent
  // currently considers active. Editing a card that isn't the active one
  // would silently read/write the WRONG server's RAM, so this block is
  // gated on `isActive` and offers the same "Set as Active" action
  // ManageSheet's row menu already exposes instead.
  import { onDestroy } from 'svelte';
  import Card from '../../components/base/Card.svelte';
  import Button from '../../components/base/Button.svelte';
  import Field from '../../components/base/Field.svelte';
  import NumberField from '../../components/base/NumberField.svelte';
  import StatusDot from '../../components/base/StatusDot.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { bytesLabel, call, errorMessage, mutate } from '../shared/types';
  import { getPlatform } from '../../platform';
  import { serverEditorPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let server: Schema['ServerDTO'];
  export let isActive = false;
  export let canControl = true;
  export let onRenamed: (name: string) => void;
  export let onDirectoryChanged: (directory: string) => void;
  export let onDeleted: () => void;
  export let onRequestActivate: () => void;
  export let onPortsChanged: () => Promise<void>;

  let nameDraft = server.name;
  let renaming = false;
  let directoryPicking = false;
  let notice = '';

  let ram: Schema['RAMConfigResponseDTO'] | undefined;
  let minRamDraft = '';
  let maxRamDraft = '';
  let ramSaving = false;
  let loadedForActive = false;
  let ramSaveTimer: ReturnType<typeof setTimeout> | undefined;
  let directorySize: number | undefined;
  let directorySizeLoading = false;
  let loadedDirectorySizeKey = '';
  let gamePortDraft = String(server.gamePort ?? (server.serverType === 'bedrock' ? 19132 : 25565));
  let bedrockPortDraft = server.bedrockPort === undefined ? '' : String(server.bedrockPort);
  let portSaving = false;
  let portSaveTimer: ReturnType<typeof setTimeout> | undefined;

  let eulaAccepted: boolean | undefined;
  let eulaBusy = false;
  let eulaLoading = false;
  let loadedEulaFor = '';

  let confirmingDelete = false;
  let deleting = false;

  $: nameDirty = nameDraft.trim() !== server.name && nameDraft.trim().length > 0;
  $: ramDirty =
    !!ram && (minRamDraft !== String(ram.minRamGB) || maxRamDraft !== String(ram.maxRamGB));
  $: isJava = server.serverType !== 'bedrock';
  $: directorySizeKey = `${server.id}:${server.directory}`;
  $: hasBedrockPort = isJava && server.bedrockPort !== undefined;

  $: if (isActive && !loadedForActive) {
    loadedForActive = true;
    void loadRam();
  }
  $: if (!isActive) {
    loadedForActive = false;
    ram = undefined;
  }
  $: if (directorySizeKey !== loadedDirectorySizeKey) {
    loadedDirectorySizeKey = directorySizeKey;
    void loadDirectorySize(directorySizeKey);
  }
  $: if (server.id !== loadedEulaFor) {
    loadedEulaFor = server.id;
    eulaAccepted = undefined;
    void loadEula(server.id);
  }

  async function loadRam(): Promise<void> {
    ram = await call(api, ram, serverEditorPaths.ram);
    if (ram) {
      minRamDraft = String(ram.minRamGB);
      maxRamDraft = String(ram.maxRamGB);
    }
  }

  async function loadEula(serverId: string): Promise<void> {
    eulaLoading = true;
    try {
      const result = await call<Schema['ServerEULAResultDTO']>(
        api,
        { success: false, message: 'EULA status unavailable.' },
        serverEditorPaths.eulaStatus(serverId),
      );
      eulaAccepted = result.accepted;
    } finally {
      eulaLoading = false;
    }
  }

  async function loadDirectorySize(requestKey: string): Promise<void> {
    directorySizeLoading = true;
    directorySize = undefined;
    const result = await call<Schema['ServerDirectorySizeResponseDTO']>(
      api,
      { serverId: server.id },
      serverEditorPaths.directorySize(server.id),
    );
    if (directorySizeKey !== requestKey) return;
    directorySize = result.sizeBytes;
    directorySizeLoading = false;
  }

  async function saveName(): Promise<void> {
    const name = nameDraft.trim();
    if (!name || renaming) return;
    renaming = true;
    try {
      const result = await mutate<Schema['ServerRenameResultDTO']>(api, serverEditorPaths.rename, {
        serverId: server.id,
        name,
      });
      notice = result.message;
      onRenamed(name);
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      renaming = false;
    }
  }

  async function browseForDirectory(): Promise<void> {
    if (directoryPicking || !canControl) return;
    directoryPicking = true;
    try {
      const selected = await (await getPlatform()).pickFolder('Choose server directory');
      const directory = selected?.trim();
      if (!directory) return;
      const result = await mutate<Schema['ServerDirectoryResultDTO']>(
        api,
        serverEditorPaths.directory,
        { serverId: server.id, directory },
      );
      notice = result.message;
      onDirectoryChanged(result.directory ?? directory);
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      directoryPicking = false;
    }
  }

  function parseRamDraft(value: string): number | undefined {
    const trimmed = value.trim();
    if (!trimmed) return undefined;
    const parsed = Number(trimmed);
    return Number.isFinite(parsed) && parsed >= 0 ? parsed : undefined;
  }

  function parsePortDraft(value: string): number | undefined {
    const parsed = Number(value.trim());
    return Number.isInteger(parsed) && parsed >= 1 && parsed <= 65535 ? parsed : undefined;
  }

  function scheduleRamSave(): void {
    if (!canControl) return;
    if (ramSaveTimer) clearTimeout(ramSaveTimer);
    ramSaveTimer = setTimeout(() => {
      ramSaveTimer = undefined;
      void saveRam();
    }, 450);
  }

  function handleMinRamChange(value: string): void {
    minRamDraft = value;
    scheduleRamSave();
  }

  function handleMaxRamChange(value: string): void {
    maxRamDraft = value;
    scheduleRamSave();
  }

  async function saveRam(): Promise<void> {
    if (!canControl || !ramDirty || ramSaving) return;
    const minRamGB = parseRamDraft(minRamDraft);
    const maxRamGB = parseRamDraft(maxRamDraft);
    if (minRamGB === undefined || maxRamGB === undefined) {
      notice = 'Enter a valid minimum and maximum RAM value.';
      return;
    }
    ramSaving = true;
    try {
      const result = await mutate<Schema['RAMConfigUpdateResultDTO']>(
        api,
        serverEditorPaths.ram,
        // Send the complete pair so an automatic save never reaches the
        // agent as an empty partial update when both fields were edited.
        { minRamGB, maxRamGB },
      );
      notice = result.restartRequired
        ? `${result.message ?? 'RAM allocation saved.'} Restart the server to apply.`
        : (result.message ?? 'RAM allocation saved.');
      await loadRam();
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      ramSaving = false;
    }
  }

  function schedulePortSave(): void {
    if (!canControl || !isActive) return;
    if (portSaveTimer) clearTimeout(portSaveTimer);
    portSaveTimer = setTimeout(() => {
      portSaveTimer = undefined;
      void savePorts();
    }, 450);
  }

  function handleGamePortChange(value: string): void {
    gamePortDraft = value;
    schedulePortSave();
  }

  function handleBedrockPortChange(value: string): void {
    bedrockPortDraft = value;
    schedulePortSave();
  }

  async function savePorts(): Promise<void> {
    if (!canControl || !isActive || portSaving) return;
    const gamePort = parsePortDraft(gamePortDraft);
    const bedrockPort = hasBedrockPort ? parsePortDraft(bedrockPortDraft) : undefined;
    if (gamePort === undefined || (hasBedrockPort && bedrockPort === undefined)) {
      notice = 'Enter ports between 1 and 65535.';
      return;
    }
    portSaving = true;
    try {
      const result = await mutate<Schema['SettingsUpdateResultDTO']>(
        api,
        serverEditorPaths.settings,
        { changes: { 'server-port': String(gamePort) } },
      );
      let restartRequired = result.restartRequired;
      if (bedrockPort !== undefined) {
        await mutate<Schema['GeyserConfigUpdateResultDTO']>(api, serverEditorPaths.geyser, {
          port: bedrockPort,
        });
        restartRequired = true;
      }
      notice = restartRequired ? 'Ports saved. Restart the server to apply.' : 'Ports saved.';
      await onPortsChanged();
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      portSaving = false;
    }
  }

  onDestroy(() => {
    if (ramSaveTimer) clearTimeout(ramSaveTimer);
    if (portSaveTimer) clearTimeout(portSaveTimer);
  });

  async function acceptEula(): Promise<void> {
    if (eulaBusy) return;
    eulaBusy = true;
    try {
      const result = await mutate<Schema['ServerEULAResultDTO']>(api, serverEditorPaths.eula, {
        serverId: server.id,
      });
      eulaAccepted = result.accepted ?? result.success;
      notice = result.message;
    } catch (error) {
      notice = errorMessage(error);
    } finally {
      eulaBusy = false;
    }
  }

  async function confirmDelete(): Promise<void> {
    deleting = true;
    try {
      const result = await mutate<Schema['ServerDeleteResultDTO']>(api, serverEditorPaths.delete, {
        serverId: server.id,
      });
      notice = result.message;
      onDeleted();
    } catch (error) {
      notice = errorMessage(error);
      deleting = false;
      confirmingDelete = false;
    }
  }
</script>

<div class="tab">
  {#if notice}<p class="notice" role="status">{notice}</p>{/if}

  <section class="zone">
    <p class="msc2-type-overline">Identity</p>
    <Card padding="0">
      <div class="row">
        <span class="name">Display Name</span>
        <div class="control">
          <Field bind:value={nameDraft} width="220px" />
          <Button
            variant="secondary"
            size="sm"
            disabled={!nameDirty || renaming || !canControl}
            onclick={saveName}>{renaming ? 'Saving…' : 'Save'}</Button
          >
        </div>
      </div>
      <div class="row bordered">
        <span class="name">Server Directory</span>
        <div class="directory-control">
          <span class="dir">{server.directory}</span>
          <Button
            variant="secondary"
            size="sm"
            disabled={directoryPicking || !canControl}
            onclick={browseForDirectory}>{directoryPicking ? 'Opening…' : 'Browse…'}</Button
          >
        </div>
      </div>
    </Card>
    <p class="hint">
      Choose the folder where this server's files live. Changing it updates the configured path; it
      does not move files on disk.
    </p>
  </section>

  <section class="zone">
    <p class="msc2-type-overline">Memory</p>
    {#if !isActive}
      <Card>
        <div class="notice-row">
          <div class="notice-text">
            <span class="name">Set as active to edit memory</span>
            <p class="hint">
              RAM allocation is only editable for the currently active server, since the agent
              doesn't expose a way to target another one directly.
            </p>
          </div>
          <Button variant="secondary" size="sm" onclick={onRequestActivate}>Set as Active</Button>
        </div>
      </Card>
    {:else if !ram}
      <p class="hint">Loading memory allocation…</p>
    {:else}
      <Card padding="0">
        <div class="row">
          <span class="name">Minimum RAM</span>
          <div class="control">
            <NumberField
              value={minRamDraft}
              min={0}
              step={0.1}
              width="80px"
              disabled={!canControl || ramSaving}
              onValueChange={handleMinRamChange}
            />
            <span class="unit">GB</span>
          </div>
        </div>
        <div class="row bordered">
          <span class="name">Maximum RAM</span>
          <div class="control">
            <NumberField
              value={maxRamDraft}
              min={0}
              step={0.1}
              width="80px"
              disabled={!canControl || ramSaving}
              onValueChange={handleMaxRamChange}
            />
            <span class="unit">GB</span>
          </div>
        </div>
      </Card>
      {#if ramSaving}<p class="hint memory-status">Saving…</p>{/if}
    {/if}
  </section>

  <section class="zone">
    <p class="msc2-type-overline">Ports</p>
    {#if !isActive}
      <Card>
        <div class="notice-row">
          <div class="notice-text">
            <span class="name">Set as active to edit ports</span>
            <p class="hint">
              Port changes apply to the currently active server and take effect after its next
              restart.
            </p>
          </div>
          <Button variant="secondary" size="sm" onclick={onRequestActivate}>Set as Active</Button>
        </div>
      </Card>
    {:else}
      <Card padding="0">
        <div class="row">
          <span class="name">{isJava ? 'Java Port' : 'Bedrock Port'}</span>
          <div class="control">
            <NumberField
              value={gamePortDraft}
              min={1}
              max={65535}
              step={1}
              width="90px"
              disabled={!canControl || portSaving}
              onValueChange={handleGamePortChange}
            />
          </div>
        </div>
        {#if hasBedrockPort}
          <div class="row bordered">
            <span class="name">Bedrock / Geyser Port</span>
            <div class="control">
              <NumberField
                value={bedrockPortDraft}
                min={1}
                max={65535}
                step={1}
                width="90px"
                disabled={!canControl || portSaving}
                onValueChange={handleBedrockPortChange}
              />
            </div>
          </div>
        {/if}
      </Card>
      <p class="hint">
        Changes the local server port only. Router forwarding and Playit mappings are separate.
      </p>
      {#if portSaving}<p class="hint memory-status">Saving…</p>{/if}
    {/if}
  </section>

  <section class="zone">
    <p class="msc2-type-overline">Storage</p>
    <Card padding="0">
      <div class="row">
        <span class="name">Server Folder Size</span>
        <span class="storage-value"
          >{directorySizeLoading ? 'Loading…' : bytesLabel(directorySize)}</span
        >
      </div>
    </Card>
  </section>

  {#if isJava}
    <section class="zone">
      <p class="msc2-type-overline">EULA</p>
      <Card padding="0">
        <div class="row">
          <div class="eula-info">
            <StatusDot
              tone={eulaAccepted ? 'ok' : 'warn'}
              label={eulaLoading
                ? 'Checking…'
                : eulaAccepted
                  ? 'Accepted'
                  : 'Not confirmed here yet'}
              showDot={false}
            />
            <span class="hint">Minecraft End User License Agreement</span>
          </div>
          <Button
            variant="primary"
            size="sm"
            disabled={eulaBusy || eulaLoading || !canControl}
            onclick={acceptEula}
            anchorId="ob_accept_eula">{eulaAccepted ? 'Accepted' : 'Accept EULA'}</Button
          >
        </div>
      </Card>
    </section>
  {/if}

  {#if canControl}
    <section class="zone danger">
      <p class="msc2-type-overline">Danger Zone</p>
      {#if !confirmingDelete}
        <div class="danger-row">
          <span class="hint"
            >Permanently delete this server's folder from disk and remove it from the controller.</span
          >
          <Button variant="destructive" size="sm" onclick={() => (confirmingDelete = true)}
            >Delete Server…</Button
          >
        </div>
      {:else}
        <div class="danger-row">
          <span class="hint"
            >Delete "{server.name}" and remove it from the controller? The agent refuses this while
            running.</span
          >
          <div class="danger-actions">
            <Button variant="secondary" size="sm" onclick={() => (confirmingDelete = false)}
              >Cancel</Button
            >
            <Button variant="destructive" size="sm" disabled={deleting} onclick={confirmDelete}
              >{deleting ? 'Deleting…' : 'Delete Server'}</Button
            >
          </div>
        </div>
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
  .dir {
    font-size: 12px;
    font-family: var(--msc2-font-mono, monospace);
    color: var(--msc2-text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 320px;
  }
  .directory-control {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .control {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .unit {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .storage-value {
    font-size: 12px;
    font-family: var(--msc2-font-mono, monospace);
    color: var(--msc2-text-secondary);
  }
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .memory-status {
    min-height: 16px;
  }
  .eula-info {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .danger .danger-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .danger-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }
</style>
