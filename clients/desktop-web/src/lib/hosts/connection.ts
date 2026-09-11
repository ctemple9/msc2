import {
  DesktopSessionAuth,
  loadTauriDesktopCredentialBridge,
  loadTauriSshTunnelBridge,
  type DesktopSshTunnelBridge,
  type SshTunnelStatus,
} from '../auth/desktop';
import { DEFAULT_LOCAL_FORWARDED_PORT, hostRouteCandidates, type HostRecord } from './types';

export interface HostConnectionResult {
  readonly baseUrl: string;
  readonly route: 'lan' | 'tailscale' | 'ssh' | 'manual';
  readonly detail: string;
}

export interface HostConnectionOptions {
  /** Kept only in memory so a password-authenticated tunnel can be reused this session. */
  readonly sshPassword?: string;
}

/** Coordinates direct routes and the app-owned tunnel for one saved host. */
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

    if (host.tryDirectFirst) {
      for (const candidate of hostRouteCandidates(host)) {
        const probe = await this.probe(auth, host.id, candidate.baseUrl);
        if (probe.reachable) {
          return {
            baseUrl: candidate.baseUrl,
            route: candidate.route,
            detail: `Connected over ${candidate.route === 'lan' ? 'LAN' : 'Tailscale'}.`,
          };
        }
      }
    }

    if (!host.tryDirectFirst && host.manualAgentAddress) {
      const baseUrl = host.manualAgentAddress;
      const probe = await this.probe(auth, host.id, baseUrl);
      if (probe.reachable)
        return { baseUrl, route: 'manual', detail: 'Connected through the saved tunnel.' };
    }

    const tunnel = await this.ensureTunnel(host);
    if (!['connecting', 'connected'].includes(tunnel.state)) {
      throw new Error(tunnel.exitReason ?? 'The managed SSH tunnel could not be started.');
    }
    const baseUrl = `http://127.0.0.1:${tunnel.localPort || host.localForwardedPort || DEFAULT_LOCAL_FORWARDED_PORT}`;
    const connected = await this.waitForRoute(auth, host.id, baseUrl);
    if (!connected) {
      throw new Error(tunnel.stderr || 'The managed SSH tunnel did not reach the remote agent.');
    }
    return { baseUrl, route: 'ssh', detail: 'Connected through the managed SSH tunnel.' };
  }

  async stop(hostId: string): Promise<void> {
    try {
      await (await this.sshBridge()).stop(hostId);
    } catch {
      // No session is the normal state for a host that used a direct route.
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
      sshHost: host.ssh.hostname,
      sshPort: host.ssh.port,
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
  ): Promise<boolean> {
    for (let attempt = 0; attempt < 20; attempt += 1) {
      if ((await this.probe(auth, hostId, baseUrl)).reachable) return true;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    return false;
  }

  private async probe(auth: DesktopSessionAuth, hostId: string, baseUrl: string) {
    try {
      return await auth.probeHostRoute(hostId, baseUrl);
    } catch {
      return { reachable: false, status: null, detail: 'Route did not respond.' };
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
