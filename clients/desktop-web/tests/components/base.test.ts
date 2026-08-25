import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import cardSource from '../../src/lib/components/base/Card.svelte?raw';
import buttonSource from '../../src/lib/components/base/Button.svelte?raw';
import segmentedControlSource from '../../src/lib/components/base/SegmentedControl.svelte?raw';
import toggleSource from '../../src/lib/components/base/Toggle.svelte?raw';
import fieldSource from '../../src/lib/components/base/Field.svelte?raw';
import numberFieldSource from '../../src/lib/components/base/NumberField.svelte?raw';
import selectSource from '../../src/lib/components/base/Select.svelte?raw';
import badgeSource from '../../src/lib/components/base/Badge.svelte?raw';
import listRowSource from '../../src/lib/components/base/ListRow.svelte?raw';
import emptyStateSource from '../../src/lib/components/base/EmptyState.svelte?raw';
import statusDotSource from '../../src/lib/components/base/StatusDot.svelte?raw';
import sheetSource from '../../src/lib/components/base/Sheet.svelte?raw';

const tokensCss = readFileSync(
  fileURLToPath(new URL('../../src/lib/styles/tokens.css', import.meta.url)),
  'utf8',
);

describe('S0 tokens.css — locked values', () => {
  it('defines the four surface tiers', () => {
    expect(tokensCss).toContain('--msc2-tier-atmosphere: #0d0d0f');
    expect(tokensCss).toContain('--msc2-tier-chrome: #141417');
    expect(tokensCss).toContain('--msc2-tier-content: #1c1c21');
    expect(tokensCss).toContain('--msc2-tier-terminal: #0a0a0c');
  });

  it('defines the status ramp', () => {
    expect(tokensCss).toContain('--msc2-status-ok: #4dc778');
    expect(tokensCss).toContain('--msc2-status-warn: #ff9140');
    expect(tokensCss).toContain('--msc2-status-error: #e24b4a');
    expect(tokensCss).toContain('--msc2-status-bedrock: #59a1ff');
  });

  it('defines the white-opacity text steps', () => {
    expect(tokensCss).toContain('--msc2-text-primary: rgba(255, 255, 255, 0.95)');
    expect(tokensCss).toContain('--msc2-text-secondary: rgba(255, 255, 255, 0.55)');
    expect(tokensCss).toContain('--msc2-text-tertiary: rgba(255, 255, 255, 0.4)');
  });

  it('defines the 4pt spacing scale and the radius scale', () => {
    for (const px of ['2px', '4px', '8px', '12px', '16px', '20px', '24px', '32px']) {
      expect(tokensCss).toContain(px);
    }
    expect(tokensCss).toContain('--msc2-radius-1: 6px');
    expect(tokensCss).toContain('--msc2-radius-2: 10px');
    expect(tokensCss).toContain('--msc2-radius-3: 14px');
    expect(tokensCss).toContain('--msc2-radius-4: 18px');
  });

  it('defines the 7-role type scale at the locked weights', () => {
    expect(tokensCss).toContain('.msc2-type-page');
    expect(tokensCss).toContain('font-size: 21px');
    expect(tokensCss).toContain('.msc2-type-section');
    expect(tokensCss).toContain('.msc2-type-card');
    expect(tokensCss).toContain('.msc2-type-body');
    expect(tokensCss).toContain('.msc2-type-meta');
    expect(tokensCss).toContain('.msc2-type-overline');
    expect(tokensCss).toContain('.msc2-type-mono');
    // nothing heavier than 600 anywhere in the scale
    expect(tokensCss.match(/font-weight:\s*(\d+)/g)).not.toBeNull();
    for (const match of tokensCss.matchAll(/font-weight:\s*(\d+)/g)) {
      expect(Number(match[1])).toBeLessThanOrEqual(600);
    }
  });

  it('keeps the Phase 11 --msc-* tokens untouched for unconverted screens', () => {
    expect(tokensCss).toContain('--msc-bg: #101820');
    expect(tokensCss).toContain('--msc-accent: #8fe3cf');
  });
});

describe('Card — the one card depth', () => {
  it('is flat, borderless, radius 12, one whisper shadow', () => {
    expect(cardSource).toContain('var(--msc2-tier-content)');
    expect(cardSource).toContain('border-radius: 12px');
    expect(cardSource).toContain('var(--msc2-shadow-card)');
    expect(cardSource).not.toContain('border:');
    expect(cardSource).not.toContain('linear-gradient');
  });
});

describe('Button — 2 shapes, colored only by meaning', () => {
  it('exposes the locked variant set', () => {
    for (const variant of ['primary', 'start', 'stop', 'secondary', 'destructive', 'ghost-icon']) {
      expect(buttonSource).toContain(`'${variant}'`);
    }
  });

  it('spends color only on start/stop, never a gradient or shadow', () => {
    expect(buttonSource).toContain('var(--msc2-status-ok)');
    expect(buttonSource).toContain('var(--msc2-status-error)');
    expect(buttonSource).not.toContain('linear-gradient');
    expect(buttonSource).not.toContain('box-shadow');
  });

  it('presses with a scale, not a translate', () => {
    expect(buttonSource).toContain('scale(0.98)');
  });
});

describe('SegmentedControl — neutral selection, not accent-tinted', () => {
  it('selects with the neutral elevated fill', () => {
    expect(segmentedControlSource).toContain('var(--msc2-neutral-elevated)');
    expect(segmentedControlSource).not.toContain('var(--msc2-status-ok)');
  });
});

describe('Toggle — green means on', () => {
  it('uses the ok status color for the on state and a neutral track when off', () => {
    expect(toggleSource).toContain('var(--msc2-status-ok)');
    expect(toggleSource).toContain('var(--msc2-neutral-muted)');
  });
});

describe('Field / NumberField / Select — dark inset, neutral focus ring', () => {
  it('focuses with a brighter neutral border, never a glow', () => {
    for (const source of [fieldSource, numberFieldSource, selectSource]) {
      expect(source).toContain('var(--msc2-hairline-field-focus)');
      expect(source).not.toContain('box-shadow');
    }
  });
});

describe('Badge — category neutral vs status tinted', () => {
  it('keeps category neutral and status tied to the ramp', () => {
    expect(badgeSource).toContain('var(--msc2-neutral-elevated)');
    expect(badgeSource).toContain('var(--msc2-status-ok)');
    expect(badgeSource).toContain('var(--msc2-status-ok-tint)');
  });
});

describe('ListRow — divided, not carded', () => {
  it('insets its divider past the icon and never renders its own card', () => {
    expect(listRowSource).toContain('margin-left: 41px');
    expect(listRowSource).not.toContain('border-radius: 12px');
    expect(listRowSource).not.toContain('box-shadow');
  });
});

describe('EmptyState — centered, muted, no decoration', () => {
  it('has no gradient or accent color', () => {
    expect(emptyStateSource).not.toContain('linear-gradient');
    expect(emptyStateSource).not.toContain('var(--msc2-status-');
  });
});

describe('StatusDot — always dot + labeled text', () => {
  it('requires a label prop', () => {
    expect(statusDotSource).toContain('export let label: string;');
  });
});

describe('Sheet — three fixed widths only', () => {
  it('locks to 480 / 640 / 820', () => {
    expect(sheetSource).toContain("sm: '480px'");
    expect(sheetSource).toContain("md: '640px'");
    expect(sheetSource).toContain("lg: '820px'");
  });
});
