import { describe, it, expect } from 'vitest';
import {
  // Types
  ReadingLevel,
  HealthStatusCategory,

  // ARIA labels
  RISK_LEVEL_LABELS,
  SEVERITY_LABELS,
  CRISIS_LEVEL_LABELS,
  VACCINE_TYPE_LABELS,
  DEVELOPMENTAL_DOMAIN_LABELS,
  getRiskLevelLabel,
  getSeverityLabel,
  getCrisisLevelLabel,
  formatScopesForScreenReader,
  getReadingLevelDescription,

  // Screen reader utilities
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

  // Contrast utilities
  checkContrast,
  CONTRAST_THRESHOLDS,
  HIGH_CONTRAST_COLORS,
  KEYBOARD_SHORTCUTS,
  generateKeyboardHelpText,
  DEFAULT_ACCESSIBILITY_CONFIG,
} from '../src/accessibility';

describe('ReadingLevel enum', () => {
  it('should have all reading levels', () => {
    expect(ReadingLevel.Elementary).toBe('elementary');
    expect(ReadingLevel.Intermediate).toBe('intermediate');
    expect(ReadingLevel.Standard).toBe('standard');
    expect(ReadingLevel.Advanced).toBe('advanced');
    expect(ReadingLevel.Professional).toBe('professional');
  });
});

describe('HealthStatusCategory enum', () => {
  it('should have all status categories', () => {
    expect(HealthStatusCategory.Normal).toBe('normal');
    expect(HealthStatusCategory.Attention).toBe('attention');
    expect(HealthStatusCategory.Warning).toBe('warning');
    expect(HealthStatusCategory.Critical).toBe('critical');
    expect(HealthStatusCategory.Unknown).toBe('unknown');
  });
});

describe('ARIA Label Maps', () => {
  it('should have risk level labels', () => {
    expect(RISK_LEVEL_LABELS['NoRisk']).toBe('No risk identified');
    expect(RISK_LEVEL_LABELS['LowRisk']).toBe('Low risk, monitoring recommended');
    expect(RISK_LEVEL_LABELS['HighRisk']).toBe('High risk, intervention recommended');
  });

  it('should have severity labels', () => {
    expect(SEVERITY_LABELS['None']).toBe('No symptoms');
    expect(SEVERITY_LABELS['Moderate']).toBe('Moderate symptoms');
    expect(SEVERITY_LABELS['Severe']).toBe('Severe symptoms');
  });

  it('should have crisis level labels', () => {
    expect(CRISIS_LEVEL_LABELS['None']).toBe('No crisis indicators');
    expect(CRISIS_LEVEL_LABELS['Imminent']).toBe('Imminent danger, emergency response required');
  });

  it('should have vaccine type labels', () => {
    expect(VACCINE_TYPE_LABELS['DTaP']).toContain('Diphtheria');
    expect(VACCINE_TYPE_LABELS['MMR']).toContain('Measles');
    expect(VACCINE_TYPE_LABELS['COVID19']).toContain('COVID');
  });

  it('should have developmental domain labels', () => {
    expect(DEVELOPMENTAL_DOMAIN_LABELS['GrossMotor']).toBeDefined();
    expect(DEVELOPMENTAL_DOMAIN_LABELS['Language']).toBeDefined();
    expect(DEVELOPMENTAL_DOMAIN_LABELS['Cognitive']).toBeDefined();
  });
});

describe('getRiskLevelLabel', () => {
  it('should return label and ariaLive status', () => {
    const low = getRiskLevelLabel('LowRisk');
    expect(low.label).toBe('Low risk, monitoring recommended');
    expect(low.ariaLive).toBe('polite');

    const high = getRiskLevelLabel('HighRisk');
    expect(high.label).toBe('High risk, intervention recommended');
    expect(high.ariaLive).toBe('assertive');
  });

  it('should handle unknown levels', () => {
    const unknown = getRiskLevelLabel('SomeUnknownLevel');
    expect(unknown.label).toContain('SomeUnknownLevel');
    expect(unknown.ariaLive).toBe('polite');
  });
});

describe('getSeverityLabel', () => {
  it('should return severity description', () => {
    expect(getSeverityLabel('None')).toBe('No symptoms');
    expect(getSeverityLabel('Mild')).toBe('Mild symptoms');
    expect(getSeverityLabel('Severe')).toBe('Severe symptoms');
  });
});

describe('getCrisisLevelLabel', () => {
  it('should return label with emergency info', () => {
    const none = getCrisisLevelLabel('None');
    expect(none.label).toBe('No crisis indicators');
    expect(none.isEmergency).toBe(false);
    expect(none.ariaLive).toBe('polite');

    const imminent = getCrisisLevelLabel('Imminent');
    expect(imminent.label).toBe('Imminent danger, emergency response required');
    expect(imminent.isEmergency).toBe(true);
    expect(imminent.ariaLive).toBe('assertive');
  });
});

describe('formatScopesForScreenReader', () => {
  it('should handle empty scopes', () => {
    expect(formatScopesForScreenReader([])).toBe('No access permissions granted');
  });

  it('should handle single scope', () => {
    const result = formatScopesForScreenReader(['read']);
    expect(result).toContain('One permission');
  });

  it('should handle multiple scopes', () => {
    const result = formatScopesForScreenReader(['read', 'write', 'share']);
    expect(result).toContain('3 permissions');
  });
});

describe('getReadingLevelDescription', () => {
  it('should return descriptions for all levels', () => {
    expect(getReadingLevelDescription(ReadingLevel.Elementary)).toContain('Simple words');
    expect(getReadingLevelDescription(ReadingLevel.Standard)).toContain('Standard');
    expect(getReadingLevelDescription(ReadingLevel.Professional)).toContain('Medical professional');
  });
});

describe('formatAccessibleDateTime', () => {
  it('should format recent timestamps', () => {
    const now = Date.now() * 1000; // Convert to microseconds (Holochain format)
    const result = formatAccessibleDateTime(now);

    expect(result.timestamp).toBe(now);
    expect(result.fullDate).toBeDefined();
    expect(result.relativeTime).toBe('just now');
    expect(result.ariaLabel).toContain(result.relativeTime);
  });

  it('should format older timestamps', () => {
    const twoDaysAgo = (Date.now() - 2 * 24 * 60 * 60 * 1000) * 1000;
    const result = formatAccessibleDateTime(twoDaysAgo);

    expect(result.relativeTime).toBe('2 days ago');
  });
});

describe('formatPatientForScreenReader', () => {
  it('should format basic patient info', () => {
    const result = formatPatientForScreenReader({
      name: 'John Doe',
    });

    expect(result.ariaLabel).toBe('Patient John Doe');
    expect(result.summary).toBe('Patient John Doe');
    expect(result.attributes.get('Name')).toBe('John Doe');
  });

  it('should include allergies', () => {
    const result = formatPatientForScreenReader({
      name: 'Jane Doe',
      allergies: ['Penicillin', 'Peanuts'],
    });

    expect(result.attributes.get('Allergies')).toContain('2 allergies');
  });

  it('should show no allergies message', () => {
    const result = formatPatientForScreenReader({
      name: 'Jane Doe',
      allergies: [],
    });

    expect(result.attributes.get('Allergies')).toBe('No known allergies');
  });
});

describe('formatSdohScreeningForScreenReader', () => {
  it('should format screening with risks', () => {
    const result = formatSdohScreeningForScreenReader({
      overallRiskLevel: 'ModerateRisk',
      categoriesAtRisk: ['FoodInsecurity', 'TransportationAccess'],
    });

    expect(result.summary).toContain('Moderate risk');
    expect(result.attributes.get('Areas of Concern')).toContain('2 areas');
  });

  it('should handle no areas of concern', () => {
    const result = formatSdohScreeningForScreenReader({
      overallRiskLevel: 'NoRisk',
      categoriesAtRisk: [],
    });

    expect(result.attributes.get('Areas of Concern')).toBe('No areas of concern identified');
  });
});

describe('formatMentalHealthScreeningForScreenReader', () => {
  it('should format screening with crisis indicators', () => {
    const result = formatMentalHealthScreeningForScreenReader({
      instrument: 'PHQ-9',
      score: 22,
      severity: 'Severe',
      crisisIndicators: true,
    });

    expect(result.summary).toContain('Crisis indicators present');
    expect(result.attributes.get('Crisis Indicators')).toContain('immediate attention');
  });

  it('should format normal screening', () => {
    const result = formatMentalHealthScreeningForScreenReader({
      instrument: 'PHQ-9',
      score: 4,
      severity: 'Minimal',
      crisisIndicators: false,
    });

    expect(result.attributes.get('Crisis Indicators')).toBe('No crisis indicators');
  });
});

describe('formatChronicCareForScreenReader', () => {
  it('should format active enrollment', () => {
    const result = formatChronicCareForScreenReader({
      condition: 'Diabetes',
      isActive: true,
      pendingAlerts: 2,
    });

    expect(result.summary).toContain('Diabetes');
    expect(result.summary).toContain('active');
    expect(result.attributes.get('Alerts')).toBe('2 pending alerts');
  });

  it('should format inactive enrollment', () => {
    const result = formatChronicCareForScreenReader({
      condition: 'Hypertension',
      isActive: false,
    });

    expect(result.attributes.get('Status')).toBe('Enrollment inactive');
  });
});

describe('formatListForScreenReader', () => {
  it('should format list with positions', () => {
    const items = ['Apple', 'Banana', 'Cherry'];
    const result = formatListForScreenReader(items, (item) => ({
      label: item,
      description: `A ${item.toLowerCase()}`,
    }));

    expect(result.length).toBe(3);
    expect(result[0].position).toBe(1);
    expect(result[0].total).toBe(3);
    expect(result[0].label).toContain('Item 1 of 3');
    expect(result[2].label).toContain('Item 3 of 3');
  });
});

describe('createAccessibleHealthStatus', () => {
  it('should create normal status', () => {
    const result = createAccessibleHealthStatus(HealthStatusCategory.Normal, 'All vitals stable');

    expect(result.status).toBe(HealthStatusCategory.Normal);
    expect(result.ariaLive).toBe('polite');
    expect(result.priority).toBe('low');
    expect(result.urgent).toBe(false);
  });

  it('should create critical status', () => {
    const result = createAccessibleHealthStatus(HealthStatusCategory.Critical, 'Immediate intervention needed');

    expect(result.status).toBe(HealthStatusCategory.Critical);
    expect(result.ariaLive).toBe('assertive');
    expect(result.priority).toBe('high');
    expect(result.urgent).toBe(true);
  });
});

describe('formatPrivacyBudgetForScreenReader', () => {
  it('should format healthy budget', () => {
    const result = formatPrivacyBudgetForScreenReader(8.0, 10.0);

    expect(result.remaining).toBe(8.0);
    expect(result.total).toBe(10.0);
    expect(result.percentRemaining).toBe(80);
    expect(result.status).toBe(HealthStatusCategory.Normal);
    expect(result.recommendation).toBeUndefined();
  });

  it('should format low budget with recommendation', () => {
    const result = formatPrivacyBudgetForScreenReader(1.5, 10.0);

    expect(result.percentRemaining).toBe(15);
    expect(result.status).toBe(HealthStatusCategory.Warning);
    expect(result.recommendation).toContain('low');
  });

  it('should format critical budget', () => {
    const result = formatPrivacyBudgetForScreenReader(0.5, 10.0);

    expect(result.status).toBe(HealthStatusCategory.Critical);
    expect(result.recommendation).toContain('exhausted');
  });
});

describe('formatMedicalTermForScreenReader', () => {
  it('should format at standard level', () => {
    const result = formatMedicalTermForScreenReader('hypertension', undefined, ReadingLevel.Standard);
    expect(result).toContain('hypertension');
  });

  it('should simplify at elementary level', () => {
    const result = formatMedicalTermForScreenReader('hypertension', undefined, ReadingLevel.Elementary);
    expect(result).toContain('high blood pressure');
  });

  it('should explain at intermediate level', () => {
    const result = formatMedicalTermForScreenReader('hypertension', undefined, ReadingLevel.Intermediate);
    expect(result).toContain('which means');
    expect(result).toContain('high blood pressure');
  });

  it('should include code at professional level', () => {
    const result = formatMedicalTermForScreenReader('hypertension', 'I10', ReadingLevel.Professional);
    expect(result).toContain('code: I10');
  });
});

describe('generateAnnouncement', () => {
  it('should generate success announcement', () => {
    const result = generateAnnouncement('success', 'Record saved');
    expect(result.text).toContain('Success');
    expect(result.text).toContain('Record saved');
    expect(result.ariaLive).toBe('polite');
  });

  it('should generate error announcement', () => {
    const result = generateAnnouncement('error', 'Save failed', 'Please try again');
    expect(result.text).toContain('Error');
    expect(result.text).toContain('Please try again');
    expect(result.ariaLive).toBe('assertive');
  });
});

describe('checkContrast', () => {
  it('should calculate contrast ratio', () => {
    // Black on white should be ~21:1
    const result = checkContrast('#000000', '#FFFFFF');
    expect(result.ratio).toBeGreaterThan(20);
    expect(result.passesAA).toBe(true);
    expect(result.passesAAA).toBe(true);
  });

  it('should detect insufficient contrast', () => {
    // Light gray on white has low contrast
    const result = checkContrast('#CCCCCC', '#FFFFFF');
    expect(result.ratio).toBeLessThan(3);
    expect(result.passesAA).toBe(false);
  });

  it('should handle invalid hex colors gracefully', () => {
    // When invalid, getLuminance returns 0, so ratio depends on valid color
    const result = checkContrast('invalid', '#FFFFFF');
    // The ratio will be high (21:1) because invalid returns 0 luminance
    expect(result.ratio).toBeGreaterThan(1);
  });
});

describe('CONTRAST_THRESHOLDS', () => {
  it('should have WCAG 2.1 thresholds', () => {
    expect(CONTRAST_THRESHOLDS.AA_NORMAL).toBe(4.5);
    expect(CONTRAST_THRESHOLDS.AA_LARGE).toBe(3.0);
    expect(CONTRAST_THRESHOLDS.AAA_NORMAL).toBe(7.0);
    expect(CONTRAST_THRESHOLDS.AAA_LARGE).toBe(4.5);
  });
});

describe('HIGH_CONTRAST_COLORS', () => {
  it('should have all color categories defined', () => {
    expect(HIGH_CONTRAST_COLORS.normal).toBeDefined();
    expect(HIGH_CONTRAST_COLORS.warning).toBeDefined();
    expect(HIGH_CONTRAST_COLORS.critical).toBeDefined();
    expect(HIGH_CONTRAST_COLORS.info).toBeDefined();
  });

  it('should have foreground and background for each color', () => {
    for (const [name, colors] of Object.entries(HIGH_CONTRAST_COLORS)) {
      expect(colors.foreground).toBeDefined();
      expect(colors.background).toBeDefined();
      expect(colors.foreground).toMatch(/^#[0-9A-Fa-f]{6}$/);
      expect(colors.background).toMatch(/^#[0-9A-Fa-f]{6}$/);
    }
  });
});

describe('KEYBOARD_SHORTCUTS', () => {
  it('should have navigation shortcuts', () => {
    expect(KEYBOARD_SHORTCUTS.navigation.length).toBeGreaterThan(0);
    expect(KEYBOARD_SHORTCUTS.navigation[0].key).toBeDefined();
    expect(KEYBOARD_SHORTCUTS.navigation[0].description).toBeDefined();
  });

  it('should have list shortcuts', () => {
    expect(KEYBOARD_SHORTCUTS.lists.length).toBeGreaterThan(0);
  });

  it('should have form shortcuts', () => {
    expect(KEYBOARD_SHORTCUTS.forms.length).toBeGreaterThan(0);
  });
});

describe('generateKeyboardHelpText', () => {
  it('should generate help text for navigation', () => {
    const text = generateKeyboardHelpText('navigation');
    expect(text).toContain('Tab');
    expect(text).toContain('Escape');
  });

  it('should generate help text for lists', () => {
    const text = generateKeyboardHelpText('lists');
    expect(text).toContain('Arrow');
  });
});

describe('DEFAULT_ACCESSIBILITY_CONFIG', () => {
  it('should have sensible defaults', () => {
    expect(DEFAULT_ACCESSIBILITY_CONFIG.language).toBe('en');
    expect(DEFAULT_ACCESSIBILITY_CONFIG.highContrast).toBe(false);
    expect(DEFAULT_ACCESSIBILITY_CONFIG.extendedDescriptions).toBe(false);
    expect(DEFAULT_ACCESSIBILITY_CONFIG.readingLevel).toBe('standard');
    expect(DEFAULT_ACCESSIBILITY_CONFIG.includePronunciation).toBe(true);
  });
});
