<script lang="ts">
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { errorMessage, mutate } from '../shared/types';
  import { pollOperation, serverEditorPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let initialMajor = 25;
  export let canControl = true;
  export let onClose: () => void;
  export let onInstalled: (event: {
    major: number;
    runtimePath?: string;
  }) => void | Promise<void> = () => {};

  const installOptions = [
    {
      major: 25,
      title: 'Java 25',
      minecraftRange: 'Minecraft 26.1 (latest) and newer',
      recommended: true,
    },
    {
      major: 21,
      title: 'Java 21',
      minecraftRange: 'Minecraft 1.20.5 – 1.21.x',
      recommended: false,
    },
    { major: 17, title: 'Java 17', minecraftRange: 'Minecraft 1.17 – 1.20.4', recommended: false },
    { major: 8, title: 'Java 8', minecraftRange: 'Minecraft 1.16.5 and older', recommended: false },
  ] as const;

  function supportedMajor(value: number): number {
    return installOptions.some((option) => option.major === value) ? value : 25;
  }

  let selectedMajor = supportedMajor(initialMajor);
  let installBusy = false;
  let installStatusLine = '';
  let installFailure = '';
  let installedMajor: number | undefined;

  async function startInstall(): Promise<void> {
    if (installBusy) return;
    installBusy = true;
    installFailure = '';
    installStatusLine = 'Starting install…';
    try {
      const result = await mutate<Schema['JavaRuntimeInstallResultDTO']>(
        api,
        serverEditorPaths.javaRuntimeInstall,
        { major: selectedMajor },
      );
      const operation = await pollOperation(api, result.operationId, (tick) => {
        installStatusLine = tick.statusLine ?? installStatusLine;
      });
      if (!operation || operation.state !== 'succeeded') {
        throw new Error(
          operation?.error?.message ?? result.message ?? 'The install did not complete.',
        );
      }
      const operationResult = operation.result as Record<string, unknown> | null | undefined;
      const runtimePath =
        typeof operationResult?.runtimePath === 'string' ? operationResult.runtimePath : undefined;
      await onInstalled({ major: selectedMajor, runtimePath });
      installedMajor = selectedMajor;
      installBusy = false;
    } catch (error) {
      installFailure = errorMessage(error);
      installBusy = false;
    }
  }

  function close(): void {
    if (installBusy || installedMajor !== undefined) return;
    onClose();
  }
</script>

<Sheet title="Install Java" size="sm" onClose={close}>
  {#if installedMajor !== undefined}
    <p class="success-title">Java {installedMajor} installed successfully.</p>
    <p class="explain">
      MSC installed the Adoptium Temurin runtime and selected it for this host. Return to your
      server setup and try again.
    </p>
    <div class="footer">
      <Button variant="primary" onclick={onClose}>Okay</Button>
    </div>
  {:else}
    <p class="explain">
      Pick the version that matches your Minecraft version. MSC downloads and installs it for this
      host.
    </p>
    <div class="list" role="radiogroup" aria-label="Java version to install">
      {#each installOptions as option (option.major)}
        <button
          type="button"
          class="row"
          class:selected={selectedMajor === option.major}
          disabled={installBusy}
          onclick={() => (selectedMajor = option.major)}
        >
          <span class="runtime-info">
            <span class="runtime-heading">
              <span class="runtime-version">{option.title}</span>
              {#if option.recommended}<span class="tag">Recommended</span>{/if}
            </span>
            <span class="runtime-path">{option.minecraftRange}</span>
          </span>
        </button>
      {/each}
    </div>
    {#if installStatusLine && installBusy}<p class="explain">{installStatusLine}</p>{/if}
    {#if installFailure}<p class="explain warn">{installFailure}</p>{/if}
    <div class="footer">
      <Button variant="secondary" disabled={installBusy} onclick={close}>Cancel</Button>
      <Button
        variant="primary"
        disabled={installBusy || !canControl}
        onclick={() => void startInstall()}>{installBusy ? 'Installing…' : 'Install'}</Button
      >
    </div>
  {/if}
</Sheet>

<style>
  .success-title {
    margin: 0 0 8px;
    font-size: 14px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .explain {
    margin: 0 0 12px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
  .explain.warn {
    color: var(--msc2-status-warn);
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 260px;
    overflow-y: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;
    margin-bottom: 12px;
  }
  .list::-webkit-scrollbar {
    display: none;
    width: 0;
  }
  .list .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 11px 14px;
    background: var(--msc2-tier-chrome);
    border: 1px solid transparent;
    border-radius: 8px;
    color: var(--msc2-text-primary);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .list .row:disabled {
    opacity: 0.7;
    cursor: not-allowed;
  }
  .list .row.selected {
    border-color: rgba(255, 255, 255, 0.28);
  }
  .runtime-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .runtime-heading {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .runtime-version {
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .runtime-path {
    overflow: hidden;
    color: var(--msc2-text-tertiary);
    font-family: var(--msc2-font-mono, monospace);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag {
    flex-shrink: 0;
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.4px;
    text-transform: uppercase;
    color: var(--msc2-text-tertiary);
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
