<script lang="ts">
  import { bundleIdentity } from './lib/bundle-identity';

  let shellMessage = 'Ready for an agent connection';

  function acknowledgeShell(): void {
    shellMessage = 'The shared client shell is running';
  }
</script>

<svelte:head>
  <meta name="description" content="Minecraft Server Controller" />
</svelte:head>

<main class="app-shell" data-bundle-id={bundleIdentity.id} data-client-surface="shared">
  <section class="welcome-card" aria-labelledby="welcome-title">
    <p class="eyebrow">Minecraft Server Controller</p>
    <h1 id="welcome-title">MSC 2</h1>
    <p class="welcome-copy">
      One shared client foundation for the desktop shell and the browser interface.
    </p>
    <p class="status" role="status">{shellMessage}</p>
    <button type="button" onclick={acknowledgeShell}>Acknowledge shell</button>
    <small>Bundle {bundleIdentity.id} · v{bundleIdentity.version}</small>
  </section>
</main>
