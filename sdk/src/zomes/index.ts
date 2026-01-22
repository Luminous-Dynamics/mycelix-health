/**
 * Zome Clients Module
 *
 * Exports all zome-specific clients for direct usage.
 */

// Patient Management
export { PatientClient } from './patient';
export type {
  CreatePatientInput,
  PatientSearchCriteria,
  PatientRecord,
} from './patient';

// Consent Management
export { ConsentClient } from './consent';
export type {
  GrantConsentInput,
  ConsentRecord,
  ConsentSummary,
} from './consent';

// Health Commons (Data Pools & DP Queries)
export { CommonsClient } from './commons';
export type {
  CreatePoolInput,
  ContributeDataInput,
  QueryOptions,
} from './commons';

// Clinical Trials
export { TrialsClient } from './trials';
export type {
  CreateTrialInput,
  ReportAdverseEventInput,
  TrialRecord,
  EnrollmentRecord,
  EligibilityResult,
} from './trials';
