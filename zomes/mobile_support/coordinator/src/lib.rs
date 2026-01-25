//! Mobile Support Coordinator Zome
//!
//! Implements mobile-optimized operations for healthcare including
//! offline sync, conflict resolution, device management, and notifications.

use hdk::prelude::*;
use mobile_support_integrity::*;

// ============================================================================
// Device Management
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterDeviceInput {
    pub device_id: String,
    pub device_name: String,
    pub platform: DevicePlatform,
    pub platform_version: String,
    pub app_version: String,
    pub push_token: Option<String>,
    pub public_key: String,
    pub has_biometric: bool,
    pub biometric_type: Option<String>,
    pub storage_quota: u64,
}

#[hdk_extern]
pub fn register_device(input: RegisterDeviceInput) -> ExternResult<ActionHash> {
    let agent_info = agent_info()?;
    let now = sys_time()?;

    let device = RegisteredDevice {
        device_id: input.device_id,
        device_name: input.device_name,
        platform: input.platform,
        platform_version: input.platform_version,
        app_version: input.app_version,
        push_token: input.push_token,
        public_key: input.public_key,
        has_biometric: input.has_biometric,
        biometric_type: input.biometric_type,
        status: DeviceStatus::Active,
        owner_agent: agent_info.agent_initial_pubkey.clone(),
        last_seen: now,
        last_sync: None,
        storage_quota: input.storage_quota,
        storage_used: 0,
        registered_at: now,
    };

    let action_hash = create_entry(EntryTypes::RegisteredDevice(device))?;

    // Link agent to device
    create_link(
        agent_info.agent_initial_pubkey,
        action_hash.clone(),
        LinkTypes::AgentToDevices,
        (),
    )?;

    Ok(action_hash)
}

#[hdk_extern]
pub fn get_device(action_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(action_hash, GetOptions::default())
}

#[hdk_extern]
pub fn get_my_devices(_: ()) -> ExternResult<Vec<Record>> {
    let agent_info = agent_info()?;

    let links = get_links(
        LinkQuery::try_new(agent_info.agent_initial_pubkey, LinkTypes::AgentToDevices)?, GetStrategy::default(),
    )?;

    let mut devices = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                devices.push(record);
            }
        }
    }

    Ok(devices)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateDeviceStatusInput {
    pub device_hash: ActionHash,
    pub status: DeviceStatus,
}

#[hdk_extern]
pub fn update_device_status(input: UpdateDeviceStatusInput) -> ExternResult<ActionHash> {
    let record = get(input.device_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Device not found".to_string())))?;

    let mut device: RegisteredDevice = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid device".to_string())))?;

    device.status = input.status;
    device.last_seen = sys_time()?;

    update_entry(input.device_hash, EntryTypes::RegisteredDevice(device))
}

#[hdk_extern]
pub fn heartbeat(device_hash: ActionHash) -> ExternResult<ActionHash> {
    let record = get(device_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Device not found".to_string())))?;

    let mut device: RegisteredDevice = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid device".to_string())))?;

    device.last_seen = sys_time()?;

    update_entry(device_hash, EntryTypes::RegisteredDevice(device))
}

// ============================================================================
// Sync Management
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateCheckpointInput {
    pub device_hash: ActionHash,
    pub last_action_hash: Option<ActionHash>,
    pub pending_upload_count: u32,
    pub pending_download_count: u32,
    pub checkpoint_data: Option<String>,
}

#[hdk_extern]
pub fn create_checkpoint(input: CreateCheckpointInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let checkpoint = SyncCheckpoint {
        checkpoint_id: format!("cp_{}", now.as_micros()),
        device_hash: input.device_hash.clone(),
        last_action_hash: input.last_action_hash,
        last_timestamp: now,
        pending_upload_count: input.pending_upload_count,
        pending_download_count: input.pending_download_count,
        status: SyncStatus::Synced,
        error_message: None,
        checkpoint_data: input.checkpoint_data,
        created_at: now,
    };

    let action_hash = create_entry(EntryTypes::SyncCheckpoint(checkpoint))?;

    create_link(
        input.device_hash,
        action_hash.clone(),
        LinkTypes::DeviceToCheckpoints,
        (),
    )?;

    Ok(action_hash)
}

#[hdk_extern]
pub fn get_latest_checkpoint(device_hash: ActionHash) -> ExternResult<Option<Record>> {
    let links = get_links(LinkQuery::try_new(device_hash, LinkTypes::DeviceToCheckpoints)?, GetStrategy::default())?;

    // Get most recent checkpoint
    let mut latest: Option<(Timestamp, Record)> = None;

    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                if let Some(checkpoint) = record
                    .entry()
                    .to_app_option::<SyncCheckpoint>()
                    .ok()
                    .flatten()
                {
                    match &latest {
                        None => latest = Some((checkpoint.created_at, record)),
                        Some((ts, _)) if checkpoint.created_at > *ts => {
                            latest = Some((checkpoint.created_at, record))
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(latest.map(|(_, record)| record))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueueSyncInput {
    pub device_hash: ActionHash,
    pub entry_type: String,
    pub entry_hash: ActionHash,
    pub operation: String,
    pub priority: SyncPriority,
    pub direction: String,
}

#[hdk_extern]
pub fn queue_sync(input: QueueSyncInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let queue_entry = SyncQueueEntry {
        queue_id: format!("sq_{}", now.as_micros()),
        device_hash: input.device_hash.clone(),
        entry_type: input.entry_type,
        entry_hash: input.entry_hash,
        operation: input.operation,
        priority: input.priority,
        direction: input.direction,
        retry_count: 0,
        max_retries: 3,
        status: SyncStatus::Pending,
        error: None,
        queued_at: now,
        processed_at: None,
    };

    let action_hash = create_entry(EntryTypes::SyncQueueEntry(queue_entry))?;

    create_link(
        input.device_hash,
        action_hash.clone(),
        LinkTypes::DeviceToQueue,
        (),
    )?;

    Ok(action_hash)
}

#[hdk_extern]
pub fn get_pending_sync(device_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(device_hash, LinkTypes::DeviceToQueue)?, GetStrategy::default())?;

    let mut pending = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                if let Some(entry) = record
                    .entry()
                    .to_app_option::<SyncQueueEntry>()
                    .ok()
                    .flatten()
                {
                    if entry.status == SyncStatus::Pending {
                        pending.push(record);
                    }
                }
            }
        }
    }

    // Sort by priority
    pending.sort_by(|a, b| {
        let entry_a: Option<SyncQueueEntry> = a.entry().to_app_option().ok().flatten();
        let entry_b: Option<SyncQueueEntry> = b.entry().to_app_option().ok().flatten();

        match (entry_a, entry_b) {
            (Some(a), Some(b)) => {
                let priority_a = priority_to_num(&a.priority);
                let priority_b = priority_to_num(&b.priority);
                priority_a.cmp(&priority_b)
            }
            _ => std::cmp::Ordering::Equal,
        }
    });

    Ok(pending)
}

fn priority_to_num(priority: &SyncPriority) -> u8 {
    match priority {
        SyncPriority::Critical => 0,
        SyncPriority::High => 1,
        SyncPriority::Normal => 2,
        SyncPriority::Low => 3,
        SyncPriority::Background => 4,
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MarkSyncCompleteInput {
    pub queue_entry_hash: ActionHash,
}

#[hdk_extern]
pub fn mark_sync_complete(input: MarkSyncCompleteInput) -> ExternResult<ActionHash> {
    let record = get(input.queue_entry_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Queue entry not found".to_string()
        )))?;

    let mut entry: SyncQueueEntry = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid queue entry".to_string()
        )))?;

    entry.status = SyncStatus::Synced;
    entry.processed_at = Some(sys_time()?);

    update_entry(input.queue_entry_hash, EntryTypes::SyncQueueEntry(entry))
}

// ============================================================================
// Conflict Resolution
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct RecordConflictInput {
    pub entry_type: String,
    pub local_hash: ActionHash,
    pub remote_hash: ActionHash,
    pub local_timestamp: Timestamp,
    pub remote_timestamp: Timestamp,
    pub local_data: String,
    pub remote_data: String,
    pub resolution_strategy: ConflictStrategy,
}

#[hdk_extern]
pub fn record_conflict(input: RecordConflictInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let conflict = DataConflict {
        conflict_id: format!("conflict_{}", now.as_micros()),
        entry_type: input.entry_type,
        local_hash: input.local_hash,
        remote_hash: input.remote_hash,
        local_timestamp: input.local_timestamp,
        remote_timestamp: input.remote_timestamp,
        local_data: input.local_data,
        remote_data: input.remote_data,
        resolution_strategy: input.resolution_strategy,
        is_resolved: false,
        resolved_by: None,
        resolution_hash: None,
        resolution_notes: None,
        detected_at: now,
        resolved_at: None,
    };

    create_entry(EntryTypes::DataConflict(conflict))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResolveConflictInput {
    pub conflict_hash: ActionHash,
    pub resolution_hash: ActionHash,
    pub resolution_notes: Option<String>,
}

#[hdk_extern]
pub fn resolve_conflict(input: ResolveConflictInput) -> ExternResult<ActionHash> {
    let agent_info = agent_info()?;
    let record = get(input.conflict_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Conflict not found".to_string()
        )))?;

    let mut conflict: DataConflict = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid conflict entry".to_string()
        )))?;

    conflict.is_resolved = true;
    conflict.resolved_by = Some(agent_info.agent_initial_pubkey);
    conflict.resolution_hash = Some(input.resolution_hash);
    conflict.resolution_notes = input.resolution_notes;
    conflict.resolved_at = Some(sys_time()?);

    update_entry(input.conflict_hash, EntryTypes::DataConflict(conflict))
}

#[hdk_extern]
pub fn get_unresolved_conflicts(device_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(device_hash, LinkTypes::DeviceToConflicts)?, GetStrategy::default())?;

    let mut conflicts = Vec::new();
    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                if let Some(conflict) = record
                    .entry()
                    .to_app_option::<DataConflict>()
                    .ok()
                    .flatten()
                {
                    if !conflict.is_resolved {
                        conflicts.push(record);
                    }
                }
            }
        }
    }

    Ok(conflicts)
}

// ============================================================================
// Offline Cache
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct CacheEntryInput {
    pub device_hash: ActionHash,
    pub entry_hash: ActionHash,
    pub entry_type: String,
    pub priority: SyncPriority,
    pub cached_data: String,
    pub compression: CompressionType,
    pub original_size: u32,
    pub compressed_size: u32,
    pub expires_in_seconds: Option<u64>,
}

#[hdk_extern]
pub fn cache_entry(input: CacheEntryInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let expires_at = input.expires_in_seconds.map(|secs| {
        Timestamp::from_micros(now.as_micros() + (secs as i64 * 1_000_000))
    });

    let cache = OfflineCache {
        cache_id: format!("cache_{}", now.as_micros()),
        device_hash: input.device_hash.clone(),
        entry_hash: input.entry_hash,
        entry_type: input.entry_type,
        priority: input.priority,
        cached_data: input.cached_data,
        compression: input.compression,
        original_size: input.original_size,
        compressed_size: input.compressed_size,
        expires_at,
        last_accessed: now,
        access_count: 0,
        cached_at: now,
    };

    let action_hash = create_entry(EntryTypes::OfflineCache(cache))?;

    create_link(
        input.device_hash,
        action_hash.clone(),
        LinkTypes::DeviceToCache,
        (),
    )?;

    Ok(action_hash)
}

#[hdk_extern]
pub fn get_cached_entries(device_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(device_hash, LinkTypes::DeviceToCache)?, GetStrategy::default())?;

    let mut cached = Vec::new();
    let now = sys_time()?;

    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                if let Some(cache) = record
                    .entry()
                    .to_app_option::<OfflineCache>()
                    .ok()
                    .flatten()
                {
                    // Check if not expired
                    let is_valid = match cache.expires_at {
                        Some(exp) => now < exp,
                        None => true,
                    };

                    if is_valid {
                        cached.push(record);
                    }
                }
            }
        }
    }

    Ok(cached)
}

// ============================================================================
// Push Notifications
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterPushInput {
    pub device_hash: ActionHash,
    pub push_service: String,
    pub push_token: String,
    pub enabled_types: Vec<NotificationType>,
    pub quiet_start: Option<String>,
    pub quiet_end: Option<String>,
    pub timezone: String,
}

#[hdk_extern]
pub fn register_push(input: RegisterPushInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let registration = PushRegistration {
        registration_id: format!("push_{}", now.as_micros()),
        device_hash: input.device_hash,
        push_service: input.push_service,
        push_token: input.push_token,
        enabled_types: input.enabled_types,
        quiet_start: input.quiet_start,
        quiet_end: input.quiet_end,
        timezone: input.timezone,
        is_active: true,
        created_at: now,
        updated_at: now,
    };

    create_entry(EntryTypes::PushRegistration(registration))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SendNotificationInput {
    pub device_hash: ActionHash,
    pub notification_type: NotificationType,
    pub title: String,
    pub body: String,
    pub data: Option<String>,
    pub priority: SyncPriority,
    pub scheduled_for: Option<Timestamp>,
    pub expires_in_seconds: Option<u64>,
}

#[hdk_extern]
pub fn send_notification(input: SendNotificationInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let scheduled = input.scheduled_for.unwrap_or(now);
    let expires_at = input.expires_in_seconds.map(|secs| {
        Timestamp::from_micros(now.as_micros() + (secs as i64 * 1_000_000))
    });

    let notification = PendingNotification {
        notification_id: format!("notif_{}", now.as_micros()),
        device_hash: input.device_hash.clone(),
        notification_type: input.notification_type,
        title: input.title,
        body: input.body,
        data: input.data,
        priority: input.priority,
        scheduled_for: scheduled,
        expires_at,
        is_sent: false,
        sent_at: None,
        delivered_at: None,
        read_at: None,
        created_at: now,
    };

    let action_hash = create_entry(EntryTypes::PendingNotification(notification))?;

    create_link(
        input.device_hash,
        action_hash.clone(),
        LinkTypes::DeviceToNotifications,
        (),
    )?;

    Ok(action_hash)
}

#[hdk_extern]
pub fn get_pending_notifications(device_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(device_hash, LinkTypes::DeviceToNotifications)?, GetStrategy::default())?;

    let mut notifications = Vec::new();
    let now = sys_time()?;

    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                if let Some(notif) = record
                    .entry()
                    .to_app_option::<PendingNotification>()
                    .ok()
                    .flatten()
                {
                    // Check if pending, not expired, and ready to send
                    let is_ready = !notif.is_sent
                        && notif.scheduled_for <= now
                        && notif.expires_at.map(|exp| now < exp).unwrap_or(true);

                    if is_ready {
                        notifications.push(record);
                    }
                }
            }
        }
    }

    Ok(notifications)
}

#[hdk_extern]
pub fn mark_notification_sent(notification_hash: ActionHash) -> ExternResult<ActionHash> {
    let record = get(notification_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Notification not found".to_string()
        )))?;

    let mut notif: PendingNotification = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid notification".to_string()
        )))?;

    notif.is_sent = true;
    notif.sent_at = Some(sys_time()?);

    update_entry(notification_hash, EntryTypes::PendingNotification(notif))
}

// ============================================================================
// QR Code Operations
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateQrCodeInput {
    pub purpose: String,
    pub payload: String,
    pub device_hash: ActionHash,
    pub single_use: bool,
    pub max_uses: Option<u32>,
    pub expires_in_seconds: u64,
}

#[hdk_extern]
pub fn create_qr_code(input: CreateQrCodeInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let expires_at = Timestamp::from_micros(
        now.as_micros() + (input.expires_in_seconds as i64 * 1_000_000),
    );

    let qr = QrCodeRecord {
        qr_id: format!("qr_{}", now.as_micros()),
        purpose: input.purpose,
        payload: input.payload,
        key_reference: None,
        created_by_device: input.device_hash,
        single_use: input.single_use,
        use_count: 0,
        max_uses: input.max_uses,
        expires_at,
        is_active: true,
        created_at: now,
    };

    let action_hash = create_entry(EntryTypes::QrCodeRecord(qr))?;

    let anchor = anchor(LinkTypes::ActiveQrCodes, "active_qr".to_string())?;
    create_link(anchor, action_hash.clone(), LinkTypes::ActiveQrCodes, ())?;

    Ok(action_hash)
}

#[hdk_extern]
pub fn use_qr_code(qr_hash: ActionHash) -> ExternResult<ActionHash> {
    let record = get(qr_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("QR code not found".to_string())))?;

    let mut qr: QrCodeRecord = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid QR code".to_string())))?;

    let now = sys_time()?;

    // Check if expired
    if now > qr.expires_at {
        return Err(wasm_error!(WasmErrorInner::Guest("QR code expired".to_string())));
    }

    // Check if still active
    if !qr.is_active {
        return Err(wasm_error!(WasmErrorInner::Guest("QR code inactive".to_string())));
    }

    // Check if already used (single use)
    if qr.single_use && qr.use_count > 0 {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "QR code already used".to_string()
        )));
    }

    // Check max uses
    if let Some(max) = qr.max_uses {
        if qr.use_count >= max {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "QR code max uses reached".to_string()
            )));
        }
    }

    qr.use_count += 1;

    if qr.single_use {
        qr.is_active = false;
    }

    update_entry(qr_hash, EntryTypes::QrCodeRecord(qr))
}

// ============================================================================
// Emergency Data Snapshot
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateEmergencySnapshotInput {
    pub patient_hash: ActionHash,
    pub device_hash: ActionHash,
    pub blood_type: Option<String>,
    pub allergies: String,
    pub medications: String,
    pub conditions: String,
    pub emergency_contacts: String,
    pub insurance_info: Option<String>,
    pub advance_directives: Option<String>,
    pub organ_donor: Option<bool>,
}

#[hdk_extern]
pub fn create_emergency_snapshot(input: CreateEmergencySnapshotInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let snapshot = EmergencyDataSnapshot {
        snapshot_id: format!("emerg_{}", now.as_micros()),
        patient_hash: input.patient_hash.clone(),
        device_hash: input.device_hash,
        blood_type: input.blood_type,
        allergies: input.allergies,
        medications: input.medications,
        conditions: input.conditions,
        emergency_contacts: input.emergency_contacts,
        insurance_info: input.insurance_info,
        advance_directives: input.advance_directives,
        organ_donor: input.organ_donor,
        emergency_qr: None,
        updated_at: now,
    };

    let action_hash = create_entry(EntryTypes::EmergencyDataSnapshot(snapshot))?;

    create_link(
        input.patient_hash,
        action_hash.clone(),
        LinkTypes::PatientToEmergencySnapshot,
        (),
    )?;

    Ok(action_hash)
}

#[hdk_extern]
pub fn get_emergency_snapshot(patient_hash: ActionHash) -> ExternResult<Option<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToEmergencySnapshot)?, GetStrategy::default(),
    )?;

    // Get most recent snapshot
    let mut latest: Option<(Timestamp, Record)> = None;

    for link in links {
        if let Some(target) = link.target.into_action_hash() {
            if let Some(record) = get(target, GetOptions::default())? {
                if let Some(snapshot) = record
                    .entry()
                    .to_app_option::<EmergencyDataSnapshot>()
                    .ok()
                    .flatten()
                {
                    match &latest {
                        None => latest = Some((snapshot.updated_at, record)),
                        Some((ts, _)) if snapshot.updated_at > *ts => {
                            latest = Some((snapshot.updated_at, record))
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(latest.map(|(_, record)| record))
}

// ============================================================================
// Bandwidth Tracking
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
pub struct RecordBandwidthInput {
    pub device_hash: ActionHash,
    pub period_start: Timestamp,
    pub period_end: Timestamp,
    pub bytes_uploaded: u64,
    pub bytes_downloaded: u64,
    pub entries_synced: u32,
    pub conflicts_resolved: u32,
    pub avg_latency_ms: u32,
}

#[hdk_extern]
pub fn record_bandwidth(input: RecordBandwidthInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let usage = BandwidthUsage {
        usage_id: format!("bw_{}", now.as_micros()),
        device_hash: input.device_hash.clone(),
        period_start: input.period_start,
        period_end: input.period_end,
        bytes_uploaded: input.bytes_uploaded,
        bytes_downloaded: input.bytes_downloaded,
        entries_synced: input.entries_synced,
        conflicts_resolved: input.conflicts_resolved,
        avg_latency_ms: input.avg_latency_ms,
        created_at: now,
    };

    let action_hash = create_entry(EntryTypes::BandwidthUsage(usage))?;

    create_link(
        input.device_hash,
        action_hash.clone(),
        LinkTypes::DeviceToBandwidth,
        (),
    )?;

    Ok(action_hash)
}

// ============================================================================
// Utility Functions
// ============================================================================

fn anchor(_link_type: LinkTypes, anchor_text: String) -> ExternResult<EntryHash> {
    let path = Path::from(anchor_text);
    path.path_entry_hash()
}
