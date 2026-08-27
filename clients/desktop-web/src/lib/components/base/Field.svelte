<script lang="ts">
  // Dark inset field. Focus = brighter neutral ring, never a glow.
  // docs/msc2/renderings/primitives.html
  export let value = '';
  export let type: 'text' | 'password' | 'email' = 'text';
  export let placeholder = '';
  export let disabled = false;
  export let width = '100%';
  export let multiline = false;
</script>

{#if multiline}
  <textarea
    bind:value
    {placeholder}
    {disabled}
    class="field multiline"
    style="width: {width};"
    rows="2"
  ></textarea>
{:else}
  <input {type} bind:value {placeholder} {disabled} class="field" style="width: {width};" />
{/if}

<style>
  .field {
    box-sizing: border-box;
    font-family: inherit;
    font-size: 13px;
    color: #fff;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-field);
    border-radius: 8px;
    padding: 7px 10px;
    outline: none;
  }

  .field:focus-visible {
    border-color: var(--msc2-hairline-field-focus);
  }

  .field:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .field.multiline {
    resize: vertical;
    min-height: 54px;
    font-family: var(--msc2-font-mono, monospace);
    line-height: 1.5;
  }
</style>
