/**
 * Token Manager for EHR Gateway
 *
 * Handles OAuth2 token storage, refresh, and lifecycle management
 * for SMART on FHIR authentication flows.
 */

import * as jose from 'jose';

export interface TokenInfo {
  accessToken: string;
  refreshToken?: string;
  tokenType: string;
  expiresAt: Date;
  scope: string[];
  patientId?: string;
  ehrSystem: string;
}

interface StoredToken {
  info: TokenInfo;
  createdAt: number;
}

export class TokenManager {
  private tokens: Map<string, StoredToken> = new Map();
  private refreshThresholdMs: number;

  constructor(refreshThresholdMinutes = 5) {
    this.refreshThresholdMs = refreshThresholdMinutes * 60 * 1000;
  }

  /**
   * Store a token for a given key (typically patient or session ID)
   */
  storeToken(key: string, tokenInfo: TokenInfo): void {
    this.tokens.set(key, {
      info: tokenInfo,
      createdAt: Date.now(),
    });
  }

  /**
   * Retrieve a token if it exists and is valid
   */
  getToken(key: string): TokenInfo | null {
    const stored = this.tokens.get(key);
    if (!stored) {
      return null;
    }

    if (this.isExpired(stored.info)) {
      this.tokens.delete(key);
      return null;
    }

    return stored.info;
  }

  /**
   * Check if a token needs refresh (approaching expiration)
   */
  needsRefresh(key: string): boolean {
    const stored = this.tokens.get(key);
    if (!stored) {
      return false;
    }

    const timeUntilExpiry = stored.info.expiresAt.getTime() - Date.now();
    return timeUntilExpiry < this.refreshThresholdMs;
  }

  /**
   * Check if a token is expired
   */
  isExpired(tokenInfo: TokenInfo): boolean {
    return tokenInfo.expiresAt.getTime() <= Date.now();
  }

  /**
   * Remove a token
   */
  removeToken(key: string): boolean {
    return this.tokens.delete(key);
  }

  /**
   * Clear all tokens for a specific EHR system
   */
  clearTokensForSystem(ehrSystem: string): void {
    for (const [key, stored] of this.tokens.entries()) {
      if (stored.info.ehrSystem === ehrSystem) {
        this.tokens.delete(key);
      }
    }
  }

  /**
   * Clear all expired tokens
   */
  clearExpiredTokens(): number {
    let cleared = 0;
    for (const [key, stored] of this.tokens.entries()) {
      if (this.isExpired(stored.info)) {
        this.tokens.delete(key);
        cleared++;
      }
    }
    return cleared;
  }

  /**
   * Parse and validate a JWT access token
   */
  async parseAccessToken(accessToken: string): Promise<jose.JWTPayload | null> {
    try {
      const decoded = jose.decodeJwt(accessToken);
      return decoded;
    } catch {
      return null;
    }
  }

  /**
   * Extract scopes from a token response
   */
  parseScopes(scopeString: string): string[] {
    return scopeString.split(' ').filter(s => s.length > 0);
  }

  /**
   * Check if a token has a specific scope
   */
  hasScope(tokenInfo: TokenInfo, requiredScope: string): boolean {
    return tokenInfo.scope.includes(requiredScope);
  }

  /**
   * Check if a token has all required scopes
   */
  hasAllScopes(tokenInfo: TokenInfo, requiredScopes: string[]): boolean {
    return requiredScopes.every(scope => tokenInfo.scope.includes(scope));
  }

  /**
   * Get all active tokens count
   */
  getActiveTokenCount(): number {
    this.clearExpiredTokens();
    return this.tokens.size;
  }

  /**
   * Create a token info object from OAuth2 token response
   */
  createTokenInfo(
    ehrSystem: string,
    response: {
      access_token: string;
      refresh_token?: string;
      token_type?: string;
      expires_in?: number;
      scope?: string;
      patient?: string;
    }
  ): TokenInfo {
    return {
      accessToken: response.access_token,
      refreshToken: response.refresh_token,
      tokenType: response.token_type || 'Bearer',
      expiresAt: new Date(Date.now() + (response.expires_in || 3600) * 1000),
      scope: this.parseScopes(response.scope || ''),
      patientId: response.patient,
      ehrSystem,
    };
  }
}
