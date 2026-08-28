<script lang="ts">
  // Real port of AddServerWizardView.swift's outer shell (step chips, scroll
  // body, Back/Continue/Done footer) plus its step 1, Choose Path. Structural
  // precedent: worlds/WorldConversionWizard.svelte's single-component,
  // `type Step`, {#if step === n} shape -- no generic reusable "wizard
  // framework" invented for this.
  //
  // Path cards port the oracle's WizardPathCard *information* (title,
  // subtitle, selected state) but not its literal accent-tinted
  // icon-in-a-box treatment -- antiAIslop.md tell #6/#11. They use the same
  // flat neutral selected-state treatment already established for
  // WorldConversionWizard's target-row (a plain border, no accent tint):
  // accent stays reserved for running-state/active-tab/primary-action/status.
  // The step counter follows the same rule -- the oracle's chips are kept in
  // shape (numbered dot, connector, label) but redone in neutral tones only,
  // per this block's own note in rolling-plan.md (antiAIslop rule #8).
  //
  // Step 1 (Choose Path) and, for the Fresh path, step 2 (Configure,
  // P12.18b/c) are real. Every step past that is still a placeholder --
  // P12.18d-i replace each one in turn; the step-chip labels below already
  // reflect the oracle's real sequence so nothing here needs to change shape
  // when they land, just content.
  import Sheet from '../../../components/base/Sheet.svelte';
  import Button from '../../../components/base/Button.svelte';
  import ConfigureStep from './ConfigureStep.svelte';
  import type { ScreenApi } from '../../shared/types';
  import {
    canAdvanceConfigure,
    defaultWizardDraft,
    wizardStepLabels,
    type WizardPath,
  } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let onClose: () => void;

  let path: WizardPath = 'importExisting';
  let currentStep = 1;
  let draft = defaultWizardDraft();

  $: labels = wizardStepLabels(path);
  $: totalSteps = labels.length;
  $: canContinue =
    currentStep === 1 || (currentStep === 2 && path === 'fresh' && canAdvanceConfigure(draft));

  function continueStep(): void {
    if (currentStep < totalSteps && canContinue) currentStep += 1;
  }

  function backStep(): void {
    if (currentStep > 1) currentStep -= 1;
  }
</script>

<Sheet title="Add Server" size="lg" {onClose}>
  <div class="wizard">
    <div class="steps" role="list" aria-label="Add Server progress">
      {#each labels as label, index (label)}
        {@const stepNum = index + 1}
        <div class="step" role="listitem">
          <span
            class="dot"
            class:done={stepNum < currentStep}
            class:current={stepNum === currentStep}
          >
            {#if stepNum < currentStep}✓{:else}{stepNum}{/if}
          </span>
          <span class="label" class:seen={stepNum <= currentStep}>{label}</span>
        </div>
        {#if stepNum < totalSteps}
          <div class="connector" class:done={stepNum < currentStep}></div>
        {/if}
      {/each}
    </div>

    <div class="content">
      {#if currentStep === 1}
        <div class="intro">
          <h2>How do you want to add this server?</h2>
          <p>Import a server you already have, or start a brand new one from scratch.</p>
        </div>
        <div class="paths">
          <button
            type="button"
            class="path-card"
            class:selected={path === 'importExisting'}
            onclick={() => (path = 'importExisting')}
          >
            <span class="path-title">Import Existing</span>
            <span class="path-subtitle"
              >Drop a folder or .zip — MSC reads and configures it for you.</span
            >
          </button>
          <button
            type="button"
            class="path-card"
            class:selected={path === 'fresh'}
            onclick={() => (path = 'fresh')}
          >
            <span class="path-title">Start Fresh</span>
            <span class="path-subtitle"
              >MSC downloads and sets up a brand new server from scratch.</span
            >
          </button>
        </div>
      {:else if currentStep === 2 && path === 'fresh'}
        <ConfigureStep {api} bind:draft />
      {:else}
        <p class="stub">"{labels[currentStep - 1]}" lands in a later step.</p>
      {/if}
    </div>

    <div class="footer">
      {#if currentStep > 1}
        <Button variant="secondary" onclick={backStep}>Back</Button>
      {/if}
      <div class="spacer"></div>
      <Button variant="primary" onclick={continueStep} disabled={!canContinue}>Continue</Button>
    </div>
  </div>
</Sheet>

<style>
  .wizard {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  /* Step counter -- shape kept from the oracle's chips, color stripped to
     neutral tiers/opacity only (no accent), per antiAIslop rule #8. */
  .steps {
    display: flex;
    align-items: center;
  }
  .step {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }
  .dot {
    width: 20px;
    height: 20px;
    flex-shrink: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-subtle);
    font-size: 10px;
    font-weight: 600;
    color: var(--msc2-text-tertiary);
  }
  .dot.current {
    border-color: rgba(255, 255, 255, 0.4);
    color: var(--msc2-text-primary);
  }
  .dot.done {
    background: rgba(255, 255, 255, 0.1);
    color: var(--msc2-text-secondary);
  }
  .label {
    font-size: 11.5px;
    color: var(--msc2-text-tertiary);
    white-space: nowrap;
  }
  .label.seen {
    color: var(--msc2-text-secondary);
  }
  .connector {
    flex: 1;
    height: 1px;
    margin: 0 8px;
    background: var(--msc2-hairline-subtle);
  }
  .connector.done {
    background: rgba(255, 255, 255, 0.28);
  }

  .content {
    display: flex;
    flex-direction: column;
    gap: 18px;
    min-height: 260px;
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

  .paths {
    display: flex;
    gap: 12px;
  }
  .path-card {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    text-align: left;
    padding: 16px;
    min-height: 96px;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 10px;
    font: inherit;
    cursor: pointer;
  }
  .path-card.selected {
    border-color: rgba(255, 255, 255, 0.32);
    background: rgba(255, 255, 255, 0.05);
  }
  .path-title {
    font-size: 13.5px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .path-subtitle {
    font-size: 12px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }

  .stub {
    margin: 0;
    font-size: 12.5px;
    color: var(--msc2-text-tertiary);
  }

  .footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-top: 14px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .spacer {
    flex: 1;
  }
</style>
