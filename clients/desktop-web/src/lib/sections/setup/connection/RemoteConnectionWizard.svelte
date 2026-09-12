<script lang="ts">
  import Button from '../../../components/base/Button.svelte';
  import Field from '../../../components/base/Field.svelte';
  import NumberField from '../../../components/base/NumberField.svelte';
  import Select from '../../../components/base/Select.svelte';
  import SegmentedControl from '../../../components/base/SegmentedControl.svelte';
  import type { RemoteDesktopPairingResult } from '../../../auth/desktop';
  import { formatConnectionFailure } from '../../../hosts/connection-errors';
  import {
    DEFAULT_LOCAL_FORWARDED_PORT,
    DEFAULT_MANAGEMENT_PORT,
    DEFAULT_SSH_PORT,
    sshHostnameFromAddress,
  } from '../../../hosts/types';
  import type {
    HostRecord,
    HostRoute,
    RemoteHostConnectionInput,
    SshAuthentication,
  } from '../../../hosts/types';

  export let hosts: readonly HostRecord[] = [];
  export let initialHost: HostRecord | undefined = undefined;
  export let onConnect:
    ((input: RemoteHostConnectionInput) => Promise<RemoteDesktopPairingResult | void>) | undefined =
    undefined;

  type WizardStep = 'details' | 'review';

  const routeOptions = [
    { value: 'lan', label: 'LAN' },
    { value: 'tailscale', label: 'Tailscale' },
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
  let sshUsername = '';
  let authentication: SshAuthentication = 'agent';
  let privateKeyPath = '';
  let sshPassword = '';
  let managementPort: number | string = 48001;
  let localForwardedPort: number | string = 48002;
  let preferredRoute: HostRoute = 'lan';
  let manualTunnel = false;
  let manualAgentAddress = 'http://127.0.0.1:48002';
  let pairingCode = '';
  let expectedHostKeyFingerprint = '';
  let hostKeyReview: RemoteDesktopPairingResult | undefined;
  let busy = false;
  let errorMessage = '';
  let copiedCommand = false;

  $: initialHostKey = initialHost?.id ?? 'new-host';
  $: if (initialHostKey !== initializedHostKey) {
    initializedHostKey = initialHostKey;
    if (initialHost) loadHost(initialHost);
    else resetForm();
  }
  let initializedHostKey = '';

  $: normalizedLanAddress = lanAddress.trim();
  $: normalizedTailscaleAddress = tailscaleAddress.trim();
  $: preferredAddress =
    preferredRoute === 'tailscale' ? normalizedTailscaleAddress : normalizedLanAddress;
  $: normalizedSshHostname = sshHostnameFromAddress(
    preferredAddress || normalizedLanAddress || normalizedTailscaleAddress,
  );
  $: normalizedUsername = sshUsername.trim() || 'username';
  $: sshTarget = `${normalizedUsername}@${normalizedSshHostname}`;
  $: numericManagementPort = portNumber(managementPort);
  $: numericLocalForwardedPort = portNumber(localForwardedPort);
  $: tunnelCommand =
    `ssh -N -L 127.0.0.1:${numericLocalForwardedPort ?? localForwardedPort}:127.0.0.1:${numericManagementPort ?? managementPort} ${sshTarget}`;
  $: forwardedPortConflict =
    numericLocalForwardedPort !== undefined &&
    hosts.some((host) => host.localForwardedPort === numericLocalForwardedPort);
  $: suggestedForwardedPort = nextAvailablePort(
    numericLocalForwardedPort ?? DEFAULT_LOCAL_FORWARDED_PORT,
  );

  function resetForm(): void {
    step = 'details';
    displayName = '';
    lanAddress = '';
    tailscaleAddress = '';
    sshUsername = '';
    authentication = 'agent';
    privateKeyPath = '';
    sshPassword = '';
    managementPort = 48001;
    localForwardedPort = 48002;
    preferredRoute = 'lan';
    manualTunnel = false;
    manualAgentAddress = 'http://127.0.0.1:48002';
    pairingCode = '';
    expectedHostKeyFingerprint = '';
    hostKeyReview = undefined;
    errorMessage = '';
  }

  function loadHost(host: HostRecord): void {
    resetForm();
    displayName = host.displayName;
    lanAddress = host.lanAddresses[0] ?? '';
    tailscaleAddress = host.tailscaleAddresses[0] ?? '';
    sshUsername = host.ssh.username;
    authentication = host.ssh.authentication;
    privateKeyPath = host.ssh.privateKeyPath ?? '';
    managementPort = host.managementPort;
    localForwardedPort = host.localForwardedPort ?? 48002;
    preferredRoute =
      host.preferredRouteOrder[0] ?? (host.tailscaleAddresses.length ? 'tailscale' : 'lan');
    manualTunnel = !host.tryDirectFirst && Boolean(host.manualAgentAddress);
    manualAgentAddress = host.manualAgentAddress ?? manualAgentAddress;
  }

  function validationMessage(): string {
    if (!displayName.trim()) return 'Give this host a name so you can recognize it later.';
    if (!normalizedLanAddress && !normalizedTailscaleAddress) {
      return 'Enter a LAN address, a Tailscale address, or both.';
    }
    if (preferredRoute === 'tailscale' && !normalizedTailscaleAddress) {
      return 'Enter a Tailscale address before choosing Tailscale for SSH.';
    }
    if (!sshUsername.trim()) return 'Enter the SSH username for the remote computer.';
    if (portNumber(managementPort) === undefined || portNumber(localForwardedPort) === undefined) {
      return 'Ports must be whole numbers from 1 to 65535.';
    }
    if (authentication === 'password' && !sshPassword) {
      if (!initialHost) return 'Enter the SSH password for this connection attempt.';
    }
    if (authentication === 'private-key' && !privateKeyPath.trim()) {
      return 'Enter the path or OS reference for the private key.';
    }
    if (manualTunnel && !manualAgentAddress.trim()) {
      return 'Enter the local address of the tunnel you already maintain.';
    }
    return '';
  }

  function portNumber(value: number | string): number | undefined {
    if (typeof value === 'string' && !value.trim()) return undefined;
    const parsed = typeof value === 'number' ? value : Number(value);
    return Number.isInteger(parsed) && parsed >= 1 && parsed <= 65535 ? parsed : undefined;
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

  function selectedBaseUrl(): string {
    if (manualTunnel) return manualAgentAddress.trim();
    return `http://127.0.0.1:${portNumber(localForwardedPort) ?? DEFAULT_LOCAL_FORWARDED_PORT}`;
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
        ...(initialHost ? { existingHostId: initialHost.id } : {}),
        displayName: displayName.trim(),
        lanAddress: normalizedLanAddress,
        ...(normalizedTailscaleAddress ? { tailscaleAddress: normalizedTailscaleAddress } : {}),
        preferredRoute,
        ssh: {
          hostname: normalizedSshHostname,
          port: DEFAULT_SSH_PORT,
          username: sshUsername.trim(),
          authentication,
          ...(authentication === 'private-key' && privateKeyPath.trim()
            ? { privateKeyPath: privateKeyPath.trim() }
            : {}),
        },
        ...(authentication === 'password' && sshPassword ? { sshPassword } : {}),
        managementPort: portNumber(managementPort) ?? DEFAULT_MANAGEMENT_PORT,
        localForwardedPort: portNumber(localForwardedPort) ?? DEFAULT_LOCAL_FORWARDED_PORT,
        manualTunnel,
        ...(manualTunnel ? { manualAgentAddress: manualAgentAddress.trim() } : {}),
        baseUrl: selectedBaseUrl(),
        pairingCode: pairingCode.trim(),
        ...(expectedHostKeyFingerprint ? { expectedHostKeyFingerprint } : {}),
      });
      if (result && result.state !== 'paired') {
        hostKeyReview = result;
        return;
      }
      hostKeyReview = undefined;
      expectedHostKeyFingerprint = '';
      pairingCode = '';
    } catch (error) {
      errorMessage = formatConnectionFailure(error, 'ssh');
    } finally {
      busy = false;
    }
  }
</script>

<div class="wizard" aria-label="Connect to another host wizard">
  {#if step === 'details'}
    <section class="form-section">
      <p class="msc2-type-overline">The host</p>
      <label class="field-label">
        Host name
        <Field bind:value={displayName} placeholder="Home server" />
        <span class="field-help">The name shown in MSC's host switcher.</span>
      </label>
      <div class="field-grid two-up">
        <label class="field-label">
          <span class="field-label-title">LAN address or hostname</span>
          <Field bind:value={lanAddress} placeholder="192.168.1.42 or minecraft.local" />
          <span class="field-help">Use this when both computers share a network.</span>
        </label>
        <label class="field-label">
          <span class="field-label-title"
            >Tailscale address or hostname <span class="optional">Optional</span></span
          >
          <Field
            bind:value={tailscaleAddress}
            placeholder="100.80.20.10 or server.tailnet.ts.net"
          />
          <span class="field-help">An optional private route; MSC does not require Tailscale.</span>
        </label>
      </div>
      <details class="address-help">
        <summary>How do I find these addresses?</summary>
        <div class="address-help-content">
          <p class="section-help">
            Find these on the computer running the MSC agent, not on this computer.
          </p>
          <div>
            <h3 class="address-help-heading">LAN address</h3>
            <ul>
              <li>
                <strong>Windows:</strong> open Command Prompt, run <code>ipconfig</code>, and find
                the IPv4 address under the active Wi-Fi or Ethernet connection.
              </li>
              <li>
                <strong>macOS:</strong> open System Settings → Network, select the active connection,
                and find its IP address.
              </li>
              <li>
                <strong>Linux:</strong> run <code>hostname -I</code> in Terminal and use the address for
                the active network.
              </li>
            </ul>
          </div>
          <div>
            <h3 class="address-help-heading">Tailscale address</h3>
            <p class="section-help">
              If Tailscale is installed and connected on the agent computer, find its address in the
              Tailscale app or run <code>tailscale ip -4</code> there. Tailscale is optional.
            </p>
          </div>
        </div>
      </details>
    </section>

    <section class="form-section">
      <p class="msc2-type-overline">SSH access</p>
      <p class="section-help">
        MSC uses SSH to create a private tunnel to the address selected below. Your password is used
        only for the connection attempt; MSC saves a key path, never the key contents.
      </p>
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
      <p class="msc2-type-overline">SSH tunnel</p>
      <div class="route-row">
        <div>
          <span class="field-label">Connect over</span>
          <SegmentedControl
            options={routeOptions}
            value={preferredRoute}
            onchange={(value) => (preferredRoute = value as HostRoute)}
          />
        </div>
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

    <div class="wizard-actions">
      <Button variant="primary" onclick={goToReview}>Review connection</Button>
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
        <span>SSH route</span><strong
          >{manualTunnel
            ? 'Existing tunnel'
            : preferredRoute === 'lan'
              ? 'LAN'
              : 'Tailscale'}</strong
        >
      </div>
      <div class="summary-row"><span>Agent port</span><strong>{managementPort}</strong></div>
      <div class="summary-row">
        <span>Local tunnel port</span><strong>{localForwardedPort}</strong>
      </div>
      <div class="summary-row">
        <span>SSH identity</span><strong
          >{sshUsername.trim()}@{normalizedSshHostname} · {authentication}</strong
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
      {#if hostKeyReview}
        <div class="host-key-review" role="alert">
          {#if hostKeyReview.state === 'awaiting-host-key'}
            <p class="msc2-type-overline">Could not save this connection</p>
            <p>
              MSC could not remember this computer's SSH identity. Check that secure storage is
              available, then try again.
            </p>
          {:else}
            <p class="msc2-type-overline">SSH identity changed</p>
            <p>
              This server's SSH identity no longer matches the one MSC remembers. If you recently
              reinstalled SSH or replaced the server, confirm it is still your computer before
              continuing.
            </p>
            <Button
              variant="secondary"
              disabled={busy || !hostKeyReview.hostKeyFingerprint}
              onclick={() => {
                expectedHostKeyFingerprint = hostKeyReview?.hostKeyFingerprint ?? '';
                void connect();
              }}>Trust updated identity and continue</Button
            >
          {/if}
        </div>
      {/if}
      <label class="field-label">
        One-use pairing code <span class="optional">Manual fallback</span>
        <Field
          type="password"
          bind:value={pairingCode}
          placeholder="Only needed if remote pairing cannot run"
        />
        <span class="field-help"
          >Normally MSC creates and exchanges this code over the managed SSH session. If the remote <span
            class="mono">msc</span
          > command is unavailable, create it on the host and paste it here; it expires after one use.</span
        >
      </label>
    </section>

    <div class="wizard-actions">
      <Button variant="secondary" disabled={busy} onclick={returnToDetails}>Back</Button>
      <Button variant="primary" disabled={busy} onclick={() => void connect()}>
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
  .pairing-section {
    display: grid;
    gap: 12px;
  }

  .wizard {
    gap: 18px;
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

  .field-label-title {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }

  .field-help {
    color: var(--msc2-text-tertiary);
    font-size: 11px;
  }

  .address-help summary {
    color: var(--msc2-text-secondary);
    cursor: pointer;
    font-size: 12px;
  }

  .address-help-content {
    display: grid;
    gap: 10px;
    padding-top: 10px;
  }

  .address-help-heading {
    margin: 0 0 4px;
    color: var(--msc2-text-primary);
    font-size: 12px;
    font-weight: 500;
  }

  .address-help-content ul {
    display: grid;
    gap: 5px;
    margin: 0;
    padding-left: 18px;
    color: var(--msc2-text-secondary);
    font-size: 12px;
    line-height: 1.5;
  }

  .address-help-content strong {
    color: var(--msc2-text-primary);
    font-weight: 500;
  }

  .address-help-content code {
    color: var(--msc2-text-primary);
    font-family: var(--msc2-font-mono, monospace);
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
  }
</style>
