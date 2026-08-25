<script lang="ts">
  // One surface, hairline dividers inset past the icon — not a card per row.
  // Compose inside <Card padding="0"> for the divided-list container.
  // docs/msc2/renderings/primitives.html
  export let title: string;
  export let subtitle = '';
  export let last = false;
  export let onclick: ((event: MouseEvent) => void) | undefined = undefined;
</script>

{#if onclick}
  <button type="button" class="row clickable" {onclick}>
    <span class="icon"><slot name="icon" /></span>
    <span class="text">
      <span class="title">{title}</span>
      {#if subtitle}<span class="subtitle">{subtitle}</span>{/if}
    </span>
    <span class="trailing">
      <slot name="trailing">
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path
            d="M9 6l6 6-6 6"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </slot>
    </span>
  </button>
{:else}
  <div class="row">
    <span class="icon"><slot name="icon" /></span>
    <span class="text">
      <span class="title">{title}</span>
      {#if subtitle}<span class="subtitle">{subtitle}</span>{/if}
    </span>
    <span class="trailing">
      <slot name="trailing">
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path
            d="M9 6l6 6-6 6"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </slot>
    </span>
  </div>
{/if}
{#if !last}<div class="divider"></div>{/if}

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 11px 14px;
    width: 100%;
    box-sizing: border-box;
    background: transparent;
    border: none;
    font: inherit;
    text-align: left;
  }

  .clickable {
    cursor: pointer;
  }
  .clickable:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .icon {
    font-size: 17px;
    color: rgba(255, 255, 255, 0.45);
    display: inline-flex;
    align-items: center;
  }

  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .title {
    font-size: 13px;
    color: var(--msc2-text-primary);
  }

  .subtitle {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.45);
  }

  .trailing {
    display: inline-flex;
    align-items: center;
    color: rgba(255, 255, 255, 0.3);
  }

  .divider {
    height: 1px;
    background: var(--msc2-hairline-subtle);
    margin-left: 41px;
  }
</style>
