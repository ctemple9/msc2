<script lang="ts">
  // Same field language as Field, narrower by default. docs/msc2/renderings/primitives.html
  //
  // The native <input type="number"> spin buttons render as the OS/browser's
  // own light widget with no CSS hook to recolor -- on MSC 2's dark field
  // they read as a stray white toggle stuck to the side (caught live during
  // P12.8 review). Hidden here and replaced with a dark two-button stepper
  // that drives the same native stepUp()/stepDown() (so min/max/step clamping
  // stays exactly what the browser already does), matching every other field
  // control instead of leaking OS chrome.
  export let value: number | string = '';
  export let placeholder = '';
  export let disabled = false;
  export let width = '70px';
  export let min: number | undefined = undefined;
  export let max: number | undefined = undefined;
  export let step: number | undefined = undefined;
  export let onValueChange: ((value: string) => void) | undefined = undefined;

  let inputEl: HTMLInputElement;

  function handleInput(event: Event): void {
    value = (event.currentTarget as HTMLInputElement).value;
    onValueChange?.(value as string);
  }

  function bump(delta: number): void {
    if (disabled || !inputEl) return;
    if (delta > 0) inputEl.stepUp();
    else inputEl.stepDown();
    value = inputEl.value;
    onValueChange?.(value as string);
  }
</script>

<div class="wrap" style="width: {width};">
  <input
    bind:this={inputEl}
    type="number"
    {value}
    {placeholder}
    {disabled}
    {min}
    {max}
    {step}
    oninput={handleInput}
    class="field"
  />
  <div class="stepper">
    <button
      type="button"
      class="step"
      tabindex="-1"
      {disabled}
      aria-label="Increase"
      onclick={() => bump(1)}
    >
      <svg width="8" height="5" viewBox="0 0 8 5" fill="none" aria-hidden="true">
        <path
          d="M1 4l3-3 3 3"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
    <button
      type="button"
      class="step"
      tabindex="-1"
      {disabled}
      aria-label="Decrease"
      onclick={() => bump(-1)}
    >
      <svg width="8" height="5" viewBox="0 0 8 5" fill="none" aria-hidden="true">
        <path
          d="M1 1l3 3 3-3"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
  </div>
</div>

<style>
  .wrap {
    position: relative;
    display: inline-flex;
  }

  .field {
    box-sizing: border-box;
    width: 100%;
    font-family: inherit;
    font-size: 13px;
    color: #fff;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-field);
    border-radius: 8px;
    padding: 7px 26px 7px 10px;
    outline: none;
    appearance: textfield;
    -moz-appearance: textfield;
  }

  .field::-webkit-inner-spin-button,
  .field::-webkit-outer-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  .field:focus-visible {
    border-color: var(--msc2-hairline-field-focus);
  }

  .field:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .stepper {
    position: absolute;
    right: 3px;
    top: 3px;
    bottom: 3px;
    width: 18px;
    display: flex;
    flex-direction: column;
    border-radius: 5px;
    overflow: hidden;
    background: var(--msc2-neutral-elevated);
  }

  .step {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    padding: 0;
    color: rgba(255, 255, 255, 0.55);
    cursor: pointer;
  }

  .step:first-child {
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  }

  .step:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.08);
    color: rgba(255, 255, 255, 0.9);
  }

  .step:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
</style>
