<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '../../components/base/Button.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { errorMessage, mutate } from '../shared/types';
  import { pollOperation } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let serverName = 'Server';
  export let operationKind: 'initiate' | 'start' = 'start';
  export let errorCode = '';
  export let failureMessage = '';
  export let problems: Schema['StartupProblemDTO'][] = [];
  export let onRetry: () => void | Promise<void> = () => {};

  let currentProblems: Schema['StartupProblemDTO'][] = problems;
  let repairProblemId = '';
  let repairNotice = '';
  let retrying = false;

  $: if (problems !== currentProblems && !repairProblemId) currentProblems = problems;

  onMount(() => {
    if (errorCode !== 'server_startup_timeout') void refreshProblems();
  });

  async function refreshProblems(): Promise<void> {
    if (!api) return;
    try {
      const response = await api.get<Schema['HealthProblemsResponseDTO']>('/v1/health/problems');
      currentProblems = response.problems;
    } catch {
      // The operation error remains useful when the health route is briefly
      // unavailable, and the console stays available below the diagnosis.
    }
  }

  function titleForCode(): string {
    if (currentProblems.length) return `${serverName} could not start`;
    switch (errorCode) {
      case 'unusable_java_runtime':
        return 'Java could not run this server';
      case 'bedrock_provisioning_failed':
        return 'Bedrock could not be prepared';
      case 'capability_unavailable':
        return 'This server capability is unavailable';
      case 'bedrock_start_failed':
        return 'Bedrock could not start';
      case 'server_port_in_use':
        return 'The server port is already in use';
      case 'server_startup_timeout':
        return 'The server did not become ready';
      default:
        return `${serverName} could not start`;
    }
  }

  function summaryForCode(): string {
    if (currentProblems.length) return problemExplanation(currentProblems[0]);
    if (failureMessage) return failureMessage;
    return 'The process stopped before the server became ready.';
  }

  function problemExplanation(problem: Schema['StartupProblemDTO']): string {
    if (problem.kind === 'missingDependency' && problem.missingDependency) {
      const requirement = problem.requirement ? ` ${problem.requirement}.` : '';
      return `${problem.offenderName} cannot load because ${problem.missingDependency} is missing.${requirement}`;
    }
    if (problem.kind === 'incompatibleVersion') {
      return `${problem.offenderName} is not compatible with this server version.${problem.requirement ? ` ${problem.requirement}.` : ''}`;
    }
    if (problem.kind === 'duplicate') {
      return `${problem.offenderName} is installed more than once.`;
    }
    if (problem.kind === 'loadError') {
      return `${problem.offenderName} failed while loading.${problem.requirement ? ` ${problem.requirement}.` : ''}`;
    }
    return problem.requirement || 'The server stopped before it became ready.';
  }

  function actionLabel(action: string, problem: Schema['StartupProblemDTO']): string {
    switch (action) {
      case 'install':
        return problem.missingDependency
          ? `Install ${problem.missingDependency}`
          : 'Install dependency';
      case 'update':
        return `Update ${problem.offenderName}`;
      case 'disable':
        return `Disable ${problem.offenderName}`;
      case 'delete':
        return `Remove ${problem.offenderName}`;
      default:
        return action;
    }
  }

  function actionTone(
    problem: Schema['StartupProblemDTO'],
    action: string,
  ): 'primary' | 'secondary' {
    return problem.availableActions[0] === action ? 'primary' : 'secondary';
  }

  async function repair(problem: Schema['StartupProblemDTO'], action: string): Promise<void> {
    if (!api || repairProblemId) return;
    repairProblemId = problem.id;
    repairNotice = '';
    try {
      const result = await mutate<Schema['HealthRepairResultDTO']>(api, '/v1/health/repair', {
        problemId: problem.id,
        action,
      });
      if (result.operationId) {
        const operation = await pollOperation(api, result.operationId);
        if (operation?.state !== 'succeeded') {
          throw new Error(operation?.error?.message ?? 'The repair did not complete.');
        }
      }
      currentProblems = result.updated?.problems ?? [];
      repairNotice = `${result.message} Review the diagnosis, then retry ${operationKind}.`;
    } catch (error) {
      repairNotice = errorMessage(error);
    } finally {
      repairProblemId = '';
    }
  }

  async function retry(): Promise<void> {
    if (retrying) return;
    retrying = true;
    try {
      await onRetry();
    } finally {
      retrying = false;
    }
  }
</script>

<div class="failure-panel" aria-live="polite">
  <div class="failure-heading">
    <p class="msc2-type-overline">Startup diagnosis</p>
    <h2>{titleForCode()}</h2>
    <p class="summary">{summaryForCode()}</p>
  </div>

  {#if currentProblems.length}
    <section class="findings" aria-label="Startup findings">
      <p class="section-label">
        {currentProblems.length === 1 ? 'What stopped the server' : 'What stopped the server'}
      </p>
      {#each currentProblems as problem (problem.id)}
        <div class="finding">
          <div class="finding-copy">
            <strong>{problem.kindTitle}</strong>
            <p>{problemExplanation(problem)}</p>
            {#if problem.installedFile}<code>{problem.installedFile}</code>{/if}
          </div>
          {#if problem.availableActions.length}
            <div class="finding-actions">
              {#each problem.availableActions as action}
                <Button
                  variant={actionTone(problem, action)}
                  size="sm"
                  disabled={repairProblemId !== ''}
                  onclick={() => void repair(problem, action)}
                  >{actionLabel(action, problem)}</Button
                >
              {/each}
            </div>
          {/if}
        </div>
        {#if problem.rawExcerpt}
          <details class="evidence">
            <summary>Show the matching log excerpt</summary>
            <pre>{problem.rawExcerpt}</pre>
          </details>
        {/if}
      {/each}
    </section>
  {:else}
    <section class="next-checks" aria-label="Recommended checks">
      <p class="section-label">Recommended next checks</p>
      <p>MSC could not identify one safe automatic repair from the available evidence.</p>
      <ul>
        <li>Check the selected Java runtime and server version.</li>
        <li>Check the server file, configuration, world, and configured ports.</li>
        <li>Review the console below for the first meaningful error.</li>
      </ul>
    </section>
  {/if}

  {#if repairNotice}<p class="repair-notice" role="status">{repairNotice}</p>{/if}

  <div class="failure-actions">
    <Button
      variant="primary"
      disabled={retrying || repairProblemId !== ''}
      onclick={() => void retry()}
    >
      {retrying ? 'Retrying…' : `Retry ${operationKind}`}
    </Button>
  </div>
</div>

<style>
  .failure-panel {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .failure-heading,
  .findings,
  .next-checks {
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  h2 {
    margin: 0;
    color: var(--msc2-text-primary);
    font-size: 15px;
    font-weight: 600;
  }
  .summary,
  .next-checks > p:not(.section-label),
  .repair-notice,
  .finding p {
    margin: 0;
    color: var(--msc2-text-tertiary);
    font-size: 12px;
    line-height: 1.55;
  }
  .section-label {
    margin: 0;
    color: var(--msc2-text-secondary);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .finding {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 11px 0;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .finding:first-of-type {
    border-top: none;
    padding-top: 2px;
  }
  .finding-copy {
    display: flex;
    min-width: 0;
    flex: 1;
    flex-direction: column;
    gap: 4px;
  }
  .finding-copy strong {
    color: var(--msc2-text-primary);
    font-size: 12px;
    font-weight: 500;
  }
  .finding-copy code {
    overflow: hidden;
    color: var(--msc2-text-secondary);
    font-family: var(--msc2-font-mono);
    font-size: 10px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .finding-actions {
    display: flex;
    flex: 0 0 auto;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 6px;
  }
  .evidence {
    margin: -3px 0 4px;
    color: var(--msc2-text-secondary);
    font-size: 11px;
  }
  .evidence summary {
    cursor: pointer;
  }
  pre {
    max-height: 150px;
    margin: 8px 0 0;
    padding: 8px;
    overflow: auto;
    color: var(--msc2-text-secondary);
    background: var(--msc2-tier-terminal);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 6px;
    font-family: var(--msc2-font-mono);
    font-size: 10px;
    line-height: 1.45;
    white-space: pre-wrap;
  }
  .next-checks {
    padding-top: 2px;
  }
  ul {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin: 2px 0 0;
    padding-left: 17px;
    color: var(--msc2-text-tertiary);
    font-size: 12px;
    line-height: 1.45;
  }
  .repair-notice {
    color: var(--msc2-status-ok);
  }
  .failure-actions {
    display: flex;
    justify-content: flex-end;
    padding-top: 10px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  @media (max-width: 560px) {
    .finding {
      flex-direction: column;
    }
    .finding-actions {
      justify-content: flex-start;
    }
  }
</style>
