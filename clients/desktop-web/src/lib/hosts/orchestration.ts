import {
  DesktopSessionAuth,
  loadTauriDesktopCredentialBridge,
  type RemoteDesktopPairingResult,
} from '../auth/desktop';
import type { AgentReadiness } from '../platform';
import { forgetSavedRemoteHost, saveRemoteHost } from './saved';
import { HostConnectionManager, formatConnectionFailure } from './connection';
import { HostStore } from './registry';
import {
  createRemoteHostRecord,
  hostManagementUrl,
  withPreferredHostRoute,
  DEFAULT_SSH_PORT,
  type HostId,
  type HostRecord,
  type HostRoute,
  type RemoteHostConnectionInput,
} from './types';

interface HostOrchestrationOptions {
  readonly store: HostStore;
  readonly connectionManager: HostConnectionManager;
  readonly localHostId: HostId;
  readonly hosts: () => readonly HostRecord[];
  readonly selectedHostId: () => HostId;
  readonly selectHostId: (id: HostId) => void;
  readonly desktopShell: () => boolean;
  readonly agentReadiness: () => AgentReadiness;
  readonly setShellMessage: (message: string) => void;
  readonly clearLoadedSections: () => void;
  readonly refreshHosts: () => void;
  readonly initializeClient: (generation?: number) => Promise<void>;
  readonly selectSetupSection: (generation: number) => Promise<void>;
}

/** Coordinates host selection, remote pairing, and stale-connection protection. */
export class HostConnectionOrchestrator {
  private generation = 0;

  constructor(private readonly options: HostOrchestrationOptions) {}

  get currentGeneration(): number {
    return this.generation;
  }

  nextGeneration(): number {
    this.generation += 1;
    return this.generation;
  }

  isCurrent(generation: number): boolean {
    return generation === this.generation;
  }

  hostSummaries(): Map<HostId, { connection: string; serverCount: number }> {
    const summaries = new Map<HostId, { connection: string; serverCount: number }>();
    for (const host of this.options.hosts()) {
      const cache = this.options.store.getState(host.id).cache;
      summaries.set(host.id, { connection: cache.connection, serverCount: cache.servers.length });
    }
    return summaries;
  }

  async switchHost(id: HostId): Promise<void> {
    const { options } = this;
    if (id === options.selectedHostId()) return;
    const generation = this.nextGeneration();
    const previousHostId = options.selectedHostId();
    const activeOperation = options.store
      .getState(previousHostId)
      .cache.operations.find(
        (operation) => operation.state === 'queued' || operation.state === 'running',
      );
    options.store.selectHost(id);
    options.clearLoadedSections();
    options.selectHostId(id);
    if (options.desktopShell() && previousHostId !== options.localHostId) {
      await options.connectionManager.stop(previousHostId);
      if (!this.isCurrent(generation)) return;
    }
    await options.initializeClient(generation);
    if (!this.isCurrent(generation)) return;
    if (options.agentReadiness() === 'ready' && activeOperation) {
      options.setShellMessage(
        `Switched hosts. ${activeOperation.statusLine ?? activeOperation.type} continues on ${previousHostId}; returning to that host will restore its progress.`,
      );
    }
    await options.selectSetupSection(generation);
  }

  async addRemoteHost(
    input: RemoteHostConnectionInput,
  ): Promise<string | RemoteDesktopPairingResult> {
    const { options } = this;
    if (input.existingHostId) {
      const existing = options.hosts().find((host) => host.id === input.existingHostId);
      if (!existing) throw new Error('The saved host being edited is no longer registered.');
      const repaired = createRemoteHostRecord({
        id: existing.id,
        displayName: input.displayName,
        baseUrl: input.baseUrl,
        lanAddresses: input.lanAddress ? [input.lanAddress, ...existing.lanAddresses.slice(1)] : [],
        tailscaleAddresses: input.tailscaleAddress
          ? [input.tailscaleAddress, ...existing.tailscaleAddresses.slice(1)]
          : [],
        preferredRouteOrder: [
          input.preferredRoute,
          input.preferredRoute === 'lan' ? 'tailscale' : 'lan',
        ],
        ssh: input.ssh,
        managementPort: input.managementPort,
        localForwardedPort: input.localForwardedPort,
        ...(input.manualTunnel && input.manualAgentAddress
          ? { manualAgentAddress: input.manualAgentAddress }
          : {}),
      });
      options.store.updateHost(repaired);
      saveRemoteHost(repaired);
      options.connectionManager.rememberSessionPassword(repaired.id, input.sshPassword);
      options.refreshHosts();
      return repaired.id;
    }

    const auth = new DesktopSessionAuth(await loadTauriDesktopCredentialBridge());
    let result: RemoteDesktopPairingResult;
    try {
      result = await auth.automateRemotePairing({
        baseUrl: input.baseUrl,
        ssh: {
          sshHost: input.ssh.hostname,
          sshPort: DEFAULT_SSH_PORT,
          username: input.ssh.username,
          authentication: input.ssh.authentication,
          ...(input.ssh.privateKeyPath ? { privateKeyPath: input.ssh.privateKeyPath } : {}),
          ...(input.sshPassword ? { password: input.sshPassword } : {}),
          localPort: input.localForwardedPort,
          remotePort: input.managementPort,
          ...(input.expectedHostKeyFingerprint
            ? { expectedHostKeyFingerprint: input.expectedHostKeyFingerprint }
            : {}),
          rememberHostKey: true,
        },
      });
    } catch (error) {
      if (!input.pairingCode.trim()) {
        throw new Error(
          `${formatConnectionFailure(error, 'ssh')} To use the manual fallback, create a desktop pairing code on the remote host and enter it here.`,
        );
      }
      const manual = await auth.redeemRemotePairing(input.baseUrl, input.pairingCode);
      result = {
        state: 'paired',
        agentHostId: manual.agentHostId,
        hostKeyFingerprint: null,
        storedHostKeyFingerprint: null,
        detail: 'The manual desktop pairing code was accepted.',
      };
    }
    if (result.state !== 'paired') return result;
    const agentHostId = result.agentHostId;
    if (!agentHostId) throw new Error('The remote pairing returned no host identity.');
    const host = createRemoteHostRecord({
      id: agentHostId,
      displayName: input.displayName,
      baseUrl: input.baseUrl,
      lanAddresses: input.lanAddress ? [input.lanAddress] : [],
      tailscaleAddresses: input.tailscaleAddress ? [input.tailscaleAddress] : [],
      preferredRouteOrder: [
        input.preferredRoute,
        input.preferredRoute === 'lan' ? 'tailscale' : 'lan',
      ],
      ssh: input.ssh,
      managementPort: input.managementPort,
      localForwardedPort: input.localForwardedPort,
      ...(input.manualTunnel && input.manualAgentAddress
        ? { manualAgentAddress: input.manualAgentAddress }
        : {}),
    });
    const existing = options.store.listHosts().find((registered) => registered.id === host.id);
    if (existing) options.store.updateHost(host);
    else options.store.addHost(host);
    saveRemoteHost(host);
    options.connectionManager.rememberSessionPassword(host.id, input.sshPassword);
    options.refreshHosts();
    return agentHostId;
  }

  async connectRemoteHost(
    input: RemoteHostConnectionInput,
  ): Promise<RemoteDesktopPairingResult | void> {
    const result = await this.addRemoteHost(input);
    if (typeof result !== 'string') return result;
    if (result === this.options.selectedHostId()) await this.options.initializeClient();
    else await this.switchHost(result);
  }

  async selectHostRoute(id: HostId, route: HostRoute): Promise<void> {
    const host = this.options.hosts().find((candidate) => candidate.id === id);
    if (!host) return;
    const addresses = route === 'lan' ? host.lanAddresses : host.tailscaleAddresses;
    if (!addresses.length) return;
    const updated = withPreferredHostRoute(host, route);
    this.options.store.updateHost(updated);
    saveRemoteHost(updated);
    this.options.refreshHosts();
    if (id === this.options.selectedHostId()) await this.options.initializeClient();
  }

  async pairAgain(pairingCode: string): Promise<void> {
    const { options } = this;
    const currentHostId = options.selectedHostId();
    if (!options.desktopShell() || currentHostId === options.localHostId) {
      throw new Error('Fresh pairing is available only for a remote desktop host.');
    }
    const previousHost = options.hosts().find((host) => host.id === currentHostId);
    if (!previousHost) throw new Error('The selected host is no longer registered.');

    const auth = new DesktopSessionAuth(await loadTauriDesktopCredentialBridge());
    // The host reset already revoked this credential; clear the desktop copy
    // before pairing so an interrupted recovery cannot leave stale state.
    await auth.forgetCredentials([previousHost.id], false);
    const result = await auth.redeemRemotePairing(hostManagementUrl(previousHost), pairingCode);
    const replacementHost = {
      id: result.agentHostId,
      displayName: previousHost.displayName,
      lanAddresses: [...previousHost.lanAddresses],
      tailscaleAddresses: [...previousHost.tailscaleAddresses],
      preferredRouteOrder: [...previousHost.preferredRouteOrder],
      ssh: { ...previousHost.ssh },
      managementPort: previousHost.managementPort,
      ...(previousHost.localForwardedPort === undefined
        ? {}
        : { localForwardedPort: previousHost.localForwardedPort }),
      tryDirectFirst: previousHost.tryDirectFirst,
      ...(previousHost.manualAgentAddress
        ? { manualAgentAddress: previousHost.manualAgentAddress }
        : {}),
    };
    options.store.removeHost(previousHost.id);
    options.store.addHost(replacementHost);
    forgetSavedRemoteHost(previousHost.id);
    saveRemoteHost(replacementHost);
    options.store.selectHost(result.agentHostId);
    options.selectHostId(result.agentHostId);
    options.refreshHosts();
    await options.initializeClient();
  }

  async removeRemoteHost(id: HostId): Promise<void> {
    const { options } = this;
    if (id === options.localHostId) return;
    await options.connectionManager.stop(id);
    options.connectionManager.forgetHost(id);
    const auth = new DesktopSessionAuth(await loadTauriDesktopCredentialBridge());
    await auth.forgetCredentials([id], false);
    if (id === options.selectedHostId()) await this.switchHost(options.localHostId);
    options.store.removeHost(id);
    forgetSavedRemoteHost(id);
    options.refreshHosts();
  }

  async removeCurrentRemoteHost(): Promise<void> {
    if (
      !this.options.desktopShell() ||
      this.options.selectedHostId() === this.options.localHostId
    ) {
      throw new Error('The local agent cannot be removed from this desktop.');
    }
    await this.removeRemoteHost(this.options.selectedHostId());
  }

  async disconnectCurrentRemoteHost(): Promise<void> {
    if (
      !this.options.desktopShell() ||
      this.options.selectedHostId() === this.options.localHostId
    ) {
      throw new Error('The local agent is already selected on this desktop.');
    }
    await this.switchHost(this.options.localHostId);
  }
}
