<script lang="ts">
  // One restrained, neutral line-icon set for shell chrome and actions only —
  // never on informational elements (docs/msc2/antiAIslop.md #6, #7).
  export let name:
    | 'sidebar'
    | 'console'
    | 'external-link'
    | 'help'
    | 'settings'
    | 'refresh'
    | 'selector'
    | 'play'
    | 'chevron-right'
    | 'chevron-down'
    | 'chevron-up'
    | 'user';
  export let size = 15;

  const paths: Record<string, string> = {
    sidebar: 'M4 5h16v14H4zM9 5v14',
    console: 'M4 5h16v14H4zM4 15h16',
    'external-link': 'M14 4h6v6M20 4l-9 9M19 13v5a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V7a2 2 0 0 1 2-2h5',
    help: 'M9.5 9a2.5 2.5 0 1 1 3.4 2.3c-.9.4-1.4 1-1.4 1.9v.3M12 17h.01',
    settings:
      'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.4 13a7.4 7.4 0 0 0 0-2l2.02-1.58-1.92-3.32-2.38.96a7.4 7.4 0 0 0-1.72-1L14.5 2.6h-3.85l-.4 2.55a7.4 7.4 0 0 0-1.72 1l-2.38-.96-1.92 3.32L6.23 10a7.4 7.4 0 0 0 0 2l-2.02 1.58 1.92 3.32 2.38-.96a7.4 7.4 0 0 0 1.72 1l.4 2.55h3.85l.4-2.55a7.4 7.4 0 0 0 1.72-1l2.38.96 1.92-3.32z',
    refresh: 'M4 4v5h5M20 20v-5h-5M4.5 15a8 8 0 0 0 14.9 2.5M19.5 9A8 8 0 0 0 4.6 6.5',
    selector: 'M7 9l5-5 5 5M7 15l5 5 5-5',
    play: 'M7 5l12 7-12 7z',
    'chevron-right': 'M9 6l6 6-6 6',
    'chevron-down': 'M6 9l6 6 6-6',
    'chevron-up': 'M6 15l6-6 6 6',
    user: 'M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM4 20a8 8 0 0 1 16 0',
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill={name === 'play' ? 'currentColor' : 'none'}
  aria-hidden="true"
  class="shell-icon"
>
  <path
    d={paths[name]}
    stroke={name === 'play' ? 'none' : 'currentColor'}
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
  />
</svg>

<style>
  .shell-icon {
    display: block;
    flex-shrink: 0;
  }
</style>
