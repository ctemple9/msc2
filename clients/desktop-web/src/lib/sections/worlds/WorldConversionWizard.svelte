<script lang="ts">
  // Ports WorldConversionWizardView.swift's shape (preflight -> target
  // server -> target version -> summary -> converting -> done), simplified
  // to what this agent's own routes actually offer. P12.4a exposed Chunker's
  // real, already-working supported_formats over GET /v1/worlds/convert/formats,
  // so the version picker below is real -- not a guessed/hardcoded list.
  //
  // One placement option is intentionally missing, not overlooked: MSC 1's
  // wizard also lets a conversion replace an existing slot on the target
  // server (WorldConvertRequestDTO.targetSlotId). That needs the target
  // server's own slot list, and /v1/worlds only ever answers for the
  // *active* server -- there's no route to list a different, non-active
  // server's slots. Every conversion here places into a new slot
  // (targetName) until that gap gets its own backend step.
  import { onMount } from 'svelte';
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import Select from '../../components/base/Select.svelte';
  import Field from '../../components/base/Field.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { mutate } from '../shared/types';
  import { ApiError } from '../../api/client';
  import {
    compatibleTargetServers,
    formatDisplayName,
    pollOperation,
    targetFormats,
    worldPaths,
  } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let sourceSlot: Schema['WorldSlotDTO'];
  export let sourceServer: Schema['ServerDTO'] | undefined;
  export let sourceServerRunning: boolean;
  export let servers: readonly Schema['ServerDTO'][];
  export let onClose: () => void;
  export let onConverted: () => void;

  type Step = 'preflight' | 'target' | 'version' | 'summary' | 'converting' | 'done' | 'failed';
  let step: Step = 'preflight';
  let selectedTargetId: string | undefined;
  let selectedFormat = '';
  let newSlotName = '';
  let statusLine = 'Starting…';
  let failureMessage = '';

  type FormatsState =
    | { kind: 'loading' }
    | { kind: 'ready'; formats: string[] }
    | { kind: 'unavailable'; message: string };
  let formatsState: FormatsState = { kind: 'loading' };

  $: targetEdition = sourceServer?.serverType === 'bedrock' ? 'Java' : 'Bedrock';
  $: candidates = compatibleTargetServers(servers, sourceServer);
  $: selectedTarget = candidates.find((server) => server.id === selectedTargetId);
  $: availableFormats =
    formatsState.kind === 'ready' ? targetFormats(formatsState.formats, sourceServer) : [];
  $: preflightOk = !sourceServerRunning && candidates.length > 0 && formatsState.kind === 'ready';
  $: defaultSlotName = `${sourceSlot.name} (from ${sourceServer?.serverType === 'bedrock' ? 'Bedrock' : 'Java'})`;

  async function loadFormats(): Promise<void> {
    formatsState = { kind: 'loading' };
    if (!api) {
      formatsState = { kind: 'unavailable', message: 'Not connected to an agent.' };
      return;
    }
    try {
      const response = await api.get<Schema['WorldConvertFormatsResponseDTO']>(
        worldPaths.convertFormats,
      );
      formatsState = { kind: 'ready', formats: response.formats };
    } catch (error) {
      formatsState = {
        kind: 'unavailable',
        message:
          error instanceof ApiError
            ? error.error.message
            : 'Could not reach Chunker on this agent.',
      };
    }
  }

  onMount(() => {
    void loadFormats();
  });

  function enterVersionStep(): void {
    if (!selectedFormat && availableFormats.length > 0) {
      selectedFormat = availableFormats[availableFormats.length - 1];
    }
    step = 'version';
  }

  function enterSummaryStep(): void {
    if (!newSlotName.trim()) newSlotName = defaultSlotName;
    step = 'summary';
  }

  async function startConversion(): Promise<void> {
    if (!selectedTarget || !selectedFormat) return;
    step = 'converting';
    statusLine = 'Starting conversion…';
    try {
      const result = await mutate<Schema['WorldConvertResultDTO']>(api, worldPaths.convert, {
        sourceSlotId: sourceSlot.id,
        targetServerId: selectedTarget.id,
        targetFormat: selectedFormat,
        targetName: newSlotName.trim(),
      });
      const operation = await pollOperation(api, result.operationId, (tick) => {
        statusLine = tick.statusLine ?? statusLine;
      });
      if (operation?.state === 'succeeded') {
        onConverted();
        step = 'done';
      } else {
        failureMessage = operation?.error?.message ?? 'Conversion did not complete.';
        step = 'failed';
      }
    } catch (error) {
      failureMessage = error instanceof Error ? error.message : 'Failed to start the conversion.';
      step = 'failed';
    }
  }
</script>

<Sheet title="Convert World" size="md" onClose={step === 'converting' ? undefined : onClose}>
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
        {#if formatsState.kind === 'loading'}
          <li>Checking Chunker on this agent…</li>
        {:else if formatsState.kind === 'unavailable'}
          <li class="bad">{formatsState.message}</li>
        {:else}
          <li>Chunker is installed — {formatsState.formats.length} format(s) supported.</li>
        {/if}
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
        <Button variant="primary" disabled={!selectedTarget} onclick={enterVersionStep}>
          Next
        </Button>
      </div>
    </div>
  {:else if step === 'version'}
    <div class="body">
      <p class="msc2-type-overline">Target {targetEdition} version</p>
      {#if availableFormats.length === 0}
        <p class="explain">
          No {targetEdition} versions were reported by the installed Chunker jar.
        </p>
      {:else}
        <Select
          options={availableFormats.map((format) => ({
            value: format,
            label: formatDisplayName(format),
          }))}
          bind:value={selectedFormat}
        />
        <p class="explain">
          Pick the version that matches your target server. Newer versions include more blocks and
          biomes but require that version's Chunker support.
        </p>
      {/if}
      <div class="footer">
        <Button variant="secondary" onclick={() => (step = 'target')}>Back</Button>
        <Button variant="primary" disabled={!selectedFormat} onclick={enterSummaryStep}>
          Next
        </Button>
      </div>
    </div>
  {:else if step === 'summary'}
    <div class="body">
      <p class="msc2-type-overline">Ready to convert</p>
      <div class="summary">
        <div class="summary-row">
          <span class="label">From</span>
          <span class="value">{sourceServer?.name ?? '—'} · "{sourceSlot.name}"</span>
        </div>
        <div class="summary-row">
          <span class="label">To</span>
          <span class="value">{selectedTarget?.name ?? '—'}</span>
        </div>
        <div class="summary-row">
          <span class="label">Version</span>
          <span class="value">{formatDisplayName(selectedFormat)}</span>
        </div>
        <div class="summary-row column">
          <span class="label">New slot name</span>
          <Field bind:value={newSlotName} placeholder={defaultSlotName} />
        </div>
      </div>
      <p class="explain">
        A backup of "{selectedTarget?.name}"'s current active world is taken automatically before
        the conversion completes. The original world on "{sourceServer?.name}" is never changed.
        Conversion may take several minutes — don't close the app or stop either server while it
        runs.
      </p>
      <div class="footer">
        <Button variant="secondary" onclick={() => (step = 'version')}>Back</Button>
        <Button
          variant="primary"
          disabled={!newSlotName.trim()}
          onclick={() => void startConversion()}
        >
          Convert World
        </Button>
      </div>
    </div>
  {:else if step === 'converting'}
    <div class="body">
      <p class="msc2-type-overline">Converting…</p>
      <p class="status-line">{statusLine}</p>
      <p class="explain">Do not close this window or stop either server while conversion runs.</p>
    </div>
  {:else if step === 'done'}
    <div class="body centered">
      <p class="lede">Conversion Started</p>
      <p class="explain">
        "{sourceSlot.name}" is being converted onto "{selectedTarget?.name}". Its progress shows up
        as an operation; the new slot will appear once it finishes.
      </p>
      <div class="footer">
        <Button variant="primary" onclick={onClose}>Done</Button>
      </div>
    </div>
  {:else}
    <div class="body centered">
      <p class="lede error-text">Conversion Failed</p>
      <p class="explain">{failureMessage}</p>
      <div class="footer">
        <Button variant="secondary" onclick={onClose}>Close</Button>
        <Button variant="primary" onclick={() => (step = 'summary')}>Back</Button>
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
  .body.centered {
    align-items: flex-start;
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
  .summary {
    display: flex;
    flex-direction: column;
    gap: 10px;
    background: var(--msc2-tier-chrome);
    border-radius: 8px;
    padding: 12px;
  }
  .summary-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .summary-row.column {
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
  }
  .summary-row .label {
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .summary-row .value {
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .status-line {
    margin: 0;
    font-family: var(--msc2-font-mono);
    font-size: 12px;
    color: var(--msc2-text-secondary);
    background: var(--msc2-tier-chrome);
    border-radius: 8px;
    padding: 10px 12px;
  }
  .lede {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .error-text {
    color: var(--msc2-status-warn);
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
