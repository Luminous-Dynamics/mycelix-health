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

// FHIR Mapping (Phase 3 - Clinical Integration)
export { FhirMappingClient } from './fhir-mapping';
export type {
  FhirIdentifier,
  FhirQuantity,
  FhirDosage,
  FhirPatientMapping,
  FhirObservationMapping,
  FhirConditionMapping,
  FhirMedicationMapping,
  FhirBundleRecord,
  CreatePatientMappingInput,
  ExportPatientBundleInput,
  ImportFhirBundleInput,
} from './fhir-mapping';

// Clinical Decision Support (Phase 3)
export { CdsClient } from './cds';
export type {
  InteractionSeverity,
  AlertType,
  AlertPriority,
  GuidelineCategory,
  EvidenceLevel,
  DrugInteraction,
  DrugAllergyInteraction,
  ClinicalAlert,
  ClinicalGuideline,
  GuidelineRecommendation,
  PatientGuidelineStatus,
  InteractionCheckResponse,
  CreateAlertInput,
  CheckInteractionsInput,
} from './cds';

// Provider Directory (Phase 3)
export { ProviderDirectoryClient } from './provider-directory';
export type {
  ProviderStatus,
  AffiliationType,
  PersonName,
  PracticeLocation,
  ProviderProfile,
  NpiVerificationResult,
  ProviderAffiliation,
  ProviderSearchCriteria,
  RegisterProviderInput,
  UpdateProviderInput,
  AddAffiliationInput,
} from './provider-directory';

// Telehealth (Phase 3)
export { TelehealthClient } from './telehealth';
export type {
  SessionType,
  SessionStatus,
  WaitingRoomStatus,
  TelehealthSession,
  WaitingRoomEntry,
  SessionDocumentation,
  AvailableSlot,
  ProviderSchedule,
  SessionSummary,
  ScheduleSessionInput,
  CreateDocumentationInput,
  GetAvailableSlotsInput,
  UpdateSessionInput,
} from './telehealth';

// SDOH - Social Determinants of Health (Phase 4 - Equity & Access)
export { SdohClient } from './sdoh';
export {
  ScreeningInstrument,
  SdohDomain,
  SdohCategory,
  RiskLevel,
  InterventionStatus,
  ResourceType,
} from './sdoh';
export type {
  ScreeningResponse,
  SdohScreening,
  CommunityResource,
  SdohIntervention,
  InterventionFollowUp,
  PatientSdohSummary,
  CreateScreeningInput as CreateSdohScreeningInput,
  CreateResourceInput,
  ResourceSearchCriteria,
  CreateInterventionInput,
  UpdateInterventionInput,
  CreateFollowUpInput,
} from './sdoh';

// Mental Health (Phase 4 - Equity & Access)
export { MentalHealthClient } from './mental-health';
export {
  MentalHealthInstrument,
  Severity,
  CrisisLevel,
  TreatmentModality,
  SafetyPlanStatus,
  SubstanceCategory,
  Part2ConsentType,
} from './mental-health';
export type {
  MentalHealthScreening,
  MoodEntry,
  TreatmentGoal,
  PsychMedication,
  MentalHealthTreatmentPlan,
  ContactInfo as SafetyPlanContact,
  SafetyPlan,
  CrisisEvent,
  Part2Consent,
  TherapyNote,
  CreateScreeningInput as CreateMentalHealthScreeningInput,
  CreateMoodEntryInput,
  CreateSafetyPlanInput,
  CreateCrisisEventInput,
  CreatePart2ConsentInput,
} from './mental-health';

// Chronic Care (Phase 4 - Equity & Access)
export { ChronicCareClient } from './chronic-care';
export {
  DiabetesType,
  NYHAClass,
  GOLDStage,
  CKDStage,
  AlertSeverity,
} from './chronic-care';
export type {
  ChronicCondition,
  ChronicDiseaseEnrollment,
  CareGoal,
  ChronicCarePlan,
  PatientReportedOutcome,
  DiabetesMetrics,
  HeartFailureMetrics,
  COPDMetrics,
  MedicationAdherence,
  ChronicCareAlert,
  ExacerbationEvent,
  ChronicCareSummary,
  CreateEnrollmentInput,
  CreateCarePlanInput,
  RecordOutcomeInput,
  AdherenceRateInput,
  AdherenceRateOutput,
  AcknowledgeAlertInput,
} from './chronic-care';

// Pediatric Care (Phase 4 - Equity & Access)
export { PediatricClient } from './pediatric';
export {
  VaccineType,
  ImmunizationStatus,
  DevelopmentalDomain,
  MilestoneStatus,
  FeedingType,
} from './pediatric';
export type {
  GrowthMeasurement,
  ImmunizationRecord,
  DevelopmentalMilestone,
  WellChildVisit,
  PediatricCondition,
  SchoolHealthRecord,
  AdolescentHealth,
  NewbornRecord,
  RecordGrowthInput,
  RecordImmunizationInput,
  RecordMilestoneInput,
  RecordWellChildVisitInput,
  CalculatePercentilesInput,
  GrowthPercentiles,
  ImmunizationStatusInput,
  ImmunizationStatusOutput,
  DevelopmentalSummary,
} from './pediatric';
