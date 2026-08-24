import type {
  AgentAction,
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
  };
}
