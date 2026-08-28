<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '../../components/base/Button.svelte';
  import Card from '../../components/base/Card.svelte';
  import ConfirmDialog from '../../components/ConfirmDialog.svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import StatusDot from '../../components/base/StatusDot.svelte';
  import type { ScreenApi } from '../shared/types';
  import { errorMessage, mutate } from '../shared/types';

  type ResetMode = 'configuration' | 'everything';
  type ResetPhase = 'ready' | 'working' | 'finished';

  type HostResetAccepted = {
    operationId: string;
    hostId: string;
    mode: ResetMode;
    agentState: 'restarting' | 'needs_pairing' | 'unavailable';
    message: string;
  };

  type OperationSnapshot = {
    state: 'queued' | 'running' | 'succeeded' | 'failed' | 'cancelled' | string;
    statusLine?: string;
    error?: { message?: string };
  };

  export let api: ScreenApi | undefined = undefined;
  export let hostLabel = 'Selected host';
  export let permissions: readonly string[] = [];
  export let isDesktopShell = false;
  export let isLocalHost = false;
  export let onClose: () => void;
  export let onClientReset: () => Promise<void>;
  export let onHostResetComplete: (result: HostResetAccepted) => Promise<void>;

  let serversRootPath = '';
  let rootError = '';
  let mode: ResetMode = 'configuration';
  let confirmation = '';
  let phase: ResetPhase = 'ready';
  let operationId = '';
  let statusLine = '';
  let resultMessage = '';
  let error = '';
  let showHostConfirmation = false;
  let showClientConfirmation = false;
  let clientBusy = false;

  const isAdmin = permissions.includes('admin');
  const expectedConfirmation = 'RESET AGENT';
  $: hostConfirmationReady =
    isAdmin &&
    !!api &&
    !!serversRootPath &&
    confirmation === expectedConfirmation &&
    phase === 'ready';
  $: modeTitle = mode === 'configuration' ? 'Configuration only' : 'Everything';

  onMount(async () => {
    if (!api) return;
    try {
      const response = await api.get<{ path: string }>('/v1/config/servers-root');
      serversRootPath = response.path;
    } catch (caught) {
      rootError = errorMessage(caught);
    }
  });

  function selectMode(next: ResetMode): void {
    if (phase !== 'ready') return;
    mode = next;
    confirmation = '';
  }

  async function resetClient(): Promise<void> {
    if (clientBusy) return;
    clientBusy = true;
    error = '';
    try {
      await onClientReset();
    } catch (caught) {
      error = errorMessage(caught);
      clientBusy = false;
    }
  }

  async function resetHost(): Promise<void> {
    if (!api || !hostConfirmationReady) return;
    phase = 'working';
    showHostConfirmation = false;
    error = '';
    statusLine = 'Submitting the host reset…';
    try {
      const accepted = await mutate<HostResetAccepted>(api, '/v1/host/reset', {
        mode,
        confirmation,
      });
      operationId = accepted.operationId;
      resultMessage = accepted.message;
      statusLine = 'Reset accepted. Waiting for the agent to finish…';
      await followOperation(accepted.operationId);
      phase = 'finished';
      await onHostResetComplete(accepted);
    } catch (caught) {
      phase = 'ready';
      error = errorMessage(caught);
    }
  }

  async function followOperation(id: string): Promise<void> {
    if (!api) return;
    for (let attempt = 0; attempt < 12; attempt += 1) {
      try {
        const operation = await api.get<OperationSnapshot>(
          `/v1/operations/${encodeURIComponent(id)}`,
        );
        statusLine = operation.statusLine ?? 'Resetting host state…';
        if (['succeeded', 'failed', 'cancelled'].includes(operation.state)) {
          if (operation.state !== 'succeeded') {
            throw new Error(operation.error?.message ?? 'The host reset did not complete.');
          }
          return;
        }
      } catch (caught) {
        // The old credential is allowed to disappear before the terminal
        // operation snapshot is readable. The accepted response is already
        // the source of truth for the recovery path.
        void caught;
        statusLine = 'The agent went offline as the reset took effect.';
        return;
      }
      await new Promise((resolve) => setTimeout(resolve, 350));
    }
    statusLine = 'The reset was accepted; the agent is taking longer than expected.';
  }

  function hostStateLabel(result: HostResetAccepted): string {
    if (result.agentState === 'needs_pairing') return 'Fresh pairing required';
    if (result.agentState === 'restarting') return 'Agent restarting';
    return 'Agent unavailable';
  }
</script>

<Sheet title="Reset" size="md" {onClose}>
  {#if phase === 'finished'}
    <div class="stack">
      <p class="msc2-type-overline">Reset accepted</p>
      <h2>{hostLabel} has been reset</h2>
      <p class="copy">{resultMessage}</p>
      <p class="copy">Operation: <span class="mono">{operationId}</span></p>
      <Button variant="secondary" onclick={onClose}>Close</Button>
    </div>
  {:else}
    <div class="stack">
      <section>
        <p class="msc2-type-overline">This device</p>
        <h2>Reset this client</h2>
        <p class="copy">
          Forget remembered hosts, credentials, preferences, and first-launch progress on this
          device. It does not change any host or Minecraft files.
        </p>
        <Button
          variant="destructive"
          disabled={clientBusy}
          onclick={() => (showClientConfirmation = true)}>Reset this client…</Button
        >
      </section>

      <section>
        <p class="msc2-type-overline">Selected host</p>
        <h2>Reset {hostLabel}</h2>
        <p class="copy">
          This action clears MSC state on the host.
          {#if isLocalHost && isDesktopShell}
            After a full reset, this desktop can also remove its own agent service.
          {:else}
            It does not uninstall the agent service on another computer.
          {/if}
        </p>
        <Card padding="0">
          <div class="detail-row"><span>Host</span><strong>{hostLabel}</strong></div>
          <div class="detail-row">
            <span>Server folder</span><span class="mono path">{serversRootPath || 'Loading…'}</span>
          </div>
        </Card>
      </section>

      {#if isAdmin}
        <section class="host-reset">
          <p class="msc2-type-overline">Host reset mode</p>
          <div class="mode-list" role="radiogroup" aria-label="Host reset mode">
            <button
              type="button"
              class:chosen={mode === 'configuration'}
              class="mode"
              role="radio"
              aria-checked={mode === 'configuration'}
              onclick={() => selectMode('configuration')}
            >
              <strong>Configuration only</strong>
              <span
                >Clear MSC configuration, credentials, and sessions. Keep worlds, jars, logs,
                backups, and the server folder.</span
              >
            </button>
            <button
              type="button"
              class:chosen={mode === 'everything'}
              class="mode"
              role="radio"
              aria-checked={mode === 'everything'}
              onclick={() => selectMode('everything')}
            >
              <strong>Everything</strong>
              <span
                >Also remove the complete managed server folder. The agent service remains installed
                unless this is the local desktop.</span
              >
            </button>
          </div>
          <div class="confirmation">
            <label for="host-reset-confirm"
              >Type <span class="mono">{expectedConfirmation}</span> to continue</label
            >
            <input
              id="host-reset-confirm"
              class="confirm-field"
              bind:value={confirmation}
              autocomplete="off"
              spellcheck="false"
            />
          </div>
          {#if rootError}<p class="error" role="alert">
              Could not read the server folder: {rootError}
            </p>{/if}{#if error}<p class="error" role="alert">{error}</p>{/if}
          <div class="actions">
            <Button
              variant="destructive"
              disabled={!hostConfirmationReady}
              onclick={() => (showHostConfirmation = true)}>Reset {modeTitle}…</Button
            >
          </div>
        </section>
      {:else}
        <p class="copy">Administrator access is required to reset a host.</p>
      {/if}

      {#if phase === 'working'}
        <div class="progress" role="status" aria-live="polite">
          <StatusDot tone="warn" label="Reset in progress" />
          <p>{statusLine}</p>
        </div>
      {/if}
    </div>
  {/if}
</Sheet>

<ConfirmDialog
  open={showClientConfirmation}
  title="Reset this client?"
  message="This removes this device's remembered hosts, credentials, preferences, and onboarding state. Host files and the agent service stay unchanged."
  context="This device only"
  confirmLabel="Reset this client"
  onConfirm={() => void resetClient()}
  onClose={() => (showClientConfirmation = false)}
/>

<ConfirmDialog
  open={showHostConfirmation}
  title={`Reset ${hostLabel}?`}
  message={mode === 'configuration'
    ? 'MSC configuration, sessions, and credentials will be cleared. Minecraft worlds, jars, logs, backups, and the server folder will remain.'
    : 'MSC configuration and the complete managed server folder will be removed. This cannot be undone.'}
  context={`Host: ${hostLabel} · ${modeTitle}`}
  confirmLabel={`Reset ${modeTitle}`}
  onConfirm={() => void resetHost()}
  onClose={() => (showHostConfirmation = false)}
/>

<style>
  /* The final confirmation is rendered above this sheet. Keep its backdrop
     above the sheet's scrim too, otherwise the sheet intercepts the click. */
  :global(.backdrop) {
    z-index: 200 !important;
  }

  .stack,
  section,
  .mode-list {
    display: grid;
    gap: 10px;
  }
  .stack {
    gap: 22px;
  }
  h2,
  p {
    margin: 0;
  }
  h2 {
    font-size: 16px;
    font-weight: 600;
  }
  .copy,
  .mode span,
  .progress p {
    color: var(--msc2-text-secondary);
    font-size: 12px;
    line-height: 1.55;
  }
  .detail-row {
    display: grid;
    grid-template-columns: 110px minmax(0, 1fr);
    gap: 12px;
    padding: 11px 14px;
    color: var(--msc2-text-secondary);
    font-size: 12px;
  }
  .detail-row + .detail-row {
    border-top: 1px solid var(--msc2-hairline-faint);
  }
  .detail-row strong {
    color: var(--msc2-text-primary);
    font-weight: 500;
  }
  .mono {
    font-family: var(--msc2-font-mono, monospace);
  }
  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: right;
  }
  .mode {
    display: grid;
    gap: 4px;
    padding: 12px 14px;
    color: var(--msc2-text-primary);
    text-align: left;
    background: var(--msc2-tier-content);
    border: 1px solid var(--msc2-hairline-faint);
    border-radius: 9px;
    font: inherit;
    cursor: pointer;
  }
  .mode:hover,
  .mode.chosen {
    border-color: var(--msc2-hairline-field-focus);
  }
  .mode strong {
    font-size: 13px;
    font-weight: 500;
  }
  .confirmation {
    display: grid;
    gap: 6px;
    margin-top: 4px;
  }
  .confirmation label {
    color: var(--msc2-text-secondary);
    font-size: 12px;
  }
  .confirm-field {
    box-sizing: border-box;
    width: 100%;
    padding: 7px 10px;
    color: var(--msc2-text-primary);
    font: inherit;
    font-size: 13px;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-field);
    border-radius: 8px;
    outline: none;
  }
  .confirm-field:focus-visible {
    border-color: var(--msc2-hairline-field-focus);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
  }
  .progress {
    display: grid;
    gap: 8px;
    padding-top: 4px;
  }
  .error {
    color: var(--msc2-status-error);
    font-size: 12px;
    line-height: 1.5;
  }
  @media (max-width: 560px) {
    .detail-row {
      grid-template-columns: 1fr;
      gap: 4px;
    }
    .path {
      text-align: left;
    }
  }
</style>
