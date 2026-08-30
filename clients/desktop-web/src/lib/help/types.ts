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

/** Mirrors `RouterGuideStepDTO` (crates/msc-agent/src/routes/help.rs). */
export type RouterGuideStep = {
  id: string;
  kind: string;
  title: string;
  body: string;
  referencedTokens: readonly string[];
  alternateTerms: readonly string[];
};

export type RouterGuideNote = {
  id: string;
  title?: string | null;
  body: string;
};

export type RouterGuideSharedSections = {
  includeSharedIntro: boolean;
  includeSharedPrerequisites: boolean;
  includeSharedValueSummary: boolean;
  includeSharedTroubleshootingFooter: boolean;
};

export type RouterGuideReviewMetadata = {
  sourceConfidence: string;
  lastReviewed?: string | null;
  reviewNotes?: string | null;
};

/** Mirrors `RouterGuideDTO` -- the complete, unresolved guide record (no
 *  runtime token substitution yet). See `ResolvedRouterGuide` for the
 *  composed-and-resolved shape `GET /v1/guides/router/{guideId}` returns. */
export type RouterGuide = {
  id: string;
  displayName: string;
  category: string;
  family: string;
  searchKeywords: readonly string[];
  adminAddresses: readonly string[];
  adminSurface: string;
  menuPath: readonly string[];
  alternateMenuNames: readonly string[];
  steps: readonly RouterGuideStep[];
  notes: readonly RouterGuideNote[];
  troubleshooting: readonly string[];
  sharedSections: RouterGuideSharedSections;
  review: RouterGuideReviewMetadata;
  providerDisplayName?: string | null;
  deviceDisplayName?: string | null;
};

export type RouterTroubleshootingTopic = {
  id: string;
  title: string;
  summary: string;
  suggestedNextActions: readonly string[];
};

/** Client-owned reference copy for the troubleshooting symptom checklist --
 *  served alongside the catalog because rule evaluation, not this text, is
 *  the executable behavior (crates/msc-domain/src/router/troubleshooting.rs). */
export type RouterSymptom = {
  id: string;
  title: string;
  description: string;
};

export type RouterGuideCatalog = {
  guides: readonly RouterGuide[];
  troubleshooting: readonly RouterTroubleshootingTopic[];
  symptoms: readonly RouterSymptom[];
};

export type RouterGuideSummary = {
  id: string;
  family: string;
  category: string;
  displayName: string;
  providerDisplayName?: string | null;
  deviceDisplayName?: string | null;
};

export type RouterGuideMatchCandidate = {
  guide: RouterGuideSummary;
  score: number;
  reasons: readonly string[];
};

/** Mirrors `RouterFallbackResolutionDTO` -- the fallback decision tree's
 *  output (crates/msc-domain/src/router/fallback_tree.rs), shown as a
 *  banner/suggestion in both the picker's search results and a
 *  troubleshooting analysis. */
export type RouterFallbackResolution = {
  kind: string;
  availability: string;
  matchedGuideId?: string | null;
  fallbackGuideId?: string | null;
  desiredFamily?: string | null;
  inferredFamilies: readonly string[];
  explanationBullets: readonly string[];
  recommendedNextNodeId?: string | null;
  suggestedSearchTerms: readonly string[];
  matchedQuery?: string | null;
};

/** Mirrors `RouterGuideSearchDTO` (`GET /v1/guides/router/search`). */
export type RouterGuideSearchResult = {
  query: string;
  normalizedQuery: string;
  normalizedTokens: readonly string[];
  inferredFamilies: readonly string[];
  candidates: readonly RouterGuideMatchCandidate[];
  suggestedFallbackGuide?: RouterGuideSummary | null;
  isAmbiguous: boolean;
  matchedDirectGuide: boolean;
  fallbackResolution: RouterFallbackResolution;
};

export type RouterRuntimeSummary = {
  selectedServerId?: string | null;
  selectedServerName?: string | null;
  detectedLocalIpAddress?: string | null;
  detectedGatewayIpAddress?: string | null;
  javaPort?: number | null;
  bedrockPort?: number | null;
  recommendedProtocol?: string | null;
  bedrockEnabled?: boolean | null;
};

/** Mirrors the composer's tagged `SectionItem` (crates/msc-domain/src/
 *  router/composer.rs), after runtime token resolution. */
export type RouterResolvedItem =
  | { type: 'paragraph'; title?: string | null; body: string }
  | { type: 'bulletList'; title?: string | null; bullets: readonly string[] }
  | {
      type: 'menuPath';
      title?: string | null;
      path: readonly string[];
      alternateMenuNames: readonly string[];
    }
  | {
      type: 'step';
      id: string;
      kind: string;
      title: string;
      body: string;
      alternateTerms: readonly string[];
    }
  | { type: 'note'; id: string; title?: string | null; body: string }
  | {
      type: 'troubleshootingTopic';
      id: string;
      title: string;
      summary: string;
      suggestedNextActions: readonly string[];
    };

export type RouterResolvedSection = {
  id: string;
  kind: string;
  title: string;
  origin: string;
  items: readonly RouterResolvedItem[];
};

export type RouterUnresolvedToken = {
  sectionId: string;
  token: string;
};

/** Mirrors `ResolvedRouterGuideDTO` (`GET /v1/guides/router/{guideId}`) --
 *  the composer's section list after runtime_resolver.rs has substituted
 *  `{{token}}` placeholders against the selected server's live context. */
export type ResolvedRouterGuide = {
  guide: RouterGuide;
  runtime: RouterRuntimeSummary;
  sections: readonly RouterResolvedSection[];
  unresolvedTokens: readonly RouterUnresolvedToken[];
};

export type RouterAnalysisCause = {
  id: string;
  confidence: string;
  score: number;
  severity: string;
  matchedSymptoms: readonly string[];
  topic: RouterTroubleshootingTopic;
};

/** Mirrors `RouterTroubleshootingAnalyzeResponseDTO`
 *  (`POST /v1/guides/router/troubleshooting/analyze`). */
export type RouterTroubleshootingAnalysis = {
  symptoms: readonly string[];
  likelyCauses: readonly RouterAnalysisCause[];
  recommendedActions: readonly string[];
  escalationBullets: readonly string[];
  fallbackResolution?: RouterFallbackResolution | null;
  summary: string;
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
