/**
 * Chronic Care Zome Client
 *
 * Provides access to chronic disease management including diabetes,
 * heart failure, COPD, CKD, and other chronic conditions.
 */

import type { AppClient, ActionHash, AgentPubKey, Timestamp } from '@holochain/client';

// Chronic Care Types

export enum DiabetesType {
  Type1 = 'Type1',
  Type2 = 'Type2',
  Gestational = 'Gestational',
  LADA = 'LADA',
  MODY = 'MODY',
  Other = 'Other',
}

export enum NYHAClass {
  ClassI = 'ClassI',
  ClassII = 'ClassII',
  ClassIII = 'ClassIII',
  ClassIV = 'ClassIV',
}

export enum GOLDStage {
  Mild = 'Mild',
  Moderate = 'Moderate',
  Severe = 'Severe',
  VerySevere = 'VerySevere',
}

export enum CKDStage {
  Stage1 = 'Stage1',
  Stage2 = 'Stage2',
  Stage3a = 'Stage3a',
  Stage3b = 'Stage3b',
  Stage4 = 'Stage4',
  Stage5 = 'Stage5',
}

export enum AlertSeverity {
  Info = 'Info',
  Warning = 'Warning',
  Urgent = 'Urgent',
  Critical = 'Critical',
}

export type ChronicCondition =
  | { type: 'Diabetes'; data: DiabetesType }
  | { type: 'HeartFailure'; data: NYHAClass }
  | { type: 'COPD'; data: GOLDStage }
  | { type: 'ChronicKidneyDisease'; data: CKDStage }
  | { type: 'Hypertension' }
  | { type: 'Asthma' }
  | { type: 'CancerSurvivorship'; data: string }
  | { type: 'MultipleSclerosis' }
  | { type: 'RheumatoidArthritis' }
  | { type: 'Obesity' }
  | { type: 'Other'; data: string };

export interface ChronicDiseaseEnrollment {
  patientHash: ActionHash;
  condition: ChronicCondition;
  diagnosisDate: Timestamp;
  enrollmentDate: Timestamp;
  primaryProviderHash: ActionHash;
  careTeamHashes: ActionHash[];
  isActive: boolean;
  riskLevel: string;
  notes?: string;
  createdAt: Timestamp;
}

export interface CareGoal {
  goalId: string;
  description: string;
  targetValue?: string;
  currentValue?: string;
  targetDate?: Timestamp;
  status: string;
}

export interface ChronicCarePlan {
  enrollmentHash: ActionHash;
  patientHash: ActionHash;
  condition: ChronicCondition;
  goals: CareGoal[];
  medications: string[];
  selfManagementTasks: string[];
  monitoringSchedule: string;
  dietaryRecommendations?: string;
  exerciseRecommendations?: string;
  educationTopics: string[];
  nextReviewDate: Timestamp;
  createdBy: AgentPubKey;
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export interface PatientReportedOutcome {
  enrollmentHash: ActionHash;
  patientHash: ActionHash;
  measurementType: string;
  value: number;
  unit: string;
  recordedAt: Timestamp;
  notes?: string;
  deviceId?: string;
  createdAt: Timestamp;
}

export interface DiabetesMetrics {
  patientHash: ActionHash;
  measurementDate: Timestamp;
  fastingGlucose?: number;
  postprandialGlucose?: number;
  hba1c?: number;
  timeInRange?: number;
  hypoglycemicEvents: number;
  hyperglycemicEvents: number;
  insulinUnits?: number;
  carbsConsumed?: number;
  notes?: string;
  createdAt: Timestamp;
}

export interface HeartFailureMetrics {
  patientHash: ActionHash;
  measurementDate: Timestamp;
  weightKg: number;
  weightChangeKg?: number;
  bloodPressureSystolic: number;
  bloodPressureDiastolic: number;
  heartRate: number;
  oxygenSaturation?: number;
  edemaLevel?: number;
  dyspneaLevel?: number;
  orthopnea: boolean;
  pnd: boolean;
  notes?: string;
  createdAt: Timestamp;
}

export interface COPDMetrics {
  patientHash: ActionHash;
  measurementDate: Timestamp;
  peakFlow?: number;
  fev1?: number;
  oxygenSaturation?: number;
  respiratoryRate?: number;
  dyspneaScore?: number;
  coughSeverity?: number;
  sputumProduction?: number;
  rescueInhalerUses: number;
  exacerbation: boolean;
  notes?: string;
  createdAt: Timestamp;
}

export interface MedicationAdherence {
  patientHash: ActionHash;
  medicationName: string;
  rxnormCode?: string;
  scheduledDate: Timestamp;
  scheduledTime: string;
  taken: boolean;
  timeTaken?: Timestamp;
  doseSkippedReason?: string;
  sideEffects?: string[];
  notes?: string;
  createdAt: Timestamp;
}

export interface ChronicCareAlert {
  patientHash: ActionHash;
  enrollmentHash: ActionHash;
  alertType: string;
  severity: AlertSeverity;
  message: string;
  triggerValue?: string;
  threshold?: string;
  recommendedAction?: string;
  acknowledged: boolean;
  acknowledgedBy?: ActionHash;
  acknowledgedAt?: Timestamp;
  createdAt: Timestamp;
}

export interface ExacerbationEvent {
  enrollmentHash: ActionHash;
  patientHash: ActionHash;
  eventDate: Timestamp;
  condition: ChronicCondition;
  severity: AlertSeverity;
  symptoms: string[];
  triggerFactors: string[];
  interventionRequired: string;
  hospitalizationRequired: boolean;
  outcome?: string;
  notes?: string;
  createdAt: Timestamp;
}

export interface ChronicCareSummary {
  patientHash: ActionHash;
  activeEnrollments: number;
  conditions: string[];
  pendingAlerts: number;
}

// Input types

export interface CreateEnrollmentInput {
  patientHash: ActionHash;
  condition: ChronicCondition;
  diagnosisDate: Timestamp;
  primaryProviderHash: ActionHash;
  careTeamHashes?: ActionHash[];
  riskLevel?: string;
  notes?: string;
}

export interface CreateCarePlanInput {
  enrollmentHash: ActionHash;
  patientHash: ActionHash;
  condition: ChronicCondition;
  goals: CareGoal[];
  medications: string[];
  selfManagementTasks: string[];
  monitoringSchedule: string;
  dietaryRecommendations?: string;
  exerciseRecommendations?: string;
  educationTopics: string[];
  nextReviewDate: Timestamp;
}

export interface RecordOutcomeInput {
  enrollmentHash: ActionHash;
  patientHash: ActionHash;
  measurementType: string;
  value: number;
  unit: string;
  notes?: string;
  deviceId?: string;
}

export interface AdherenceRateInput {
  patientHash: ActionHash;
  medicationName?: string;
  startDate?: Timestamp;
  endDate?: Timestamp;
}

export interface AdherenceRateOutput {
  takenCount: number;
  totalCount: number;
  adherenceRate: number;
}

export interface AcknowledgeAlertInput {
  alertHash: ActionHash;
  acknowledgedBy: ActionHash;
}

/**
 * Chronic Care Zome Client
 */
export class ChronicCareClient {
  constructor(
    private client: AppClient,
    private roleName: string = 'health'
  ) {}

  private async call<T>(fnName: string, payload: unknown): Promise<T> {
    const result = await this.client.callZome({
      role_name: this.roleName,
      zome_name: 'chronic_care',
      fn_name: fnName,
      payload,
    });
    return result as T;
  }

  // Enrollment methods

  async enrollPatient(input: CreateEnrollmentInput): Promise<ActionHash> {
    return this.call('enroll_patient', input);
  }

  async getPatientEnrollments(patientHash: ActionHash): Promise<ChronicDiseaseEnrollment[]> {
    return this.call('get_patient_enrollments', patientHash);
  }

  // Care plan methods

  async createCarePlan(input: CreateCarePlanInput): Promise<ActionHash> {
    return this.call('create_care_plan', input);
  }

  async getCarePlans(enrollmentHash: ActionHash): Promise<ChronicCarePlan[]> {
    return this.call('get_care_plans', enrollmentHash);
  }

  async updateCarePlan(
    originalActionHash: ActionHash,
    updatedPlan: ChronicCarePlan
  ): Promise<ActionHash> {
    return this.call('update_care_plan', { originalActionHash, updatedPlan });
  }

  // Outcome recording methods

  async recordOutcome(input: RecordOutcomeInput): Promise<ActionHash> {
    return this.call('record_outcome', input);
  }

  async recordDiabetesMetrics(metrics: DiabetesMetrics): Promise<ActionHash> {
    return this.call('record_diabetes_metrics', metrics);
  }

  async recordHeartFailureMetrics(metrics: HeartFailureMetrics): Promise<ActionHash> {
    return this.call('record_heart_failure_metrics', metrics);
  }

  async recordCOPDMetrics(metrics: COPDMetrics): Promise<ActionHash> {
    return this.call('record_copd_metrics', metrics);
  }

  // Medication adherence methods

  async recordMedicationAdherence(adherence: MedicationAdherence): Promise<ActionHash> {
    return this.call('record_medication_adherence', adherence);
  }

  async getAdherenceRate(input: AdherenceRateInput): Promise<AdherenceRateOutput> {
    return this.call('get_adherence_rate', input);
  }

  // Alert methods

  async createAlert(alert: ChronicCareAlert): Promise<ActionHash> {
    return this.call('create_alert', alert);
  }

  async acknowledgeAlert(input: AcknowledgeAlertInput): Promise<ActionHash> {
    return this.call('acknowledge_alert', input);
  }

  async getPendingAlerts(enrollmentHash: ActionHash): Promise<ChronicCareAlert[]> {
    return this.call('get_pending_alerts', enrollmentHash);
  }

  // Exacerbation methods

  async recordExacerbation(event: ExacerbationEvent): Promise<ActionHash> {
    return this.call('record_exacerbation', event);
  }

  // Summary methods

  async getChronicCareSummary(patientHash: ActionHash): Promise<ChronicCareSummary> {
    return this.call('get_chronic_care_summary', patientHash);
  }
}
