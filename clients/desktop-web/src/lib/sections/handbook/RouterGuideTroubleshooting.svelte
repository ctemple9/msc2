<script lang="ts">
  // MSC 1 RouterPortForwardGuideTroubleshootingScreen.swift's symptom-driven
  // diagnosis, rebuilt to S0 against the real troubleshooting engine
  // (crates/msc-domain/src/router/troubleshooting.rs, wired by P12.16a).
  // The oracle color-codes causes/actions/escalation by card (neutral/blue/
  // orange); here only severity -- a real computed system state, not a
  // content classification -- keeps a reserved status color
  // (docs/msc2/antiAIslop.md; see routerLabels.ts).
  import Badge from '../../components/base/Badge.svelte';
  import Button from '../../components/base/Button.svelte';
  import Card from '../../components/base/Card.svelte';
  import { ApiError } from '../../api/client';
  import type { ScreenApi } from '../shared/types';
  import type { RouterSymptom, RouterTroubleshootingAnalysis } from '../../help/types';
  import { severityLabel, severityTone } from './routerLabels';

  export let api: ScreenApi | undefined = undefined;
  export let symptoms: readonly RouterSymptom[] = [];
  export let onBack: () => void;
  export let onOpenGuide: (id: string) => void;

  type Phase =
    | { kind: 'idle' }
    | { kind: 'busy' }
    | { kind: 'done'; analysis: RouterTroubleshootingAnalysis }
    | { kind: 'failed'; message: string };

  let selected = new Set<string>();
  let phase: Phase = { kind: 'idle' };
  let expandedCauseIds = new Set<string>();

  function toggleSymptom(id: string): void {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selected = next;
    phase = { kind: 'idle' };
  }

  function toggleCause(id: string): void {
    const next = new Set(expandedCauseIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    expandedCauseIds = next;
  }

  async function analyze(): Promise<void> {
    if (!api) {
      phase = { kind: 'failed', message: 'Connect to an agent to run a diagnosis.' };
      return;
    }
    phase = { kind: 'busy' };
    try {
      const analysis = await api.post<RouterTroubleshootingAnalysis>(
        '/v1/guides/router/troubleshooting/analyze',
        { symptoms: [...selected] },
      );
      phase = { kind: 'done', analysis };
    } catch (error) {
      phase = {
        kind: 'failed',
        message:
          error instanceof ApiError
            ? error.error.message
            : 'Could not analyze the selected symptoms.',
      };
    }
  }

  function reset(): void {
    selected = new Set();
    expandedCauseIds = new Set();
    phase = { kind: 'idle' };
  }
</script>

<div class="troubleshooting">
  <p class="msc2-type-overline">Step 3 of 3 — Diagnose issues</p>

  <div class="checklist">
    <p class="msc2-type-overline">What are you seeing?</p>
    <Card padding="0">
      {#each symptoms as symptom, index (symptom.id)}
        <button
          type="button"
          class="symptom-row"
          class:selected={selected.has(symptom.id)}
          aria-pressed={selected.has(symptom.id)}
          onclick={() => toggleSymptom(symptom.id)}
        >
          <span class="check" aria-hidden="true">{selected.has(symptom.id) ? '✓' : ''}</span>
          <span class="text">
            <span class="msc2-type-card">{symptom.title}</span>
            <span class="msc2-type-meta">{symptom.description}</span>
          </span>
        </button>
        {#if index < symptoms.length - 1}<div class="divider"></div>{/if}
      {/each}
    </Card>
  </div>

  {#if selected.size === 0 && phase.kind !== 'done'}
    <p class="msc2-type-meta">Select the symptoms above to get a diagnosis.</p>
  {:else}
    <Button
      variant="primary"
      disabled={selected.size === 0 || phase.kind === 'busy'}
      onclick={analyze}
    >
      {phase.kind === 'busy' ? 'Analyzing…' : 'Analyze'}
    </Button>
  {/if}

  {#if phase.kind === 'failed'}
    <p class="msc2-type-meta error">{phase.message}</p>
  {:else if phase.kind === 'done'}
    {@const analysis = phase.analysis}
    <div class="results">
      <Card>
        <p class="msc2-type-overline">Diagnosis</p>
        <p class="msc2-type-card">{analysis.summary}</p>
      </Card>

      {#if analysis.likelyCauses.length === 0}
        <Card>
          <p class="msc2-type-card">No specific cause matched.</p>
          <p class="msc2-type-body muted">
            Try the advanced troubleshooting guide for deeper diagnostics.
          </p>
          <Button size="sm" variant="secondary" onclick={() => onOpenGuide('advanced-troubleshooting')}>
            Open advanced troubleshooting
          </Button>
        </Card>
      {:else}
        <div class="causes">
          <p class="msc2-type-overline">Likely causes</p>
          {#each analysis.likelyCauses as cause (cause.id)}
            <Card padding="0">
              <button
                type="button"
                class="cause-header"
                onclick={() => toggleCause(cause.id)}
                aria-expanded={expandedCauseIds.has(cause.id)}
              >
                <Badge variant="status" tone={severityTone(cause.severity)}>
                  {severityLabel(cause.severity)}
                </Badge>
                <span class="msc2-type-card cause-title">{cause.topic.title}</span>
                <span class="chevron" class:open={expandedCauseIds.has(cause.id)}>›</span>
              </button>
              {#if expandedCauseIds.has(cause.id)}
                <div class="cause-body">
                  <p class="msc2-type-body muted">{cause.topic.summary}</p>
                  {#if cause.topic.suggestedNextActions.length}
                    <p class="msc2-type-meta">Suggested actions:</p>
                    <ul class="msc2-type-body muted">
                      {#each cause.topic.suggestedNextActions as action (action)}
                        <li>{action}</li>
                      {/each}
                    </ul>
                  {/if}
                </div>
              {/if}
            </Card>
          {/each}
        </div>
      {/if}

      {#if analysis.recommendedActions.length}
        <Card>
          <p class="msc2-type-overline">Recommended actions</p>
          <ul class="msc2-type-body muted">
            {#each analysis.recommendedActions as action (action)}
              <li>{action}</li>
            {/each}
          </ul>
        </Card>
      {/if}

      {#if analysis.escalationBullets.length}
        <Card>
          <p class="msc2-type-overline">Important</p>
          <ul class="msc2-type-body muted">
            {#each analysis.escalationBullets as bullet (bullet)}
              <li>{bullet}</li>
            {/each}
          </ul>
        </Card>
      {/if}

      {#if analysis.fallbackResolution?.fallbackGuideId || analysis.fallbackResolution?.matchedGuideId}
        {@const targetId =
          analysis.fallbackResolution.matchedGuideId ?? analysis.fallbackResolution.fallbackGuideId}
        <Card>
          <p class="msc2-type-overline">Suggested path</p>
          {#if analysis.fallbackResolution.explanationBullets.length}
            <ul class="msc2-type-body muted">
              {#each analysis.fallbackResolution.explanationBullets as bullet (bullet)}
                <li>{bullet}</li>
              {/each}
            </ul>
          {/if}
          {#if targetId}
            <Button size="sm" variant="secondary" onclick={() => onOpenGuide(targetId)}>
              Open suggested guide
            </Button>
          {/if}
        </Card>
      {/if}
    </div>
  {/if}

  <div class="footer">
    <Button size="sm" variant="secondary" onclick={onBack}>Back</Button>
    {#if selected.size > 0 || phase.kind === 'done'}
      <Button size="sm" variant="secondary" onclick={reset}>Clear and restart</Button>
    {/if}
  </div>
</div>

<style>
  /* Router Guide runs a size step above the shared type scale, and this
     screen a step above that again -- Cameron's own visual-review call. */
  .troubleshooting :global(.msc2-type-overline) {
    font-size: 12px;
  }
  .troubleshooting :global(.msc2-type-body),
  .troubleshooting :global(.msc2-type-card) {
    font-size: 16px;
  }
  .troubleshooting :global(.msc2-type-meta) {
    font-size: 14px;
    color: var(--msc2-text-primary);
  }
  .troubleshooting {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .checklist,
  .results,
  .causes {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .symptom-row {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    width: 100%;
    padding: 10px 14px;
    border: none;
    background: transparent;
    color: var(--msc2-text-primary);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .symptom-row:hover {
    background: var(--msc2-hairline-subtle);
  }
  .symptom-row.selected {
    background: var(--msc2-neutral-elevated);
  }
  .check {
    width: 16px;
    height: 16px;
    flex: none;
    margin-top: 1px;
    border-radius: 4px;
    border: 1px solid var(--msc2-hairline);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    color: var(--msc2-text-primary);
  }
  .symptom-row.selected .check {
    background: var(--msc2-neutral-muted);
    color: var(--msc2-text-primary);
    border-color: var(--msc2-neutral-muted);
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .divider {
    height: 1px;
    background: var(--msc2-hairline-subtle);
    margin-left: 40px;
  }
  .muted {
    color: var(--msc2-text-primary);
  }
  .error {
    color: var(--msc2-status-error);
  }
  ul {
    margin: 4px 0 0;
    padding-left: 1.1rem;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .cause-header {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 11px 14px;
    border: none;
    background: transparent;
    color: var(--msc2-text-primary);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .cause-title {
    flex: 1;
  }
  .chevron {
    color: var(--msc2-text-tertiary);
    transform: rotate(90deg);
    transition: transform 0.15s ease;
  }
  .chevron.open {
    transform: rotate(-90deg);
  }
  .cause-body {
    padding: 0 14px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 10px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
</style>
