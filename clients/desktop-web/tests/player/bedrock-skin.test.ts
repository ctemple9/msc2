import { describe, expect, it } from 'vitest';
import {
  bedrockBodyFallbackUrl,
  dottedGamertag,
  floodgateUuidFromXuid,
  xboxGamertagCandidates,
} from '../../src/lib/player/bedrockSkin';

describe('Bedrock avatar lookup', () => {
  it('uses MSC 1’s dotted gamertag convention and fallback body endpoint', () => {
    expect(dottedGamertag('Cam Craft')).toBe('.Cam Craft');
    expect(dottedGamertag('.Cam Craft')).toBe('.Cam Craft');
    expect(bedrockBodyFallbackUrl('Cam Craft')).toBe(
      'https://api.mcheads.org/body/.Cam%20Craft/160',
    );
  });

  it('retries Xbox lookup with spaces when Geyser replaced them with underscores', () => {
    expect(xboxGamertagCandidates('Cam_Craft')).toEqual(['Cam_Craft', 'Cam Craft']);
    expect(xboxGamertagCandidates('Cam Craft')).toEqual(['Cam Craft']);
  });

  it('converts an Xbox XUID into the Floodgate UUID used by MC Heads', () => {
    expect(floodgateUuidFromXuid('2535443338451450')).toBe('00000000-0000-0000-0009-01f8e78959fa');
    expect(floodgateUuidFromXuid('not-a-number')).toBeUndefined();
  });
});
