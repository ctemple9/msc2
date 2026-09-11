<script lang="ts">
  import Button from '../../../components/base/Button.svelte';
  import Field from '../../../components/base/Field.svelte';
  import NumberField from '../../../components/base/NumberField.svelte';
  import Select from '../../../components/base/Select.svelte';
  import SegmentedControl from '../../../components/base/SegmentedControl.svelte';
  import type { RemoteDesktopPairingResult } from '../../../auth/desktop';
  import type {
    HostRecord,
    HostRoute,
    RemoteHostConnectionInput,
    SshAuthentication,
  } from '../../../hosts/types';

  export let hosts: readonly HostRecord[] = [];
  export let onConnect:
    | ((input: RemoteHostConnectionInput) => Promise<RemoteDesktopPairingResult | void>)
    | undefined = undefined;

  type WizardStep = 'details' | 'review';

  const routeOptions = [
    { value: 'lan', label: 'LAN first' },
    { value: 'tailscale', label: 'Tailscale first' },
  ];
  const authenticationOptions = [
    { value: 'agent', label: 'SSH agent' },
    { value: 'private-key', label: 'Private key' },
    { value: 'password', label: 'Password' },
  ];

  let step: WizardStep = 'details';
  let displayName = '';
  let lanAddress = '';
  let tailscaleAddress = '';
  let sshHostname = '';
  let sshPort = 22;
  let sshUsername = '';
  let authentication: SshAuthentication = 'agent';
  let privateKeyPath = '';
  let sshPassword = '';
  let managementPort = 48001;
  let localForwardedPort = 48002;
  let preferredRoute: HostRoute = 'lan';
  let tryDirectFirst = true;
  let manualTunnel = false;
  let manualAgentAddress = 'http://127.0.0.1:48002';
  let pairingCode = '';
  let expectedHostKeyFingerprint = '';
  let hostKeyReview: RemoteDesktopPairingResult | undefined;
  let busy = false;
  let errorMessage = '';
  let copiedCommand = false;

  $: normalizedLanAddress = lanAddress.trim();
  $: normalizedTailscaleAddress = tailscaleAddress.trim();
  $: normalizedSshHostname = sshHostname.trim() || normalizedLanAddress || 'host-address';
  $: normalizedUsername = sshUsername.trim() || 'username';
  $: sshTarget = `${normalizedUsername}@${normalizedSshHostname}`;
  $: tunnelCommand = `ssh -N -L ${localForwardedPort}:127.0.0.1:${managementPort} ${sshTarget}`;
  $: forwardedPortConflict = hosts.some((host) => host.localForwardedPort === localForwardedPort);
  $: suggestedForwardedPort = nextAvailablePort(localForwardedPort);
  $: detailsValid = validationMessage() === '';

  function validationMessage(): string {
    if (!displayName.trim()) return 'Give this host a name so you can recognize it later.';
    if (!normalizedLanAddress && !normalizedTailscaleAddress) {
      return 'Enter a LAN address, a Tailscale address, or both.';
    }
    if (preferredRoute === 'tailscale' && !normalizedTailscaleAddress) {
      return 'Enter a Tailscale address before choosing Tailscale first.';
    }
    if (!sshHostname.trim()) return 'Enter the hostname or IP address used by SSH.';
    if (!sshUsername.trim()) return 'Enter the SSH username for the remote computer.';
    if (!validPort(sshPort) || !validPort(managementPort) || !validPort(localForwardedPort)) {
      return 'Ports must be whole numbers from 1 to 65535.';
    }
    if (authentication === 'password' && !sshPassword) {
      return 'Enter the SSH password for this connection attempt.';
    }
    if (authentication === 'private-key' && !privateKeyPath.trim()) {
      return 'Enter the path or OS reference for the private key.';
    }
    if (manualTunnel && !manualAgentAddress.trim()) {
      return 'Enter the local address of the tunnel you already maintain.';
    }
    return '';
  }

  function validPort(value: number): boolean {
    return Number.isInteger(value) && value >= 1 && value <= 65535;
  }

  function nextAvailablePort(start: number): number {
    const occupied = new Set(
      hosts
        .map((host) => host.localForwardedPort)
        .filter((port): port is number => typeof port === 'number'),
    );
    let candidate = Math.max(1024, start + (occupied.has(start) ? 1 : 0));
    while (occupied.has(candidate) && candidate < 65535) candidate += 1;
    return candidate;
  }

  function prepareForwardedPort(): void {
    if (forwardedPortConflict) localForwardedPort = suggestedForwardedPort;
  }

  function goToReview(): void {
    errorMessage = validationMessage();
    if (errorMessage) return;
    prepareForwardedPort();
    errorMessage = '';
    step = 'review';
  }

  function returnToDetails(): void {
    errorMessage = '';
    hostKeyReview = undefined;
    expectedHostKeyFingerprint = '';
    step = 'details';
  }

  function directOrigin(address: string, port: number): string {
    const trimmed = address.trim();
    if (!trimmed) return `http://127.0.0.1:${port}`;
    try {
      const parsed = new URL(trimmed.includes('://') ? trimmed : `http://${trimmed}`);
      if (!parsed.port) parsed.port = String(port);
      parsed.pathname = '';
      parsed.search = '';
      parsed.hash = '';
      return parsed.origin;
    } catch {
      return `http://${trimmed}:${port}`;
    }
  }

  function selectedBaseUrl(): string {
    if (manualTunnel) return manualAgentAddress.trim();
    const address =
      preferredRoute === 'tailscale' ? normalizedTailscaleAddress : normalizedLanAddress;
    return directOrigin(
      address || normalizedLanAddress || normalizedTailscaleAddress,
      managementPort,
    );
  }

  async function copyCommand(): Promise<void> {
    try {
      await navigator.clipboard.writeText(tunnelCommand);
      copiedCommand = true;
      setTimeout(() => (copiedCommand = false), 1500);
    } catch {
      // The command remains selectable in the explanation field.
    }
  }

  async function connect(): Promise<void> {
    if (!onConnect || busy) return;
    errorMessage = validationMessage();
    if (errorMessage) {
      step = 'details';
      return;
    }
    prepareForwardedPort();
    busy = true;
    errorMessage = '';
    try {
      const result = await onConnect({
        displayName: displayName.trim(),
        lanAddress: normalizedLanAddress,
        ...(normalizedTailscaleAddress ? { tailscaleAddress: normalizedTailscaleAddress } : {}),
        preferredRoute,
        tryDirectFirst: manualTunnel ? false : tryDirectFirst,
        ssh: {
          hostname: sshHostname.trim(),
          port: sshPort,
          username: sshUsername.trim(),
          authentication,
          ...(authentication === 'private-key' && privateKeyPath.trim()
            ? { privateKeyPath: privateKeyPath.trim() }
            : {}),
        },
        ...(authentication === 'password' && sshPassword ? { sshPassword } : {}),
        managementPort,
        localForwardedPort,
        manualTunnel,
        ...(manualTunnel ? { manualAgentAddress: manualAgentAddress.trim() } : {}),
        baseUrl: selectedBaseUrl(),
        pairingCode: pairingCode.trim(),
        ...(expectedHostKeyFingerprint
          ? { expectedHostKeyFingerprint }
          : {}),
      });
      if (result && result.state !== 'paired') {
        hostKeyReview = result;
        return;
      }
      hostKeyReview = undefined;
      expectedHostKeyFingerprint = '';
      pairingCode = '';
    } catch (error) {
      errorMessage = String(error);
    } finally {
      busy = false;
    }
  }
</script>

<div class="wizard" aria-label="Connect to another host wizard">
  <div class="wizard-track" aria-label="Connection setup progress">
    <span class:current={step === 'details'}>1. Connection details</span>
    <span class:current={step === 'review'}>2. Looks good?</span>
  </div>

  {#if step === 'details'}
    <div class="intro">
      <h2>Tell MSC how to reach the other computer</h2>
      <p>
        These details describe the computer that runs the MSC agent. Addresses and SSH settings are
        saved with the host so you can repair a changed IP later; passwords are never saved here.
      </p>
    </div>

    <section class="form-section">
      <p class="msc2-type-overline">The host</p>
      <label class="field-label">
        Host name
        <Field bind:value={displayName} placeholder="Home server" />
        <span class="field-help">The name shown in MSC's host switcher.</span>
      </label>
      <div class="field-grid two-up">
        <label class="field-label">
          LAN address or hostname
          <Field bind:value={lanAddress} placeholder="192.168.1.42 or minecraft.local" />
          <span class="field-help">Use this when both computers share a network.</span>
        </label>
        <label class="field-label">
          Tailscale address or hostname <span class="optional">Optional</span>
          <Field
            bind:value={tailscaleAddress}
            placeholder="100.80.20.10 or server.tailnet.ts.net"
          />
          <span class="field-help">An optional private route; MSC does not require Tailscale.</span>
        </label>
      </div>
    </section>

    <section class="form-section">
      <p class="msc2-type-overline">SSH access</p>
      <p class="section-help">
        MSC can use SSH to carry the management connection securely when the agent is not directly
        reachable. The password is held only for the connection attempt; a key path is remembered,
        never the key contents.
      </p>
      <div class="field-grid two-up">
        <label class="field-label">
          SSH hostname or IP
          <Field bind:value={sshHostname} placeholder="192.168.1.42" />
        </label>
        <label class="field-label">
          SSH port
          <NumberField bind:value={sshPort} min={1} max={65535} width="100%" />
        </label>
      </div>
      <div class="field-grid two-up">
        <label class="field-label">
          Username
          <Field bind:value={sshUsername} placeholder="camerontemple" />
        </label>
        <label class="field-label">
          Authentication
          <Select options={authenticationOptions} bind:value={authentication} />
        </label>
      </div>
      {#if authentication === 'password'}
        <label class="field-label">
          SSH password
          <Field
            type="password"
            bind:value={sshPassword}
            placeholder="Used for this connection only"
          />
        </label>
      {:else if authentication === 'private-key'}
        <label class="field-label">
          Private-key path or OS key reference
          <Field bind:value={privateKeyPath} placeholder="~/.ssh/id_ed25519" />
        </label>
      {:else}
        <p class="field-help">MSC will ask the operating system's SSH agent for the key.</p>
      {/if}
    </section>

    <section class="form-section">
      <p class="msc2-type-overline">Connection route</p>
      <div class="route-row">
        <div>
          <span class="field-label">Try this direct route first</span>
          <SegmentedControl
            options={routeOptions}
            value={preferredRoute}
            onchange={(value) => (preferredRoute = value as HostRoute)}
          />
        </div>
        <label class="check-label">
          <input type="checkbox" bind:checked={tryDirectFirst} />
          <span>
            <strong>Try direct access before SSH</strong>
            <small>Use LAN or Tailscale when it responds; otherwise use the tunnel plan.</small>
          </span>
        </label>
      </div>
      <div class="field-grid two-up">
        <label class="field-label">
          Remote MSC management port
          <NumberField bind:value={managementPort} min={1} max={65535} width="100%" />
          <span class="field-help"
            >The agent listens on 48001 by default. This is not a Minecraft port.</span
          >
        </label>
        <label class="field-label">
          Local forwarded port
          <NumberField bind:value={localForwardedPort} min={1024} max={65535} width="100%" />
          <span class="field-help">MSC uses 48002 by default on this computer.</span>
        </label>
      </div>
      {#if forwardedPortConflict}
        <p class="inline-warning">
          Port {localForwardedPort} is already assigned to a saved host. MSC will switch this host to
          {suggestedForwardedPort} before review.
        </p>
      {/if}
    </section>

    <details class="advanced-path">
      <summary>I already maintain my own SSH tunnel</summary>
      <div class="advanced-content">
        <p class="section-help">
          Use this only if you already run the tunnel yourself. The normal path lets MSC manage the
          SSH session and its lifecycle.
        </p>
        <label class="check-label">
          <input type="checkbox" bind:checked={manualTunnel} />
          <span><strong>Connect through my existing tunnel</strong></span>
        </label>
        {#if manualTunnel}
          <label class="field-label">
            Local agent address
            <Field bind:value={manualAgentAddress} placeholder="http://127.0.0.1:48002" />
          </label>
        {/if}
      </div>
    </details>

    <div class="wizard-actions">
      <Button variant="primary" disabled={!detailsValid} onclick={goToReview}
        >Review connection</Button
      >
    </div>
  {:else}
    <div class="intro">
      <h2>Review this host before saving it</h2>
      <p>MSC will use this plan to reach the agent and keep the host's identity stable.</p>
    </div>

    <section class="review-summary">
      <div class="summary-row"><span>Host</span><strong>{displayName.trim()}</strong></div>
      <div class="summary-row">
        <span>Direct addresses</span><strong
          >{normalizedLanAddress || '—'}{normalizedTailscaleAddress
            ? ` · ${normalizedTailscaleAddress}`
            : ''}</strong
        >
      </div>
      <div class="summary-row">
        <span>Preferred route</span><strong
          >{manualTunnel
            ? 'Existing tunnel'
            : preferredRoute === 'lan'
              ? 'LAN'
              : 'Tailscale'}{!manualTunnel && tryDirectFirst ? ' before SSH' : ''}</strong
        >
      </div>
      <div class="summary-row"><span>Agent port</span><strong>{managementPort}</strong></div>
      <div class="summary-row">
        <span>Local tunnel port</span><strong>{localForwardedPort}</strong>
      </div>
      <div class="summary-row">
        <span>SSH identity</span><strong
          >{sshUsername.trim()}@{sshHostname.trim()} · {authentication}</strong
        >
      </div>
    </section>

    <section class="command-explanation">
      <p class="msc2-type-overline">What MSC is doing</p>
      <p>
        The managed tunnel is equivalent to the command below. It forwards this computer's local
        port to the agent's loopback-only management port on the other computer. You do not need to
        open Terminal or run this command yourself.
      </p>
      <div class="command-row">
        <Field value={tunnelCommand} />
        <Button size="sm" variant="secondary" onclick={() => void copyCommand()}>
          {copiedCommand ? 'Copied' : 'Copy'}
        </Button>
      </div>
    </section>

    <section class="pairing-section">
      {#if hostKeyReview?.hostKeyFingerprint}
        <div class="host-key-review" role="status">
          <p class="msc2-type-overline">Review the remote computer's identity</p>
          <p>{hostKeyReview.detail}</p>
          {#if hostKeyReview.storedHostKeyFingerprint}
            <div class="fingerprint-row">
              <span>Previously remembered</span>
              <strong>{hostKeyReview.storedHostKeyFingerprint}</strong>
            </div>
          {/if}
          <div class="fingerprint-row">
            <span>Now observed</span>
            <strong>{hostKeyReview.hostKeyFingerprint}</strong>
          </div>
          <Button
            variant="secondary"
            disabled={busy}
            onclick={() => {
              expectedHostKeyFingerprint = hostKeyReview?.hostKeyFingerprint ?? '';
              void connect();
            }}>I have checked this fingerprint</Button
          >
        </div>
      {/if}
      <label class="field-label">
        One-use pairing code <span class="optional">Manual fallback</span>
        <Field bind:value={pairingCode} placeholder="Only needed if remote pairing cannot run" />
        <span class="field-help"
          >Normally MSC creates and exchanges this code over the managed SSH session. If the
          remote <span class="mono">msc</span> command is unavailable, create it on the host and
          paste it here; it expires after one use.</span
        >
      </label>
    </section>

    <div class="wizard-actions">
      <Button variant="secondary" disabled={busy} onclick={returnToDetails}>Back</Button>
      <Button
        variant="primary"
        disabled={busy}
        onclick={() => void connect()}
      >
        {busy ? 'Creating authorization…' : 'Save and connect'}
      </Button>
    </div>
  {/if}

  {#if errorMessage}
    <p class="error" role="alert">{errorMessage}</p>
  {/if}
</div>

<style>
  .wizard,
  .form-section,
  .intro,
  .field-grid,
  .advanced-content,
  .pairing-section {
    display: grid;
    gap: 12px;
  }

  .wizard {
    gap: 18px;
  }

  .wizard-track {
    display: flex;
    gap: 18px;
    padding-bottom: 10px;
    border-bottom: 1px solid var(--msc2-hairline-faint);
    color: var(--msc2-text-tertiary);
    font-size: 11px;
  }

  .wizard-track span.current {
    color: var(--msc2-text-primary);
    font-weight: 500;
  }

  h2,
  p {
    margin: 0;
  }

  h2 {
    color: var(--msc2-text-primary);
    font-size: 15px;
    font-weight: 600;
  }

  .intro p,
  .section-help,
  .field-help,
  .command-explanation p,
  .inline-warning,
  .error {
    color: var(--msc2-text-secondary);
    font-size: 12px;
    line-height: 1.5;
  }

  .form-section {
    gap: 10px;
    padding-top: 14px;
    border-top: 1px solid var(--msc2-hairline-faint);
  }

  .field-grid.two-up {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .field-label {
    display: grid;
    gap: 5px;
    color: var(--msc2-text-primary);
    font-size: 12px;
  }

  .field-help {
    color: var(--msc2-text-tertiary);
    font-size: 11px;
  }

  .optional {
    color: var(--msc2-text-tertiary);
    font-size: 11px;
  }

  .route-row {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 18px;
  }

  .route-row > div {
    display: grid;
    gap: 7px;
  }

  .check-label {
    display: flex;
    align-items: start;
    gap: 8px;
    max-width: 310px;
    color: var(--msc2-text-secondary);
    font-size: 12px;
    line-height: 1.4;
  }

  .check-label input {
    margin-top: 2px;
    accent-color: var(--msc2-status-ok);
  }

  .check-label span {
    display: grid;
    gap: 2px;
  }

  .check-label strong {
    color: var(--msc2-text-primary);
    font-weight: 500;
  }

  .check-label small {
    color: var(--msc2-text-tertiary);
    font-size: 11px;
  }

  .inline-warning {
    color: var(--msc2-status-warn);
  }

  .advanced-path {
    border-top: 1px solid var(--msc2-hairline-faint);
    padding-top: 12px;
  }

  .advanced-path summary {
    color: var(--msc2-text-secondary);
    cursor: pointer;
    font-size: 12px;
  }

  .advanced-content {
    padding-top: 10px;
  }

  .wizard-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .review-summary {
    display: grid;
    background: var(--msc2-tier-chrome);
    border-radius: 9px;
    overflow: hidden;
  }

  .summary-row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 9px 12px;
    border-top: 1px solid var(--msc2-hairline-subtle);
    font-size: 12px;
  }

  .summary-row:first-child {
    border-top: none;
  }

  .summary-row span {
    color: var(--msc2-text-tertiary);
  }

  .summary-row strong {
    color: var(--msc2-text-primary);
    font-weight: 500;
    text-align: right;
    overflow-wrap: anywhere;
  }

  .command-explanation {
    display: grid;
    gap: 8px;
    padding-top: 14px;
    border-top: 1px solid var(--msc2-hairline-faint);
  }

  .host-key-review {
    display: grid;
    gap: 8px;
    padding-top: 2px;
  }

  .host-key-review > p:not(.msc2-type-overline) {
    color: var(--msc2-text-secondary);
    font-size: 12px;
    line-height: 1.5;
  }

  .fingerprint-row {
    display: grid;
    grid-template-columns: 150px minmax(0, 1fr);
    gap: 10px;
    color: var(--msc2-text-tertiary);
    font-size: 11px;
  }

  .fingerprint-row strong {
    overflow-wrap: anywhere;
    color: var(--msc2-text-primary);
    font-family: var(--msc2-font-mono, monospace);
    font-weight: 400;
  }

  .command-row {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .command-row :global(.field) {
    min-width: 0;
    flex: 1;
    font-family: var(--msc2-font-mono, monospace);
    font-size: 11px;
  }

  .error {
    color: var(--msc2-status-error);
  }

  @media (max-width: 640px) {
    .field-grid.two-up,
    .route-row {
      grid-template-columns: 1fr;
      display: grid;
      align-items: start;
    }

    .wizard-track {
      gap: 10px;
    }
  }
</style>
