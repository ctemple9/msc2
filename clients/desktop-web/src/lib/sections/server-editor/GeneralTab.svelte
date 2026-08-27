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
  // "Server Directory" is likewise real (`ConfigServer.server_dir`) but has
  // no update route at all -- MSC 1's Browse... only ever repoints the config
  // record at an already-existing folder (no filesystem move happens there
  // either), but nothing here can send that repoint anywhere, so it renders
  // read-only.
  //
  // Memory (RAM) is a real route (`/v1/config/ram`) but, like every route
  // this tab touches besides rename/eula/delete, it has no serverId
  // parameter -- crates/msc-agent/src/routes/versions.rs's
  // get_ram_config/set_ram_config always act on whichever server the agent
  // currently considers active. Editing a card that isn't the active one
  // would silently read/write the WRONG server's RAM, so this block is
  // gated on `isActive` and offers the same "Set as Active" action
  // ManageSheet's row menu already exposes instead.
  import Card from '../../components/base/Card.svelte';
  import Button from '../../components/base/Button.svelte';
  import Field from '../../components/base/Field.svelte';
  import NumberField from '../../components/base/NumberField.svelte';
  import StatusDot from '../../components/base/StatusDot.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { call, errorMessage, mutate } from '../shared/types';
  import { serverEditorPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let server: Schema['ServerDTO'];
  export let isActive = false;
  export let canControl = true;
  export let onRenamed: (name: string) => void;
  export let onDeleted: () => void;
  export let onRequestActivate: () => void;

  let nameDraft = server.name;
  let renaming = false;
  let notice = '';

  let ram: Schema['RAMConfigResponseDTO'] | undefined;
  let minRamDraft = '';
  let maxRamDraft = '';
  let ramSaving = false;
  let loadedForActive = false;

  let eulaAccepted: boolean | undefined;
  let eulaBusy = false;

  let confirmingDelete = false;
  let deleting = false;

  $: nameDirty = nameDraft.trim() !== server.name && nameDraft.trim().length > 0;
  $: ramDirty =
    !!ram && (minRamDraft !== String(ram.minRamGB) || maxRamDraft !== String(ram.maxRamGB));
  $: isJava = server.serverType !== 'bedrock';

  $: if (isActive && !loadedForActive) {
    loadedForActive = true;
    void loadRam();
  }
  $: if (!isActive) {
    loadedForActive = false;
    ram = undefined;
  }

  async function loadRam(): Promise<void> {
    ram = await call(api, ram, serverEditorPaths.ram);
    if (ram) {
      minRamDraft = String(ram.minRamGB);
      maxRamDraft = String(ram.maxRamGB);
    }
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

  async function saveRam(): Promise<void> {
    if (!ramDirty || ramSaving) return;
    ramSaving = true;
    try {
      const changes: Record<string, number> = {};
      if (ram && minRamDraft !== String(ram.minRamGB)) changes.minRamGB = Number(minRamDraft);
      if (ram && maxRamDraft !== String(ram.maxRamGB)) changes.maxRamGB = Number(maxRamDraft);
      const result = await mutate<Schema['RAMConfigUpdateResultDTO']>(
        api,
        serverEditorPaths.ram,
        changes,
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
        <span class="dir">{server.directory}</span>
      </div>
    </Card>
    <p class="hint">
      Server Directory is set when the server is created; changing it isn't available here yet.
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
            <NumberField bind:value={minRamDraft} min={0} step={0.5} width="80px" />
            <span class="unit">GB</span>
          </div>
        </div>
        <div class="row bordered">
          <span class="name">Maximum RAM</span>
          <div class="control">
            <NumberField bind:value={maxRamDraft} min={0} step={0.5} width="80px" />
            <span class="unit">GB</span>
          </div>
        </div>
      </Card>
      <div class="memory-footer">
        <span class="hint"
          >{ram.physicalRAMGB} GB physical · {ram.recommendedMaxGB} GB recommended max</span
        >
        <Button
          variant="secondary"
          size="sm"
          disabled={!ramDirty || ramSaving || !canControl}
          onclick={saveRam}>{ramSaving ? 'Saving…' : 'Save'}</Button
        >
      </div>
    {/if}
  </section>

  {#if isJava}
    <section class="zone">
      <p class="msc2-type-overline">EULA</p>
      <Card padding="0">
        <div class="row">
          <div class="eula-info">
            <StatusDot
              tone={eulaAccepted ? 'ok' : 'warn'}
              label={eulaAccepted ? 'Accepted' : 'Not confirmed here yet'}
            />
            <span class="hint">Minecraft End User License Agreement</span>
          </div>
          <Button
            variant="primary"
            size="sm"
            disabled={eulaBusy || !canControl}
            onclick={acceptEula}>{eulaAccepted ? 'Accepted' : 'Accept EULA'}</Button
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
  .control {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .unit {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .memory-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
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
