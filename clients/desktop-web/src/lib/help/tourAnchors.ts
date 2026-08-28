import { writable } from 'svelte/store';

/**
 * Anchor ids this MSC 2 build actually wires up to real UI, ported 1:1 from
 * MSC 1's `OnboardingAnchorID` string values. The agent's `/v1/guides/onboarding`
 * data is oracle-faithful and untouched (content/order/branching are agent
 * data per that guide's own `presentationBoundary`). The Add Server wizard
 * now exposes its real multi-page flow; the Packs tab was permanently dropped
 * (P12.5). A guide step
 * whose anchor isn't in this set points at UI that doesn't exist in this
 * client build -- `applicableTourSteps` (./onboarding.ts) filters those out
 * rather than showing a coach mark with nothing to highlight.
 */
export const KNOWN_TOUR_ANCHOR_IDS: ReadonlySet<string> = new Set([
  'ob_manage_servers',
  'ob_create_server',
  'ob_wizard_path_picker',
  'ob_wizard_fresh_card',
  'ob_wizard_continue',
  'ob_server_name',
  'ob_server_type',
  'ob_server_category',
  'ob_server_flavor',
  'ob_server_source',
  'ob_server_crossplay',
  'ob_server_xbox_broadcast',
  'ob_server_settings',
  'ob_server_connectivity',
  'ob_server_connectivity_ports',
  'ob_confirm_page',
  'ob_wizard_body',
  'ob_wizard_sheet',
  'ob_world_source',
  'ob_world_creation',
  'ob_create_save',
  'ob_manage_done',
  'ob_accept_eula',
  'ob_start_button',
  'ob_console_panel',
  'ob_console_divider_handle',
  'ob_details_overview_tab',
  'ob_details_players_tab',
  'ob_details_worlds_tab',
  'ob_details_performance_tab',
  'ob_details_components_tab',
  'ob_details_settings_tab',
  'ob_details_files_tab',
]);

export type AnchorRect = { top: number; left: number; width: number; height: number };

/** Live viewport rects of every mounted `use:onboardingAnchor` element, keyed by anchor id. */
export const anchorFrames = writable<Record<string, AnchorRect>>({});

function measure(node: HTMLElement): AnchorRect {
  const rect = node.getBoundingClientRect();
  return { top: rect.top, left: rect.left, width: rect.width, height: rect.height };
}

/**
 * `use:onboardingAnchor={'ob_start_button'}` on a real element reports its
 * live rect under that id so `TourOverlay.svelte` can spotlight it. Mirrors
 * MSC 1's `contextualHelpAnchor`/`ContextualHelpAnchorModifier`: the element
 * reports itself; it never queries the tour to know if it should.
 */
export function onboardingAnchor(node: HTMLElement, anchorId: string | undefined) {
  let id = anchorId;

  function report(): void {
    if (!id) return;
    const rect = measure(node);
    if (rect.width === 0 && rect.height === 0) return;
    anchorFrames.update((frames) => ({ ...frames, [id as string]: rect }));
  }

  function clear(clearId: string | undefined): void {
    if (!clearId) return;
    anchorFrames.update((frames) => {
      if (!(clearId in frames)) return frames;
      const next = { ...frames };
      delete next[clearId];
      return next;
    });
  }

  report();
  const observer = new ResizeObserver(report);
  observer.observe(node);
  window.addEventListener('resize', report);
  window.addEventListener('scroll', report, true);

  return {
    update(nextId: string | undefined): void {
      clear(id);
      id = nextId;
      report();
    },
    destroy(): void {
      observer.disconnect();
      window.removeEventListener('resize', report);
      window.removeEventListener('scroll', report, true);
      clear(id);
    },
  };
}
