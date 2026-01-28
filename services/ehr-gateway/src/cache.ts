/**
 * EHR Gateway Cache
 *
 * Provides in-memory caching with TTL for:
 * - OAuth tokens (reduces token refresh calls)
 * - FHIR metadata (CapabilityStatement)
 * - Recently fetched resources (reduces duplicate API calls)
 */

export interface CacheEntry<T> {
  value: T;
  expiresAt: number;
  createdAt: number;
}

export interface CacheStats {
  hits: number;
  misses: number;
  evictions: number;
  size: number;
  hitRate: number;
}

export interface CacheConfig {
  /** Maximum number of entries in cache */
  maxEntries?: number;
  /** Default TTL in milliseconds */
  defaultTtlMs?: number;
  /** Whether to track statistics */
  trackStats?: boolean;
  /** Callback when entry is evicted */
  onEviction?: (key: string, value: unknown) => void;
}

/**
 * Generic in-memory cache with TTL support
 */
export class Cache<T = unknown> {
  private entries: Map<string, CacheEntry<T>> = new Map();
  private config: Required<Omit<CacheConfig, 'onEviction'>> & Pick<CacheConfig, 'onEviction'>;
  private stats = {
    hits: 0,
    misses: 0,
    evictions: 0,
  };

  constructor(config: CacheConfig = {}) {
    this.config = {
      maxEntries: config.maxEntries ?? 1000,
      defaultTtlMs: config.defaultTtlMs ?? 5 * 60 * 1000, // 5 minutes default
      trackStats: config.trackStats ?? true,
      onEviction: config.onEviction,
    };
  }

  /**
   * Get a value from cache
   *
   * @param key - Cache key
   * @returns Cached value or undefined if not found or expired
   */
  get(key: string): T | undefined {
    const entry = this.entries.get(key);

    if (!entry) {
      if (this.config.trackStats) this.stats.misses++;
      return undefined;
    }

    // Check if expired
    if (Date.now() > entry.expiresAt) {
      this.delete(key);
      if (this.config.trackStats) this.stats.misses++;
      return undefined;
    }

    if (this.config.trackStats) this.stats.hits++;
    return entry.value;
  }

  /**
   * Set a value in cache
   *
   * @param key - Cache key
   * @param value - Value to cache
   * @param ttlMs - Optional TTL override in milliseconds
   */
  set(key: string, value: T, ttlMs?: number): void {
    // Evict if at max capacity
    if (this.entries.size >= this.config.maxEntries && !this.entries.has(key)) {
      this.evictOldest();
    }

    const now = Date.now();
    const expiresAt = now + (ttlMs ?? this.config.defaultTtlMs);

    this.entries.set(key, {
      value,
      expiresAt,
      createdAt: now,
    });
  }

  /**
   * Check if a key exists and is not expired
   */
  has(key: string): boolean {
    const entry = this.entries.get(key);
    if (!entry) return false;
    if (Date.now() > entry.expiresAt) {
      this.delete(key);
      return false;
    }
    return true;
  }

  /**
   * Delete a key from cache
   */
  delete(key: string): boolean {
    const entry = this.entries.get(key);
    if (entry && this.config.onEviction) {
      this.config.onEviction(key, entry.value);
    }
    return this.entries.delete(key);
  }

  /**
   * Clear all entries
   */
  clear(): void {
    if (this.config.onEviction) {
      for (const [key, entry] of this.entries) {
        this.config.onEviction(key, entry.value);
      }
    }
    this.entries.clear();
    this.stats = { hits: 0, misses: 0, evictions: 0 };
  }

  /**
   * Get cache statistics
   */
  getStats(): CacheStats {
    const total = this.stats.hits + this.stats.misses;
    return {
      ...this.stats,
      size: this.entries.size,
      hitRate: total > 0 ? this.stats.hits / total : 0,
    };
  }

  /**
   * Get or set pattern - fetch and cache if not present
   *
   * @param key - Cache key
   * @param fetcher - Function to fetch value if not cached
   * @param ttlMs - Optional TTL override
   * @returns Cached or freshly fetched value
   */
  async getOrSet(key: string, fetcher: () => Promise<T>, ttlMs?: number): Promise<T> {
    const cached = this.get(key);
    if (cached !== undefined) {
      return cached;
    }

    const value = await fetcher();
    this.set(key, value, ttlMs);
    return value;
  }

  /**
   * Evict expired entries
   *
   * @returns Number of entries evicted
   */
  evictExpired(): number {
    const now = Date.now();
    let evicted = 0;

    for (const [key, entry] of this.entries) {
      if (now > entry.expiresAt) {
        this.delete(key);
        evicted++;
      }
    }

    if (this.config.trackStats) this.stats.evictions += evicted;
    return evicted;
  }

  private evictOldest(): void {
    // Find oldest entry
    let oldestKey: string | null = null;
    let oldestTime = Infinity;

    for (const [key, entry] of this.entries) {
      if (entry.createdAt < oldestTime) {
        oldestTime = entry.createdAt;
        oldestKey = key;
      }
    }

    if (oldestKey) {
      this.delete(oldestKey);
      if (this.config.trackStats) this.stats.evictions++;
    }
  }
}

/**
 * Specialized cache for OAuth tokens
 *
 * Automatically handles:
 * - Refresh buffer (refresh before expiry)
 * - Token-specific TTL from token response
 */
export class TokenCache {
  private cache: Cache<CachedToken>;
  private refreshBufferMs: number;

  constructor(config: { refreshBufferMs?: number } = {}) {
    this.refreshBufferMs = config.refreshBufferMs ?? 5 * 60 * 1000; // 5 min buffer
    this.cache = new Cache<CachedToken>({
      maxEntries: 100, // Usually one per connection
      defaultTtlMs: 30 * 60 * 1000, // 30 min default
    });
  }

  /**
   * Get a token if still valid (with buffer time)
   */
  get(connectionId: string): CachedToken | undefined {
    const token = this.cache.get(connectionId);
    if (!token) return undefined;

    // Check if within refresh buffer
    if (token.expiresAt && Date.now() > token.expiresAt - this.refreshBufferMs) {
      return undefined; // Treat as expired, trigger refresh
    }

    return token;
  }

  /**
   * Store a token
   */
  set(connectionId: string, token: CachedToken): void {
    // Calculate TTL based on token expiry
    let ttlMs: number | undefined;
    if (token.expiresAt) {
      ttlMs = Math.max(0, token.expiresAt - Date.now());
    }

    this.cache.set(connectionId, token, ttlMs);
  }

  /**
   * Remove a token (e.g., on revocation)
   */
  revoke(connectionId: string): void {
    this.cache.delete(connectionId);
  }

  /**
   * Check if a valid token exists
   */
  hasValidToken(connectionId: string): boolean {
    return this.get(connectionId) !== undefined;
  }

  /**
   * Clear all cached tokens
   */
  clear(): void {
    this.cache.clear();
  }
}

/**
 * Cached token information structure
 *
 * Simpler than the full TokenInfo from token-manager, optimized for caching.
 */
export interface CachedToken {
  accessToken: string;
  refreshToken?: string;
  expiresAt?: number;
  scope?: string;
  tokenType?: string;
}

/**
 * Specialized cache for FHIR metadata
 *
 * Stores CapabilityStatement and other metadata with longer TTL
 */
export class MetadataCache {
  private cache: Cache<FhirMetadata>;

  constructor() {
    this.cache = new Cache<FhirMetadata>({
      maxEntries: 50,
      defaultTtlMs: 60 * 60 * 1000, // 1 hour for metadata
    });
  }

  /**
   * Get cached metadata for a FHIR server
   */
  getCapabilityStatement(baseUrl: string): FhirCapabilityStatement | undefined {
    const metadata = this.cache.get(`cap:${baseUrl}`);
    return metadata?.capabilityStatement;
  }

  /**
   * Cache a capability statement
   */
  setCapabilityStatement(baseUrl: string, statement: FhirCapabilityStatement): void {
    const existing = this.cache.get(`cap:${baseUrl}`) || {};
    this.cache.set(`cap:${baseUrl}`, {
      ...existing,
      capabilityStatement: statement,
      fetchedAt: Date.now(),
    });
  }

  /**
   * Check if server supports a resource type
   */
  supportsResourceType(baseUrl: string, resourceType: string): boolean | undefined {
    const statement = this.getCapabilityStatement(baseUrl);
    if (!statement) return undefined;

    return statement.rest?.some(rest =>
      rest.resource?.some(r => r.type === resourceType)
    );
  }

  /**
   * Get supported operations for a resource type
   */
  getSupportedOperations(baseUrl: string, resourceType: string): string[] {
    const statement = this.getCapabilityStatement(baseUrl);
    if (!statement) return [];

    const resource = statement.rest?.flatMap(rest =>
      rest.resource?.filter(r => r.type === resourceType) || []
    )[0];

    if (!resource) return [];

    return resource.interaction?.map(i => i.code) || [];
  }

  /**
   * Clear all cached metadata
   */
  clear(): void {
    this.cache.clear();
  }
}

/**
 * FHIR metadata types
 */
export interface FhirMetadata {
  capabilityStatement?: FhirCapabilityStatement;
  fetchedAt?: number;
}

export interface FhirCapabilityStatement {
  resourceType: 'CapabilityStatement';
  status: string;
  kind: string;
  fhirVersion: string;
  format: string[];
  rest?: FhirRestComponent[];
}

export interface FhirRestComponent {
  mode: 'client' | 'server';
  resource?: FhirResourceComponent[];
}

export interface FhirResourceComponent {
  type: string;
  interaction?: Array<{ code: string }>;
  searchParam?: Array<{ name: string; type: string }>;
}

/**
 * Specialized cache for recently fetched FHIR resources
 *
 * Reduces duplicate API calls for frequently accessed resources
 */
export class ResourceCache {
  private cache: Cache<FhirResource>;

  constructor(config: { ttlMs?: number; maxEntries?: number } = {}) {
    this.cache = new Cache<FhirResource>({
      maxEntries: config.maxEntries ?? 500,
      defaultTtlMs: config.ttlMs ?? 2 * 60 * 1000, // 2 minutes for resources
    });
  }

  /**
   * Generate cache key for a resource
   */
  private key(baseUrl: string, resourceType: string, id: string): string {
    return `${baseUrl}:${resourceType}:${id}`;
  }

  /**
   * Get a cached resource
   */
  get(baseUrl: string, resourceType: string, id: string): FhirResource | undefined {
    return this.cache.get(this.key(baseUrl, resourceType, id));
  }

  /**
   * Cache a resource
   */
  set(baseUrl: string, resource: FhirResource): void {
    if (!resource.id || !resource.resourceType) return;
    this.cache.set(
      this.key(baseUrl, resource.resourceType, resource.id),
      resource
    );
  }

  /**
   * Invalidate a cached resource
   */
  invalidate(baseUrl: string, resourceType: string, id: string): void {
    this.cache.delete(this.key(baseUrl, resourceType, id));
  }

  /**
   * Get or fetch a resource
   */
  async getOrFetch(
    baseUrl: string,
    resourceType: string,
    id: string,
    fetcher: () => Promise<FhirResource>
  ): Promise<FhirResource> {
    return this.cache.getOrSet(
      this.key(baseUrl, resourceType, id),
      fetcher
    );
  }

  /**
   * Get cache statistics
   */
  getStats(): CacheStats {
    return this.cache.getStats();
  }

  /**
   * Clear all cached resources
   */
  clear(): void {
    this.cache.clear();
  }
}

/**
 * Basic FHIR resource structure
 */
export interface FhirResource {
  resourceType: string;
  id?: string;
  meta?: {
    lastUpdated?: string;
    versionId?: string;
  };
  [key: string]: unknown;
}

/**
 * Combined EHR Gateway cache manager
 *
 * Provides unified access to all cache layers
 */
export class EhrCacheManager {
  public readonly tokens: TokenCache;
  public readonly metadata: MetadataCache;
  public readonly resources: ResourceCache;

  constructor(config: EhrCacheConfig = {}) {
    this.tokens = new TokenCache({
      refreshBufferMs: config.tokenRefreshBufferMs,
    });
    this.metadata = new MetadataCache();
    this.resources = new ResourceCache({
      ttlMs: config.resourceTtlMs,
      maxEntries: config.maxResourceCacheEntries,
    });
  }

  /**
   * Clear all caches
   */
  clearAll(): void {
    this.tokens.clear();
    this.metadata.clear();
    this.resources.clear();
  }

  /**
   * Get combined statistics
   */
  getStats(): {
    tokens: CacheStats;
    resources: CacheStats;
  } {
    return {
      tokens: { hits: 0, misses: 0, evictions: 0, size: 0, hitRate: 0 }, // TokenCache doesn't track stats
      resources: this.resources.getStats(),
    };
  }
}

export interface EhrCacheConfig {
  /** Buffer time before token expiry to trigger refresh (ms) */
  tokenRefreshBufferMs?: number;
  /** TTL for cached resources (ms) */
  resourceTtlMs?: number;
  /** Maximum number of resources to cache */
  maxResourceCacheEntries?: number;
}
