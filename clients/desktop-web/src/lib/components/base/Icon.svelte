<script lang="ts">
  // Neutral content-area line icons — health cards, section overlines, list
  // rows. Never colored on an informational element (docs/msc2/antiAIslop.md
  // #6). Same visual language as ShellIcon (24x24, stroke 1.8) but this set
  // is for card/section content rather than chrome.
  export let name:
    | 'network'
    | 'waveform'
    | 'grid'
    | 'seal-check'
    | 'folder'
    | 'cup'
    | 'chip'
    | 'box'
    | 'download'
    | 'people'
    | 'world'
    | 'chat'
    | 'note'
    | 'clock'
    | 'id-card';
  export let size = 14;

  const paths: Record<string, string> = {
    network: 'M3 12h18M12 3c3 3 3 15 0 18M12 3c-3 3-3 15 0 18',
    waveform: 'M2 12h3l2-7 3 14 3-10 2 6h3l2-3h4',
    grid: 'M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z',
    'seal-check': 'M12 3l8 4.5v9L12 21l-8-4.5v-9zM8.5 12l2.4 2.4L15.5 9.5',
    folder: 'M4 6h5l2 2h9v10H4z',
    cup: 'M5 4h10v9a5 5 0 0 1-10 0zM15 7h2a2 2 0 0 1 0 4h-2',
    chip: 'M6 6h12v12H6zM9 3v3M15 3v3M9 18v3M15 18v3M3 9h3M3 15h3M18 9h3M18 15h3',
    box: 'M12 3l8 4.5v9L12 21l-8-4.5v-9zM12 3v18M4 7.5l8 4.5 8-4.5',
    download: 'M12 4v10M8 10l4 4 4-4M4 18h16',
    people: 'M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM4 20a8 8 0 0 1 16 0',
    world: 'M3 20l6-10 4 5 3-3.5 5 8.5z',
    chat: 'M4 5h16v10H8l-4 4z',
    note: 'M6 3h9l3 3v15H6zM15 3v3h3M8.5 12h7M8.5 16h4.5',
    clock: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 7v5l3.5 2',
    'id-card':
      'M3.5 5h17v14h-17zM12 10.5a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM7.5 17c.8-1.8 2.4-2.5 4.5-2.5s3.7.7 4.5 2.5M15 8.5h3',
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  aria-hidden="true"
  class="content-icon"
>
  <path
    d={paths[name]}
    stroke="currentColor"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
  />
</svg>

<style>
  .content-icon {
    display: block;
    flex-shrink: 0;
  }
</style>
