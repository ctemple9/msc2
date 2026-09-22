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
  import { onDestroy, tick } from 'svelte';
  import Sheet from '../../../components/base/Sheet.svelte';
  import Button from '../../../components/base/Button.svelte';
  import {
    activeTourStep,
    tourJavaSelectionPending,
    tourServerContext,
    tourServerCreated,
  } from '../../../help/onboarding';
  import ConfigureStep from './ConfigureStep.svelte';
  import NetworkStep from './NetworkStep.svelte';
  import WorldStep from './WorldStep.svelte';
  import AddOnsStep from './AddOnsStep.svelte';
  import UploadStep from './UploadStep.svelte';
  import ReviewStep from './ReviewStep.svelte';
  import ConfirmStep from './ConfirmStep.svelte';
  import ModpackCreationSummarySheet from './ModpackCreationSummarySheet.svelte';
  import JavaInstallSheet from '../../server-editor/JavaInstallSheet.svelte';
  import { ApiError } from '../../../api/client';
  import {
    dispatchOnboardingAnchorAction,
    onboardingAnchor,
  } from '../../../help/tourAnchors';
  import type { Schema, ScreenApi } from '../../shared/types';
  import { errorMessage } from '../../shared/types';
  import { serverEditorPaths } from '../../server-editor/model';
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
    javaSelectionKey,
    ServerCreationError,
    modpackCreationSummary,
    versionsForCreatePath,
    wizardStepLabels,
    type ModpackCreationSummary,
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
  let modpackSummary: ModpackCreationSummary | undefined;
  let showModpackSummary = false;
  let showJavaRecovery = false;
  let showJavaInstall = false;
  let showJavaSelection = false;
  let javaSelectionLoading = false;
  let javaSelectionError = '';
  let javaRuntimes: Schema['JavaRuntimeDTO'][] = [];
  let javaSelectedPath = '';
  let javaInstallMode: 'selection' | 'recovery' = 'recovery';
  // Configure's Continue is intercepted by Java selection; defer its tour
  // action until a runtime is confirmed.
  let continueAfterJavaSelection = false;
  let javaRequiredMajor = 25;
  let javaMinecraftVersion = '';
  let javaFailureMessage = '';

  $: tourPathLocked = $activeTourStep === 'choose-path';
  $: if (tourPathLocked && path !== 'fresh') selectPath('fresh');
  $: tourServerContext.set({
    serverType: draft.serverType,
    javaCategory: draft.javaCategory,
    javaFlavor: draft.javaFlavor,
    enableCrossPlay: draft.enableCrossPlay,
  });

  onDestroy(() => {
    tourServerContext.set(null);
    tourServerCreated.set(false);
    tourJavaSelectionPending.set(false);
  });

  // `AddServerWizardView.swift`'s `hasAddOnsStep` -- inserts a sixth "Add-ons"
  // step at position 5 (Fresh path only) once the chosen Java flavor accepts
  // add-ons, shifting Confirm from 5 to 6. When it's false, position 5 is
  // Confirm directly and the layout is unchanged from before this step.
  $: showAddOns = path === 'fresh' && hasAddOnsStep(draft);
  $: showModpack = path === 'modpack' || draft.stagedModpack !== undefined;
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
    } else if (showModpack && draft.stagedModpack?.inspection.packName) {
      displayName = draft.stagedModpack.inspection.packName;
    } else if (path === 'importExisting' && draft.importSourcePath) {
      displayName = importDisplayNameFromPath(draft.importSourcePath);
    }
  }
  $: canContinue =
    currentStep === 1 ||
    (currentStep === 2 && path === 'fresh' && canAdvanceConfigure(draft)) ||
    (currentStep === 2 &&
      (path === 'importExisting' || path === 'modpack') &&
      canAdvanceUpload(draft)) ||
    (currentStep === 3 && path === 'fresh' && canAdvanceNetwork(draft)) ||
    (currentStep === 3 && showModpack && canAdvanceNetwork(draft)) ||
    // Review's own `canAdvance` case is unconditional in the oracle --
    // nothing on this step blocks Continue once it's reachable at all.
    (currentStep === 3 && path === 'importExisting' && !showModpack) ||
    (currentStep === 4 && path === 'fresh' && canAdvanceWorld(draft)) ||
    (currentStep === 4 && showModpack && canAdvanceWorld(draft)) ||
    (currentStep === 4 && path === 'importExisting' && !showModpack && canAdvanceNetwork(draft)) ||
    (currentStep === 5 && path === 'fresh' && showAddOns);

  function continueStep(): void {
    if (currentStep >= totalSteps || !canContinue) return;
    if (
      javaSelectionBelongsHere() &&
      !hasCurrentJavaSelection() &&
      !showJavaSelection &&
      !showJavaInstall
    ) {
      continueAfterJavaSelection = true;
      // The anchor action is raised by the same browser click. Set this before
      // opening the sheet so TourOverlay rejects that event synchronously.
      tourJavaSelectionPending.set(true);
      void openJavaSelection();
      return;
    }
    advanceWizardAndTour();
  }

  function advanceWizardAndTour(): void {
    currentStep += 1;
    // Continue remains spotlightable, but its automatic click announcement is
    // disabled. Only this completed wizard transition can advance the tour.
    void tick().then(() => dispatchOnboardingAnchorAction('ob_wizard_continue'));
  }

  function javaSelectionBelongsHere(): boolean {
    return draft.serverType === 'java' && currentStep === 2 && (path === 'fresh' || showModpack);
  }

  function hasCurrentJavaSelection(): boolean {
    return Boolean(draft.javaPath && draft.javaSelectionKey === javaSelectionKey(draft));
  }

  function handleImportScanned(): void {
    if (path === 'importExisting') currentStep = 3;
  }

  function selectPath(next: WizardPath): void {
    path = next;
    draft = {
      ...draft,
      stagedModpack: undefined,
      importSourcePath: undefined,
      importIsZip: false,
      importScan: undefined,
      javaPath: undefined,
      javaSelectionKey: undefined,
    };
  }

  function backStep(): void {
    if (currentStep > 1) currentStep -= 1;
  }

  async function beginCreate(): Promise<void> {
    if (!canCreateServer(displayName) || isCreating) return;
    if (draft.serverType === 'java' && !hasCurrentJavaSelection()) {
      await openJavaSelection();
      return;
    }
    isCreating = true;
    statusMessage =
      path === 'importExisting' && !showModpack ? 'Importing server…' : 'Creating server…';
    const onProgress = (line: string) => (statusMessage = line);
    try {
      let createdPackSummary: ModpackCreationSummary | undefined;
      if (path === 'importExisting' && !showModpack) {
        const result = await importServerFromDraft(api, draft, displayName, onProgress);
        createWarnings = result.warnings;
      } else {
        const result = await createServerFromDraft(api, draft, displayName, onProgress);
        createWarnings = result.warnings;
        createdPackSummary = result.modpackSummary;
      }
      modpackSummary = createdPackSummary;
      createSucceeded = true;
      tourServerCreated.set(true);
      onCreated();
      if (createdPackSummary) showModpackSummary = true;
    } catch (error) {
      if (!offerJavaRecovery(error)) statusMessage = errorMessage(error);
    } finally {
      isCreating = false;
    }
  }

  function requiredJavaMajor(message: string): number {
    const match = message.match(/needs Java (\d+)/i);
    const major = match ? Number(match[1]) : NaN;
    return [8, 17, 21, 25].includes(major) ? major : 25;
  }

  function requiredJavaMajorForVersion(version: string | undefined): number {
    const numericParts = (version ?? '')
      .split('.')
      .map((part) => Number(part))
      .filter((part) => Number.isFinite(part));
    const first = numericParts[0];
    if (first === 1) {
      const minor = numericParts[1] ?? 0;
      const patch = numericParts[2] ?? 0;
      if (minor >= 21 || (minor === 20 && patch >= 5)) return 21;
      if (minor >= 17) return 17;
      return 8;
    }
    return 25;
  }

  async function resolveJavaMinecraftVersion(): Promise<string | undefined> {
    const packVersion = draft.stagedModpack?.inspection.minecraftVersion?.trim();
    if (packVersion) return packVersion;
    if (!api || draft.serverType !== 'java') return undefined;
    const response = await api.get<Schema['VersionsResponseDTO']>(
      versionsForCreatePath('java', draft.javaFlavor),
    );
    const entry = draft.versionId
      ? response.versions?.find((candidate) => candidate.id === draft.versionId)
      : (response.versions?.find((candidate) => candidate.isLatest) ?? response.versions?.[0]);
    return entry?.mcVersion;
  }

  async function openJavaSelection(): Promise<void> {
    if (!api || javaSelectionLoading) return;
    showJavaSelection = true;
    javaSelectionLoading = true;
    javaSelectionError = '';
    javaSelectedPath = '';
    try {
      const minecraftVersion = await resolveJavaMinecraftVersion();
      javaMinecraftVersion = minecraftVersion ?? 'the selected Minecraft version';
      javaRequiredMajor = requiredJavaMajorForVersion(minecraftVersion);
      const response = await api.get<Schema['JavaRuntimesResponseDTO']>(
        serverEditorPaths.javaRuntimes,
      );
      javaRuntimes = response.runtimes ?? [];
    } catch (error) {
      javaSelectionError = errorMessage(error);
      javaRuntimes = [];
    } finally {
      javaSelectionLoading = false;
    }
  }

  function runtimeCanRun(runtime: Schema['JavaRuntimeDTO']): boolean {
    return (runtime.majorVersion ?? 0) >= javaRequiredMajor;
  }

  function chooseJavaRuntime(runtime: Schema['JavaRuntimeDTO']): void {
    if (runtimeCanRun(runtime)) javaSelectedPath = runtime.executablePath;
  }

  function closeJavaSelection(): void {
    if (!javaSelectionLoading) {
      showJavaSelection = false;
      continueAfterJavaSelection = false;
      tourJavaSelectionPending.set(false);
    }
  }

  function confirmJavaSelection(): void {
    if (!javaSelectedPath) return;
    draft = {
      ...draft,
      javaPath: javaSelectedPath,
      javaSelectionKey: javaSelectionKey(draft),
    };
    showJavaSelection = false;
    if (continueAfterJavaSelection) {
      continueAfterJavaSelection = false;
      advanceWizardAndTour();
      if ($activeTourStep === 'server-settings') {
        tourJavaSelectionPending.set(false);
      } else {
        tourJavaSelectionPending.set(false);
      }
    } else {
      tourJavaSelectionPending.set(false);
    }
  }

  function openJavaInstallFromSelection(): void {
    javaInstallMode = 'selection';
    showJavaSelection = false;
    showJavaInstall = true;
  }

  function offerJavaRecovery(error: unknown): boolean {
    const code =
      error instanceof ServerCreationError
        ? error.code
        : error instanceof ApiError
          ? error.error.code
          : undefined;
    if (code !== 'unusable_java_runtime') {
      return false;
    }
    const message = error instanceof ApiError ? error.error.message : errorMessage(error);
    javaFailureMessage = message;
    javaRequiredMajor = requiredJavaMajor(message);
    showJavaRecovery = true;
    statusMessage = '';
    return true;
  }

  function cancelJavaRecovery(): void {
    showJavaRecovery = false;
    javaFailureMessage = '';
  }

  function openJavaInstall(): void {
    javaInstallMode = 'recovery';
    showJavaRecovery = false;
    showJavaInstall = true;
  }

  function closeJavaInstall(): void {
    showJavaInstall = false;
    if (javaInstallMode === 'selection') showJavaSelection = true;
  }

  async function selectInstalledJava(event: {
    major: number;
    runtimePath?: string;
  }): Promise<void> {
    if (!event.runtimePath) {
      throw new Error(`Java ${event.major} installed, but MSC could not select its runtime path.`);
    }
    javaSelectedPath = event.runtimePath;
    draft = {
      ...draft,
      javaPath: event.runtimePath,
      javaSelectionKey: javaSelectionKey(draft),
    };
  }
</script>

<Sheet
  title="Add Server"
  size="lg"
  onClose={isCreating || showJavaRecovery || showJavaSelection || showJavaInstall
    ? undefined
    : onClose}
>
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
          <p>Bring in an existing server or modpack, or start from scratch.</p>
        </div>
        <div class="paths" use:onboardingAnchor={'ob_wizard_path_picker'}>
          <button
            type="button"
            class="path-card"
            class:selected={path === 'importExisting'}
            disabled={tourPathLocked}
            onclick={() => selectPath('importExisting')}
          >
            <span class="path-title">Import or Create from Modpack</span>
            <span class="path-subtitle"
              >Drop a server folder or .zip to import it, or choose a .mrpack or CurseForge archive
              to create a new server from its pinned files.</span
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
      {:else if currentStep === 2 && (path === 'importExisting' || path === 'modpack')}
        <UploadStep {api} bind:draft onScanned={handleImportScanned} />
      {:else if currentStep === 3 && path === 'fresh'}
        <NetworkStep bind:draft />
      {:else if currentStep === 3 && showModpack}
        <NetworkStep bind:draft />
      {:else if currentStep === 3 && path === 'importExisting'}
        <ReviewStep bind:draft />
      {:else if currentStep === 4 && path === 'fresh'}
        <WorldStep {api} bind:draft />
      {:else if currentStep === 4 && showModpack}
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
        {#if javaSelectionBelongsHere() && !hasCurrentJavaSelection()}
          <Button variant="primary" onclick={continueStep} disabled={!canContinue}
            >Continue</Button
          >
        {:else}
          <Button
            variant="primary"
            anchorId="ob_wizard_continue"
            announceOnboardingAction={false}
            onclick={continueStep}
            disabled={!canContinue}>Continue</Button
          >
        {/if}
      {/if}
    </div>
  </div>
</Sheet>

{#if showModpackSummary && modpackSummary}
  <ModpackCreationSummarySheet
    {api}
    bind:summary={modpackSummary}
    onClose={() => (showModpackSummary = false)}
  />
{/if}

{#if showJavaRecovery}
  <Sheet title="Java version required" size="sm" onClose={cancelJavaRecovery}>
    <div class="recovery-copy">
      <p class="recovery-title">This Minecraft version needs Java {javaRequiredMajor}.</p>
      <p>{javaFailureMessage}</p>
      <p>
        Download Java {javaRequiredMajor} from Adoptium now, and MSC will select it for this host. Your
        server choices are still here when you return.
      </p>
    </div>
    <div class="recovery-actions">
      <Button variant="secondary" onclick={cancelJavaRecovery}>Cancel</Button>
      <Button variant="primary" onclick={openJavaInstall}>Download Java {javaRequiredMajor}</Button>
    </div>
  </Sheet>
{/if}

{#if showJavaInstall}
  <JavaInstallSheet
    {api}
    initialMajor={javaRequiredMajor}
    selectionScope="this server"
    onClose={closeJavaInstall}
    onInstalled={selectInstalledJava}
  />
{/if}

{#if showJavaSelection}
  <Sheet title="Choose Java for this server" size="sm" onClose={closeJavaSelection}>
    <p class="selection-explain">
      {javaMinecraftVersion} uses Java {javaRequiredMajor}. Select the runtime MSC should use for
      this server. This choice does not change Java for your other servers.
    </p>
    {#if javaSelectionLoading}
      <p class="selection-explain">Detecting installed Java runtimes…</p>
    {:else if javaSelectionError}
      <p class="selection-explain warn">{javaSelectionError}</p>
    {:else if javaRuntimes.length === 0}
      <p class="selection-explain warn">
        No Java runtimes were detected. Install Java {javaRequiredMajor}, then return here to select
        it.
      </p>
    {:else}
      <div class="runtime-list" role="radiogroup" aria-label="Java runtime for this server">
        {#each javaRuntimes as runtime (runtime.executablePath)}
          <button
            type="button"
            class="runtime-choice"
            class:selected={javaSelectedPath === runtime.executablePath}
            class:incompatible={!runtimeCanRun(runtime)}
            disabled={!runtimeCanRun(runtime)}
            onclick={() => chooseJavaRuntime(runtime)}
          >
            <span class="runtime-copy">
              <span class="runtime-heading">
                <span class="runtime-version"
                  >{runtime.majorVersion ? `Java ${runtime.majorVersion}` : 'Java'}</span
                >
                <span class="runtime-name">{runtime.name}</span>
              </span>
              <span class="runtime-path">{runtime.executablePath}</span>
            </span>
            <span class="runtime-action"
              >{runtimeCanRun(runtime) ? 'Select' : 'Needs newer Java'}</span
            >
          </button>
        {/each}
      </div>
    {/if}
    <div class="selection-footer">
      <Button variant="secondary" onclick={openJavaInstallFromSelection}>Install Java…</Button>
      <span class="selection-spacer"></span>
      <Button variant="secondary" onclick={closeJavaSelection}>Cancel</Button>
      <Button
        variant="primary"
        disabled={!javaSelectedPath || javaSelectionLoading}
        onclick={confirmJavaSelection}>Use selected Java</Button
      >
    </div>
  </Sheet>
{/if}

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

  .recovery-copy {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .recovery-copy p {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
  .recovery-copy .recovery-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .recovery-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 20px;
  }

  .selection-explain {
    margin: 0 0 12px;
    font-size: 12px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
  .selection-explain.warn {
    color: var(--msc2-status-warn);
  }
  .runtime-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 280px;
    overflow-y: auto;
    margin-bottom: 14px;
  }
  .runtime-choice {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
    padding: 11px 14px;
    background: var(--msc2-tier-chrome);
    border: 1px solid transparent;
    border-radius: 8px;
    color: var(--msc2-text-primary);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .runtime-choice.selected {
    border-color: rgba(255, 255, 255, 0.32);
  }
  .runtime-choice.incompatible {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .runtime-copy {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .runtime-heading {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .runtime-version {
    font-weight: 500;
  }
  .runtime-name,
  .runtime-path {
    overflow: hidden;
    color: var(--msc2-text-tertiary);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .runtime-name {
    font-size: 11px;
  }
  .runtime-path {
    font-family: var(--msc2-font-mono, monospace);
    font-size: 10.5px;
  }
  .runtime-action {
    flex-shrink: 0;
    color: var(--msc2-text-tertiary);
    font-size: 10px;
  }
  .selection-footer {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .selection-spacer {
    flex: 1;
  }
</style>
