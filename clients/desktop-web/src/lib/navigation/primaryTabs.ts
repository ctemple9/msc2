// The seven MSC 2 detail tabs, in their fixed order (docs/msc2/renderings/shell.html,
// MSC 1 DetailsView/MSCTabBar). MSC 1 has an eighth tab, Packs (DetailsPacksTabView) --
// deliberately dropped for MSC 2 (rolling-plan.md P12.5, owner decision 2026-08-26):
// Cameron doesn't use it, and it's the one named exception to Phase 12's
// every-MSC-1-screen gate (msc2-port-plan.md). This is deliberately not the
// extensible section registry in registry.ts/route.ts — Bedrock/profile
// extensibility is a separate reserved route family there. A tab is only
// "available" once its own Phase 12 step registers a matching section id in
// App.svelte's `sections`.
export type PrimaryTab = {
  readonly id: string;
  readonly label: string;
};

export const PRIMARY_TABS: readonly PrimaryTab[] = [
  { id: 'home', label: 'Overview' },
  { id: 'players-online', label: 'Players' },
  { id: 'worlds', label: 'Worlds' },
  { id: 'performance', label: 'Performance' },
  { id: 'components', label: 'Components' },
  { id: 'settings', label: 'Settings' },
  { id: 'files', label: 'Files' },
];
