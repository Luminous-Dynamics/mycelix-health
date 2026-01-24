/**
 * Mental Health Zome Client
 *
 * Provides access to behavioral health screening, treatment planning,
 * crisis management, and 42 CFR Part 2 compliant substance abuse records.
 */

import type { AppClient, ActionHash, AgentPubKey, Record as HolochainRecord, Timestamp } from '@holochain/client';

// Mental Health Types

export enum MentalHealthInstrument {
  PHQ9 = 'PHQ9',
  PHQ2 = 'PHQ2',
  GAD7 = 'GAD7',
  CSSRS = 'CSSRS',
  CAGE = 'CAGE',
  AUDIT = 'AUDIT',
  DAST10 = 'DAST10',
  PCL5 = 'PCL5',
  MDQ = 'MDQ',
  EPDS = 'EPDS',
  PSC17 = 'PSC17',
  Custom = 'Custom',
}

export enum Severity {
  None = 'None',
  Minimal = 'Minimal',
  Mild = 'Mild',
  Moderate = 'Moderate',
  ModeratelySevere = 'ModeratelySevere',
  Severe = 'Severe',
}

export enum CrisisLevel {
  None = 'None',
  LowRisk = 'LowRisk',
  ModerateRisk = 'ModerateRisk',
  HighRisk = 'HighRisk',
  Imminent = 'Imminent',
}

export enum TreatmentModality {
  IndividualTherapy = 'IndividualTherapy',
  GroupTherapy = 'GroupTherapy',
  FamilyTherapy = 'FamilyTherapy',
  Medication = 'Medication',
  IntensiveOutpatient = 'IntensiveOutpatient',
  PartialHospitalization = 'PartialHospitalization',
  Inpatient = 'Inpatient',
  CrisisIntervention = 'CrisisIntervention',
  PeerSupport = 'PeerSupport',
  Telehealth = 'Telehealth',
  Other = 'Other',
}

export enum SafetyPlanStatus {
  Active = 'Active',
  NeedsUpdate = 'NeedsUpdate',
  Expired = 'Expired',
  NotApplicable = 'NotApplicable',
}

export enum SubstanceCategory {
  Alcohol = 'Alcohol',
  Cannabis = 'Cannabis',
  Opioids = 'Opioids',
  Stimulants = 'Stimulants',
  Sedatives = 'Sedatives',
  Hallucinogens = 'Hallucinogens',
  Tobacco = 'Tobacco',
  Other = 'Other',
}

export enum Part2ConsentType {
  GeneralDisclosure = 'GeneralDisclosure',
  RedisclosureProhibited = 'RedisclosureProhibited',
  MedicalEmergency = 'MedicalEmergency',
  Research = 'Research',
  CourtOrder = 'CourtOrder',
  AuditEvaluation = 'AuditEvaluation',
}

export interface MentalHealthScreening {
  patientHash: ActionHash;
  providerHash: AgentPubKey;
  instrument: MentalHealthInstrument;
  screeningDate: Timestamp;
  rawScore: number;
  severity: Severity;
  responses: Array<[string, number]>;
  interpretation: string;
  followUpRecommended: boolean;
  crisisIndicatorsPresent: boolean;
  notes?: string;
  createdAt: Timestamp;
}

export interface MoodEntry {
  patientHash: ActionHash;
  entryDate: Timestamp;
  moodScore: number;
  anxietyScore: number;
  sleepQuality: number;
  sleepHours?: number;
  energyLevel: number;
  appetite?: string;
  medicationsTaken: boolean;
  activities: string[];
  triggers: string[];
  copingStrategiesUsed: string[];
  notes?: string;
  createdAt: Timestamp;
}

export interface TreatmentGoal {
  goalId: string;
  description: string;
  targetDate?: Timestamp;
  progress: string;
  interventions: string[];
}

export interface PsychMedication {
  name: string;
  rxnormCode?: string;
  dosage: string;
  frequency: string;
  prescriberHash: ActionHash;
  startDate: Timestamp;
  targetSymptoms: string[];
  sideEffectsReported: string[];
}

export interface MentalHealthTreatmentPlan {
  patientHash: ActionHash;
  providerHash: AgentPubKey;
  primaryDiagnosisIcd10: string;
  secondaryDiagnoses: string[];
  treatmentGoals: TreatmentGoal[];
  modalities: TreatmentModality[];
  medications: PsychMedication[];
  sessionFrequency: string;
  estimatedDuration?: string;
  crisisPlanHash?: ActionHash;
  effectiveDate: Timestamp;
  reviewDate: Timestamp;
  status: string;
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export interface ContactInfo {
  name: string;
  relationship?: string;
  phone: string;
  availableHours?: string;
}

export interface SafetyPlan {
  patientHash: ActionHash;
  providerHash: AgentPubKey;
  warningSigns: string[];
  internalCopingStrategies: string[];
  peopleForDistraction: ContactInfo[];
  peopleForHelp: ContactInfo[];
  professionalsToContact: ContactInfo[];
  crisisLine988: boolean;
  additionalCrisisResources: string[];
  environmentSafetySteps: string[];
  reasonsForLiving: string[];
  status: SafetyPlanStatus;
  createdAt: Timestamp;
  lastReviewed: Timestamp;
  nextReviewDate: Timestamp;
}

export interface CrisisEvent {
  patientHash: ActionHash;
  reporterHash: AgentPubKey;
  eventDate: Timestamp;
  crisisLevel: CrisisLevel;
  suicidalIdeation: boolean;
  homicidalIdeation: boolean;
  selfHarm: boolean;
  substanceIntoxication: boolean;
  psychoticSymptoms: boolean;
  description: string;
  interventionTaken: string;
  disposition: string;
  followUpPlan: string;
  safetyPlanReviewed: boolean;
  createdAt: Timestamp;
}

export interface Part2Consent {
  patientHash: ActionHash;
  consentType: Part2ConsentType;
  disclosingProgram: string;
  recipientName: string;
  recipientHash?: ActionHash;
  purpose: string;
  informationToDisclose: string[];
  substancesCovered: SubstanceCategory[];
  effectiveDate: Timestamp;
  expirationDate?: Timestamp;
  rightToRevokeExplained: boolean;
  patientSignatureDate: Timestamp;
  witnessName?: string;
  isRevoked: boolean;
  revocationDate?: Timestamp;
  createdAt: Timestamp;
}

export interface TherapyNote {
  patientHash: ActionHash;
  providerHash: AgentPubKey;
  sessionDate: Timestamp;
  sessionType: TreatmentModality;
  durationMinutes: number;
  presentingConcerns: string;
  mentalStatus?: string;
  interventionsUsed: string[];
  patientResponse: string;
  riskAssessment?: CrisisLevel;
  planForNextSession: string;
  isPsychotherapyNote: boolean;
  createdAt: Timestamp;
}

// Input types

export interface CreateScreeningInput {
  patientHash: ActionHash;
  instrument: MentalHealthInstrument;
  responses: Array<[string, number]>;
  notes?: string;
}

export interface CreateMoodEntryInput {
  patientHash: ActionHash;
  moodScore: number;
  anxietyScore: number;
  sleepQuality: number;
  sleepHours?: number;
  energyLevel: number;
  appetite?: string;
  medicationsTaken: boolean;
  activities: string[];
  triggers: string[];
  copingStrategiesUsed: string[];
  notes?: string;
}

export interface CreateSafetyPlanInput {
  patientHash: ActionHash;
  warningSigns: string[];
  internalCopingStrategies: string[];
  peopleForDistraction: ContactInfo[];
  peopleForHelp: ContactInfo[];
  professionalsToContact: ContactInfo[];
  crisisLine988: boolean;
  additionalCrisisResources: string[];
  environmentSafetySteps: string[];
  reasonsForLiving: string[];
}

export interface CreateCrisisEventInput {
  patientHash: ActionHash;
  crisisLevel: CrisisLevel;
  suicidalIdeation: boolean;
  homicidalIdeation: boolean;
  selfHarm: boolean;
  substanceIntoxication: boolean;
  psychoticSymptoms: boolean;
  description: string;
  interventionTaken: string;
  disposition: string;
  followUpPlan: string;
  safetyPlanReviewed: boolean;
}

export interface CreatePart2ConsentInput {
  patientHash: ActionHash;
  consentType: Part2ConsentType;
  disclosingProgram: string;
  recipientName: string;
  recipientHash?: ActionHash;
  purpose: string;
  informationToDisclose: string[];
  substancesCovered: SubstanceCategory[];
  expirationDate?: Timestamp;
}

/**
 * Mental Health Zome Client
 */
export class MentalHealthClient {
  constructor(
    private client: AppClient,
    private roleName: string = 'mycelix-health'
  ) {}

  private async call<T>(fnName: string, payload: unknown): Promise<T> {
    const result = await this.client.callZome({
      role_name: this.roleName,
      zome_name: 'mental_health',
      fn_name: fnName,
      payload,
    });
    return result as T;
  }

  // Screening methods

  async createScreening(input: CreateScreeningInput): Promise<HolochainRecord> {
    return this.call('create_screening', input);
  }

  async getPatientScreenings(patientHash: ActionHash): Promise<HolochainRecord[]> {
    return this.call('get_patient_screenings', patientHash);
  }

  // Mood tracking methods

  async createMoodEntry(input: CreateMoodEntryInput): Promise<HolochainRecord> {
    return this.call('create_mood_entry', input);
  }

  async getPatientMoodEntries(patientHash: ActionHash): Promise<HolochainRecord[]> {
    return this.call('get_patient_mood_entries', patientHash);
  }

  // Safety plan methods

  async createSafetyPlan(input: CreateSafetyPlanInput): Promise<HolochainRecord> {
    return this.call('create_safety_plan', input);
  }

  async getPatientSafetyPlan(patientHash: ActionHash): Promise<HolochainRecord | null> {
    return this.call('get_patient_safety_plan', patientHash);
  }

  async updateSafetyPlan(
    originalHash: ActionHash,
    updatedPlan: CreateSafetyPlanInput
  ): Promise<HolochainRecord> {
    return this.call('update_safety_plan', { originalHash, updatedPlan });
  }

  // Crisis management methods

  async createCrisisEvent(input: CreateCrisisEventInput): Promise<HolochainRecord> {
    return this.call('create_crisis_event', input);
  }

  async getPatientCrisisEvents(patientHash: ActionHash): Promise<HolochainRecord[]> {
    return this.call('get_patient_crisis_events', patientHash);
  }

  // 42 CFR Part 2 consent methods

  async createPart2Consent(input: CreatePart2ConsentInput): Promise<HolochainRecord> {
    return this.call('create_part2_consent', input);
  }

  async getPatientPart2Consents(patientHash: ActionHash): Promise<HolochainRecord[]> {
    return this.call('get_patient_part2_consents', patientHash);
  }

  async revokePart2Consent(consentHash: ActionHash): Promise<HolochainRecord> {
    return this.call('revoke_part2_consent', consentHash);
  }

  // Therapy notes methods

  async createTherapyNote(note: TherapyNote): Promise<HolochainRecord> {
    return this.call('create_therapy_note', note);
  }

  async getPatientTherapyNotes(patientHash: ActionHash): Promise<HolochainRecord[]> {
    return this.call('get_patient_therapy_notes', patientHash);
  }
}
