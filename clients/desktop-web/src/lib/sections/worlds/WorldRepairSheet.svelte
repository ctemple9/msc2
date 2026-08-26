<script lang="ts">
  // Ports WorldRepairView.swift's three phases (prompt -> repairing -> done).
  // Bedrock-only, and only for the currently active slot (the agent's own
  // /v1/worlds/repair guard). Real gap, faced honestly rather than faked:
  // the route is fully frozen and wired end to end, but its actual level.dat
  // regeneration workflow doesn't exist on this runtime yet -- every call
  // returns 409 repair_unavailable today (crates/msc-agent/src/routes/worlds.rs
  // repair()'s own doc comment). That response gets its own honest "not
  // available yet" state below instead of a fake success/log-streaming
  // animation; the moment a future step wires the real regeneration, this
  // same call already succeeds with no client change.
  import Sheet from '../../components/base/Sheet.svelte';
  import Button from '../../components/base/Button.svelte';
  import type { Schema, ScreenApi } from '../shared/types';
  import { mutate } from '../shared/types';
  import { ApiError } from '../../api/client';
  import { worldPaths } from './model';

  export let api: ScreenApi | undefined = undefined;
  export let activeSlotId: string | undefined;
  export let onClose: () => void;
  export let onRepaired: (updated: Schema['WorldSlotsResponseDTO']) => void;

  type Phase =
    | { kind: 'prompt' }
    | { kind: 'busy' }
    | { kind: 'unavailable' }
    | { kind: 'failed'; message: string };

  let phase: Phase = { kind: 'prompt' };

  async function startRepair(): Promise<void> {
    if (!activeSlotId) {
      phase = { kind: 'failed', message: 'No active world slot to repair.' };
      return;
    }
    phase = { kind: 'busy' };
    try {
      const result = await mutate<Schema['WorldMutationResultDTO']>(api, worldPaths.repair, {
        slotId: activeSlotId,
      });
      if (result.updated) onRepaired(result.updated);
      onClose();
    } catch (error) {
      if (error instanceof ApiError && error.error.code === 'repair_unavailable') {
        phase = { kind: 'unavailable' };
      } else {
        phase = {
          kind: 'failed',
          message:
            error instanceof Error ? error.message : 'Something went wrong repairing the world.',
        };
      }
    }
  }
</script>

<Sheet title="Repair World" size="sm" onClose={phase.kind === 'busy' ? undefined : onClose}>
  {#if phase.kind === 'prompt'}
    <div class="body">
      <p class="lede">Fixes connection failures after a Bedrock update.</p>
      <div class="error-sample">
        "The server you are attempting to join may not exist or may be locked"
      </div>
      <p class="explain">
        After a Minecraft update, the server's world format file (level.dat) can become incompatible
        with the new version, causing every connection to silently fail.
      </p>
      <div class="bullets">
        <p class="msc2-type-overline">What repair does</p>
        <ul>
          <li>Creates a backup of the current world first</li>
          <li>Starts the server briefly to generate an updated format file</li>
          <li>Replaces only the format file — builds and world data are untouched</li>
          <li>Removes temporary files when done</li>
        </ul>
      </div>
      <div class="footer">
        <Button variant="secondary" onclick={onClose}>Cancel</Button>
        <Button variant="primary" onclick={startRepair}>Repair World</Button>
      </div>
    </div>
  {:else if phase.kind === 'busy'}
    <div class="body busy">
      <p>Repairing world…</p>
      <p class="explain">Do not close this window or start the server manually.</p>
    </div>
  {:else if phase.kind === 'unavailable'}
    <div class="body">
      <p class="lede">Not available on this runtime yet</p>
      <p class="explain">
        World repair is fully wired in MSC's API but the Bedrock runtime doesn't expose the
        level.dat regeneration workflow yet. Nothing was changed.
      </p>
      <div class="footer">
        <Button variant="primary" onclick={onClose}>Close</Button>
      </div>
    </div>
  {:else}
    <div class="body">
      <p class="lede error-text">Repair Failed</p>
      <p class="explain">{phase.message}</p>
      <div class="footer">
        <Button variant="secondary" onclick={onClose}>Close</Button>
        <Button variant="primary" onclick={startRepair}>Try Again</Button>
      </div>
    </div>
  {/if}
</Sheet>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 14px;
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
  .error-sample {
    font-family: var(--msc2-font-mono);
    font-size: 11px;
    color: var(--msc2-text-tertiary);
    background: var(--msc2-tier-chrome);
    border-radius: 8px;
    padding: 8px 10px;
  }
  .explain {
    margin: 0;
    font-size: 12px;
    color: var(--msc2-text-tertiary);
    line-height: 1.5;
  }
  .bullets ul {
    margin: 6px 0 0;
    padding-left: 18px;
    font-size: 12px;
    color: var(--msc2-text-secondary);
    line-height: 1.7;
  }
  .footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .busy p:first-child {
    font-size: 13px;
    color: var(--msc2-text-primary);
  }
</style>
