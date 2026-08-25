<script lang="ts">
  // Same field language as Field/NumberField, with a neutral chevron.
  // docs/msc2/renderings/primitives.html
  export let options: readonly { value: string; label: string }[] = [];
  export let value = '';
  export let disabled = false;
  export let width = '100%';
  export let onchange: ((value: string) => void) | undefined = undefined;

  function handleChange(event: Event) {
    value = (event.currentTarget as HTMLSelectElement).value;
    onchange?.(value);
  }
</script>

<div class="wrap" style="width: {width};">
  <select class="select" {disabled} bind:value onchange={handleChange}>
    {#each options as option (option.value)}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
  <svg class="chevron" width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
    <path
      d="M6 9l6 6 6-6"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
</div>

<style>
  .wrap {
    position: relative;
    display: inline-block;
  }

  .select {
    box-sizing: border-box;
    width: 100%;
    appearance: none;
    font-family: inherit;
    font-size: 13px;
    color: #fff;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-field);
    border-radius: 8px;
    padding: 7px 32px 7px 10px;
    outline: none;
    cursor: pointer;
  }

  .select:focus-visible {
    border-color: var(--msc2-hairline-field-focus);
  }

  .select:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .chevron {
    position: absolute;
    right: 9px;
    top: 50%;
    transform: translateY(-50%);
    color: rgba(255, 255, 255, 0.5);
    pointer-events: none;
  }
</style>
