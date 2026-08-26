<script lang="ts">
  // Ports WorldConversionWizardView.swift's shape (preflight -> pick a
  // target server -> pick the target version -> convert), simplified to
  // what this agent can actually do today. Real gap, faced honestly:
  // WorldConvertRequestDTO.targetFormat must be one of Chunker's own
  // installed, edition-specific format strings (never a client guess —
  // MSC 1's own wizard always populates this from
  // ChunkerManager.supportedFormats(javaPath:), an actual query against the
  // installed jar), and no route exposes that list yet
  // (msc_application::world_conversion::WorldConverter::supported_formats has
  // no HTTP handler in crates/msc-agent/src/routes/worlds.rs). Rather than
  // fabricate a hardcoded format list or a freeform text field the user has
  // to guess a working value for, this wizard runs its real preflight and
  // target-server steps, then stops at an honest "not available yet" panel
  // instead of a fake version picker. A follow-up contract-amendment step
  // (the same shape as P12.3a) can add the missing route and extend this
  // wizard the rest of the way once it exists.
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import type { Schema } from '../shared/types';
  import { compatibleTargetServers } from './model';

  export let sourceSlot: Schema['WorldSlotDTO'];
  export let sourceServer: Schema['ServerDTO'] | undefined;
  export let sourceServerRunning: boolean;
  export let servers: readonly Schema['ServerDTO'][];
  export let onClose: () => void;

  type Step = 'preflight' | 'target' | 'blocked';
  let step: Step = 'preflight';
  let selectedTargetId: string | undefined;

  $: targetEdition = sourceServer?.serverType === 'bedrock' ? 'Java' : 'Bedrock';
  $: candidates = compatibleTargetServers(servers, sourceServer);
  $: preflightOk = !sourceServerRunning && candidates.length > 0;
  $: selectedTarget = candidates.find((server) => server.id === selectedTargetId);
</script>

<Sheet title="Convert World" size="md" {onClose}>
  <div class="source">
    <span class="msc2-type-overline">Source</span>
    <span class="source-line">{sourceServer?.name ?? '—'} · "{sourceSlot.name}"</span>
  </div>

  {#if step === 'preflight'}
    <div class="body">
      <p class="msc2-type-overline">Checking requirements</p>
      <ul class="checklist">
        <li class:bad={sourceServerRunning}>
          {sourceServerRunning
            ? `"${sourceServer?.name ?? 'This server'}" is currently running. Stop it before converting.`
            : `"${sourceServer?.name ?? 'This server'}" is stopped — safe to convert.`}
        </li>
        <li class:bad={candidates.length === 0}>
          {candidates.length === 0
            ? `No ${targetEdition} servers found. Create one, then return here.`
            : `${candidates.length} ${targetEdition} server(s) available.`}
        </li>
      </ul>
      <div class="footer">
        <Button variant="secondary" onclick={onClose}>Cancel</Button>
        <Button variant="primary" disabled={!preflightOk} onclick={() => (step = 'target')}>
          Next
        </Button>
      </div>
    </div>
  {:else if step === 'target'}
    <div class="body">
      <p class="msc2-type-overline">Target {targetEdition} server</p>
      <div class="target-list">
        {#each candidates as server (server.id)}
          <button
            type="button"
            class="target-row"
            class:selected={selectedTargetId === server.id}
            onclick={() => (selectedTargetId = server.id)}
          >
            <span class="target-name">{server.name}</span>
            {#if selectedTargetId === server.id}<span class="check">✓</span>{/if}
          </button>
        {/each}
      </div>
      <div class="footer">
        <Button variant="secondary" onclick={() => (step = 'preflight')}>Back</Button>
        <Button variant="primary" disabled={!selectedTarget} onclick={() => (step = 'blocked')}>
          Next
        </Button>
      </div>
    </div>
  {:else}
    <div class="body">
      <p class="msc2-type-overline">Not available on this runtime yet</p>
      <p class="explain">
        Converting "{sourceSlot.name}" to {selectedTarget?.name} needs a target-version list from Chunker,
        MSC's world converter. That discovery route isn't exposed by this agent yet, so MSC can't offer
        a real (non-guessed) version choice. Nothing was changed on either server.
      </p>
      <div class="footer">
        <Button variant="secondary" onclick={() => (step = 'target')}>Back</Button>
        <Button variant="primary" onclick={onClose}>Close</Button>
      </div>
    </div>
  {/if}
</Sheet>

<style>
  .source {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 16px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--msc2-hairline-faint);
  }
  .source span:first-child {
    color: var(--msc2-text-tertiary);
  }
  .source-line {
    font-size: 13px;
    color: var(--msc2-text-primary);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .checklist {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: var(--msc2-tier-chrome);
    border-radius: 8px;
    padding: 12px;
  }
  .checklist li {
    font-size: 12px;
    color: var(--msc2-status-ok);
  }
  .checklist li.bad {
    color: var(--msc2-status-warn);
  }
  .target-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .target-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    background: var(--msc2-tier-chrome);
    border: 1px solid transparent;
    border-radius: 8px;
    color: var(--msc2-text-primary);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .target-row.selected {
    border-color: rgba(255, 255, 255, 0.28);
  }
  .check {
    color: var(--msc2-status-ok);
  }
  .explain {
    margin: 0;
    font-size: 12px;
    line-height: 1.6;
    color: var(--msc2-text-tertiary);
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
