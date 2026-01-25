/**
 * @mycelix/health-sdk
 *
 * TypeScript SDK for Mycelix-Health Holochain hApp
 *
 * Features:
 * - Type-safe zome clients mirroring Rust structs
 * - Built-in differential privacy safety with client-side budget validation
 * - Automatic "Check Budget -> Validate -> Query" workflow enforcement
 * - Privacy Fuel Gauge for UI integration
 *
 * @example
 * ```typescript
 * import { MycelixHealthClient } from '@mycelix/health-sdk';
 * import { AppWebsocket } from '@holochain/client';
 *
 * // Connect directly with SDK
 * const health = await MycelixHealthClient.connect({
 *   url: 'ws://localhost:8888',
 *   appId: 'mycelix-health',
 * });
 *
 * // Or use an existing client
 * const client = await AppWebsocket.connect({ url: 'ws://localhost:8888' });
 * const health = MycelixHealthClient.fromClient(client);
 *
 * // Create a patient record
 * const patient = await health.patients.createPatient({
 *   first_name: 'Jane',
 *   last_name: 'Doe',
 *   date_of_birth: '1990-01-15',
 *   contact: { email: 'jane@example.com' },
 * });
 *
 * // Check privacy budget before querying
 * const budgetStatus = await health.commons.getBudgetStatus(patient.hash, poolHash);
 * console.log(`Privacy budget: ${budgetStatus.percentRemaining}% remaining`);
 *
 * // Execute DP query with automatic safety enforcement
 * const result = await health.commons.countWithPrivacy(poolHash, patient.hash, 0.5);
 * console.log(`Noisy count: ${result.value}`);
 * ```
 */

import { AppClient, AppWebsocket, encodeHashToBase64 } from '@holochain/client';
import { PatientClient } from './zomes/patient';
import { ConsentClient } from './zomes/consent';
import { CommonsClient } from './zomes/commons';
import { TrialsClient } from './zomes/trials';
// Phase 3 - Clinical Integration
import { FhirMappingClient } from './zomes/fhir-mapping';
import { CdsClient } from './zomes/cds';
import { ProviderDirectoryClient } from './zomes/provider-directory';
import { TelehealthClient } from './zomes/telehealth';
// Phase 4 - Equity & Access
import { SdohClient } from './zomes/sdoh';
import { MentalHealthClient } from './zomes/mental-health';
import { ChronicCareClient } from './zomes/chronic-care';
import { PediatricClient } from './zomes/pediatric';
// Phase 5 - Advanced Research
import { ResearchCommonsClient } from './zomes/research-commons';
import { TrialMatchingClient } from './zomes/trial-matching';
import { IrbClient } from './zomes/irb';
import { FederatedLearningClient } from './zomes/federated-learning';
import { PopulationHealthClient } from './zomes/population-health';
// Phase 6 - Global Scale
import { IpsClient } from './zomes/ips';
import { I18nClient } from './zomes/i18n';
import { DisasterResponseClient } from './zomes/disaster-response';
import { VerifiableCredentialsClient } from './zomes/verifiable-credentials';
import { MobileSupportClient } from './zomes/mobile-support';
import { HealthSdkError, HealthSdkErrorCode, DEFAULT_CONFIG } from './types';
import type { MycelixHealthConfig } from './types';

/**
 * Helper to encode agent pub key to hex string for logging
 */
function encodeAgentPubKey(pubKey: Uint8Array): string {
  return encodeHashToBase64(pubKey);
}

// Re-export all types
export * from './types';

// Re-export zome clients for direct usage
export * from './zomes';

// Re-export privacy utilities
export * from './privacy';

// Re-export accessibility utilities
export * from './accessibility';

/**
 * MycelixHealthClient
 *
 * Unified client for the Mycelix-Health hApp providing access to all zomes.
 *
 * @example
 * ```typescript
 * // With existing client
 * const health = new MycelixHealthClient(existingClient);
 *
 * // Or connect directly
 * const health = await MycelixHealthClient.connect({
 *   url: 'ws://localhost:8888',
 *   appId: 'mycelix-health',
 * });
 * ```
 */
export class MycelixHealthClient {
  /**
   * Patient record management
   */
  public readonly patients: PatientClient;

  /**
   * Consent management (grants, revocations, authorization checks)
   */
  public readonly consent: ConsentClient;

  /**
   * Health Commons (data pools, DP queries, privacy budget)
   */
  public readonly commons: CommonsClient;

  /**
   * Clinical trials (enrollment, adverse events)
   */
  public readonly trials: TrialsClient;

  // Phase 3 - Clinical Integration

  /**
   * FHIR R4 mapping (patient bundles, terminology validation)
   */
  public readonly fhirMapping: FhirMappingClient;

  /**
   * Clinical Decision Support (drug interactions, alerts, guidelines)
   */
  public readonly cds: CdsClient;

  /**
   * Provider Directory (NPI verification, search, affiliations)
   */
  public readonly providerDirectory: ProviderDirectoryClient;

  /**
   * Telehealth (sessions, scheduling, waiting room)
   */
  public readonly telehealth: TelehealthClient;

  // Phase 4 - Equity & Access

  /**
   * Social Determinants of Health (SDOH screening, resources, interventions)
   */
  public readonly sdoh: SdohClient;

  /**
   * Mental Health (screenings, crisis management, 42 CFR Part 2 consent)
   */
  public readonly mentalHealth: MentalHealthClient;

  /**
   * Chronic Care (disease management, care plans, adherence tracking)
   */
  public readonly chronicCare: ChronicCareClient;

  /**
   * Pediatric Care (growth, immunizations, developmental milestones)
   */
  public readonly pediatric: PediatricClient;

  // Phase 5 - Advanced Research

  /**
   * Research Commons (open datasets, data use agreements)
   */
  public readonly researchCommons: ResearchCommonsClient;

  /**
   * Trial Matching (AI-powered eligibility, recommendations)
   */
  public readonly trialMatching: TrialMatchingClient;

  /**
   * IRB (protocol submissions, ethical review)
   */
  public readonly irb: IrbClient;

  /**
   * Federated Learning (privacy-preserving ML)
   */
  public readonly federatedLearning: FederatedLearningClient;

  /**
   * Population Health (surveillance, community health)
   */
  public readonly populationHealth: PopulationHealthClient;

  // Phase 6 - Global Scale

  /**
   * International Patient Summary (cross-border health data)
   */
  public readonly ips: IpsClient;

  /**
   * Internationalization (medical terminology translation)
   */
  public readonly i18n: I18nClient;

  /**
   * Disaster Response (emergency coordination)
   */
  public readonly disasterResponse: DisasterResponseClient;

  /**
   * Verifiable Credentials (W3C VC for health data)
   */
  public readonly verifiableCredentials: VerifiableCredentialsClient;

  /**
   * Mobile Support (offline-first sync)
   */
  public readonly mobileSupport: MobileSupportClient;

  /**
   * The underlying Holochain client
   */
  public readonly client: AppClient;

  /**
   * Configuration used to create this client
   */
  public readonly config: Required<MycelixHealthConfig>;

  private constructor(
    client: AppClient,
    config: Required<MycelixHealthConfig>
  ) {
    this.client = client;
    this.config = config;

    // Initialize all zome clients
    this.patients = new PatientClient(client, config.roleName);
    this.consent = new ConsentClient(client, config.roleName);
    this.commons = new CommonsClient(client, config.roleName);
    this.trials = new TrialsClient(client, config.roleName);

    // Phase 3 - Clinical Integration
    this.fhirMapping = new FhirMappingClient(client, config.roleName);
    this.cds = new CdsClient(client, config.roleName);
    this.providerDirectory = new ProviderDirectoryClient(client, config.roleName);
    this.telehealth = new TelehealthClient(client, config.roleName);

    // Phase 4 - Equity & Access
    this.sdoh = new SdohClient(client, config.roleName);
    this.mentalHealth = new MentalHealthClient(client, config.roleName);
    this.chronicCare = new ChronicCareClient(client, config.roleName);
    this.pediatric = new PediatricClient(client, config.roleName);

    // Phase 5 - Advanced Research
    this.researchCommons = new ResearchCommonsClient(client, config.roleName);
    this.trialMatching = new TrialMatchingClient(client, config.roleName);
    this.irb = new IrbClient(client, config.roleName);
    this.federatedLearning = new FederatedLearningClient(client, config.roleName);
    this.populationHealth = new PopulationHealthClient(client, config.roleName);

    // Phase 6 - Global Scale
    this.ips = new IpsClient(client, config.roleName);
    this.i18n = new I18nClient(client, config.roleName);
    this.disasterResponse = new DisasterResponseClient(client, config.roleName);
    this.verifiableCredentials = new VerifiableCredentialsClient(client, config.roleName);
    this.mobileSupport = new MobileSupportClient(client, config.roleName);
  }

  /**
   * Create a MycelixHealthClient from an existing AppClient
   *
   * @param client - Existing Holochain client
   * @param config - Optional configuration overrides
   * @returns Configured MycelixHealthClient
   */
  static fromClient(
    client: AppClient,
    config: Partial<MycelixHealthConfig> = {}
  ): MycelixHealthClient {
    const fullConfig: Required<MycelixHealthConfig> = {
      ...DEFAULT_CONFIG,
      ...config,
    };

    return new MycelixHealthClient(client, fullConfig);
  }

  /**
   * Connect to Holochain and create a MycelixHealthClient
   *
   * @param config - Connection configuration
   * @returns Connected MycelixHealthClient
   * @throws HealthSdkError if connection fails
   */
  static async connect(
    config: Partial<MycelixHealthConfig> = {}
  ): Promise<MycelixHealthClient> {
    const fullConfig: Required<MycelixHealthConfig> = {
      ...DEFAULT_CONFIG,
      ...config,
    };

    let client: AppClient;
    let attempts = 0;

    while (attempts < fullConfig.retry.maxAttempts) {
      try {
        client = await AppWebsocket.connect({
          url: new URL(fullConfig.url),
        });

        // Verify connection by getting app info
        const appInfo = await client.appInfo();
        if (!appInfo) {
          throw new Error('Failed to get app info');
        }

        if (fullConfig.debug) {
          console.log(`[health-sdk] Connected to ${fullConfig.appId}`);
          console.log(`[health-sdk] Agent: ${encodeAgentPubKey(client.myPubKey).slice(0, 16)}...`);
        }

        return new MycelixHealthClient(client, fullConfig);
      } catch (error) {
        attempts++;
        const message = error instanceof Error ? error.message : String(error);

        if (attempts >= fullConfig.retry.maxAttempts) {
          throw new HealthSdkError(
            HealthSdkErrorCode.CONNECTION_FAILED,
            `Failed to connect after ${attempts} attempts: ${message}`,
            { url: fullConfig.url, appId: fullConfig.appId }
          );
        }

        // Exponential backoff
        const delay = fullConfig.retry.delayMs * Math.pow(fullConfig.retry.backoffMultiplier, attempts - 1);

        if (fullConfig.debug) {
          console.log(`[health-sdk] Connection attempt ${attempts} failed, retrying in ${delay}ms...`);
        }

        await new Promise(resolve => setTimeout(resolve, delay));
      }
    }

    // TypeScript requires this, but it's unreachable
    throw new HealthSdkError(
      HealthSdkErrorCode.CONNECTION_FAILED,
      'Connection failed'
    );
  }

  /**
   * Get the current agent's public key
   *
   * @returns Agent public key as Uint8Array
   */
  getAgentPubKey(): Uint8Array {
    return this.client.myPubKey;
  }

  /**
   * Check if connected to Holochain
   *
   * @returns true if connection is healthy
   */
  async isConnected(): Promise<boolean> {
    try {
      const appInfo = await this.client.appInfo();
      return appInfo !== null;
    } catch {
      return false;
    }
  }

  /**
   * Get a summary of all zome capabilities
   *
   * Useful for debugging and introspection.
   */
  getCapabilities(): {
    patients: string[];
    consent: string[];
    commons: string[];
    trials: string[];
    fhirMapping: string[];
    cds: string[];
    providerDirectory: string[];
    telehealth: string[];
    sdoh: string[];
    mentalHealth: string[];
    chronicCare: string[];
    pediatric: string[];
    // Phase 5 - Advanced Research
    researchCommons: string[];
    trialMatching: string[];
    irb: string[];
    federatedLearning: string[];
    populationHealth: string[];
    // Phase 6 - Global Scale
    ips: string[];
    i18n: string[];
    disasterResponse: string[];
    verifiableCredentials: string[];
    mobileSupport: string[];
  } {
    return {
      patients: [
        'createPatient',
        'getPatient',
        'getMyPatient',
        'updatePatient',
        'searchPatients',
        'addAllergy',
        'addMedication',
        'setPrimaryProvider',
      ],
      consent: [
        'grantConsent',
        'revokeConsent',
        'getConsent',
        'listPatientConsents',
        'listGranteeConsents',
        'checkAuthorization',
        'amIAuthorized',
        'getConsentSummary',
        'updateConsentScope',
        'extendConsent',
      ],
      commons: [
        'createPool',
        'getPool',
        'listActivePools',
        'contributeData',
        'getContributionCount',
        'getBudgetLedger',
        'getBudgetStatus',
        'canQuery',
        'queryAggregate',
        'countWithPrivacy',
        'sumWithPrivacy',
        'averageWithPrivacy',
      ],
      trials: [
        'createTrial',
        'getTrial',
        'getTrialByExternalId',
        'updateTrialStatus',
        'listTrialsByStatus',
        'listRecruitingTrials',
        'searchTrialsByCondition',
        'checkEligibility',
        'enrollPatient',
        'withdrawPatient',
        'getEnrollmentStatus',
        'listTrialEnrollments',
        'listPatientEnrollments',
        'reportAdverseEvent',
        'getAdverseEvent',
        'listTrialAdverseEvents',
        'listAdverseEventsBySeverity',
        'updateAdverseEventOutcome',
        'getTrialStatistics',
      ],
      // Phase 3 - Clinical Integration
      fhirMapping: [
        'createPatientMapping',
        'getPatientMapping',
        'exportPatientBundle',
        'importFhirBundle',
        'validateLoincCode',
        'validateSnomedCode',
        'validateIcd10Code',
        'createObservationMapping',
        'createConditionMapping',
        'createMedicationMapping',
      ],
      cds: [
        'checkDrugInteractions',
        'checkDrugAllergyInteractions',
        'createClinicalAlert',
        'getPatientAlerts',
        'acknowledgeAlert',
        'dismissAlert',
        'createGuideline',
        'getApplicableGuidelines',
        'updatePatientGuidelineStatus',
      ],
      providerDirectory: [
        'registerProvider',
        'getProvider',
        'updateProvider',
        'searchProviders',
        'verifyNpi',
        'addAffiliation',
        'removeAffiliation',
        'getProviderAffiliations',
        'setAcceptingNewPatients',
        'getAcceptingProviders',
      ],
      telehealth: [
        'scheduleSession',
        'getSession',
        'updateSession',
        'cancelSession',
        'startSession',
        'endSession',
        'joinWaitingRoom',
        'leaveWaitingRoom',
        'getWaitingRoom',
        'admitFromWaitingRoom',
        'createDocumentation',
        'getSessionDocumentation',
        'getAvailableSlots',
        'getProviderSessions',
        'getPatientSessions',
      ],
      // Phase 4 - Equity & Access
      sdoh: [
        'createScreening',
        'getScreening',
        'getPatientScreenings',
        'createResource',
        'getResource',
        'searchResources',
        'createIntervention',
        'getScreeningInterventions',
        'updateIntervention',
        'createFollowUp',
        'getInterventionFollowUps',
        'getPatientSdohSummary',
      ],
      mentalHealth: [
        'createScreening',
        'getPatientScreenings',
        'createMoodEntry',
        'getPatientMoodEntries',
        'createSafetyPlan',
        'getPatientSafetyPlan',
        'updateSafetyPlan',
        'createCrisisEvent',
        'getPatientCrisisEvents',
        'createPart2Consent',
        'getPatientPart2Consents',
        'revokePart2Consent',
        'createTherapyNote',
        'getPatientTherapyNotes',
      ],
      chronicCare: [
        'enrollPatient',
        'getPatientEnrollments',
        'createCarePlan',
        'getCarePlans',
        'updateCarePlan',
        'recordOutcome',
        'recordDiabetesMetrics',
        'recordHeartFailureMetrics',
        'recordCOPDMetrics',
        'recordMedicationAdherence',
        'getAdherenceRate',
        'createAlert',
        'acknowledgeAlert',
        'getPendingAlerts',
        'recordExacerbation',
        'getChronicCareSummary',
      ],
      pediatric: [
        'recordGrowth',
        'getGrowthHistory',
        'calculateGrowthPercentiles',
        'recordImmunization',
        'getImmunizationHistory',
        'getImmunizationStatus',
        'recordMilestone',
        'getPatientMilestones',
        'getDevelopmentalSummary',
        'recordWellChildVisit',
        'getPatientWellChildVisits',
        'recordCondition',
        'getPatientConditions',
        'createSchoolHealthRecord',
        'getSchoolHealthRecords',
        'recordAdolescentAssessment',
        'getAdolescentAssessments',
        'createNewbornRecord',
        'getNewbornRecord',
      ],
      // Phase 5 - Advanced Research
      researchCommons: [
        'createDataset',
        'getDataset',
        'searchDatasets',
        'listPublicDatasets',
        'requestAccess',
        'approveAccess',
        'denyAccess',
        'getPendingRequests',
        'addContributionCredit',
        'getDatasetCredits',
      ],
      trialMatching: [
        'createPatientProfile',
        'getPatientProfile',
        'findMatches',
        'getRecommendations',
        'refreshRecommendations',
        'checkEligibility',
        'expressInterest',
        'getInterestedPatients',
      ],
      irb: [
        'createProtocol',
        'getProtocol',
        'submitForReview',
        'getPendingProtocols',
        'submitReview',
        'getProtocolReviews',
        'approveProtocol',
        'rejectProtocol',
        'requestRevisions',
        'getMyProtocols',
        'getMembers',
      ],
      federatedLearning: [
        'createProject',
        'getProject',
        'listActiveProjects',
        'joinProject',
        'leaveProject',
        'startRound',
        'getCurrentRound',
        'submitUpdate',
        'getRoundUpdates',
        'aggregateUpdates',
        'getAggregatedModel',
        'getProjectModels',
        'getMyParticipation',
      ],
      populationHealth: [
        'createIndicator',
        'getIndicator',
        'listIndicators',
        'recordMetric',
        'getMetrics',
        'getCurrentAlerts',
        'acknowledgeAlert',
        'getTrends',
        'getCommunityProfile',
        'compareRegions',
      ],
      // Phase 6 - Global Scale
      ips: [
        'createIps',
        'getIps',
        'getPatientIps',
        'updateIps',
        'finalizeIps',
        'exportIps',
        'importIps',
        'validateIps',
        'getIpsHistory',
      ],
      i18n: [
        'getMedicalTerm',
        'addLocalizedTerm',
        'verifyTranslation',
        'translate',
        'addTranslationMemory',
        'getSuggestions',
        'addGlossaryEntry',
        'getGlossaryEntries',
        'getSupportedLocales',
        'getTerminologyCoverage',
      ],
      disasterResponse: [
        'declareDisaster',
        'getDisaster',
        'getActiveDisasters',
        'updateDisasterStatus',
        'registerResource',
        'getAvailableResources',
        'requestResources',
        'getPendingRequests',
        'fulfillRequest',
        'recordTriage',
        'getTriageRecords',
        'getTriageSummary',
        'issueEvacuationOrder',
        'getActiveEvacuationOrders',
      ],
      verifiableCredentials: [
        'issueCredential',
        'getCredential',
        'getSubjectCredentials',
        'getIssuedCredentials',
        'revokeCredential',
        'isRevoked',
        'createPresentation',
        'getPresentation',
        'verifyCredential',
        'verifyPresentation',
        'registerSchema',
        'getSchemas',
        'establishTrust',
        'getTrustedIssuers',
      ],
      mobileSupport: [
        'registerDevice',
        'getDevice',
        'updateDevice',
        'getMyDevices',
        'queueSyncItem',
        'getSyncQueue',
        'getPendingItems',
        'executeSync',
        'markSynced',
        'reportSyncFailure',
        'getConflicts',
        'resolveConflict',
        'cacheData',
        'getCachedData',
        'clearExpiredCache',
        'setBandwidthProfile',
        'getBandwidthProfile',
        'getSyncStats',
      ],
    };
  }
}

// Default export
export default MycelixHealthClient;
