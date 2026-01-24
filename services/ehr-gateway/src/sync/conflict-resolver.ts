/**
 * Conflict Resolver
 *
 * Handles conflicts that arise during bidirectional sync
 * between Mycelix-Health and external EHR systems.
 */

import type { ActionHash } from '@holochain/client';
import type { ConflictInfo } from '../types.js';

export type ConflictResolutionStrategy =
  | 'local_wins'
  | 'remote_wins'
  | 'most_recent'
  | 'manual'
  | 'merge';

export interface ConflictResolution {
  conflictId: string;
  strategy: ConflictResolutionStrategy;
  resolvedData: unknown;
  resolvedAt: Date;
  resolvedBy: string;
  notes?: string;
}

export interface ConflictResolverConfig {
  defaultStrategy: ConflictResolutionStrategy;
  autoResolveThreshold?: number; // Auto-resolve if difference is below threshold
  mergeRules?: MergeRules;
}

export interface MergeRules {
  preferLocalFields?: string[];
  preferRemoteFields?: string[];
  concatenateFields?: string[];
  customMergers?: Record<string, (local: unknown, remote: unknown) => unknown>;
}

export interface ConflictRecord {
  id: string;
  conflict: ConflictInfo;
  resolution?: ConflictResolution;
  createdAt: Date;
  status: 'pending' | 'resolved' | 'deferred';
}

export class ConflictResolver {
  private config: ConflictResolverConfig;
  private conflicts: Map<string, ConflictRecord> = new Map();

  constructor(config: ConflictResolverConfig) {
    this.config = {
      autoResolveThreshold: 0,
      ...config,
    };
  }

  /**
   * Detect if there's a conflict between local and remote versions
   */
  detectConflict(
    resourceType: string,
    resourceId: string,
    localData: unknown,
    remoteData: unknown,
    localVersion: string,
    remoteVersion: string
  ): ConflictInfo | null {
    // Check if versions are different
    if (localVersion === remoteVersion) {
      return null;
    }

    // Check if data is actually different
    const localJson = JSON.stringify(localData);
    const remoteJson = JSON.stringify(remoteData);

    if (localJson === remoteJson) {
      return null;
    }

    // Determine conflict type
    let conflictType: 'update' | 'delete' | 'create' = 'update';

    if (localData === null && remoteData !== null) {
      conflictType = 'delete';
    } else if (localData !== null && remoteData === null) {
      conflictType = 'create';
    }

    return {
      resourceType,
      resourceId,
      localVersion,
      remoteVersion,
      localData,
      remoteData,
      conflictType,
    };
  }

  /**
   * Register a conflict for resolution
   */
  registerConflict(conflict: ConflictInfo): string {
    const id = this.generateConflictId(conflict);

    const record: ConflictRecord = {
      id,
      conflict,
      createdAt: new Date(),
      status: 'pending',
    };

    this.conflicts.set(id, record);

    return id;
  }

  /**
   * Resolve a conflict using the specified strategy
   */
  async resolveConflict(
    conflictId: string,
    strategy: ConflictResolutionStrategy = this.config.defaultStrategy,
    resolvedBy: string = 'system',
    manualData?: unknown
  ): Promise<ConflictResolution> {
    const record = this.conflicts.get(conflictId);

    if (!record) {
      throw new Error(`Conflict not found: ${conflictId}`);
    }

    if (record.status === 'resolved') {
      throw new Error(`Conflict already resolved: ${conflictId}`);
    }

    let resolvedData: unknown;

    switch (strategy) {
      case 'local_wins':
        resolvedData = record.conflict.localData;
        break;

      case 'remote_wins':
        resolvedData = record.conflict.remoteData;
        break;

      case 'most_recent':
        resolvedData = this.resolveMostRecent(record.conflict);
        break;

      case 'merge':
        resolvedData = this.mergeData(
          record.conflict.localData,
          record.conflict.remoteData
        );
        break;

      case 'manual':
        if (manualData === undefined) {
          throw new Error('Manual resolution requires resolvedData');
        }
        resolvedData = manualData;
        break;

      default:
        throw new Error(`Unknown resolution strategy: ${strategy}`);
    }

    const resolution: ConflictResolution = {
      conflictId,
      strategy,
      resolvedData,
      resolvedAt: new Date(),
      resolvedBy,
    };

    record.resolution = resolution;
    record.status = 'resolved';

    return resolution;
  }

  /**
   * Defer a conflict for later resolution
   */
  deferConflict(conflictId: string): void {
    const record = this.conflicts.get(conflictId);

    if (!record) {
      throw new Error(`Conflict not found: ${conflictId}`);
    }

    record.status = 'deferred';
  }

  /**
   * Get pending conflicts
   */
  getPendingConflicts(): ConflictRecord[] {
    return Array.from(this.conflicts.values())
      .filter(r => r.status === 'pending');
  }

  /**
   * Get deferred conflicts
   */
  getDeferredConflicts(): ConflictRecord[] {
    return Array.from(this.conflicts.values())
      .filter(r => r.status === 'deferred');
  }

  /**
   * Get resolved conflicts
   */
  getResolvedConflicts(): ConflictRecord[] {
    return Array.from(this.conflicts.values())
      .filter(r => r.status === 'resolved');
  }

  /**
   * Get conflict by ID
   */
  getConflict(conflictId: string): ConflictRecord | undefined {
    return this.conflicts.get(conflictId);
  }

  /**
   * Auto-resolve conflicts that can be handled automatically
   */
  async autoResolveConflicts(resolvedBy: string = 'auto-resolver'): Promise<ConflictResolution[]> {
    const resolutions: ConflictResolution[] = [];

    for (const [id, record] of this.conflicts.entries()) {
      if (record.status !== 'pending') continue;

      const canAutoResolve = this.canAutoResolve(record.conflict);

      if (canAutoResolve) {
        try {
          const resolution = await this.resolveConflict(
            id,
            this.config.defaultStrategy,
            resolvedBy
          );
          resolutions.push(resolution);
        } catch {
          // Skip conflicts that fail auto-resolution
        }
      }
    }

    return resolutions;
  }

  /**
   * Check if a conflict can be auto-resolved
   */
  private canAutoResolve(conflict: ConflictInfo): boolean {
    // Delete conflicts typically need manual review
    if (conflict.conflictType === 'delete') {
      return false;
    }

    // Calculate similarity between local and remote
    const similarity = this.calculateSimilarity(
      conflict.localData,
      conflict.remoteData
    );

    // Auto-resolve if similarity is above threshold
    return similarity >= (1 - (this.config.autoResolveThreshold || 0));
  }

  /**
   * Calculate similarity between two data objects
   */
  private calculateSimilarity(local: unknown, remote: unknown): number {
    if (local === remote) return 1;
    if (local === null || remote === null) return 0;

    const localJson = JSON.stringify(local);
    const remoteJson = JSON.stringify(remote);

    // Simple Jaccard-like similarity
    const localSet = new Set(localJson.split(''));
    const remoteSet = new Set(remoteJson.split(''));

    const intersection = new Set([...localSet].filter(x => remoteSet.has(x)));
    const union = new Set([...localSet, ...remoteSet]);

    return intersection.size / union.size;
  }

  /**
   * Resolve using most recent timestamp
   */
  private resolveMostRecent(conflict: ConflictInfo): unknown {
    const localTime = this.extractTimestamp(conflict.localData);
    const remoteTime = this.extractTimestamp(conflict.remoteData);

    if (!localTime || !remoteTime) {
      // Fall back to local wins if timestamps unavailable
      return conflict.localData;
    }

    return localTime > remoteTime ? conflict.localData : conflict.remoteData;
  }

  /**
   * Extract timestamp from data
   */
  private extractTimestamp(data: unknown): Date | null {
    if (!data || typeof data !== 'object') return null;

    const obj = data as Record<string, unknown>;

    // Try common timestamp fields
    const timestampFields = [
      'lastUpdated',
      'last_updated',
      'updatedAt',
      'updated_at',
      'modifiedAt',
      'modified_at',
      'meta.lastUpdated',
    ];

    for (const field of timestampFields) {
      const value = this.getNestedValue(obj, field);
      if (value) {
        const date = new Date(value as string);
        if (!isNaN(date.getTime())) {
          return date;
        }
      }
    }

    return null;
  }

  /**
   * Get nested value from object
   */
  private getNestedValue(obj: Record<string, unknown>, path: string): unknown {
    return path.split('.').reduce((curr, key) => {
      if (curr && typeof curr === 'object') {
        return (curr as Record<string, unknown>)[key];
      }
      return undefined;
    }, obj as unknown);
  }

  /**
   * Merge two data objects
   */
  private mergeData(local: unknown, remote: unknown): unknown {
    if (!local) return remote;
    if (!remote) return local;

    if (typeof local !== 'object' || typeof remote !== 'object') {
      // For non-objects, prefer local
      return local;
    }

    if (Array.isArray(local) && Array.isArray(remote)) {
      return this.mergeArrays(local, remote);
    }

    return this.mergeObjects(
      local as Record<string, unknown>,
      remote as Record<string, unknown>
    );
  }

  /**
   * Merge two objects
   */
  private mergeObjects(
    local: Record<string, unknown>,
    remote: Record<string, unknown>
  ): Record<string, unknown> {
    const result: Record<string, unknown> = { ...remote };
    const rules = this.config.mergeRules || {};

    for (const [key, localValue] of Object.entries(local)) {
      // Check if field has custom merger
      if (rules.customMergers?.[key]) {
        result[key] = rules.customMergers[key](localValue, remote[key]);
        continue;
      }

      // Check if field should prefer local
      if (rules.preferLocalFields?.includes(key)) {
        result[key] = localValue;
        continue;
      }

      // Check if field should prefer remote
      if (rules.preferRemoteFields?.includes(key)) {
        continue; // Already in result from spread
      }

      // Check if field should be concatenated
      if (rules.concatenateFields?.includes(key)) {
        if (Array.isArray(localValue) && Array.isArray(remote[key])) {
          result[key] = [...new Set([...localValue, ...(remote[key] as unknown[])])];
        } else if (typeof localValue === 'string' && typeof remote[key] === 'string') {
          result[key] = `${remote[key]}; ${localValue}`;
        }
        continue;
      }

      // Default: recursively merge objects, prefer local for primitives
      if (typeof localValue === 'object' && localValue !== null &&
          typeof remote[key] === 'object' && remote[key] !== null) {
        result[key] = this.mergeData(localValue, remote[key]);
      } else if (localValue !== undefined) {
        result[key] = localValue;
      }
    }

    return result;
  }

  /**
   * Merge two arrays
   */
  private mergeArrays(local: unknown[], remote: unknown[]): unknown[] {
    // Deduplicate by stringified comparison
    const seen = new Set<string>();
    const result: unknown[] = [];

    for (const item of [...local, ...remote]) {
      const key = JSON.stringify(item);
      if (!seen.has(key)) {
        seen.add(key);
        result.push(item);
      }
    }

    return result;
  }

  /**
   * Generate unique conflict ID
   */
  private generateConflictId(conflict: ConflictInfo): string {
    const timestamp = Date.now();
    const random = Math.random().toString(36).substring(7);
    return `${conflict.resourceType}-${conflict.resourceId}-${timestamp}-${random}`;
  }

  /**
   * Get conflict statistics
   */
  getStatistics(): {
    total: number;
    pending: number;
    resolved: number;
    deferred: number;
    byType: Record<string, number>;
    byConflictType: Record<string, number>;
  } {
    const stats = {
      total: this.conflicts.size,
      pending: 0,
      resolved: 0,
      deferred: 0,
      byType: {} as Record<string, number>,
      byConflictType: {} as Record<string, number>,
    };

    for (const record of this.conflicts.values()) {
      switch (record.status) {
        case 'pending':
          stats.pending++;
          break;
        case 'resolved':
          stats.resolved++;
          break;
        case 'deferred':
          stats.deferred++;
          break;
      }

      const resourceType = record.conflict.resourceType;
      stats.byType[resourceType] = (stats.byType[resourceType] || 0) + 1;

      const conflictType = record.conflict.conflictType;
      stats.byConflictType[conflictType] = (stats.byConflictType[conflictType] || 0) + 1;
    }

    return stats;
  }

  /**
   * Clear resolved conflicts older than specified age
   */
  clearOldResolved(maxAgeMs: number = 7 * 24 * 60 * 60 * 1000): number {
    const cutoff = Date.now() - maxAgeMs;
    let cleared = 0;

    for (const [id, record] of this.conflicts.entries()) {
      if (record.status === 'resolved' &&
          record.resolution &&
          record.resolution.resolvedAt.getTime() < cutoff) {
        this.conflicts.delete(id);
        cleared++;
      }
    }

    return cleared;
  }
}
