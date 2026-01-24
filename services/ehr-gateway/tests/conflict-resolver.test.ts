/**
 * Conflict Resolver Tests
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
  ConflictResolver,
  type ConflictResolutionStrategy,
  type ConflictResolverConfig,
} from '../src/sync/conflict-resolver.js';
import type { ConflictInfo } from '../src/types.js';

describe('ConflictResolver', () => {
  let resolver: ConflictResolver;
  let defaultConfig: ConflictResolverConfig;

  beforeEach(() => {
    defaultConfig = {
      defaultStrategy: 'most_recent',
      autoResolveThreshold: 0.1,
    };
    resolver = new ConflictResolver(defaultConfig);
  });

  describe('detectConflict', () => {
    it('returns null when versions match', () => {
      const conflict = resolver.detectConflict(
        'Patient',
        'patient-123',
        { name: 'John' },
        { name: 'John' },
        '1',
        '1'
      );

      expect(conflict).toBeNull();
    });

    it('returns null when data is identical despite version difference', () => {
      const conflict = resolver.detectConflict(
        'Patient',
        'patient-123',
        { name: 'John' },
        { name: 'John' },
        '1',
        '2'
      );

      // Same data, different versions - no conflict
      expect(conflict).toBeNull();
    });

    it('detects conflict when versions and data differ', () => {
      const conflict = resolver.detectConflict(
        'Patient',
        'patient-123',
        { name: 'John' },
        { name: 'Jane' },
        '1',
        '2'
      );

      expect(conflict).not.toBeNull();
      expect(conflict?.resourceType).toBe('Patient');
      expect(conflict?.resourceId).toBe('patient-123');
      expect(conflict?.conflictType).toBe('update');
    });

    it('detects delete conflict', () => {
      const conflict = resolver.detectConflict(
        'Patient',
        'patient-123',
        null, // Local deleted
        { name: 'John' },
        '1',
        '2'
      );

      expect(conflict).not.toBeNull();
      expect(conflict?.conflictType).toBe('delete');
    });

    it('detects create conflict', () => {
      const conflict = resolver.detectConflict(
        'Patient',
        'patient-123',
        { name: 'John' },
        null, // Remote deleted
        '1',
        '2'
      );

      expect(conflict).not.toBeNull();
      expect(conflict?.conflictType).toBe('create');
    });
  });

  describe('registerConflict', () => {
    it('registers a conflict and returns an ID', () => {
      const conflict: ConflictInfo = {
        resourceType: 'Patient',
        resourceId: 'patient-123',
        localVersion: '1',
        remoteVersion: '2',
        localData: { name: 'John' },
        remoteData: { name: 'Jane' },
        conflictType: 'update',
      };

      const id = resolver.registerConflict(conflict);

      expect(id).toBeDefined();
      expect(id).toContain('Patient-patient-123');
    });
  });

  describe('resolveConflict', () => {
    let conflictId: string;

    beforeEach(() => {
      const conflict: ConflictInfo = {
        resourceType: 'Patient',
        resourceId: 'patient-456',
        localVersion: '1',
        remoteVersion: '2',
        localData: { name: 'LocalName', updated_at: '2024-01-15T10:00:00Z' },
        remoteData: { name: 'RemoteName', updated_at: '2024-01-15T12:00:00Z' },
        conflictType: 'update',
      };
      conflictId = resolver.registerConflict(conflict);
    });

    it('resolves with local_wins strategy', async () => {
      const resolution = await resolver.resolveConflict(conflictId, 'local_wins', 'test-user');

      expect(resolution.strategy).toBe('local_wins');
      expect((resolution.resolvedData as { name: string }).name).toBe('LocalName');
    });

    it('resolves with remote_wins strategy', async () => {
      const resolution = await resolver.resolveConflict(conflictId, 'remote_wins', 'test-user');

      expect(resolution.strategy).toBe('remote_wins');
      expect((resolution.resolvedData as { name: string }).name).toBe('RemoteName');
    });

    it('resolves with most_recent strategy', async () => {
      const resolution = await resolver.resolveConflict(conflictId, 'most_recent', 'test-user');

      expect(resolution.strategy).toBe('most_recent');
      // Remote has later timestamp
      expect((resolution.resolvedData as { name: string }).name).toBe('RemoteName');
    });

    it('resolves with manual strategy', async () => {
      const manualData = { name: 'ManuallyResolved' };
      const resolution = await resolver.resolveConflict(conflictId, 'manual', 'test-user', manualData);

      expect(resolution.strategy).toBe('manual');
      expect((resolution.resolvedData as { name: string }).name).toBe('ManuallyResolved');
    });

    it('throws error for manual strategy without data', async () => {
      await expect(
        resolver.resolveConflict(conflictId, 'manual', 'test-user')
      ).rejects.toThrow('Manual resolution requires resolvedData');
    });

    it('resolves with merge strategy', async () => {
      // Register conflict with different fields
      const mergeConflict: ConflictInfo = {
        resourceType: 'Patient',
        resourceId: 'patient-merge',
        localVersion: '1',
        remoteVersion: '2',
        localData: { name: 'John', phone: '555-1234' },
        remoteData: { name: 'John', email: 'john@example.com' },
        conflictType: 'update',
      };
      const mergeId = resolver.registerConflict(mergeConflict);

      const resolution = await resolver.resolveConflict(mergeId, 'merge', 'test-user');

      expect(resolution.strategy).toBe('merge');
      const merged = resolution.resolvedData as Record<string, unknown>;
      expect(merged.name).toBe('John');
      expect(merged.phone).toBe('555-1234');
      expect(merged.email).toBe('john@example.com');
    });

    it('throws error for non-existent conflict', async () => {
      await expect(
        resolver.resolveConflict('non-existent-id', 'local_wins', 'test-user')
      ).rejects.toThrow('Conflict not found');
    });

    it('throws error for already resolved conflict', async () => {
      await resolver.resolveConflict(conflictId, 'local_wins', 'test-user');

      await expect(
        resolver.resolveConflict(conflictId, 'remote_wins', 'test-user')
      ).rejects.toThrow('Conflict already resolved');
    });
  });

  describe('getPendingConflicts', () => {
    it('returns empty array when no conflicts', () => {
      expect(resolver.getPendingConflicts()).toEqual([]);
    });

    it('returns only pending conflicts', async () => {
      const conflict1: ConflictInfo = {
        resourceType: 'Patient',
        resourceId: 'patient-1',
        localVersion: '1',
        remoteVersion: '2',
        localData: { name: 'A' },
        remoteData: { name: 'B' },
        conflictType: 'update',
      };

      const conflict2: ConflictInfo = {
        resourceType: 'Patient',
        resourceId: 'patient-2',
        localVersion: '1',
        remoteVersion: '2',
        localData: { name: 'C' },
        remoteData: { name: 'D' },
        conflictType: 'update',
      };

      const id1 = resolver.registerConflict(conflict1);
      resolver.registerConflict(conflict2);

      // Resolve first conflict
      await resolver.resolveConflict(id1, 'local_wins', 'test-user');

      const pending = resolver.getPendingConflicts();

      expect(pending.length).toBe(1);
      expect(pending[0].conflict.resourceId).toBe('patient-2');
    });
  });

  describe('getResolvedConflicts', () => {
    it('returns only resolved conflicts', async () => {
      const conflict: ConflictInfo = {
        resourceType: 'Patient',
        resourceId: 'patient-1',
        localVersion: '1',
        remoteVersion: '2',
        localData: { name: 'A' },
        remoteData: { name: 'B' },
        conflictType: 'update',
      };

      const id = resolver.registerConflict(conflict);
      await resolver.resolveConflict(id, 'local_wins', 'test-user');

      const resolved = resolver.getResolvedConflicts();

      expect(resolved.length).toBe(1);
      expect(resolved[0].status).toBe('resolved');
    });
  });

  describe('deferConflict', () => {
    it('marks conflict as deferred', () => {
      const conflict: ConflictInfo = {
        resourceType: 'Patient',
        resourceId: 'patient-defer',
        localVersion: '1',
        remoteVersion: '2',
        localData: { name: 'A' },
        remoteData: { name: 'B' },
        conflictType: 'update',
      };

      const id = resolver.registerConflict(conflict);
      resolver.deferConflict(id);

      const deferred = resolver.getDeferredConflicts();
      expect(deferred.length).toBe(1);
      expect(deferred[0].status).toBe('deferred');
    });

    it('throws error for non-existent conflict', () => {
      expect(() => resolver.deferConflict('non-existent')).toThrow('Conflict not found');
    });
  });

  describe('getConflict', () => {
    it('returns conflict by ID', () => {
      const conflict: ConflictInfo = {
        resourceType: 'Patient',
        resourceId: 'patient-get',
        localVersion: '1',
        remoteVersion: '2',
        localData: { name: 'A' },
        remoteData: { name: 'B' },
        conflictType: 'update',
      };

      const id = resolver.registerConflict(conflict);
      const retrieved = resolver.getConflict(id);

      expect(retrieved).toBeDefined();
      expect(retrieved?.conflict.resourceId).toBe('patient-get');
    });

    it('returns undefined for non-existent ID', () => {
      expect(resolver.getConflict('non-existent')).toBeUndefined();
    });
  });

  describe('getStatistics', () => {
    it('returns correct statistics', async () => {
      const conflict1: ConflictInfo = {
        resourceType: 'Patient',
        resourceId: 'patient-1',
        localVersion: '1',
        remoteVersion: '2',
        localData: { name: 'A' },
        remoteData: { name: 'B' },
        conflictType: 'update',
      };

      const conflict2: ConflictInfo = {
        resourceType: 'Observation',
        resourceId: 'obs-1',
        localVersion: '1',
        remoteVersion: '2',
        localData: { value: 1 },
        remoteData: { value: 2 },
        conflictType: 'update',
      };

      const id1 = resolver.registerConflict(conflict1);
      resolver.registerConflict(conflict2);

      await resolver.resolveConflict(id1, 'local_wins', 'test-user');

      const stats = resolver.getStatistics();

      expect(stats.total).toBe(2);
      expect(stats.resolved).toBe(1);
      expect(stats.pending).toBe(1);
      expect(stats.byType['Patient']).toBe(1);
      expect(stats.byType['Observation']).toBe(1);
      expect(stats.byConflictType['update']).toBe(2);
    });
  });

  describe('autoResolveConflicts', () => {
    it('auto-resolves highly similar conflicts', async () => {
      // Use a config with higher threshold for testing
      const autoResolver = new ConflictResolver({
        defaultStrategy: 'most_recent',
        autoResolveThreshold: 0.5, // 50% threshold - allows more auto-resolution
      });

      // Create very similar data (high similarity)
      const conflict: ConflictInfo = {
        resourceType: 'Patient',
        resourceId: 'patient-auto',
        localVersion: '1',
        remoteVersion: '2',
        localData: { name: 'John Smith', age: 30, city: 'Austin' },
        remoteData: { name: 'John Smith', age: 30, city: 'Dallas' },
        conflictType: 'update',
      };

      autoResolver.registerConflict(conflict);

      const resolutions = await autoResolver.autoResolveConflicts('auto-bot');

      // Highly similar data should auto-resolve
      expect(resolutions.length).toBe(1);
      expect(resolutions[0].resolvedBy).toBe('auto-bot');
    });

    it('does not auto-resolve delete conflicts', async () => {
      const autoResolver = new ConflictResolver({
        defaultStrategy: 'most_recent',
        autoResolveThreshold: 0.9, // Very high threshold
      });

      const deleteConflict: ConflictInfo = {
        resourceType: 'Patient',
        resourceId: 'patient-delete',
        localVersion: '1',
        remoteVersion: '2',
        localData: null, // Deleted locally
        remoteData: { name: 'John' },
        conflictType: 'delete',
      };

      autoResolver.registerConflict(deleteConflict);

      const resolutions = await autoResolver.autoResolveConflicts('auto-bot');

      // Delete conflicts should not auto-resolve
      expect(resolutions.length).toBe(0);
      expect(autoResolver.getPendingConflicts().length).toBe(1);
    });
  });

  describe('clearOldResolved', () => {
    it('does not clear recently resolved conflicts', async () => {
      const conflict: ConflictInfo = {
        resourceType: 'Patient',
        resourceId: 'patient-recent',
        localVersion: '1',
        remoteVersion: '2',
        localData: { name: 'A' },
        remoteData: { name: 'B' },
        conflictType: 'update',
      };

      const id = resolver.registerConflict(conflict);
      await resolver.resolveConflict(id, 'local_wins', 'test-user');

      // Default is 7 days - should NOT clear something just resolved
      const cleared = resolver.clearOldResolved();

      expect(cleared).toBe(0);
      expect(resolver.getResolvedConflicts().length).toBe(1);
    });

    it('does not clear pending or deferred conflicts', async () => {
      const conflict1: ConflictInfo = {
        resourceType: 'Patient',
        resourceId: 'patient-pending',
        localVersion: '1',
        remoteVersion: '2',
        localData: { name: 'A' },
        remoteData: { name: 'B' },
        conflictType: 'update',
      };

      const conflict2: ConflictInfo = {
        resourceType: 'Patient',
        resourceId: 'patient-deferred',
        localVersion: '1',
        remoteVersion: '2',
        localData: { name: 'C' },
        remoteData: { name: 'D' },
        conflictType: 'update',
      };

      resolver.registerConflict(conflict1); // Stays pending
      const id2 = resolver.registerConflict(conflict2);
      resolver.deferConflict(id2); // Deferred

      // Try to clear with any age - should not affect pending/deferred
      const cleared = resolver.clearOldResolved(0);

      expect(cleared).toBe(0); // Only clears resolved conflicts
      expect(resolver.getPendingConflicts().length).toBe(1);
      expect(resolver.getDeferredConflicts().length).toBe(1);
    });
  });

  describe('merge rules', () => {
    it('applies preferLocalFields rule', async () => {
      const configWithRules: ConflictResolverConfig = {
        defaultStrategy: 'merge',
        mergeRules: {
          preferLocalFields: ['phone'],
        },
      };
      const resolverWithRules = new ConflictResolver(configWithRules);

      const conflict: ConflictInfo = {
        resourceType: 'Patient',
        resourceId: 'patient-rules',
        localVersion: '1',
        remoteVersion: '2',
        localData: { name: 'John', phone: '555-LOCAL' },
        remoteData: { name: 'Jane', phone: '555-REMOTE' },
        conflictType: 'update',
      };

      const id = resolverWithRules.registerConflict(conflict);
      const resolution = await resolverWithRules.resolveConflict(id, 'merge', 'test-user');

      const merged = resolution.resolvedData as Record<string, unknown>;
      expect(merged.phone).toBe('555-LOCAL'); // Local preferred
      expect(merged.name).toBe('John'); // Local also wins for default behavior
    });

    it('applies preferRemoteFields rule', async () => {
      const configWithRules: ConflictResolverConfig = {
        defaultStrategy: 'merge',
        mergeRules: {
          preferRemoteFields: ['email'],
        },
      };
      const resolverWithRules = new ConflictResolver(configWithRules);

      const conflict: ConflictInfo = {
        resourceType: 'Patient',
        resourceId: 'patient-remote-pref',
        localVersion: '1',
        remoteVersion: '2',
        localData: { name: 'John', email: 'local@example.com' },
        remoteData: { name: 'John', email: 'remote@example.com' },
        conflictType: 'update',
      };

      const id = resolverWithRules.registerConflict(conflict);
      const resolution = await resolverWithRules.resolveConflict(id, 'merge', 'test-user');

      const merged = resolution.resolvedData as Record<string, unknown>;
      expect(merged.email).toBe('remote@example.com'); // Remote preferred
    });
  });
});
