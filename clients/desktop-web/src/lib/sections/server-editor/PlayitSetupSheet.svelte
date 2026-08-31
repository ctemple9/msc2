<script lang="ts">
  // The native account flow from MSC 1's ContentView Playit sheet, moved behind
  // the agent-owned P12.20a operation. The client collects credentials only
  // long enough to submit them, then clears both fields before the operation
  // begins. It never receives or displays the resulting agent key.
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import Field from '../../components/base/Field.svelte';
  import StatusDot from '../../components/base/StatusDot.svelte';
  import { openExternal } from '../../platform';
  import type { Schema, ScreenApi } from '../shared/types';
  import { mutate } from '../shared/types';
  import {
    PLAYIT_SETUP_STEPS,
    playitSetupError,
    playitSetupProgressForStatus,
    playitSetupStepsForMode,
    pollOperation,
    serverEditorPaths,
    type PlayitSetupContext,
    type PlayitSetupProgressKey,
    type PlayitSetupStep,
    type PlayitSetupAccepted,
    type PlayitResetResult,
  } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let playit: Schema['PlayitStatusResponseDTO'] | undefined = undefined;
  export let context: PlayitSetupContext = 'settings';
  export let voiceOnly = false;
  export let visible = true;
  export let onClose: () => void;
  /** Records that the user submitted the native Playit attempt, even if it later fails. */
  export let onAttempted: () => void = () => {};
  /** Called as soon as the agent reports success. In initiation mode the
   * parent uses this to resume its waiting transport row, after which this
   * sheet closes so the row is immediately visible again. */
  export let onComplete: () => void = () => {};
  export let onReset: () => void = () => {};

  type Phase =
    | 'form'
    | 'submitting'
    | 'progress'
    | 'failed'
    | 'cancelled'
    | 'configured'
    | 'confirm-reset'
    | 'resetting'
    | 'reset';

  let phase: Phase = 'form';
  let email = '';
  let password = '';
  let operationId = '';
  let statusLine = '';
  let progressKey: PlayitSetupProgressKey = 'signing_in';
  let error = '';
  let resetMessage = '';
  let cancelRequested = false;
  let setupSteps: readonly PlayitSetupStep[] = PLAYIT_SETUP_STEPS;

  $: setupSteps = playitSetupStepsForMode(voiceOnly);
  $: currentStepIndex = Math.max(
    0,
    setupSteps.findIndex((step) => step.key === progressKey),
  );
  $: busy = phase === 'submitting' || phase === 'progress' || phase === 'resetting';
  $: title = context === 'initiation' ? 'Connect Playit for first start' : 'Set up Playit';
  $: intro = voiceOnly
    ? 'Your Playit agent is already configured. Sign in again to add the missing Simple Voice Chat tunnel; existing tunnels will be reused.'
    : playit?.hasSecretKey
      ? "Sign in again to repair this host's Playit setup or add any tunnel the agent says is missing."
      : 'Sign in with your Playit account and MSC will create or reuse the agent and applicable tunnels for this server.';
  $: submitLabel = voiceOnly ? 'Add voice tunnel' : 'Set up Playit';
  $: liveStatus = statusLine || setupSteps[currentStepIndex]?.label || 'Working…';

  async function openAccountPage(): Promise<void> {
    try {
      await openExternal('https://playit.gg/login');
    } catch (caught) {
      error = playitSetupError(caught);
    }
  }

  async function submit(): Promise<void> {
    if (busy || !api) return;
    const submittedEmail = email.trim();
    const submittedPassword = password;
    if (!submittedEmail || !submittedPassword) {
      error = 'Enter your Playit email and password to continue.';
      phase = 'form';
      return;
    }

    // Do not leave submitted credentials in rendered fields while the agent
    // performs the longer claim, reuse, and tunnel work.
    email = '';
    password = '';
    error = '';
    statusLine = 'Signing in…';
    progressKey = 'signing_in';
    cancelRequested = false;
    onAttempted();
    phase = 'submitting';

    try {
      const accepted = await mutate<PlayitSetupAccepted>(api, serverEditorPaths.playitSetup, {
        email: submittedEmail,
        password: submittedPassword,
      });
      operationId = accepted.operationId;
      statusLine = accepted.message ?? 'Sign-in accepted. Preparing Playit setup…';
      progressKey = playitSetupProgressForStatus(statusLine, progressKey);
      phase = 'progress';

      if (cancelRequested) await requestCancellation();
      const operation = await pollOperation(api, operationId, (tick) => {
        statusLine = tick.statusLine ?? statusLine;
        progressKey = playitSetupProgressForStatus(statusLine, progressKey);
      });
      if (operation?.state === 'succeeded') {
        statusLine = operation.statusLine ?? 'Playit is ready.';
        phase = 'configured';
        onComplete();
        if (context === 'initiation') onClose();
      } else if (operation?.state === 'cancelled') {
        statusLine = operation.statusLine ?? 'Playit setup cancelled.';
        phase = 'cancelled';
      } else {
        error = playitSetupError(
          operation?.error?.message ??
            'Playit setup did not complete. Check the agent and try again.',
        );
        phase = 'failed';
      }
    } catch (caught) {
      error = playitSetupError(caught);
      phase = 'failed';
    }
  }

  async function requestCancellation(): Promise<void> {
    if (!api || !operationId || cancelRequested) return;
    cancelRequested = true;
    error = '';
    try {
      const operation = await api.post<Schema['OperationDTO']>(
        `/v1/operations/${encodeURIComponent(operationId)}/cancel`,
      );
      statusLine = operation.statusLine ?? 'Cancellation requested…';
    } catch (caught) {
      cancelRequested = false;
      error = `Could not cancel Playit setup: ${playitSetupError(caught)}`;
    }
  }

  function retry(): void {
    if (busy) return;
    phase = 'form';
    operationId = '';
    statusLine = '';
    error = '';
    cancelRequested = false;
  }

  function beginReset(): void {
    if (busy) return;
    phase = 'confirm-reset';
    error = '';
  }

  async function resetLocalSetup(): Promise<void> {
    if (!api || busy) return;
    phase = 'resetting';
    error = '';
    statusLine = "Stopping Playit and clearing this host's local setup…";
    try {
      const result = await mutate<PlayitResetResult>(api, serverEditorPaths.playitReset);
      if (result.operationId) {
        const operation = await pollOperation(api, result.operationId, (tick) => {
          statusLine = tick.statusLine ?? statusLine;
        });
        if (operation?.state !== 'succeeded') {
          throw new Error(operation?.error?.message ?? 'The local Playit reset did not complete.');
        }
      }
      resetMessage =
        result.message ??
        (result.result === 'already_clear'
          ? 'This host already had no local Playit setup.'
          : 'Playit setup was cleared from this host.');
      phase = 'reset';
      onReset();
    } catch (caught) {
      error = playitSetupError(caught);
      phase = 'confirm-reset';
    }
  }
</script>

<Sheet {title} size="md" {visible} onClose={busy ? undefined : onClose}>
  {#if phase === 'form' || phase === 'failed'}
    <div class="stack">
      <div>
        <p class="msc2-type-overline">Native setup</p>
        <h2>{context === 'initiation' ? 'Finish the transport setup' : 'No key to copy'}</h2>
        <p class="copy">{intro}</p>
      </div>

      <div class="fields">
        <label>
          <span>Email</span>
          <Field bind:value={email} type="email" placeholder="you@example.com" />
        </label>
        <label>
          <span>Password</span>
          <Field bind:value={password} type="password" placeholder="Playit password" />
        </label>
      </div>

      <a
        class="account-link"
        href="https://playit.gg/account"
        target="_blank"
        rel="noopener noreferrer"
        onclick={(event) => {
          event.preventDefault();
          void openAccountPage();
        }}
      >
        Create a free Playit account
      </a>

      <p class="warning" role="note">
        Accounts protected by two-factor authentication cannot be completed in this native flow yet.
        If Playit asks for a second factor, no changes will be made.
      </p>

      {#if phase === 'failed'}
        <p class="error" role="alert">{error}</p>
      {:else if error}
        <p class="error" role="alert">{error}</p>
      {/if}

      <div class="actions">
        {#if playit?.hasSecretKey}
          <Button variant="destructive" size="sm" onclick={beginReset}>Reset local setup…</Button>
        {/if}
        <span class="action-spacer"></span>
        <Button variant="secondary" onclick={onClose}>Cancel</Button>
        <Button
          variant="primary"
          disabled={!api || !email.trim() || !password}
          onclick={() => void submit()}
        >
          {phase === 'failed' ? 'Try again' : submitLabel}
        </Button>
      </div>
    </div>
  {:else if phase === 'submitting' || phase === 'progress'}
    <div class="stack">
      <div>
        <p class="msc2-type-overline">Playit setup</p>
        <h2>{voiceOnly ? 'Adding the voice tunnel' : 'Setting up your tunnels'}</h2>
        <p class="copy">
          MSC is doing this through the agent. Your password is not retained in this sheet.
        </p>
      </div>

      <div class="progress-list" aria-label="Playit setup progress" aria-live="polite">
        {#each setupSteps as step, index (step.key)}
          <div
            class="progress-row"
            class:current={index === currentStepIndex}
            class:complete={index < currentStepIndex}
          >
            <span class="progress-state">
              {index < currentStepIndex ? 'Done' : index === currentStepIndex ? 'Now' : 'Next'}
            </span>
            <span>{step.label}</span>
          </div>
        {/each}
      </div>

      <p class="status-line" role="status">{liveStatus}</p>
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      <div class="actions">
        <span class="action-spacer"></span>
        <Button
          variant="secondary"
          disabled={!operationId || cancelRequested}
          onclick={() => void requestCancellation()}
          >{cancelRequested ? 'Cancelling…' : 'Cancel setup'}</Button
        >
      </div>
    </div>
  {:else if phase === 'configured'}
    <div class="stack">
      <div>
        <p class="msc2-type-overline">Playit setup complete</p>
        <h2>{voiceOnly ? 'Voice tunnel added' : 'Playit is ready'}</h2>
        <p class="copy">
          MSC has finished the agent setup and will reuse the applicable named tunnels on this host.
          {#if context === 'initiation'}The first-start flow can now continue waiting for the
            transport.{/if}
        </p>
      </div>
      <StatusDot tone="ok" label="Configured" />
      {#if playit?.javaAddress || playit?.bedrockAddress || playit?.voiceAddress}
        <div class="addresses">
          {#if playit?.javaAddress}<div>
              <span>Java</span><code>{playit.javaAddress}</code>
            </div>{/if}
          {#if playit?.bedrockAddress}<div>
              <span>Bedrock</span><code>{playit.bedrockAddress}</code>
            </div>{/if}
          {#if playit?.voiceAddress}<div>
              <span>Voice</span><code>{playit.voiceAddress}</code>
            </div>{/if}
        </div>
      {:else}
        <p class="copy quiet">Public addresses will appear here after the agent reports them.</p>
      {/if}
      <div class="actions">
        <Button variant="destructive" size="sm" onclick={beginReset}>Reset local setup…</Button>
        <span class="action-spacer"></span>
        <Button variant="secondary" onclick={onClose}>Done</Button>
      </div>
    </div>
  {:else if phase === 'cancelled'}
    <div class="stack">
      <p class="msc2-type-overline">Playit setup</p>
      <h2>Setup cancelled</h2>
      <StatusDot tone="warn" label="Cancelled" />
      <p class="copy">No Playit setup was completed. You can start again whenever you are ready.</p>
      <div class="actions">
        <span class="action-spacer"></span>
        <Button variant="secondary" onclick={onClose}>Close</Button>
        <Button variant="primary" onclick={retry}>Try again</Button>
      </div>
    </div>
  {:else if phase === 'confirm-reset'}
    <div class="stack">
      <p class="msc2-type-overline">Reset local setup</p>
      <h2>Forget this host's Playit connection?</h2>
      <p class="copy">
        MSC will stop its helper and remove the saved key, agent ID, public addresses, and setup
        prompt from this host. Your Playit account, agent, and cloud tunnels are not deleted.
      </p>
      {#if error}<p class="error" role="alert">{error}</p>{/if}
      <div class="actions">
        <span class="action-spacer"></span>
        <Button variant="secondary" onclick={retry}>Cancel</Button>
        <Button variant="destructive" onclick={() => void resetLocalSetup()}
          >Reset local setup</Button
        >
      </div>
    </div>
  {:else if phase === 'resetting'}
    <div class="stack">
      <p class="msc2-type-overline">Reset local setup</p>
      <h2>Clearing this host's Playit state</h2>
      <p class="status-line" role="status">{statusLine}</p>
    </div>
  {:else}
    <div class="stack">
      <p class="msc2-type-overline">Reset complete</p>
      <h2>Local Playit setup cleared</h2>
      <StatusDot tone="warn" label="Not configured" />
      <p class="copy">{resetMessage}</p>
      <p class="copy quiet">Your Playit account, agent, and cloud tunnels were left untouched.</p>
      <div class="actions">
        <span class="action-spacer"></span>
        <Button variant="secondary" onclick={onClose}>Close</Button>
      </div>
    </div>
  {/if}
</Sheet>

<style>
  .stack {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  h2 {
    margin: 5px 0 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .copy {
    margin: 8px 0 0;
    font-size: 13px;
    line-height: 1.55;
    color: var(--msc2-text-secondary);
  }
  .copy.quiet {
    margin: 0;
    color: var(--msc2-text-tertiary);
  }
  .fields {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .account-link {
    align-self: flex-start;
    border-bottom: 1px solid var(--msc2-text-tertiary);
    padding-bottom: 1px;
    color: var(--msc2-text-primary);
    background: transparent;
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    text-decoration: none;
  }
  .account-link:hover {
    border-bottom-color: var(--msc2-text-primary);
  }
  .warning,
  .error,
  .status-line {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
  }
  .warning {
    color: var(--msc2-status-warn);
  }
  .error {
    color: var(--msc2-status-error);
  }
  .status-line {
    color: var(--msc2-text-secondary);
  }
  .progress-list {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--msc2-hairline-subtle);
    border-bottom: 1px solid var(--msc2-hairline-subtle);
  }
  .progress-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    padding: 9px 0;
    border-top: 1px solid var(--msc2-hairline-subtle);
    color: var(--msc2-text-tertiary);
    font-size: 12px;
  }
  .progress-row:first-child {
    border-top: 0;
  }
  .progress-row.current {
    color: var(--msc2-text-primary);
  }
  .progress-row.complete {
    color: var(--msc2-text-secondary);
  }
  .progress-state {
    min-width: 42px;
    color: var(--msc2-text-tertiary);
    font-size: 10px;
    font-weight: 500;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .progress-row.current .progress-state {
    color: var(--msc2-text-primary);
  }
  .addresses {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 2px;
  }
  .addresses div {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  code {
    color: var(--msc2-text-primary);
    font-family: var(--msc2-font-mono, monospace);
    font-size: 12px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .action-spacer {
    flex: 1;
  }
</style>
