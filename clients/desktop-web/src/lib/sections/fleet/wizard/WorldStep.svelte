<script lang="ts">
  // Real port of AddServerWizardView.swift's step4FreshWorld -- the source
  // picker stays here, while world-local settings use the same form as the
  // Worlds tab. The current create contract carries the Essentials
  // projection; the shared form marks the rest unavailable until the server
  // exists instead of collecting values that would be silently lost.
  //
  // Real gap found and handled, not silently worked around: the oracle
  // offers a third World Source, an existing world *folder*, alongside New
  // World and a backup ZIP. `ServerCreateRequestDTO` has no field to carry a
  // pre-made world into creation at all (confirmed against the frozen
  // contract -- worldName/worldSeed are the only world-shape fields it
  // takes), so both non-fresh sources need the same staged-upload mechanism
  // `worlds/ImportWorldZipSheet.svelte` already uses for a *single file's*
  // bytes. A folder has no such primitive anywhere in this codebase or the
  // contract -- `worlds/ReplaceWorldSheet.svelte` already hit this exact
  // gap for the identical oracle picker ("World Folder…") and dropped it
  // for the same reason: "a browser file picker has no folder-to-archive
  // equivalent." This step follows that same precedent rather than
  // re-litigating it: the World Source picker below offers only New World
  // and From Backup (.zip); `WizardDraft.worldSourceMode` has no `folder`
  // value to select in the first place, so there is nothing to fake a
  // picker control for.
  //
  // The backup ZIP is staged immediately on pick (same as
  // ImportWorldZipSheet's chooseAndStage) rather than deferred, since
  // staging bytes commits nothing -- only P12.18g's real create call
  // redeems the resulting stagedUploadId.
  import Button from '../../../components/base/Button.svelte';
  import SegmentedControl from '../../../components/base/SegmentedControl.svelte';
  import { onboardingAnchor } from '../../../help/tourAnchors';
  import { getPlatform } from '../../../platform';
  import type { PickedFile } from '../../../platform/types';
  import type { ScreenApi } from '../../shared/types';
  import { errorMessage } from '../../shared/types';
  import WorldSettingsForm from '../../worlds/WorldSettingsForm.svelte';
  import { defaultWorldSettingsValues, type WorldSettingsValues } from '../../worlds/model';
  import { type WizardDraft, type WorldSourceMode } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let draft: WizardDraft;

  let fileInput: HTMLInputElement;
  let staging = false;
  let stageError: string | undefined;
  let worldSettings: WorldSettingsValues = {
    ...defaultWorldSettingsValues(draft.serverType),
    name: draft.worldName,
    seed: draft.worldSeed,
    difficulty: draft.worldDifficulty,
    defaultGameMode: draft.worldGamemode,
  };

  function selectSourceMode(mode: string): void {
    draft.worldSourceMode = mode as WorldSourceMode;
    stageError = undefined;
  }

  function browseBrowserFile(): Promise<PickedFile | null> {
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

  async function chooseBackup(): Promise<void> {
    if (!api?.upload || staging) return;
    staging = true;
    stageError = undefined;
    try {
      const picked = await (
        await getPlatform()
      ).pickFile({ label: 'Choose a world backup ZIP', extensions: ['zip'] }, () =>
        browseBrowserFile(),
      );
      if (!picked) return;
      const staged = await api.upload('world-import', picked.bytes);
      draft.stagedWorldBackup = { fileName: picked.name, stagedUploadId: staged.stagedUploadId };
    } catch (error) {
      stageError = errorMessage(error);
    } finally {
      staging = false;
    }
  }

  function updateWorldSettings(next: WorldSettingsValues): void {
    worldSettings = next;
    draft.worldName = next.name;
    draft.worldSeed = next.seed;
    draft.worldDifficulty = next.difficulty as WizardDraft['worldDifficulty'];
    draft.worldGamemode = next.defaultGameMode as WizardDraft['worldGamemode'];
  }
</script>

<div class="world">
  <input bind:this={fileInput} type="file" accept=".zip" class="hidden-input" />

  <div class="intro">
    <h2>What should the first world be?</h2>
    <p>Start with a brand new world, or bring in one from a backup.</p>
  </div>

  <section class="block" use:onboardingAnchor={'ob_world_source'}>
    <p class="msc2-type-overline">World Source</p>
    <SegmentedControl
      options={[
        { value: 'fresh', label: 'New World' },
        { value: 'backupZip', label: 'From Backup (.zip)' },
      ]}
      value={draft.worldSourceMode}
      onchange={selectSourceMode}
    />
  </section>

  {#if draft.worldSourceMode === 'fresh'}
    <div use:onboardingAnchor={'ob_world_creation'}>
      <WorldSettingsForm
        mode="wizard"
        heading="First world settings"
        serverType={draft.serverType}
        values={worldSettings}
        onChange={updateWorldSettings}
      />
    </div>
  {:else}
    <section class="block">
      <Button variant="secondary" disabled={staging} onclick={() => void chooseBackup()}>
        {staging ? 'Staging…' : 'Choose backup .zip…'}
      </Button>
      {#if draft.stagedWorldBackup}
        <p class="hint">Selected: {draft.stagedWorldBackup.fileName}</p>
      {:else if stageError}
        <p class="hint warn">{stageError}</p>
      {:else}
        <p class="hint">No file selected.</p>
      {/if}
    </section>
  {/if}
</div>

<style>
  .world {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .intro {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .intro h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .intro p {
    margin: 0;
    font-size: 12.5px;
    color: var(--msc2-text-tertiary);
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
  }
  .hint {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
  .hint.warn {
    color: var(--msc2-status-warn);
  }
  .hidden-input {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    overflow: hidden;
  }
</style>
