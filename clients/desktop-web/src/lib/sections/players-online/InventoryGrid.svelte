<script lang="ts">
  // Ports PlayerInventoryView.swift's three zones (equipment, 3x9 main,
  // hotbar) at the same slot numbering: 100-103 armor, -106 offhand, 0-8
  // hotbar, 9-35 main. Item icons come straight from the same public asset
  // repo the Swift source uses — a plain <img>, same CORS-avoidance trick
  // already used for minotar.net/mc-heads.net avatars elsewhere in this client.
  import type { Schema } from '../shared/types';

  export let inventory: readonly Schema['InventoryItemDTO'][] = [];

  const ICON_BASE =
    'https://raw.githubusercontent.com/InventivetalentDev/minecraft-assets/1.21.1/assets/minecraft/textures';

  $: bySlot = new Map(inventory.map((item) => [item.slot, item]));

  function iconUrl(iconName: string): string {
    return `${ICON_BASE}/item/${iconName}.png`;
  }
  function iconFallback(event: Event, iconName: string): void {
    const img = event.currentTarget as HTMLImageElement;
    if (img.dataset.fallback) return;
    img.dataset.fallback = '1';
    img.src = `${ICON_BASE}/block/${iconName}.png`;
  }

  function title(item: Schema['InventoryItemDTO']): string {
    const enchants = item.enchantments.map((e) => e.displayName).join(', ');
    const parts = [item.displayName, item.count > 1 ? `x${item.count}` : null, enchants || null];
    return parts.filter(Boolean).join(' — ');
  }
</script>

{#snippet slot(item: Schema['InventoryItemDTO'] | undefined, highlighted: boolean)}
  <div class="slot" class:highlighted title={item ? title(item) : undefined}>
    {#if item}
      <img
        class="icon"
        src={iconUrl(item.iconName)}
        alt=""
        loading="lazy"
        onerror={(event) => iconFallback(event, item.iconName)}
      />
      {#if item.count > 1}<span class="count">{item.count}</span>{/if}
    {/if}
  </div>
{/snippet}

<div class="grid">
  <div class="zone">
    <span class="label">Equipment</span>
    <div class="row">
      {#each [103, 102, 101, 100] as armorSlot (armorSlot)}
        {@render slot(bySlot.get(armorSlot), false)}
      {/each}
      <div class="gap"></div>
      <div class="offhand">
        {@render slot(bySlot.get(-106), false)}
        <span class="offhand-label">Off</span>
      </div>
    </div>
  </div>

  <div class="divider"></div>

  <div class="zone">
    <span class="label">Inventory</span>
    <div class="main">
      {#each [0, 1, 2] as row (row)}
        <div class="row">
          {#each Array(9) as _, col (col)}
            {@render slot(bySlot.get(9 + row * 9 + col), false)}
          {/each}
        </div>
      {/each}
    </div>
  </div>

  <div class="hotbar-divider"></div>

  <div class="row">
    {#each Array(9) as _, col (col)}
      {@render slot(bySlot.get(col), true)}
    {/each}
  </div>
</div>

<style>
  .grid {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .zone {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .label {
    font-size: 11px;
    font-weight: 500;
    color: var(--msc2-text-tertiary);
  }
  .row {
    display: flex;
    gap: 3px;
  }
  .main {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .divider {
    height: 1px;
    background: var(--msc2-hairline-subtle);
  }
  .hotbar-divider {
    height: 1px;
    background: rgba(255, 255, 255, 0.07);
  }
  .gap {
    width: 22px;
  }
  .offhand {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }
  .offhand-label {
    font-size: 9px;
    color: var(--msc2-text-tertiary);
  }
  .slot {
    position: relative;
    width: 36px;
    height: 36px;
    flex: none;
    box-sizing: border-box;
    border-radius: 4px;
    background: var(--msc2-tier-terminal);
    border: 1px solid var(--msc2-hairline-subtle);
  }
  .slot.highlighted {
    border-color: rgba(255, 255, 255, 0.18);
  }
  .icon {
    position: absolute;
    inset: 4px;
    width: calc(100% - 8px);
    height: calc(100% - 8px);
    image-rendering: pixelated;
    object-fit: contain;
  }
  .count {
    position: absolute;
    right: 2px;
    bottom: 1px;
    font-size: 10px;
    font-weight: 600;
    color: #fff;
    text-shadow: 0 1px 1px rgba(0, 0, 0, 0.9);
  }
</style>
