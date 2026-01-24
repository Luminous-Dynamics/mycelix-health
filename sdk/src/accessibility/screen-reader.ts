/**
 * Screen Reader Support Utilities for Mycelix Health SDK
 *
 * Provides utilities for generating screen reader friendly output
 * for health data, medical records, and clinical information.
 */

import {
  type AccessibleDisplay,
  type AccessibleHealthStatus,
  type AccessibleDateTime,
  type AccessibleListItem,
  type AccessiblePrivacyBudget,
  HealthStatusCategory,
  ReadingLevel,
} from './types';
import {
  getRiskLevelLabel,
  getSeverityLabel,
  SDOH_CATEGORY_LABELS,
  CHRONIC_CONDITION_LABELS,
} from './aria-labels';

/**
 * Format a timestamp for screen readers
 */
export function formatAccessibleDateTime(timestamp: number): AccessibleDateTime {
  const date = new Date(timestamp / 1000); // Holochain timestamps are in microseconds
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  let relativeTime: string;
  if (diffDays === 0) {
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    if (diffHours === 0) {
      const diffMinutes = Math.floor(diffMs / (1000 * 60));
      relativeTime = diffMinutes <= 1 ? 'just now' : `${diffMinutes} minutes ago`;
    } else {
      relativeTime = diffHours === 1 ? '1 hour ago' : `${diffHours} hours ago`;
    }
  } else if (diffDays === 1) {
    relativeTime = 'yesterday';
  } else if (diffDays < 7) {
    relativeTime = `${diffDays} days ago`;
  } else if (diffDays < 30) {
    const weeks = Math.floor(diffDays / 7);
    relativeTime = weeks === 1 ? '1 week ago' : `${weeks} weeks ago`;
  } else if (diffDays < 365) {
    const months = Math.floor(diffDays / 30);
    relativeTime = months === 1 ? '1 month ago' : `${months} months ago`;
  } else {
    const years = Math.floor(diffDays / 365);
    relativeTime = years === 1 ? '1 year ago' : `${years} years ago`;
  }

  const fullDate = date.toLocaleDateString('en-US', {
    weekday: 'long',
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  });

  return {
    timestamp,
    fullDate,
    relativeTime,
    ariaLabel: `${fullDate}, ${relativeTime}`,
  };
}

/**
 * Create accessible display for a patient record
 */
export function formatPatientForScreenReader(patient: {
  name: string;
  dateOfBirth?: number;
  gender?: string;
  allergies?: string[];
  conditions?: string[];
}): AccessibleDisplay<typeof patient> {
  const attributes = new Map<string, string>();
  attributes.set('Name', patient.name);

  if (patient.dateOfBirth) {
    const dob = new Date(patient.dateOfBirth / 1000);
    const age = Math.floor((Date.now() - dob.getTime()) / (1000 * 60 * 60 * 24 * 365));
    attributes.set('Age', `${age} years old`);
  }

  if (patient.gender) {
    attributes.set('Gender', patient.gender);
  }

  if (patient.allergies && patient.allergies.length > 0) {
    const allergyCount = patient.allergies.length;
    attributes.set(
      'Allergies',
      allergyCount === 1
        ? `1 allergy: ${patient.allergies[0]}`
        : `${allergyCount} allergies: ${patient.allergies.join(', ')}`
    );
  } else {
    attributes.set('Allergies', 'No known allergies');
  }

  if (patient.conditions && patient.conditions.length > 0) {
    attributes.set('Active Conditions', patient.conditions.join(', '));
  }

  const summary = `Patient ${patient.name}`;
  const description = Array.from(attributes.entries())
    .map(([key, value]) => `${key}: ${value}`)
    .join('. ');

  return {
    data: patient,
    ariaLabel: summary,
    summary,
    description,
    attributes,
  };
}

/**
 * Create accessible display for SDOH screening results
 */
export function formatSdohScreeningForScreenReader(screening: {
  overallRiskLevel: string;
  categoriesAtRisk: string[];
  completedDate?: number;
}): AccessibleDisplay<typeof screening> {
  const { label: riskLabel } = getRiskLevelLabel(screening.overallRiskLevel);
  const attributes = new Map<string, string>();

  attributes.set('Overall Risk', riskLabel);

  if (screening.categoriesAtRisk.length > 0) {
    const categoryLabels = screening.categoriesAtRisk.map(
      (cat) => SDOH_CATEGORY_LABELS[cat] || cat
    );
    attributes.set(
      'Areas of Concern',
      `${categoryLabels.length} areas: ${categoryLabels.join(', ')}`
    );
  } else {
    attributes.set('Areas of Concern', 'No areas of concern identified');
  }

  if (screening.completedDate) {
    const dateInfo = formatAccessibleDateTime(screening.completedDate);
    attributes.set('Screening Date', dateInfo.fullDate);
  }

  const summary = `Social determinants screening: ${riskLabel}`;
  const description = Array.from(attributes.entries())
    .map(([key, value]) => `${key}: ${value}`)
    .join('. ');

  return {
    data: screening,
    ariaLabel: summary,
    summary,
    description,
    attributes,
  };
}

/**
 * Create accessible display for mental health screening
 */
export function formatMentalHealthScreeningForScreenReader(screening: {
  instrument: string;
  score: number;
  severity: string;
  crisisIndicators: boolean;
}): AccessibleDisplay<typeof screening> {
  const severityLabel = getSeverityLabel(screening.severity);
  const attributes = new Map<string, string>();

  attributes.set('Screening Type', screening.instrument);
  attributes.set('Score', screening.score.toString());
  attributes.set('Severity', severityLabel);
  attributes.set(
    'Crisis Indicators',
    screening.crisisIndicators
      ? 'Crisis indicators present - immediate attention recommended'
      : 'No crisis indicators'
  );

  const summary = screening.crisisIndicators
    ? `Mental health screening: ${severityLabel}. Crisis indicators present.`
    : `Mental health screening: ${severityLabel}`;

  const description = Array.from(attributes.entries())
    .map(([key, value]) => `${key}: ${value}`)
    .join('. ');

  return {
    data: screening,
    ariaLabel: summary,
    summary,
    description,
    attributes,
  };
}

/**
 * Create accessible display for chronic care enrollment
 */
export function formatChronicCareForScreenReader(enrollment: {
  condition: string;
  enrollmentDate?: number;
  isActive: boolean;
  pendingAlerts?: number;
}): AccessibleDisplay<typeof enrollment> {
  const conditionLabel = CHRONIC_CONDITION_LABELS[enrollment.condition] || enrollment.condition;
  const attributes = new Map<string, string>();

  attributes.set('Condition', conditionLabel);
  attributes.set('Status', enrollment.isActive ? 'Actively enrolled' : 'Enrollment inactive');

  if (enrollment.enrollmentDate) {
    const dateInfo = formatAccessibleDateTime(enrollment.enrollmentDate);
    attributes.set('Enrolled', dateInfo.relativeTime);
  }

  if (enrollment.pendingAlerts !== undefined && enrollment.pendingAlerts > 0) {
    attributes.set(
      'Alerts',
      enrollment.pendingAlerts === 1
        ? '1 pending alert'
        : `${enrollment.pendingAlerts} pending alerts`
    );
  }

  const summary = `Chronic care: ${conditionLabel}, ${enrollment.isActive ? 'active' : 'inactive'}`;
  const description = Array.from(attributes.entries())
    .map(([key, value]) => `${key}: ${value}`)
    .join('. ');

  return {
    data: enrollment,
    ariaLabel: summary,
    summary,
    description,
    attributes,
  };
}

/**
 * Create accessible list navigation
 */
export function formatListForScreenReader<T>(
  items: T[],
  itemFormatter: (item: T) => { label: string; description: string }
): AccessibleListItem<T>[] {
  return items.map((item, index) => {
    const formatted = itemFormatter(item);
    return {
      item,
      position: index + 1,
      total: items.length,
      label: `Item ${index + 1} of ${items.length}: ${formatted.label}`,
      description: formatted.description,
    };
  });
}

/**
 * Create accessible health status indicator
 */
export function createAccessibleHealthStatus(
  status: HealthStatusCategory,
  details: string
): AccessibleHealthStatus {
  const statusDescriptions: Record<HealthStatusCategory, string> = {
    normal: 'Status normal, no action required',
    attention: 'Needs attention, review recommended',
    warning: 'Warning, action may be needed',
    critical: 'Critical, immediate action required',
    unknown: 'Status unknown',
  };

  const isUrgent = status === 'critical' || status === 'warning';

  return {
    status,
    statusDescription: `${statusDescriptions[status]}. ${details}`,
    ariaLive: isUrgent ? 'assertive' : 'polite',
    priority: status === 'critical' ? 'high' : status === 'warning' ? 'medium' : 'low',
    urgent: isUrgent,
  };
}

/**
 * Create accessible privacy budget display
 */
export function formatPrivacyBudgetForScreenReader(
  remaining: number,
  total: number
): AccessiblePrivacyBudget {
  const percentRemaining = (remaining / total) * 100;

  let status: HealthStatusCategory;
  let recommendation: string | undefined;

  if (percentRemaining > 50) {
    status = HealthStatusCategory.Normal;
  } else if (percentRemaining > 25) {
    status = HealthStatusCategory.Attention;
    recommendation = 'Consider limiting queries to preserve remaining budget';
  } else if (percentRemaining > 10) {
    status = HealthStatusCategory.Warning;
    recommendation = 'Privacy budget is low. Prioritize essential queries only.';
  } else {
    status = HealthStatusCategory.Critical;
    recommendation = 'Privacy budget nearly exhausted. Only critical queries recommended.';
  }

  const explanation = `Your privacy budget protects your data by limiting how much information can be extracted. ${
    percentRemaining > 0
      ? `You have ${percentRemaining.toFixed(1)}% of your budget remaining.`
      : 'Your budget is exhausted. No more queries can be made until it renews.'
  }`;

  const result: AccessiblePrivacyBudget = {
    remaining,
    total,
    percentRemaining,
    status,
    ariaLabel: `Privacy budget: ${percentRemaining.toFixed(0)}% remaining, ${remaining.toFixed(
      2
    )} of ${total} epsilon`,
    explanation,
  };

  if (recommendation) {
    result.recommendation = recommendation;
  }

  return result;
}

/**
 * Format medical term for screen reader with appropriate reading level
 */
export function formatMedicalTermForScreenReader(
  term: string,
  code: string | undefined,
  readingLevel: ReadingLevel = ReadingLevel.Standard
): string {
  const pronunciations: Record<string, string> = {
    hypertension: 'hy-per-TEN-shun',
    diabetes: 'dy-uh-BEE-teez',
    hypoglycemia: 'hy-poh-gly-SEE-mee-uh',
    tachycardia: 'tak-ih-KAR-dee-uh',
    dyspnea: 'DISP-nee-uh',
  };

  const simpleExplanations: Record<string, string> = {
    hypertension: 'high blood pressure',
    diabetes: 'a condition where blood sugar is too high',
    hypoglycemia: 'low blood sugar',
    tachycardia: 'fast heartbeat',
    dyspnea: 'difficulty breathing',
  };

  let result = term;
  const termLower = term.toLowerCase();
  const simple = simpleExplanations[termLower];

  if (readingLevel === ReadingLevel.Elementary && simple) {
    result = simple;
  } else if (readingLevel === ReadingLevel.Intermediate && simple) {
    result = `${term}, which means ${simple}`;
  }

  if (code && (readingLevel === ReadingLevel.Advanced || readingLevel === ReadingLevel.Professional)) {
    result = `${result} (code: ${code})`;
  }

  const pronunciation = pronunciations[termLower];
  if (pronunciation && readingLevel !== ReadingLevel.Professional) {
    result = `${result}, pronounced ${pronunciation}`;
  }

  return result;
}

/**
 * Generate announcement text for screen readers
 */
export function generateAnnouncement(
  type: 'success' | 'error' | 'warning' | 'info',
  message: string,
  details?: string
): { text: string; ariaLive: 'polite' | 'assertive' } {
  const prefixes = {
    success: 'Success',
    error: 'Error',
    warning: 'Warning',
    info: 'Information',
  };

  const text = details
    ? `${prefixes[type]}: ${message}. ${details}`
    : `${prefixes[type]}: ${message}`;

  const ariaLive = type === 'error' || type === 'warning' ? 'assertive' : 'polite';

  return { text, ariaLive };
}
