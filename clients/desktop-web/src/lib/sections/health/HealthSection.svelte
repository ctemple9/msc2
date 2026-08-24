<script lang="ts">
  import { onMount } from 'svelte';
  import ActionButton from '../../components/ActionButton.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import HelpLink from '../../help/HelpLink.svelte';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, errorMessage, mutate } from '../shared/types';

  export let api: ScreenProps['api'] = undefined;
  export let hostId = 'local-agent';
  export let serverId = 'survival';
  let health: Schema['HealthResponseDTO'] = {
    cards: [],
    overallSeverity: 'unknown',
    serverName: 'Survival',
    serverRunning: false,
    serverType: 'paper',
  };
  let problems: Schema['HealthProblemsResponseDTO'] = {
    isSoftFail: false,
    problems: [],
    serverRunning: false,
    serverType: 'paper',
  };
  let notice = '';
  onMount(async () => {
    health = await call(api, health, '/v1/health');
    problems = await call(api, problems, '/v1/health/problems');
  });
  async function repair(problem: Schema['StartupProblemDTO'], action: string): Promise<void> {
    try {
      const result = await mutate<Schema['HealthRepairResultDTO']>(api, '/v1/health/repair', {
        problemId: problem.id,
        action,
      });
      notice = result.message;
    } catch (error) {
      notice = errorMessage(error);
    }
  }
</script>

<div class="screen">
  <ScreenHeader
    eyebrow="Diagnostics"
    title="Health"
    description="Cards explain the current server condition and repairs retain the agent's help and operation context."
    status={health.overallSeverity}
    statusTone={health.overallSeverity === 'critical'
      ? 'danger'
      : health.overallSeverity === 'ok'
        ? 'positive'
        : 'warning'}
    actionLabel="Refresh health"
    onAction={async () => {
      health = await call(api, health, '/v1/health');
      problems = await call(api, problems, '/v1/health/problems');
    }}
  />
  {#if notice}<p class="muted" role="status">{notice}</p>{/if}
  {#if health.note}<p class="capability-notice">{health.note}</p>{/if}
  <div class="screen-grid three">
    {#each health.cards as card (card.id)}<article class="screen-card">
        <div class="screen-card-header">
          <h3>{card.title}</h3>
          <span class="tag">{card.shortLabel}</span>
        </div>
        <p>{card.detail ?? 'No additional detail.'}</p>
        {#if card.actionCode}<ActionButton
            kind="quiet"
            label={card.actionLabel ?? 'Repair'}
            onclick={() => problems.problems[0] && repair(problems.problems[0], card.actionCode!)}
            >{card.actionLabel ?? 'Repair'}</ActionButton
          >{/if}
      </article>{:else}<div class="screen-card">
        <h3>Waiting for diagnostics</h3>
        <p class="muted">The connected agent has not returned health cards.</p>
      </div>{/each}
  </div>
  <section class="screen-card">
    <div class="screen-card-header">
      <h3>Startup problems</h3>
      <span class="metric-label">{problems.problems.length} found</span>
    </div>
    {#if problems.note}<p class="muted">
        {problems.note}
      </p>{/if}{#each problems.problems as problem (problem.id)}<div class="operation-row">
        <div>
          <strong>{problem.kindTitle}</strong>
          <p>{problem.offenderName} · {problem.rawExcerpt}</p>
          {#if problem.helpId}<HelpLink helpId={problem.helpId} {hostId} {serverId} />{/if}
        </div>
        <div class="screen-actions">
          {#each problem.availableActions as action}<ActionButton
              kind={action === 'delete' ? 'danger' : 'quiet'}
              label={action}
              disabled={problem.isRepairing}
              onclick={() => repair(problem, action)}>{action}</ActionButton
            >{/each}
        </div>
      </div>{:else}<p class="muted">No startup problems are recorded for this server.</p>{/each}
  </section>
</div>
