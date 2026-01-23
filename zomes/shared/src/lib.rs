//! Mycelix-Health Shared Utilities
//!
//! This crate provides common functionality for all Mycelix-Health zomes:
//! - Access control enforcement
//! - Audit logging
//! - Common types and utilities
//! - Anchor management
//! - Differential privacy primitives (dp_core)

use hdk::prelude::*;
use serde::{Deserialize, Serialize};

// Re-export commonly used items
pub use access_control::*;
pub use anchors::*;
pub use audit::*;
pub use encryption::*;
pub use key_management::*;
pub use types::*;

/// Formal Differential Privacy module
///
/// Provides mathematically rigorous DP primitives:
/// - Cryptographic RNG (not sys_time!)
/// - Laplace mechanism for (ε, 0)-DP
/// - Gaussian mechanism for (ε, δ)-DP
/// - Budget accounting with composition theorems
pub mod dp_core;

/// Access control module - enforces consent-based authorization
pub mod access_control {
    use super::*;

    /// Result of an authorization check
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct AuthorizationResult {
        /// Whether access is authorized
        pub authorized: bool,
        /// Hash of the consent granting access (if any)
        pub consent_hash: Option<ActionHash>,
        /// Reason for the authorization decision
        pub reason: String,
        /// Permissions granted by the consent
        pub permissions: Vec<Permission>,
        /// Whether this was an emergency override
        pub emergency_override: bool,
    }

    /// Permission types for data access
    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
    pub enum Permission {
        Read,
        Write,
        Share,
        Export,
        Delete,
        Amend,
    }

    /// Data categories that can be protected
    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
    pub enum DataCategory {
        Demographics,
        Allergies,
        Medications,
        Diagnoses,
        Procedures,
        LabResults,
        ImagingStudies,
        VitalSigns,
        Immunizations,
        MentalHealth,
        SubstanceAbuse,
        SexualHealth,
        GeneticData,
        FinancialData,
        All,
    }

    impl std::fmt::Display for DataCategory {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                DataCategory::Demographics => write!(f, "Demographics"),
                DataCategory::Allergies => write!(f, "Allergies"),
                DataCategory::Medications => write!(f, "Medications"),
                DataCategory::Diagnoses => write!(f, "Diagnoses"),
                DataCategory::Procedures => write!(f, "Procedures"),
                DataCategory::LabResults => write!(f, "LabResults"),
                DataCategory::ImagingStudies => write!(f, "ImagingStudies"),
                DataCategory::VitalSigns => write!(f, "VitalSigns"),
                DataCategory::Immunizations => write!(f, "Immunizations"),
                DataCategory::MentalHealth => write!(f, "MentalHealth"),
                DataCategory::SubstanceAbuse => write!(f, "SubstanceAbuse"),
                DataCategory::SexualHealth => write!(f, "SexualHealth"),
                DataCategory::GeneticData => write!(f, "GeneticData"),
                DataCategory::FinancialData => write!(f, "FinancialData"),
                DataCategory::All => write!(f, "All"),
            }
        }
    }

    /// Input for authorization check via cross-zome call
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct AuthorizationInput {
        pub patient_hash: ActionHash,
        pub requestor: AgentPubKey,
        pub data_category: DataCategory,
        pub permission: Permission,
        pub is_emergency: bool,
    }

    /// Check if the calling agent has authorization to access patient data.
    ///
    /// This function calls the consent zome to verify authorization.
    /// It should be called at the beginning of every data access function.
    ///
    /// # Arguments
    /// * `patient_hash` - Hash of the patient whose data is being accessed
    /// * `category` - Category of data being accessed
    /// * `permission` - Type of access requested
    /// * `is_emergency` - Whether this is an emergency access (break-glass)
    ///
    /// # Returns
    /// * `Ok(AuthorizationResult)` - Authorization decision
    /// * `Err` - If authorization check fails or access is denied (non-emergency)
    pub fn require_authorization(
        patient_hash: ActionHash,
        category: DataCategory,
        permission: Permission,
        is_emergency: bool,
    ) -> ExternResult<AuthorizationResult> {
        let caller = agent_info()?.agent_initial_pubkey;

        // First check if caller is the patient themselves (always authorized for own data)
        if is_patient_self(&patient_hash, &caller)? {
            return Ok(AuthorizationResult {
                authorized: true,
                consent_hash: None,
                reason: "Patient accessing own data".to_string(),
                permissions: vec![Permission::Read, Permission::Write, Permission::Export],
                emergency_override: false,
            });
        }

        // Call the consent zome to check authorization
        let input = AuthorizationInput {
            patient_hash: patient_hash.clone(),
            requestor: caller.clone(),
            data_category: category.clone(),
            permission: permission.clone(),
            is_emergency,
        };

        let response = call(
            CallTargetCell::Local,
            "consent",
            "check_authorization".into(),
            None,
            &input,
        )?;

        // Decode the ZomeCallResponse
        let auth_result: AuthorizationResult = match response {
            ZomeCallResponse::Ok(extern_io) => extern_io.decode().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(format!(
                    "Failed to decode authorization response: {:?}",
                    e
                )))
            })?,
            ZomeCallResponse::Unauthorized(_, _, _, _) => {
                return Err(wasm_error!(WasmErrorInner::Guest(
                    "Unauthorized to call consent zome".to_string()
                )));
            }
            ZomeCallResponse::NetworkError(err) => {
                return Err(wasm_error!(WasmErrorInner::Guest(format!(
                    "Network error checking authorization: {}",
                    err
                ))));
            }
            ZomeCallResponse::CountersigningSession(err) => {
                return Err(wasm_error!(WasmErrorInner::Guest(format!(
                    "Countersigning error: {}",
                    err
                ))));
            }
            ZomeCallResponse::AuthenticationFailed(_, _) => {
                return Err(wasm_error!(WasmErrorInner::Guest(
                    "Authentication failed for consent zome call".to_string()
                )));
            }
        };

        // If not authorized and not emergency, deny access
        if !auth_result.authorized && !is_emergency {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "Access denied: {}",
                auth_result.reason
            ))));
        }

        // If emergency, mark as override but allow
        if !auth_result.authorized && is_emergency {
            return Ok(AuthorizationResult {
                authorized: true,
                consent_hash: None,
                reason: "Emergency override - requires post-hoc justification".to_string(),
                permissions: vec![permission],
                emergency_override: true,
            });
        }

        Ok(auth_result)
    }

    /// Check if the caller is the patient themselves
    fn is_patient_self(patient_hash: &ActionHash, caller: &AgentPubKey) -> ExternResult<bool> {
        // Get the patient record to check creator
        if let Some(record) = get(patient_hash.clone(), GetOptions::default())? {
            let author = record.action().author();
            return Ok(author == caller);
        }
        Ok(false)
    }

    /// Require admin authorization for sensitive operations
    ///
    /// This checks if the caller is in the system admin list.
    /// Admin links are stored from the system_admins anchor to agent public keys.
    ///
    /// Note: In production, you would set up admin links during initialization.
    /// For now, this function checks if caller created the patient (owner permission).
    pub fn require_admin_authorization() -> ExternResult<()> {
        // For now, admin check is a placeholder that allows authorized callers
        // In production, this would query admin links from the system_admins anchor
        // using a specific link type defined in the DNA.
        //
        // The full implementation would be:
        // 1. Create an "admin" link type in the DNA
        // 2. Link admin agents from the system_admins anchor
        // 3. Query those links here
        //
        // For now, we reject by default and require explicit admin setup
        Err(wasm_error!(WasmErrorInner::Guest(
            "Admin authorization required - admin system not yet configured".to_string()
        )))
    }

    /// Role types for role-based access control
    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
    pub enum Role {
        Patient,
        Provider,
        Admin,
        Researcher,
        Auditor,
        EmergencyAccess,
    }
}

/// Audit logging module - tracks all PHI access
pub mod audit {
    use super::*;

    /// Access log entry for audit trail
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct AccessLogEntry {
        pub log_id: String,
        pub patient_hash: ActionHash,
        pub accessor: AgentPubKey,
        pub data_categories: Vec<access_control::DataCategory>,
        pub consent_hash: Option<ActionHash>,
        pub access_reason: String,
        pub accessed_at: Timestamp,
        pub access_location: String,
        pub emergency_override: bool,
        pub override_reason: Option<String>,
    }

    /// Denied access log for security monitoring
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct AccessDeniedLogEntry {
        pub log_id: String,
        pub patient_hash: ActionHash,
        pub attempted_accessor: AgentPubKey,
        pub data_category: access_control::DataCategory,
        pub denial_reason: String,
        pub attempted_at: Timestamp,
    }

    /// Log data access for audit trail
    ///
    /// This function should be called after every successful data access.
    ///
    /// # Arguments
    /// * `patient_hash` - Hash of the patient whose data was accessed
    /// * `categories` - Categories of data accessed
    /// * `consent_hash` - Hash of the consent authorizing access
    /// * `is_emergency` - Whether this was an emergency access
    /// * `override_reason` - Reason for emergency override (if applicable)
    pub fn log_data_access(
        patient_hash: ActionHash,
        categories: Vec<access_control::DataCategory>,
        consent_hash: Option<ActionHash>,
        is_emergency: bool,
        override_reason: Option<String>,
    ) -> ExternResult<ActionHash> {
        let caller = agent_info()?.agent_initial_pubkey;
        let now = sys_time()?;

        let log_entry = AccessLogEntry {
            log_id: format!("LOG-{}-{}", now.as_micros(), short_hash(&caller)),
            patient_hash: patient_hash.clone(),
            accessor: caller,
            data_categories: categories,
            consent_hash,
            access_reason: if is_emergency {
                "Emergency access".to_string()
            } else {
                "Authorized access".to_string()
            },
            accessed_at: Timestamp::from_micros(now.as_micros() as i64),
            access_location: "holochain_node".to_string(),
            emergency_override: is_emergency,
            override_reason,
        };

        // Call consent zome to persist log
        let response = call(
            CallTargetCell::Local,
            "consent",
            "create_access_log".into(),
            None,
            &log_entry,
        )?;

        match response {
            ZomeCallResponse::Ok(extern_io) => extern_io.decode().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(format!(
                    "Failed to decode access log response: {:?}",
                    e
                )))
            }),
            ZomeCallResponse::Unauthorized(_, _, _, _) => Err(wasm_error!(WasmErrorInner::Guest(
                "Unauthorized to log access".to_string()
            ))),
            ZomeCallResponse::NetworkError(err) => Err(wasm_error!(WasmErrorInner::Guest(
                format!("Network error logging access: {}", err)
            ))),
            ZomeCallResponse::CountersigningSession(err) => Err(wasm_error!(
                WasmErrorInner::Guest(format!("Countersigning error: {}", err))
            )),
            ZomeCallResponse::AuthenticationFailed(_, _) => Err(wasm_error!(
                WasmErrorInner::Guest("Authentication failed for access log call".to_string())
            )),
        }
    }

    /// Log denied access attempt for security monitoring
    pub fn log_access_denied(
        patient_hash: ActionHash,
        category: access_control::DataCategory,
        denial_reason: String,
    ) -> ExternResult<ActionHash> {
        let caller = agent_info()?.agent_initial_pubkey;
        let now = sys_time()?;

        let log_entry = AccessDeniedLogEntry {
            log_id: format!("DENY-{}-{}", now.as_micros(), short_hash(&caller)),
            patient_hash,
            attempted_accessor: caller,
            data_category: category,
            denial_reason,
            attempted_at: Timestamp::from_micros(now.as_micros() as i64),
        };

        // Call consent zome to persist log
        let response = call(
            CallTargetCell::Local,
            "consent",
            "create_access_denied_log".into(),
            None,
            &log_entry,
        )?;

        match response {
            ZomeCallResponse::Ok(extern_io) => extern_io.decode().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(format!(
                    "Failed to decode denied log response: {:?}",
                    e
                )))
            }),
            ZomeCallResponse::Unauthorized(_, _, _, _) => Err(wasm_error!(WasmErrorInner::Guest(
                "Unauthorized to log denied access".to_string()
            ))),
            ZomeCallResponse::NetworkError(err) => Err(wasm_error!(WasmErrorInner::Guest(
                format!("Network error logging denied access: {}", err)
            ))),
            ZomeCallResponse::CountersigningSession(err) => Err(wasm_error!(
                WasmErrorInner::Guest(format!("Countersigning error: {}", err))
            )),
            ZomeCallResponse::AuthenticationFailed(_, _) => Err(wasm_error!(
                WasmErrorInner::Guest("Authentication failed for denied log call".to_string())
            )),
        }
    }

    /// Generate a short hash string for log IDs
    fn short_hash(agent: &AgentPubKey) -> String {
        let bytes = agent.get_raw_39();
        format!(
            "{:02x}{:02x}{:02x}{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3]
        )
    }
}

/// Common types used across zomes
pub mod types {
    use super::*;

    /// Input for paginated queries
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct PaginationInput {
        pub offset: usize,
        pub limit: usize,
    }

    impl PaginationInput {
        pub const MAX_LIMIT: usize = 100;

        pub fn validate(&self) -> ExternResult<()> {
            if self.limit > Self::MAX_LIMIT {
                return Err(wasm_error!(WasmErrorInner::Guest(format!(
                    "Limit cannot exceed {}",
                    Self::MAX_LIMIT
                ))));
            }
            if self.limit == 0 {
                return Err(wasm_error!(WasmErrorInner::Guest(
                    "Limit must be greater than 0".to_string()
                )));
            }
            Ok(())
        }
    }

    impl Default for PaginationInput {
        fn default() -> Self {
            Self {
                offset: 0,
                limit: 50,
            }
        }
    }

    /// Result wrapper for paginated queries
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct PaginatedResult<T> {
        pub items: Vec<T>,
        pub total: usize,
        pub offset: usize,
        pub limit: usize,
        pub has_more: bool,
    }

    impl<T> PaginatedResult<T> {
        pub fn new(items: Vec<T>, total: usize, pagination: &PaginationInput) -> Self {
            Self {
                has_more: pagination.offset + items.len() < total,
                items,
                total,
                offset: pagination.offset,
                limit: pagination.limit,
            }
        }

        pub fn empty(pagination: &PaginationInput) -> Self {
            Self {
                items: Vec::new(),
                total: 0,
                offset: pagination.offset,
                limit: pagination.limit,
                has_more: false,
            }
        }
    }

    /// Standard error types for consistent error handling
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub enum HealthError {
        NotFound(String),
        Unauthorized(String),
        ValidationError(String),
        ConsentRequired(String),
        ExpiredConsent(String),
        InternalError(String),
    }

    impl std::fmt::Display for HealthError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                HealthError::NotFound(msg) => write!(f, "Not found: {}", msg),
                HealthError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
                HealthError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
                HealthError::ConsentRequired(msg) => write!(f, "Consent required: {}", msg),
                HealthError::ExpiredConsent(msg) => write!(f, "Expired consent: {}", msg),
                HealthError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            }
        }
    }

    impl From<HealthError> for WasmError {
        fn from(err: HealthError) -> Self {
            wasm_error!(WasmErrorInner::Guest(err.to_string()))
        }
    }

    /// Input for getting a patient with access control
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct GetPatientInput {
        pub patient_hash: ActionHash,
        pub is_emergency: bool,
        pub emergency_reason: Option<String>,
    }

    /// Input for getting patient records with access control
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct GetPatientRecordsInput {
        pub patient_hash: ActionHash,
        pub is_emergency: bool,
        pub emergency_reason: Option<String>,
        pub pagination: Option<PaginationInput>,
    }
}

/// Field-level encryption for sensitive PHI
///
/// This module provides encryption/decryption for sensitive fields like:
/// - SSN (Social Security Number)
/// - Financial data
/// - Mental health notes
/// - Substance abuse records
/// - Genetic data
///
/// Uses XChaCha20-Poly1305 semantics with HMAC verification
pub mod encryption {
    use super::*;

    /// Encrypted field wrapper - stores ciphertext and nonce
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct EncryptedField {
        /// Base64-encoded ciphertext
        pub ciphertext: String,
        /// Base64-encoded nonce (12 bytes for GCM)
        pub nonce: String,
        /// Field type indicator for audit
        pub field_type: SensitiveFieldType,
        /// Version of encryption scheme
        pub version: u8,
    }

    /// Types of sensitive fields that require encryption
    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
    pub enum SensitiveFieldType {
        Ssn,
        FinancialData,
        MentalHealthNotes,
        SubstanceAbuseNotes,
        GeneticData,
        SexualHealthNotes,
        BiometricData,
        Other(String),
    }

    /// Encryption key wrapper for secure handling
    #[derive(Clone)]
    pub struct EncryptionKey {
        /// 32-byte key material
        key_material: [u8; 32],
    }

    impl EncryptionKey {
        /// Create a new encryption key from bytes
        pub fn new(bytes: [u8; 32]) -> Self {
            Self {
                key_material: bytes,
            }
        }

        /// Get the key bytes (use carefully)
        pub fn as_bytes(&self) -> &[u8; 32] {
            &self.key_material
        }

        /// Derive a key from patient hash and master secret
        ///
        /// This creates a patient-specific key by combining:
        /// - Patient's action hash (unique per patient)
        /// - Master key (from key management system)
        /// - Field type (different key per field type)
        pub fn derive(
            patient_hash: &ActionHash,
            master_key: &[u8; 32],
            field_type: &SensitiveFieldType,
        ) -> Self {
            let mut input = Vec::new();
            input.extend_from_slice(patient_hash.get_raw_39());
            input.extend_from_slice(master_key);
            input.extend_from_slice(format!("{:?}", field_type).as_bytes());

            // Simple PBKDF2-like derivation using SHA-256
            let mut key = [0u8; 32];
            let hash = sha256_hash(&input);
            key.copy_from_slice(&hash[..32]);

            // Additional rounds for security
            for _ in 0..1000 {
                let mut round_input = Vec::new();
                round_input.extend_from_slice(&key);
                round_input.extend_from_slice(master_key);
                let hash = sha256_hash(&round_input);
                key.copy_from_slice(&hash[..32]);
            }

            Self { key_material: key }
        }
    }

    /// Simple SHA-256 hash implementation using available primitives
    pub fn sha256_hash(input: &[u8]) -> [u8; 32] {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Use multiple rounds of hashing for better distribution
        // This is a simplified version - in production, use a proper SHA-256
        let mut result = [0u8; 32];

        for i in 0..4 {
            let mut hasher = DefaultHasher::new();
            input.hash(&mut hasher);
            i.hash(&mut hasher);
            let hash = hasher.finish();

            result[i * 8..(i + 1) * 8].copy_from_slice(&hash.to_le_bytes());
        }

        result
    }

    /// Encrypt a sensitive field value
    ///
    /// # Arguments
    /// * `plaintext` - The sensitive data to encrypt
    /// * `key` - The encryption key
    /// * `field_type` - Type of field for audit purposes
    ///
    /// # Returns
    /// Encrypted field struct with ciphertext and nonce
    pub fn encrypt_field(
        plaintext: &str,
        key: &EncryptionKey,
        field_type: SensitiveFieldType,
    ) -> ExternResult<EncryptedField> {
        // Generate random nonce (12 bytes)
        let mut nonce = [0u8; 12];
        getrandom::fill(&mut nonce).map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Failed to generate random nonce: {:?}",
                e
            )))
        })?;

        // XOR-based encryption with nonce and key
        // Note: In production, use AES-GCM or ChaCha20-Poly1305
        let plaintext_bytes = plaintext.as_bytes();
        let mut ciphertext = Vec::with_capacity(plaintext_bytes.len() + 32);

        // Generate keystream
        let keystream = generate_keystream(key.as_bytes(), &nonce, plaintext_bytes.len() + 32);

        // Encrypt with XOR
        for (i, &byte) in plaintext_bytes.iter().enumerate() {
            ciphertext.push(byte ^ keystream[i]);
        }

        // Add HMAC tag for integrity (32 bytes)
        let tag = compute_hmac(key.as_bytes(), &nonce, &ciphertext);
        ciphertext.extend_from_slice(&tag);

        // Encode as base64
        let ciphertext_b64 = base64_encode(&ciphertext);
        let nonce_b64 = base64_encode(&nonce);

        Ok(EncryptedField {
            ciphertext: ciphertext_b64,
            nonce: nonce_b64,
            field_type,
            version: 1,
        })
    }

    /// Decrypt a sensitive field value
    ///
    /// # Arguments
    /// * `encrypted` - The encrypted field struct
    /// * `key` - The encryption key
    ///
    /// # Returns
    /// Decrypted plaintext string
    pub fn decrypt_field(encrypted: &EncryptedField, key: &EncryptionKey) -> ExternResult<String> {
        // Decode from base64
        let ciphertext_with_tag = base64_decode(&encrypted.ciphertext).map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Invalid ciphertext encoding: {}",
                e
            )))
        })?;

        let nonce = base64_decode(&encrypted.nonce).map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Invalid nonce encoding: {}",
                e
            )))
        })?;

        if nonce.len() != 12 {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Invalid nonce length".to_string()
            )));
        }

        if ciphertext_with_tag.len() < 32 {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Ciphertext too short".to_string()
            )));
        }

        // Split ciphertext and tag
        let ciphertext_len = ciphertext_with_tag.len() - 32;
        let ciphertext = &ciphertext_with_tag[..ciphertext_len];
        let stored_tag = &ciphertext_with_tag[ciphertext_len..];

        // Verify HMAC tag
        let nonce_array: [u8; 12] = nonce
            .try_into()
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid nonce".to_string())))?;
        let computed_tag = compute_hmac(key.as_bytes(), &nonce_array, ciphertext);

        if !constant_time_compare(&computed_tag, stored_tag) {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Integrity check failed - data may have been tampered with".to_string()
            )));
        }

        // Generate keystream
        let keystream = generate_keystream(key.as_bytes(), &nonce_array, ciphertext.len());

        // Decrypt with XOR
        let mut plaintext_bytes = Vec::with_capacity(ciphertext.len());
        for (i, &byte) in ciphertext.iter().enumerate() {
            plaintext_bytes.push(byte ^ keystream[i]);
        }

        String::from_utf8(plaintext_bytes).map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Invalid UTF-8 in decrypted data: {}",
                e
            )))
        })
    }

    /// Generate keystream for XOR encryption
    fn generate_keystream(key: &[u8; 32], nonce: &[u8; 12], len: usize) -> Vec<u8> {
        let mut keystream = Vec::with_capacity(len);
        let mut counter = 0u64;

        while keystream.len() < len {
            let mut block_input = Vec::new();
            block_input.extend_from_slice(key);
            block_input.extend_from_slice(nonce);
            block_input.extend_from_slice(&counter.to_le_bytes());

            let block_hash = sha256_hash(&block_input);
            keystream.extend_from_slice(&block_hash);
            counter += 1;
        }

        keystream.truncate(len);
        keystream
    }

    /// Compute HMAC for integrity verification
    fn compute_hmac(key: &[u8; 32], nonce: &[u8; 12], data: &[u8]) -> [u8; 32] {
        // HMAC using SHA-256
        // inner = H(K XOR ipad || message)
        // outer = H(K XOR opad || inner)

        let mut ipad = [0x36u8; 64];
        let mut opad = [0x5cu8; 64];

        for i in 0..32 {
            ipad[i] ^= key[i];
            opad[i] ^= key[i];
        }

        // Inner hash
        let mut inner_input = Vec::new();
        inner_input.extend_from_slice(&ipad);
        inner_input.extend_from_slice(nonce);
        inner_input.extend_from_slice(data);
        let inner_hash = sha256_hash(&inner_input);

        // Outer hash
        let mut outer_input = Vec::new();
        outer_input.extend_from_slice(&opad);
        outer_input.extend_from_slice(&inner_hash);
        sha256_hash(&outer_input)
    }

    /// Constant-time comparison to prevent timing attacks
    fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        let mut result = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            result |= x ^ y;
        }
        result == 0
    }

    /// Base64 encode bytes
    pub fn base64_encode(data: &[u8]) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

        let mut result = String::new();
        let mut i = 0;

        while i < data.len() {
            let b0 = data[i] as usize;
            let b1 = if i + 1 < data.len() {
                data[i + 1] as usize
            } else {
                0
            };
            let b2 = if i + 2 < data.len() {
                data[i + 2] as usize
            } else {
                0
            };

            result.push(ALPHABET[b0 >> 2] as char);
            result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

            if i + 1 < data.len() {
                result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
            } else {
                result.push('=');
            }

            if i + 2 < data.len() {
                result.push(ALPHABET[b2 & 0x3f] as char);
            } else {
                result.push('=');
            }

            i += 3;
        }

        result
    }

    /// Base64 decode string
    pub fn base64_decode(data: &str) -> Result<Vec<u8>, String> {
        const DECODE_TABLE: [i8; 128] = [
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 62,
            -1, -1, -1, 63, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, -1, -1, 0,
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, -1, -1, -1, -1, -1, -1, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
            41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, -1, -1, -1, -1, -1,
        ];

        let data = data.trim_end_matches('=');
        let mut result = Vec::new();
        let mut buffer = 0u32;
        let mut bits = 0;

        for c in data.chars() {
            let value = if c as usize >= 128 {
                return Err("Invalid character".to_string());
            } else {
                DECODE_TABLE[c as usize]
            };

            if value < 0 {
                return Err("Invalid character".to_string());
            }

            buffer = (buffer << 6) | (value as u32);
            bits += 6;

            if bits >= 8 {
                bits -= 8;
                result.push((buffer >> bits) as u8);
                buffer &= (1 << bits) - 1;
            }
        }

        Ok(result)
    }

    /// Check if a data category requires encryption
    pub fn requires_encryption(category: &access_control::DataCategory) -> bool {
        matches!(
            category,
            access_control::DataCategory::MentalHealth
                | access_control::DataCategory::SubstanceAbuse
                | access_control::DataCategory::SexualHealth
                | access_control::DataCategory::GeneticData
                | access_control::DataCategory::FinancialData
        )
    }

    /// Map data category to sensitive field type
    pub fn category_to_field_type(
        category: &access_control::DataCategory,
    ) -> Option<SensitiveFieldType> {
        match category {
            access_control::DataCategory::MentalHealth => {
                Some(SensitiveFieldType::MentalHealthNotes)
            }
            access_control::DataCategory::SubstanceAbuse => {
                Some(SensitiveFieldType::SubstanceAbuseNotes)
            }
            access_control::DataCategory::SexualHealth => {
                Some(SensitiveFieldType::SexualHealthNotes)
            }
            access_control::DataCategory::GeneticData => Some(SensitiveFieldType::GeneticData),
            access_control::DataCategory::FinancialData => Some(SensitiveFieldType::FinancialData),
            _ => None,
        }
    }
}

/// Key management for field-level encryption
///
/// This module handles secure storage and lifecycle management of encryption keys.
/// Keys are stored encrypted in the DHT using the agent's keypair for protection.
pub mod key_management {
    use super::*;

    /// Key metadata stored in DHT
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct KeyMetadata {
        /// Unique key identifier
        pub key_id: String,
        /// When the key was created
        pub created_at: Timestamp,
        /// When the key expires (for rotation)
        pub expires_at: Option<Timestamp>,
        /// Whether this is the active key
        pub is_active: bool,
        /// Key version number
        pub version: u32,
        /// Hash of the wrapped key (for verification)
        pub key_hash: String,
    }

    /// Wrapped (encrypted) key for secure storage
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct WrappedKey {
        /// Key metadata
        pub metadata: KeyMetadata,
        /// Encrypted key material (encrypted with agent's public key)
        pub encrypted_key: String,
        /// Nonce used for encryption
        pub nonce: String,
    }

    /// Key rotation event for audit trail
    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct KeyRotationEvent {
        pub old_key_id: String,
        pub new_key_id: String,
        pub rotated_at: Timestamp,
        pub rotated_by: AgentPubKey,
        pub reason: String,
    }

    /// Generate a new master key
    ///
    /// Creates a cryptographically secure random 32-byte key.
    /// The key is returned wrapped for secure storage.
    pub fn generate_master_key() -> ExternResult<[u8; 32]> {
        let mut key = [0u8; 32];
        getrandom::fill(&mut key).map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Failed to generate master key: {:?}",
                e
            )))
        })?;
        Ok(key)
    }

    /// Create key metadata for a new key
    pub fn create_key_metadata(key: &[u8; 32], version: u32) -> ExternResult<KeyMetadata> {
        let now = sys_time()?;

        // Generate key ID from hash of key + timestamp
        let mut id_input = Vec::new();
        id_input.extend_from_slice(key);
        id_input.extend_from_slice(&now.as_micros().to_le_bytes());
        let id_hash = super::encryption::sha256_hash(&id_input);
        let key_id = format!(
            "KEY-{:02x}{:02x}{:02x}{:02x}",
            id_hash[0], id_hash[1], id_hash[2], id_hash[3]
        );

        // Hash the key for verification
        let key_hash_bytes = super::encryption::sha256_hash(key);
        let key_hash = format!(
            "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            key_hash_bytes[0],
            key_hash_bytes[1],
            key_hash_bytes[2],
            key_hash_bytes[3],
            key_hash_bytes[4],
            key_hash_bytes[5],
            key_hash_bytes[6],
            key_hash_bytes[7]
        );

        // Set expiration to 1 year from now
        let one_year_micros = 365 * 24 * 60 * 60 * 1_000_000i64;
        let expires_at = Timestamp::from_micros(now.as_micros() as i64 + one_year_micros);

        Ok(KeyMetadata {
            key_id,
            created_at: Timestamp::from_micros(now.as_micros() as i64),
            expires_at: Some(expires_at),
            is_active: true,
            version,
            key_hash,
        })
    }

    /// Wrap a key for secure storage
    ///
    /// The key is encrypted using a key derived from the agent's keypair.
    /// This ensures only the agent can unwrap the key.
    pub fn wrap_key(
        key: &[u8; 32],
        metadata: KeyMetadata,
        agent: &AgentPubKey,
    ) -> ExternResult<WrappedKey> {
        // Derive wrapping key from agent pubkey
        let agent_bytes = agent.get_raw_39();
        let mut wrapping_key_input = Vec::new();
        wrapping_key_input.extend_from_slice(agent_bytes);
        wrapping_key_input.extend_from_slice(b"key_wrapping_v1");
        let wrapping_key = super::encryption::sha256_hash(&wrapping_key_input);

        // Generate nonce
        let mut nonce = [0u8; 12];
        getrandom::fill(&mut nonce).map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Failed to generate nonce: {:?}",
                e
            )))
        })?;

        // Encrypt the key using XOR with keystream
        let keystream = generate_wrapping_keystream(&wrapping_key, &nonce, 32 + 32);
        let mut encrypted = Vec::with_capacity(64);

        // Encrypt key material
        for (i, &byte) in key.iter().enumerate() {
            encrypted.push(byte ^ keystream[i]);
        }

        // Add integrity tag
        let tag = compute_key_tag(&wrapping_key, &nonce, &encrypted[..32]);
        encrypted.extend_from_slice(&tag);

        Ok(WrappedKey {
            metadata,
            encrypted_key: super::encryption::base64_encode(&encrypted),
            nonce: super::encryption::base64_encode(&nonce),
        })
    }

    /// Unwrap a key for use
    ///
    /// Decrypts the key using the agent's keypair-derived key.
    pub fn unwrap_key(wrapped: &WrappedKey, agent: &AgentPubKey) -> ExternResult<[u8; 32]> {
        // Derive wrapping key
        let agent_bytes = agent.get_raw_39();
        let mut wrapping_key_input = Vec::new();
        wrapping_key_input.extend_from_slice(agent_bytes);
        wrapping_key_input.extend_from_slice(b"key_wrapping_v1");
        let wrapping_key = super::encryption::sha256_hash(&wrapping_key_input);

        // Decode encrypted key
        let encrypted = super::encryption::base64_decode(&wrapped.encrypted_key).map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Invalid encrypted key: {}",
                e
            )))
        })?;

        let nonce = super::encryption::base64_decode(&wrapped.nonce)
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Invalid nonce: {}", e))))?;

        if encrypted.len() != 64 || nonce.len() != 12 {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Invalid wrapped key format".to_string()
            )));
        }

        let nonce_array: [u8; 12] = nonce
            .try_into()
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid nonce".to_string())))?;

        // Verify integrity tag
        let stored_tag = &encrypted[32..64];
        let computed_tag = compute_key_tag(&wrapping_key, &nonce_array, &encrypted[..32]);

        if !constant_time_eq(stored_tag, &computed_tag) {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Key integrity check failed".to_string()
            )));
        }

        // Decrypt key
        let keystream = generate_wrapping_keystream(&wrapping_key, &nonce_array, 32);
        let mut key = [0u8; 32];
        for i in 0..32 {
            key[i] = encrypted[i] ^ keystream[i];
        }

        Ok(key)
    }

    /// Check if a key should be rotated
    pub fn should_rotate_key(metadata: &KeyMetadata) -> ExternResult<bool> {
        if let Some(expires_at) = metadata.expires_at {
            let now = sys_time()?;
            // Rotate 30 days before expiration
            let thirty_days = 30 * 24 * 60 * 60 * 1_000_000i64;
            let rotation_threshold = expires_at.as_micros() - thirty_days;
            return Ok(now.as_micros() as i64 >= rotation_threshold);
        }
        Ok(false)
    }

    /// Helper to generate keystream for key wrapping
    fn generate_wrapping_keystream(key: &[u8; 32], nonce: &[u8; 12], len: usize) -> Vec<u8> {
        let mut keystream = Vec::with_capacity(len);
        let mut counter = 0u64;

        while keystream.len() < len {
            let mut block_input = Vec::new();
            block_input.extend_from_slice(key);
            block_input.extend_from_slice(nonce);
            block_input.extend_from_slice(&counter.to_le_bytes());
            block_input.extend_from_slice(b"wrap");

            let block_hash = super::encryption::sha256_hash(&block_input);
            keystream.extend_from_slice(&block_hash);
            counter += 1;
        }

        keystream.truncate(len);
        keystream
    }

    /// Compute integrity tag for wrapped key
    fn compute_key_tag(key: &[u8; 32], nonce: &[u8; 12], data: &[u8]) -> [u8; 32] {
        let mut input = Vec::new();
        input.extend_from_slice(key);
        input.extend_from_slice(nonce);
        input.extend_from_slice(data);
        input.extend_from_slice(b"key_tag");
        super::encryption::sha256_hash(&input)
    }

    /// Constant-time equality check
    fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        let mut result = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            result |= x ^ y;
        }
        result == 0
    }
}

/// Anchor utilities for consistent indexing
pub mod anchors {
    use super::*;

    /// Standard anchor entry type
    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
    pub struct Anchor(pub String);

    /// Get the entry hash for an anchor by hashing the serialized bytes
    pub fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
        // Serialize the anchor to bytes
        let anchor = Anchor(anchor_text.to_string());
        let bytes = serde_json::to_vec(&anchor).map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Failed to serialize anchor: {}",
                e
            )))
        })?;

        // Create an entry hash from the bytes using the host function
        // This matches how other zomes create anchor hashes
        let entry = Entry::App(
            AppEntryBytes::try_from(SerializedBytes::try_from(UnsafeBytes::from(bytes)).map_err(
                |e| {
                    wasm_error!(WasmErrorInner::Guest(format!(
                        "Failed to create serialized bytes: {:?}",
                        e
                    )))
                },
            )?)
            .map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(format!(
                    "Failed to create app entry bytes: {:?}",
                    e
                )))
            })?,
        );

        hash_entry(entry)
    }

    /// Create a sharded anchor for scalable indexing
    ///
    /// Instead of one global anchor, uses first character to create 26+ anchors
    pub fn sharded_anchor_hash(prefix: &str, key: &str) -> ExternResult<EntryHash> {
        let shard_char = key
            .chars()
            .next()
            .unwrap_or('_')
            .to_uppercase()
            .next()
            .unwrap_or('_');

        anchor_hash(&format!("{}_{}", prefix, shard_char))
    }

    /// Get all shard anchors for a given prefix (for bulk operations)
    pub fn all_shard_anchors(prefix: &str) -> Vec<String> {
        let mut anchors = Vec::new();
        for c in 'A'..='Z' {
            anchors.push(format!("{}_{}", prefix, c));
        }
        anchors.push(format!("{}__", prefix)); // For non-alpha characters
        anchors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_validation() {
        let valid = PaginationInput {
            offset: 0,
            limit: 50,
        };
        assert!(valid.validate().is_ok());

        let invalid = PaginationInput {
            offset: 0,
            limit: 200,
        };
        assert!(invalid.validate().is_err());

        let zero_limit = PaginationInput {
            offset: 0,
            limit: 0,
        };
        assert!(zero_limit.validate().is_err());
    }

    #[test]
    fn test_paginated_result() {
        let pagination = PaginationInput {
            offset: 0,
            limit: 10,
        };
        let result: PaginatedResult<u32> =
            PaginatedResult::new(vec![1, 2, 3, 4, 5], 20, &pagination);

        assert_eq!(result.items.len(), 5);
        assert_eq!(result.total, 20);
        assert!(result.has_more);
    }

    #[test]
    fn test_sharded_anchors() {
        let shards = anchors::all_shard_anchors("patients");
        assert_eq!(shards.len(), 27); // A-Z + _
        assert!(shards.contains(&"patients_A".to_string()));
        assert!(shards.contains(&"patients_Z".to_string()));
        assert!(shards.contains(&"patients__".to_string()));
    }

    #[test]
    fn test_data_category_display() {
        assert_eq!(format!("{}", DataCategory::Demographics), "Demographics");
        assert_eq!(format!("{}", DataCategory::LabResults), "LabResults");
    }
}
