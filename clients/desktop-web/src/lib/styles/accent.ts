export type AccentChoice = {
  id: string;
  label: string;
  color: string;
  strong: string;
  focus: string;
};

export const ACCENT_PRESETS: readonly AccentChoice[] = [
  {
    id: 'green',
    label: 'Green',
    color: '#22c85a',
    strong: '#6ee7a0',
    focus: 'rgba(34, 200, 90, 0.32)',
  },
  {
    id: 'blue',
    label: 'Blue',
    color: '#3b82f6',
    strong: '#93c5fd',
    focus: 'rgba(59, 130, 246, 0.32)',
  },
  {
    id: 'purple',
    label: 'Purple',
    color: '#8b5cf6',
    strong: '#c4b5fd',
    focus: 'rgba(139, 92, 246, 0.32)',
  },
  {
    id: 'orange',
    label: 'Orange',
    color: '#f97316',
    strong: '#fdba74',
    focus: 'rgba(249, 115, 22, 0.32)',
  },
  {
    id: 'red',
    label: 'Red',
    color: '#ef4444',
    strong: '#fca5a5',
    focus: 'rgba(239, 68, 68, 0.32)',
  },
  {
    id: 'teal',
    label: 'Teal',
    color: '#14b8a6',
    strong: '#5eead4',
    focus: 'rgba(20, 184, 166, 0.32)',
  },
];

const STORAGE_KEY = 'msc.accent';
const SERVER_TYPES_STORAGE_KEY = 'msc.server-types';
const HEX_COLOR = /^#[0-9a-f]{6}$/i;

function customChoice(color: string): AccentChoice {
  return {
    id: color.toLowerCase(),
    label: 'Custom',
    color,
    strong: color,
    focus: `${color}55`,
  };
}

export function applyAccent(choice: AccentChoice | string): void {
  if (typeof document === 'undefined') return;
  const accent =
    typeof choice === 'string'
      ? (ACCENT_PRESETS.find((preset) => preset.id === choice) ??
        (HEX_COLOR.test(choice) ? customChoice(choice) : ACCENT_PRESETS[0]))
      : choice;
  document.documentElement.style.setProperty('--msc-accent', accent.color);
  document.documentElement.style.setProperty('--msc-accent-strong', accent.strong);
  document.documentElement.style.setProperty('--msc-focus', `0 0 0 0.2rem ${accent.focus}`);
}

export function restoreAccent(): void {
  if (typeof localStorage === 'undefined') return;
  applyAccent(localStorage.getItem(STORAGE_KEY) ?? 'green');
}

export function saveAccent(choice: AccentChoice | string): void {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(STORAGE_KEY, typeof choice === 'string' ? choice : choice.id);
  }
  applyAccent(choice);
}

export function storedAccent(): string {
  if (typeof localStorage === 'undefined') return 'green';
  const value = localStorage.getItem(STORAGE_KEY);
  return value && (ACCENT_PRESETS.some((preset) => preset.id === value) || HEX_COLOR.test(value))
    ? value
    : 'green';
}

export function resetSetupPreferences(): void {
  if (typeof localStorage !== 'undefined') {
    localStorage.removeItem(STORAGE_KEY);
    localStorage.removeItem(SERVER_TYPES_STORAGE_KEY);
  }
  applyAccent('green');
}
