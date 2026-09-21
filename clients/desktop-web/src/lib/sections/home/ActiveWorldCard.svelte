<script lang="ts">
  // MSC 1 OverviewActiveWorldCardView, with live world time supplied by the
  // agent's performance snapshot rather than a client-side clock or disk
  // read. The card keeps that line compact so it remains an activity card,
  // not a second statistics panel.
  // Thumbnails use the same deterministic gradient placeholder MSC 1 falls
  // back to when a slot has no saved photo. Saved thumbnails are fetched
  // through the authenticated screen API because CSS image requests cannot
  // carry the desktop client's bearer credentials.
  import { onDestroy } from 'svelte';
  import Card from '../../components/base/Card.svelte';
  import Button from '../../components/base/Button.svelte';
  import Icon from '../../components/base/Icon.svelte';
  import { imageObjectUrl, type Schema, type ScreenApi } from '../shared/types';
  import { slotThumbnailUrl } from '../worlds/model';

  export let api: ScreenApi | undefined = undefined;
  export let slot: Schema['WorldSlotDTO'] | undefined = undefined;
  export let performance: Schema['PerformanceSnapshotDTO'] | undefined = undefined;
  export let isBedrock = false;
  export let difficulty: string | undefined = undefined;
  export let gamemode: string | undefined = undefined;
  export let onSwitch: () => void = () => {};
  export let onBackup: () => void = () => {};

  let thumbnailObjectUrl: string | undefined;
  let thumbnailLoadToken = 0;

  $: thumbnailPath = slot ? slotThumbnailUrl(slot) : undefined;
  $: thumbnailUrl = thumbnailPath
    ? (api?.resourceUrl?.(thumbnailPath) ?? thumbnailPath)
    : undefined;
  $: thumbnail = api?.getBytes
    ? thumbnailObjectUrl
    : thumbnailUrl
      ? `${thumbnailUrl}${thumbnailUrl.includes('?') ? '&' : '?'}v=0`
      : undefined;

  $: if (api?.getBytes && thumbnailPath) {
    void loadThumbnail(thumbnailPath);
  } else if (api?.getBytes) {
    clearThumbnail();
  }

  async function loadThumbnail(path: string): Promise<void> {
    const token = ++thumbnailLoadToken;
    clearThumbnailUrl();
    try {
      const bytes = await api?.getBytes?.(path);
      if (!bytes || token !== thumbnailLoadToken) return;
      thumbnailObjectUrl = imageObjectUrl(bytes);
    } catch {
      if (token === thumbnailLoadToken) thumbnailObjectUrl = undefined;
    }
  }

  function clearThumbnailUrl(): void {
    if (thumbnailObjectUrl) URL.revokeObjectURL(thumbnailObjectUrl);
    thumbnailObjectUrl = undefined;
  }

  function clearThumbnail(): void {
    thumbnailLoadToken += 1;
    clearThumbnailUrl();
  }

  onDestroy(clearThumbnail);

  function hue(seed: string): number {
    let h = 0;
    for (let i = 0; i < seed.length; i += 1) h = (h * 31 + seed.charCodeAt(i)) % 360;
    return h;
  }

  function placeholderStyle(name: string): string {
    const baseHue = hue(name);
    return `background: linear-gradient(160deg, hsl(${baseHue} 40% 42%), hsl(${(baseHue + 30) % 360} 45% 22%));`;
  }

  function formatWorldTime(ticks: number | undefined): string | undefined {
    if (ticks === undefined || !Number.isFinite(ticks)) return undefined;
    const daytimeTicks = ((Math.trunc(ticks) % 24000) + 24000) % 24000;
    const clockTicks = (daytimeTicks + 6000) % 24000;
    const hours = Math.floor(clockTicks / 1000);
    const minutes = Math.floor(((clockTicks % 1000) * 60) / 1000);
    return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}`;
  }

  $: worldTimeLabel =
    performance?.worldDay !== undefined && performance?.worldTimeTicks !== undefined
      ? `Day ${performance.worldDay} · ${formatWorldTime(performance.worldTimeTicks)}`
      : undefined;
</script>

<Card padding="14px 16px">
  <div class="overline">
    <span class="msc2-type-overline">Active World</span>
  </div>

  {#if slot}
    <div class="row">
      <div
        class="thumb"
        style={thumbnail ? `background-image: url(${thumbnail});` : placeholderStyle(slot.name)}
      >
        {#if !thumbnail}<Icon name="world" size={22} />{/if}
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
    </div>

    <div class="world-time" aria-live="polite">
      <span class="k">World time</span>
      <span class:unavailable={!worldTimeLabel} class="v">
        {worldTimeLabel ?? 'Unavailable'}
      </span>
    </div>

    <div class="actions">
      <Button variant="secondary" size="sm" onclick={onSwitch}>Switch</Button>
      <Button variant="secondary" size="sm" onclick={onBackup}>Backup</Button>
    </div>
  {:else}
    <div class="empty">
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
    justify-content: center;
  }
  .world-time {
    display: flex;
    align-items: baseline;
    gap: 6px;
    margin: 0 0 12px;
  }
  .world-time .unavailable {
    color: var(--msc2-text-tertiary);
    font-weight: 400;
  }
  .actions :global(.btn) {
    flex: 1;
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
