<script lang="ts">
  // Same field language as Field, narrower by default. docs/msc2/renderings/primitives.html
  export let value: number | string = '';
  export let placeholder = '';
  export let disabled = false;
  export let width = '70px';
  export let min: number | undefined = undefined;
  export let max: number | undefined = undefined;
  export let step: number | undefined = undefined;
  export let onchange: ((value: string) => void) | undefined = undefined;

  // Controlled like Select, not bind:value -- a caller driving `value` off a
  // dynamically-keyed record (this field's first consumer, Settings, does)
  // needs the plain string it typed, not the native input's own number
  // coercion.
  function handleInput(event: Event): void {
    value = (event.currentTarget as HTMLInputElement).value;
    onchange?.(value as string);
  }
</script>

<input
  type="number"
  {value}
  {placeholder}
  {disabled}
  {min}
  {max}
  {step}
  oninput={handleInput}
  class="field"
  style="width: {width};"
/>

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
