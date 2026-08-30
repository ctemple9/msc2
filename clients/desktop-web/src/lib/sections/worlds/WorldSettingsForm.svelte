<script lang="ts">
  // One world-local form shared by the Add Server wizard and the Worlds tab.
  // The profile route owns the actual capability/lifecycle decision; this
  // component only makes that decision legible instead of guessing that a
  // Minecraft setting applies everywhere.
  import Field from '../../components/base/Field.svelte';
  import Select from '../../components/base/Select.svelte';
  import Toggle from '../../components/base/Toggle.svelte';
  import {
    WORLD_DIFFICULTY_OPTIONS,
    WORLD_GAMEMODE_OPTIONS,
    WORLD_TYPE_OPTIONS,
    profileFieldIsReadOnly,
    profileFieldIsUnavailable,
    profileFieldUnavailableReason,
    type WorldProfileFieldMetadata,
    type WorldServerType,
    type WorldSettingsCapabilities,
    type WorldSettingsValues,
  } from './model';

  export let values: WorldSettingsValues;
  export let serverType: WorldServerType;
  /** `wizard` is the first-world form; `create` is the post-slot form, where
   * the complete profile can be saved; `edit` additionally locks
   * creation-only values. */
  export let mode: 'wizard' | 'create' | 'edit' = 'edit';
  export let metadata: Record<string, WorldProfileFieldMetadata> = {};
  export let capabilities: WorldSettingsCapabilities | undefined = undefined;
  export let heading: string | undefined = undefined;
  export let serverSettingsHref: string | undefined = undefined;
  export let onChange: ((values: WorldSettingsValues) => void) | undefined = undefined;

  let lastPublishedValues = '';

  const javaFields = new Set([
    'generation.flat-preset',
    'generation.biome-source',
    'generation.generator-options',
    'generation.data-packs',
    'gameplay.hardcore',
    'gameplay.commands',
  ]);
  const bedrockFields = new Set([
    'gameplay.cheats',
    'gameplay.experiments',
    'gameplay.coordinates',
    'gameplay.starting-map',
    'gameplay.supported-toggles',
  ]);

  function update(patch: Partial<WorldSettingsValues>): void {
    values = { ...values, ...patch, ...(capabilities ? { capabilities } : {}) };
  }

  $: if (capabilities && values.capabilities !== capabilities) {
    values = { ...values, capabilities };
  }

  // Field and Select are deliberately small presentational primitives. Their
  // two-way bindings update this component's draft; this single publisher
  // keeps the same draft available to both parent flows without adding a new
  // input primitive just for this form.
  $: {
    const serialized = JSON.stringify(values);
    if (serialized !== lastPublishedValues) {
      lastPublishedValues = serialized;
      onChange?.(values);
    }
  }

  function unavailable(key: string): boolean {
    return profileFieldIsUnavailable(key, serverType, mode, metadata, capabilities);
  }

  function reason(key: string): string | undefined {
    return profileFieldUnavailableReason(key, serverType, mode, metadata, capabilities);
  }

  function readOnly(key: string): boolean {
    return profileFieldIsReadOnly(key, mode, metadata);
  }

  function fieldNote(key: string): string | undefined {
    if (unavailable(key)) return reason(key);
    if (readOnly(key))
      return 'Used only when a new world is generated. Create a new world to change it.';
    const lifecycle = metadata[key]?.lifecycle;
    if (lifecycle === 'restart_required') return 'Saved now; applies after the server restarts.';
    if (lifecycle === 'apply_on_activation') return 'Used when this world becomes active.';
    if (metadata[key]?.valueState === 'unknown') return 'The agent could not verify this value.';
    return undefined;
  }

  function optionList(
    options: readonly { value: string; label: string }[],
    value: string,
  ): readonly { value: string; label: string }[] {
    return value ? options : [{ value: '', label: 'Unknown / not detected' }, ...options];
  }

  function hasEditionField(fields: Set<string>): boolean {
    return [...fields].some((key) => !unavailable(key));
  }

  function hasSupportedToggles(): boolean {
    return Object.keys(values.supportedToggles).length > 0;
  }

  function unknownFieldNotice(): string | undefined {
    const unknown = Object.entries(metadata).some(
      ([key, field]) => key.startsWith('unknown.') || field.valueState === 'unknown',
    );
    return unknown
      ? 'Some properties are unknown to this MSC version. They remain preserved and are not rewritten.'
      : undefined;
  }

  $: creativeSelected = values.defaultGameMode === 'creative';
  $: bedrockSafetyRisk =
    serverType === 'bedrock' &&
    (creativeSelected ||
      values.cheats === true ||
      Object.values(values.supportedToggles).some(Boolean));
</script>

<div class="form">
  {#if heading}
    <div class="intro">
      <h2>{heading}</h2>
      <p>Choose the defaults that travel with this world slot.</p>
    </div>
  {/if}

  {#if capabilities}
    <section class="context" aria-label="Advanced settings context">
      <p class="context-label">Advanced settings context</p>
      <p class="context-value">
        {capabilities.context.serverType === 'bedrock' ? 'Bedrock' : 'Java'}
        · {capabilities.context.minecraftVersion ??
          (capabilities.context.serverType === 'bedrock'
            ? 'verified runtime'
            : 'Minecraft version not selected')}
        {#if capabilities.context.javaFlavor}
          · {capabilities.context.javaFlavor}
        {/if}
        {#if capabilities.context.loaderVersion}
          · loader {capabilities.context.loaderVersion}
        {/if}
      </p>
      {#if capabilities.context.javaRuntime}
        <p class="hint">
          Java runtime: {capabilities.context.javaRuntime.state}
          {#if capabilities.context.javaRuntime.reason}
            — {capabilities.context.javaRuntime.reason}
          {/if}
        </p>
      {/if}
    </section>
  {/if}

  <p class="ownership">
    These settings are saved with this world. Server settings—ports, player limits, access, MOTD,
    runtime, and network helpers—apply to every world.
    {#if serverSettingsHref}
      <a href={serverSettingsHref}>Open Server Settings</a>
    {/if}
  </p>

  {#if mode !== 'wizard' && capabilities?.thirdParty}
    <p class="boundary" role="note">
      <span class="boundary-label">{capabilities.thirdParty.label}</span>
      {capabilities.thirdParty.message}
      {#if serverSettingsHref && capabilities.thirdParty.handoff === 'server_settings'}
        <a href={serverSettingsHref}>Open Server Settings</a>
      {/if}
    </p>
  {/if}

  {#if unknownFieldNotice()}
    <p class="boundary" role="status">{unknownFieldNotice()}</p>
  {/if}

  <section class="essentials">
    <p class="section-title">Essentials</p>
    <div class="field-grid">
      <label class="field-group wide">
        <span class="label">World Name</span>
        <Field bind:value={values.name} placeholder="e.g. Survival World" />
        <span class="hint"
          >The display name for this world slot, separate from the server name.</span
        >
      </label>

      <label class="field-group">
        <span class="label">Seed</span>
        <Field
          bind:value={values.seed}
          placeholder="Optional — random if blank"
          disabled={readOnly('identity.seed')}
        />
        {#if fieldNote('identity.seed')}
          <span class="hint">{fieldNote('identity.seed')}</span>
        {:else}
          <span class="hint">Used only the first time this world generates terrain.</span>
        {/if}
      </label>

      <label class="field-group">
        <span class="label">Difficulty</span>
        <Select
          options={optionList(WORLD_DIFFICULTY_OPTIONS, values.difficulty)}
          value={values.difficulty}
          disabled={unavailable('gameplay.difficulty') || readOnly('gameplay.difficulty')}
          onchange={(value) => update({ difficulty: value })}
        />
        {#if fieldNote('gameplay.difficulty')}<span class="hint"
            >{fieldNote('gameplay.difficulty')}</span
          >{/if}
      </label>

      <label class="field-group">
        <span class="label">Default Game Mode</span>
        <Select
          options={optionList(WORLD_GAMEMODE_OPTIONS, values.defaultGameMode)}
          value={values.defaultGameMode}
          disabled={unavailable('gameplay.default-game-mode') ||
            readOnly('gameplay.default-game-mode')}
          onchange={(value) => update({ defaultGameMode: value })}
        />
        {#if fieldNote('gameplay.default-game-mode')}
          <span class="hint">{fieldNote('gameplay.default-game-mode')}</span>
        {/if}
      </label>
    </div>
  </section>

  {#if creativeSelected}
    <p class="safety-note" role="note">
      Creative changes the world's gameplay rules. The agent will require an acknowledgement before
      applying it; on Bedrock, achievements may be permanently disabled for this world.
    </p>
  {/if}

  <details class="disclosure">
    <summary>
      <span>
        <span class="disclosure-title">World Generation</span>
        <span class="disclosure-subtitle">Terrain, folder, and first-creation options</span>
      </span>
      <span class="chevron" aria-hidden="true">⌄</span>
    </summary>
    <div class="disclosure-body">
      <div class="field-grid">
        <label class="field-group">
          <span class="label">Minecraft Folder Name</span>
          {#if unavailable('identity.level-name')}
            <span class="unavailable">Unavailable: {reason('identity.level-name')}</span>
          {:else}
            <Field
              bind:value={values.levelName}
              placeholder="Use the default folder name"
              disabled={readOnly('identity.level-name')}
            />
            {#if fieldNote('identity.level-name')}<span class="hint"
                >{fieldNote('identity.level-name')}</span
              >{/if}
          {/if}
        </label>

        <label class="field-group">
          <span class="label">World Type</span>
          {#if unavailable('generation.world-type')}
            <span class="unavailable">Unavailable: {reason('generation.world-type')}</span>
          {:else}
            <Select
              options={optionList(WORLD_TYPE_OPTIONS, values.worldType)}
              value={values.worldType}
              disabled={readOnly('generation.world-type')}
              onchange={(value) => update({ worldType: value })}
            />
            {#if fieldNote('generation.world-type')}<span class="hint"
                >{fieldNote('generation.world-type')}</span
              >{/if}
          {/if}
        </label>

        {#if serverType === 'java' || !unavailable('generation.flat-preset')}
          <label
            class="field-group"
            class:unavailable-group={unavailable('generation.flat-preset')}
          >
            <span class="label">Flat Preset</span>
            {#if unavailable('generation.flat-preset')}
              <span class="unavailable">Unavailable: {reason('generation.flat-preset')}</span>
            {:else}
              <Field
                bind:value={values.flatPreset}
                placeholder="Optional"
                disabled={readOnly('generation.flat-preset')}
              />
              {#if fieldNote('generation.flat-preset')}<span class="hint"
                  >{fieldNote('generation.flat-preset')}</span
                >{/if}
            {/if}
          </label>
        {/if}

        {#if serverType === 'java' || !unavailable('generation.biome-source')}
          <label class="field-group">
            <span class="label">Biome Source</span>
            {#if unavailable('generation.biome-source')}
              <span class="unavailable">Unavailable: {reason('generation.biome-source')}</span>
            {:else}
              <Field
                bind:value={values.biomeSource}
                placeholder="Default"
                disabled={readOnly('generation.biome-source')}
              />
              {#if fieldNote('generation.biome-source')}<span class="hint"
                  >{fieldNote('generation.biome-source')}</span
                >{/if}
            {/if}
          </label>
        {/if}

        <label class="field-group">
          <span class="label">Structures</span>
          {#if unavailable('generation.structures')}
            <span class="unavailable">Unavailable: {reason('generation.structures')}</span>
          {:else}
            <Toggle
              checked={values.structures === true}
              label="Generate structures"
              disabled={readOnly('generation.structures')}
              onchange={(checked) => update({ structures: checked })}
            />
            {#if fieldNote('generation.structures')}<span class="hint"
                >{fieldNote('generation.structures')}</span
              >{/if}
          {/if}
        </label>

        <label class="field-group">
          <span class="label">Bonus Chest</span>
          {#if unavailable('generation.bonus-chest')}
            <span class="unavailable">Unavailable: {reason('generation.bonus-chest')}</span>
          {:else}
            <Toggle
              checked={values.bonusChest === true}
              label="Generate a bonus chest"
              disabled={readOnly('generation.bonus-chest')}
              onchange={(checked) => update({ bonusChest: checked })}
            />
            {#if fieldNote('generation.bonus-chest')}<span class="hint"
                >{fieldNote('generation.bonus-chest')}</span
              >{/if}
          {/if}
        </label>

        {#if serverType === 'java' || !unavailable('generation.generator-options')}
          <label class="field-group wide">
            <span class="label">Generator Options</span>
            {#if unavailable('generation.generator-options')}
              <span class="unavailable">Unavailable: {reason('generation.generator-options')}</span>
            {:else}
              <Field
                bind:value={values.generatorOptions}
                placeholder="Optional generator payload"
                multiline
                disabled={readOnly('generation.generator-options')}
              />
              {#if fieldNote('generation.generator-options')}<span class="hint"
                  >{fieldNote('generation.generator-options')}</span
                >{/if}
            {/if}
          </label>
        {/if}

        {#if serverType === 'java' || !unavailable('generation.data-packs')}
          <label class="field-group wide">
            <span class="label">Data Packs</span>
            {#if unavailable('generation.data-packs')}
              <span class="unavailable">Unavailable: {reason('generation.data-packs')}</span>
            {:else}
              <Field
                bind:value={values.dataPacks}
                placeholder="One pack name per line"
                multiline
                disabled={readOnly('generation.data-packs')}
              />
              {#if fieldNote('generation.data-packs')}<span class="hint"
                  >{fieldNote('generation.data-packs')}</span
                >{/if}
            {/if}
          </label>
        {/if}
      </div>
    </div>
  </details>

  <details class="disclosure">
    <summary>
      <span>
        <span class="disclosure-title">Gameplay Rules</span>
        <span class="disclosure-subtitle"
          >Commands, gamerules, cheats, and edition-specific toggles</span
        >
      </span>
      <span class="chevron" aria-hidden="true">⌄</span>
    </summary>
    <div class="disclosure-body">
      <div class="field-grid">
        {#if hasEditionField(javaFields)}
          <label class="field-group">
            <span class="label">Hardcore</span>
            {#if unavailable('gameplay.hardcore')}
              <span class="unavailable">Unavailable: {reason('gameplay.hardcore')}</span>
            {:else}
              <Toggle
                checked={values.hardcore === true}
                label="Hardcore mode"
                disabled={readOnly('gameplay.hardcore')}
                onchange={(checked) => update({ hardcore: checked })}
              />
              {#if fieldNote('gameplay.hardcore')}<span class="hint"
                  >{fieldNote('gameplay.hardcore')}</span
                >{/if}
            {/if}
          </label>

          <label class="field-group">
            <span class="label">Allow Commands</span>
            {#if unavailable('gameplay.commands')}
              <span class="unavailable">Unavailable: {reason('gameplay.commands')}</span>
            {:else}
              <Toggle
                checked={values.commands === true}
                label="Allow commands"
                disabled={readOnly('gameplay.commands')}
                onchange={(checked) => update({ commands: checked })}
              />
              {#if fieldNote('gameplay.commands')}<span class="hint"
                  >{fieldNote('gameplay.commands')}</span
                >{/if}
            {/if}
          </label>
        {/if}

        <label class="field-group wide">
          <span class="label">Gamerules</span>
          {#if unavailable('gameplay.gamerules')}
            <span class="unavailable">Unavailable: {reason('gameplay.gamerules')}</span>
          {:else}
            <Field
              bind:value={values.gamerules}
              placeholder="keepInventory=true — one rule per line"
              multiline
              disabled={readOnly('gameplay.gamerules')}
            />
            <span class="hint"
              >Use one rule=value pair per line; unrecognized rules are preserved.</span
            >
          {/if}
        </label>

        {#if hasEditionField(bedrockFields)}
          <label class="field-group">
            <span class="label">Cheats</span>
            {#if unavailable('gameplay.cheats')}
              <span class="unavailable">Unavailable: {reason('gameplay.cheats')}</span>
            {:else}
              <Toggle
                checked={values.cheats === true}
                label="Enable cheats"
                disabled={readOnly('gameplay.cheats')}
                onchange={(checked) => update({ cheats: checked })}
              />
              {#if fieldNote('gameplay.cheats')}<span class="hint"
                  >{fieldNote('gameplay.cheats')}</span
                >{/if}
            {/if}
          </label>

          <label class="field-group">
            <span class="label">Coordinates</span>
            {#if unavailable('gameplay.coordinates')}
              <span class="unavailable">Unavailable: {reason('gameplay.coordinates')}</span>
            {:else}
              <Toggle
                checked={values.coordinates === true}
                label="Show coordinates"
                disabled={readOnly('gameplay.coordinates')}
                onchange={(checked) => update({ coordinates: checked })}
              />
              {#if fieldNote('gameplay.coordinates')}<span class="hint"
                  >{fieldNote('gameplay.coordinates')}</span
                >{/if}
            {/if}
          </label>

          <label class="field-group">
            <span class="label">Starting Map</span>
            {#if unavailable('gameplay.starting-map')}
              <span class="unavailable">Unavailable: {reason('gameplay.starting-map')}</span>
            {:else}
              <Toggle
                checked={values.startingMap === true}
                label="Give players a starting map"
                disabled={readOnly('gameplay.starting-map')}
                onchange={(checked) => update({ startingMap: checked })}
              />
              {#if fieldNote('gameplay.starting-map')}<span class="hint"
                  >{fieldNote('gameplay.starting-map')}</span
                >{/if}
            {/if}
          </label>

          <label class="field-group wide">
            <span class="label">Experiments</span>
            {#if unavailable('gameplay.experiments')}
              <span class="unavailable">Unavailable: {reason('gameplay.experiments')}</span>
            {:else}
              <Field
                bind:value={values.experiments}
                placeholder="Experiment name=true — one per line"
                multiline
                disabled={readOnly('gameplay.experiments')}
              />
              {#if fieldNote('gameplay.experiments')}<span class="hint"
                  >{fieldNote('gameplay.experiments')}</span
                >{/if}
            {/if}
          </label>
        {/if}

        {#if hasSupportedToggles()}
          {#each Object.entries(values.supportedToggles) as [key, enabled] (key)}
            <label class="field-group">
              <span class="label">{key}</span>
              <Toggle
                checked={enabled}
                label={key}
                disabled={unavailable('gameplay.supported-toggles') ||
                  readOnly('gameplay.supported-toggles')}
                onchange={(checked) =>
                  update({ supportedToggles: { ...values.supportedToggles, [key]: checked } })}
              />
              {#if fieldNote('gameplay.supported-toggles')}
                <span class="hint">{fieldNote('gameplay.supported-toggles')}</span>
              {/if}
            </label>
          {/each}
        {:else if mode !== 'wizard'}
          <p class="hint wide">No additional gameplay toggles are advertised by this server.</p>
        {/if}
      </div>
    </div>
  </details>

  {#if bedrockSafetyRisk}
    <p class="safety-note" role="note">
      Bedrock safety warning: Creative, cheats, and some experiments may permanently disable Xbox
      achievements for this world, even if you turn them off later. Saving will ask for a separate
      acknowledgement.
    </p>
  {/if}
</div>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .intro {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .intro h2 {
    margin: 0;
    color: var(--msc2-text-primary);
    font-size: 15px;
    font-weight: 600;
  }
  .intro p,
  .ownership,
  .context-value,
  .boundary,
  .hint,
  .safety-note {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.5;
  }
  .intro p,
  .ownership,
  .hint {
    color: var(--msc2-text-tertiary);
  }
  .context {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 10px 0;
    border-top: 1px solid var(--msc2-hairline-subtle);
    border-bottom: 1px solid var(--msc2-hairline-subtle);
  }
  .context-label,
  .context-value {
    margin: 0;
  }
  .context-label,
  .boundary-label {
    color: var(--msc2-text-secondary);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.07em;
    text-transform: uppercase;
  }
  .context-value {
    color: var(--msc2-text-primary);
    font-size: 12px;
  }
  .boundary {
    padding-left: 10px;
    border-left: 2px solid var(--msc2-hairline-field);
    color: var(--msc2-text-tertiary);
  }
  .boundary-label {
    margin-right: 5px;
  }
  .boundary a {
    margin-left: 4px;
    color: var(--msc2-text-secondary);
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .ownership a {
    margin-left: 4px;
    color: var(--msc2-text-secondary);
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .essentials {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .section-title,
  .label {
    color: var(--msc2-text-primary);
    font-size: 12px;
    font-weight: 500;
  }
  .section-title {
    margin: 0;
  }
  .field-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px 18px;
  }
  .field-group {
    display: flex;
    min-width: 0;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
  }
  .field-group.wide,
  .hint.wide {
    grid-column: 1 / -1;
  }
  .field-group :global(.field),
  .field-group :global(.wrap) {
    width: 100% !important;
  }
  .field-group :global(.track) {
    margin-top: 2px;
  }
  .disclosure {
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .disclosure summary {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 0;
    color: var(--msc2-text-primary);
    cursor: pointer;
    list-style: none;
  }
  .disclosure summary::-webkit-details-marker {
    display: none;
  }
  .disclosure summary:focus-visible {
    outline: 1px solid var(--msc2-hairline-field-focus);
    outline-offset: 3px;
  }
  .disclosure-title,
  .disclosure-subtitle {
    display: block;
  }
  .disclosure-title {
    font-size: 13px;
    font-weight: 500;
  }
  .disclosure-subtitle {
    margin-top: 2px;
    color: var(--msc2-text-tertiary);
    font-size: 11px;
  }
  .chevron {
    color: var(--msc2-text-tertiary);
    font-size: 16px;
    transition: transform 120ms ease;
  }
  details[open] .chevron {
    transform: rotate(180deg);
  }
  .disclosure-body {
    padding: 2px 0 10px;
  }
  .unavailable {
    width: 100%;
    box-sizing: border-box;
    padding: 8px 10px;
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 8px;
    color: var(--msc2-text-tertiary);
    font-size: 11px;
    line-height: 1.45;
  }
  .safety-note {
    padding: 9px 11px;
    border: 1px solid var(--msc2-hairline-strong);
    border-radius: 8px;
    color: var(--msc2-text-secondary);
  }
  @media (max-width: 560px) {
    .field-grid {
      grid-template-columns: minmax(0, 1fr);
    }
    .field-group.wide,
    .hint.wide {
      grid-column: auto;
    }
  }
</style>
