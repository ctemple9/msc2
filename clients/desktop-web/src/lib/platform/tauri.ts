import type {
  AgentAction,
  AgentServiceAction,
  AgentServiceStatus,
  DesktopNotification,
  FilePickerRequest,
  MenuEntry,
  PickedFile,
  PlatformAdapter,
  TauriPlatformDependencies,
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
    // Cancelling a native picker is a completed user choice, not a reason to
    // open a second browser picker. Browser fallback happens at platform load.
    pickFile: (request, _browserFallback) => dependencies.pickFile(request),
    notify: async (notification, browserFallback) => {
      try {
        await dependencies.notify(notification);
      } catch {
        await browserFallback();
      }
    },
    showMenu: async (entries, browserFallback) => {
      try {
        await dependencies.showMenu(entries);
      } catch {
        await browserFallback();
      }
    },
    closeWindow: async (browserFallback) => {
      try {
        await dependencies.closeWindow();
      } catch {
        await browserFallback();
      }
    },
    openExternal: (url: string) => dependencies.openExternal(url),
    openLocalAgentBrowser: () => dependencies.openLocalAgentBrowser(),
    revealInFileManager: async (path: string, browserFallback: () => Promise<void>) => {
      try {
        await dependencies.revealInFileManager(path);
      } catch {
        await browserFallback();
      }
    },
    onCloseRequested: dependencies.onCloseRequested,
    // P11.23 supplies per-host pairing and secret-store behavior. This seam
    // deliberately cannot fabricate a local token before that contract exists.
    credentialFor: async (_hostId: string) => null,
    requestAgentAction: async (_action: AgentAction, browserFallback: () => Promise<void>) => {
      await browserFallback();
    },
    agentHealthCheck: dependencies.agentHealthCheck,
    agentServiceStatus: dependencies.agentServiceStatus,
    manageAgentService: dependencies.manageAgentService,
  };
}

export async function loadTauriPlatform(): Promise<PlatformAdapter> {
  const [{ open }, { readFile }, notification, { Menu }, { getCurrentWindow }, { invoke }] =
    await Promise.all([
      import('@tauri-apps/plugin-dialog'),
      import('@tauri-apps/plugin-fs'),
      import('@tauri-apps/plugin-notification'),
      import('@tauri-apps/api/menu'),
      import('@tauri-apps/api/window'),
      import('@tauri-apps/api/core'),
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
    openExternal: (url: string) => invoke('open_external_url', { url }),
    openLocalAgentBrowser: () => invoke('open_local_agent_browser'),
    revealInFileManager: (path: string) => invoke('reveal_in_file_manager', { path }),
    onCloseRequested: (handler: () => void) => getCurrentWindow().onCloseRequested(handler),
    agentHealthCheck: () => invoke<boolean>('agent_health_check'),
    agentServiceStatus: () => invoke<AgentServiceStatus>('agent_service_status'),
    manageAgentService: (action: AgentServiceAction) =>
      invoke<AgentServiceStatus>('manage_agent_service', { action }),
  });
}

function fileName(path: string): string {
  return path.split(/[\\/]/).at(-1) || 'selected-file';
}
