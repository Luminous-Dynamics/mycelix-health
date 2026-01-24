/**
 * Accessibility Module for Mycelix Health SDK
 *
 * Provides comprehensive accessibility support including:
 * - ARIA labels for all domain types
 * - Screen reader friendly formatters
 * - Medical terminology explanations
 * - Internationalization support
 * - Contrast checking utilities
 *
 * @example
 * ```typescript
 * import { formatPatientForScreenReader, getRiskLevelLabel } from '@mycelix/health-sdk/accessibility';
 *
 * const patient = await client.patients.getPatient(hash);
 * const accessible = formatPatientForScreenReader(patient);
 * console.log(accessible.ariaLabel); // "Patient John Doe"
 * console.log(accessible.description); // Full description for screen readers
 * ```
 */

// Export types
export * from './types';

// Export ARIA label utilities
export {
  // Label maps
  CONSENT_SCOPE_LABELS,
  TRIAL_PHASE_LABELS,
  TRIAL_STATUS_LABELS,
  SDOH_DOMAIN_LABELS,
  SDOH_CATEGORY_LABELS,
  RISK_LEVEL_LABELS,
  SEVERITY_LABELS,
  CRISIS_LEVEL_LABELS,
  MENTAL_HEALTH_INSTRUMENT_LABELS,
  CHRONIC_CONDITION_LABELS,
  VACCINE_TYPE_LABELS,
  DEVELOPMENTAL_DOMAIN_LABELS,
  MILESTONE_STATUS_LABELS,
  ALERT_SEVERITY_LABELS,
  INTERVENTION_STATUS_LABELS,
  // Label getter functions
  getConsentScopeLabel,
  getTrialPhaseLabel,
  getRiskLevelLabel,
  getSeverityLabel,
  getCrisisLevelLabel,
  formatScopesForScreenReader,
  getReadingLevelDescription,
} from './aria-labels';

// Export screen reader utilities
export {
  formatAccessibleDateTime,
  formatPatientForScreenReader,
  formatSdohScreeningForScreenReader,
  formatMentalHealthScreeningForScreenReader,
  formatChronicCareForScreenReader,
  formatListForScreenReader,
  createAccessibleHealthStatus,
  formatPrivacyBudgetForScreenReader,
  formatMedicalTermForScreenReader,
  generateAnnouncement,
} from './screen-reader';

/**
 * Default accessibility configuration
 */
export const DEFAULT_ACCESSIBILITY_CONFIG = {
  language: 'en',
  highContrast: false,
  extendedDescriptions: false,
  readingLevel: 'standard' as const,
  includePronunciation: true,
};

/**
 * WCAG 2.1 contrast ratio thresholds
 */
export const CONTRAST_THRESHOLDS = {
  AA_NORMAL: 4.5,
  AA_LARGE: 3.0,
  AAA_NORMAL: 7.0,
  AAA_LARGE: 4.5,
};

/**
 * Check if a contrast ratio meets WCAG requirements
 */
export function checkContrast(
  foreground: string,
  background: string
): {
  ratio: number;
  passesAA: boolean;
  passesAAA: boolean;
  passesAALarge: boolean;
  passesAAALarge: boolean;
} {
  const getLuminance = (hex: string): number => {
    const rgb = hexToRgb(hex);
    if (!rgb) return 0;

    const toLinear = (c: number): number => {
      c = c / 255;
      return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
    };

    const r = toLinear(rgb.r);
    const g = toLinear(rgb.g);
    const b = toLinear(rgb.b);

    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  };

  const l1 = getLuminance(foreground);
  const l2 = getLuminance(background);
  const ratio = (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);

  return {
    ratio,
    passesAA: ratio >= CONTRAST_THRESHOLDS.AA_NORMAL,
    passesAAA: ratio >= CONTRAST_THRESHOLDS.AAA_NORMAL,
    passesAALarge: ratio >= CONTRAST_THRESHOLDS.AA_LARGE,
    passesAAALarge: ratio >= CONTRAST_THRESHOLDS.AAA_LARGE,
  };
}

/**
 * Convert hex color to RGB
 */
function hexToRgb(hex: string): { r: number; g: number; b: number } | null {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  if (!result || !result[1] || !result[2] || !result[3]) {
    return null;
  }
  return {
    r: parseInt(result[1], 16),
    g: parseInt(result[2], 16),
    b: parseInt(result[3], 16),
  };
}

/**
 * Get recommended high contrast colors
 */
export const HIGH_CONTRAST_COLORS = {
  // Status colors
  normal: { foreground: '#006400', background: '#E8F5E9' },
  warning: { foreground: '#8B4513', background: '#FFF3E0' },
  critical: { foreground: '#8B0000', background: '#FFEBEE' },
  info: { foreground: '#00008B', background: '#E3F2FD' },

  // Risk levels
  noRisk: { foreground: '#006400', background: '#E8F5E9' },
  lowRisk: { foreground: '#2E7D32', background: '#F1F8E9' },
  moderateRisk: { foreground: '#FF8F00', background: '#FFF8E1' },
  highRisk: { foreground: '#D84315', background: '#FBE9E7' },
  urgent: { foreground: '#B71C1C', background: '#FFEBEE' },
};

/**
 * Keyboard shortcut help text
 */
export const KEYBOARD_SHORTCUTS = {
  navigation: [
    { key: 'Tab', description: 'Move to next focusable element' },
    { key: 'Shift+Tab', description: 'Move to previous focusable element' },
    { key: 'Enter', description: 'Activate selected item' },
    { key: 'Space', description: 'Toggle selection or expand/collapse' },
    { key: 'Escape', description: 'Close dialog or cancel action' },
  ],
  lists: [
    { key: 'Arrow Down', description: 'Move to next item' },
    { key: 'Arrow Up', description: 'Move to previous item' },
    { key: 'Home', description: 'Move to first item' },
    { key: 'End', description: 'Move to last item' },
  ],
  forms: [
    { key: 'Tab', description: 'Move to next field' },
    { key: 'Shift+Tab', description: 'Move to previous field' },
    { key: 'Enter', description: 'Submit form' },
    { key: 'Escape', description: 'Cancel form' },
  ],
};

/**
 * Generate keyboard shortcut help text for screen readers
 */
export function generateKeyboardHelpText(context: 'navigation' | 'lists' | 'forms'): string {
  const shortcuts = KEYBOARD_SHORTCUTS[context];
  return shortcuts.map((s) => `${s.key}: ${s.description}`).join('. ');
}
