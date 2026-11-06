<script lang="ts">
  import PluginBrowserSheet from '../components/PluginBrowserSheet.svelte';
  import ContentSourcesSheet from '../components/ContentSourcesSheet.svelte';
  import { addOnKind } from '../components/model';
  import { ApiError } from '../../api/client';
  import { onMount } from 'svelte';
  import ActionButton from '../../components/ActionButton.svelte';
  import ScreenHeader from '../shared/ScreenHeader.svelte';
  import HelpLink from '../../help/HelpLink.svelte';
  import type { Schema, ScreenProps, ScreenApi } from '../shared/types';
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
  let contentRepair:
    { api: ScreenApi; server: Schema['ServerDTO']; version?: string; update: boolean } | undefined;
  async function refreshAfterContent(): Promise<void> {
    const scoped = contentRepair?.api;
    if (!scoped || scoped.isCurrent?.() === false) return;
    try {
      const [nextHealth, nextProblems, content] = await Promise.all([
        scoped.get<Schema['HealthResponseDTO']>('/v1/health'),
        scoped.get<Schema['HealthProblemsResponseDTO']>('/v1/health/problems'),
        scoped.get<{ requirements: { state: string }[] }>(
          `/v1/servers/${encodeURIComponent(serverId)}/content/collection`,
        ),
      ]);
      if (scoped.isCurrent?.() === false) return;
      health = nextHealth;
      problems = nextProblems;
      const count = content.requirements.filter(
        (item) => !['matched', 'serverOnly', 'optionalMissing'].includes(item.state),
      ).length;
      notice = count
        ? `Content changed. ${count} player requirements need review in Components → Player content.`
        : 'Content changed; player requirements checked.';
    } catch (error) {
      if (scoped.isCurrent?.() !== false) notice = errorMessage(error);
    }
  }
  onMount(async () => {
    health = await call(api, health, '/v1/health');
    problems = await call(api, problems, '/v1/health/problems');
  });
  async function repair(problem: Schema['StartupProblemDTO'], action: string): Promise<void> {
    const scoped = api?.snapshot?.() ?? api;
    if (!scoped || scoped.isCurrent?.() === false) return;
    try {
      const result = await mutate<Schema['HealthRepairResultDTO']>(scoped, '/v1/health/repair', {
        problemId: problem.id,
        action,
      });
      notice = result.message;
    } catch (error) {
      notice = errorMessage(error);
      if (error instanceof ApiError && error.error.code === 'content_review_required' && api) {
        const details = error.error.details as { provider?: string; serverId?: string } | undefined;
        if (
          details?.provider !== 'curseforge' ||
          details.serverId !== serverId ||
          scoped.isCurrent?.() === false
        )
          return;
        try {
          const [servers, versions] = await Promise.all([
            scoped.get<Schema['ServerDTO'][]>('/v1/servers'),
            scoped.get<Schema['VersionsResponseDTO']>('/v1/versions'),
          ]);
          if (scoped.isCurrent?.() === false) return;
          const server = servers.find((item) => item.id === serverId);
          if (server)
            contentRepair = {
              api: scoped,
              server,
              version: versions.currentVersion ?? undefined,
              update: action === 'update',
            };
        } catch (cause) {
          notice = errorMessage(cause);
        }
      }
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

{#if contentRepair}
  {#if contentRepair.update}
    <ContentSourcesSheet
      api={contentRepair.api}
      serverId={contentRepair.server.id}
      onClose={() => (contentRepair = undefined)}
      onChanged={() => void refreshAfterContent()}
    />
  {:else}
    <PluginBrowserSheet
      api={contentRepair.api}
      serverId={contentRepair.server.id}
      serverName={contentRepair.server.name}
      addOnKind={addOnKind(contentRepair.server.javaFlavor)}
      javaFlavor={contentRepair.server.javaFlavor ?? undefined}
      serverMinecraftVersion={contentRepair.version}
      initialProvider="curseforge"
      onClose={() => (contentRepair = undefined)}
      onInstalled={() => void refreshAfterContent()}
    />
  {/if}
{/if}
