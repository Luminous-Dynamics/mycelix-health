/**
 * Mobile Support Zome Client
 *
 * Client for offline-first mobile health applications.
 * Part of Phase 6 - Global Scale.
 */

import type { AppClient, ActionHash, Timestamp } from '@holochain/client';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

// Enums
export enum SyncStatus {
  Pending = 'Pending',
  InProgress = 'InProgress',
  Completed = 'Completed',
  Failed = 'Failed',
  Conflict = 'Conflict',
}

export enum ConflictResolution {
  ServerWins = 'ServerWins',
  ClientWins = 'ClientWins',
  Merge = 'Merge',
  Manual = 'Manual',
}

export enum DataPriority {
  Critical = 'Critical',
  High = 'High',
  Normal = 'Normal',
  Low = 'Low',
}

// Types
export interface SyncQueue {
  queue_hash: ActionHash;
  device_id: string;
  pending_items: number;
  last_sync: Timestamp;
  status: SyncStatus;
}

export interface SyncItem {
  item_hash: ActionHash;
  queue_hash: ActionHash;
  operation: 'create' | 'update' | 'delete';
  entry_type: string;
  entry_hash?: ActionHash;
  payload: string;
  priority: DataPriority;
  created_at: Timestamp;
  synced_at?: Timestamp;
  status: SyncStatus;
  retry_count: number;
  error_message?: string;
}

export interface SyncConflict {
  conflict_hash: ActionHash;
  item_hash: ActionHash;
  local_version: string;
  remote_version: string;
  local_timestamp: Timestamp;
  remote_timestamp: Timestamp;
  resolution?: ConflictResolution;
  resolved_at?: Timestamp;
}

export interface OfflineCache {
  cache_hash: ActionHash;
  device_id: string;
  entry_type: string;
  entry_hash: ActionHash;
  cached_data: string;
  cached_at: Timestamp;
  expires_at?: Timestamp;
  priority: DataPriority;
}

export interface DeviceRegistration {
  device_hash: ActionHash;
  device_id: string;
  device_name: string;
  platform: string;
  app_version: string;
  last_seen: Timestamp;
  sync_enabled: boolean;
  push_token?: string;
}

export interface BandwidthProfile {
  profile_hash: ActionHash;
  device_id: string;
  max_payload_bytes: number;
  sync_frequency_seconds: number;
  compress_data: boolean;
  sync_on_wifi_only: boolean;
  batch_size: number;
}

// Input types
export interface RegisterDeviceInput {
  device_id: string;
  device_name: string;
  platform: string;
  app_version: string;
  push_token?: string;
}

export interface QueueSyncItemInput {
  operation: 'create' | 'update' | 'delete';
  entry_type: string;
  entry_hash?: ActionHash;
  payload: string;
  priority: DataPriority;
}

export interface CacheDataInput {
  entry_type: string;
  entry_hash: ActionHash;
  data: string;
  priority: DataPriority;
  ttl_seconds?: number;
}

export interface SetBandwidthProfileInput {
  max_payload_bytes: number;
  sync_frequency_seconds: number;
  compress_data: boolean;
  sync_on_wifi_only: boolean;
  batch_size: number;
}

export interface SyncResult {
  items_synced: number;
  items_failed: number;
  conflicts_detected: number;
  bytes_transferred: number;
  duration_ms: number;
}

/**
 * Mobile Support Zome Client
 */
export class MobileSupportClient {
  private readonly roleName: string;
  private readonly zomeName = 'mobile_support';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  /**
   * Register a device
   */
  async registerDevice(input: RegisterDeviceInput): Promise<ActionHash> {
    return this.call<ActionHash>('register_device', input);
  }

  /**
   * Get device registration
   */
  async getDevice(deviceId: string): Promise<DeviceRegistration | null> {
    return this.call<DeviceRegistration | null>('get_device', deviceId);
  }

  /**
   * Update device info
   */
  async updateDevice(deviceId: string, updates: Partial<RegisterDeviceInput>): Promise<ActionHash> {
    return this.call<ActionHash>('update_device', { device_id: deviceId, updates });
  }

  /**
   * Get my devices
   */
  async getMyDevices(): Promise<DeviceRegistration[]> {
    return this.call<DeviceRegistration[]>('get_my_devices', null);
  }

  /**
   * Queue a sync item
   */
  async queueSyncItem(input: QueueSyncItemInput): Promise<ActionHash> {
    return this.call<ActionHash>('queue_sync_item', input);
  }

  /**
   * Get sync queue for a device
   */
  async getSyncQueue(deviceId: string): Promise<SyncQueue | null> {
    return this.call<SyncQueue | null>('get_sync_queue', deviceId);
  }

  /**
   * Get pending sync items
   */
  async getPendingItems(deviceId: string): Promise<SyncItem[]> {
    return this.call<SyncItem[]>('get_pending_items', deviceId);
  }

  /**
   * Execute sync
   */
  async executeSync(deviceId: string): Promise<SyncResult> {
    return this.call<SyncResult>('execute_sync', deviceId);
  }

  /**
   * Mark item as synced
   */
  async markSynced(itemHash: ActionHash): Promise<void> {
    return this.call<void>('mark_synced', itemHash);
  }

  /**
   * Report sync failure
   */
  async reportSyncFailure(itemHash: ActionHash, errorMessage: string): Promise<void> {
    return this.call<void>('report_sync_failure', {
      item_hash: itemHash,
      error_message: errorMessage,
    });
  }

  /**
   * Get sync conflicts
   */
  async getConflicts(deviceId: string): Promise<SyncConflict[]> {
    return this.call<SyncConflict[]>('get_conflicts', deviceId);
  }

  /**
   * Resolve a conflict
   */
  async resolveConflict(conflictHash: ActionHash, resolution: ConflictResolution): Promise<void> {
    return this.call<void>('resolve_conflict', {
      conflict_hash: conflictHash,
      resolution,
    });
  }

  /**
   * Cache data for offline use
   */
  async cacheData(input: CacheDataInput): Promise<ActionHash> {
    return this.call<ActionHash>('cache_data', input);
  }

  /**
   * Get cached data
   */
  async getCachedData(entryHash: ActionHash): Promise<OfflineCache | null> {
    return this.call<OfflineCache | null>('get_cached_data', entryHash);
  }

  /**
   * Clear expired cache
   */
  async clearExpiredCache(deviceId: string): Promise<number> {
    return this.call<number>('clear_expired_cache', deviceId);
  }

  /**
   * Set bandwidth profile
   */
  async setBandwidthProfile(deviceId: string, profile: SetBandwidthProfileInput): Promise<ActionHash> {
    return this.call<ActionHash>('set_bandwidth_profile', { device_id: deviceId, ...profile });
  }

  /**
   * Get bandwidth profile
   */
  async getBandwidthProfile(deviceId: string): Promise<BandwidthProfile | null> {
    return this.call<BandwidthProfile | null>('get_bandwidth_profile', deviceId);
  }

  /**
   * Get sync statistics
   */
  async getSyncStats(deviceId: string): Promise<{
    total_synced: number;
    total_failed: number;
    total_conflicts: number;
    last_sync: Timestamp;
    bytes_transferred: number;
  }> {
    return this.call<{
      total_synced: number;
      total_failed: number;
      total_conflicts: number;
      last_sync: Timestamp;
      bytes_transferred: number;
    }>('get_sync_stats', deviceId);
  }

  private async call<T>(fnName: string, payload: unknown): Promise<T> {
    try {
      const result = await this.client.callZome({
        role_name: this.roleName,
        zome_name: this.zomeName,
        fn_name: fnName,
        payload,
      });
      return result as T;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      throw new HealthSdkError(
        HealthSdkErrorCode.ZOME_CALL_FAILED,
        `Mobile Support zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
