<script lang="ts">
  // Ports CommandPaletteView.swift + its GuidedCommandBuilderView/ArgFieldView
  // (docs/msc2/antiAIslop.md). This sheet only builds a command string and
  // hands it to `onUse` -- it never sends anything itself. P12.10b wires it
  // (and the console's own send path) into ConsoleDock.
  //
  // Two MSC 1 affordances dropped, not silently: per-category icon+color
  // (rule #11 -- color is a shared, scarce resource, not a per-row rail;
  // grouping uses a plain overline header instead) and the `@AppStorage`
  // favorites/star system, a separate, smaller feature this step's own plan
  // text never named.
  //
  // MSC 1's player-argument field is one view with two states, not two
  // sheets: a row of tappable online-player chips plus a text field when
  // players are online, or just the text field when nobody is -- exactly
  // what Cameron's own reference screenshot (no players online) shows.
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import Field from '../../components/base/Field.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import {
    COMMAND_CATEGORIES,
    buildCommand,
    commandSyntaxHint,
    commandsFor,
    hasRequiredArgs,
    type CommandCategory,
    type CommandPlayerName,
    type MinecraftCommandDef,
  } from './model';

  export let serverType: string | undefined = undefined;
  export let onlinePlayers: readonly CommandPlayerName[] = [];
  export let onClose: () => void;
  export let onUse: (command: string) => void;

  let searchText = '';
  let category: CommandCategory | undefined;
  let selected: MinecraftCommandDef | undefined;
  let argValues: string[] = [];

  $: available = commandsFor(serverType);
  $: filtered = available.filter((def) => {
    if (category && def.category !== category) return false;
    const needle = searchText.trim().toLowerCase();
    if (!needle) return true;
    return (
      def.name.toLowerCase().includes(needle) || def.description.toLowerCase().includes(needle)
    );
  });
  $: showGrouped = !searchText.trim() && !category;
  $: builtCommand = selected ? buildCommand(selected, argValues) : '';

  function open(def: MinecraftCommandDef): void {
    if (hasRequiredArgs(def)) {
      selected = def;
      argValues = def.argumentSlots.map(() => '');
    } else {
      onUse(`/${def.name}`);
      onClose();
    }
  }

  function back(): void {
    selected = undefined;
    argValues = [];
  }

  function useSelected(): void {
    if (!selected) return;
    onUse(builtCommand);
    onClose();
  }

  async function copyPreview(): Promise<void> {
    await navigator.clipboard?.writeText(builtCommand);
  }
</script>

<Sheet title={selected ? `/${selected.name}` : 'Command Palette'} size="md" {onClose}>
  {#if selected}
    {@const definition = selected}
    <div class="builder">
      <Button variant="secondary" size="sm" onclick={back}>Back</Button>

      <div class="summary">
        <p class="description">{definition.description}</p>
        <p class="syntax">{commandSyntaxHint(definition)}</p>
      </div>

      {#each definition.argumentSlots as slot, index (index)}
        <div class="arg-field">
          <p class="arg-label">
            <span class="arg-index">Argument {index + 1} of {definition.argumentSlots.length}</span>
            <span>{slot.label}</span>
          </p>
          {#if slot.kind === 'player' && onlinePlayers.length}
            <div class="chip-row">
              {#each onlinePlayers as onlinePlayer (onlinePlayer.name)}
                <button
                  type="button"
                  class="pick-chip"
                  class:selected={argValues[index] === onlinePlayer.name}
                  onclick={() => (argValues[index] = onlinePlayer.name)}
                >
                  {onlinePlayer.name}
                </button>
              {/each}
            </div>
            <Field bind:value={argValues[index]} placeholder={slot.label} />
          {:else if slot.kind === 'keyword' && slot.options}
            <div class="chip-row">
              {#each slot.options as option (option)}
                <button
                  type="button"
                  class="pick-chip"
                  class:selected={argValues[index] === option}
                  onclick={() => (argValues[index] = option)}
                >
                  {option}
                </button>
              {/each}
            </div>
          {:else}
            <Field bind:value={argValues[index]} placeholder={slot.label} />
          {/if}
        </div>
      {/each}

      <div class="preview-block">
        <span class="msc2-type-overline">Preview</span>
        <div class="preview-row">
          <span class="preview-text">{builtCommand}</span>
          <button
            type="button"
            class="icon-action"
            title="Copy command"
            aria-label="Copy command"
            onclick={() => void copyPreview()}
          >
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" aria-hidden="true">
              <rect
                x="9"
                y="9"
                width="11"
                height="11"
                rx="1.5"
                stroke="currentColor"
                stroke-width="1.6"
              />
              <path
                d="M6 15H5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h9a1 1 0 0 1 1 1v1"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </button>
        </div>
      </div>

      <Button variant="primary" onclick={useSelected}>Use Command →</Button>
    </div>
  {:else}
    <div class="browse">
      <Field bind:value={searchText} placeholder="Search commands…" />

      <div class="chip-row">
        <button
          type="button"
          class="chip"
          class:active={!category}
          onclick={() => (category = undefined)}
        >
          All
        </button>
        {#each COMMAND_CATEGORIES as cat (cat)}
          <button
            type="button"
            class="chip"
            class:active={category === cat}
            onclick={() => (category = cat)}
          >
            {cat}
          </button>
        {/each}
      </div>

      <div class="command-list">
        {#if filtered.length === 0}
          <p class="empty">No commands match "{searchText}".</p>
        {:else if showGrouped}
          {#each COMMAND_CATEGORIES as cat (cat)}
            {@const items = filtered.filter((def) => def.category === cat)}
            {#if items.length}
              <p class="msc2-type-overline group-header">{cat}</p>
              {#each items as def (def.name)}
                <button type="button" class="command-row" onclick={() => open(def)}>
                  <span class="command-info">
                    <span class="command-name">
                      /{def.name}
                      {#if hasRequiredArgs(def)}<Icon name="chevron" size={10} />{/if}
                    </span>
                    <span class="command-description">{def.description}</span>
                  </span>
                </button>
              {/each}
            {/if}
          {/each}
        {:else}
          {#each filtered as def (def.name)}
            <button type="button" class="command-row" onclick={() => open(def)}>
              <span class="command-info">
                <span class="command-name">
                  /{def.name}
                  {#if hasRequiredArgs(def)}<Icon name="chevron" size={10} />{/if}
                </span>
                <span class="command-description">{def.description}</span>
              </span>
            </button>
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</Sheet>

<style>
  .browse,
  .builder {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .chip-row {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .chip {
    font-family: inherit;
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-text-secondary);
    background: var(--msc2-tier-content);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 999px;
    padding: 5px 12px;
    cursor: pointer;
  }
  .chip.active {
    color: var(--msc2-text-primary);
    background: var(--msc2-neutral-elevated);
    font-weight: 600;
  }
  .command-list {
    display: flex;
    flex-direction: column;
    max-height: 420px;
    overflow-y: auto;
  }
  .group-header {
    margin: 12px 0 4px;
  }
  .group-header:first-child {
    margin-top: 0;
  }
  .command-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 8px;
    width: 100%;
    box-sizing: border-box;
    background: transparent;
    border: none;
    border-radius: 8px;
    text-align: left;
    cursor: pointer;
    color: inherit;
    font: inherit;
  }
  .command-row:hover {
    background: rgba(255, 255, 255, 0.04);
  }
  .command-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .command-name {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-family: var(--msc2-font-mono);
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .command-name :global(svg) {
    color: var(--msc2-text-tertiary);
  }
  .command-description {
    font-size: 11px;
    color: var(--msc2-text-secondary);
  }
  .empty {
    padding: 32px 8px;
    text-align: center;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .summary {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px;
    background: var(--msc2-tier-content);
    border-radius: 10px;
  }
  .description {
    margin: 0;
    font-size: 13px;
    color: var(--msc2-text-secondary);
  }
  .syntax {
    margin: 0;
    font-family: var(--msc2-font-mono);
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .arg-field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .arg-label {
    margin: 0;
    display: flex;
    align-items: baseline;
    gap: 8px;
    font-size: 12px;
    font-weight: 500;
    color: var(--msc2-text-secondary);
  }
  .arg-index {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.6px;
    text-transform: uppercase;
    color: var(--msc2-text-tertiary);
  }
  .pick-chip {
    font-family: inherit;
    font-size: 12px;
    color: var(--msc2-text-secondary);
    background: var(--msc2-tier-content);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 999px;
    padding: 5px 10px;
    cursor: pointer;
  }
  .pick-chip.selected {
    color: var(--msc2-text-primary);
    background: rgba(59, 130, 246, 0.12);
    border-color: var(--msc2-selection);
    font-weight: 600;
  }
  .preview-block {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .preview-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    background: var(--msc2-tier-terminal);
    border: 1px solid var(--msc2-hairline-faint);
    border-radius: 8px;
  }
  .preview-text {
    flex: 1;
    min-width: 0;
    font-family: var(--msc2-font-mono);
    font-size: 12px;
    color: var(--msc2-text-primary);
    overflow-x: auto;
    white-space: nowrap;
  }
  .icon-action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    flex-shrink: 0;
    color: var(--msc2-text-tertiary);
    background: transparent;
    border: none;
    border-radius: 6px;
    cursor: pointer;
  }
  .icon-action:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--msc2-text-primary);
  }
</style>
