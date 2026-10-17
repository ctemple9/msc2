import { writable } from 'svelte/store';
import type { ScreenApi } from './types';
import { errorMessage } from './types';

export const transferNotices = writable<readonly { hostId: string; message: string }[]>([]);
export function reportTransferFailure(api: ScreenApi | undefined, error: unknown): void {
  const hostId = api?.hostId?.();
  if (!hostId) return;
  const message = errorMessage(error);
  transferNotices.update((notices) =>
    [...notices.filter((n) => n.hostId !== hostId), { hostId, message }].slice(-16),
  );
}
export function dismissTransferFailure(hostId: string): void {
  transferNotices.update((notices) => notices.filter((n) => n.hostId !== hostId));
}
