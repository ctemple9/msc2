import type {
  AgentServiceAction,
  AgentServiceStatus,
  DesktopNotification,
  FilePickerRequest,
  FileChunkSource,
  MenuEntry,
  PickedFile,
  PlatformAdapter,
  TauriPlatformDependencies,
  UpdateCheckResult,
  UpdateInstallResult,
} from './types';

/**
 * Builds the desktop adapter from explicit native operations. Keeping this
 * constructor injectable lets the boundary be tested without a Tauri window.
 */
export function createTauriPlatform(dependencies: TauriPlatformDependencies): PlatformAdapter {
  return {
    kind: 'tauri',
    pickFolder: (label) => dependencies.pickFolder(label),
    pickFilePath: (request) => dependencies.pickFilePath(request),
    readFile: async (path) => {
      if (!dependencies.readFile) {
        throw new Error('Reading a dropped file is unavailable in this desktop build.');
      }
      return dependencies.readFile(path);
    },
    readFileStream: async (path) => {
      if (!dependencies.readFileStream)
        throw new Error('Streaming file reads are unavailable in this desktop build.');
      return dependencies.readFileStream(path);
    },
    pickFile: (request) => dependencies.pickFile(request),
    pickFileStream: (request) => {
      if (!dependencies.pickFileStream)
        throw new Error('Streaming file reads are unavailable in this desktop build.');
      return dependencies.pickFileStream(request);
    },
    notify: dependencies.notify,
    showMenu: dependencies.showMenu,
    closeWindow: dependencies.closeWindow,
    quitApplication: dependencies.quitApplication,
    openExternal: (url: string) => dependencies.openExternal(url),
    revealInFileManager: dependencies.revealInFileManager,
    onFileDrop: (handler) => dependencies.onFileDrop(handler),
    onCloseRequested: dependencies.onCloseRequested,
    // P11.23 supplies per-host pairing and secret-store behavior. This seam
    // deliberately cannot fabricate a local token before that contract exists.
    credentialFor: async (_hostId: string) => null,
    agentHealthCheck: dependencies.agentHealthCheck,
    agentServiceStatus: dependencies.agentServiceStatus,
    manageAgentService: dependencies.manageAgentService,
    checkForUpdates:
      dependencies.checkForUpdates ??
      (async (): Promise<UpdateCheckResult> => {
        throw new Error('Checking for updates is unavailable in this desktop build.');
      }),
    installUpdate:
      dependencies.installUpdate ??
      (async (): Promise<UpdateInstallResult> => {
        throw new Error('Installing updates is unavailable in this desktop build.');
      }),
  };
}

export async function loadTauriPlatform(): Promise<PlatformAdapter> {
  const [
    { open },
    { open: openFile, readFile, SeekMode },
    notification,
    { Menu },
    { getCurrentWindow },
    { invoke },
    { getCurrentWebview },
  ] = await Promise.all([
    import('@tauri-apps/plugin-dialog'),
    import('@tauri-apps/plugin-fs'),
    import('@tauri-apps/plugin-notification'),
    import('@tauri-apps/api/menu'),
    import('@tauri-apps/api/window'),
    import('@tauri-apps/api/core'),
    import('@tauri-apps/api/webview'),
  ]);

  return createTauriPlatform({
    async pickFolder(label: string): Promise<string | null> {
      const picked = await open({
        title: label,
        directory: true,
        multiple: false,
      });
      return typeof picked === 'string' ? picked : null;
    },
    async pickFilePath(request: FilePickerRequest): Promise<string | null> {
      const picked = await open({
        title: request.label,
        filters: request.extensions?.length
          ? [{ name: request.label, extensions: [...request.extensions] }]
          : undefined,
        directory: false,
        multiple: false,
      });
      return typeof picked === 'string' ? picked : null;
    },
    readFile: (path: string) => readFile(path),
    async readFileStream(path: string): Promise<FileChunkSource> {
      const file = await openFile(path, { read: true });
      const size = (await file.stat()).size;
      return {
        name: fileName(path),
        size,
        async readChunk(offset, maxBytes) {
          await file.seek(offset, SeekMode.Start);
          const bytes = new Uint8Array(Math.min(maxBytes, size - offset));
          let read = 0;
          while (read < bytes.byteLength) {
            const count = await file.read(bytes.subarray(read));
            if (count === null) break;
            read += count;
          }
          return bytes.subarray(0, read);
        },
        close: () => file.close(),
      };
    },
    async pickFile(request: FilePickerRequest): Promise<PickedFile | null> {
      const picked = await open({
        title: request.label,
        filters: request.extensions?.length
          ? [{ name: request.label, extensions: [...request.extensions] }]
          : undefined,
        multiple: false,
      });
      if (!picked || Array.isArray(picked)) return null;
      return {
        name: fileName(picked),
        bytes: await readFile(picked),
      };
    },
    async pickFileStream(request: FilePickerRequest): Promise<FileChunkSource | null> {
      const picked = await open({
        title: request.label,
        filters: request.extensions?.length
          ? [{ name: request.label, extensions: [...request.extensions] }]
          : undefined,
        multiple: false,
      });
      if (!picked || Array.isArray(picked)) return null;
      const file = await openFile(picked, { read: true });
      const size = (await file.stat()).size;
      return {
        name: fileName(picked),
        size,
        async readChunk(offset, maxBytes) {
          await file.seek(offset, SeekMode.Start);
          const bytes = new Uint8Array(Math.min(maxBytes, size - offset));
          let read = 0;
          while (read < bytes.byteLength) {
            const count = await file.read(bytes.subarray(read));
            if (count === null) break;
            read += count;
          }
          return bytes.subarray(0, read);
        },
        close: () => file.close(),
      };
    },
    async notify(notificationRequest: DesktopNotification): Promise<void> {
      const granted =
        (await notification.isPermissionGranted()) ||
        (await notification.requestPermission()) === 'granted';
      if (granted) notification.sendNotification(notificationRequest);
    },
    async showMenu(entries: readonly MenuEntry[]): Promise<void> {
      const menu = await Menu.new({
        items: entries.map((entry) => ({
          id: entry.id,
          text: entry.label,
          action: entry.onSelect,
        })),
      });
      await menu.popup();
    },
    closeWindow: () => getCurrentWindow().close(),
    quitApplication: () => invoke('quit_app'),
    openExternal: (url: string) => invoke('open_external_url', { url }),
    revealInFileManager: (path: string) => invoke('reveal_in_file_manager', { path }),
    // Tauri's webview intercepts native OS drag-drop before it reaches the
    // DOM (dragDropEnabled defaults true, tauri.conf.json), so this is the
    // only source of a *real* dropped path -- an HTML5 `ondrop` handler in
    // the component only ever sees an empty `dataTransfer` here.
    onFileDrop: async (handler: (paths: readonly string[]) => void) =>
      getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === 'drop') handler(event.payload.paths);
      }),
    onCloseRequested: (handler: () => void) => getCurrentWindow().onCloseRequested(handler),
    agentHealthCheck: () => invoke<boolean>('agent_health_check'),
    agentServiceStatus: () => invoke<AgentServiceStatus>('agent_service_status'),
    manageAgentService: (action: AgentServiceAction) =>
      invoke<AgentServiceStatus>('manage_agent_service', { action }),
    checkForUpdates: () => invoke<UpdateCheckResult>('check_for_updates'),
    installUpdate: (releaseId: string) =>
      invoke<UpdateInstallResult>('install_coordinated_update', {
        request: { releaseId },
      }),
  });
}

function fileName(path: string): string {
  return path.split(/[\\/]/).at(-1) || 'selected-file';
}
