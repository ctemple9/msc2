<script lang="ts">
  // Dark inset field. Focus = brighter neutral ring, never a glow.
  // docs/msc2/renderings/primitives.html
  export let value = '';
  export let type: 'text' | 'password' | 'email' = 'text';
  export let placeholder = '';
  export let disabled = false;
  export let width = '100%';
</script>

<input {type} bind:value {placeholder} {disabled} class="field" style="width: {width};" />

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
</style>
