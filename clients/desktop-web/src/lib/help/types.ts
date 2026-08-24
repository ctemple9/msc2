export type HelpTopic = {
  id: string;
  kind: 'handbook' | 'concept' | 'contextual' | string;
  title: string;
  category?: string;
  analogy?: string;
  markdown: string;
  relatedIds: readonly string[];
};

export type HelpCatalog = {
  topics: readonly Pick<HelpTopic, 'id' | 'kind' | 'title' | 'category'>[];
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
  guides: readonly { id: string; displayName: string; steps: readonly string[] }[];
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
