<script lang="ts">
  import { onMount } from 'svelte';

  export let fallbackMs = 900;
  let visible = true;
  let reducedMotion = false;

  onMount(() => {
    reducedMotion = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
    const finish = () => (visible = false);
    if (reducedMotion) {
      finish();
      return;
    }
    const timer = window.setTimeout(finish, fallbackMs);
    return () => window.clearTimeout(timer);
  });
</script>

{#if visible}
  <div
    class:reduced={reducedMotion}
    class="splash"
    role="status"
    aria-label="Opening Minecraft Server Controller"
  >
    <span aria-hidden="true">◆ ◆ ◆</span>
  </div>
{/if}

<style>
  /* MSC 1's bundled splash_intro was not extractable. This bounded CSS mark is the reviewed replacement. */
  .splash {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    color: var(--msc-accent);
    background: var(--msc-bg);
    font-size: 2rem;
    letter-spacing: 0.5rem;
    animation: settle 0.65s ease-out both;
  }
  .splash.reduced {
    animation: none;
  }
  @keyframes settle {
    0% {
      opacity: 0;
      transform: scale(1.04);
    }
    100% {
      opacity: 1;
      transform: scale(1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .splash {
      animation: none;
    }
  }
</style>
