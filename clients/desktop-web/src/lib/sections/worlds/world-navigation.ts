import { writable } from 'svelte/store';

export type OverviewWorldAction = 'change-thumbnail' | 'view-map';

export type WorldNavigationRequest = {
  id: number;
  hostId: string;
  serverId: string;
  slotId: string;
  action: OverviewWorldAction;
};

export const worldNavigationRequest = writable<WorldNavigationRequest | undefined>(undefined);

let nextRequestId = 0;

export function requestWorldNavigation(request: Omit<WorldNavigationRequest, 'id'>): void {
  worldNavigationRequest.set({ ...request, id: ++nextRequestId });
}

export function clearWorldNavigationRequest(id: number): void {
  worldNavigationRequest.update((request) => (request?.id === id ? undefined : request));
}
