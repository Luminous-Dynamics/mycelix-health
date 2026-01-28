/**
 * EHR Gateway
 *
 * Main orchestrator for EHR integration. Provides a unified interface
 * for connecting to and syncing with external EHR systems.
 */

import type { AppClient } from '@holochain/client';
import { SmartOnFhirAuth, type SmartAuthConfig } from './auth/smart-on-fhir.js';
import { TokenManager, type TokenInfo } from './auth/token-manager.js';
import { GenericFhirAdapter, type FhirAdapterConfig } from './adapters/generic-fhir.js';
import { EpicAdapter, type EpicAdapterConfig } from './adapters/epic.js';
import { CernerAdapter, type CernerAdapterConfig } from './adapters/cerner.js';
import { PullService, type PullConfig, type PullOptions } from './sync/pull-service.js';
import { PushService, type PushConfig, type PushOptions } from './sync/push-service.js';
import { ConflictResolver, type ConflictResolverConfig, type ConflictResolutionStrategy } from './sync/conflict-resolver.js';
import { EhrCacheManager, type EhrCacheConfig, type CacheStats } from './cache.js';
import type { EhrSystem, EhrEndpoint, SyncResult, SyncDirection, ConflictInfo } from './types.js';

export interface EhrGatewayConfig {
  holochainClient: AppClient;
  defaultTimeout?: number;
  maxRetries?: number;
  conflictStrategy?: ConflictResolutionStrategy;
  /** Cache configuration */
  cache?: EhrCacheConfig;
  /** Whether caching is enabled (default: true) */
  enableCache?: boolean;
}

export interface ConnectionConfig {
  endpoint: EhrEndpoint;
  authConfig: SmartAuthConfig;
  adapterConfig?: Partial<FhirAdapterConfig>;
}

interface ActiveConnection {
  system: EhrSystem;
  endpoint: EhrEndpoint;
  adapter: GenericFhirAdapter;
  auth: SmartOnFhirAuth;
  tokenManager: TokenManager;
  pullService: PullService;
  pushService: PushService;
}

export class EhrGateway {
  private config: EhrGatewayConfig;
  private connections: Map<string, ActiveConnection> = new Map();
  private conflictResolver: ConflictResolver;
  private cacheManager: EhrCacheManager | null;

  constructor(config: EhrGatewayConfig) {
    this.config = {
      defaultTimeout: 30000,
      maxRetries: 3,
      conflictStrategy: 'most_recent',
      enableCache: true,
      ...config,
    };

    this.conflictResolver = new ConflictResolver({
      defaultStrategy: this.config.conflictStrategy || 'most_recent',
      autoResolveThreshold: 0.1,
      mergeRules: {
        preferLocalFields: ['consent_status', 'privacy_settings'],
        preferRemoteFields: ['clinical_status', 'verification_status'],
      },
    });

    // Initialize cache manager if enabled
    this.cacheManager = this.config.enableCache
      ? new EhrCacheManager(this.config.cache)
      : null;
  }

  /**
   * Connect to an EHR system
   */
  async connect(connectionId: string, config: ConnectionConfig): Promise<void> {
    const { endpoint, authConfig, adapterConfig } = config;

    // Create token manager
    const tokenManager = new TokenManager();

    // Create auth handler
    const auth = new SmartOnFhirAuth(authConfig, tokenManager);

    // Create appropriate adapter based on EHR system
    const fullAdapterConfig: FhirAdapterConfig = {
      baseUrl: endpoint.baseUrl,
      timeout: this.config.defaultTimeout,
      maxRetries: this.config.maxRetries,
      ...adapterConfig,
    };

    let adapter: GenericFhirAdapter;

    switch (endpoint.system) {
      case 'epic':
        adapter = new EpicAdapter({
          ...fullAdapterConfig,
          epicClientId: authConfig.clientId,
        } as EpicAdapterConfig);
        break;

      case 'cerner':
        adapter = new CernerAdapter(fullAdapterConfig as CernerAdapterConfig);
        break;

      default:
        adapter = new GenericFhirAdapter(fullAdapterConfig);
    }

    // Create sync services
    const pullService = new PullService({
      holochainClient: this.config.holochainClient,
      fhirAdapter: adapter,
    });

    const pushService = new PushService({
      holochainClient: this.config.holochainClient,
      fhirAdapter: adapter,
    });

    // Store connection
    this.connections.set(connectionId, {
      system: endpoint.system,
      endpoint,
      adapter,
      auth,
      tokenManager,
      pullService,
      pushService,
    });
  }

  /**
   * Disconnect from an EHR system
   */
  disconnect(connectionId: string): void {
    // Revoke cached tokens for this connection
    if (this.cacheManager) {
      this.cacheManager.tokens.revoke(connectionId);
    }
    this.connections.delete(connectionId);
  }

  /**
   * Get authorization URL for SMART on FHIR flow
   */
  async getAuthorizationUrl(
    connectionId: string,
    launchContext?: string
  ): Promise<string> {
    const connection = this.getConnection(connectionId);
    return connection.auth.buildAuthorizationUrl(
      connection.endpoint.baseUrl,
      connection.system,
      launchContext
    );
  }

  /**
   * Complete authorization with code from callback
   */
  async completeAuthorization(
    connectionId: string,
    code: string,
    state: string
  ): Promise<TokenInfo> {
    const connection = this.getConnection(connectionId);
    const tokenInfo = await connection.auth.exchangeCodeForTokens(
      connection.endpoint.baseUrl,
      code,
      state
    );

    // Cache the token for faster subsequent access
    if (this.cacheManager && tokenInfo) {
      this.cacheManager.tokens.set(connectionId, {
        accessToken: tokenInfo.accessToken,
        refreshToken: tokenInfo.refreshToken,
        expiresAt: tokenInfo.expiresAt?.getTime(),
        scope: tokenInfo.scope?.join(' '),
        tokenType: tokenInfo.tokenType,
      });
    }

    return tokenInfo;
  }

  /**
   * Get active token for a connection
   *
   * Checks cache first for faster access, then falls back to token manager.
   * Note: Cache is checked using connectionId, token manager uses a separate key.
   */
  getToken(connectionId: string, key: string): TokenInfo | null {
    // Fall back to token manager (source of truth)
    const connection = this.getConnection(connectionId);
    return connection.tokenManager.getToken(key);
  }

  /**
   * Get cached token for quick access
   *
   * Returns the cached token if available and valid, or null.
   * Use this for quick checks without full token manager lookup.
   */
  getCachedToken(connectionId: string): { accessToken: string; expiresAt?: number } | null {
    if (!this.cacheManager) {
      return null;
    }

    const cached = this.cacheManager.tokens.get(connectionId);
    if (!cached) {
      return null;
    }

    return {
      accessToken: cached.accessToken,
      expiresAt: cached.expiresAt,
    };
  }

  /**
   * Pull patient data from EHR
   */
  async pullPatientData(
    connectionId: string,
    patientId: string,
    tokenKey: string,
    options: PullOptions = {}
  ): Promise<SyncResult[]> {
    const connection = this.getConnection(connectionId);
    const tokenInfo = connection.tokenManager.getToken(tokenKey);

    if (!tokenInfo) {
      throw new Error('No valid token found');
    }

    // Check if token needs refresh
    if (connection.tokenManager.needsRefresh(tokenKey)) {
      const refreshed = await connection.auth.refreshToken(
        connection.endpoint.baseUrl,
        tokenInfo
      );
      connection.tokenManager.storeToken(tokenKey, refreshed);
    }

    const result = await connection.pullService.pullPatientData(
      patientId,
      connection.tokenManager.getToken(tokenKey)!,
      options
    );

    // Return just the sync results for backwards compatibility
    return result.syncResults;
  }

  /**
   * Push patient data to EHR
   */
  async pushPatientData(
    connectionId: string,
    patientHash: Uint8Array,
    tokenKey: string,
    options: PushOptions = {}
  ): Promise<SyncResult[]> {
    const connection = this.getConnection(connectionId);
    const tokenInfo = connection.tokenManager.getToken(tokenKey);

    if (!tokenInfo) {
      throw new Error('No valid token found');
    }

    // Check if token needs refresh
    if (connection.tokenManager.needsRefresh(tokenKey)) {
      const refreshed = await connection.auth.refreshToken(
        connection.endpoint.baseUrl,
        tokenInfo
      );
      connection.tokenManager.storeToken(tokenKey, refreshed);
    }

    return connection.pushService.pushPatientData(
      patientHash,
      connection.tokenManager.getToken(tokenKey)!,
      options
    );
  }

  /**
   * Perform bidirectional sync
   */
  async syncPatient(
    connectionId: string,
    patientId: string,
    patientHash: Uint8Array,
    tokenKey: string,
    options: { pull?: PullOptions; push?: PushOptions } = {}
  ): Promise<{
    pullResults: SyncResult[];
    pushResults: SyncResult[];
    conflicts: ConflictInfo[];
  }> {
    // Pull first to get latest from EHR
    const pullResults = await this.pullPatientData(
      connectionId,
      patientId,
      tokenKey,
      options.pull
    );

    // Detect conflicts
    const conflicts: ConflictInfo[] = [];

    // For each pulled resource, check if there's a local change that conflicts
    for (const result of pullResults) {
      if (result.success) {
        const conflict = await this.detectConflict(
          connectionId,
          result.resourceType,
          result.resourceId
        );

        if (conflict) {
          conflicts.push(conflict);
          this.conflictResolver.registerConflict(conflict);
        }
      }
    }

    // Auto-resolve non-critical conflicts
    await this.conflictResolver.autoResolveConflicts('sync-service');

    // Push local changes (excluding conflicted resources)
    const conflictedIds = new Set(conflicts.map(c => c.resourceId));
    const pushOptions: PushOptions = {
      ...options.push,
      recordHashes: options.push?.recordHashes?.filter(
        h => !conflictedIds.has(h.toString())
      ),
    };

    const pushResults = await this.pushPatientData(
      connectionId,
      patientHash,
      tokenKey,
      pushOptions
    );

    return {
      pullResults,
      pushResults,
      conflicts,
    };
  }

  /**
   * Detect conflict for a resource
   */
  private async detectConflict(
    connectionId: string,
    resourceType: string,
    resourceId: string
  ): Promise<ConflictInfo | null> {
    // Get local and remote versions
    const localData = await this.getLocalResource(resourceType, resourceId);
    const remoteData = await this.getRemoteResource(connectionId, resourceType, resourceId);

    if (!localData || !remoteData) {
      return null;
    }

    return this.conflictResolver.detectConflict(
      resourceType,
      resourceId,
      localData.data,
      remoteData.data,
      localData.version,
      remoteData.version
    );
  }

  /**
   * Get local resource from Holochain
   */
  private async getLocalResource(
    resourceType: string,
    resourceId: string
  ): Promise<{ data: unknown; version: string } | null> {
    try {
      const result = await this.config.holochainClient.callZome({
        cap_secret: undefined,
        role_name: 'health',
        zome_name: 'fhir_mapping',
        fn_name: 'get_local_resource',
        payload: { resource_type: resourceType, resource_id: resourceId },
      });

      return result as { data: unknown; version: string };
    } catch {
      return null;
    }
  }

  /**
   * Get remote resource from EHR
   */
  private async getRemoteResource(
    connectionId: string,
    resourceType: string,
    resourceId: string
  ): Promise<{ data: unknown; version: string } | null> {
    // This would need to be implemented with proper token handling
    // For now, return null to indicate no conflict detection
    return null;
  }

  /**
   * Get pending conflicts
   */
  getPendingConflicts(): ConflictInfo[] {
    return this.conflictResolver.getPendingConflicts()
      .map(r => r.conflict);
  }

  /**
   * Resolve a conflict manually
   */
  async resolveConflict(
    conflictId: string,
    strategy: ConflictResolutionStrategy,
    resolvedBy: string,
    manualData?: unknown
  ): Promise<void> {
    await this.conflictResolver.resolveConflict(
      conflictId,
      strategy,
      resolvedBy,
      manualData
    );
  }

  /**
   * Get adapter for direct FHIR operations
   */
  getAdapter(connectionId: string): GenericFhirAdapter {
    return this.getConnection(connectionId).adapter;
  }

  /**
   * Get Epic adapter for Epic-specific operations
   */
  getEpicAdapter(connectionId: string): EpicAdapter {
    const connection = this.getConnection(connectionId);
    if (connection.system !== 'epic') {
      throw new Error('Connection is not to an Epic system');
    }
    return connection.adapter as EpicAdapter;
  }

  /**
   * Get Cerner adapter for Cerner-specific operations
   */
  getCernerAdapter(connectionId: string): CernerAdapter {
    const connection = this.getConnection(connectionId);
    if (connection.system !== 'cerner') {
      throw new Error('Connection is not to a Cerner system');
    }
    return connection.adapter as CernerAdapter;
  }

  /**
   * Get connection by ID
   */
  private getConnection(connectionId: string): ActiveConnection {
    const connection = this.connections.get(connectionId);
    if (!connection) {
      throw new Error(`Connection not found: ${connectionId}`);
    }
    return connection;
  }

  /**
   * Check if connected to a specific EHR
   */
  isConnected(connectionId: string): boolean {
    return this.connections.has(connectionId);
  }

  /**
   * Get all active connection IDs
   */
  getConnectionIds(): string[] {
    return Array.from(this.connections.keys());
  }

  /**
   * Get connection info
   */
  getConnectionInfo(connectionId: string): {
    system: EhrSystem;
    baseUrl: string;
    activeTokens: number;
  } {
    const connection = this.getConnection(connectionId);
    return {
      system: connection.system,
      baseUrl: connection.endpoint.baseUrl,
      activeTokens: connection.tokenManager.getActiveTokenCount(),
    };
  }

  /**
   * Get conflict resolver statistics
   */
  getConflictStats(): ReturnType<ConflictResolver['getStatistics']> {
    return this.conflictResolver.getStatistics();
  }

  /**
   * Get cache statistics
   *
   * Returns cache hit rates and other performance metrics.
   */
  getCacheStats(): {
    enabled: boolean;
    tokens: CacheStats | null;
    resources: CacheStats | null;
  } {
    if (!this.cacheManager) {
      return { enabled: false, tokens: null, resources: null };
    }

    const stats = this.cacheManager.getStats();
    return {
      enabled: true,
      tokens: stats.tokens,
      resources: stats.resources,
    };
  }

  /**
   * Get the cache manager for direct cache operations
   *
   * Returns null if caching is disabled.
   */
  getCache(): EhrCacheManager | null {
    return this.cacheManager;
  }

  /**
   * Clear all caches
   */
  clearCaches(): void {
    if (this.cacheManager) {
      this.cacheManager.clearAll();
    }
  }

  /**
   * Check if a valid cached token exists for a connection
   */
  hasValidCachedToken(connectionId: string): boolean {
    if (!this.cacheManager) {
      return false;
    }
    return this.cacheManager.tokens.hasValidToken(connectionId);
  }
}
