<script lang="ts">
  // Green = enabled (Mac-native, matches Start). docs/msc2/renderings/primitives.html
  export let checked = false;
  export let label = '';
  export let disabled = false;
  export let onchange: ((checked: boolean) => void) | undefined = undefined;

  function toggle() {
    if (disabled) return;
    checked = !checked;
    onchange?.(checked);
  }
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={label || undefined}
  class="track"
  class:on={checked}
  {disabled}
  onclick={toggle}
>
  <span class="thumb"></span>
</button>

<style>
  .track {
    width: 38px;
    height: 22px;
    border-radius: 11px;
    background: var(--msc2-neutral-muted);
    border: none;
    padding: 0;
    position: relative;
    cursor: pointer;
    transition: background 150ms ease;
  }

  .track.on {
    background: var(--msc2-status-ok);
  }

  .track:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #d0d0d0;
    transition: transform 150ms ease;
  }

  .track.on .thumb {
    background: #fff;
    transform: translateX(16px);
  }
</style>
