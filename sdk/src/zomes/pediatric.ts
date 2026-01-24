/**
 * Pediatric Zome Client
 *
 * Provides access to pediatric care including growth tracking,
 * immunizations, developmental milestones, and well-child visits.
 */

import type { AppClient, ActionHash, AgentPubKey, Timestamp } from '@holochain/client';

// Pediatric Types

export enum VaccineType {
  HepB = 'HepB',
  RV = 'RV',
  DTaP = 'DTaP',
  Hib = 'Hib',
  PCV13 = 'PCV13',
  IPV = 'IPV',
  Influenza = 'Influenza',
  MMR = 'MMR',
  Varicella = 'Varicella',
  HepA = 'HepA',
  MenACWY = 'MenACWY',
  Tdap = 'Tdap',
  HPV = 'HPV',
  MenB = 'MenB',
  COVID19 = 'COVID19',
  Other = 'Other',
}

export enum ImmunizationStatus {
  Completed = 'Completed',
  InProgress = 'InProgress',
  Overdue = 'Overdue',
  NotStarted = 'NotStarted',
  Contraindicated = 'Contraindicated',
  Declined = 'Declined',
}

export enum DevelopmentalDomain {
  GrossMotor = 'GrossMotor',
  FineMotor = 'FineMotor',
  Language = 'Language',
  Cognitive = 'Cognitive',
  SocialEmotional = 'SocialEmotional',
  SelfHelp = 'SelfHelp',
}

export enum MilestoneStatus {
  NotYetExpected = 'NotYetExpected',
  Expected = 'Expected',
  Achieved = 'Achieved',
  Delayed = 'Delayed',
  AtRisk = 'AtRisk',
  Concerning = 'Concerning',
}

export enum FeedingType {
  Breastfeeding = 'Breastfeeding',
  FormulaFeeding = 'FormulaFeeding',
  Mixed = 'Mixed',
  Solids = 'Solids',
  TableFood = 'TableFood',
}

export interface GrowthMeasurement {
  patientHash: ActionHash;
  measurementDate: Timestamp;
  ageMonths: number;
  weightKg: number;
  heightCm: number;
  headCircumferenceCm?: number;
  bmi?: number;
  weightPercentile?: number;
  heightPercentile?: number;
  bmiPercentile?: number;
  headPercentile?: number;
  measuredBy: AgentPubKey;
  notes?: string;
  createdAt: Timestamp;
}

export interface ImmunizationRecord {
  patientHash: ActionHash;
  vaccineType: VaccineType;
  vaccineName: string;
  cvxCode?: string;
  mvxCode?: string;
  lotNumber: string;
  expirationDate: Timestamp;
  administrationDate: Timestamp;
  doseNumber: number;
  dosesInSeries: number;
  site?: string;
  route?: string;
  administeredBy: AgentPubKey;
  administeredAt: string;
  visGiven: boolean;
  visDate?: Timestamp;
  reaction?: string;
  notes?: string;
  createdAt: Timestamp;
}

export interface DevelopmentalMilestone {
  patientHash: ActionHash;
  assessmentDate: Timestamp;
  ageMonths: number;
  domain: DevelopmentalDomain;
  milestoneName: string;
  expectedAgeMonths: number;
  status: MilestoneStatus;
  assessedBy: AgentPubKey;
  notes?: string;
  referralMade: boolean;
  referralDetails?: string;
  createdAt: Timestamp;
}

export interface WellChildVisit {
  patientHash: ActionHash;
  visitDate: Timestamp;
  visitType: string;
  ageAtVisit: string;
  providerHash: AgentPubKey;
  growthMeasurementHash?: ActionHash;
  immunizationsGiven: ActionHash[];
  developmentalScreening: boolean;
  screeningToolUsed?: string;
  concerns: string[];
  anticipatoryGuidanceTopics: string[];
  nextVisitRecommended?: Timestamp;
  referralsMade: string[];
  notes?: string;
  createdAt: Timestamp;
}

export interface PediatricCondition {
  patientHash: ActionHash;
  conditionName: string;
  icd10Code: string;
  onsetDate: Timestamp;
  diagnosedBy: AgentPubKey;
  severity?: string;
  isActive: boolean;
  treatmentPlan?: string;
  specialistHash?: ActionHash;
  notes?: string;
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export interface SchoolHealthRecord {
  patientHash: ActionHash;
  schoolName: string;
  gradeLevel: string;
  schoolYear: string;
  physicalExamDate: Timestamp;
  physicalExamHash?: ActionHash;
  immunizationsComplete: boolean;
  healthConditionsDisclosed: string[];
  medicationsAtSchool: string[];
  accommodationsNeeded: string[];
  emergencyContactsUpdated: boolean;
  actionPlanHashes: ActionHash[];
  createdAt: Timestamp;
  updatedAt: Timestamp;
}

export interface AdolescentHealth {
  patientHash: ActionHash;
  assessmentDate: Timestamp;
  providerHash: AgentPubKey;
  headsssCompleted: boolean;
  homeEnvironment?: string;
  educationEmployment?: string;
  activities?: string;
  drugsAlcohol?: string;
  sexuality?: string;
  suicideDepression?: string;
  safety?: string;
  confidentialityDiscussed: boolean;
  riskFactorsIdentified: string[];
  protectiveFactorsIdentified: string[];
  interventionsRecommended: string[];
  notes?: string;
  createdAt: Timestamp;
}

export interface NewbornRecord {
  patientHash: ActionHash;
  birthDate: Timestamp;
  birthWeight: number;
  birthLength: number;
  headCircumference: number;
  apgar1Min: number;
  apgar5Min: number;
  gestationalAge: number;
  deliveryType: string;
  birthComplications: string[];
  newbornScreeningDate?: Timestamp;
  newbornScreeningResults?: string;
  hearingScreeningDate?: Timestamp;
  hearingScreeningResult?: string;
  vitaminKGiven: boolean;
  eyeProphylaxisGiven: boolean;
  hepBVaccineGiven: boolean;
  feedingType: FeedingType;
  motherHash?: ActionHash;
  notes?: string;
  createdAt: Timestamp;
}

// Input types

export interface RecordGrowthInput {
  patientHash: ActionHash;
  ageMonths: number;
  weightKg: number;
  heightCm: number;
  headCircumferenceCm?: number;
  notes?: string;
}

export interface RecordImmunizationInput {
  patientHash: ActionHash;
  vaccineType: VaccineType;
  vaccineName: string;
  cvxCode?: string;
  mvxCode?: string;
  lotNumber: string;
  expirationDate: Timestamp;
  doseNumber: number;
  dosesInSeries: number;
  site?: string;
  route?: string;
  administeredAt: string;
  visGiven: boolean;
  visDate?: Timestamp;
  notes?: string;
}

export interface RecordMilestoneInput {
  patientHash: ActionHash;
  ageMonths: number;
  domain: DevelopmentalDomain;
  milestoneName: string;
  expectedAgeMonths: number;
  status: MilestoneStatus;
  notes?: string;
  referralMade: boolean;
  referralDetails?: string;
}

export interface RecordWellChildVisitInput {
  patientHash: ActionHash;
  visitType: string;
  ageAtVisit: string;
  growthMeasurementHash?: ActionHash;
  immunizationsGiven: ActionHash[];
  developmentalScreening: boolean;
  screeningToolUsed?: string;
  concerns: string[];
  anticipatoryGuidanceTopics: string[];
  nextVisitRecommended?: Timestamp;
  referralsMade: string[];
  notes?: string;
}

export interface CalculatePercentilesInput {
  weightKg: number;
  heightCm: number;
  headCircumferenceCm?: number;
  ageMonths: number;
  sex: string;
}

export interface GrowthPercentiles {
  weightPercentile: number;
  heightPercentile: number;
  bmi: number;
  bmiPercentile: number;
  headPercentile?: number;
}

export interface ImmunizationStatusInput {
  patientHash: ActionHash;
  ageMonths: number;
}

export interface ImmunizationStatusOutput {
  patientHash: ActionHash;
  ageMonths: number;
  upToDate: boolean;
  missingVaccines: VaccineType[];
  dueSoonVaccines: VaccineType[];
  completedVaccines: VaccineType[];
  nextDueDate?: Timestamp;
}

export interface DevelopmentalSummary {
  patientHash: ActionHash;
  ageMonths: number;
  overallStatus: string;
  domainStatuses: Record<DevelopmentalDomain, MilestoneStatus>;
  concerningAreas: DevelopmentalDomain[];
  referralsNeeded: boolean;
}

/**
 * Pediatric Zome Client
 */
export class PediatricClient {
  constructor(
    private client: AppClient,
    private roleName: string = 'mycelix-health'
  ) {}

  private async call<T>(fnName: string, payload: unknown): Promise<T> {
    const result = await this.client.callZome({
      role_name: this.roleName,
      zome_name: 'pediatric',
      fn_name: fnName,
      payload,
    });
    return result as T;
  }

  // Growth tracking methods

  async recordGrowth(measurement: GrowthMeasurement): Promise<ActionHash> {
    return this.call('record_growth', measurement);
  }

  async getGrowthHistory(patientHash: ActionHash): Promise<GrowthMeasurement[]> {
    return this.call('get_growth_history', patientHash);
  }

  async calculateGrowthPercentiles(input: CalculatePercentilesInput): Promise<GrowthPercentiles> {
    return this.call('calculate_growth_percentiles', input);
  }

  // Immunization methods

  async recordImmunization(record: ImmunizationRecord): Promise<ActionHash> {
    return this.call('record_immunization', record);
  }

  async getImmunizationHistory(patientHash: ActionHash): Promise<ImmunizationRecord[]> {
    return this.call('get_immunization_history', patientHash);
  }

  async getImmunizationStatus(input: ImmunizationStatusInput): Promise<ImmunizationStatusOutput> {
    return this.call('get_immunization_status', input);
  }

  // Developmental milestone methods

  async recordMilestone(milestone: DevelopmentalMilestone): Promise<ActionHash> {
    return this.call('record_milestone', milestone);
  }

  async getPatientMilestones(patientHash: ActionHash): Promise<DevelopmentalMilestone[]> {
    return this.call('get_patient_milestones', patientHash);
  }

  async getDevelopmentalSummary(
    patientHash: ActionHash,
    ageMonths: number
  ): Promise<DevelopmentalSummary> {
    return this.call('get_developmental_summary', { patientHash, ageMonths });
  }

  // Well-child visit methods

  async recordWellChildVisit(visit: WellChildVisit): Promise<ActionHash> {
    return this.call('record_well_child_visit', visit);
  }

  async getPatientWellChildVisits(patientHash: ActionHash): Promise<WellChildVisit[]> {
    return this.call('get_patient_well_child_visits', patientHash);
  }

  // Condition tracking methods

  async recordCondition(condition: PediatricCondition): Promise<ActionHash> {
    return this.call('record_condition', condition);
  }

  async getPatientConditions(patientHash: ActionHash): Promise<PediatricCondition[]> {
    return this.call('get_patient_conditions', patientHash);
  }

  // School health methods

  async createSchoolHealthRecord(record: SchoolHealthRecord): Promise<ActionHash> {
    return this.call('create_school_health_record', record);
  }

  async getSchoolHealthRecords(patientHash: ActionHash): Promise<SchoolHealthRecord[]> {
    return this.call('get_school_health_records', patientHash);
  }

  // Adolescent health methods

  async recordAdolescentAssessment(assessment: AdolescentHealth): Promise<ActionHash> {
    return this.call('record_adolescent_assessment', assessment);
  }

  async getAdolescentAssessments(patientHash: ActionHash): Promise<AdolescentHealth[]> {
    return this.call('get_adolescent_assessments', patientHash);
  }

  // Newborn methods

  async createNewbornRecord(record: NewbornRecord): Promise<ActionHash> {
    return this.call('create_newborn_record', record);
  }

  async getNewbornRecord(patientHash: ActionHash): Promise<NewbornRecord | null> {
    return this.call('get_newborn_record', patientHash);
  }
}
