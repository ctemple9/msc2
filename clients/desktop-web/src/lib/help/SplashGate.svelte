<script lang="ts">
  import { onMount } from 'svelte';

  const SPLASH_DURATION_MS = 3_000;
  export let onComplete: () => void = () => {};
  let visible = true;
  let completed = false;
  let reducedMotion = false;
  let splashTimer: ReturnType<typeof setTimeout> | undefined;

  function finish(): void {
    if (completed) return;
    completed = true;
    visible = false;
    if (splashTimer) clearTimeout(splashTimer);
    onComplete();
  }

  onMount(() => {
    reducedMotion = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
    if (reducedMotion) {
      finish();
      return;
    }
    splashTimer = window.setTimeout(finish, SPLASH_DURATION_MS);
    return () => {
      if (splashTimer) window.clearTimeout(splashTimer);
    };
  });
</script>

{#if visible}
  <div
    class:reduced={reducedMotion}
    class="splash"
    role="status"
    aria-label="Opening Minecraft Server Controller"
  >
    <img class="splash-icon" src="/msc-icon.png" alt="" aria-hidden="true" />
  </div>
{/if}

<style>
  .splash {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    color: white;
    background: rgb(0 0 0);
  }

  .splash-icon {
    display: block;
    width: min(14rem, 45vw, 45vh);
    height: auto;
    image-rendering: pixelated;
  }

  .splash.reduced {
    animation: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .splash {
      animation: none;
    }
  }
</style>
