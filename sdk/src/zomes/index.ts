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

// Research Commons (Phase 5 - Advanced Research)
export { ResearchCommonsClient } from './research-commons';
export {
  DatasetAccessLevel,
  LicenseType,
  DataQualityScore,
  AccessRequestStatus,
} from './research-commons';
export type {
  DatasetMetadata,
  ResearchDataset,
  DataUseAgreement,
  ContributionCredit,
  CreateDatasetInput,
  RequestAccessInput,
  ApproveAccessInput,
} from './research-commons';

// Trial Matching (Phase 5 - Advanced Research)
export { TrialMatchingClient } from './trial-matching';
export type {
  TrialEligibilityCriteria,
  PatientProfile,
  TrialPreferences,
  MatchResult,
  TrialRecommendation,
  CreatePatientProfileInput,
  FindMatchesInput,
} from './trial-matching';

// IRB (Phase 5 - Advanced Research)
export { IrbClient } from './irb';
export {
  ProtocolStatus,
  ReviewType,
  ReviewerRole,
  VoteType,
} from './irb';
export type {
  ProtocolSubmission,
  IrbMember,
  ProtocolReview,
  IrbMeeting,
  CreateProtocolInput,
  SubmitReviewInput,
} from './irb';

// Federated Learning (Phase 5 - Advanced Research)
export { FederatedLearningClient } from './federated-learning';
export {
  ProjectStatus,
  RoundStatus,
  AggregationMethod,
} from './federated-learning';
export type {
  ModelArchitecture,
  FederatedProject,
  TrainingRound,
  ModelUpdate,
  AggregatedModel,
  CreateProjectInput as CreateFederatedProjectInput,
  SubmitUpdateInput,
  AggregateUpdatesInput,
} from './federated-learning';

// Population Health (Phase 5 - Advanced Research)
export { PopulationHealthClient } from './population-health';
export {
  SurveillanceType,
  AlertLevel,
  ReportingFrequency,
} from './population-health';
export type {
  SurveillanceIndicator,
  PopulationMetric,
  HealthAlert as PopulationHealthAlert,
  PopulationTrend,
  CommunityHealthProfile,
  CreateIndicatorInput,
  RecordMetricInput,
  GetTrendsInput,
} from './population-health';

// IPS - International Patient Summary (Phase 6 - Global Scale)
export { IpsClient } from './ips';
export { IpsStatus } from './ips';
export type {
  IpsMedication,
  IpsAllergy,
  IpsCondition,
  IpsProcedure,
  IpsImmunization,
  InternationalPatientSummary,
  IpsExport,
  CreateIpsInput,
  UpdateIpsInput,
} from './ips';

// i18n - Internationalization (Phase 6 - Global Scale)
export { I18nClient } from './i18n';
export type {
  LocalizedTerm,
  TranslationMemory,
  GlossaryEntry,
  SupportedLocale,
  GetMedicalTermInput,
  AddLocalizedTermInput,
  TranslateInput,
  AddTranslationMemoryInput,
  AddGlossaryEntryInput,
} from './i18n';

// Disaster Response (Phase 6 - Global Scale)
export { DisasterResponseClient } from './disaster-response';
export {
  DisasterType,
  DisasterStatus,
  ResourceType as DisasterResourceType,
  ResourceStatus,
  TriageCategory,
} from './disaster-response';
export type {
  DisasterDeclaration,
  EmergencyResource,
  ResourceRequest,
  MassTriageRecord,
  EvacuationOrder,
  DeclareDisasterInput,
  RegisterResourceInput,
  RequestResourcesInput,
  RecordTriageInput,
} from './disaster-response';

// Verifiable Credentials (Phase 6 - Global Scale)
export { VerifiableCredentialsClient } from './verifiable-credentials';
export {
  CredentialType,
  CredentialStatus,
  ProofType,
} from './verifiable-credentials';
export type {
  VerifiableCredential,
  VerifiablePresentation,
  CredentialSchema,
  SchemaProperty,
  RevocationEntry,
  TrustRelationship,
  IssueCredentialInput,
  CreatePresentationInput,
  VerifyCredentialInput,
  VerificationResult,
} from './verifiable-credentials';

// Mobile Support (Phase 6 - Global Scale)
export { MobileSupportClient } from './mobile-support';
export {
  SyncStatus,
  ConflictResolution,
  DataPriority,
} from './mobile-support';
export type {
  SyncQueue,
  SyncItem,
  SyncConflict,
  OfflineCache,
  DeviceRegistration,
  BandwidthProfile,
  RegisterDeviceInput,
  QueueSyncItemInput,
  CacheDataInput,
  SetBandwidthProfileInput,
  SyncResult,
} from './mobile-support';
