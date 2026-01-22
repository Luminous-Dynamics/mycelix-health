/**
 * Clinical Trials Zome Client
 *
 * Client for clinical trial management in Mycelix-Health.
 * Handles trial registration, enrollment, and adverse event reporting.
 */

import type { AppClient, ActionHash, Timestamp } from '@holochain/client';
import type {
  ClinicalTrial,
  TrialPhase,
  TrialStatus,
  EligibilityCriteria,
  AdverseEvent,
} from '../types';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

/**
 * Input for creating a clinical trial
 */
export interface CreateTrialInput {
  trial_id: string;
  title: string;
  description: string;
  sponsor: string;
  phase: TrialPhase;
  eligibility_criteria: EligibilityCriteria;
  target_enrollment: number;
  start_date: Timestamp;
  expected_end_date?: Timestamp;
  ind_number?: string;
  irb_approval?: string;
}

/**
 * Input for reporting an adverse event
 */
export interface ReportAdverseEventInput {
  trial_hash: ActionHash;
  patient_hash: ActionHash;
  event_type: string;
  severity: 'Mild' | 'Moderate' | 'Severe' | 'LifeThreatening' | 'Fatal';
  description: string;
  onset_date: Timestamp;
  related_to_treatment?: boolean;
}

/**
 * Trial record with hash
 */
export interface TrialRecord {
  hash: ActionHash;
  trial: ClinicalTrial;
}

/**
 * Enrollment record
 */
export interface EnrollmentRecord {
  hash: ActionHash;
  trial_hash: ActionHash;
  patient_hash: ActionHash;
  enrolled_at: Timestamp;
  consent_hash: ActionHash;
  status: 'Active' | 'Completed' | 'Withdrawn' | 'Terminated';
}

/**
 * Eligibility check result
 */
export interface EligibilityResult {
  eligible: boolean;
  reasons: string[];
  unmetCriteria: string[];
}

/**
 * Clinical Trials Zome Client
 */
export class TrialsClient {
  private readonly roleName: string;
  private readonly zomeName = 'trials';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  // ============================================================================
  // TRIAL MANAGEMENT
  // ============================================================================

  /**
   * Create a new clinical trial
   *
   * @param input - Trial details
   * @returns Created trial record
   */
  async createTrial(input: CreateTrialInput): Promise<TrialRecord> {
    return this.call<TrialRecord>('create_trial', {
      ...input,
      status: 'Recruiting' as TrialStatus,
      current_enrollment: 0,
    });
  }

  /**
   * Get a trial by hash
   *
   * @param trialHash - Hash of the trial
   * @returns Trial record or null if not found
   */
  async getTrial(trialHash: ActionHash): Promise<ClinicalTrial | null> {
    return this.call<ClinicalTrial | null>('get_trial', trialHash);
  }

  /**
   * Get a trial by its external ID (e.g., NCT number)
   *
   * @param trialId - External trial identifier
   * @returns Trial record or null if not found
   */
  async getTrialByExternalId(trialId: string): Promise<TrialRecord | null> {
    return this.call<TrialRecord | null>('get_trial_by_id', trialId);
  }

  /**
   * Update trial status
   *
   * @param trialHash - Hash of the trial
   * @param status - New status
   * @returns Updated trial record
   */
  async updateTrialStatus(
    trialHash: ActionHash,
    status: TrialStatus
  ): Promise<TrialRecord> {
    return this.call<TrialRecord>('update_trial_status', {
      trial_hash: trialHash,
      status,
    });
  }

  /**
   * List all trials with a specific status
   *
   * @param status - Trial status to filter by
   * @returns Array of trial records
   */
  async listTrialsByStatus(status: TrialStatus): Promise<TrialRecord[]> {
    return this.call<TrialRecord[]>('list_trials_by_status', status);
  }

  /**
   * List recruiting trials
   *
   * Convenience method for finding trials accepting patients.
   *
   * @returns Array of recruiting trial records
   */
  async listRecruitingTrials(): Promise<TrialRecord[]> {
    return this.listTrialsByStatus('Recruiting');
  }

  /**
   * Search trials by condition
   *
   * @param condition - Medical condition to search for
   * @param limit - Maximum results
   * @returns Matching trial records
   */
  async searchTrialsByCondition(
    condition: string,
    limit: number = 50
  ): Promise<TrialRecord[]> {
    return this.call<TrialRecord[]>('search_trials_by_condition', {
      condition,
      limit,
    });
  }

  // ============================================================================
  // ENROLLMENT
  // ============================================================================

  /**
   * Check if a patient is eligible for a trial
   *
   * @param trialHash - Hash of the trial
   * @param patientHash - Hash of the patient
   * @returns Eligibility result with reasons
   */
  async checkEligibility(
    trialHash: ActionHash,
    patientHash: ActionHash
  ): Promise<EligibilityResult> {
    return this.call<EligibilityResult>('check_eligibility', {
      trial_hash: trialHash,
      patient_hash: patientHash,
    });
  }

  /**
   * Enroll a patient in a trial
   *
   * @param trialHash - Hash of the trial
   * @param patientHash - Hash of the patient
   * @param consentHash - Hash of the consent record authorizing enrollment
   * @returns Enrollment record
   */
  async enrollPatient(
    trialHash: ActionHash,
    patientHash: ActionHash,
    consentHash: ActionHash
  ): Promise<EnrollmentRecord> {
    // First check eligibility
    const eligibility = await this.checkEligibility(trialHash, patientHash);

    if (!eligibility.eligible) {
      throw new HealthSdkError(
        HealthSdkErrorCode.VALIDATION_FAILED,
        'Patient is not eligible for this trial',
        { reasons: eligibility.reasons, unmetCriteria: eligibility.unmetCriteria }
      );
    }

    return this.call<EnrollmentRecord>('enroll_patient', {
      trial_hash: trialHash,
      patient_hash: patientHash,
      consent_hash: consentHash,
    });
  }

  /**
   * Withdraw a patient from a trial
   *
   * @param enrollmentHash - Hash of the enrollment record
   * @param reason - Reason for withdrawal
   * @returns Updated enrollment record
   */
  async withdrawPatient(
    enrollmentHash: ActionHash,
    reason: string
  ): Promise<EnrollmentRecord> {
    return this.call<EnrollmentRecord>('withdraw_patient', {
      enrollment_hash: enrollmentHash,
      reason,
    });
  }

  /**
   * Get enrollment status for a patient in a trial
   *
   * @param trialHash - Hash of the trial
   * @param patientHash - Hash of the patient
   * @returns Enrollment record or null if not enrolled
   */
  async getEnrollmentStatus(
    trialHash: ActionHash,
    patientHash: ActionHash
  ): Promise<EnrollmentRecord | null> {
    return this.call<EnrollmentRecord | null>('get_enrollment', {
      trial_hash: trialHash,
      patient_hash: patientHash,
    });
  }

  /**
   * List all enrollments for a trial
   *
   * @param trialHash - Hash of the trial
   * @returns Array of enrollment records
   */
  async listTrialEnrollments(trialHash: ActionHash): Promise<EnrollmentRecord[]> {
    return this.call<EnrollmentRecord[]>('list_trial_enrollments', trialHash);
  }

  /**
   * List all trials a patient is enrolled in
   *
   * @param patientHash - Hash of the patient
   * @returns Array of enrollment records with trial info
   */
  async listPatientEnrollments(patientHash: ActionHash): Promise<EnrollmentRecord[]> {
    return this.call<EnrollmentRecord[]>('list_patient_enrollments', patientHash);
  }

  // ============================================================================
  // ADVERSE EVENTS
  // ============================================================================

  /**
   * Report an adverse event
   *
   * @param input - Adverse event details
   * @returns Adverse event record hash
   */
  async reportAdverseEvent(input: ReportAdverseEventInput): Promise<ActionHash> {
    return this.call<ActionHash>('report_adverse_event', {
      ...input,
      reported_at: Date.now() * 1000, // microseconds
    });
  }

  /**
   * Get adverse event by hash
   *
   * @param eventHash - Hash of the adverse event
   * @returns Adverse event or null if not found
   */
  async getAdverseEvent(eventHash: ActionHash): Promise<AdverseEvent | null> {
    return this.call<AdverseEvent | null>('get_adverse_event', eventHash);
  }

  /**
   * List adverse events for a trial
   *
   * @param trialHash - Hash of the trial
   * @returns Array of adverse events
   */
  async listTrialAdverseEvents(trialHash: ActionHash): Promise<AdverseEvent[]> {
    return this.call<AdverseEvent[]>('list_trial_adverse_events', trialHash);
  }

  /**
   * List adverse events by severity
   *
   * @param trialHash - Hash of the trial
   * @param severity - Minimum severity level
   * @returns Array of adverse events
   */
  async listAdverseEventsBySeverity(
    trialHash: ActionHash,
    severity: 'Mild' | 'Moderate' | 'Severe' | 'LifeThreatening' | 'Fatal'
  ): Promise<AdverseEvent[]> {
    return this.call<AdverseEvent[]>('list_adverse_events_by_severity', {
      trial_hash: trialHash,
      min_severity: severity,
    });
  }

  /**
   * Update adverse event outcome
   *
   * @param eventHash - Hash of the adverse event
   * @param outcome - Outcome description
   * @param relatedToTreatment - Whether related to treatment
   * @returns Updated event hash
   */
  async updateAdverseEventOutcome(
    eventHash: ActionHash,
    outcome: string,
    relatedToTreatment?: boolean
  ): Promise<ActionHash> {
    return this.call<ActionHash>('update_adverse_event_outcome', {
      event_hash: eventHash,
      outcome,
      related_to_treatment: relatedToTreatment,
    });
  }

  // ============================================================================
  // STATISTICS
  // ============================================================================

  /**
   * Get trial statistics
   *
   * @param trialHash - Hash of the trial
   * @returns Trial statistics
   */
  async getTrialStatistics(trialHash: ActionHash): Promise<{
    totalEnrolled: number;
    activeParticipants: number;
    withdrawnCount: number;
    completedCount: number;
    adverseEventCount: number;
    severeAdverseEventCount: number;
  }> {
    return this.call('get_trial_statistics', trialHash);
  }

  private async call<T>(fnName: string, payload: unknown): Promise<T> {
    try {
      const result = await this.client.callZome({
        role_name: this.roleName,
        zome_name: this.zomeName,
        fn_name: fnName,
        payload,
      });
      return result as T;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);

      if (message.includes('not eligible')) {
        throw new HealthSdkError(
          HealthSdkErrorCode.VALIDATION_FAILED,
          message,
          { fnName, payload }
        );
      }

      if (message.includes('unauthorized') || message.includes('consent')) {
        throw new HealthSdkError(
          HealthSdkErrorCode.UNAUTHORIZED,
          message,
          { fnName, payload }
        );
      }

      throw new HealthSdkError(
        HealthSdkErrorCode.ZOME_CALL_FAILED,
        `Trials zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
