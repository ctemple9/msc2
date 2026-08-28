<script lang="ts">
  // Real port of AddServerWizardView.swift's shared step3Network -- Port
  // Forwarding vs Tunnel(playit.gg) path cards, then the port fields that
  // vary by context: Java port + Bedrock/Geyser port when cross-play is on,
  // a single Bedrock server port when serverType is Bedrock, or a single
  // Java port otherwise. Built off WizardDraft alone, with no Fresh-specific
  // assumption baked in, so P12.18h's Import path can reuse this component
  // unchanged per this step's own plan text.
  //
  // Connectivity cards reuse the flat neutral selected-state card language
  // already established by Choose Path/Configure (antiAIslop rule #6/#11),
  // not the oracle's accent-tinted WizardPathCard icon treatment.
  //
  // Not ported: the oracle's "Port Forwarding Guide" button, which opens
  // RouterPortForwardGuideSheet. P12.16 (guides/handbook), the step that
  // owns that content, has not landed -- there is no pre-P12.16 location
  // either; `RouterGuideCatalogDTO` (openapi.json) is still an empty
  // placeholder schema with no route implementation behind it. Left out
  // rather than wired to a dead link; add it back once P12.16 ships real
  // guide content.
  import NumberField from '../../../components/base/NumberField.svelte';
  import { onboardingAnchor } from '../../../help/tourAnchors';
  import type { WizardDraft } from './model';

  export let draft: WizardDraft;

  $: isBedrock = draft.serverType === 'bedrock';

  function setEnablePlayit(enabled: boolean): void {
    draft.enablePlayit = enabled;
  }
</script>

<div class="network">
  <div class="intro">
    <h2>How will friends connect?</h2>
    <p>Choose how players outside your local network will join your server.</p>
  </div>

  <div class="cards two-up" use:onboardingAnchor={'ob_server_connectivity'}>
    <button
      type="button"
      class="card"
      class:selected={!draft.enablePlayit}
      onclick={() => setEnablePlayit(false)}
    >
      <span class="card-title">Port Forwarding</span>
      <span class="card-subtitle"
        >Open a port on your router. Full control, no relay, best latency.</span
      >
    </button>
    <button
      type="button"
      class="card"
      class:selected={draft.enablePlayit}
      onclick={() => setEnablePlayit(true)}
    >
      <span class="card-title">Tunnel (playit.gg)</span>
      <span class="card-subtitle">No router access needed. Free relay service. Adds ~10–50 ms.</span
      >
    </button>
  </div>

  <div class="ports" use:onboardingAnchor={'ob_server_connectivity_ports'}>
    {#if isBedrock}
      <section class="block">
        <p class="msc2-type-overline">
          {draft.enablePlayit ? 'Local Port (UDP)' : 'Server Port (UDP)'}
        </p>
        <NumberField
          value={draft.bedrockPort}
          min={1}
          max={65535}
          width="120px"
          onchange={(value) => (draft.bedrockPort = Number(value) || 19132)}
        />
      </section>
    {:else}
      <div class="port-row">
        <section class="block">
          <p class="msc2-type-overline">
            {draft.enablePlayit ? 'Local Port (TCP)' : 'Java Port (TCP)'}
          </p>
          <NumberField
            value={draft.javaPort}
            min={1}
            max={65535}
            width="120px"
            onchange={(value) => (draft.javaPort = Number(value) || 25565)}
          />
        </section>
        {#if draft.enableCrossPlay}
          <section class="block">
            <p class="msc2-type-overline">Bedrock / Geyser Port (UDP)</p>
            <NumberField
              value={draft.crossPlayBedrockPort}
              min={1}
              max={65535}
              width="120px"
              onchange={(value) => (draft.crossPlayBedrockPort = Number(value) || 19132)}
            />
          </section>
        {/if}
      </div>
    {/if}

    {#if draft.enablePlayit}
      <p class="hint">
        playit.gg will assign a public address (e.g. abc.joinmc.link:25565) the first time you start
        your server. Your local port is what the server listens on — no router config needed.
      </p>
    {:else}
      <p class="hint">
        Forward {isBedrock ? 'this port' : 'these ports'} on your router so players outside your network
        can connect.
      </p>
    {/if}
  </div>
</div>

<style>
  .network {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .intro {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .intro h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--msc2-text-primary);
  }
  .intro p {
    margin: 0;
    font-size: 12.5px;
    color: var(--msc2-text-tertiary);
  }

  .cards {
    display: flex;
    gap: 10px;
  }
  .cards.two-up > .card {
    flex: 1;
  }
  .card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    text-align: left;
    padding: 12px 14px;
    background: var(--msc2-tier-chrome);
    border: 1px solid var(--msc2-hairline-subtle);
    border-radius: 10px;
    font: inherit;
    cursor: pointer;
  }
  .card.selected {
    border-color: rgba(255, 255, 255, 0.32);
    background: rgba(255, 255, 255, 0.05);
  }
  .card-title {
    font-size: 13px;
    font-weight: 500;
    color: var(--msc2-text-primary);
  }
  .card-subtitle {
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }

  .ports {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .port-row {
    display: flex;
    gap: 24px;
  }
  .block {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .hint {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.5;
    color: var(--msc2-text-tertiary);
  }
</style>
