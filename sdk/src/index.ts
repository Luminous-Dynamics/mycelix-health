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
    };
  }
}

// Default export
export default MycelixHealthClient;
