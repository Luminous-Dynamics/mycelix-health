/**
 * Privacy Budget Manager
 *
 * Client-side "Privacy Fuel Gauge" logic for tracking and validating
 * differential privacy budget consumption.
 *
 * This module enforces safety interlocks BEFORE queries hit the network,
 * providing instant UI feedback and saving network traffic.
 */

import type {
  BudgetLedgerEntry,
  PrivacyBudgetStatus,
  DpQueryParams,
} from '../types';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

/**
 * Recommended epsilon values for different sensitivity levels
 */
export const RECOMMENDED_EPSILON = {
  /** Highly sensitive data (HIV status, psychiatric records) */
  VERY_HIGH_SENSITIVITY: 0.1,
  /** Sensitive medical data (diagnoses, lab results) */
  HIGH_SENSITIVITY: 0.5,
  /** Moderate sensitivity (aggregate health metrics) */
  MODERATE_SENSITIVITY: 1.0,
  /** Lower sensitivity (general demographics) */
  LOW_SENSITIVITY: 2.0,
  /** Non-sensitive aggregate statistics */
  MINIMAL_SENSITIVITY: 5.0,
} as const;

/**
 * Maximum allowed epsilon values (security limits)
 */
export const EPSILON_LIMITS = {
  /** Absolute maximum epsilon (per query) */
  MAX_PER_QUERY: 10.0,
  /** Minimum epsilon (must be positive) */
  MIN_VALUE: 1e-10,
  /** Maximum delta for (ε, δ)-DP */
  MAX_DELTA: 0.01,
} as const;

/**
 * Privacy Budget Manager
 *
 * Provides client-side budget tracking, validation, and UI helpers.
 */
export class PrivacyBudgetManager {
  /**
   * Calculate the current status from a budget ledger entry
   *
   * @param ledger - The budget ledger entry from the DHT
   * @returns Computed privacy budget status
   */
  static calculateStatus(ledger: BudgetLedgerEntry): PrivacyBudgetStatus {
    const remaining = Math.max(0, ledger.total_epsilon - ledger.consumed_epsilon);
    const percentRemaining = ledger.total_epsilon > 0
      ? (remaining / ledger.total_epsilon) * 100
      : 0;

    return {
      total: ledger.total_epsilon,
      consumed: ledger.consumed_epsilon,
      remaining,
      queriesAnswered: ledger.query_count,
      isExhausted: remaining < EPSILON_LIMITS.MIN_VALUE,
      percentRemaining: Math.round(percentRemaining * 10) / 10, // 1 decimal place
    };
  }

  /**
   * Validate that a query can be executed with the current budget
   *
   * @param status - Current budget status
   * @param cost - Epsilon cost of the proposed query
   * @throws HealthSdkError if validation fails
   */
  static validateQuery(status: PrivacyBudgetStatus, cost: number): void {
    // Validate epsilon is positive
    if (cost <= 0) {
      throw new HealthSdkError(
        HealthSdkErrorCode.INVALID_EPSILON,
        'Privacy cost (epsilon) must be positive',
        { provided: cost, minimum: EPSILON_LIMITS.MIN_VALUE }
      );
    }

    // Validate epsilon is not too large
    if (cost > EPSILON_LIMITS.MAX_PER_QUERY) {
      throw new HealthSdkError(
        HealthSdkErrorCode.INVALID_EPSILON,
        `Epsilon exceeds maximum allowed per query (${EPSILON_LIMITS.MAX_PER_QUERY})`,
        { provided: cost, maximum: EPSILON_LIMITS.MAX_PER_QUERY }
      );
    }

    // Check if budget is already exhausted
    if (status.isExhausted) {
      throw new HealthSdkError(
        HealthSdkErrorCode.BUDGET_EXHAUSTED,
        'Privacy budget is exhausted. No further queries are allowed.',
        {
          total: status.total,
          consumed: status.consumed,
          queriesAnswered: status.queriesAnswered,
        }
      );
    }

    // Check if sufficient budget remains
    if (status.remaining < cost) {
      throw new HealthSdkError(
        HealthSdkErrorCode.BUDGET_EXHAUSTED,
        `Insufficient privacy budget. Required: ${cost.toFixed(4)}, Remaining: ${status.remaining.toFixed(4)}`,
        {
          required: cost,
          remaining: status.remaining,
          shortfall: cost - status.remaining,
        }
      );
    }
  }

  /**
   * Validate DP query parameters
   *
   * @param params - DP query parameters to validate
   * @throws HealthSdkError if validation fails
   */
  static validateParams(params: DpQueryParams): void {
    // Validate epsilon
    if (params.epsilon <= 0) {
      throw new HealthSdkError(
        HealthSdkErrorCode.INVALID_EPSILON,
        'Epsilon must be positive',
        { provided: params.epsilon }
      );
    }

    if (params.epsilon > EPSILON_LIMITS.MAX_PER_QUERY) {
      throw new HealthSdkError(
        HealthSdkErrorCode.INVALID_EPSILON,
        `Epsilon exceeds maximum (${EPSILON_LIMITS.MAX_PER_QUERY})`,
        { provided: params.epsilon, maximum: EPSILON_LIMITS.MAX_PER_QUERY }
      );
    }

    // Validate delta if using Gaussian mechanism
    if (params.noise_mechanism === 'Gaussian') {
      if (params.delta === undefined || params.delta === null) {
        throw new HealthSdkError(
          HealthSdkErrorCode.INVALID_DELTA,
          'Delta is required for Gaussian mechanism',
          { mechanism: params.noise_mechanism }
        );
      }

      if (params.delta < 0 || params.delta >= 1) {
        throw new HealthSdkError(
          HealthSdkErrorCode.INVALID_DELTA,
          'Delta must be in range [0, 1)',
          { provided: params.delta }
        );
      }

      if (params.delta > EPSILON_LIMITS.MAX_DELTA) {
        throw new HealthSdkError(
          HealthSdkErrorCode.INVALID_DELTA,
          `Delta exceeds recommended maximum (${EPSILON_LIMITS.MAX_DELTA})`,
          { provided: params.delta, maximum: EPSILON_LIMITS.MAX_DELTA }
        );
      }
    }

    // Validate sensitivity
    if (params.sensitivity_bound <= 0) {
      throw new HealthSdkError(
        HealthSdkErrorCode.INVALID_SENSITIVITY,
        'Sensitivity must be positive',
        { provided: params.sensitivity_bound }
      );
    }
  }

  /**
   * Estimate how many queries of a given epsilon can be answered
   *
   * @param status - Current budget status
   * @param epsilon - Epsilon per query
   * @returns Estimated number of remaining queries
   */
  static estimateRemainingQueries(status: PrivacyBudgetStatus, epsilon: number): number {
    if (epsilon <= 0 || status.isExhausted) {
      return 0;
    }
    return Math.floor(status.remaining / epsilon);
  }

  /**
   * Calculate optimal epsilon to spread remaining budget over N queries
   *
   * @param status - Current budget status
   * @param desiredQueries - Number of queries to budget for
   * @returns Recommended epsilon per query
   */
  static calculateOptimalEpsilon(status: PrivacyBudgetStatus, desiredQueries: number): number {
    if (desiredQueries <= 0 || status.isExhausted) {
      return 0;
    }

    const optimal = status.remaining / desiredQueries;

    // Clamp to valid range
    return Math.max(EPSILON_LIMITS.MIN_VALUE, Math.min(optimal, EPSILON_LIMITS.MAX_PER_QUERY));
  }

  /**
   * Generate a UI-friendly budget display
   *
   * @param status - Current budget status
   * @returns Object with display strings and severity level
   */
  static getDisplayInfo(status: PrivacyBudgetStatus): BudgetDisplayInfo {
    let severity: BudgetSeverity;
    let message: string;

    if (status.isExhausted) {
      severity = 'critical';
      message = 'Budget exhausted - no queries remaining';
    } else if (status.percentRemaining < 10) {
      severity = 'critical';
      message = `Critical: Only ${status.percentRemaining}% budget remaining`;
    } else if (status.percentRemaining < 25) {
      severity = 'warning';
      message = `Warning: ${status.percentRemaining}% budget remaining`;
    } else if (status.percentRemaining < 50) {
      severity = 'caution';
      message = `${status.percentRemaining}% budget remaining`;
    } else {
      severity = 'healthy';
      message = `${status.percentRemaining}% budget available`;
    }

    return {
      severity,
      message,
      remaining: status.remaining.toFixed(4),
      total: status.total.toFixed(4),
      consumed: status.consumed.toFixed(4),
      percentRemaining: status.percentRemaining,
      queriesAnswered: status.queriesAnswered,
      canQuery: !status.isExhausted,
    };
  }

  /**
   * Simulate budget consumption for planning purposes
   *
   * @param status - Current budget status
   * @param plannedQueries - Array of epsilon values for planned queries
   * @returns Simulated final status after all queries
   */
  static simulateConsumption(
    status: PrivacyBudgetStatus,
    plannedQueries: number[]
  ): PrivacyBudgetStatus {
    let remaining = status.remaining;
    let queriesAnswered = status.queriesAnswered;

    for (const epsilon of plannedQueries) {
      if (epsilon <= 0) continue;
      if (remaining < epsilon) break;

      remaining -= epsilon;
      queriesAnswered++;
    }

    return {
      total: status.total,
      consumed: status.total - remaining,
      remaining,
      queriesAnswered,
      isExhausted: remaining < EPSILON_LIMITS.MIN_VALUE,
      percentRemaining: status.total > 0 ? (remaining / status.total) * 100 : 0,
    };
  }
}

/**
 * Budget severity levels for UI display
 */
export type BudgetSeverity = 'healthy' | 'caution' | 'warning' | 'critical';

/**
 * Budget display information for UI
 */
export interface BudgetDisplayInfo {
  severity: BudgetSeverity;
  message: string;
  remaining: string;
  total: string;
  consumed: string;
  percentRemaining: number;
  queriesAnswered: number;
  canQuery: boolean;
}
