/**
 * SDOH (Social Determinants of Health) Zome Client
 *
 * Provides access to social determinants screening, community resources,
 * and intervention tracking functionality.
 */

import type { AppClient, ActionHash, AgentPubKey, Record as HolochainRecord, Timestamp } from '@holochain/client';

// SDOH Types

export enum ScreeningInstrument {
  PRAPARE = 'PRAPARE',
  AHCHRSN = 'AHCHRSN',
  WeCare = 'WeCare',
  Custom = 'Custom',
}

export enum SdohDomain {
  EconomicStability = 'EconomicStability',
  EducationAccess = 'EducationAccess',
  HealthcareAccess = 'HealthcareAccess',
  NeighborhoodEnvironment = 'NeighborhoodEnvironment',
  SocialCommunity = 'SocialCommunity',
}

export enum SdohCategory {
  Employment = 'Employment',
  FoodInsecurity = 'FoodInsecurity',
  HousingInstability = 'HousingInstability',
  Transportation = 'Transportation',
  Utilities = 'Utilities',
  Literacy = 'Literacy',
  LanguageBarrier = 'LanguageBarrier',
  EducationLevel = 'EducationLevel',
  HealthInsurance = 'HealthInsurance',
  ProviderAvailability = 'ProviderAvailability',
  HealthLiteracy = 'HealthLiteracy',
  SafetyConcerns = 'SafetyConcerns',
  EnvironmentalHazards = 'EnvironmentalHazards',
  AccessToHealthyFood = 'AccessToHealthyFood',
  SocialIsolation = 'SocialIsolation',
  InterpersonalViolence = 'InterpersonalViolence',
  IncarceratedFamilyMember = 'IncarceratedFamilyMember',
  RefugeeImmigrantStatus = 'RefugeeImmigrantStatus',
  Discrimination = 'Discrimination',
  Stress = 'Stress',
}

export enum RiskLevel {
  NoRisk = 'NoRisk',
  LowRisk = 'LowRisk',
  ModerateRisk = 'ModerateRisk',
  HighRisk = 'HighRisk',
  Urgent = 'Urgent',
}

export enum InterventionStatus {
  Identified = 'Identified',
  ReferralMade = 'ReferralMade',
  InProgress = 'InProgress',
  Completed = 'Completed',
  Declined = 'Declined',
  UnableToComplete = 'UnableToComplete',
}

export enum ResourceType {
  FoodPantry = 'FoodPantry',
  HousingAssistance = 'HousingAssistance',
  TransportationService = 'TransportationService',
  UtilityAssistance = 'UtilityAssistance',
  EmploymentServices = 'EmploymentServices',
  LegalAid = 'LegalAid',
  MentalHealthServices = 'MentalHealthServices',
  SubstanceAbuseServices = 'SubstanceAbuseServices',
  DomesticViolenceServices = 'DomesticViolenceServices',
  ChildcareServices = 'ChildcareServices',
  EducationProgram = 'EducationProgram',
  LanguageServices = 'LanguageServices',
  Other = 'Other',
}

export interface ScreeningResponse {
  questionId: string;
  questionText: string;
  response: string;
  responseCode?: string;
  category: SdohCategory;
  riskIndicated: boolean;
}

export interface SdohScreening {
  patientHash: ActionHash;
  screenerHash: AgentPubKey;
  instrument: ScreeningInstrument;
  screeningDate: Timestamp;
  responses: ScreeningResponse[];
  overallRiskLevel: RiskLevel;
  domainsAtRisk: SdohDomain[];
  categoriesAtRisk: SdohCategory[];
  notes?: string;
  consentObtained: boolean;
  createdAt: Timestamp;
}

export interface CommunityResource {
  name: string;
  resourceType: ResourceType;
  categoriesServed: SdohCategory[];
  description: string;
  address?: string;
  city: string;
  state: string;
  zipCode: string;
  phone?: string;
  website?: string;
  email?: string;
  hoursOfOperation?: string;
  eligibilityRequirements?: string;
  languagesAvailable: string[];
  acceptsUninsured: boolean;
  acceptsMedicaid: boolean;
  isActive: boolean;
  lastVerified: Timestamp;
  createdBy: AgentPubKey;
  createdAt: Timestamp;
}

export interface SdohIntervention {
  screeningHash: ActionHash;
  patientHash: ActionHash;
  category: SdohCategory;
  resourceHash?: ActionHash;
  resourceName: string;
  interventionType: string;
  status: InterventionStatus;
  referredBy: AgentPubKey;
  referredDate: Timestamp;
  followUpDate?: Timestamp;
  outcome?: string;
  barrierToCompletion?: string;
  notes?: string;
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export interface InterventionFollowUp {
  interventionHash: ActionHash;
  followUpDate: Timestamp;
  contactMethod: string;
  contactedBy: AgentPubKey;
  patientReached: boolean;
  currentStatus: InterventionStatus;
  patientFeedback?: string;
  needResolved: boolean;
  barriersIdentified: string[];
  nextSteps?: string;
  nextFollowUpDate?: Timestamp;
  createdAt: Timestamp;
}

export interface PatientSdohSummary {
  patientHash: ActionHash;
  latestScreeningDate?: Timestamp;
  overallRiskLevel?: RiskLevel;
  activeNeeds: SdohCategory[];
  pendingInterventions: number;
  completedInterventions: number;
  needsFollowUp: boolean;
}

// Input types

export interface CreateScreeningInput {
  patientHash: ActionHash;
  instrument: ScreeningInstrument;
  responses: ScreeningResponse[];
  notes?: string;
  consentObtained: boolean;
}

export interface CreateResourceInput {
  name: string;
  resourceType: ResourceType;
  categoriesServed: SdohCategory[];
  description: string;
  address?: string;
  city: string;
  state: string;
  zipCode: string;
  phone?: string;
  website?: string;
  email?: string;
  hoursOfOperation?: string;
  eligibilityRequirements?: string;
  languagesAvailable: string[];
  acceptsUninsured: boolean;
  acceptsMedicaid: boolean;
}

export interface ResourceSearchCriteria {
  zipCode?: string;
  category?: SdohCategory;
  acceptsUninsured?: boolean;
  acceptsMedicaid?: boolean;
  language?: string;
}

export interface CreateInterventionInput {
  screeningHash: ActionHash;
  patientHash: ActionHash;
  category: SdohCategory;
  resourceHash?: ActionHash;
  resourceName: string;
  interventionType: string;
  notes?: string;
}

export interface UpdateInterventionInput {
  interventionHash: ActionHash;
  status: InterventionStatus;
  outcome?: string;
  barrierToCompletion?: string;
  notes?: string;
}

export interface CreateFollowUpInput {
  interventionHash: ActionHash;
  contactMethod: string;
  patientReached: boolean;
  currentStatus: InterventionStatus;
  patientFeedback?: string;
  needResolved: boolean;
  barriersIdentified: string[];
  nextSteps?: string;
  nextFollowUpDate?: Timestamp;
}

/**
 * SDOH Zome Client
 */
export class SdohClient {
  constructor(
    private client: AppClient,
    private roleName: string = 'health'
  ) {}

  private async call<T>(fnName: string, payload: unknown): Promise<T> {
    const result = await this.client.callZome({
      role_name: this.roleName,
      zome_name: 'sdoh',
      fn_name: fnName,
      payload,
    });
    return result as T;
  }

  // Screening methods

  async createScreening(input: CreateScreeningInput): Promise<HolochainRecord> {
    return this.call('create_screening', input);
  }

  async getScreening(hash: ActionHash): Promise<HolochainRecord | null> {
    return this.call('get_screening', hash);
  }

  async getPatientScreenings(patientHash: ActionHash): Promise<HolochainRecord[]> {
    return this.call('get_patient_screenings', patientHash);
  }

  // Resource methods

  async createResource(input: CreateResourceInput): Promise<HolochainRecord> {
    return this.call('create_resource', input);
  }

  async getResource(hash: ActionHash): Promise<HolochainRecord | null> {
    return this.call('get_resource', hash);
  }

  async searchResources(criteria: ResourceSearchCriteria): Promise<HolochainRecord[]> {
    return this.call('search_resources', criteria);
  }

  // Intervention methods

  async createIntervention(input: CreateInterventionInput): Promise<HolochainRecord> {
    return this.call('create_intervention', input);
  }

  async getScreeningInterventions(screeningHash: ActionHash): Promise<HolochainRecord[]> {
    return this.call('get_screening_interventions', screeningHash);
  }

  async updateIntervention(input: UpdateInterventionInput): Promise<HolochainRecord> {
    return this.call('update_intervention', input);
  }

  // Follow-up methods

  async createFollowUp(input: CreateFollowUpInput): Promise<HolochainRecord> {
    return this.call('create_follow_up', input);
  }

  async getInterventionFollowUps(interventionHash: ActionHash): Promise<HolochainRecord[]> {
    return this.call('get_intervention_follow_ups', interventionHash);
  }

  // Summary methods

  async getPatientSdohSummary(patientHash: ActionHash): Promise<PatientSdohSummary> {
    return this.call('get_patient_sdoh_summary', patientHash);
  }
}
