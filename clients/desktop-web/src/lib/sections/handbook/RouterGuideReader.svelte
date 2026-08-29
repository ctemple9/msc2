<script lang="ts">
  // MSC 1 RouterPortForwardGuideReader.swift's composed guide screen,
  // rebuilt to S0 against the real composer + runtime-resolver engines
  // (crates/msc-domain/src/router/{composer,runtime_resolver}.rs, wired by
  // P12.16a). The oracle tints each section by kind (purple intro / blue
  // prerequisites / green values / neutral steps and notes) -- dropped for
  // one flat Card per section, matching HandbookBrowser's callout precedent
  // (docs/msc2/antiAIslop.md rule #1; P12.16's "spend the accent budget on
  // a defined system status, not a content classification"). An
  // undetectable local IP is a real system state, not decoration, so it
  // keeps the reserved warning treatment MSC 1 gives it.
  import Badge from '../../components/base/Badge.svelte';
  import Button from '../../components/base/Button.svelte';
  import Card from '../../components/base/Card.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import { ApiError } from '../../api/client';
  import type { ScreenApi } from '../shared/types';
  import type { ResolvedRouterGuide, RouterResolvedItem } from '../../help/types';
  import { confidenceLabel } from './routerLabels';

  export let api: ScreenApi | undefined = undefined;
  export let guideId: string;
  export let onBack: () => void;
  export let onTroubleshoot: () => void;

  type Phase =
    | { kind: 'loading' }
    | { kind: 'ready'; resolved: ResolvedRouterGuide }
    | { kind: 'no-active-server' }
    | { kind: 'failed'; message: string };

  let phase: Phase = { kind: 'loading' };
  let completedStepIds = new Set<string>();
  let expandedTopicIds = new Set<string>();
  let introExpanded = true;
  let notesExpanded = false;
  let copiedValue: string | null = null;

  $: void load(guideId);

  async function load(id: string): Promise<void> {
    phase = { kind: 'loading' };
    completedStepIds = new Set();
    expandedTopicIds = new Set();
    introExpanded = true;
    notesExpanded = false;
    if (!api) {
      phase = { kind: 'failed', message: 'Connect to an agent to open a router guide.' };
      return;
    }
    try {
      const resolved = await api.get<ResolvedRouterGuide>(
        `/v1/guides/router/${encodeURIComponent(id)}`,
      );
      phase = { kind: 'ready', resolved };
    } catch (error) {
      if (error instanceof ApiError && error.error.code === 'no_active_server') {
        phase = { kind: 'no-active-server' };
      } else {
        phase = {
          kind: 'failed',
          message: error instanceof Error ? error.message : 'Could not load this guide.',
        };
      }
    }
  }

  function stepItems(resolved: ResolvedRouterGuide): Extract<RouterResolvedItem, { type: 'step' }>[] {
    return resolved.sections
      .flatMap((section) => section.items)
      .filter((item): item is Extract<RouterResolvedItem, { type: 'step' }> => item.type === 'step');
  }

  function toggleStep(id: string): void {
    const next = new Set(completedStepIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    completedStepIds = next;
  }
  function toggleTopic(id: string): void {
    const next = new Set(expandedTopicIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    expandedTopicIds = next;
  }
  function toggleAllSteps(resolved: ResolvedRouterGuide): void {
    const ids = stepItems(resolved).map((step) => step.id);
    const allDone = ids.length > 0 && ids.every((id) => completedStepIds.has(id));
    completedStepIds = new Set(allDone ? [] : ids);
  }

  async function copy(value: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(value);
      copiedValue = value;
      setTimeout(() => {
        if (copiedValue === value) copiedValue = null;
      }, 1500);
    } catch {
      // Clipboard access can be denied by the browser; the value stays visible to select.
    }
  }

  const valueLabelTokens: readonly { prefix: string; token: string }[] = [
    { prefix: 'Target device', token: 'detected_local_ip_address' },
    { prefix: 'Java port', token: 'java_port' },
    { prefix: 'Recommended protocol', token: 'recommended_protocol' },
    { prefix: 'Bedrock port', token: 'bedrock_port' },
    { prefix: 'Bedrock enabled', token: 'bedrock_enabled' },
  ];
  function tokenForBullet(bullet: string): string | null {
    const label = bullet.split(': ')[0] ?? bullet;
    return valueLabelTokens.find((entry) => label.startsWith(entry.prefix))?.token ?? null;
  }
  function splitBullet(bullet: string): { label: string; value: string } {
    const index = bullet.indexOf(': ');
    return index === -1
      ? { label: bullet, value: '' }
      : { label: bullet.slice(0, index), value: bullet.slice(index + 2) };
  }
  function isUnresolved(resolved: ResolvedRouterGuide, sectionId: string, bullet: string): boolean {
    const token = tokenForBullet(bullet);
    if (!token) return false;
    return resolved.unresolvedTokens.some((entry) => entry.sectionId === sectionId && entry.token === token);
  }
</script>

<div class="reader">
  <p class="msc2-type-overline">Step 2 of 3 — Follow the steps</p>

  {#if phase.kind === 'loading'}
    <p class="msc2-type-meta">Loading guide…</p>
  {:else if phase.kind === 'no-active-server'}
    <EmptyState
      title="Select an active server first"
      message="This guide fills in your server's real values once a server is active."
    />
    <div class="footer">
      <Button size="sm" variant="secondary" onclick={onBack}>Back to picker</Button>
    </div>
  {:else if phase.kind === 'failed'}
    <EmptyState title="This guide could not be loaded" message={phase.message} />
    <div class="footer">
      <Button size="sm" variant="secondary" onclick={onBack}>Back to picker</Button>
    </div>
  {:else}
    {@const resolved = phase.resolved}
    <div class="head">
      <h3 class="msc2-type-page">{resolved.guide.displayName}</h3>
      <Badge>{confidenceLabel(resolved.guide.review.sourceConfidence)}</Badge>
    </div>

    {#each resolved.sections as section (section.id)}
      {#if section.kind === 'intro'}
        <Card padding="0">
          <button
            type="button"
            class="section-toggle"
            onclick={() => (introExpanded = !introExpanded)}
            aria-expanded={introExpanded}
          >
            <span class="msc2-type-overline">{section.title}</span>
            <span class="chevron" class:open={introExpanded}>›</span>
          </button>
          {#if introExpanded}
            <div class="section-body">
              {#each section.items as item, index (index)}
                {#if item.type === 'paragraph'}
                  <p class="msc2-type-body muted">{item.body}</p>
                {/if}
              {/each}
            </div>
          {/if}
        </Card>
      {:else if section.kind === 'prerequisites'}
        <Card>
          <p class="msc2-type-overline">{section.title}</p>
          {#each section.items as item, index (index)}
            {#if item.type === 'bulletList'}
              <ul class="msc2-type-body muted">
                {#each item.bullets as bullet (bullet)}
                  <li>{bullet}</li>
                {/each}
              </ul>
            {/if}
          {/each}
        </Card>
      {:else if section.kind === 'valueSummary'}
        <Card padding="0">
          <p class="msc2-type-overline value-summary-title">{section.title}</p>
          {#each section.items as item, index (index)}
            {#if item.type === 'bulletList'}
              {#if isUnresolved(resolved, section.id, item.bullets[0] ?? '')}
                <div class="ip-callout">
                  <p class="msc2-type-body">Your Mac's local IP could not be detected.</p>
                  <p class="msc2-type-meta">This is the most important value you'll enter. To find it manually:</p>
                  <ol class="msc2-type-meta">
                    <li>Open System Settings</li>
                    <li>Go to Network</li>
                    <li>Select your active connection (Wi-Fi or Ethernet)</li>
                    <li>Your IP address is shown -- it usually starts with 192.168 or 10.0</li>
                  </ol>
                </div>
              {/if}
              {#each item.bullets as bullet, bulletIndex (bulletIndex)}
                {@const split = splitBullet(bullet)}
                {@const unresolved = isUnresolved(resolved, section.id, bullet)}
                <div class="value-row">
                  <div class="value-text">
                    <span class="msc2-type-meta">{split.label}</span>
                    <span class="msc2-type-mono" class:muted={unresolved}>{split.value}</span>
                  </div>
                  {#if !unresolved}
                    <button type="button" class="copy-btn" onclick={() => copy(split.value)}>
                      {copiedValue === split.value ? 'Copied' : 'Copy'}
                    </button>
                  {/if}
                </div>
                {#if bulletIndex < item.bullets.length - 1}<div class="divider"></div>{/if}
              {/each}
            {/if}
          {/each}
        </Card>
      {:else if section.kind === 'menuPath'}
        <Card>
          <p class="msc2-type-overline">{section.title}</p>
          {#each section.items as item, index (index)}
            {#if item.type === 'menuPath'}
              {#if item.path.length}
                <p class="msc2-type-mono breadcrumb">{item.path.join(' › ')}</p>
              {/if}
              {#if item.alternateMenuNames.length}
                <p class="msc2-type-meta aside">
                  Similar labels may include: {item.alternateMenuNames.join(', ')}
                </p>
              {/if}
            {/if}
          {/each}
        </Card>
      {:else if section.kind === 'routerSpecificSteps'}
        {@const steps = section.items.filter(
          (item): item is Extract<RouterResolvedItem, { type: 'step' }> => item.type === 'step',
        )}
        <div class="steps-header">
          <span class="msc2-type-meta">{steps.length} {steps.length === 1 ? 'step' : 'steps'}</span>
          <Button size="sm" variant="secondary" onclick={() => toggleAllSteps(resolved)}>
            {steps.length > 0 && steps.every((step) => completedStepIds.has(step.id))
              ? 'Reset'
              : 'Mark all done'}
          </Button>
        </div>
        <div class="steps">
          {#each steps as step, stepIndex (step.id)}
            {@const done = completedStepIds.has(step.id)}
            <Card>
              <div class="step">
                <button
                  type="button"
                  class="step-check"
                  class:done
                  onclick={() => toggleStep(step.id)}
                  aria-pressed={done}
                >
                  {done ? '✓' : stepIndex + 1}
                </button>
                <div class="step-body">
                  <p class="msc2-type-card" class:muted={done}>{step.title}</p>
                  <p class="msc2-type-body step-desc muted">{step.body}</p>
                  {#if step.alternateTerms.length}
                    <p class="msc2-type-meta aside">Also called: {step.alternateTerms.join(', ')}</p>
                  {/if}
                </div>
              </div>
            </Card>
          {/each}
        </div>
      {:else if section.kind === 'notes'}
        <Card padding="0">
          <button
            type="button"
            class="section-toggle"
            onclick={() => (notesExpanded = !notesExpanded)}
            aria-expanded={notesExpanded}
          >
            <span class="msc2-type-overline">{section.title}</span>
            <span class="chevron" class:open={notesExpanded}>›</span>
          </button>
          {#if notesExpanded}
            <div class="section-body notes">
              {#each section.items as item, index (index)}
                {#if item.type === 'note'}
                  {#if item.title}<p class="msc2-type-overline">{item.title}</p>{/if}
                  <p class="msc2-type-body muted">{item.body}</p>
                {/if}
              {/each}
            </div>
          {/if}
        </Card>
      {:else if section.kind === 'troubleshootingFooter'}
        <Card>
          <p class="msc2-type-overline">{section.title}</p>
          {#each section.items as item, index (index)}
            {#if item.type === 'paragraph'}
              <p class="msc2-type-body muted">{item.body}</p>
            {:else if item.type === 'troubleshootingTopic'}
              <div class="topic">
                <button
                  type="button"
                  class="topic-header"
                  onclick={() => toggleTopic(item.id)}
                  aria-expanded={expandedTopicIds.has(item.id)}
                >
                  <span class="msc2-type-card">{item.title}</span>
                  <span class="chevron" class:open={expandedTopicIds.has(item.id)}>›</span>
                </button>
                {#if expandedTopicIds.has(item.id)}
                  <div class="topic-body">
                    <p class="msc2-type-body muted">{item.summary}</p>
                    {#if item.suggestedNextActions.length}
                      <ul class="msc2-type-body muted">
                        {#each item.suggestedNextActions as action (action)}
                          <li>{action}</li>
                        {/each}
                      </ul>
                    {/if}
                  </div>
                {/if}
              </div>
            {/if}
          {/each}
          <Button size="sm" variant="secondary" onclick={onTroubleshoot}>Troubleshooting →</Button>
        </Card>
      {/if}
    {/each}

    <p class="msc2-type-meta variance">
      Screen layouts vary by firmware version. Menu paths and labels may differ.
    </p>

    <div class="footer">
      <Button size="sm" variant="secondary" onclick={onBack}>Back to picker</Button>
      <Button size="sm" variant="secondary" onclick={onTroubleshoot}>Troubleshooting →</Button>
    </div>
  {/if}
</div>

<style>
  /* Router Guide runs a size step above the shared type scale -- Cameron's
     own visual-review call, this component only. */
  .reader :global(.msc2-type-overline) {
    font-size: 11px;
  }
  .reader :global(.msc2-type-body),
  .reader :global(.msc2-type-card) {
    font-size: 15px;
  }
  .reader :global(.msc2-type-meta) {
    font-size: 13px;
    color: var(--msc2-text-primary);
  }
  .reader :global(.msc2-type-mono) {
    font-size: 13px;
  }
  .reader {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .head h3 {
    margin: 0;
  }
  .muted {
    color: var(--msc2-text-primary);
  }
  .aside {
    font-style: italic;
  }
  .section-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 12px 16px;
    border: none;
    background: transparent;
    font: inherit;
    cursor: pointer;
  }
  .section-body {
    padding: 0 16px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-top: 1px solid var(--msc2-hairline-subtle);
    padding-top: 12px;
  }
  .section-body.notes {
    gap: 12px;
  }
  .chevron {
    color: var(--msc2-text-tertiary);
    transform: rotate(90deg);
    transition: transform 0.15s ease;
  }
  .chevron.open {
    transform: rotate(-90deg);
  }
  ul,
  ol {
    margin: 0;
    padding-left: 1.1rem;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .value-summary-title {
    padding: 15px 16px 0;
  }
  .ip-callout {
    margin: 12px 16px 0;
    padding: 10px 12px;
    border: 1px solid var(--msc2-status-warn);
    border-radius: var(--msc2-radius-2);
    background: var(--msc2-status-warn-tint);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .value-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 16px;
  }
  .value-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .copy-btn {
    flex: none;
    border: 1px solid var(--msc2-hairline);
    border-radius: var(--msc2-radius-1);
    background: transparent;
    color: var(--msc2-text-secondary);
    font-size: 11px;
    padding: 4px 9px;
    cursor: pointer;
  }
  .copy-btn:hover {
    color: var(--msc2-text-primary);
  }
  .divider {
    height: 1px;
    background: var(--msc2-hairline-subtle);
    margin: 0 16px;
  }
  .breadcrumb {
    margin: 4px 0;
  }
  .steps-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .steps {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .step {
    display: flex;
    gap: 12px;
  }
  .step-check {
    flex: none;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    border: none;
    background: var(--msc2-neutral-muted);
    color: var(--msc2-text-primary);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }
  .step-check.done {
    background: var(--msc2-neutral-elevated);
    color: var(--msc2-text-secondary);
  }
  .step-body {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
  }
  .step-body p {
    margin: 0;
  }
  .step-desc {
    font-style: italic;
  }
  .copy-btn {
    align-self: flex-start;
  }
  .topic {
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .topic:first-of-type {
    border-top: none;
  }
  .topic-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 10px 0;
    border: none;
    background: transparent;
    font: inherit;
    cursor: pointer;
  }
  .topic-body {
    padding: 0 0 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .variance {
    text-align: center;
  }
  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 10px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
</style>
