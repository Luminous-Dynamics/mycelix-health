import { describe, it, expect } from 'vitest';
import {
  HealthSdkError,
  HealthSdkErrorCode,
  DEFAULT_CONFIG,
} from '../src/types';

describe('HealthSdkError', () => {
  it('should create error with code and message', () => {
    const error = new HealthSdkError(
      HealthSdkErrorCode.BUDGET_EXHAUSTED,
      'Privacy budget exhausted'
    );

    expect(error.code).toBe(HealthSdkErrorCode.BUDGET_EXHAUSTED);
    expect(error.message).toBe('Privacy budget exhausted');
    expect(error.name).toBe('HealthSdkError');
    expect(error.details).toBeUndefined();
  });

  it('should create error with details', () => {
    const error = new HealthSdkError(
      HealthSdkErrorCode.INVALID_EPSILON,
      'Epsilon too large',
      { provided: 15.0, maximum: 10.0 }
    );

    expect(error.details).toEqual({ provided: 15.0, maximum: 10.0 });
  });

  it('should serialize to JSON correctly', () => {
    const error = new HealthSdkError(
      HealthSdkErrorCode.UNAUTHORIZED,
      'No consent',
      { action: 'read' }
    );

    const json = error.toJSON();

    expect(json).toEqual({
      name: 'HealthSdkError',
      code: 'UNAUTHORIZED',
      message: 'No consent',
      details: { action: 'read' },
    });
  });

  it('should be instanceof Error', () => {
    const error = new HealthSdkError(
      HealthSdkErrorCode.UNKNOWN,
      'Something went wrong'
    );

    expect(error).toBeInstanceOf(Error);
    expect(error).toBeInstanceOf(HealthSdkError);
  });
});

describe('HealthSdkErrorCode', () => {
  it('should have all expected error codes', () => {
    // Privacy errors
    expect(HealthSdkErrorCode.BUDGET_EXHAUSTED).toBe('BUDGET_EXHAUSTED');
    expect(HealthSdkErrorCode.INVALID_EPSILON).toBe('INVALID_EPSILON');
    expect(HealthSdkErrorCode.INVALID_DELTA).toBe('INVALID_DELTA');
    expect(HealthSdkErrorCode.INVALID_SENSITIVITY).toBe('INVALID_SENSITIVITY');

    // Authorization errors
    expect(HealthSdkErrorCode.UNAUTHORIZED).toBe('UNAUTHORIZED');
    expect(HealthSdkErrorCode.CONSENT_EXPIRED).toBe('CONSENT_EXPIRED');
    expect(HealthSdkErrorCode.CONSENT_REVOKED).toBe('CONSENT_REVOKED');

    // Network errors
    expect(HealthSdkErrorCode.CONNECTION_FAILED).toBe('CONNECTION_FAILED');
    expect(HealthSdkErrorCode.ZOME_CALL_FAILED).toBe('ZOME_CALL_FAILED');
    expect(HealthSdkErrorCode.TIMEOUT).toBe('TIMEOUT');

    // Validation errors
    expect(HealthSdkErrorCode.INVALID_INPUT).toBe('INVALID_INPUT');
    expect(HealthSdkErrorCode.VALIDATION_FAILED).toBe('VALIDATION_FAILED');

    // General errors
    expect(HealthSdkErrorCode.UNKNOWN).toBe('UNKNOWN');
  });
});

describe('DEFAULT_CONFIG', () => {
  it('should have sensible defaults', () => {
    expect(DEFAULT_CONFIG.appId).toBe('mycelix-health');
    expect(DEFAULT_CONFIG.roleName).toBe('mycelix_health');
    expect(DEFAULT_CONFIG.url).toBe('ws://localhost:8888');
    expect(DEFAULT_CONFIG.debug).toBe(false);
  });

  it('should have retry configuration', () => {
    expect(DEFAULT_CONFIG.retry).toBeDefined();
    expect(DEFAULT_CONFIG.retry.maxAttempts).toBe(3);
    expect(DEFAULT_CONFIG.retry.delayMs).toBe(1000);
    expect(DEFAULT_CONFIG.retry.backoffMultiplier).toBe(2);
  });
});
