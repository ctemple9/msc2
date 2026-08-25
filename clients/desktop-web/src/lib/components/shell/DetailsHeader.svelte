<script lang="ts">
  // Selected-server identity row: name, type/flavor badges, running state,
  // directory path. Wash = one of the four sanctioned bannerColor spots.
  // docs/msc2/renderings/shell.html, MSC 1 DetailsHeaderSectionView.swift.
  import Badge from '../base/Badge.svelte';
  import StatusDot from '../base/StatusDot.svelte';
  import { bannerColorAccent } from '../../styles/bannerColor';

  export let serverName: string;
  export let serverType: string | undefined = undefined;
  export let javaFlavor: string | undefined = undefined;
  export let directory: string | undefined = undefined;
  export let running: boolean;
  export let bannerColor: string;

  $: typeLabel = serverType === 'java' ? 'Java' : serverType === 'bedrock' ? 'Bedrock' : serverType;
</script>

<div class="header" style="--wash: {bannerColorAccent(bannerColor, 0.06)};">
  <div class="wash" aria-hidden="true"></div>
  <div class="row">
    <span class="name">{serverName}</span>
    {#if typeLabel}<Badge variant="category">{typeLabel}</Badge>{/if}
    {#if javaFlavor}<Badge variant="category">{javaFlavor}</Badge>{/if}
    <span class="fill"></span>
    {#if running}<StatusDot tone="ok" label="Running" />{/if}
  </div>
  {#if directory}<p class="path">{directory}</p>{/if}
</div>

<style>
  .header {
    position: relative;
    flex-shrink: 0;
    padding: 12px 16px;
    background: var(--msc2-tier-chrome);
    border-bottom: 1px solid var(--msc2-hairline-faint);
  }
  .wash {
    position: absolute;
    inset: 0;
    background: var(--wash);
    pointer-events: none;
  }
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .name {
    font-size: 19px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .fill {
    flex: 1;
  }
  .path {
    position: relative;
    margin: 2px 0 0;
    font-size: 11px;
    color: var(--msc2-text-tertiary);
  }
</style>
