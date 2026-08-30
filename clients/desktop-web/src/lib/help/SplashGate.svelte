<script lang="ts">
  import { onMount } from 'svelte';

  export let fallbackMs = 900;
  export let onComplete: () => void = () => {};
  let visible = true;
  let completed = false;
  let reducedMotion = false;
  let videoFailed = false;
  let videoElement: HTMLVideoElement | undefined;
  let playbackStarted = false;
  let fallbackTimer: ReturnType<typeof setTimeout> | undefined;

  function finish(): void {
    if (completed) return;
    completed = true;
    visible = false;
    if (fallbackTimer) clearTimeout(fallbackTimer);
    onComplete();
  }

  function handleVideoError(): void {
    if (videoFailed || completed) return;
    videoFailed = true;
    fallbackTimer = setTimeout(finish, fallbackMs);
  }

  function startPlayback(): void {
    const video = videoElement;
    if (playbackStarted || completed || !video) return;
    playbackStarted = true;
    void video.play().catch(() => {
      playbackStarted = false;
      handleVideoError();
    });
  }

  onMount(() => {
    reducedMotion = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
    if (reducedMotion) {
      finish();
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
        bind:this={videoElement}
        muted
        playsinline
        preload="auto"
        aria-hidden="true"
        oncanplay={startPlayback}
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
