<script lang="ts">
  import Sheet from '../../components/base/Sheet.svelte';
  export let title: string;
  export let provider: 'modrinth' | 'curseforge' = 'modrinth';
  export let disabled = false;
  export let onClose: (() => void) | undefined;
  export let onProviderChanged: () => void = () => {};
</script>

<Sheet {title} size="lg" {onClose}>
  <div class="providers" role="group" aria-label="Content provider">
    {#each ['modrinth', 'curseforge'] as name}
      <button
        type="button"
        class:chosen={provider === name}
        aria-pressed={provider === name}
        {disabled}
        onclick={() => {
          provider = name as typeof provider;
          onProviderChanged();
        }}>{name === 'modrinth' ? 'Modrinth' : 'CurseForge'}</button
      >
    {/each}
  </div>
  <slot />
</Sheet>

<style>
  .providers {
    display: flex;
    gap: 20px;
    margin-bottom: 18px;
    border-bottom: 1px solid var(--msc2-hairline-subtle);
  }
  .providers button {
    padding: 0 0 10px;
    background: none;
    border: 0;
    border-bottom: 2px solid transparent;
    color: var(--msc2-text-tertiary);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .providers button.chosen {
    border-bottom-color: var(--msc2-selection);
    color: var(--msc2-text-primary);
    font-weight: 500;
  }
  .providers button:focus-visible {
    outline: 2px solid var(--msc2-selection);
    outline-offset: 3px;
  }
</style>
