<script lang="ts">
  // Ports MSCSettingsView.swift's MSC Settings sheet, reduced to what the
  // frozen contract actually backs -- see this step's rolling-plan.md entry
  // for the full accounting. Four MSC 1 cards are gone because MSC 2
  // replaced the thing they managed, not because they were never ported:
  // Remote Access -> named tokens + Tailscale (AccessSection, D-012);
  // Bedrock Runtime toggle -> automatic per-host runtime detection
  // (Phase 10); CurseForge API Key -> D-027's manual-download import, which
  // uses no key; Process Management (orphan scan, relaunch-on-crash) -> the
  // agent is itself the OS-managed persistent service, so MSC 1's "orphaned
  // by a crashed app" problem doesn't exist here. Four more -- Config
  // Recovery, Storage, Archives, Network Ports -- have no route in the
  // contract at all and were never superseded, just never built; Cameron's
  // 2026-08-27 call was to drop Config Recovery and Archives, and keep
  // Storage and Network Ports on the list for a future contract-amendment
  // step. "Testing reset" and "Open App Support Folder" are dropped
  // outright -- the former has no backend and reads as a dev-only escape
  // hatch, the latter has no meaning for a possibly-remote agent host.
  //
  // What's left and real: Appearance (the per-server accent banner --
  // client-local by design, see styles/bannerColor.ts, which already had
  // save/read helpers with zero call sites until this sheet) and Open
  // Server Folder (GET /v1/config/servers-root + the revealInFileManager
  // seam P12.9 built for the Files tab). Two cards isn't enough to justify
  // General/Remote/Data tabs, so this ships as one flat sheet instead.
  import { onMount } from 'svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import Card from '../../components/base/Card.svelte';
  import Button from '../../components/base/Button.svelte';
  import { getPlatform } from '../../platform';
  import { bannerColorFor, setBannerColorFor, clampBannerColor } from '../../styles/bannerColor';
  import type { Schema, ScreenApi } from '../shared/types';
  import { call, errorMessage } from '../shared/types';

  export let api: ScreenApi | undefined = undefined;
  export let hostId: string;
  export let serverId: string | undefined = undefined;
  export let serverLabel: string | undefined = undefined;
  export let onClose: () => void;
  export let onAccentColorSaved: () => void = () => {};

  let colorDraft = serverId ? bannerColorFor(hostId, serverId) : clampBannerColor('');
  let colorNotice = '';

  let serversRootPath = '';
  let revealBusy = false;
  let revealNotice = '';

  $: colorDirty = !!serverId && colorDraft !== bannerColorFor(hostId, serverId);

  onMount(async () => {
    const root = await call<Schema['ServersRootResponseDTO']>(
      api,
      { path: '' },
      '/v1/config/servers-root',
    );
    serversRootPath = root.path;
  });

  function handleColorInput(event: Event): void {
    colorDraft = (event.currentTarget as HTMLInputElement).value;
  }

  function saveColor(): void {
    if (!serverId || !colorDirty) return;
    setBannerColorFor(hostId, serverId, colorDraft);
    colorDraft = bannerColorFor(hostId, serverId);
    colorNotice = 'Accent color saved on this device.';
    onAccentColorSaved();
  }

  async function openServersFolder(): Promise<void> {
    if (revealBusy || !serversRootPath) return;
    revealBusy = true;
    revealNotice = '';
    try {
      await (
        await getPlatform()
      ).revealInFileManager(serversRootPath, async () => {
        revealNotice = 'Open Server Folder needs the desktop app.';
      });
    } catch (error) {
      revealNotice = errorMessage(error);
    } finally {
      revealBusy = false;
    }
  }
</script>

<Sheet title="MSC Settings" size="md" {onClose}>
  <div class="settings">
    <section class="zone">
      <p class="msc2-type-overline">Appearance</p>
      <Card padding="0">
        <div class="row">
          <div class="row-text">
            <span class="name">Accent Color{serverLabel ? ` — ${serverLabel}` : ''}</span>
            <span class="hint">Colors the running-state banner. Saved on this device only.</span>
          </div>
          <div class="control">
            <input
              type="color"
              class="swatch"
              value={clampBannerColor(colorDraft)}
              disabled={!serverId}
              oninput={handleColorInput}
              aria-label="Accent color"
            />
            <Button variant="secondary" size="sm" disabled={!colorDirty} onclick={saveColor}
              >Save</Button
            >
          </div>
        </div>
      </Card>
      {#if colorNotice}<p class="hint">{colorNotice}</p>{/if}
      {#if !serverId}<p class="hint">Select a server to set its accent color.</p>{/if}
    </section>

    <section class="zone">
      <p class="msc2-type-overline">Data &amp; Folders</p>
      <Card padding="0">
        <div class="row">
          <div class="row-text">
            <span class="name">Servers Folder</span>
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
  </div>
</Sheet>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 18px;
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
  .swatch {
    box-sizing: border-box;
    width: 28px;
    height: 28px;
    padding: 0;
    background: none;
    border: 1px solid var(--msc2-hairline-field);
    border-radius: 8px;
    cursor: pointer;
  }
  .swatch::-webkit-color-swatch-wrapper {
    padding: 2px;
  }
  .swatch::-webkit-color-swatch {
    border: none;
    border-radius: 5px;
  }
  .swatch:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .hint.dir {
    overflow: hidden;
    font-family: var(--msc2-font-mono, monospace);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
