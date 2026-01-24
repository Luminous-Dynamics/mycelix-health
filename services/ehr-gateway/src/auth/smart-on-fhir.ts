/**
 * SMART on FHIR Authentication
 *
 * Implements the SMART App Launch Framework for OAuth2-based
 * authentication with EHR systems.
 */

import * as jose from 'jose';
import { TokenManager, type TokenInfo } from './token-manager.js';
import type { EhrSystem } from '../types.js';

export interface SmartAuthConfig {
  clientId: string;
  clientSecret?: string;
  redirectUri: string;
  scopes: string[];
  usesPKCE?: boolean;
  usesPrivateKeyJwt?: boolean;
  privateKey?: jose.KeyLike;
}

export interface SmartMetadata {
  authorizationEndpoint: string;
  tokenEndpoint: string;
  introspectionEndpoint?: string;
  revocationEndpoint?: string;
  capabilities: string[];
  codeChallengeMethodsSupported?: string[];
  tokenEndpointAuthMethodsSupported?: string[];
}

interface AuthorizationState {
  state: string;
  codeVerifier?: string;
  ehrSystem: EhrSystem;
  redirectUri: string;
  createdAt: number;
}

export class SmartOnFhirAuth {
  private config: SmartAuthConfig;
  private tokenManager: TokenManager;
  private pendingAuthorizations: Map<string, AuthorizationState> = new Map();
  private metadataCache: Map<string, SmartMetadata> = new Map();

  constructor(config: SmartAuthConfig, tokenManager?: TokenManager) {
    this.config = config;
    this.tokenManager = tokenManager || new TokenManager();
  }

  /**
   * Discover SMART configuration from a FHIR server
   */
  async discoverMetadata(fhirBaseUrl: string): Promise<SmartMetadata> {
    const cached = this.metadataCache.get(fhirBaseUrl);
    if (cached) {
      return cached;
    }

    // Try .well-known/smart-configuration first (SMART App Launch 2.0)
    let metadata: SmartMetadata | null = null;

    try {
      const smartConfigUrl = `${fhirBaseUrl}/.well-known/smart-configuration`;
      const response = await fetch(smartConfigUrl);
      if (response.ok) {
        const data = await response.json() as {
          authorization_endpoint: string;
          token_endpoint: string;
          introspection_endpoint?: string;
          revocation_endpoint?: string;
          capabilities?: string[];
          code_challenge_methods_supported?: string[];
          token_endpoint_auth_methods_supported?: string[];
        };
        metadata = {
          authorizationEndpoint: data.authorization_endpoint,
          tokenEndpoint: data.token_endpoint,
          introspectionEndpoint: data.introspection_endpoint,
          revocationEndpoint: data.revocation_endpoint,
          capabilities: data.capabilities || [],
          codeChallengeMethodsSupported: data.code_challenge_methods_supported,
          tokenEndpointAuthMethodsSupported: data.token_endpoint_auth_methods_supported,
        };
      }
    } catch {
      // Fall through to capability statement
    }

    // Fall back to metadata endpoint in CapabilityStatement
    if (!metadata) {
      const capabilityUrl = `${fhirBaseUrl}/metadata`;
      const response = await fetch(capabilityUrl, {
        headers: { Accept: 'application/fhir+json' },
      });

      if (!response.ok) {
        throw new Error(`Failed to fetch FHIR metadata: ${response.status}`);
      }

      const capability = await response.json() as {
        rest?: Array<{
          security?: {
            extension?: Array<{
              url: string;
              extension?: Array<{ url: string; valueUri?: string }>;
            }>;
          };
        }>;
      };
      const security = capability.rest?.[0]?.security;
      const oauth = security?.extension?.find(
        (ext: { url: string }) => ext.url === 'http://fhir-registry.smarthealthit.org/StructureDefinition/oauth-uris'
      );

      if (!oauth) {
        throw new Error('No OAuth endpoints found in CapabilityStatement');
      }

      const getUri = (name: string) =>
        oauth.extension?.find((e: { url: string }) => e.url === name)?.valueUri;

      const authEndpoint = getUri('authorize');
      const tokenEndpoint = getUri('token');

      if (!authEndpoint || !tokenEndpoint) {
        throw new Error('Missing required OAuth endpoints in CapabilityStatement');
      }

      metadata = {
        authorizationEndpoint: authEndpoint,
        tokenEndpoint: tokenEndpoint,
        introspectionEndpoint: getUri('introspect'),
        revocationEndpoint: getUri('revoke'),
        capabilities: [],
      };
    }

    if (!metadata || !metadata.authorizationEndpoint || !metadata.tokenEndpoint) {
      throw new Error('Missing required OAuth endpoints');
    }

    this.metadataCache.set(fhirBaseUrl, metadata);
    return metadata;
  }

  /**
   * Generate PKCE code verifier and challenge
   */
  private async generatePKCE(): Promise<{ verifier: string; challenge: string }> {
    const verifier = this.generateRandomString(64);
    const encoder = new TextEncoder();
    const data = encoder.encode(verifier);
    const digest = await crypto.subtle.digest('SHA-256', data);
    const challenge = this.base64UrlEncode(new Uint8Array(digest));
    return { verifier, challenge };
  }

  /**
   * Generate a random string for state/nonce
   */
  private generateRandomString(length: number): string {
    const array = new Uint8Array(length);
    crypto.getRandomValues(array);
    return this.base64UrlEncode(array);
  }

  /**
   * Base64 URL encode
   */
  private base64UrlEncode(buffer: Uint8Array): string {
    const base64 = btoa(String.fromCharCode(...buffer));
    return base64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
  }

  /**
   * Build the authorization URL for the initial OAuth2 redirect
   */
  async buildAuthorizationUrl(
    fhirBaseUrl: string,
    ehrSystem: EhrSystem,
    launchContext?: string
  ): Promise<string> {
    const metadata = await this.discoverMetadata(fhirBaseUrl);
    const state = this.generateRandomString(32);

    const params = new URLSearchParams({
      response_type: 'code',
      client_id: this.config.clientId,
      redirect_uri: this.config.redirectUri,
      scope: this.config.scopes.join(' '),
      state,
      aud: fhirBaseUrl,
    });

    if (launchContext) {
      params.set('launch', launchContext);
    }

    const authState: AuthorizationState = {
      state,
      ehrSystem,
      redirectUri: this.config.redirectUri,
      createdAt: Date.now(),
    };

    // Add PKCE if configured
    if (this.config.usesPKCE) {
      const pkce = await this.generatePKCE();
      params.set('code_challenge', pkce.challenge);
      params.set('code_challenge_method', 'S256');
      authState.codeVerifier = pkce.verifier;
    }

    this.pendingAuthorizations.set(state, authState);

    // Clean up old pending authorizations (older than 10 minutes)
    const cutoff = Date.now() - 10 * 60 * 1000;
    for (const [key, value] of this.pendingAuthorizations.entries()) {
      if (value.createdAt < cutoff) {
        this.pendingAuthorizations.delete(key);
      }
    }

    return `${metadata.authorizationEndpoint}?${params.toString()}`;
  }

  /**
   * Exchange authorization code for tokens
   */
  async exchangeCodeForTokens(
    fhirBaseUrl: string,
    code: string,
    state: string
  ): Promise<TokenInfo> {
    const authState = this.pendingAuthorizations.get(state);
    if (!authState) {
      throw new Error('Invalid or expired authorization state');
    }

    this.pendingAuthorizations.delete(state);
    const metadata = await this.discoverMetadata(fhirBaseUrl);

    const params = new URLSearchParams({
      grant_type: 'authorization_code',
      code,
      redirect_uri: authState.redirectUri,
      client_id: this.config.clientId,
    });

    if (authState.codeVerifier) {
      params.set('code_verifier', authState.codeVerifier);
    }

    const headers: Record<string, string> = {
      'Content-Type': 'application/x-www-form-urlencoded',
    };

    // Add client authentication
    if (this.config.usesPrivateKeyJwt && this.config.privateKey) {
      const assertion = await this.createClientAssertion(metadata.tokenEndpoint);
      params.set('client_assertion_type', 'urn:ietf:params:oauth:client-assertion-type:jwt-bearer');
      params.set('client_assertion', assertion);
    } else if (this.config.clientSecret) {
      const credentials = btoa(`${this.config.clientId}:${this.config.clientSecret}`);
      headers['Authorization'] = `Basic ${credentials}`;
    }

    const response = await fetch(metadata.tokenEndpoint, {
      method: 'POST',
      headers,
      body: params.toString(),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Token exchange failed: ${response.status} - ${error}`);
    }

    const tokenResponse = await response.json() as {
      access_token: string;
      token_type?: string;
      expires_in?: number;
      refresh_token?: string;
      scope?: string;
      patient?: string;
    };
    const tokenInfo = this.tokenManager.createTokenInfo(authState.ehrSystem, tokenResponse);
    this.tokenManager.storeToken(tokenInfo.patientId || state, tokenInfo);

    return tokenInfo;
  }

  /**
   * Refresh an access token
   */
  async refreshToken(
    fhirBaseUrl: string,
    tokenInfo: TokenInfo
  ): Promise<TokenInfo> {
    if (!tokenInfo.refreshToken) {
      throw new Error('No refresh token available');
    }

    const metadata = await this.discoverMetadata(fhirBaseUrl);

    const params = new URLSearchParams({
      grant_type: 'refresh_token',
      refresh_token: tokenInfo.refreshToken,
      client_id: this.config.clientId,
    });

    const headers: Record<string, string> = {
      'Content-Type': 'application/x-www-form-urlencoded',
    };

    if (this.config.usesPrivateKeyJwt && this.config.privateKey) {
      const assertion = await this.createClientAssertion(metadata.tokenEndpoint);
      params.set('client_assertion_type', 'urn:ietf:params:oauth:client-assertion-type:jwt-bearer');
      params.set('client_assertion', assertion);
    } else if (this.config.clientSecret) {
      const credentials = btoa(`${this.config.clientId}:${this.config.clientSecret}`);
      headers['Authorization'] = `Basic ${credentials}`;
    }

    const response = await fetch(metadata.tokenEndpoint, {
      method: 'POST',
      headers,
      body: params.toString(),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Token refresh failed: ${response.status} - ${error}`);
    }

    const tokenResponse = await response.json() as {
      access_token: string;
      token_type?: string;
      expires_in?: number;
      refresh_token?: string;
      scope?: string;
      patient?: string;
    };
    const newTokenInfo = this.tokenManager.createTokenInfo(tokenInfo.ehrSystem, tokenResponse);

    // Preserve refresh token if not returned
    if (!newTokenInfo.refreshToken && tokenInfo.refreshToken) {
      newTokenInfo.refreshToken = tokenInfo.refreshToken;
    }

    this.tokenManager.storeToken(newTokenInfo.patientId || 'default', newTokenInfo);
    return newTokenInfo;
  }

  /**
   * Create a JWT client assertion for private_key_jwt auth
   */
  private async createClientAssertion(tokenEndpoint: string): Promise<string> {
    if (!this.config.privateKey) {
      throw new Error('Private key not configured');
    }

    const now = Math.floor(Date.now() / 1000);

    const jwt = await new jose.SignJWT({})
      .setProtectedHeader({ alg: 'RS384', typ: 'JWT' })
      .setIssuer(this.config.clientId)
      .setSubject(this.config.clientId)
      .setAudience(tokenEndpoint)
      .setIssuedAt(now)
      .setExpirationTime(now + 300)
      .setJti(this.generateRandomString(16))
      .sign(this.config.privateKey);

    return jwt;
  }

  /**
   * Revoke a token
   */
  async revokeToken(fhirBaseUrl: string, token: string): Promise<void> {
    const metadata = await this.discoverMetadata(fhirBaseUrl);

    if (!metadata.revocationEndpoint) {
      throw new Error('Token revocation not supported');
    }

    const params = new URLSearchParams({
      token,
      client_id: this.config.clientId,
    });

    const response = await fetch(metadata.revocationEndpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
      body: params.toString(),
    });

    if (!response.ok) {
      throw new Error(`Token revocation failed: ${response.status}`);
    }
  }

  /**
   * Get the token manager
   */
  getTokenManager(): TokenManager {
    return this.tokenManager;
  }
}
