/**
 * @mycelix/health-sdk Type Definitions
 *
 * Strict TypeScript mirrors of Rust structs from mycelix-health zomes.
 * These types ensure client-side type safety before data hits the network.
 */

import type { ActionHash, AgentPubKey, Timestamp, EntryHash } from '@holochain/client';

// ============================================================================
// CORE HOLOCHAIN TYPES (Re-exports for convenience)
// ============================================================================

export type { ActionHash, AgentPubKey, Timestamp, EntryHash };

// ============================================================================
// PRIVACY BUDGET TYPES
// ============================================================================

/**
 * Budget composition method - mirrors Rust enum
 */
export type BudgetCompositionMethod =
  | { type: 'Basic' }
  | { type: 'Advanced'; delta_prime: number };

/**
 * Privacy budget ledger entry stored in DHT
 * Mirrors: BudgetLedgerEntry in commons/integrity/src/lib.rs
 */
export interface BudgetLedgerEntry {
  patient_hash: ActionHash;
  pool_hash: ActionHash;
  total_epsilon: number;
  consumed_epsilon: number;
  total_delta: number;
  consumed_delta: number;
  query_count: number;
  composition_method: BudgetCompositionMethod;
  created_at: Timestamp;
  last_updated: Timestamp;
}

/**
 * Client-side budget status (computed from ledger entry)
 */
export interface PrivacyBudgetStatus {
  /** Total epsilon allocated */
  total: number;
  /** Epsilon consumed so far */
  consumed: number;
  /** Remaining epsilon budget */
  remaining: number;
  /** Number of queries answered */
  queriesAnswered: number;
  /** Whether budget is exhausted */
  isExhausted: boolean;
  /** Percentage of budget remaining (0-100) */
  percentRemaining: number;
}

// ============================================================================
// DIFFERENTIAL PRIVACY TYPES
// ============================================================================

/**
 * Noise mechanism selection
 * Mirrors: NoiseMechanism in commons/coordinator/src/lib.rs
 */
export type NoiseMechanism =
  | 'Laplace'      // Pure ε-DP
  | 'Gaussian'     // (ε, δ)-DP
  | 'Exponential'  // For categorical data
  | 'RandomizedResponse'; // For binary data

/**
 * Query type for aggregate statistics
 */
export type QueryType =
  | 'Count'
  | 'Sum'
  | 'Average'
  | 'Median'
  | 'Histogram';

/**
 * Parameters for a differentially private query
 */
export interface DpQueryParams {
  /** Privacy cost (must be > 0) */
  epsilon: number;
  /** Privacy failure probability (for Gaussian mechanism) */
  delta?: number;
  /** Maximum change when one record changes */
  sensitivity_bound: number;
  /** Noise mechanism to use */
  noise_mechanism: NoiseMechanism;
}

/**
 * Request for an aggregate DP query
 */
export interface AggregateQueryRequest {
  /** Hash of the data pool to query */
  pool_hash: ActionHash;
  /** Type of aggregation */
  query_type: QueryType;
  /** DP parameters */
  params: DpQueryParams;
  /** Optional time range filter */
  time_range?: {
    start: Timestamp;
    end: Timestamp;
  };
  /** Optional data category filter */
  data_categories?: string[];
}

/**
 * Result of a DP query
 */
export interface DpQueryResult {
  /** The noisy result value */
  value: number;
  /** Epsilon consumed by this query */
  epsilon_consumed: number;
  /** Delta consumed (if Gaussian) */
  delta_consumed: number;
  /** Estimated standard error */
  standard_error?: number;
  /** 95% confidence interval */
  confidence_interval?: [number, number];
  /** Timestamp of query execution */
  executed_at: Timestamp;
}

// ============================================================================
// DATA POOL TYPES
// ============================================================================

/**
 * Governance model for a data pool
 */
export type GovernanceModel =
  | 'Democratic'
  | 'Cooperative'
  | 'Stewardship'
  | 'Hybrid';

/**
 * Data pool configuration
 * Mirrors: DataPool in commons/integrity/src/lib.rs
 */
export interface DataPool {
  /** Human-readable name */
  name: string;
  /** Description of pool purpose */
  description: string;
  /** Data categories accepted */
  data_categories: string[];
  /** Required consent level */
  required_consent_level: string;
  /** Default privacy parameters */
  default_epsilon: number;
  /** Per-user budget allocation */
  budget_per_user: number;
  /** How the pool is governed */
  governance_model: GovernanceModel;
  /** Minimum contributors before queries allowed */
  min_contributors: number;
  /** Maximum queries per time period */
  max_queries_per_period?: number;
  /** Whether pool is active */
  is_active: boolean;
  /** Creator of the pool */
  created_by: AgentPubKey;
  /** Creation timestamp */
  created_at: Timestamp;
}

/**
 * Data contribution to a pool
 */
export interface DataContribution {
  /** Hash of the pool */
  pool_hash: ActionHash;
  /** Hash of the contributor's identity */
  contributor: ActionHash;
  /** Encrypted data payload */
  encrypted_data: Uint8Array;
  /** Category of contributed data */
  data_category: string;
  /** Timestamp of contribution */
  contributed_at: Timestamp;
  /** Hash of consent record authorizing this */
  consent_hash: ActionHash;
  /** Budget consumed by this contribution */
  budget_consumed: number;
}

// ============================================================================
// PATIENT TYPES
// ============================================================================

/**
 * Contact information
 */
export interface ContactInfo {
  email?: string;
  phone?: string;
  address?: string;
  preferred_contact_method?: 'Email' | 'Phone' | 'Mail';
}

/**
 * Emergency contact
 */
export interface EmergencyContact {
  name: string;
  relationship: string;
  phone: string;
  email?: string;
}

/**
 * Patient record
 * Mirrors: Patient in patient/integrity/src/lib.rs
 */
export interface Patient {
  /** Legal first name */
  first_name: string;
  /** Legal last name */
  last_name: string;
  /** Date of birth (ISO 8601) */
  date_of_birth: string;
  /** Medical record number (if any) */
  mrn?: string;
  /** Contact information */
  contact: ContactInfo;
  /** Emergency contacts */
  emergency_contacts: EmergencyContact[];
  /** Known allergies */
  allergies: string[];
  /** Current medications */
  medications: string[];
  /** Insurance information */
  insurance_id?: string;
  /** Primary care provider */
  primary_provider?: AgentPubKey;
  /** Creation timestamp */
  created_at: Timestamp;
}

// ============================================================================
// CONSENT TYPES
// ============================================================================

/**
 * Consent scope levels
 */
export type ConsentScope =
  | 'FullAccess'
  | 'ReadOnly'
  | 'AggregateOnly'
  | 'EmergencyOnly'
  | 'ResearchOnly'
  | 'Custom';

/**
 * Consent record
 * Mirrors: Consent in consent/integrity/src/lib.rs
 */
export interface Consent {
  /** Patient granting consent */
  patient_hash: ActionHash;
  /** Entity receiving consent */
  grantee: AgentPubKey;
  /** Scope of consent */
  scope: ConsentScope;
  /** Specific data categories covered */
  data_categories: string[];
  /** Purpose of data use */
  purpose: string;
  /** When consent becomes active */
  valid_from: Timestamp;
  /** When consent expires */
  valid_until?: Timestamp;
  /** Whether consent is currently active */
  is_active: boolean;
  /** Creation timestamp */
  created_at: Timestamp;
}

/**
 * Consent check request
 */
export interface AuthorizationCheck {
  patient_hash: ActionHash;
  requester: AgentPubKey;
  action: string;
  data_categories: string[];
}

/**
 * Consent check result
 */
export interface AuthorizationResult {
  authorized: boolean;
  consent_hash?: ActionHash;
  reason?: string;
  expires_at?: Timestamp;
}

// ============================================================================
// CLINICAL TRIALS TYPES
// ============================================================================

/**
 * Trial phase
 */
export type TrialPhase = 'Phase1' | 'Phase2' | 'Phase3' | 'Phase4' | 'Observational';

/**
 * Trial status
 */
export type TrialStatus =
  | 'Recruiting'
  | 'Active'
  | 'Suspended'
  | 'Completed'
  | 'Terminated';

/**
 * Clinical trial
 * Mirrors: ClinicalTrial in trials/integrity/src/lib.rs
 */
export interface ClinicalTrial {
  /** NCT number or equivalent */
  trial_id: string;
  /** Trial title */
  title: string;
  /** Brief description */
  description: string;
  /** Principal investigator */
  principal_investigator: AgentPubKey;
  /** Sponsoring organization */
  sponsor: string;
  /** Current phase */
  phase: TrialPhase;
  /** Current status */
  status: TrialStatus;
  /** Eligibility criteria (structured) */
  eligibility_criteria: EligibilityCriteria;
  /** Target enrollment */
  target_enrollment: number;
  /** Current enrollment */
  current_enrollment: number;
  /** Start date */
  start_date: Timestamp;
  /** Expected end date */
  expected_end_date?: Timestamp;
  /** FDA IND number if applicable */
  ind_number?: string;
  /** IRB approval number */
  irb_approval?: string;
}

/**
 * Eligibility criteria for trial enrollment
 */
export interface EligibilityCriteria {
  min_age?: number;
  max_age?: number;
  gender?: 'Male' | 'Female' | 'All';
  conditions: string[];
  exclusions: string[];
  required_tests?: string[];
}

/**
 * Adverse event report
 */
export interface AdverseEvent {
  trial_hash: ActionHash;
  patient_hash: ActionHash;
  event_type: string;
  severity: 'Mild' | 'Moderate' | 'Severe' | 'LifeThreatening' | 'Fatal';
  description: string;
  onset_date: Timestamp;
  reported_by: AgentPubKey;
  reported_at: Timestamp;
  related_to_treatment?: boolean;
  outcome?: string;
}

// ============================================================================
// SDK CONFIGURATION TYPES
// ============================================================================

/**
 * SDK configuration options
 */
export interface MycelixHealthConfig {
  /** Holochain app ID */
  appId?: string;
  /** Role name in the hApp */
  roleName?: string;
  /** WebSocket URL for conductor */
  url?: string;
  /** Enable debug logging */
  debug?: boolean;
  /** Retry configuration */
  retry?: {
    maxAttempts: number;
    delayMs: number;
    backoffMultiplier: number;
  };
}

/**
 * Default configuration values
 */
export const DEFAULT_CONFIG: Required<MycelixHealthConfig> = {
  appId: 'mycelix-health',
  roleName: 'health',
  url: 'ws://localhost:8888',
  debug: false,
  retry: {
    maxAttempts: 3,
    delayMs: 1000,
    backoffMultiplier: 2,
  },
};

// ============================================================================
// ERROR TYPES
// ============================================================================

/**
 * SDK error codes
 */
export enum HealthSdkErrorCode {
  // Privacy errors
  BUDGET_EXHAUSTED = 'BUDGET_EXHAUSTED',
  INVALID_EPSILON = 'INVALID_EPSILON',
  INVALID_DELTA = 'INVALID_DELTA',
  INVALID_SENSITIVITY = 'INVALID_SENSITIVITY',

  // Authorization errors
  UNAUTHORIZED = 'UNAUTHORIZED',
  CONSENT_EXPIRED = 'CONSENT_EXPIRED',
  CONSENT_REVOKED = 'CONSENT_REVOKED',

  // Network errors
  CONNECTION_FAILED = 'CONNECTION_FAILED',
  ZOME_CALL_FAILED = 'ZOME_CALL_FAILED',
  TIMEOUT = 'TIMEOUT',

  // Validation errors
  INVALID_INPUT = 'INVALID_INPUT',
  VALIDATION_FAILED = 'VALIDATION_FAILED',

  // General errors
  UNKNOWN = 'UNKNOWN',
}

/**
 * SDK error with structured information
 */
export class HealthSdkError extends Error {
  constructor(
    public readonly code: HealthSdkErrorCode,
    message: string,
    public readonly details?: Record<string, unknown>
  ) {
    super(message);
    this.name = 'HealthSdkError';
  }

  toJSON() {
    return {
      name: this.name,
      code: this.code,
      message: this.message,
      details: this.details,
    };
  }
}
