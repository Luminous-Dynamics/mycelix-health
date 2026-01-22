import { describe, it, expect } from 'vitest';
import {
  PrivacyBudgetManager,
  RECOMMENDED_EPSILON,
  EPSILON_LIMITS,
} from '../src/privacy/budget';
import type { BudgetLedgerEntry, PrivacyBudgetStatus } from '../src/types';

// Helper to create a mock ledger entry
function createLedger(overrides: Partial<BudgetLedgerEntry> = {}): BudgetLedgerEntry {
  return {
    patient_hash: new Uint8Array(32),
    pool_hash: new Uint8Array(32),
    total_epsilon: 10.0,
    consumed_epsilon: 0.0,
    total_delta: 0.01,
    consumed_delta: 0.0,
    query_count: 0,
    composition_method: { type: 'Basic' },
    created_at: BigInt(Date.now() * 1000),
    last_updated: BigInt(Date.now() * 1000),
    ...overrides,
  };
}

describe('PrivacyBudgetManager', () => {
  describe('calculateStatus', () => {
    it('should calculate status for fresh budget', () => {
      const ledger = createLedger();
      const status = PrivacyBudgetManager.calculateStatus(ledger);

      expect(status.total).toBe(10.0);
      expect(status.consumed).toBe(0.0);
      expect(status.remaining).toBe(10.0);
      expect(status.queriesAnswered).toBe(0);
      expect(status.isExhausted).toBe(false);
      expect(status.percentRemaining).toBe(100);
    });

    it('should calculate status for partially consumed budget', () => {
      const ledger = createLedger({
        consumed_epsilon: 3.0,
        query_count: 6,
      });
      const status = PrivacyBudgetManager.calculateStatus(ledger);

      expect(status.total).toBe(10.0);
      expect(status.consumed).toBe(3.0);
      expect(status.remaining).toBe(7.0);
      expect(status.queriesAnswered).toBe(6);
      expect(status.isExhausted).toBe(false);
      expect(status.percentRemaining).toBe(70);
    });

    it('should mark budget as exhausted when remaining is negligible', () => {
      const ledger = createLedger({
        consumed_epsilon: 10.0 - 1e-11, // Less than MIN_VALUE remaining
      });
      const status = PrivacyBudgetManager.calculateStatus(ledger);

      expect(status.isExhausted).toBe(true);
      expect(status.remaining).toBeCloseTo(0, 10);
    });

    it('should clamp remaining to zero if consumed exceeds total', () => {
      const ledger = createLedger({
        consumed_epsilon: 11.0, // Over budget
      });
      const status = PrivacyBudgetManager.calculateStatus(ledger);

      expect(status.remaining).toBe(0);
      expect(status.isExhausted).toBe(true);
    });

    it('should handle zero total budget', () => {
      const ledger = createLedger({
        total_epsilon: 0,
      });
      const status = PrivacyBudgetManager.calculateStatus(ledger);

      expect(status.percentRemaining).toBe(0);
      expect(status.isExhausted).toBe(true);
    });
  });

  describe('validateQuery', () => {
    const healthyStatus: PrivacyBudgetStatus = {
      total: 10.0,
      consumed: 2.0,
      remaining: 8.0,
      queriesAnswered: 4,
      isExhausted: false,
      percentRemaining: 80,
    };

    it('should allow valid queries', () => {
      expect(() => {
        PrivacyBudgetManager.validateQuery(healthyStatus, 0.5);
      }).not.toThrow();
    });

    it('should reject negative epsilon', () => {
      expect(() => {
        PrivacyBudgetManager.validateQuery(healthyStatus, -0.5);
      }).toThrow('must be positive');
    });

    it('should reject zero epsilon', () => {
      expect(() => {
        PrivacyBudgetManager.validateQuery(healthyStatus, 0);
      }).toThrow('must be positive');
    });

    it('should reject epsilon exceeding max', () => {
      expect(() => {
        PrivacyBudgetManager.validateQuery(healthyStatus, 15.0);
      }).toThrow('exceeds maximum');
    });

    it('should reject when budget is exhausted', () => {
      const exhausted: PrivacyBudgetStatus = {
        ...healthyStatus,
        remaining: 0,
        isExhausted: true,
      };

      expect(() => {
        PrivacyBudgetManager.validateQuery(exhausted, 0.5);
      }).toThrow('exhausted');
    });

    it('should reject when insufficient budget remains', () => {
      const lowBudget: PrivacyBudgetStatus = {
        ...healthyStatus,
        remaining: 0.3,
      };

      expect(() => {
        PrivacyBudgetManager.validateQuery(lowBudget, 0.5);
      }).toThrow('Insufficient');
    });
  });

  describe('validateParams', () => {
    it('should accept valid Laplace params', () => {
      expect(() => {
        PrivacyBudgetManager.validateParams({
          epsilon: 0.5,
          sensitivity_bound: 1.0,
          noise_mechanism: 'Laplace',
        });
      }).not.toThrow();
    });

    it('should accept valid Gaussian params', () => {
      expect(() => {
        PrivacyBudgetManager.validateParams({
          epsilon: 0.5,
          delta: 1e-6,
          sensitivity_bound: 1.0,
          noise_mechanism: 'Gaussian',
        });
      }).not.toThrow();
    });

    it('should reject Gaussian without delta', () => {
      expect(() => {
        PrivacyBudgetManager.validateParams({
          epsilon: 0.5,
          sensitivity_bound: 1.0,
          noise_mechanism: 'Gaussian',
        });
      }).toThrow('Delta is required');
    });

    it('should reject delta >= 1', () => {
      expect(() => {
        PrivacyBudgetManager.validateParams({
          epsilon: 0.5,
          delta: 1.0,
          sensitivity_bound: 1.0,
          noise_mechanism: 'Gaussian',
        });
      }).toThrow('range [0, 1)');
    });

    it('should reject delta exceeding max', () => {
      expect(() => {
        PrivacyBudgetManager.validateParams({
          epsilon: 0.5,
          delta: 0.1, // > 0.01 max
          sensitivity_bound: 1.0,
          noise_mechanism: 'Gaussian',
        });
      }).toThrow('exceeds recommended maximum');
    });

    it('should reject zero sensitivity', () => {
      expect(() => {
        PrivacyBudgetManager.validateParams({
          epsilon: 0.5,
          sensitivity_bound: 0,
          noise_mechanism: 'Laplace',
        });
      }).toThrow('Sensitivity must be positive');
    });
  });

  describe('estimateRemainingQueries', () => {
    const status: PrivacyBudgetStatus = {
      total: 10.0,
      consumed: 2.0,
      remaining: 8.0,
      queriesAnswered: 4,
      isExhausted: false,
      percentRemaining: 80,
    };

    it('should estimate remaining queries correctly', () => {
      expect(PrivacyBudgetManager.estimateRemainingQueries(status, 0.5)).toBe(16);
      expect(PrivacyBudgetManager.estimateRemainingQueries(status, 1.0)).toBe(8);
      expect(PrivacyBudgetManager.estimateRemainingQueries(status, 2.0)).toBe(4);
    });

    it('should return 0 for exhausted budget', () => {
      const exhausted: PrivacyBudgetStatus = { ...status, isExhausted: true, remaining: 0 };
      expect(PrivacyBudgetManager.estimateRemainingQueries(exhausted, 0.5)).toBe(0);
    });

    it('should return 0 for invalid epsilon', () => {
      expect(PrivacyBudgetManager.estimateRemainingQueries(status, 0)).toBe(0);
      expect(PrivacyBudgetManager.estimateRemainingQueries(status, -1)).toBe(0);
    });
  });

  describe('calculateOptimalEpsilon', () => {
    const status: PrivacyBudgetStatus = {
      total: 10.0,
      consumed: 0.0,
      remaining: 10.0,
      queriesAnswered: 0,
      isExhausted: false,
      percentRemaining: 100,
    };

    it('should calculate optimal epsilon for desired queries', () => {
      expect(PrivacyBudgetManager.calculateOptimalEpsilon(status, 10)).toBe(1.0);
      expect(PrivacyBudgetManager.calculateOptimalEpsilon(status, 20)).toBe(0.5);
      expect(PrivacyBudgetManager.calculateOptimalEpsilon(status, 100)).toBe(0.1);
    });

    it('should clamp to max epsilon', () => {
      const result = PrivacyBudgetManager.calculateOptimalEpsilon(status, 1);
      expect(result).toBe(EPSILON_LIMITS.MAX_PER_QUERY);
    });

    it('should return 0 for exhausted budget', () => {
      const exhausted: PrivacyBudgetStatus = { ...status, isExhausted: true };
      expect(PrivacyBudgetManager.calculateOptimalEpsilon(exhausted, 10)).toBe(0);
    });

    it('should return 0 for invalid query count', () => {
      expect(PrivacyBudgetManager.calculateOptimalEpsilon(status, 0)).toBe(0);
      expect(PrivacyBudgetManager.calculateOptimalEpsilon(status, -5)).toBe(0);
    });
  });

  describe('getDisplayInfo', () => {
    it('should return healthy for high budget', () => {
      const status: PrivacyBudgetStatus = {
        total: 10.0,
        consumed: 2.0,
        remaining: 8.0,
        queriesAnswered: 4,
        isExhausted: false,
        percentRemaining: 80,
      };

      const display = PrivacyBudgetManager.getDisplayInfo(status);

      expect(display.severity).toBe('healthy');
      expect(display.canQuery).toBe(true);
      expect(display.remaining).toBe('8.0000');
    });

    it('should return caution for moderate budget', () => {
      const status: PrivacyBudgetStatus = {
        total: 10.0,
        consumed: 6.0,
        remaining: 4.0,
        queriesAnswered: 12,
        isExhausted: false,
        percentRemaining: 40,
      };

      const display = PrivacyBudgetManager.getDisplayInfo(status);

      expect(display.severity).toBe('caution');
      expect(display.canQuery).toBe(true);
    });

    it('should return warning for low budget', () => {
      const status: PrivacyBudgetStatus = {
        total: 10.0,
        consumed: 8.0,
        remaining: 2.0,
        queriesAnswered: 16,
        isExhausted: false,
        percentRemaining: 20,
      };

      const display = PrivacyBudgetManager.getDisplayInfo(status);

      expect(display.severity).toBe('warning');
      expect(display.canQuery).toBe(true);
    });

    it('should return critical for very low budget', () => {
      const status: PrivacyBudgetStatus = {
        total: 10.0,
        consumed: 9.5,
        remaining: 0.5,
        queriesAnswered: 19,
        isExhausted: false,
        percentRemaining: 5,
      };

      const display = PrivacyBudgetManager.getDisplayInfo(status);

      expect(display.severity).toBe('critical');
      expect(display.canQuery).toBe(true);
    });

    it('should return critical for exhausted budget', () => {
      const status: PrivacyBudgetStatus = {
        total: 10.0,
        consumed: 10.0,
        remaining: 0,
        queriesAnswered: 20,
        isExhausted: true,
        percentRemaining: 0,
      };

      const display = PrivacyBudgetManager.getDisplayInfo(status);

      expect(display.severity).toBe('critical');
      expect(display.canQuery).toBe(false);
      expect(display.message).toContain('exhausted');
    });
  });

  describe('simulateConsumption', () => {
    const status: PrivacyBudgetStatus = {
      total: 10.0,
      consumed: 0.0,
      remaining: 10.0,
      queriesAnswered: 0,
      isExhausted: false,
      percentRemaining: 100,
    };

    it('should simulate consumption correctly', () => {
      const simulated = PrivacyBudgetManager.simulateConsumption(status, [1.0, 1.0, 1.0]);

      expect(simulated.consumed).toBe(3.0);
      expect(simulated.remaining).toBe(7.0);
      expect(simulated.queriesAnswered).toBe(3);
      expect(simulated.percentRemaining).toBe(70);
    });

    it('should stop when budget exhausted', () => {
      const simulated = PrivacyBudgetManager.simulateConsumption(status, [5.0, 5.0, 5.0]);

      // Only 2 queries fit (5.0 + 5.0 = 10.0)
      expect(simulated.queriesAnswered).toBe(2);
      expect(simulated.remaining).toBe(0);
      expect(simulated.isExhausted).toBe(true);
    });

    it('should skip invalid epsilon values', () => {
      const simulated = PrivacyBudgetManager.simulateConsumption(status, [1.0, -1.0, 0, 1.0]);

      expect(simulated.queriesAnswered).toBe(2);
      expect(simulated.consumed).toBe(2.0);
    });

    it('should handle empty query list', () => {
      const simulated = PrivacyBudgetManager.simulateConsumption(status, []);

      expect(simulated.consumed).toBe(0);
      expect(simulated.queriesAnswered).toBe(0);
    });
  });

  describe('RECOMMENDED_EPSILON constants', () => {
    it('should have ascending values for decreasing sensitivity', () => {
      expect(RECOMMENDED_EPSILON.VERY_HIGH_SENSITIVITY).toBe(0.1);
      expect(RECOMMENDED_EPSILON.HIGH_SENSITIVITY).toBe(0.5);
      expect(RECOMMENDED_EPSILON.MODERATE_SENSITIVITY).toBe(1.0);
      expect(RECOMMENDED_EPSILON.LOW_SENSITIVITY).toBe(2.0);
      expect(RECOMMENDED_EPSILON.MINIMAL_SENSITIVITY).toBe(5.0);

      // Verify ordering
      expect(RECOMMENDED_EPSILON.VERY_HIGH_SENSITIVITY).toBeLessThan(RECOMMENDED_EPSILON.HIGH_SENSITIVITY);
      expect(RECOMMENDED_EPSILON.HIGH_SENSITIVITY).toBeLessThan(RECOMMENDED_EPSILON.MODERATE_SENSITIVITY);
      expect(RECOMMENDED_EPSILON.MODERATE_SENSITIVITY).toBeLessThan(RECOMMENDED_EPSILON.LOW_SENSITIVITY);
      expect(RECOMMENDED_EPSILON.LOW_SENSITIVITY).toBeLessThan(RECOMMENDED_EPSILON.MINIMAL_SENSITIVITY);
    });
  });

  describe('EPSILON_LIMITS constants', () => {
    it('should have sensible limits', () => {
      expect(EPSILON_LIMITS.MAX_PER_QUERY).toBe(10.0);
      expect(EPSILON_LIMITS.MIN_VALUE).toBe(1e-10);
      expect(EPSILON_LIMITS.MAX_DELTA).toBe(0.01);
    });
  });
});
