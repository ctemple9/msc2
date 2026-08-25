<script lang="ts">
  // MSC 1 OverviewActiveWorldCardView, minus the in-game day/time clock —
  // that reads level.dat directly in MSC 1; the agent contract has no
  // equivalent field, so it is honestly omitted here rather than faked.
  // Thumbnails use the same deterministic gradient placeholder MSC 1 falls
  // back to when a slot has no saved photo (no real thumbnail fetch wired
  // yet — WorldsSection.svelte, the tab that owns real world art, is P12.4).
  import Card from '../../components/base/Card.svelte';
  import Button from '../../components/base/Button.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import type { Schema } from '../shared/types';

  export let slot: Schema['WorldSlotDTO'] | undefined = undefined;
  export let isBedrock = false;
  export let difficulty: string | undefined = undefined;
  export let gamemode: string | undefined = undefined;
  export let maxPlayers: string | undefined = undefined;
  export let onSwitch: () => void = () => {};
  export let onBackup: () => void = () => {};

  function hue(seed: string): number {
    let h = 0;
    for (let i = 0; i < seed.length; i += 1) h = (h * 31 + seed.charCodeAt(i)) % 360;
    return h;
  }
</script>

<Card padding="14px 16px">
  <div class="overline">
    <Icon name="world" size={12} />
    <span class="msc2-type-overline">Active World</span>
  </div>

  {#if slot}
    <div class="row">
      <div
        class="thumb"
        style="background: linear-gradient(160deg, hsl({hue(slot.name)} 40% 42%), hsl({(hue(
          slot.name,
        ) +
          30) %
          360} 45% 22%));"
      >
        <Icon name="world" size={22} />
      </div>
      <div class="meta">
        <p class="name">{slot.name}</p>
        <span class="edition">{isBedrock ? 'Bedrock' : 'Java'}</span>
      </div>
    </div>

    <div class="facts">
      {#if difficulty}<div class="fact">
          <span class="k">Diff</span><span class="v">{difficulty}</span>
        </div>{/if}
      {#if gamemode}<div class="fact">
          <span class="k">Mode</span><span class="v">{gamemode}</span>
        </div>{/if}
      {#if maxPlayers}<div class="fact">
          <span class="k">Max</span><span class="v">{maxPlayers}</span>
        </div>{/if}
    </div>

    <div class="actions">
      <Button variant="secondary" size="sm" onclick={onSwitch}>Switch</Button>
      <Button variant="secondary" size="sm" onclick={onBackup}>Backup</Button>
    </div>
  {:else}
    <div class="empty">
      <Icon name="world" size={18} />
      <span>No active world yet.</span>
    </div>
  {/if}
</Card>

<style>
  .overline {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 12px;
    color: var(--msc2-text-tertiary);
  }
  .row {
    display: flex;
    gap: 10px;
    align-items: flex-start;
  }
  .thumb {
    width: 56px;
    height: 56px;
    border-radius: 8px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: rgba(255, 255, 255, 0.85);
  }
  .meta {
    min-width: 0;
  }
  .name {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
    color: var(--msc2-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .edition {
    font-size: 9px;
    font-weight: 600;
    letter-spacing: 0.5px;
    text-transform: uppercase;
    color: var(--msc2-text-tertiary);
  }
  .facts {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px 10px;
    margin: 12px 0;
  }
  .fact {
    display: flex;
    gap: 5px;
    align-items: baseline;
  }
  .k {
    font-size: 10px;
    color: var(--msc2-text-tertiary);
  }
  .v {
    font-size: 11px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .empty {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--msc2-text-tertiary);
    font-size: 12px;
    padding: 8px 0;
  }
</style>
