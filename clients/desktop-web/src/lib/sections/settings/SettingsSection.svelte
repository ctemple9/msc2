<script lang="ts">
  // Ports DetailsSettingsTabView.swift + ServerSettingsView.swift's Java form
  // to the S0 disciplined system (docs/msc2/antiAIslop.md): a schema-driven
  // World/Server (/Network) section list, edited as a local draft that stays
  // unsaved until Save Changes, matching MSC 1's "changes stay local until
  // you click Save Changes" model exactly. Same shared-component pattern
  // ComponentsSection/HomeSection/WorldsSection use (D-003).
  //
  // The backend (crates/msc-agent/src/routes/settings.rs) sends sections/
  // fields generically rather than a closed Java/Bedrock field list, so this
  // component renders whatever it's given rather than hardcoding property
  // names -- only `segmentedKeys` below reaches for a key by name, to match
  // MSC 1's exact choice of segmented-vs-dropdown per field.
  //
  // MSC 1's PreferencesJavaSection/RAM/Geyser rows (java executable path,
  // heap size, Geyser listener) are a *different* MSC 1 screen -- the
  // app-level "MSC Settings" sheet (General/Remote/Data tabs), not this
  // per-server tab (confirmed against DetailsSettingsTabView, which embeds
  // only ServerSettingsView). That sheet is P12.14's scope, not this one; the
  // previous version of this file conflated the two, which this rebuild
  // corrects rather than carries forward.
  //
  // One real, pre-existing backend gap found while wiring this, left alone
  // (routes/ wasn't in this step's scope) but worth recording plainly:
  // bedrock_sections' difficulty/gamemode fields are typed "enum" but carry
  // no `options` (unlike java_sections' difficulty/gamemode) -- Bedrock
  // settings stay unported per that file's own header comment. Rendered
  // honestly below as a plain text field until that lands.
  //
  // No per-field "Learn more" links: HelpLink renders a plain <a href> to
  // /hosts/{hostId}/servers/{serverId}/handbook?topic=..., which
  // hard-navigates the whole webview instead of switching sections in-app --
  // Cameron hit this live (splash restart, then a fresh, disconnected
  // Handbook load). Handbook itself isn't rebuilt yet either (P12.16, not
  // started), so it's not worth wiring a real in-app link here now. Re-add
  // once both exist.
  import { onMount } from 'svelte';
  import Icon from '../../components/base/Icon.svelte';
  import Button from '../../components/base/Button.svelte';
  import Card from '../../components/base/Card.svelte';
  import Toggle from '../../components/base/Toggle.svelte';
  import Field from '../../components/base/Field.svelte';
  import NumberField from '../../components/base/NumberField.svelte';
  import Select from '../../components/base/Select.svelte';
  import SegmentedControl from '../../components/base/SegmentedControl.svelte';
  import EmptyState from '../../components/base/EmptyState.svelte';
  import { ApiError } from '../../api/client';
  import type { Schema, ScreenProps } from '../shared/types';
  import { call, errorMessage, mutate } from '../shared/types';
  import { demoSettings } from './model';

  export let api: ScreenProps['api'] = undefined;
  // No Learn More links here (removed -- see note below), so nothing in this
  // section is host-scoped; kept only so the section registry can pass it
  // uniformly (ComponentsSection precedent).
  export const hostId = 'local-agent';
  export let serverId = 'survival';

  // MSC 1's ServerSettingsView uses .pickerStyle(.segmented) only for these
  // two enum fields; World Type and Op Permission Level are also `enum` but
  // stay a plain dropdown there, so the split is by key, not by option count.
  const segmentedKeys = new Set(['difficulty', 'gamemode']);

  let settings: Schema['SettingsResponseDTO'] = demoSettings;
  let original: Record<string, string> = {};
  let draft: Record<string, string> = {};
  let notice = '';
  let rejected: Schema['SettingRejectionDTO'][] = [];
  let saving = false;
  let confirmation: SafetyPrompt | undefined;
  let forceGamemodeConfirmation = false;
  let lastServerId: string | undefined;

  type SafetyPrompt = {
    token: string;
    title: string;
    message: string;
  };

  function safetyPrompt(error: unknown): SafetyPrompt | undefined {
    if (!(error instanceof ApiError) || error.error.code !== 'confirmation_required') return;
    const raw = (error.error.details as Record<string, unknown> | null | undefined)?.confirmation;
    if (!raw || typeof raw !== 'object') return;
    const prompt = raw as Record<string, unknown>;
    if (
      typeof prompt.acknowledgement !== 'string' ||
      typeof prompt.title !== 'string' ||
      typeof prompt.message !== 'string'
    ) {
      return;
    }
    return {
      token: prompt.acknowledgement,
      title: prompt.title,
      message: prompt.message,
    };
  }

  function snapshot(sections: Schema['SettingsSectionDTO'][]): Record<string, string> {
    const next: Record<string, string> = {};
    for (const section of sections) {
      for (const field of section.fields) next[field.key] = field.value;
    }
    return next;
  }

  function setValue(key: string, value: string): void {
    draft = { ...draft, [key]: value };
  }

  function handleBooleanChange(key: string, checked: boolean): void {
    if (key === 'force-gamemode' && checked) {
      forceGamemodeConfirmation = true;
      return;
    }
    if (key === 'force-gamemode') forceGamemodeConfirmation = false;
    setValue(key, checked ? 'true' : 'false');
  }

  function confirmForceGamemode(): void {
    forceGamemodeConfirmation = false;
    setValue('force-gamemode', 'true');
  }

  function revert(): void {
    forceGamemodeConfirmation = false;
    draft = { ...original };
  }

  async function load(): Promise<void> {
    settings = await call(api, settings, '/v1/settings');
    original = snapshot(settings.sections);
    draft = { ...original };
    forceGamemodeConfirmation = false;
  }

  $: changes = Object.fromEntries(
    Object.entries(draft).filter(([key, value]) => value !== original[key]),
  );
  $: dirty = Object.keys(changes).length > 0;

  function summarize(result: Schema['SettingsUpdateResultDTO']): string {
    const parts = [result.success ? 'Settings saved.' : 'No changes applied.'];
    if (result.restartRequired) parts.push('Restart the server to apply.');
    return parts.join(' ');
  }

  async function save(confirmationToken?: string): Promise<void> {
    if (!dirty) return;
    saving = true;
    confirmation = undefined;
    try {
      const effectiveConfirmation =
        confirmationToken ??
        (changes['force-gamemode'] === 'true' ? 'server_force_gamemode' : undefined);
      const result = await mutate<Schema['SettingsUpdateResultDTO']>(api, '/v1/settings', {
        changes,
        ...(effectiveConfirmation ? { confirmation: effectiveConfirmation } : {}),
      });
      if (result.sections) settings = { ...settings, sections: result.sections };
      original = snapshot(settings.sections);
      draft = { ...original };
      rejected = result.rejected ?? [];
      notice = summarize(result);
    } catch (error) {
      confirmation = safetyPrompt(error);
      if (!confirmation) notice = errorMessage(error);
    } finally {
      saving = false;
    }
  }

  $: if (serverId !== lastServerId) {
    lastServerId = serverId;
    void load();
  }

  onMount(() => void load());
</script>

<div class="settings">
  <div class="section-header">
    <div class="overline">
      <Icon name="gear" size={13} />
      <span class="msc2-type-overline">Server Properties</span>
    </div>
    <div class="header-actions">
      {#if dirty}<span class="status-label">Unsaved changes</span>{/if}
    </div>
  </div>

  {#if notice}<p class="notice" role="status">{notice}</p>{/if}
  {#if confirmation}
    <div class="confirmation" role="alert">
      <p class="confirmation-title">{confirmation.title}</p>
      <p>{confirmation.message}</p>
      <div class="confirmation-actions">
        <Button size="sm" variant="secondary" onclick={() => (confirmation = undefined)}>
          Cancel
        </Button>
        <Button size="sm" variant="primary" onclick={() => void save(confirmation?.token)}>
          Continue
        </Button>
      </div>
    </div>
  {/if}
  {#if rejected.length}
    <ul class="rejected">
      {#each rejected as item (item.key)}<li>{item.key}: {item.reason}</li>{/each}
    </ul>
  {/if}

  {#if !settings.editable}
    <EmptyState title="No server selected" message="Select a server to view its settings.">
      <Icon name="gear" size={26} slot="icon" />
    </EmptyState>
  {:else}
    {#each settings.sections as section (section.id)}
      <section class="zone">
        <p class="msc2-type-overline">{section.title}</p>
        <Card padding="0">
          {#each section.fields as field, index (field.key)}
            <div class="row" class:bordered={index > 0}>
              <span class="name">{field.label}</span>
              <div class="control">
                {#if field.type === 'bool'}
                  <Toggle
                    checked={draft[field.key] === 'true'}
                    label={field.label}
                    onchange={(checked) => handleBooleanChange(field.key, checked)}
                  />
                {:else if field.type === 'enum' && field.options?.length}
                  {#if segmentedKeys.has(field.key)}
                    <SegmentedControl
                      options={field.options}
                      value={draft[field.key] ?? ''}
                      onchange={(value) => setValue(field.key, value)}
                    />
                  {:else}
                    <Select
                      options={field.options}
                      value={draft[field.key] ?? ''}
                      width="220px"
                      onchange={(value) => setValue(field.key, value)}
                    />
                  {/if}
                {:else if field.type === 'int'}
                  <NumberField
                    value={draft[field.key] ?? ''}
                    min={field.minInt}
                    max={field.maxInt}
                    width="80px"
                    onValueChange={(value) => setValue(field.key, value)}
                  />
                  {#if field.unit}<span class="unit">{field.unit}</span>{/if}
                {:else}
                  <Field bind:value={draft[field.key]} width="260px" />
                {/if}
              </div>
            </div>
            {#if field.key === 'force-gamemode' && forceGamemodeConfirmation}
              <div class="force-confirmation" role="alert">
                <div class="force-confirmation-copy">
                  <span class="confirmation-title">Enable Force Gamemode?</span>
                  <span class="hint">Applies to every world and can override saved defaults.</span>
                </div>
                <div class="confirmation-actions">
                  <Button
                    size="sm"
                    variant="secondary"
                    onclick={() => (forceGamemodeConfirmation = false)}
                  >
                    Cancel
                  </Button>
                  <Button size="sm" variant="primary" onclick={confirmForceGamemode}>Confirm</Button
                  >
                </div>
              </div>
            {/if}
          {/each}
        </Card>
      </section>
    {/each}

    <div class="footer-actions">
      <Button size="sm" variant="secondary" disabled={!dirty || saving} onclick={revert}>
        Revert
      </Button>
      <Button size="sm" variant="primary" disabled={!dirty || saving} onclick={() => void save()}>
        {saving ? 'Saving…' : 'Save Changes'}
      </Button>
    </div>
  {/if}
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
  }
  .overline {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--msc2-text-tertiary);
  }
  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .status-label {
    color: var(--msc2-status-warn);
    font-size: 12px;
    font-weight: 500;
  }
  .hint {
    margin: -10px 0 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
  }
  .notice {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-secondary);
  }
  .rejected {
    margin: 0;
    padding-left: 18px;
    font-size: 12px;
    color: var(--msc2-status-warn);
  }
  .zone {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 11px 14px;
  }
  .row.bordered {
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .name {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .control {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .unit {
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
  .footer-actions {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
  .confirmation {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border: 1px solid var(--msc2-hairline-strong);
    border-radius: 8px;
    color: var(--msc2-text-secondary);
    font-size: 12px;
    line-height: 1.45;
  }
  .confirmation p {
    margin: 0;
  }
  .confirmation-title {
    color: var(--msc2-text-primary);
    font-weight: 600;
  }
  .confirmation-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .force-confirmation {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px;
    border-top: 1px solid var(--msc2-hairline-subtle);
  }
  .force-confirmation-copy {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .force-confirmation-copy .hint {
    margin: 0;
  }
</style>
