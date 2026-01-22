/**
 * Commons Zome Client
 *
 * Client for the Health Commons zome with built-in differential privacy safety.
 * Enforces the "Check Budget -> Validate -> Query" workflow automatically.
 */

import type { AppClient, ActionHash } from '@holochain/client';
import type {
  BudgetLedgerEntry,
  PrivacyBudgetStatus,
  AggregateQueryRequest,
  DpQueryResult,
  DataPool,
} from '../types';
import { HealthSdkError, HealthSdkErrorCode } from '../types';
import { PrivacyBudgetManager } from '../privacy/budget';

/**
 * Input for creating a data pool
 */
export interface CreatePoolInput {
  name: string;
  description: string;
  data_categories: string[];
  required_consent_level: string;
  default_epsilon: number;
  budget_per_user: number;
  governance_model: 'Democratic' | 'Cooperative' | 'Stewardship' | 'Hybrid';
  min_contributors: number;
}

/**
 * Input for contributing data to a pool
 */
export interface ContributeDataInput {
  pool_hash: ActionHash;
  encrypted_data: Uint8Array;
  data_category: string;
  consent_hash: ActionHash;
}

/**
 * Options for aggregate queries
 */
export interface QueryOptions {
  /**
   * Skip client-side budget validation (use with caution)
   * @default false
   */
  skipValidation?: boolean;

  /**
   * Patient hash for budget tracking
   * If not provided, uses the current agent's patient record
   */
  patientHash?: ActionHash;
}

/**
 * Commons Zome Client
 *
 * Provides type-safe access to the Health Commons zome with
 * automatic differential privacy safety enforcement.
 */
export class CommonsClient {
  private readonly roleName: string;
  private readonly zomeName = 'commons_coordinator';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  // ============================================================================
  // DATA POOLS
  // ============================================================================

  /**
   * Create a new data pool
   *
   * @param input - Pool configuration
   * @returns The created pool record
   */
  async createPool(input: CreatePoolInput): Promise<DataPool & { hash: ActionHash }> {
    const result = await this.call<{ hash: ActionHash; pool: DataPool }>('create_data_pool', input);
    return { ...result.pool, hash: result.hash };
  }

  /**
   * Get a data pool by hash
   *
   * @param poolHash - Hash of the pool
   * @returns The pool record or null if not found
   */
  async getPool(poolHash: ActionHash): Promise<DataPool | null> {
    return this.call<DataPool | null>('get_data_pool', poolHash);
  }

  /**
   * List all active data pools
   *
   * @returns Array of active pools
   */
  async listActivePools(): Promise<Array<DataPool & { hash: ActionHash }>> {
    return this.call<Array<DataPool & { hash: ActionHash }>>('list_active_pools', null);
  }

  // ============================================================================
  // DATA CONTRIBUTIONS
  // ============================================================================

  /**
   * Contribute data to a pool
   *
   * @param input - Contribution details
   * @returns The contribution record hash
   */
  async contributeData(input: ContributeDataInput): Promise<ActionHash> {
    return this.call<ActionHash>('contribute_to_pool', input);
  }

  /**
   * Get contribution count for a pool
   *
   * @param poolHash - Hash of the pool
   * @returns Number of contributions
   */
  async getContributionCount(poolHash: ActionHash): Promise<number> {
    return this.call<number>('get_pool_contribution_count', poolHash);
  }

  // ============================================================================
  // PRIVACY BUDGET MANAGEMENT
  // ============================================================================

  /**
   * Get the raw budget ledger entry for a patient-pool pair
   *
   * @param patientHash - Hash of the patient record
   * @param poolHash - Hash of the data pool
   * @returns Budget ledger entry from DHT
   */
  async getBudgetLedger(
    patientHash: ActionHash,
    poolHash: ActionHash
  ): Promise<BudgetLedgerEntry> {
    return this.call<BudgetLedgerEntry>('get_privacy_budget_status', {
      patient_hash: patientHash,
      pool_hash: poolHash,
    });
  }

  /**
   * Get computed budget status with UI-friendly values
   *
   * @param patientHash - Hash of the patient record
   * @param poolHash - Hash of the data pool
   * @returns Computed budget status
   */
  async getBudgetStatus(
    patientHash: ActionHash,
    poolHash: ActionHash
  ): Promise<PrivacyBudgetStatus> {
    const ledger = await this.getBudgetLedger(patientHash, poolHash);
    return PrivacyBudgetManager.calculateStatus(ledger);
  }

  /**
   * Check if a query can be executed with current budget
   *
   * @param patientHash - Hash of the patient record
   * @param poolHash - Hash of the data pool
   * @param epsilon - Epsilon cost of the query
   * @returns True if query can proceed, false otherwise
   */
  async canQuery(
    patientHash: ActionHash,
    poolHash: ActionHash,
    epsilon: number
  ): Promise<boolean> {
    try {
      const status = await this.getBudgetStatus(patientHash, poolHash);
      PrivacyBudgetManager.validateQuery(status, epsilon);
      return true;
    } catch {
      return false;
    }
  }

  // ============================================================================
  // DIFFERENTIALLY PRIVATE QUERIES
  // ============================================================================

  /**
   * Execute a differentially private aggregate query
   *
   * This method enforces the safety workflow:
   * 1. Fetch current budget status
   * 2. Validate query parameters client-side
   * 3. Validate sufficient budget exists
   * 4. Execute the query (which also validates server-side)
   *
   * @param request - The aggregate query request
   * @param patientHash - Hash of the patient for budget tracking
   * @param options - Query options
   * @returns Query result with noisy value and metadata
   * @throws HealthSdkError if validation fails or budget exhausted
   */
  async queryAggregate(
    request: AggregateQueryRequest,
    patientHash: ActionHash,
    options: QueryOptions = {}
  ): Promise<DpQueryResult> {
    // Step 1: Validate query parameters client-side
    if (!options.skipValidation) {
      PrivacyBudgetManager.validateParams(request.params);
    }

    // Step 2: Fetch and validate budget
    if (!options.skipValidation) {
      const ledger = await this.getBudgetLedger(patientHash, request.pool_hash);
      const status = PrivacyBudgetManager.calculateStatus(ledger);

      // Step 3: Client-side safety interlock
      try {
        PrivacyBudgetManager.validateQuery(status, request.params.epsilon);
      } catch (error) {
        // Enhance error with context
        if (error instanceof HealthSdkError) {
          throw new HealthSdkError(
            error.code,
            `Privacy Budget Safety Triggered: ${error.message}`,
            {
              ...error.details,
              poolHash: request.pool_hash,
              queryType: request.query_type,
            }
          );
        }
        throw error;
      }
    }

    // Step 4: Execute the query
    return this.call<DpQueryResult>('execute_dp_query', {
      ...request,
      patient_hash: patientHash,
    });
  }

  /**
   * Execute a count query with differential privacy
   *
   * Convenience method for counting records in a pool.
   *
   * @param poolHash - Hash of the pool
   * @param patientHash - Hash of the patient
   * @param epsilon - Privacy cost
   * @param options - Query options
   * @returns Noisy count result
   */
  async countWithPrivacy(
    poolHash: ActionHash,
    patientHash: ActionHash,
    epsilon: number,
    options: QueryOptions = {}
  ): Promise<DpQueryResult> {
    return this.queryAggregate(
      {
        pool_hash: poolHash,
        query_type: 'Count',
        params: {
          epsilon,
          sensitivity_bound: 1.0, // Count sensitivity is always 1
          noise_mechanism: 'Laplace',
        },
      },
      patientHash,
      options
    );
  }

  /**
   * Execute a sum query with differential privacy
   *
   * @param poolHash - Hash of the pool
   * @param patientHash - Hash of the patient
   * @param epsilon - Privacy cost
   * @param maxValue - Maximum possible value (determines sensitivity)
   * @param options - Query options
   * @returns Noisy sum result
   */
  async sumWithPrivacy(
    poolHash: ActionHash,
    patientHash: ActionHash,
    epsilon: number,
    maxValue: number,
    options: QueryOptions = {}
  ): Promise<DpQueryResult> {
    return this.queryAggregate(
      {
        pool_hash: poolHash,
        query_type: 'Sum',
        params: {
          epsilon,
          sensitivity_bound: maxValue,
          noise_mechanism: 'Laplace',
        },
      },
      patientHash,
      options
    );
  }

  /**
   * Execute an average query with differential privacy
   *
   * Note: Average queries require careful sensitivity analysis.
   * This method uses Gaussian mechanism for better confidence intervals.
   *
   * @param poolHash - Hash of the pool
   * @param patientHash - Hash of the patient
   * @param epsilon - Privacy cost
   * @param delta - Privacy failure probability
   * @param valueRange - Range of possible values (max - min)
   * @param minContributors - Minimum expected contributors (for sensitivity)
   * @param options - Query options
   * @returns Noisy average result
   */
  async averageWithPrivacy(
    poolHash: ActionHash,
    patientHash: ActionHash,
    epsilon: number,
    delta: number,
    valueRange: number,
    minContributors: number,
    options: QueryOptions = {}
  ): Promise<DpQueryResult> {
    // Sensitivity of average = range / n
    const sensitivity = valueRange / Math.max(1, minContributors);

    return this.queryAggregate(
      {
        pool_hash: poolHash,
        query_type: 'Average',
        params: {
          epsilon,
          delta,
          sensitivity_bound: sensitivity,
          noise_mechanism: 'Gaussian',
        },
      },
      patientHash,
      options
    );
  }

  // ============================================================================
  // INTERNAL HELPERS
  // ============================================================================

  /**
   * Make a zome call with proper typing
   */
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
      // Wrap Holochain errors in SDK errors
      const message = error instanceof Error ? error.message : String(error);

      if (message.includes('budget') || message.includes('exhausted')) {
        throw new HealthSdkError(
          HealthSdkErrorCode.BUDGET_EXHAUSTED,
          message,
          { fnName, payload }
        );
      }

      if (message.includes('unauthorized') || message.includes('consent')) {
        throw new HealthSdkError(
          HealthSdkErrorCode.UNAUTHORIZED,
          message,
          { fnName, payload }
        );
      }

      throw new HealthSdkError(
        HealthSdkErrorCode.ZOME_CALL_FAILED,
        `Zome call failed: ${message}`,
        { fnName, payload, originalError: message }
      );
    }
  }
}
