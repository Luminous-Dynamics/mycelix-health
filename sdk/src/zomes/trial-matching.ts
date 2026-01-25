/**
 * Trial Matching Zome Client
 *
 * Client for AI-powered clinical trial matching and eligibility assessment.
 * Part of Phase 5 - Advanced Research.
 */

import type { AppClient, ActionHash, Timestamp } from '@holochain/client';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

// Types
export interface TrialEligibilityCriteria {
  criterion_id: string;
  description: string;
  inclusion: boolean;
  required: boolean;
  medical_codes?: string[];
}

export interface PatientProfile {
  profile_hash: ActionHash;
  patient_hash: ActionHash;
  age: number;
  conditions: string[];
  medications: string[];
  lab_values: Record<string, number>;
  preferences: TrialPreferences;
  updated_at: Timestamp;
}

export interface TrialPreferences {
  max_travel_distance_km?: number;
  preferred_locations?: string[];
  excluded_interventions?: string[];
  time_commitment_max_hours_per_week?: number;
}

export interface MatchResult {
  trial_hash: ActionHash;
  trial_id: string;
  trial_title: string;
  match_score: number;
  matched_criteria: string[];
  unmatched_criteria: string[];
  requires_review: boolean;
  distance_km?: number;
}

export interface TrialRecommendation {
  recommendation_hash: ActionHash;
  patient_hash: ActionHash;
  matches: MatchResult[];
  generated_at: Timestamp;
  expires_at: Timestamp;
}

// Input types
export interface CreatePatientProfileInput {
  patient_hash: ActionHash;
  age: number;
  conditions: string[];
  medications: string[];
  lab_values: Record<string, number>;
  preferences: TrialPreferences;
}

export interface FindMatchesInput {
  patient_hash: ActionHash;
  max_results?: number;
  min_score?: number;
}

/**
 * Trial Matching Zome Client
 */
export class TrialMatchingClient {
  private readonly roleName: string;
  private readonly zomeName = 'trial_matching';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  /**
   * Create or update a patient's matching profile
   */
  async createPatientProfile(input: CreatePatientProfileInput): Promise<ActionHash> {
    return this.call<ActionHash>('create_patient_profile', input);
  }

  /**
   * Get a patient's matching profile
   */
  async getPatientProfile(patientHash: ActionHash): Promise<PatientProfile | null> {
    return this.call<PatientProfile | null>('get_patient_profile', patientHash);
  }

  /**
   * Find matching trials for a patient
   */
  async findMatches(input: FindMatchesInput): Promise<MatchResult[]> {
    return this.call<MatchResult[]>('find_matches', input);
  }

  /**
   * Get trial recommendations for a patient
   */
  async getRecommendations(patientHash: ActionHash): Promise<TrialRecommendation | null> {
    return this.call<TrialRecommendation | null>('get_recommendations', patientHash);
  }

  /**
   * Refresh recommendations for a patient
   */
  async refreshRecommendations(patientHash: ActionHash): Promise<TrialRecommendation> {
    return this.call<TrialRecommendation>('refresh_recommendations', patientHash);
  }

  /**
   * Check eligibility for a specific trial
   */
  async checkEligibility(
    patientHash: ActionHash,
    trialHash: ActionHash
  ): Promise<MatchResult> {
    return this.call<MatchResult>('check_eligibility', {
      patient_hash: patientHash,
      trial_hash: trialHash,
    });
  }

  /**
   * Express interest in a trial
   */
  async expressInterest(
    patientHash: ActionHash,
    trialHash: ActionHash
  ): Promise<ActionHash> {
    return this.call<ActionHash>('express_interest', {
      patient_hash: patientHash,
      trial_hash: trialHash,
    });
  }

  /**
   * Get interested patients for a trial (for investigators)
   */
  async getInterestedPatients(trialHash: ActionHash): Promise<ActionHash[]> {
    return this.call<ActionHash[]>('get_interested_patients', trialHash);
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
      throw new HealthSdkError(
        HealthSdkErrorCode.ZOME_CALL_FAILED,
        `Trial Matching zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
