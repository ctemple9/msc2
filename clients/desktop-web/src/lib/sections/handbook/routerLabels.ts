/**
 * Presentation-only label/tone lookups for router-guide raw enum values the
 * agent serves (crates/msc-domain/src/router/{composer,troubleshooting}.rs).
 * No prose lives here -- only how a known raw value reads and, where the
 * value is a real computed system state (severity), which reserved status
 * color it gets. Confidence and category are content classifications, not
 * system states, so they stay plain text -- same call P12.16 made for the
 * Handbook's callout styles (docs/msc2/antiAIslop.md).
 */

export function confidenceLabel(raw: string): string {
  switch (raw) {
    case 'verified_recently':
      return 'Verified recently';
    case 'common_flow':
      return 'Common flow';
    case 'older_interface_may_vary':
      return 'May vary';
    case 'community_based':
      return 'Community-based';
    default:
      return raw;
  }
}

export function severityLabel(raw: string): string {
  switch (raw) {
    case 'high':
      return 'High';
    case 'medium':
      return 'Medium';
    case 'low':
      return 'Low';
    default:
      return raw;
  }
}

export function severityTone(raw: string): 'ok' | 'warn' | 'error' {
  switch (raw) {
    case 'high':
      return 'error';
    case 'medium':
      return 'warn';
    default:
      return 'ok';
  }
}
