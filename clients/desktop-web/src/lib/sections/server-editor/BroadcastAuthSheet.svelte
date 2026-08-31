<script lang="ts">
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import { openExternal } from '../../platform';
  import type { Schema, ScreenApi } from '../shared/types';
  import { mutate } from '../shared/types';
  import { serverEditorPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let prompt: Schema['BroadcastAuthPromptDTO'];
  export let onClose: () => void;

  let busy = false;
  let error = '';

  async function openSignIn(): Promise<void> {
    const url = prompt.linkURL;
    if (!url) return;
    try {
      await openExternal(url);
    } catch (caught) {
      error = caught instanceof Error ? caught.message : 'The sign-in page could not be opened.';
    }
  }

  async function done(): Promise<void> {
    if (busy) return;
    busy = true;
    error = '';
    try {
      await mutate(api, serverEditorPaths.broadcastAuthPromptDismiss);
      onClose();
    } catch (caught) {
      error =
        caught instanceof Error ? caught.message : 'The sign-in prompt could not be dismissed.';
      busy = false;
    }
  }
</script>

<Sheet title="Sign in to Xbox Broadcast" size="sm" onClose={busy ? undefined : done}>
  <div class="stack">
    <div>
      <p class="msc2-type-overline">Microsoft sign-in</p>
      <h2>Finish Xbox Broadcast setup</h2>
      <p class="copy">
        Open Microsoft's device sign-in page and use the code below with the dedicated Xbox
        Broadcast account.
      </p>
    </div>

    <div class="code-block" aria-label="Microsoft device sign-in code">
      <span>Device code</span>
      <code>{prompt.code ?? 'Shown by the helper'}</code>
    </div>

    {#if prompt.linkURL}
      <Button variant="secondary" onclick={() => void openSignIn()}>Open Microsoft sign-in</Button>
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}

    <div class="actions">
      <span class="action-spacer"></span>
      <Button variant="primary" disabled={busy} onclick={() => void done()}>
        {busy ? 'Finishing…' : 'Done'}
      </Button>
    </div>
  </div>
</Sheet>

<style>
  .stack {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  h2 {
    margin: 0;
    color: var(--msc2-text-primary);
    font-size: 15px;
    font-weight: 600;
  }
  .copy,
  .error {
    margin: 6px 0 0;
    color: var(--msc2-text-tertiary);
    font-size: 12px;
    line-height: 1.55;
  }
  .code-block {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 12px;
    padding: 12px;
    background: var(--msc2-tier-terminal);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 8px;
  }
  .code-block span {
    color: var(--msc2-text-secondary);
    font-size: 12px;
  }
  code {
    color: var(--msc2-text-primary);
    font-family: var(--msc2-font-mono);
    font-size: 16px;
    letter-spacing: 0.08em;
  }
  .error {
    color: var(--msc2-status-error);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
  }
  .action-spacer {
    flex: 1;
  }
</style>
