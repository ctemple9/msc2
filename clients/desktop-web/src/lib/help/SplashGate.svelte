<script lang="ts">
  import { onMount } from 'svelte';

  export let fallbackMs = 900;
  let visible = true;
  let reducedMotion = false;
  let videoFailed = false;
  let fallbackTimer: ReturnType<typeof setTimeout> | undefined;

  function finish(): void {
    visible = false;
    if (fallbackTimer) clearTimeout(fallbackTimer);
  }

  function handleVideoError(): void {
    videoFailed = true;
    fallbackTimer = setTimeout(finish, fallbackMs);
  }

  onMount(() => {
    reducedMotion = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
    if (reducedMotion) {
      visible = false;
      return;
    }
    const timer = window.setTimeout(finish, 12_000);
    return () => {
      window.clearTimeout(timer);
      if (fallbackTimer) window.clearTimeout(fallbackTimer);
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
    {#if !videoFailed}
      <video
        class="splash-video"
        autoplay
        muted
        playsinline
        preload="auto"
        aria-hidden="true"
        onended={finish}
        onerror={handleVideoError}
      >
        <source src="/splash_intro.mp4" type="video/mp4" />
      </video>
    {:else}
      <span class="splash-fallback" aria-hidden="true">◆ ◆ ◆</span>
    {/if}
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
    background: rgb(26 24 22);
  }

  .splash-video {
    display: block;
    width: min(25vw, 25vh);
    height: auto;
    max-width: 25rem;
    max-height: 90vh;
    object-fit: contain;
  }

  .splash-fallback {
    color: var(--msc-accent);
    font-size: 2rem;
    letter-spacing: 0.5rem;
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
