import { describe, expect, it } from 'vitest';
import connectionSource from '../../src/lib/sections/home/ConnectionCard.svelte?raw';
import homeSource from '../../src/lib/sections/home/HomeSection.svelte?raw';
import controlSidebarSource from '../../src/lib/components/shell/ControlSidebar.svelte?raw';
import howToConnectSource from '../../src/lib/components/shell/sidebar/HowToConnectSection.svelte?raw';

describe('overview connection information', () => {
  it('threads the agent-reported host address into Overview and the sidebar', () => {
    expect(connectionSource).toContain('export let hostAddress: string | undefined = undefined;');
    expect(connectionSource).toContain(
      "export let playit: Schema['PlayitStatusResponseDTO'] | undefined = undefined;",
    );
    expect(connectionSource).toContain(
      '$: javaIp = showPublic ? (publicJavaEndpoint?.host ?? null) : (hostAddress ?? null);',
    );
    expect(connectionSource).toMatch(
      /publicJavaValue\s*=\s*playit\s*===\s*undefined[\s\S]*playitSelected\s*\?\s*playitJavaAddress/,
    );
    expect(connectionSource).toContain('playitSelected ? undefined : gamePort');
    expect(homeSource).toContain('hostAddress={activeServer?.hostAddress}');
    expect(homeSource).toContain('{playit}');
    expect(controlSidebarSource).toContain('hostAddress={activeServer?.hostAddress}');
    expect(controlSidebarSource).toContain('gamePort={activeServer?.gamePort}');
    expect(howToConnectSource).toContain('value: localJavaAddress');
    expect(howToConnectSource).toContain('value: localBedrockAddress');
    expect(howToConnectSource).toMatch(
      /playitSelected\s*\?\s*playitJavaAddress\s*:\s*connectivity\?\.joinAddress/,
    );
    expect(howToConnectSource).toContain('refreshTimer = setInterval(() => void load(), 8000);');
  });
});
