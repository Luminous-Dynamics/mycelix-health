/**
 * Accessibility Types for Mycelix Health SDK
 *
 * Defines types and interfaces for accessibility features including
 * ARIA labels, screen reader support, and internationalization.
 */

/**
 * Accessibility options that can be passed to SDK methods
 */
export interface AccessibilityOptions {
  /** Preferred language code (e.g., 'en', 'es', 'zh') */
  language?: string;

  /** Enable high contrast mode descriptions */
  highContrast?: boolean;

  /** Include extended descriptions for complex data */
  extendedDescriptions?: boolean;

  /** Reading level for medical terminology explanations */
  readingLevel?: ReadingLevel;

  /** Whether to include pronunciation guides for medical terms */
  includePronunciation?: boolean;
}

/**
 * Reading levels for medical terminology explanations
 */
export enum ReadingLevel {
  /** Elementary school level - very simple explanations */
  Elementary = 'elementary',
  /** Middle school level - basic explanations */
  Intermediate = 'intermediate',
  /** High school level - standard explanations */
  Standard = 'standard',
  /** College level - detailed explanations */
  Advanced = 'advanced',
  /** Medical professional level - technical terminology */
  Professional = 'professional',
}

/**
 * Accessible error information
 */
export interface AccessibleError {
  /** Machine-readable error code */
  code: string;
  /** Human-readable error message */
  message: string;
  /** Screen reader friendly description */
  ariaLabel: string;
  /** Suggested actions the user can take */
  suggestedActions: string[];
  /** Related help topic identifier */
  helpTopicId?: string;
}

/**
 * Accessible data display information
 */
export interface AccessibleDisplay<T> {
  /** The underlying data */
  data: T;
  /** Screen reader label for the data */
  ariaLabel: string;
  /** Brief summary suitable for announcements */
  summary: string;
  /** Detailed description for extended reading */
  description: string;
  /** Key-value pairs for structured screen reader output */
  attributes: Map<string, string>;
}

/**
 * Accessible status information for health data
 */
export interface AccessibleHealthStatus {
  /** Overall status category */
  status: HealthStatusCategory;
  /** Screen reader friendly status description */
  statusDescription: string;
  /** ARIA live region politeness */
  ariaLive: 'polite' | 'assertive' | 'off';
  /** Priority for screen reader announcement */
  priority: 'high' | 'medium' | 'low';
  /** Whether this requires immediate attention */
  urgent: boolean;
}

/**
 * Health status categories for accessible descriptions
 */
export enum HealthStatusCategory {
  Normal = 'normal',
  Attention = 'attention',
  Warning = 'warning',
  Critical = 'critical',
  Unknown = 'unknown',
}

/**
 * Accessible navigation context
 */
export interface AccessibleNavigationContext {
  /** Current location in the data hierarchy */
  currentPath: string[];
  /** Total items at current level */
  totalItems: number;
  /** Current item index (1-based) */
  currentIndex: number;
  /** Available navigation actions */
  availableActions: NavigationAction[];
}

/**
 * Navigation actions with keyboard shortcuts
 */
export interface NavigationAction {
  /** Action identifier */
  action: string;
  /** Human-readable action name */
  name: string;
  /** Keyboard shortcut (if available) */
  shortcut?: string;
  /** Screen reader description */
  ariaLabel: string;
}

/**
 * Accessible form field metadata
 */
export interface AccessibleFormField {
  /** Field identifier */
  id: string;
  /** Field label */
  label: string;
  /** Field description */
  description: string;
  /** Whether the field is required */
  required: boolean;
  /** Error message (if validation failed) */
  error?: string;
  /** Help text */
  helpText?: string;
  /** Related fields (for grouped navigation) */
  relatedFields?: string[];
  /** ARIA attributes */
  ariaAttributes: Record<string, string>;
}

/**
 * Medical terminology with accessible explanations
 */
export interface AccessibleMedicalTerm {
  /** The medical term */
  term: string;
  /** Standard code (ICD-10, SNOMED, etc.) */
  code?: string;
  /** Code system */
  codeSystem?: string;
  /** Simple explanation */
  simpleExplanation: string;
  /** Detailed explanation */
  detailedExplanation: string;
  /** Pronunciation guide (phonetic) */
  pronunciation?: string;
  /** Related terms */
  relatedTerms?: string[];
  /** Translations in supported languages */
  translations?: Map<string, string>;
}

/**
 * Accessible date/time display
 */
export interface AccessibleDateTime {
  /** ISO timestamp */
  timestamp: number;
  /** Full date string */
  fullDate: string;
  /** Relative time (e.g., "2 days ago") */
  relativeTime: string;
  /** Screen reader friendly format */
  ariaLabel: string;
  /** Time zone information */
  timezone?: string;
}

/**
 * Accessible privacy budget display
 */
export interface AccessiblePrivacyBudget {
  /** Current budget remaining */
  remaining: number;
  /** Total budget allocated */
  total: number;
  /** Percentage remaining */
  percentRemaining: number;
  /** Status category */
  status: HealthStatusCategory;
  /** Screen reader description */
  ariaLabel: string;
  /** Detailed explanation */
  explanation: string;
  /** Recommended action */
  recommendation?: string;
}

/**
 * Accessible list item for screen readers
 */
export interface AccessibleListItem<T> {
  /** The item data */
  item: T;
  /** Position in list (1-based) */
  position: number;
  /** Total items in list */
  total: number;
  /** Item label for screen readers */
  label: string;
  /** Item description */
  description: string;
  /** Whether this item is selected */
  selected?: boolean;
  /** Whether this item is expandable */
  expandable?: boolean;
  /** Whether this item is expanded */
  expanded?: boolean;
}

/**
 * Contrast ratio result
 */
export interface ContrastResult {
  /** Calculated contrast ratio */
  ratio: number;
  /** Whether it passes WCAG AA for normal text */
  passesAA: boolean;
  /** Whether it passes WCAG AAA for normal text */
  passesAAA: boolean;
  /** Whether it passes WCAG AA for large text */
  passesAALarge: boolean;
  /** Whether it passes WCAG AAA for large text */
  passesAAALarge: boolean;
}
