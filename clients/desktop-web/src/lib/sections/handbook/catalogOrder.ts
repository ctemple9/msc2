/**
 * Presentation-only ordering for content the agent already serves unordered
 * (BTreeMap iteration for the help catalog; a flat array for router guides).
 * No prose lives here -- only the reading order and category labels MSC 1's
 * ServerHandbookView.swift / RouterPortForwardGuidePicker.swift use, so the
 * grouped views read in the same sequence the oracle established.
 */

export const HANDBOOK_CATEGORY_ORDER: readonly { slug: string; label: string }[] = [
  { slug: 'concepts', label: 'Concepts' },
  { slug: 'java-servers', label: 'Java Servers' },
  { slug: 'modded-java', label: 'Modded Servers' },
  { slug: 'bedrock-servers', label: 'Bedrock Servers' },
  { slug: 'connection-access', label: 'Connection & Access' },
  { slug: 'server-management', label: 'Server Management' },
  { slug: 'getting-started', label: 'Getting Started' },
];

/** Reading order within (and across) categories -- mirrors `HandbookTopic.allCases`. */
export const HANDBOOK_TOPIC_ORDER: readonly string[] = [
  'handbook.overview',
  'handbook.networking-basics',
  'handbook.ram-performance',
  'handbook.standard-vs-modded',
  'handbook.paper',
  'handbook.vanilla',
  'handbook.purpur',
  'handbook.jars-java',
  'handbook.eula-online-mode',
  'handbook.plugins-crossplay',
  'handbook.fabric',
  'handbook.neoforge',
  'handbook.forge',
  'handbook.mods-browser',
  'handbook.client-requirements',
  'handbook.bedrock',
  'handbook.how-bedrock-runs',
  'handbook.port-forwarding-duckdns',
  'handbook.playit',
  'handbook.tailscale',
  'handbook.xbox-broadcast',
  'handbook.remote-access',
  'handbook.worlds-backups',
  'handbook.world-conversion',
  'handbook.server-transfer',
  'handbook.server-files',
  'handbook.watchdog',
  'handbook.player-management',
  'handbook.first-server',
  'handbook.first-modded-server',
  'handbook.first-bedrock-server',
];

export function topicOrderIndex(helpId: string): number {
  const index = HANDBOOK_TOPIC_ORDER.indexOf(helpId);
  return index === -1 ? HANDBOOK_TOPIC_ORDER.length : index;
}

export const ROUTER_CATEGORY_ORDER: readonly { slug: string; label: string }[] = [
  { slug: 'isp_gateway', label: 'Provider Gateway' },
  { slug: 'retail_router', label: 'Router' },
  { slug: 'mesh_system', label: 'Mesh System' },
  { slug: 'generic_fallback', label: 'Generic Fallback' },
  { slug: 'advanced_networking', label: 'Advanced Networking' },
];
