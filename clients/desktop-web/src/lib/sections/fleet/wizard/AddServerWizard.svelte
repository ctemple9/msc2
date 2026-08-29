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
  // P12.18b/c), step 3 (Network, P12.18d), step 4 (World, P12.18e), step 5
  // (Add-ons, P12.18f, only when the flavor accepts add-ons -- otherwise
  // step 5 is Confirm directly), and the final Confirm step (P12.18g) are
  // real. P12.18h adds the Import path's own Upload/Review steps (its
  // Network and Confirm steps reuse the Fresh path's own NetworkStep/
  // ConfirmStep components unchanged, gated by `path` where their content
  // actually differs) -- only the modpack-drop variant (P12.18i) remains.
  //
  // Confirm's Create/Done buttons replace this footer's Continue in place
  // (same "parent owns Back/primary-action, each step is a presentational
  // view over the shared draft" shape every prior step already established)
  // rather than ConfirmStep growing its own footer -- see its own header
  // comment. The sheet (and Back) refuse to close mid-create, matching
  // worlds/WorldConversionWizard.svelte's identical `onClose={... ?
  // undefined : onClose}` precedent for a durable operation already
  // in flight.
  import { onDestroy } from 'svelte';
  import Sheet from '../../../components/base/Sheet.svelte';
  import Button from '../../../components/base/Button.svelte';
  import { activeTourStep, tourServerContext } from '../../../help/onboarding';
  import ConfigureStep from './ConfigureStep.svelte';
  import NetworkStep from './NetworkStep.svelte';
  import WorldStep from './WorldStep.svelte';
  import AddOnsStep from './AddOnsStep.svelte';
  import UploadStep from './UploadStep.svelte';
  import ReviewStep from './ReviewStep.svelte';
  import ConfirmStep from './ConfirmStep.svelte';
  import { onboardingAnchor } from '../../../help/tourAnchors';
  import type { ScreenApi } from '../../shared/types';
  import { errorMessage } from '../../shared/types';
  import {
    canAdvanceConfigure,
    canAdvanceNetwork,
    canAdvanceUpload,
    canAdvanceWorld,
    canCreateServer,
    createServerFromDraft,
    defaultWizardDraft,
    hasAddOnsStep,
    importDisplayNameFromPath,
    importServerFromDraft,
    wizardStepLabels,
    type WizardPath,
  } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let onClose: () => void;
  /** Called once the real create operation succeeds, so the caller can
   *  refresh its own server list -- mirrors `ManageSheet.svelte`'s existing
   *  `refreshServers` used after Import. */
  export let onCreated: () => void = () => {};

  let path: WizardPath = 'importExisting';
  let currentStep = 1;
  let draft = defaultWizardDraft();

  let displayName = '';
  let isCreating = false;
  let statusMessage = '';
  let createSucceeded = false;
  let createWarnings: string[] = [];

  $: tourPathLocked = $activeTourStep === 'choose-path';
  $: if (tourPathLocked && path !== 'fresh') selectPath('fresh');
  $: tourServerContext.set({
    serverType: draft.serverType,
    javaCategory: draft.javaCategory,
    javaFlavor: draft.javaFlavor,
    enableCrossPlay: draft.enableCrossPlay,
  });

  onDestroy(() => tourServerContext.set(null));

  // `AddServerWizardView.swift`'s `hasAddOnsStep` -- inserts a sixth "Add-ons"
  // step at position 5 (Fresh path only) once the chosen Java flavor accepts
  // add-ons, shifting Confirm from 5 to 6. When it's false, position 5 is
  // Confirm directly and the layout is unchanged from before this step.
  $: showAddOns = path === 'fresh' && hasAddOnsStep(draft);
  $: showModpack = path === 'importExisting' && draft.stagedModpack !== undefined;
  $: labels = wizardStepLabels(path, showAddOns, showModpack);
  $: totalSteps = labels.length;
  // Confirm is always the final step of either path now that P12.18h gives
  // Import its own real steps -- `AddServerWizardView.swift`'s own
  // `confirmStepNum` is likewise just "the last step" for both paths.
  $: isConfirmStep = currentStep === totalSteps;
  // `AddServerWizardView.swift`'s own "prefill once, stay editable" default
  // -- Fresh from `serverName`, Import from the scanned source's own file/
  // folder name (`advanceStep`'s identical `currentStep == 3` prefill).
  $: if (isConfirmStep && !displayName.trim()) {
    if (path === 'fresh' && draft.serverName.trim()) {
      displayName = draft.serverName;
    } else if (path === 'importExisting' && draft.stagedModpack?.inspection.packName) {
      displayName = draft.stagedModpack.inspection.packName;
    } else if (path === 'importExisting' && draft.importSourcePath) {
      displayName = importDisplayNameFromPath(draft.importSourcePath);
    }
  }
  $: canContinue =
    currentStep === 1 ||
    (currentStep === 2 && path === 'fresh' && canAdvanceConfigure(draft)) ||
    (currentStep === 2 && path === 'importExisting' && canAdvanceUpload(draft)) ||
    (currentStep === 3 && path === 'fresh' && canAdvanceNetwork(draft)) ||
    (currentStep === 3 && path === 'importExisting' && showModpack && canAdvanceNetwork(draft)) ||
    // Review's own `canAdvance` case is unconditional in the oracle --
    // nothing on this step blocks Continue once it's reachable at all.
    (currentStep === 3 && path === 'importExisting' && !showModpack) ||
    (currentStep === 4 && path === 'fresh' && canAdvanceWorld(draft)) ||
    (currentStep === 4 && path === 'importExisting' && showModpack && canAdvanceWorld(draft)) ||
    (currentStep === 4 && path === 'importExisting' && !showModpack && canAdvanceNetwork(draft)) ||
    (currentStep === 5 && path === 'fresh' && showAddOns);

  function continueStep(): void {
    if (currentStep < totalSteps && canContinue) currentStep += 1;
  }

  function selectPath(next: WizardPath): void {
    path = next;
    if (next === 'fresh') {
      draft = {
        ...draft,
        stagedModpack: undefined,
        importSourcePath: undefined,
        importIsZip: false,
        importScan: undefined,
      };
    }
  }

  function backStep(): void {
    if (currentStep > 1) currentStep -= 1;
  }

  async function beginCreate(): Promise<void> {
    if (!canCreateServer(displayName) || isCreating) return;
    isCreating = true;
    statusMessage = path === 'fresh' ? 'Creating server…' : 'Importing server…';
    const onProgress = (line: string) => (statusMessage = line);
    try {
      const { warnings } =
        path === 'fresh'
          ? await createServerFromDraft(api, draft, displayName, onProgress)
          : showModpack
            ? await createServerFromDraft(api, draft, displayName, onProgress)
            : await importServerFromDraft(api, draft, displayName, onProgress);
      createWarnings = warnings;
      createSucceeded = true;
      onCreated();
    } catch (error) {
      statusMessage = errorMessage(error);
    } finally {
      isCreating = false;
    }
  }
</script>

<Sheet title="Add Server" size="lg" onClose={isCreating ? undefined : onClose}>
  <div class="wizard" use:onboardingAnchor={'ob_wizard_sheet'}>
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
        <div class="paths" use:onboardingAnchor={'ob_wizard_path_picker'}>
          <button
            type="button"
            class="path-card"
            class:selected={path === 'importExisting'}
            disabled={tourPathLocked}
            onclick={() => selectPath('importExisting')}
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
            use:onboardingAnchor={'ob_wizard_fresh_card'}
            onclick={() => selectPath('fresh')}
          >
            <span class="path-title">Start Fresh</span>
            <span class="path-subtitle"
              >MSC downloads and sets up a brand new server from scratch.</span
            >
          </button>
        </div>
      {:else if currentStep === 2 && path === 'fresh'}
        <ConfigureStep {api} bind:draft />
      {:else if currentStep === 2 && path === 'importExisting'}
        <UploadStep {api} bind:draft />
      {:else if currentStep === 3 && path === 'fresh'}
        <NetworkStep bind:draft />
      {:else if currentStep === 3 && path === 'importExisting' && showModpack}
        <NetworkStep bind:draft />
      {:else if currentStep === 3 && path === 'importExisting'}
        <ReviewStep bind:draft />
      {:else if currentStep === 4 && path === 'fresh'}
        <WorldStep {api} bind:draft />
      {:else if currentStep === 4 && path === 'importExisting' && showModpack}
        <WorldStep {api} bind:draft />
      {:else if currentStep === 4 && path === 'importExisting'}
        <NetworkStep bind:draft />
      {:else if currentStep === 5 && path === 'fresh' && showAddOns}
        <AddOnsStep {api} bind:draft />
      {:else if isConfirmStep}
        <ConfirmStep
          {api}
          {path}
          bind:draft
          bind:displayName
          {isCreating}
          {statusMessage}
          {createSucceeded}
          {createWarnings}
        />
      {:else}
        <p class="stub">"{labels[currentStep - 1]}" lands in a later step.</p>
      {/if}
    </div>

    <div class="footer">
      {#if !createSucceeded && currentStep > 1}
        <Button variant="secondary" onclick={backStep} disabled={isCreating}>Back</Button>
      {/if}
      <div class="spacer"></div>
      {#if createSucceeded}
        <Button variant="primary" onclick={onClose}>Done</Button>
      {:else if isConfirmStep}
        <Button
          variant="primary"
          anchorId="ob_create_save"
          onclick={() => void beginCreate()}
          disabled={!canCreateServer(displayName) || isCreating}
        >
          Create Server
        </Button>
      {:else}
        <Button
          variant="primary"
          anchorId="ob_wizard_continue"
          onclick={continueStep}
          disabled={!canContinue}>Continue</Button
        >
      {/if}
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
  .path-card:disabled {
    opacity: 0.45;
    cursor: not-allowed;
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
