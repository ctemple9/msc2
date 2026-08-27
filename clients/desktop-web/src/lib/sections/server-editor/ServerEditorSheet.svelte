<script lang="ts">
  // Ports ServerEditorView.swift, reduced to two tabs per the 2026-08-27
  // design decision recorded in docs/msc2/rolling-plan.md's P12.12 entry:
  // Settings/JARs/Backups/World all already live in Details (P12.7/P12.8/
  // P12.4/P12.4k) and are not rebuilt here; Docker is excluded while D-008
  // stays Proposed. Reached from ManageSheet.svelte's per-card "Edit..."
  // action.
  import Sheet from '../../components/base/Sheet.svelte';
  import SegmentedControl from '../../components/base/SegmentedControl.svelte';
  import GeneralTab from './GeneralTab.svelte';
  import BroadcastTab from './BroadcastTab.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { call } from '../shared/types';
  import { serverEditorPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let server: Schema['ServerDTO'];
  export let canControl = true;
  export let onClose: () => void;
  export let onServersChanged: () => Promise<void>;
  export let onSetActive: (serverId: string) => Promise<void>;

  let currentServer = server;
  let tab: 'general' | 'broadcast' = 'general';
  let activeServerId: string | undefined;
  let switching = false;

  $: isActive = activeServerId === currentServer.id;

  async function refreshActive(): Promise<void> {
    const status = await call<Schema['RemoteAPIStatus']>(
      api,
      { running: false },
      serverEditorPaths.status,
    );
    activeServerId = status.activeServerId;
  }

  void refreshActive();

  async function requestActivate(): Promise<void> {
    if (switching) return;
    switching = true;
    try {
      await onSetActive(currentServer.id);
      await refreshActive();
    } finally {
      switching = false;
    }
  }

  function handleRenamed(name: string): void {
    currentServer = { ...currentServer, name };
    void onServersChanged();
  }

  function handleDeleted(): void {
    void onServersChanged();
    onClose();
  }
</script>

<Sheet title={`Edit ${currentServer.name || '(no name)'}`} size="lg" {onClose}>
  <div class="editor">
    <SegmentedControl
      options={[
        { value: 'general', label: 'General' },
        { value: 'broadcast', label: 'Broadcast' },
      ]}
      value={tab}
      onchange={(value) => (tab = value as 'general' | 'broadcast')}
    />

    {#if tab === 'general'}
      <GeneralTab
        {api}
        server={currentServer}
        {isActive}
        {canControl}
        onRenamed={handleRenamed}
        onDeleted={handleDeleted}
        onRequestActivate={requestActivate}
      />
    {:else}
      <BroadcastTab
        {api}
        server={currentServer}
        {isActive}
        {canControl}
        onRequestActivate={requestActivate}
      />
    {/if}
  </div>
</Sheet>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
</style>
