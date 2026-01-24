/**
 * Clinical Decision Support (CDS) Zome Client
 *
 * Client for drug interaction checking, clinical alerts, and
 * evidence-based clinical guidelines in Mycelix-Health.
 */

import type { AppClient, ActionHash, AgentPubKey, Timestamp } from '@holochain/client';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

// Interaction Severity
export type InteractionSeverity = 'Contraindicated' | 'Major' | 'Moderate' | 'Minor' | 'Unknown';

// Alert Type
export type AlertType =
  | 'DrugInteraction'
  | 'AllergyAlert'
  | 'DoseWarning'
  | 'LabResult'
  | 'Preventive'
  | 'Diagnostic'
  | 'Custom';

// Alert Priority
export type AlertPriority = 'Critical' | 'High' | 'Medium' | 'Low' | 'Info';

// Guideline Category
export type GuidelineCategory =
  | 'Screening'
  | 'Prevention'
  | 'Diagnosis'
  | 'Treatment'
  | 'Monitoring'
  | 'Referral';

// Evidence Level
export type EvidenceLevel = 'A' | 'B' | 'C' | 'D' | 'Expert';

// Drug Interaction
export interface DrugInteraction {
  drug_a_rxnorm: string;
  drug_a_name: string;
  drug_b_rxnorm: string;
  drug_b_name: string;
  severity: InteractionSeverity;
  description: string;
  mechanism?: string;
  management: string;
  references: string[];
}

// Drug-Allergy Interaction
export interface DrugAllergyInteraction {
  drug_rxnorm: string;
  drug_name: string;
  allergen: string;
  cross_reactivity_risk: InteractionSeverity;
  description: string;
  alternatives: string[];
}

// Clinical Alert
export interface ClinicalAlert {
  hash?: ActionHash;
  patient_hash: ActionHash;
  alert_type: AlertType;
  priority: AlertPriority;
  title: string;
  message: string;
  source_reference?: string;
  acknowledged: boolean;
  acknowledged_by?: AgentPubKey;
  acknowledged_at?: Timestamp;
  created_at: Timestamp;
  expires_at?: Timestamp;
  action_required: boolean;
  action_taken?: string;
}

// Clinical Guideline
export interface ClinicalGuideline {
  hash?: ActionHash;
  guideline_id: string;
  title: string;
  category: GuidelineCategory;
  condition_codes: string[];
  description: string;
  recommendations: GuidelineRecommendation[];
  source: string;
  source_url?: string;
  evidence_level: EvidenceLevel;
  last_reviewed: Timestamp;
  version: string;
}

// Guideline Recommendation
export interface GuidelineRecommendation {
  text: string;
  strength: 'Strong' | 'Moderate' | 'Weak' | 'Optional';
  evidence_level: EvidenceLevel;
}

// Patient Guideline Status
export interface PatientGuidelineStatus {
  patient_hash: ActionHash;
  guideline_hash: ActionHash;
  status: 'Due' | 'Completed' | 'Overdue' | 'NotApplicable' | 'Deferred';
  last_completed?: Timestamp;
  next_due?: Timestamp;
  notes?: string;
}

// Interaction Check Response
export interface InteractionCheckResponse {
  checked_at: Timestamp;
  drug_interactions: DrugInteraction[];
  allergy_interactions: DrugAllergyInteraction[];
  has_contraindications: boolean;
  has_major_interactions: boolean;
  summary: string;
}

// Input types
export interface CreateAlertInput {
  patient_hash: ActionHash;
  alert_type: AlertType;
  priority: AlertPriority;
  title: string;
  message: string;
  source_reference?: string;
  action_required?: boolean;
  expires_at?: Timestamp;
}

export interface CheckInteractionsInput {
  patient_hash: ActionHash;
  rxnorm_codes: string[];
  allergies: string[];
}

/**
 * Clinical Decision Support Zome Client
 */
export class CdsClient {
  private readonly roleName: string;
  private readonly zomeName = 'cds';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  /**
   * Check for drug-drug interactions
   */
  async checkDrugInteractions(rxnormCodes: string[]): Promise<DrugInteraction[]> {
    return this.call<DrugInteraction[]>('check_drug_interactions', { medications: rxnormCodes });
  }

  /**
   * Check for drug-allergy conflicts
   */
  async checkAllergyConflicts(rxnormCodes: string[], allergies: string[]): Promise<DrugAllergyInteraction[]> {
    return this.call<DrugAllergyInteraction[]>('check_allergy_conflicts', {
      medications: rxnormCodes,
      allergies,
    });
  }

  /**
   * Perform full interaction check for a patient
   */
  async performInteractionCheck(input: CheckInteractionsInput): Promise<InteractionCheckResponse> {
    return this.call<InteractionCheckResponse>('perform_interaction_check', input);
  }

  /**
   * Create a clinical alert
   */
  async createAlert(input: CreateAlertInput): Promise<ActionHash> {
    return this.call<ActionHash>('create_clinical_alert', {
      ...input,
      action_required: input.action_required ?? false,
    });
  }

  /**
   * Get alerts for a patient
   */
  async getPatientAlerts(patientHash: ActionHash, includeAcknowledged = false): Promise<ClinicalAlert[]> {
    return this.call<ClinicalAlert[]>('get_patient_alerts', {
      patient_hash: patientHash,
      include_acknowledged: includeAcknowledged,
    });
  }

  /**
   * Acknowledge an alert
   */
  async acknowledgeAlert(alertHash: ActionHash, actionTaken?: string): Promise<void> {
    await this.call<void>('acknowledge_alert', {
      alert_hash: alertHash,
      action_taken: actionTaken,
    });
  }

  /**
   * Get unacknowledged alert count for a patient
   */
  async getUnacknowledgedAlertCount(patientHash: ActionHash): Promise<number> {
    return this.call<number>('get_unacknowledged_alert_count', patientHash);
  }

  /**
   * Register a drug interaction (for CDS administrators)
   */
  async registerDrugInteraction(interaction: Omit<DrugInteraction, 'hash'>): Promise<ActionHash> {
    return this.call<ActionHash>('register_drug_interaction', interaction);
  }

  /**
   * Register a drug-allergy interaction
   */
  async registerAllergyInteraction(interaction: Omit<DrugAllergyInteraction, 'hash'>): Promise<ActionHash> {
    return this.call<ActionHash>('register_allergy_interaction', interaction);
  }

  /**
   * Create a clinical guideline
   */
  async createGuideline(guideline: Omit<ClinicalGuideline, 'hash'>): Promise<ActionHash> {
    return this.call<ActionHash>('create_clinical_guideline', guideline);
  }

  /**
   * Get guidelines for a condition
   */
  async getGuidelinesForCondition(conditionCode: string): Promise<ClinicalGuideline[]> {
    return this.call<ClinicalGuideline[]>('get_guidelines_for_condition', conditionCode);
  }

  /**
   * Get applicable guidelines for a patient
   */
  async getPatientGuidelines(patientHash: ActionHash): Promise<PatientGuidelineStatus[]> {
    return this.call<PatientGuidelineStatus[]>('get_patient_guidelines', patientHash);
  }

  /**
   * Update patient guideline status
   */
  async updateGuidelineStatus(
    patientHash: ActionHash,
    guidelineHash: ActionHash,
    status: PatientGuidelineStatus['status'],
    notes?: string
  ): Promise<void> {
    await this.call<void>('update_guideline_status', {
      patient_hash: patientHash,
      guideline_hash: guidelineHash,
      status,
      notes,
    });
  }

  /**
   * Get high priority alerts requiring action
   */
  async getCriticalAlerts(patientHash: ActionHash): Promise<ClinicalAlert[]> {
    const alerts = await this.getPatientAlerts(patientHash, false);
    return alerts.filter(
      a => (a.priority === 'Critical' || a.priority === 'High') && a.action_required
    );
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
        `CDS zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
