import type {
  AgentAction,
  AgentServiceAction,
  AgentServiceStatus,
  DesktopNotification,
  FilePickerRequest,
  MenuEntry,
  PickedFile,
  PlatformAdapter,
} from './types';

/** Browser behavior is the fallback for every desktop adapter. */
export function createBrowserPlatform(): PlatformAdapter {
  return {
    kind: 'browser',
    pickFile: (_request: FilePickerRequest, browserFallback: () => Promise<PickedFile | null>) =>
      browserFallback(),
    notify: async (notification: DesktopNotification, browserFallback: () => Promise<void>) => {
      if (typeof Notification === 'undefined') {
        await browserFallback();
        return;
      }
      if (Notification.permission === 'granted') {
        new Notification(notification.title, { body: notification.body });
        return;
      }
      await browserFallback();
    },
    showMenu: async (_entries: readonly MenuEntry[], browserFallback: () => Promise<void>) => {
      await browserFallback();
    },
    closeWindow: async (browserFallback: () => Promise<void>) => {
      await browserFallback();
    },
    openExternal: async (url: string) => {
      if (typeof document === 'undefined') throw new Error('External links need a browser window.');
      const link = document.createElement('a');
      link.href = url;
      link.target = '_blank';
      link.rel = 'noreferrer noopener';
      link.click();
    },
    onCloseRequested: async (handler: () => void) => {
      if (typeof window === 'undefined') return () => undefined;
      const listener = () => handler();
      window.addEventListener('beforeunload', listener);
      return () => window.removeEventListener('beforeunload', listener);
    },
    credentialFor: async (_hostId: string) => null,
    requestAgentAction: async (_action: AgentAction, browserFallback: () => Promise<void>) => {
      await browserFallback();
    },
    agentServiceStatus: async (): Promise<AgentServiceStatus> => ({
      available: false,
      platform: 'browser',
      serviceName: 'com.ctemple.msc2.agent',
      state: 'unavailable',
      detail:
        'This browser cannot install a local background service. Install the headless package for this host, then return here to connect.',
    }),
    manageAgentService: async (_action: AgentServiceAction): Promise<AgentServiceStatus> => ({
      available: false,
      platform: 'browser',
      serviceName: 'com.ctemple.msc2.agent',
      state: 'unavailable',
      detail:
        'Local service controls need the installed desktop shell. The same agent remains manageable from this browser after headless installation.',
    }),
  };
}
