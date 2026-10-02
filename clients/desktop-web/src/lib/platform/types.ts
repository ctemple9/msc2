export type PlatformKind = 'tauri';

export interface PickedFile {
  readonly name: string;
  readonly bytes: Uint8Array;
}

/** A local file that can be read a piece at a time without holding it all in memory. */
export interface FileChunkSource {
  readonly name: string;
  readonly size: number;
  readChunk(offset: number, maxBytes: number): Promise<Uint8Array>;
  close(): Promise<void>;
}

export interface FileUploadProgress {
  readonly phase: 'selecting' | 'preparing' | 'reading' | 'uploading' | 'cancelling' | 'complete';
  readonly bytesUploaded: number;
  readonly totalBytes: number;
  readonly chunkSizeBytes?: number;
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

export type AgentServiceAction = 'install' | 'start' | 'stop' | 'repair' | 'uninstall';
export type AgentReadiness =
  'missing' | 'stopped' | 'starting' | 'ready' | 'incompatible' | 'unavailable';

export interface AgentServiceStatus {
  readonly available: boolean;
  readonly platform: string;
  readonly serviceName: string;
  readonly state: 'not-installed' | 'stopped' | 'running' | 'unavailable';
  readonly pid?: number;
  readonly detail: string;
  readonly helper?: BedrockHelperServiceStatus;
}

export interface BedrockHelperServiceStatus {
  readonly state: 'not-installed' | 'stopped' | 'running';
  readonly pid?: number;
  readonly detail: string;
}

export interface UninstallInventory {
  readonly computer: string;
  readonly platform: string;
  readonly fingerprint: string;
  readonly entries: readonly {
    kind: string;
    path: string | null;
    identity: string;
    evidence: string;
    state: 'present' | 'missing' | 'blocked';
    problem: string | null;
  }[];
  readonly exclusions: readonly string[];
  readonly warnings: readonly string[];
}

export interface LocalUninstallRequest {
  readonly confirmation: string;
  readonly fingerprint: string;
  readonly installers: readonly string[];
  readonly keepReport: boolean;
}

export interface UninstallScheduled {
  readonly state: 'scheduled';
  readonly reportPath: string;
  readonly detail: string;
}

export type UpdateState = 'current' | 'staged' | 'unavailable';

export interface UpdateCheckResult {
  readonly state: UpdateState;
  readonly releaseId?: string;
  readonly releaseNotes: string;
  readonly installMode?: string;
  readonly target?: string;
  readonly artifactFilename?: string;
  readonly stagedDirectory?: string;
  readonly detail: string;
}

export interface UpdateInstallResult {
  readonly state: 'scheduled' | 'installer-launched' | 'package-installed';
  readonly releaseId: string;
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
  /** Reads a path delivered by the desktop drag-and-drop bridge. */
  readFile?(path: string): Promise<Uint8Array>;
  readFileStream?(path: string): Promise<FileChunkSource>;
  pickFile(request: FilePickerRequest): Promise<PickedFile | null>;
  pickFileStream(request: FilePickerRequest): Promise<FileChunkSource | null>;
  notify(notification: DesktopNotification): Promise<void>;
  showMenu(entries: readonly MenuEntry[]): Promise<void>;
  closeWindow(): Promise<void>;
  quitApplication(): Promise<void>;
  openExternal(url: string): Promise<void>;
  /** Reveals `path` (an absolute local filesystem path) in the OS file
   *  manager. Only meaningful for a locally-connected agent -- callers must
   *  not invoke this for a remote host's path, since nothing local exists
   *  there to reveal. */
  revealInFileManager(path: string): Promise<void>;
  /** Fires with the real local filesystem path(s) whenever the user drops
   *  something onto the window. Returns an unsubscribe function, mirroring
   *  `onCloseRequested`. */
  onFileDrop(handler: (paths: readonly string[]) => void): Promise<() => void>;
  onCloseRequested(handler: () => void): Promise<() => void>;
  credentialFor(hostId: string): Promise<string | null>;
  agentHealthCheck(): Promise<boolean>;
  agentServiceStatus(): Promise<AgentServiceStatus>;
  manageAgentService(action: AgentServiceAction): Promise<AgentServiceStatus>;
  previewLocalUninstall(installers: readonly string[]): Promise<UninstallInventory>;
  uninstallLocal(request: LocalUninstallRequest): Promise<UninstallScheduled>;
  checkForUpdates(): Promise<UpdateCheckResult>;
  installUpdate(releaseId: string): Promise<UpdateInstallResult>;
}

export interface TauriPlatformDependencies {
  pickFolder(label: string): Promise<string | null>;
  pickFilePath(request: FilePickerRequest): Promise<string | null>;
  readFile?(path: string): Promise<Uint8Array>;
  pickFile(request: FilePickerRequest): Promise<PickedFile | null>;
  pickFileStream?(request: FilePickerRequest): Promise<FileChunkSource | null>;
  readFileStream?(path: string): Promise<FileChunkSource>;
  notify(notification: DesktopNotification): Promise<void>;
  showMenu(entries: readonly MenuEntry[]): Promise<void>;
  closeWindow(): Promise<void>;
  quitApplication(): Promise<void>;
  openExternal(url: string): Promise<void>;
  revealInFileManager(path: string): Promise<void>;
  onFileDrop(handler: (paths: readonly string[]) => void): Promise<() => void>;
  onCloseRequested(handler: () => void): Promise<() => void>;
  agentHealthCheck(): Promise<boolean>;
  agentServiceStatus(): Promise<AgentServiceStatus>;
  manageAgentService(action: AgentServiceAction): Promise<AgentServiceStatus>;
  previewLocalUninstall?(installers: readonly string[]): Promise<UninstallInventory>;
  uninstallLocal?(request: LocalUninstallRequest): Promise<UninstallScheduled>;
  checkForUpdates?(): Promise<UpdateCheckResult>;
  installUpdate?(releaseId: string): Promise<UpdateInstallResult>;
}
