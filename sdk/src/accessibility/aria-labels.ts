/**
 * ARIA Labels for Mycelix Health SDK
 *
 * Provides human-readable, screen-reader-friendly labels for all
 * enums and domain types used in the SDK.
 */

import type { ReadingLevel } from './types';

/**
 * Consent scope labels
 */
export const CONSENT_SCOPE_LABELS: Record<string, string> = {
  ReadBasicInfo: 'Read basic patient information',
  ReadMedicalRecords: 'Read medical records',
  WriteMedicalRecords: 'Write to medical records',
  ReadDiagnoses: 'Read diagnoses',
  WriteDiagnoses: 'Write diagnoses',
  ReadMedications: 'Read medication list',
  WriteMedications: 'Modify medication list',
  ReadLabResults: 'Read laboratory results',
  WriteLabResults: 'Write laboratory results',
  ReadImagingReports: 'Read imaging and radiology reports',
  WriteImagingReports: 'Write imaging reports',
  ReadVitalSigns: 'Read vital signs',
  WriteVitalSigns: 'Record vital signs',
  ReadAllergies: 'Read allergy information',
  WriteAllergies: 'Update allergy information',
  ReadImmunizations: 'Read immunization records',
  WriteImmunizations: 'Record immunizations',
  ReadClinicalNotes: 'Read clinical notes',
  WriteClinicalNotes: 'Write clinical notes',
  ReadMentalHealth: 'Read mental health records',
  WriteMentalHealth: 'Write mental health records',
  ReadSubstanceAbuse: 'Read substance abuse records (42 CFR Part 2 protected)',
  WriteSubstanceAbuse: 'Write substance abuse records',
  ParticipateInResearch: 'Participate in research studies',
  ShareWithFamily: 'Share records with designated family members',
  EmergencyAccess: 'Emergency access to records',
  FullAccess: 'Full access to all medical records',
};

/**
 * Trial phase labels
 */
export const TRIAL_PHASE_LABELS: Record<string, string> = {
  Preclinical: 'Preclinical phase, laboratory and animal studies',
  Phase1: 'Phase 1, safety testing with small group',
  Phase2: 'Phase 2, effectiveness testing with larger group',
  Phase3: 'Phase 3, large-scale effectiveness and safety comparison',
  Phase4: 'Phase 4, post-market safety monitoring',
  Observational: 'Observational study, no intervention',
};

/**
 * Trial status labels
 */
export const TRIAL_STATUS_LABELS: Record<string, string> = {
  Draft: 'Draft, trial is being designed',
  PendingApproval: 'Pending approval from ethics committee',
  Approved: 'Approved and ready to begin',
  Recruiting: 'Currently recruiting participants',
  Active: 'Active, trial is ongoing',
  Suspended: 'Temporarily suspended',
  Completed: 'Completed successfully',
  Terminated: 'Terminated early',
  Withdrawn: 'Withdrawn before enrollment',
};

/**
 * SDOH domain labels
 */
export const SDOH_DOMAIN_LABELS: Record<string, string> = {
  EconomicStability: 'Economic stability, including employment, income, and expenses',
  EducationAccess: 'Education access and quality',
  HealthcareAccess: 'Healthcare access and quality',
  NeighborhoodEnvironment: 'Neighborhood and built environment',
  SocialCommunity: 'Social and community context',
};

/**
 * SDOH category labels
 */
export const SDOH_CATEGORY_LABELS: Record<string, string> = {
  Employment: 'Employment status and job security',
  FoodInsecurity: 'Access to adequate and nutritious food',
  HousingInstability: 'Stable and safe housing',
  Transportation: 'Access to reliable transportation',
  Utilities: 'Ability to pay for utilities',
  Literacy: 'Reading and writing ability',
  LanguageBarrier: 'Language barriers to healthcare',
  EducationLevel: 'Level of formal education',
  HealthInsurance: 'Health insurance coverage',
  ProviderAvailability: 'Access to healthcare providers',
  HealthLiteracy: 'Understanding of health information',
  SafetyConcerns: 'Safety concerns in neighborhood',
  EnvironmentalHazards: 'Environmental hazards exposure',
  AccessToHealthyFood: 'Access to healthy food options',
  SocialIsolation: 'Social isolation and loneliness',
  InterpersonalViolence: 'Interpersonal violence and abuse',
  IncarceratedFamilyMember: 'Family member currently incarcerated',
  RefugeeImmigrantStatus: 'Refugee or immigrant status',
  Discrimination: 'Experience of discrimination',
  Stress: 'Chronic stress levels',
};

/**
 * Risk level labels
 */
export const RISK_LEVEL_LABELS: Record<string, string> = {
  NoRisk: 'No risk identified',
  LowRisk: 'Low risk, monitoring recommended',
  ModerateRisk: 'Moderate risk, intervention may be helpful',
  HighRisk: 'High risk, intervention recommended',
  Urgent: 'Urgent, immediate attention needed',
};

/**
 * Mental health severity labels
 */
export const SEVERITY_LABELS: Record<string, string> = {
  None: 'No symptoms',
  Minimal: 'Minimal symptoms',
  Mild: 'Mild symptoms',
  Moderate: 'Moderate symptoms',
  ModeratelySevere: 'Moderately severe symptoms',
  Severe: 'Severe symptoms',
};

/**
 * Crisis level labels
 */
export const CRISIS_LEVEL_LABELS: Record<string, string> = {
  None: 'No crisis indicators',
  LowRisk: 'Low risk, continue monitoring',
  ModerateRisk: 'Moderate risk, safety planning recommended',
  HighRisk: 'High risk, immediate intervention needed',
  Imminent: 'Imminent danger, emergency response required',
};

/**
 * Mental health instrument labels
 */
export const MENTAL_HEALTH_INSTRUMENT_LABELS: Record<string, string> = {
  PHQ9: 'Patient Health Questionnaire 9, depression screening',
  PHQ2: 'Patient Health Questionnaire 2, brief depression screen',
  GAD7: 'Generalized Anxiety Disorder 7-item scale',
  CSSRS: 'Columbia Suicide Severity Rating Scale',
  CAGE: 'CAGE questionnaire for alcohol use',
  AUDIT: 'Alcohol Use Disorders Identification Test',
  DAST10: 'Drug Abuse Screening Test, 10 items',
  PCL5: 'PTSD Checklist for DSM-5',
  MDQ: 'Mood Disorder Questionnaire for bipolar screening',
  EPDS: 'Edinburgh Postnatal Depression Scale',
  PSC17: 'Pediatric Symptom Checklist, 17 items',
};

/**
 * Chronic condition labels
 */
export const CHRONIC_CONDITION_LABELS: Record<string, string> = {
  Diabetes: 'Diabetes mellitus',
  HeartFailure: 'Heart failure',
  COPD: 'Chronic obstructive pulmonary disease',
  ChronicKidneyDisease: 'Chronic kidney disease',
  Hypertension: 'High blood pressure',
  Asthma: 'Asthma',
  CancerSurvivorship: 'Cancer survivorship',
  MultipleSclerosis: 'Multiple sclerosis',
  RheumatoidArthritis: 'Rheumatoid arthritis',
  Obesity: 'Obesity',
};

/**
 * Pediatric vaccine labels
 */
export const VACCINE_TYPE_LABELS: Record<string, string> = {
  HepB: 'Hepatitis B vaccine',
  RV: 'Rotavirus vaccine',
  DTaP: 'Diphtheria, tetanus, and pertussis vaccine',
  Hib: 'Haemophilus influenzae type b vaccine',
  PCV13: 'Pneumococcal conjugate vaccine, 13-valent',
  IPV: 'Inactivated poliovirus vaccine',
  Influenza: 'Seasonal influenza vaccine',
  MMR: 'Measles, mumps, and rubella vaccine',
  Varicella: 'Varicella, chickenpox vaccine',
  HepA: 'Hepatitis A vaccine',
  MenACWY: 'Meningococcal conjugate vaccine',
  Tdap: 'Tetanus, diphtheria, and pertussis booster',
  HPV: 'Human papillomavirus vaccine',
  MenB: 'Meningococcal B vaccine',
  COVID19: 'COVID-19 vaccine',
};

/**
 * Developmental domain labels
 */
export const DEVELOPMENTAL_DOMAIN_LABELS: Record<string, string> = {
  GrossMotor: 'Large muscle movement skills',
  FineMotor: 'Small muscle and hand coordination skills',
  Language: 'Speaking, understanding, and communication skills',
  Cognitive: 'Thinking, learning, and problem-solving skills',
  SocialEmotional: 'Social interaction and emotional regulation skills',
  SelfHelp: 'Self-care and daily living skills',
};

/**
 * Milestone status labels
 */
export const MILESTONE_STATUS_LABELS: Record<string, string> = {
  NotYetExpected: 'Not yet expected for this age',
  Expected: 'Expected to develop at this age',
  Achieved: 'Successfully achieved',
  Delayed: 'Delayed, may need evaluation',
  AtRisk: 'At risk for delay',
  Concerning: 'Concerning, evaluation recommended',
};

/**
 * Alert severity labels
 */
export const ALERT_SEVERITY_LABELS: Record<string, string> = {
  Info: 'Informational alert',
  Warning: 'Warning, attention recommended',
  Urgent: 'Urgent, prompt action needed',
  Critical: 'Critical, immediate action required',
};

/**
 * Intervention status labels
 */
export const INTERVENTION_STATUS_LABELS: Record<string, string> = {
  Identified: 'Need identified, not yet addressed',
  ReferralMade: 'Referral made to appropriate resource',
  InProgress: 'Intervention in progress',
  Completed: 'Intervention successfully completed',
  Declined: 'Patient declined intervention',
  UnableToComplete: 'Unable to complete intervention',
};

/**
 * Get ARIA label for a consent scope
 */
export function getConsentScopeLabel(scope: string): string {
  return CONSENT_SCOPE_LABELS[scope] || `Access scope: ${scope}`;
}

/**
 * Get ARIA label for a trial phase
 */
export function getTrialPhaseLabel(phase: string): string {
  return TRIAL_PHASE_LABELS[phase] || `Trial phase: ${phase}`;
}

/**
 * Get ARIA label for a risk level with appropriate urgency
 */
export function getRiskLevelLabel(level: string): { label: string; ariaLive: 'polite' | 'assertive' | 'off' } {
  const label = RISK_LEVEL_LABELS[level] || `Risk level: ${level}`;
  const ariaLive = level === 'Urgent' || level === 'HighRisk' ? 'assertive' : 'polite';
  return { label, ariaLive };
}

/**
 * Get ARIA label for severity with screen reader optimization
 */
export function getSeverityLabel(severity: string): string {
  return SEVERITY_LABELS[severity] || `Severity: ${severity}`;
}

/**
 * Get ARIA label for crisis level with emergency indicators
 */
export function getCrisisLevelLabel(level: string): {
  label: string;
  isEmergency: boolean;
  ariaLive: 'polite' | 'assertive';
} {
  const label = CRISIS_LEVEL_LABELS[level] || `Crisis level: ${level}`;
  const isEmergency = level === 'Imminent' || level === 'HighRisk';
  return { label, isEmergency, ariaLive: isEmergency ? 'assertive' : 'polite' };
}

/**
 * Format multiple scopes for screen reader
 */
export function formatScopesForScreenReader(scopes: string[]): string {
  if (scopes.length === 0) {
    return 'No access permissions granted';
  }
  const firstScope = scopes[0];
  if (scopes.length === 1 && firstScope) {
    return `One permission: ${getConsentScopeLabel(firstScope)}`;
  }
  const labels = scopes.map(getConsentScopeLabel);
  const lastLabel = labels.pop();
  return `${scopes.length} permissions: ${labels.join(', ')}, and ${lastLabel}`;
}

/**
 * Get reading level description
 */
export function getReadingLevelDescription(level: ReadingLevel): string {
  const descriptions: Record<ReadingLevel, string> = {
    elementary: 'Simple words and short sentences, suitable for children',
    intermediate: 'Clear explanations with some medical terms defined',
    standard: 'Standard health literacy level with common medical terms',
    advanced: 'Detailed explanations with technical terms',
    professional: 'Medical professional terminology',
  };
  return descriptions[level] || 'Standard reading level';
}
