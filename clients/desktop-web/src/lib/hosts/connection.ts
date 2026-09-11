import {
  DesktopSessionAuth,
  loadTauriDesktopCredentialBridge,
  loadTauriSshTunnelBridge,
  type DesktopSshTunnelBridge,
  type DesktopRouteProbeResult,
  type SshTunnelStatus,
} from '../auth/desktop';
import { formatConnectionFailure, type ConnectionErrorCategory } from './connection-errors';
import {
  DEFAULT_LOCAL_FORWARDED_PORT,
  DEFAULT_SSH_PORT,
  preferredSshHostname,
  type HostRecord,
} from './types';

export interface HostConnectionResult {
  readonly baseUrl: string;
  readonly route: 'ssh' | 'manual';
  readonly detail: string;
}

export interface HostConnectionOptions {
  /** Kept only in memory so a password-authenticated tunnel can be reused this session. */
  readonly sshPassword?: string;
}

export { formatConnectionFailure } from './connection-errors';
export type { ConnectionErrorCategory } from './connection-errors';

/** Coordinates a saved manual tunnel or the app-owned SSH tunnel for one host. */
export class HostConnectionManager {
  private readonly passwords = new Map<string, string>();
  private sshBridgePromise: Promise<DesktopSshTunnelBridge> | undefined;

  rememberSessionPassword(hostId: string, password: string | undefined): void {
    if (password) this.passwords.set(hostId, password);
  }

  forgetHost(hostId: string): void {
    this.passwords.delete(hostId);
  }

  async connect(
    host: HostRecord,
    options: HostConnectionOptions = {},
  ): Promise<HostConnectionResult> {
    const auth = new DesktopSessionAuth(await loadTauriDesktopCredentialBridge());
    if (options.sshPassword) this.rememberSessionPassword(host.id, options.sshPassword);

    if (host.manualAgentAddress) {
      const baseUrl = host.manualAgentAddress;
      const probe = await this.probe(auth, host.id, baseUrl);
      if (probe.reachable)
        return { baseUrl, route: 'manual', detail: 'Connected through the saved tunnel.' };
      if (probe.category && probe.category !== 'network') throw new Error(probe.detail);
    }

    const tunnel = await this.ensureTunnel(host);
    if (!['connecting', 'connected'].includes(tunnel.state)) {
      throw new Error(
        formatConnectionFailure(tunnel.exitReason ?? tunnel.stderr, tunnel.errorCategory ?? 'ssh'),
      );
    }
    const baseUrl = `http://127.0.0.1:${tunnel.localPort || host.localForwardedPort || DEFAULT_LOCAL_FORWARDED_PORT}`;
    const route = await this.waitForRoute(auth, host.id, baseUrl);
    if (!route.reachable) {
      if (route.category && route.category !== 'network') throw new Error(route.detail);
      const latestTunnel = (await this.readStatus(await this.sshBridge(), host.id)) ?? tunnel;
      throw new Error(
        formatConnectionFailure(
          latestTunnel.exitReason ??
            (latestTunnel.stderr || 'The managed SSH tunnel did not reach the remote agent.'),
          latestTunnel.errorCategory ?? 'ssh',
        ),
      );
    }
    return { baseUrl, route: 'ssh', detail: 'Connected through the managed SSH tunnel.' };
  }

  async stop(hostId: string): Promise<void> {
    try {
      await (await this.sshBridge()).stop(hostId);
    } catch {
      // No session is the normal state for a host that is not using a tunnel.
    }
  }

  private async ensureTunnel(host: HostRecord): Promise<SshTunnelStatus> {
    const bridge = await this.sshBridge();
    const existing = await this.readStatus(bridge, host.id);
    const localPort = host.localForwardedPort ?? DEFAULT_LOCAL_FORWARDED_PORT;
    if (
      existing &&
      ['connecting', 'connected'].includes(existing.state) &&
      existing.localPort === localPort &&
      existing.remotePort === host.managementPort
    ) {
      return existing;
    }

    return bridge.start({
      hostId: host.id,
      sshHost: preferredSshHostname(host),
      sshPort: DEFAULT_SSH_PORT,
      username: host.ssh.username,
      authentication: host.ssh.authentication,
      ...(host.ssh.privateKeyPath ? { privateKeyPath: host.ssh.privateKeyPath } : {}),
      ...(this.passwords.get(host.id) ? { password: this.passwords.get(host.id) } : {}),
      localPort,
      remotePort: host.managementPort,
      rememberHostKey: true,
    });
  }

  private async waitForRoute(
    auth: DesktopSessionAuth,
    hostId: string,
    baseUrl: string,
  ): Promise<DesktopRouteProbeResult> {
    let last: DesktopRouteProbeResult = {
      reachable: false,
      status: null,
      category: 'network',
      detail: 'The selected route did not respond.',
    };
    for (let attempt = 0; attempt < 20; attempt += 1) {
      last = await this.probe(auth, hostId, baseUrl);
      if (last.reachable || last.category !== 'network') return last;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    return last;
  }

  private async probe(
    auth: DesktopSessionAuth,
    hostId: string,
    baseUrl: string,
  ): Promise<DesktopRouteProbeResult> {
    try {
      return await auth.probeHostRoute(hostId, baseUrl);
    } catch (error) {
      return {
        reachable: false,
        status: null,
        category: 'authentication',
        detail: formatConnectionFailure(error, 'authentication'),
      };
    }
  }

  private async readStatus(
    bridge: DesktopSshTunnelBridge,
    hostId: string,
  ): Promise<SshTunnelStatus | null> {
    try {
      return await bridge.status(hostId);
    } catch {
      return null;
    }
  }

  private sshBridge(): Promise<DesktopSshTunnelBridge> {
    this.sshBridgePromise ??= loadTauriSshTunnelBridge();
    return this.sshBridgePromise;
  }
}
