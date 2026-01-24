/**
 * Token Manager Tests
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { TokenManager, type TokenInfo } from '../src/auth/token-manager.js';

describe('TokenManager', () => {
  let tokenManager: TokenManager;

  beforeEach(() => {
    tokenManager = new TokenManager(5); // 5 minute refresh threshold
  });

  describe('storeToken', () => {
    it('stores a token successfully', () => {
      const tokenInfo: TokenInfo = {
        accessToken: 'test-access-token',
        refreshToken: 'test-refresh-token',
        tokenType: 'Bearer',
        expiresAt: new Date(Date.now() + 3600 * 1000), // 1 hour
        scope: ['patient/*.read'],
        ehrSystem: 'epic',
      };

      tokenManager.storeToken('patient-123', tokenInfo);
      const retrieved = tokenManager.getToken('patient-123');

      expect(retrieved).not.toBeNull();
      expect(retrieved?.accessToken).toBe('test-access-token');
    });
  });

  describe('getToken', () => {
    it('returns null for non-existent key', () => {
      expect(tokenManager.getToken('non-existent')).toBeNull();
    });

    it('returns null for expired token', () => {
      const expiredToken: TokenInfo = {
        accessToken: 'expired-token',
        tokenType: 'Bearer',
        expiresAt: new Date(Date.now() - 1000), // 1 second ago
        scope: [],
        ehrSystem: 'epic',
      };

      tokenManager.storeToken('expired-key', expiredToken);
      expect(tokenManager.getToken('expired-key')).toBeNull();
    });

    it('returns valid token', () => {
      const validToken: TokenInfo = {
        accessToken: 'valid-token',
        tokenType: 'Bearer',
        expiresAt: new Date(Date.now() + 3600 * 1000),
        scope: [],
        ehrSystem: 'cerner',
      };

      tokenManager.storeToken('valid-key', validToken);
      const retrieved = tokenManager.getToken('valid-key');

      expect(retrieved).not.toBeNull();
      expect(retrieved?.accessToken).toBe('valid-token');
    });
  });

  describe('needsRefresh', () => {
    it('returns false for token with plenty of time', () => {
      const token: TokenInfo = {
        accessToken: 'token',
        tokenType: 'Bearer',
        expiresAt: new Date(Date.now() + 3600 * 1000), // 1 hour
        scope: [],
        ehrSystem: 'epic',
      };

      tokenManager.storeToken('key', token);
      expect(tokenManager.needsRefresh('key')).toBe(false);
    });

    it('returns true for token near expiration', () => {
      const token: TokenInfo = {
        accessToken: 'token',
        tokenType: 'Bearer',
        expiresAt: new Date(Date.now() + 2 * 60 * 1000), // 2 minutes
        scope: [],
        ehrSystem: 'epic',
      };

      tokenManager.storeToken('key', token);
      expect(tokenManager.needsRefresh('key')).toBe(true); // Within 5 min threshold
    });

    it('returns false for non-existent key', () => {
      expect(tokenManager.needsRefresh('non-existent')).toBe(false);
    });
  });

  describe('isExpired', () => {
    it('returns true for expired token', () => {
      const expiredToken: TokenInfo = {
        accessToken: 'token',
        tokenType: 'Bearer',
        expiresAt: new Date(Date.now() - 1000),
        scope: [],
        ehrSystem: 'epic',
      };

      expect(tokenManager.isExpired(expiredToken)).toBe(true);
    });

    it('returns false for valid token', () => {
      const validToken: TokenInfo = {
        accessToken: 'token',
        tokenType: 'Bearer',
        expiresAt: new Date(Date.now() + 3600 * 1000),
        scope: [],
        ehrSystem: 'epic',
      };

      expect(tokenManager.isExpired(validToken)).toBe(false);
    });
  });

  describe('removeToken', () => {
    it('removes an existing token', () => {
      const token: TokenInfo = {
        accessToken: 'token',
        tokenType: 'Bearer',
        expiresAt: new Date(Date.now() + 3600 * 1000),
        scope: [],
        ehrSystem: 'epic',
      };

      tokenManager.storeToken('key', token);
      expect(tokenManager.removeToken('key')).toBe(true);
      expect(tokenManager.getToken('key')).toBeNull();
    });

    it('returns false for non-existent key', () => {
      expect(tokenManager.removeToken('non-existent')).toBe(false);
    });
  });

  describe('clearTokensForSystem', () => {
    it('clears all tokens for a specific EHR system', () => {
      const epicToken: TokenInfo = {
        accessToken: 'epic-token',
        tokenType: 'Bearer',
        expiresAt: new Date(Date.now() + 3600 * 1000),
        scope: [],
        ehrSystem: 'epic',
      };

      const cernerToken: TokenInfo = {
        accessToken: 'cerner-token',
        tokenType: 'Bearer',
        expiresAt: new Date(Date.now() + 3600 * 1000),
        scope: [],
        ehrSystem: 'cerner',
      };

      tokenManager.storeToken('epic-1', epicToken);
      tokenManager.storeToken('epic-2', { ...epicToken, accessToken: 'epic-token-2' });
      tokenManager.storeToken('cerner-1', cernerToken);

      tokenManager.clearTokensForSystem('epic');

      expect(tokenManager.getToken('epic-1')).toBeNull();
      expect(tokenManager.getToken('epic-2')).toBeNull();
      expect(tokenManager.getToken('cerner-1')).not.toBeNull();
    });
  });

  describe('parseScopes', () => {
    it('parses space-separated scope string', () => {
      const scopes = tokenManager.parseScopes('patient/*.read patient/*.write launch/patient');
      expect(scopes).toEqual(['patient/*.read', 'patient/*.write', 'launch/patient']);
    });

    it('handles empty string', () => {
      const scopes = tokenManager.parseScopes('');
      expect(scopes).toEqual([]);
    });
  });

  describe('hasScope', () => {
    it('returns true when scope exists', () => {
      const token: TokenInfo = {
        accessToken: 'token',
        tokenType: 'Bearer',
        expiresAt: new Date(Date.now() + 3600 * 1000),
        scope: ['patient/*.read', 'patient/*.write'],
        ehrSystem: 'epic',
      };

      expect(tokenManager.hasScope(token, 'patient/*.read')).toBe(true);
    });

    it('returns false when scope does not exist', () => {
      const token: TokenInfo = {
        accessToken: 'token',
        tokenType: 'Bearer',
        expiresAt: new Date(Date.now() + 3600 * 1000),
        scope: ['patient/*.read'],
        ehrSystem: 'epic',
      };

      expect(tokenManager.hasScope(token, 'patient/*.write')).toBe(false);
    });
  });

  describe('hasAllScopes', () => {
    it('returns true when all scopes exist', () => {
      const token: TokenInfo = {
        accessToken: 'token',
        tokenType: 'Bearer',
        expiresAt: new Date(Date.now() + 3600 * 1000),
        scope: ['patient/*.read', 'patient/*.write', 'launch/patient'],
        ehrSystem: 'epic',
      };

      expect(tokenManager.hasAllScopes(token, ['patient/*.read', 'launch/patient'])).toBe(true);
    });

    it('returns false when some scopes are missing', () => {
      const token: TokenInfo = {
        accessToken: 'token',
        tokenType: 'Bearer',
        expiresAt: new Date(Date.now() + 3600 * 1000),
        scope: ['patient/*.read'],
        ehrSystem: 'epic',
      };

      expect(tokenManager.hasAllScopes(token, ['patient/*.read', 'patient/*.write'])).toBe(false);
    });
  });

  describe('createTokenInfo', () => {
    it('creates TokenInfo from OAuth response', () => {
      const response = {
        access_token: 'new-access-token',
        refresh_token: 'new-refresh-token',
        token_type: 'Bearer',
        expires_in: 3600,
        scope: 'patient/*.read patient/*.write',
        patient: 'patient-456',
      };

      const tokenInfo = tokenManager.createTokenInfo('epic', response);

      expect(tokenInfo.accessToken).toBe('new-access-token');
      expect(tokenInfo.refreshToken).toBe('new-refresh-token');
      expect(tokenInfo.tokenType).toBe('Bearer');
      expect(tokenInfo.scope).toEqual(['patient/*.read', 'patient/*.write']);
      expect(tokenInfo.patientId).toBe('patient-456');
      expect(tokenInfo.ehrSystem).toBe('epic');
      expect(tokenInfo.expiresAt.getTime()).toBeGreaterThan(Date.now());
    });

    it('handles missing optional fields with defaults', () => {
      const response = {
        access_token: 'token',
      };

      const tokenInfo = tokenManager.createTokenInfo('cerner', response);

      expect(tokenInfo.accessToken).toBe('token');
      expect(tokenInfo.tokenType).toBe('Bearer'); // Default
      expect(tokenInfo.scope).toEqual([]); // Empty scope
      expect(tokenInfo.expiresAt.getTime()).toBeGreaterThan(Date.now()); // Default 1 hour
    });
  });

  describe('clearExpiredTokens', () => {
    it('removes all expired tokens and returns count', () => {
      const expiredToken: TokenInfo = {
        accessToken: 'expired',
        tokenType: 'Bearer',
        expiresAt: new Date(Date.now() - 1000),
        scope: [],
        ehrSystem: 'epic',
      };

      const validToken: TokenInfo = {
        accessToken: 'valid',
        tokenType: 'Bearer',
        expiresAt: new Date(Date.now() + 3600 * 1000),
        scope: [],
        ehrSystem: 'cerner',
      };

      tokenManager.storeToken('expired-1', expiredToken);
      tokenManager.storeToken('expired-2', { ...expiredToken, accessToken: 'expired-2' });
      tokenManager.storeToken('valid-1', validToken);

      const cleared = tokenManager.clearExpiredTokens();

      expect(cleared).toBe(2);
      expect(tokenManager.getActiveTokenCount()).toBe(1);
    });
  });
});
