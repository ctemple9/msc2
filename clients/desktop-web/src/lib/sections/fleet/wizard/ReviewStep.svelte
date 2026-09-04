<script lang="ts">
  // Real port of AddServerWizardView.swift's step3ImportReview -- editable
  // Server Settings (Max Players, Accept EULA) and, when the scan found more
  // than one world, a picker for which becomes active. This step's own plan
  // text scopes the picker to "more than one world"; the oracle always
  // renders the section whenever any world exists, but with exactly one
  // there is nothing to actually pick -- the scan's own `defaultWorldName`
  // is already the active choice (set the moment `UploadStep.svelte`'s scan
  // succeeds), shown as-is here and on Confirm. Values only patch `draft`;
  // nothing is created yet -- the real `POST /v1/servers/import` call is
  // this path's own Confirm step.
  import NumberField from '../../../components/base/NumberField.svelte';
  import Toggle from '../../../components/base/Toggle.svelte';
  import { bytesLabel } from '../../shared/types';
  import type { WizardDraft } from './model';

  export let draft: WizardDraft;

  $: scan = draft.importScan;
  $: worlds = scan?.worlds ?? [];
  $: activeWorldName = draft.importActiveWorldName ?? scan?.defaultWorldName;
</script>

<div class="review">
  {#if scan}
    <div class="intro">
      <h2>Review server settings</h2>
      <p>These were read from the server folder. Change anything before continuing.</p>
    </div>

    <section class="block">
      <p class="msc2-type-overline">Server Settings</p>
      <div class="summary">
        <div class="row">
          <span class="label">Server type</span>
          <span class="value">{draft.serverType === 'bedrock' ? 'Bedrock' : 'Java'}</span>
        </div>
        <div class="row">
          <span class="label">Max players</span>
          <NumberField
            value={draft.importMaxPlayers}
            min={1}
            max={1000}
            width="90px"
            onValueChange={(value) =>
              (draft.importMaxPlayers = Number(value) || draft.importMaxPlayers)}
          />
        </div>
        <div class="row">
          <span class="label">Accept EULA</span>
          <Toggle
            checked={draft.importEulaAccepted}
            onchange={(checked) => (draft.importEulaAccepted = checked)}
          />
        </div>
      </div>
      {#if !draft.importEulaAccepted}
        <p class="hint warn">
          The EULA must be accepted before the server can start. You can accept it now or do it
          later in the server editor.
        </p>
      {/if}
    </section>

    <section class="block">
      <p class="msc2-type-overline">Active World</p>
      {#if worlds.length === 0}
        <p class="hint">
          No worlds were detected in this server folder. A world will be generated on first start.
        </p>
      {:else if worlds.length === 1}
        {@const only = worlds[0]}
        <p class="hint">{only.name} — {only.dimensionsLabel} · {bytesLabel(only.sizeBytes)}</p>
      {:else}
        <div class="list">
          {#each worlds as world (world.id)}
            <button
              type="button"
              class="world-row"
              class:selected={activeWorldName === world.name}
              onclick={() => (draft.importActiveWorldName = world.name)}
            >
              <span class="world-name">{world.name}</span>
              <span class="world-meta">{world.dimensionsLabel} · {bytesLabel(world.sizeBytes)}</span
              >
            </button>
          {/each}
        </div>
      {/if}
    </section>
  {/if}
</div>

<style>
  .review {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .intro {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .intro h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .intro p {
    margin: 0;
    font-size: 12.5px;
    color: var(--msc2-text-tertiary);
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
  }
  .block > .summary,
  .block > .list {
    width: 100%;
  }

  .summary {
    display: flex;
    flex-direction: column;
    background: var(--msc2-tier-chrome);
    border-radius: 10px;
    overflow: hidden;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 14px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .row:first-child {
    border-top: none;
  }
  .label {
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .value {
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }

  .hint {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
  .hint.warn {
    color: var(--msc2-status-warn);
  }

  .list {
    display: flex;
    flex-direction: column;
    background: var(--msc2-tier-chrome);
    border-radius: 10px;
    overflow: hidden;
  }
  .world-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px;
    border-top: 1px solid var(--msc2-hairline-subtle);
    background: transparent;
    border-left: none;
    border-right: none;
    border-bottom: none;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .world-row:first-child {
    border-top: none;
  }
  .world-row.selected {
    background: rgba(255, 255, 255, 0.06);
  }
  .world-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .world-meta {
    font-size: 11.5px;
    color: var(--msc2-text-tertiary);
  }
</style>
