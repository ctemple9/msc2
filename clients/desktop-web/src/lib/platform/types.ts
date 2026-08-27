export type PlatformKind = 'browser' | 'tauri';

export interface PickedFile {
  readonly name: string;
  readonly bytes: Uint8Array;
}

export interface FilePickerRequest {
  readonly label: string;
  readonly extensions?: readonly string[];
}

export interface DesktopNotification {
  readonly title: string;
  readonly body?: string;
}

export interface MenuEntry {
  readonly id: string;
  readonly label: string;
  readonly onSelect: () => void;
}

export type AgentAction = 'install' | 'update';
export type AgentServiceAction = 'install' | 'start' | 'stop' | 'repair';
export type AgentReadiness =
  'missing' | 'stopped' | 'starting' | 'ready' | 'incompatible' | 'unavailable';

export interface AgentServiceStatus {
  readonly available: boolean;
  readonly platform: string;
  readonly serviceName: string;
  readonly state: 'not-installed' | 'stopped' | 'running' | 'unavailable';
  readonly pid?: number;
  readonly detail: string;
}

/**
 * The client calls this small vocabulary instead of reaching into a desktop
 * runtime. Credentials remain intentionally unavailable until P11.23 defines
 * their authorization and secure-storage protocol.
 */
export interface PlatformAdapter {
  readonly kind: PlatformKind;
  pickFolder(label: string): Promise<string | null>;
  pickFilePath(request: FilePickerRequest): Promise<string | null>;
  pickFile(
    request: FilePickerRequest,
    browserFallback: () => Promise<PickedFile | null>,
  ): Promise<PickedFile | null>;
  notify(notification: DesktopNotification, browserFallback: () => Promise<void>): Promise<void>;
  showMenu(entries: readonly MenuEntry[], browserFallback: () => Promise<void>): Promise<void>;
  closeWindow(browserFallback: () => Promise<void>): Promise<void>;
  openExternal(url: string): Promise<void>;
  /** Opens the local agent UI in a browser with a one-use browser session. */
  openLocalAgentBrowser(): Promise<void>;
  /** Reveals `path` (an absolute local filesystem path) in the OS file
   *  manager. Only meaningful for a locally-connected agent -- callers must
   *  not invoke this for a remote host's path, since nothing local exists
   *  there to reveal. */
  revealInFileManager(path: string, browserFallback: () => Promise<void>): Promise<void>;
  onCloseRequested(handler: () => void): Promise<() => void>;
  credentialFor(hostId: string): Promise<string | null>;
  requestAgentAction(action: AgentAction, browserFallback: () => Promise<void>): Promise<void>;
  agentHealthCheck(): Promise<boolean>;
  agentServiceStatus(): Promise<AgentServiceStatus>;
  manageAgentService(action: AgentServiceAction): Promise<AgentServiceStatus>;
}

export interface TauriPlatformDependencies {
  pickFolder(label: string): Promise<string | null>;
  pickFilePath(request: FilePickerRequest): Promise<string | null>;
  pickFile(request: FilePickerRequest): Promise<PickedFile | null>;
  notify(notification: DesktopNotification): Promise<void>;
  showMenu(entries: readonly MenuEntry[]): Promise<void>;
  closeWindow(): Promise<void>;
  openExternal(url: string): Promise<void>;
  openLocalAgentBrowser(): Promise<void>;
  revealInFileManager(path: string): Promise<void>;
  onCloseRequested(handler: () => void): Promise<() => void>;
  agentHealthCheck(): Promise<boolean>;
  agentServiceStatus(): Promise<AgentServiceStatus>;
  manageAgentService(action: AgentServiceAction): Promise<AgentServiceStatus>;
}
