<script lang="ts">
  import Button from '../../components/base/Button.svelte';
  import Field from '../../components/base/Field.svelte';
  import Select from '../../components/base/Select.svelte';
  import NumberField from '../../components/base/NumberField.svelte';
  import Toggle from '../../components/base/Toggle.svelte';
  import type { Schema, ScreenApi } from '../shared/types';

  export let api: ScreenApi | undefined = undefined;
  export let serverType: 'java' | 'bedrock';
  export let minecraftVersion: string | undefined = undefined;
  export let activeServer = false;
  export let value = '';
  export let disabled = false;
  export let coordinates: boolean | null | undefined = undefined;
  export let onCoordinatesChange: ((checked: boolean) => void) | undefined = undefined;
  export let onchange: ((value: string) => void) | undefined = undefined;

  type Rule = Schema['GameruleDefinitionDTO'];
  let catalog: Schema['GameruleCatalogDTO'] | undefined;
  let requestKey = '';
  let lastApi: ScreenApi | undefined;
  let loading = false;
  let error = '';
  let manual = false;
  let browsing = false;
  let search = '';

  $: nextKey = JSON.stringify([serverType, minecraftVersion, activeServer]);
  $: shownVersion = catalog?.minecraftVersion ?? minecraftVersion;
  $: if (nextKey !== requestKey || api !== lastApi) {
    requestKey = nextKey;
    lastApi = api;
    catalog = undefined;
    if (api) void load(nextKey, api);
  }

  async function load(key: string, client: ScreenApi): Promise<void> {
    loading = true;
    error = '';
    const params = new URLSearchParams({ serverType });
    if (minecraftVersion) params.set('minecraftVersion', minecraftVersion);
    if (activeServer) params.set('activeServer', 'true');
    try {
      const response = await client.get<Schema['GameruleCatalogDTO']>(
        `/v1/catalog/gamerules?${params}`,
      );
      if (requestKey === key && lastApi === client) catalog = response;
    } catch (caught) {
      if (requestKey === key && lastApi === client)
        error = caught instanceof Error ? caught.message : 'Could not load gameplay rules.';
    } finally {
      if (requestKey === key && lastApi === client) loading = false;
    }
  }

  function canonical(id: string): string {
    return serverType === 'bedrock' ? id.toLowerCase() : id.replace(/^minecraft:/, '');
  }
  function entries(text: string): { id: string; value: string }[] {
    return text
      .split('\n')
      .filter((line) => line.trim())
      .map((line) => {
        const index = line.indexOf('=');
        return {
          id: (index < 0 ? line : line.slice(0, index)).trim(),
          value: index < 0 ? '' : line.slice(index + 1).trim(),
        };
      });
  }
  function publish(next: string): void {
    value = next;
  }
  $: onchange?.(value);
  function set(rule: Rule, next: string): void {
    if (serverType === 'bedrock' && rule.id === 'showcoordinates' && onCoordinatesChange) {
      remove(rule.id);
      onCoordinatesChange(next === 'true');
      return;
    }
    if (serverType === 'bedrock' && rule.id === 'locatorbar') remove('playerwaypoints');
    if (serverType === 'bedrock' && rule.id === 'playerwaypoints') remove('locatorbar');
    const lines = value
      .split('\n')
      .filter((line) => canonical(line.split('=')[0].trim()) !== canonical(rule.id));
    lines.push(`${rule.id}=${next}`);
    publish(lines.filter((line) => line.trim()).join('\n'));
  }
  function remove(id: string): void {
    publish(
      value
        .split('\n')
        .filter((line) => canonical(line.split('=')[0].trim()) !== canonical(id))
        .join('\n'),
    );
  }
  function invalid(rule: Rule, text: string): boolean {
    if (rule.type === 'boolean') return text !== 'true' && text !== 'false';
    if (rule.type === 'choice') return !rule.choices.includes(text);
    const number = Number(text);
    return (
      !text ||
      !Number.isInteger(number) ||
      number < (rule.minimum ?? -2147483648) ||
      number > (rule.maximum ?? 2147483647)
    );
  }
  $: selected = entries(value);
  $: displayEntries = selected.map((entry) => ({
    ...entry,
    value:
      serverType === 'bedrock' && canonical(entry.id) === 'showcoordinates' && coordinates != null
        ? String(coordinates)
        : entry.value,
    rule: catalog?.rules.find((rule) => canonical(rule.id) === canonical(entry.id)),
  }));
  $: unlisted = catalog?.available ? displayEntries.filter((entry) => !entry.rule) : [];
  $: available = (catalog?.rules ?? []).filter(
    (rule) =>
      !selected.some((entry) => canonical(entry.id) === canonical(rule.id)) &&
      `${rule.label} ${rule.id} ${rule.description}`.toLowerCase().includes(search.toLowerCase()),
  );
</script>

<div class="editor">
  <div class="toolbar">
    <div class="context">
      <span class="version">
        {shownVersion ? `Minecraft ${shownVersion}` : 'Minecraft version not verified'}
      </span>
      <span class="edition">{serverType === 'bedrock' ? 'Bedrock' : 'Java'}</span>
    </div>
    <div class="actions">
      {#if catalog?.available}
        <Button
          size="sm"
          variant="secondary"
          {disabled}
          onclick={() => {
            browsing = !browsing;
            manual = false;
          }}>Add Rule</Button
        >
      {/if}
      <Button
        size="sm"
        variant="secondary"
        {disabled}
        onclick={() => {
          manual = !manual;
          browsing = false;
        }}>{manual ? 'Show Selected Rules' : 'Edit as Text'}</Button
      >
    </div>
  </div>
  {#if loading}<p class="hint" role="status">Loading rules for this release…</p>
  {:else if error}<p class="hint" role="status">{error} Manual entry is available.</p>
  {:else if catalog?.note && !catalog.note.startsWith('Verified built-in rules for this ')}<p
      class="hint"
    >
      {catalog.note}
    </p>
  {:else if !api}<p class="hint">
      Connect to an agent to browse rules. Manual entry is available.
    </p>{/if}

  {#if manual}
    <Field
      bind:value
      multiline
      {disabled}
      placeholder={serverType === 'bedrock' ? 'keepinventory=true' : 'keepInventory=true'}
    />
    <p class="hint">One rule=value pair per line. Unlisted rules are preserved.</p>
  {:else}
    {#if selected.length === 0}<p class="hint">
        No rule overrides. Minecraft uses this world's existing rules or its defaults.
      </p>{/if}
    {#each displayEntries as entry, index (`${entry.id}-${index}`)}
      {@const rule = entry.rule}
      <div class="rule-row">
        <div class="rule-info">
          <span class="rule-name">{rule?.label ?? entry.id}</span>
          <span class="hint"
            >{rule?.description ?? 'Not listed for this release. Preserved as a manual rule.'}</span
          >
          <code>{entry.id}</code>
          {#if rule && invalid(rule, entry.value)}<span class="invalid"
              >Invalid value for this rule.</span
            >{/if}
          {#if rule?.experimental}<span class="hint"
              >Requires the matching Minecraft experiment.</span
            >{/if}
        </div>
        <div class="actions">
          {#if rule?.type === 'boolean'}
            <Toggle
              checked={entry.value === 'true'}
              label={rule.label}
              {disabled}
              onchange={(checked) => set(rule, String(checked))}
            />
          {:else if rule?.type === 'choice'}
            <Select
              value={entry.value}
              options={rule.choices.map((choice) => ({ value: choice, label: choice }))}
              ariaLabel={rule.label}
              width="130px"
              {disabled}
              onchange={(text) => set(rule, text)}
            />
          {:else if rule?.type === 'integer'}
            <NumberField
              value={entry.value}
              min={rule.minimum ?? undefined}
              max={rule.maximum ?? undefined}
              step={1}
              width="105px"
              {disabled}
              onValueChange={(text) => set(rule, text)}
            />
          {:else}
            <input
              class="number"
              type="text"
              value={entry.value}
              aria-label={rule?.label ?? entry.id}
              {disabled}
              onchange={(event) =>
                publish(
                  value
                    .split('\n')
                    .map((line) =>
                      line.split('=')[0].trim() === entry.id
                        ? `${entry.id}=${event.currentTarget.value}`
                        : line,
                    )
                    .join('\n'),
                )}
            />
          {/if}
          <Button size="sm" variant="secondary" {disabled} onclick={() => remove(entry.id)}
            >Remove</Button
          >
        </div>
      </div>
    {/each}
  {/if}
  {#if unlisted.length > 0}<p class="hint" role="status">
      {unlisted.length} saved rule{unlisted.length === 1 ? '' : 's'} not listed for this version. Review
      them before saving; MSC preserves them.
    </p>{/if}

  {#if browsing && catalog?.available}
    <Field bind:value={search} placeholder="Search rules by name or purpose" />
    <div class="picker" aria-label="Available gameplay rules">
      {#each available as rule (rule.id)}
        <div class="rule-row">
          <div class="rule-info">
            <span class="rule-name">{rule.label}</span>
            <span class="hint">{rule.description}</span>
            <code>{rule.id} · default {rule.defaultValue}</code>
            {#if rule.experimental}<span class="hint"
                >Requires the matching Minecraft experiment.</span
              >{/if}
          </div>
          {#if rule.id === 'showcoordinates' && onCoordinatesChange}
            <Toggle
              checked={coordinates === true}
              label="Coordinates"
              {disabled}
              onchange={onCoordinatesChange}
            />
          {:else}
            <Button
              size="sm"
              variant="secondary"
              {disabled}
              onclick={() => set(rule, rule.defaultValue)}>Add</Button
            >
          {/if}
        </div>
      {/each}
      {#if available.length === 0}<p class="hint">No matching rules.</p>{/if}
    </div>
  {/if}
</div>

<style>
  .editor {
    display: grid;
    gap: 10px;
  }
  .toolbar,
  .rule-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
  }
  .hint,
  code {
    color: var(--msc2-text-secondary);
    font-size: 12px;
  }
  .context {
    display: grid;
    gap: 3px;
    min-width: 0;
  }
  .version {
    color: var(--msc2-text-primary);
    font-size: 13px;
    font-weight: 500;
  }
  .edition {
    color: var(--msc2-text-secondary);
    font-size: 12px;
  }
  .hint {
    margin: 0;
    line-height: 1.5;
  }
  .rule-row {
    padding: 12px 0;
    border-bottom: 1px solid var(--msc2-hairline);
  }
  .rule-info {
    display: grid;
    gap: 4px;
    min-width: 0;
  }
  .rule-name {
    font-size: 13px;
    font-weight: 500;
  }
  code {
    overflow-wrap: anywhere;
  }
  .picker {
    max-height: 320px;
    overflow-y: auto;
  }
  .number {
    width: 105px;
    padding: 8px;
    border: 1px solid var(--msc2-hairline-field);
    border-radius: 8px;
    background: var(--msc2-tier-chrome);
    color: var(--msc2-text-primary);
    font: inherit;
  }
  .invalid {
    color: var(--msc2-status-error);
    font-size: 12px;
  }
  @media (max-width: 600px) {
    .toolbar,
    .rule-row {
      flex-wrap: wrap;
    }
  }
</style>
