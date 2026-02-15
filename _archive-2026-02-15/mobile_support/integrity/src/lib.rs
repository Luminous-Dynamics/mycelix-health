//! Mobile Support Integrity Zome
//!
//! Defines entry types for mobile-optimized healthcare operations
//! including offline sync, conflict resolution, device management,
//! and mobile-specific optimizations.

use hdi::prelude::*;

/// Device platform types
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DevicePlatform {
    /// Apple iOS
    IOS,
    /// Google Android
    Android,
    /// Web/PWA
    Web,
    /// Desktop application
    Desktop,
    /// Wearable device
    Wearable,
    /// IoT medical device
    MedicalDevice,
}

/// Device status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DeviceStatus {
    /// Active and syncing
    Active,
    /// Offline but trusted
    Offline,
    /// Lost or stolen
    Lost,
    /// Revoked access
    Revoked,
    /// Pending approval
    Pending,
    /// Retired/replaced
    Retired,
}

/// Sync status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SyncStatus {
    /// Fully synchronized
    Synced,
    /// Currently syncing
    Syncing,
    /// Pending sync
    Pending,
    /// Sync failed
    Failed,
    /// Conflict detected
    Conflict,
    /// Offline mode
    Offline,
}

/// Conflict resolution strategy
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ConflictStrategy {
    /// Server/DHT wins
    ServerWins,
    /// Client/local wins
    ClientWins,
    /// Most recent timestamp wins
    LastWriteWins,
    /// Manual resolution required
    ManualResolve,
    /// Merge changes
    Merge,
    /// Keep both versions
    KeepBoth,
}

/// Data priority for sync
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SyncPriority {
    /// Critical - sync immediately (allergies, medications)
    Critical,
    /// High - sync soon (appointments, prescriptions)
    High,
    /// Normal - standard sync
    Normal,
    /// Low - sync when convenient
    Low,
    /// Background - sync during idle
    Background,
}

/// Compression algorithm
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CompressionType {
    /// No compression
    None,
    /// Gzip compression
    Gzip,
    /// LZ4 fast compression
    Lz4,
    /// Brotli compression
    Brotli,
    /// Zstandard compression
    Zstd,
}

/// Notification type
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum NotificationType {
    /// Appointment reminder
    AppointmentReminder,
    /// Medication reminder
    MedicationReminder,
    /// Lab result ready
    LabResultReady,
    /// Message from provider
    ProviderMessage,
    /// Emergency alert
    EmergencyAlert,
    /// Sync complete
    SyncComplete,
    /// Conflict needs resolution
    ConflictAlert,
    /// Device authorization
    DeviceAuth,
    /// System update
    SystemUpdate,
}

/// Registered device
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RegisteredDevice {
    /// Device ID (unique per device)
    pub device_id: String,
    /// Device name (user-friendly)
    pub device_name: String,
    /// Platform
    pub platform: DevicePlatform,
    /// Platform version
    pub platform_version: String,
    /// App version
    pub app_version: String,
    /// Push notification token
    pub push_token: Option<String>,
    /// Device public key
    pub public_key: String,
    /// Biometric capability
    pub has_biometric: bool,
    /// Biometric type (fingerprint, face, etc.)
    pub biometric_type: Option<String>,
    /// Device status
    pub status: DeviceStatus,
    /// Owner agent
    pub owner_agent: AgentPubKey,
    /// Last seen timestamp
    pub last_seen: Timestamp,
    /// Last sync timestamp
    pub last_sync: Option<Timestamp>,
    /// Storage quota (bytes)
    pub storage_quota: u64,
    /// Storage used (bytes)
    pub storage_used: u64,
    /// Registration timestamp
    pub registered_at: Timestamp,
}

/// Sync checkpoint for resumable sync
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct SyncCheckpoint {
    /// Checkpoint ID
    pub checkpoint_id: String,
    /// Device hash
    pub device_hash: ActionHash,
    /// Last synced action hash
    pub last_action_hash: Option<ActionHash>,
    /// Last synced timestamp
    pub last_timestamp: Timestamp,
    /// Entries pending upload
    pub pending_upload_count: u32,
    /// Entries pending download
    pub pending_download_count: u32,
    /// Sync status
    pub status: SyncStatus,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Checkpoint data (serialized state)
    pub checkpoint_data: Option<String>,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Sync queue entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct SyncQueueEntry {
    /// Queue entry ID
    pub queue_id: String,
    /// Device hash
    pub device_hash: ActionHash,
    /// Entry type being synced
    pub entry_type: String,
    /// Entry action hash
    pub entry_hash: ActionHash,
    /// Operation type (create, update, delete)
    pub operation: String,
    /// Priority
    pub priority: SyncPriority,
    /// Sync direction (upload, download)
    pub direction: String,
    /// Retry count
    pub retry_count: u32,
    /// Max retries
    pub max_retries: u32,
    /// Status
    pub status: SyncStatus,
    /// Error if failed
    pub error: Option<String>,
    /// Queued timestamp
    pub queued_at: Timestamp,
    /// Processed timestamp
    pub processed_at: Option<Timestamp>,
}

/// Data conflict record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DataConflict {
    /// Conflict ID
    pub conflict_id: String,
    /// Entry type involved
    pub entry_type: String,
    /// Local version hash
    pub local_hash: ActionHash,
    /// Remote version hash
    pub remote_hash: ActionHash,
    /// Local timestamp
    pub local_timestamp: Timestamp,
    /// Remote timestamp
    pub remote_timestamp: Timestamp,
    /// Local data (serialized)
    pub local_data: String,
    /// Remote data (serialized)
    pub remote_data: String,
    /// Resolution strategy
    pub resolution_strategy: ConflictStrategy,
    /// Is resolved
    pub is_resolved: bool,
    /// Resolved by (if manual)
    pub resolved_by: Option<AgentPubKey>,
    /// Resolution action hash
    pub resolution_hash: Option<ActionHash>,
    /// Resolution notes
    pub resolution_notes: Option<String>,
    /// Detected timestamp
    pub detected_at: Timestamp,
    /// Resolved timestamp
    pub resolved_at: Option<Timestamp>,
}

/// Offline cache entry
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct OfflineCache {
    /// Cache ID
    pub cache_id: String,
    /// Device hash
    pub device_hash: ActionHash,
    /// Cached entry hash
    pub entry_hash: ActionHash,
    /// Entry type
    pub entry_type: String,
    /// Cache priority
    pub priority: SyncPriority,
    /// Compressed data
    pub cached_data: String,
    /// Compression type used
    pub compression: CompressionType,
    /// Original size (bytes)
    pub original_size: u32,
    /// Compressed size (bytes)
    pub compressed_size: u32,
    /// Expires at
    pub expires_at: Option<Timestamp>,
    /// Last accessed
    pub last_accessed: Timestamp,
    /// Access count
    pub access_count: u32,
    /// Cached timestamp
    pub cached_at: Timestamp,
}

/// Delta sync record (incremental changes)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DeltaSync {
    /// Delta ID
    pub delta_id: String,
    /// Source entry hash
    pub source_hash: ActionHash,
    /// Target entry hash
    pub target_hash: ActionHash,
    /// Entry type
    pub entry_type: String,
    /// Delta operations (JSON patch or similar)
    pub delta_operations: String,
    /// Delta size (bytes)
    pub delta_size: u32,
    /// Full entry size (bytes)
    pub full_size: u32,
    /// Compression savings percentage
    pub savings_percent: u8,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Push notification registration
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PushRegistration {
    /// Registration ID
    pub registration_id: String,
    /// Device hash
    pub device_hash: ActionHash,
    /// Push service (FCM, APNS, etc.)
    pub push_service: String,
    /// Push token
    pub push_token: String,
    /// Enabled notification types
    pub enabled_types: Vec<NotificationType>,
    /// Quiet hours start (HH:MM)
    pub quiet_start: Option<String>,
    /// Quiet hours end (HH:MM)
    pub quiet_end: Option<String>,
    /// Timezone
    pub timezone: String,
    /// Is active
    pub is_active: bool,
    /// Created timestamp
    pub created_at: Timestamp,
    /// Updated timestamp
    pub updated_at: Timestamp,
}

/// Pending notification
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PendingNotification {
    /// Notification ID
    pub notification_id: String,
    /// Target device hash
    pub device_hash: ActionHash,
    /// Notification type
    pub notification_type: NotificationType,
    /// Title
    pub title: String,
    /// Body text
    pub body: String,
    /// Data payload (JSON)
    pub data: Option<String>,
    /// Priority
    pub priority: SyncPriority,
    /// Scheduled for
    pub scheduled_for: Timestamp,
    /// Expires at
    pub expires_at: Option<Timestamp>,
    /// Is sent
    pub is_sent: bool,
    /// Sent timestamp
    pub sent_at: Option<Timestamp>,
    /// Delivery receipt
    pub delivered_at: Option<Timestamp>,
    /// Read receipt
    pub read_at: Option<Timestamp>,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// QR code record (for sharing/pairing)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct QrCodeRecord {
    /// QR code ID
    pub qr_id: String,
    /// Purpose (device_pair, share_record, emergency_access)
    pub purpose: String,
    /// Payload data (encrypted)
    pub payload: String,
    /// Encryption key reference
    pub key_reference: Option<String>,
    /// Created by device
    pub created_by_device: ActionHash,
    /// Single use
    pub single_use: bool,
    /// Use count
    pub use_count: u32,
    /// Max uses (if not single use)
    pub max_uses: Option<u32>,
    /// Expires at
    pub expires_at: Timestamp,
    /// Is active
    pub is_active: bool,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Biometric authentication record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct BiometricAuth {
    /// Auth ID
    pub auth_id: String,
    /// Device hash
    pub device_hash: ActionHash,
    /// Biometric type (fingerprint, face, iris)
    pub biometric_type: String,
    /// Template hash (not the actual biometric data)
    pub template_hash: String,
    /// Is enabled
    pub is_enabled: bool,
    /// Failed attempts
    pub failed_attempts: u32,
    /// Locked until
    pub locked_until: Option<Timestamp>,
    /// Last used
    pub last_used: Option<Timestamp>,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Bandwidth usage tracking
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct BandwidthUsage {
    /// Usage ID
    pub usage_id: String,
    /// Device hash
    pub device_hash: ActionHash,
    /// Period start
    pub period_start: Timestamp,
    /// Period end
    pub period_end: Timestamp,
    /// Bytes uploaded
    pub bytes_uploaded: u64,
    /// Bytes downloaded
    pub bytes_downloaded: u64,
    /// Entries synced
    pub entries_synced: u32,
    /// Conflicts resolved
    pub conflicts_resolved: u32,
    /// Average latency (ms)
    pub avg_latency_ms: u32,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Mobile-specific health data snapshot (for offline emergency access)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct EmergencyDataSnapshot {
    /// Snapshot ID
    pub snapshot_id: String,
    /// Patient hash
    pub patient_hash: ActionHash,
    /// Device hash
    pub device_hash: ActionHash,
    /// Blood type
    pub blood_type: Option<String>,
    /// Critical allergies (JSON array)
    pub allergies: String,
    /// Current medications (JSON array)
    pub medications: String,
    /// Critical conditions (JSON array)
    pub conditions: String,
    /// Emergency contacts (JSON array)
    pub emergency_contacts: String,
    /// Insurance info (JSON)
    pub insurance_info: Option<String>,
    /// Advance directives summary
    pub advance_directives: Option<String>,
    /// Organ donor status
    pub organ_donor: Option<bool>,
    /// QR code for emergency access
    pub emergency_qr: Option<String>,
    /// Last updated
    pub updated_at: Timestamp,
}

/// Entry types for the mobile support zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    RegisteredDevice(RegisteredDevice),
    SyncCheckpoint(SyncCheckpoint),
    SyncQueueEntry(SyncQueueEntry),
    DataConflict(DataConflict),
    OfflineCache(OfflineCache),
    DeltaSync(DeltaSync),
    PushRegistration(PushRegistration),
    PendingNotification(PendingNotification),
    QrCodeRecord(QrCodeRecord),
    BiometricAuth(BiometricAuth),
    BandwidthUsage(BandwidthUsage),
    EmergencyDataSnapshot(EmergencyDataSnapshot),
}

/// Link types for the mobile support zome
#[hdk_link_types]
pub enum LinkTypes {
    /// Agent to devices
    AgentToDevices,
    /// Device to sync checkpoints
    DeviceToCheckpoints,
    /// Device to sync queue
    DeviceToQueue,
    /// Device to conflicts
    DeviceToConflicts,
    /// Device to cache
    DeviceToCache,
    /// Device to notifications
    DeviceToNotifications,
    /// Active QR codes
    ActiveQrCodes,
    /// Device to biometric
    DeviceToBiometric,
    /// Device bandwidth history
    DeviceToBandwidth,
    /// Patient to emergency snapshot
    PatientToEmergencySnapshot,
}

/// Validation callbacks
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::RegisteredDevice(device) => validate_device(&device),
                EntryTypes::DataConflict(conflict) => validate_conflict(&conflict),
                EntryTypes::PendingNotification(notif) => validate_notification(&notif),
                _ => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_device(device: &RegisteredDevice) -> ExternResult<ValidateCallbackResult> {
    if device.device_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Device ID is required".to_string(),
        ));
    }
    if device.device_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Device name is required".to_string(),
        ));
    }
    if device.public_key.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Device public key is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_conflict(conflict: &DataConflict) -> ExternResult<ValidateCallbackResult> {
    if conflict.conflict_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Conflict ID is required".to_string(),
        ));
    }
    if conflict.entry_type.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Entry type is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_notification(notif: &PendingNotification) -> ExternResult<ValidateCallbackResult> {
    if notif.notification_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Notification ID is required".to_string(),
        ));
    }
    if notif.title.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Notification title is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
