export type ChecklistStepEntry = { number: number; title: string; detail: string };

/** Mirrors `HelpContentBlock` (crates/msc-agent/src/help.rs) — one ordered content
 *  block, additional to `HelpTopic.body`. Only handbook.* topics carry these. */
export type HelpContentBlock =
  | { type: 'body'; markdown: string }
  | { type: 'bulletList'; items: readonly string[] }
  | { type: 'callout'; style: 'tip' | 'warning' | 'pitfall' | 'note'; text: string }
  | { type: 'inApp'; items: readonly string[] }
  | { type: 'advanced'; markdown: string }
  | { type: 'checklist'; phase: string; steps: readonly ChecklistStepEntry[] }
  | { type: 'table'; headers: readonly string[]; rows: readonly (readonly string[])[] };

export type HelpTopic = {
  helpId: string;
  title: string;
  subtitle?: string;
  category: string;
  analogy?: string;
  body: string;
  relatedIds: readonly string[];
  sections?: readonly HelpContentBlock[];
};

export type HelpCatalog = {
  topics: readonly Pick<HelpTopic, 'helpId' | 'title' | 'category'>[];
};

export type ConceptGuide = {
  id: string;
  pages: readonly {
    order: number;
    helpId: string;
    eyebrow: string;
    title: string;
    body: string;
    diagram: string;
    assetStatus: string;
  }[];
};

export type RouterGuideCatalog = {
  guides: readonly {
    id: string;
    family: string;
    category: string;
    displayName: string;
    steps: readonly string[];
  }[];
  troubleshooting: readonly { id: string; title: string; summary: string }[];
};

export type OnboardingGuide = {
  id: string;
  reopen: { label: string; persistenceKey: string };
  skip: { label: string; effect: string };
  steps: readonly OnboardingStep[];
};

export type OnboardingStep = {
  order: number;
  id: string;
  title: string;
  body: string;
  actionLabel?: string;
  anchor: string | null;
  requiresUserAction: boolean;
  hideCard?: boolean;
  when?: string;
};
