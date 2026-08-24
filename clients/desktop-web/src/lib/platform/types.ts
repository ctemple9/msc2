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

/**
 * The client calls this small vocabulary instead of reaching into a desktop
 * runtime. Credentials remain intentionally unavailable until P11.23 defines
 * their authorization and secure-storage protocol.
 */
export interface PlatformAdapter {
  readonly kind: PlatformKind;
  pickFile(
    request: FilePickerRequest,
    browserFallback: () => Promise<PickedFile | null>,
  ): Promise<PickedFile | null>;
  notify(notification: DesktopNotification, browserFallback: () => Promise<void>): Promise<void>;
  showMenu(entries: readonly MenuEntry[], browserFallback: () => Promise<void>): Promise<void>;
  closeWindow(browserFallback: () => Promise<void>): Promise<void>;
  onCloseRequested(handler: () => void): Promise<() => void>;
  credentialFor(hostId: string): Promise<string | null>;
  requestAgentAction(action: AgentAction, browserFallback: () => Promise<void>): Promise<void>;
}

export interface TauriPlatformDependencies {
  pickFile(request: FilePickerRequest): Promise<PickedFile | null>;
  notify(notification: DesktopNotification): Promise<void>;
  showMenu(entries: readonly MenuEntry[]): Promise<void>;
  closeWindow(): Promise<void>;
  onCloseRequested(handler: () => void): Promise<() => void>;
}
