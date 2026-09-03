import { describe, expect, it } from 'vitest';
import connectionSource from '../../src/lib/sections/home/ConnectionCard.svelte?raw';
import homeSource from '../../src/lib/sections/home/HomeSection.svelte?raw';
import controlSidebarSource from '../../src/lib/components/shell/ControlSidebar.svelte?raw';
import howToConnectSource from '../../src/lib/components/shell/sidebar/HowToConnectSection.svelte?raw';

describe('overview connection information', () => {
  it('always shows connection values without a visibility control', () => {
    expect(connectionSource).not.toContain('showAddresses');
    expect(connectionSource).not.toContain('mask(');
    expect(connectionSource).not.toContain('class="eye"');
    expect(connectionSource).not.toContain('Addresses hidden');
  });

  it('uses the same copy-address action label for Java and Bedrock', () => {
    expect(connectionSource).not.toContain('Copy port');
    expect(connectionSource).toContain(
      "copiedLabel === (isBedrockServer ? 'Bedrock' : 'Java') ? 'Copied' : 'Copy address'",
    );
  });

  it('threads the agent-reported host address into Overview and the sidebar', () => {
    expect(connectionSource).toContain('export let hostAddress: string | undefined = undefined;');
    expect(connectionSource).toContain('export let bedrockPort: number | undefined = undefined;');
    expect(connectionSource).toContain(
      "export let playit: Schema['PlayitStatusResponseDTO'] | undefined = undefined;",
    );
    expect(connectionSource).toContain(
      '$: primaryPublicEndpoint = isBedrockServer ? publicBedrockEndpoint : publicJavaEndpoint;',
    );
    expect(connectionSource).toContain(
      '$: primaryIp = showPublic ? (primaryPublicEndpoint?.host ?? null) : (hostAddress ?? null);',
    );
    expect(connectionSource).toMatch(
      /publicJavaValue\s*=\s*playit\s*===\s*undefined[\s\S]*playitSelected\s*\?\s*playitJavaAddress/,
    );
    expect(connectionSource).toContain('playitSelected ? undefined : gamePort');
    expect(homeSource).toContain('hostAddress={activeServer?.hostAddress}');
    expect(homeSource).toContain('bedrockPort={activeServer?.bedrockPort}');
    expect(connectionSource).toContain('configuredBedrockPort = bedrockPort ?? geyser?.port');
    expect(connectionSource).toContain(
      'hasGeyser = !isBedrockServer && geyser?.isGeyserInstalled === true',
    );
    expect(howToConnectSource).toContain(
      'hasBedrockEndpoint = isBedrockServer || geyser?.isGeyserInstalled === true',
    );
    expect(howToConnectSource).toContain('if (isBedrockServer)');
    expect(howToConnectSource).toContain("key: 'java-lan'");
    expect(howToConnectSource).toContain("key: 'bedrock-lan'");
    expect(connectionSource).not.toContain('configuredBedrockPort !== undefined');
    expect(howToConnectSource).not.toContain(
      'hasBedrockEndpoint = isBedrockServer || bedrockEndpointPort',
    );
    expect(connectionSource).toContain(
      'playitSelected ? undefined : isBedrockServer ? gamePort : configuredBedrockPort',
    );
    expect(connectionSource).toContain(': configuredBedrockPort;');
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
