import { describe, expect, it } from 'vitest';
import {
  bedrockBodyFallbackUrl,
  dottedGamertag,
  floodgateUuidFromXuid,
  xboxGamertagCandidates,
} from '../../src/lib/player/bedrockSkin';

describe('Bedrock avatar lookup', () => {
  it('uses MSC 1’s dotted gamertag convention and fallback body endpoint', () => {
    expect(dottedGamertag('Example Craft')).toBe('.Example Craft');
    expect(dottedGamertag('.Example Craft')).toBe('.Example Craft');
    expect(bedrockBodyFallbackUrl('Example Craft')).toBe(
      'https://api.mcheads.org/body/.Example%20Craft/160',
    );
  });

  it('retries Xbox lookup with spaces when Geyser replaced them with underscores', () => {
    expect(xboxGamertagCandidates('Example_Craft')).toEqual(['Example_Craft', 'Example Craft']);
    expect(xboxGamertagCandidates('Example Craft')).toEqual(['Example Craft']);
  });

  it('converts an Xbox XUID into the Floodgate UUID used by MC Heads', () => {
    expect(floodgateUuidFromXuid('123456789')).toBe('00000000-0000-0000-0000-0000075bcd15');
    expect(floodgateUuidFromXuid('not-a-number')).toBeUndefined();
  });
});
