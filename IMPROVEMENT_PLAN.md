# Mycelix-Health Improvement Plan

**Version**: 1.0
**Created**: January 21, 2026
**Target Completion**: May 2026 (16 weeks)

---

## Overview

This document provides detailed implementation specifications for addressing the issues identified in the Mycelix-Health Review Report. Each improvement includes code examples, acceptance criteria, and testing requirements.

---

## Phase 1: Security Foundation (Weeks 1-4)

### 1.1 Access Control Middleware

**Objective**: Create a centralized access control system that enforces consent verification before any PHI access.

#### Implementation: Create `zomes/shared/` Crate

```toml
# zomes/shared/Cargo.toml
[package]
name = "mycelix-health-shared"
version = "0.1.0"
edition = "2021"

[dependencies]
hdk = { workspace = true }
hdi = { workspace = true }
serde = { workspace = true }
```

```rust
// zomes/shared/src/lib.rs
use hdk::prelude::*;

/// Standard result type for authorization checks
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthorizationResult {
    pub authorized: bool,
    pub consent_hash: Option<ActionHash>,
    pub reason: String,
    pub permissions: Vec<Permission>,
}

/// Permission types for data access
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    Read,
    Write,
    Share,
    Export,
    Delete,
    Amend,
}

/// Data categories that can be protected
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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

/// Check if the calling agent has authorization to access patient data
/// This function should be called at the beginning of every data access function
pub fn require_authorization(
    patient_hash: ActionHash,
    category: DataCategory,
    permission: Permission,
    is_emergency: bool,
) -> ExternResult<AuthorizationResult> {
    let caller = agent_info()?.agent_initial_pubkey;

    // Call the consent zome to check authorization
    let response: AuthorizationResult = call(
        CallTargetCell::Local,
        "consent",
        "check_authorization".into(),
        None,
        &AuthorizationInput {
            patient_hash,
            requestor: caller,
            data_category: category,
            permission,
            is_emergency,
        },
    )?;

    if !response.authorized && !is_emergency {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("Access denied: {}", response.reason)
        )));
    }

    Ok(response)
}

/// Input structure for authorization checks
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthorizationInput {
    pub patient_hash: ActionHash,
    pub requestor: AgentPubKey,
    pub data_category: DataCategory,
    pub permission: Permission,
    pub is_emergency: bool,
}

/// Standard anchor hash function (centralized)
pub fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    #[hdk_entry_helper]
    #[derive(Clone, PartialEq)]
    pub struct Anchor(pub String);

    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}
```

#### Refactor Coordinator Functions

**Before** (current - INSECURE):
```rust
// zomes/patient/coordinator/src/lib.rs
#[hdk_extern]
pub fn get_patient(hash: ActionHash) -> ExternResult<Option<Record>> {
    get(hash, GetOptions::default())
}
```

**After** (with access control):
```rust
// zomes/patient/coordinator/src/lib.rs
use mycelix_health_shared::{require_authorization, DataCategory, Permission};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetPatientInput {
    pub patient_hash: ActionHash,
    pub is_emergency: bool,
}

#[hdk_extern]
pub fn get_patient(input: GetPatientInput) -> ExternResult<Option<Record>> {
    // REQUIRED: Check authorization before accessing data
    let auth = require_authorization(
        input.patient_hash.clone(),
        DataCategory::Demographics,
        Permission::Read,
        input.is_emergency,
    )?;

    // Log the access (even if emergency)
    log_access(
        input.patient_hash.clone(),
        DataCategory::Demographics,
        auth.consent_hash,
        input.is_emergency,
    )?;

    // Only proceed if authorized
    get(input.patient_hash, GetOptions::default())
}
```

#### Functions Requiring Access Control

| Zome | Function | Category | Priority |
|------|----------|----------|----------|
| patient | `get_patient` | Demographics | CRITICAL |
| patient | `get_all_patients` | Demographics | REMOVE or restrict to admin |
| patient | `search_patients_by_name` | Demographics | CRITICAL |
| patient | `get_patient_by_mrn` | Demographics | CRITICAL |
| records | `get_patient_encounters` | All | CRITICAL |
| records | `get_patient_lab_results` | LabResults | CRITICAL |
| records | `get_patient_vitals` | VitalSigns | CRITICAL |
| records | `get_patient_imaging` | ImagingStudies | CRITICAL |
| prescriptions | `get_patient_prescriptions` | Medications | CRITICAL |
| prescriptions | `get_active_prescriptions` | Medications | CRITICAL |
| trials | `get_trial_participants` | All | HIGH |
| insurance | `get_patient_plans` | FinancialData | HIGH |

#### Acceptance Criteria

- [ ] All PHI access functions check authorization
- [ ] Unauthorized access returns clear error message
- [ ] Emergency access is logged but allowed
- [ ] Unit tests verify authorization denial
- [ ] Integration test verifies consent flow

---

### 1.2 Automatic Audit Logging

**Objective**: Ensure all PHI access is automatically logged for HIPAA compliance.

#### Implementation: Audit Logging Utility

```rust
// zomes/shared/src/audit.rs
use hdk::prelude::*;

/// Log structure for PHI access
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessLogEntry {
    pub log_id: String,
    pub patient_hash: ActionHash,
    pub accessor: AgentPubKey,
    pub data_categories: Vec<DataCategory>,
    pub consent_hash: Option<ActionHash>,
    pub access_reason: String,
    pub accessed_at: Timestamp,
    pub access_location: String,  // IP or system identifier
    pub emergency_override: bool,
    pub override_reason: Option<String>,
}

/// Automatically log data access
pub fn log_access(
    patient_hash: ActionHash,
    category: DataCategory,
    consent_hash: Option<ActionHash>,
    is_emergency: bool,
) -> ExternResult<ActionHash> {
    let caller = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let log_entry = AccessLogEntry {
        log_id: format!("LOG-{}-{}", now.as_micros(), caller),
        patient_hash: patient_hash.clone(),
        accessor: caller,
        data_categories: vec![category],
        consent_hash,
        access_reason: if is_emergency {
            "Emergency access".to_string()
        } else {
            "Authorized access".to_string()
        },
        accessed_at: Timestamp::from_micros(now.as_micros() as i64),
        access_location: "holochain_node".to_string(),
        emergency_override: is_emergency,
        override_reason: if is_emergency {
            Some("Emergency override invoked".to_string())
        } else {
            None
        },
    };

    // Call consent zome to persist log
    call(
        CallTargetCell::Local,
        "consent",
        "log_data_access".into(),
        None,
        &log_entry,
    )
}

/// Log failed access attempts
pub fn log_access_denied(
    patient_hash: ActionHash,
    category: DataCategory,
    denial_reason: String,
) -> ExternResult<ActionHash> {
    let caller = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let log_entry = AccessDeniedLog {
        log_id: format!("DENY-{}-{}", now.as_micros(), caller),
        patient_hash,
        attempted_accessor: caller,
        data_category: category,
        denial_reason,
        attempted_at: Timestamp::from_micros(now.as_micros() as i64),
    };

    call(
        CallTargetCell::Local,
        "consent",
        "log_access_denied".into(),
        None,
        &log_entry,
    )
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessDeniedLog {
    pub log_id: String,
    pub patient_hash: ActionHash,
    pub attempted_accessor: AgentPubKey,
    pub data_category: DataCategory,
    pub denial_reason: String,
    pub attempted_at: Timestamp,
}
```

#### Add to Consent Coordinator

```rust
// zomes/consent/coordinator/src/lib.rs - ADD

/// Entry type for denied access logs
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AccessDeniedLog {
    pub log_id: String,
    pub patient_hash: ActionHash,
    pub attempted_accessor: AgentPubKey,
    pub data_category: String,
    pub denial_reason: String,
    pub attempted_at: Timestamp,
}

#[hdk_extern]
pub fn log_access_denied(log: AccessDeniedLog) -> ExternResult<ActionHash> {
    let hash = create_entry(&EntryTypes::AccessDeniedLog(log.clone()))?;

    // Link to patient for audit trail
    create_link(
        log.patient_hash.clone(),
        hash.clone(),
        LinkTypes::PatientToAccessDenied,
        ()
    )?;

    // Link to accessor for investigation
    let accessor_anchor = anchor_hash(&format!("accessor_{}", log.attempted_accessor))?;
    create_link(
        accessor_anchor,
        hash.clone(),
        LinkTypes::AccessorToDeniedLogs,
        ()
    )?;

    Ok(hash)
}

/// Get all denied access attempts for a patient (for security review)
#[hdk_extern]
pub fn get_denied_access_logs(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    // Requires admin authorization
    require_admin_authorization()?;

    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToAccessDenied)?,
        GetStrategy::default()
    )?;

    let mut logs = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                logs.push(record);
            }
        }
    }
    Ok(logs)
}
```

#### Acceptance Criteria

- [ ] All PHI access automatically logged
- [ ] Failed access attempts logged
- [ ] Logs include timestamp, accessor, category
- [ ] Emergency overrides clearly marked
- [ ] Logs are immutable (append-only DHT)

---

### 1.3 Remove/Restrict Dangerous Functions

**Objective**: Eliminate functions that expose entire datasets.

#### Option A: Remove Completely

```rust
// REMOVE these functions entirely:
// - get_all_patients()
// - get_all_providers()
// - get_all_pharmacies()
```

#### Option B: Restrict to Admin Role (Recommended)

```rust
// zomes/shared/src/roles.rs

/// Role definitions for access control
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Role {
    Patient,
    Provider,
    Admin,
    Researcher,
    Auditor,
}

/// Check if the calling agent has admin role
pub fn require_admin_authorization() -> ExternResult<()> {
    let caller = agent_info()?.agent_initial_pubkey;

    // Check if caller is in admin list
    let admin_anchor = anchor_hash("system_admins")?;
    let admin_links = get_links(
        LinkQuery::try_new(admin_anchor, LinkTypes::AdminAgents)?,
        GetStrategy::default()
    )?;

    let is_admin = admin_links.iter().any(|link| {
        link.target.clone().into_agent_pub_key() == Some(caller.clone())
    });

    if !is_admin {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Admin authorization required".to_string()
        )));
    }

    Ok(())
}

// Refactored function with admin check
#[hdk_extern]
pub fn get_all_patients_admin() -> ExternResult<Vec<Record>> {
    // CRITICAL: Admin only
    require_admin_authorization()?;

    // Log admin access
    log_admin_action("get_all_patients", "Bulk patient data export")?;

    // Existing implementation
    let anchor = anchor_hash("all_patients")?;
    let links = get_links(
        LinkQuery::try_new(anchor, LinkTypes::AllPatients)?,
        GetStrategy::default()
    )?;

    let mut patients = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                patients.push(record);
            }
        }
    }
    Ok(patients)
}
```

#### Acceptance Criteria

- [ ] `get_all_patients()` requires admin role
- [ ] `get_all_providers()` requires admin role
- [ ] Admin access is logged
- [ ] Non-admin calls return clear error
- [ ] Admin list is configurable

---

## Phase 2: Data Protection (Weeks 5-8)

### 2.1 Field-Level Encryption

**Objective**: Encrypt sensitive PHI fields at rest.

#### Encryption Schema

| Entry Type | Field | Encryption |
|------------|-------|------------|
| Patient | first_name | AES-256-GCM |
| Patient | last_name | AES-256-GCM |
| Patient | date_of_birth | AES-256-GCM |
| Patient | contact.address_* | AES-256-GCM |
| Patient | contact.phone_* | AES-256-GCM |
| Patient | contact.email | AES-256-GCM |
| EmergencyContact | name | AES-256-GCM |
| EmergencyContact | phone | AES-256-GCM |
| Provider | npi | AES-256-GCM |

#### Implementation: Encryption Utilities

```rust
// zomes/shared/src/encryption.rs
use hdk::prelude::*;
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, NewAead};

/// Encrypted field wrapper
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EncryptedField {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub key_id: String,  // Reference to key in patient's key store
}

/// Patient encryption key (stored separately, patient-controlled)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PatientEncryptionKey {
    pub key_id: String,
    pub patient_hash: ActionHash,
    pub encrypted_key: Vec<u8>,  // Key encrypted with patient's agent key
    pub created_at: Timestamp,
    pub rotated_from: Option<String>,
}

/// Encrypt a string field
pub fn encrypt_field(
    plaintext: &str,
    key: &[u8; 32],
) -> ExternResult<EncryptedField> {
    let cipher = Aes256Gcm::new(Key::from_slice(key));
    let nonce_bytes: [u8; 12] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(
            format!("Encryption failed: {:?}", e)
        )))?;

    Ok(EncryptedField {
        ciphertext,
        nonce: nonce_bytes.to_vec(),
        key_id: "current".to_string(),
    })
}

/// Decrypt a field
pub fn decrypt_field(
    encrypted: &EncryptedField,
    key: &[u8; 32],
) -> ExternResult<String> {
    let cipher = Aes256Gcm::new(Key::from_slice(key));
    let nonce = Nonce::from_slice(&encrypted.nonce);

    let plaintext = cipher.decrypt(nonce, encrypted.ciphertext.as_ref())
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(
            format!("Decryption failed: {:?}", e)
        )))?;

    String::from_utf8(plaintext)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(
            format!("Invalid UTF-8: {:?}", e)
        )))
}
```

#### Modified Patient Entry Type

```rust
// zomes/patient/integrity/src/lib.rs - MODIFIED

/// Patient profile with encrypted sensitive fields
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Patient {
    pub patient_id: String,
    pub mrn: Option<String>,

    // Encrypted fields
    pub first_name: EncryptedField,  // Was: String
    pub last_name: EncryptedField,   // Was: String
    pub date_of_birth: EncryptedField, // Was: String

    // Non-sensitive fields remain plaintext
    pub biological_sex: BiologicalSex,
    pub gender_identity: Option<String>,
    pub blood_type: Option<BloodType>,

    // Encrypted contact info
    pub contact: EncryptedContactInfo,
    pub emergency_contact: Option<EncryptedEmergencyContact>,

    // Rest remains same
    pub primary_language: String,
    pub allergies: Vec<Allergy>,
    pub conditions: Vec<String>,
    pub medications: Vec<String>,
    pub mycelix_identity_hash: Option<ActionHash>,
    pub matl_trust_score: f64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,

    // NEW: Encryption key reference
    pub encryption_key_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EncryptedContactInfo {
    pub address_line1: Option<EncryptedField>,
    pub address_line2: Option<EncryptedField>,
    pub city: Option<EncryptedField>,
    pub state_province: Option<EncryptedField>,
    pub postal_code: Option<EncryptedField>,
    pub country: String,  // Non-sensitive
    pub phone_primary: Option<EncryptedField>,
    pub phone_secondary: Option<EncryptedField>,
    pub email: Option<EncryptedField>,
}
```

#### Key Management Functions

```rust
// zomes/patient/coordinator/src/lib.rs - ADD

/// Generate encryption key for a new patient
#[hdk_extern]
pub fn generate_patient_key(patient_hash: ActionHash) -> ExternResult<String> {
    let caller = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    // Generate random key
    let key: [u8; 32] = rand::random();
    let key_id = format!("KEY-{}-{}", now.as_micros(), patient_hash);

    // Encrypt the key with patient's agent public key
    // (In production, use proper asymmetric encryption)
    let encrypted_key = encrypt_with_agent_key(&key, &caller)?;

    let key_entry = PatientEncryptionKey {
        key_id: key_id.clone(),
        patient_hash,
        encrypted_key,
        created_at: Timestamp::from_micros(now.as_micros() as i64),
        rotated_from: None,
    };

    create_entry(&EntryTypes::PatientEncryptionKey(key_entry))?;

    Ok(key_id)
}

/// Rotate patient encryption key
#[hdk_extern]
pub fn rotate_patient_key(input: RotateKeyInput) -> ExternResult<String> {
    // Verify caller is patient or admin
    require_patient_or_admin(input.patient_hash.clone())?;

    // Get old key
    let old_key = get_patient_key(input.patient_hash.clone(), input.old_key_id)?;

    // Generate new key
    let new_key_id = generate_patient_key(input.patient_hash.clone())?;

    // Re-encrypt all patient data with new key
    // (This would require updating the patient entry)

    Ok(new_key_id)
}
```

#### Acceptance Criteria

- [ ] Patient names encrypted at rest
- [ ] Contact information encrypted
- [ ] DOB encrypted
- [ ] Keys are patient-controlled
- [ ] Key rotation works correctly
- [ ] Decryption only with valid consent

---

## Phase 3: Performance & Scalability (Weeks 9-12)

### 3.1 Sharded Anchors

**Objective**: Replace single global anchors with sharded anchors for O(1) scalability.

#### Implementation

```rust
// zomes/shared/src/indexing.rs

/// Get sharded anchor for patient by last name
pub fn patient_shard_anchor(last_name: &str) -> ExternResult<EntryHash> {
    let shard = last_name
        .chars()
        .next()
        .unwrap_or('_')
        .to_uppercase()
        .next()
        .unwrap_or('_');

    anchor_hash(&format!("patients_shard_{}", shard))
}

/// Get all shard anchors (for admin bulk operations)
pub fn all_patient_shard_anchors() -> Vec<String> {
    let mut shards = Vec::new();
    for c in 'A'..='Z' {
        shards.push(format!("patients_shard_{}", c));
    }
    shards.push("patients_shard__".to_string()); // For non-alpha
    shards
}
```

#### Modified Create Function

```rust
#[hdk_extern]
pub fn create_patient(patient: Patient) -> ExternResult<Record> {
    let hash = create_entry(&EntryTypes::Patient(patient.clone()))?;
    let record = get(hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Failed to get created patient".to_string())))?;

    // Use sharded anchor instead of global
    let shard_anchor = patient_shard_anchor(&decrypt_field(&patient.last_name, &get_key()?)?)?;
    create_link(
        shard_anchor,
        hash.clone(),
        LinkTypes::ShardToPatient,
        ()
    )?;

    // Also index by MRN if present
    if let Some(mrn) = &patient.mrn {
        let mrn_anchor = anchor_hash(&format!("mrn_{}", mrn))?;
        create_link(mrn_anchor, hash.clone(), LinkTypes::MrnToPatient, ())?;
    }

    Ok(record)
}
```

### 3.2 Indexed Lookups

**Objective**: O(1) lookups for MRN and NPI.

```rust
/// Get patient by MRN - O(1) lookup
#[hdk_extern]
pub fn get_patient_by_mrn(mrn: String) -> ExternResult<Option<Record>> {
    let mrn_anchor = anchor_hash(&format!("mrn_{}", mrn))?;

    let links = get_links(
        LinkQuery::try_new(mrn_anchor, LinkTypes::MrnToPatient)?,
        GetStrategy::default()
    )?;

    if let Some(link) = links.first() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            // Check authorization before returning
            require_authorization(
                hash.clone(),
                DataCategory::Demographics,
                Permission::Read,
                false
            )?;
            return get(hash, GetOptions::default());
        }
    }

    Ok(None)
}
```

### 3.3 Pagination

**Objective**: Limit query result sizes to prevent memory issues.

```rust
// zomes/shared/src/pagination.rs

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaginationInput {
    pub offset: usize,
    pub limit: usize,  // Max 100
}

impl PaginationInput {
    pub fn validate(&self) -> ExternResult<()> {
        if self.limit > 100 {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Limit cannot exceed 100".to_string()
            )));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PaginatedResult<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}

impl<T> PaginatedResult<T> {
    pub fn new(items: Vec<T>, total: usize, input: &PaginationInput) -> Self {
        Self {
            has_more: input.offset + items.len() < total,
            items,
            total,
            offset: input.offset,
            limit: input.limit,
        }
    }
}
```

#### Paginated Query Example

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetPatientEncountersInput {
    pub patient_hash: ActionHash,
    pub pagination: PaginationInput,
}

#[hdk_extern]
pub fn get_patient_encounters_paginated(
    input: GetPatientEncountersInput
) -> ExternResult<PaginatedResult<Record>> {
    input.pagination.validate()?;

    // Check authorization
    require_authorization(
        input.patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        false
    )?;

    let links = get_links(
        LinkQuery::try_new(input.patient_hash, LinkTypes::PatientToEncounters)?,
        GetStrategy::default()
    )?;

    let total = links.len();

    // Apply pagination
    let paginated_links: Vec<_> = links
        .into_iter()
        .skip(input.pagination.offset)
        .take(input.pagination.limit)
        .collect();

    let mut encounters = Vec::new();
    for link in paginated_links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                encounters.push(record);
            }
        }
    }

    Ok(PaginatedResult::new(encounters, total, &input.pagination))
}
```

#### Acceptance Criteria

- [ ] All list endpoints support pagination
- [ ] Maximum limit of 100 items enforced
- [ ] Total count returned for UI
- [ ] has_more flag for infinite scroll
- [ ] Sharded anchors for patients
- [ ] Indexed lookups for MRN/NPI

---

## Phase 4: Testing & Documentation (Weeks 13-16)

### 4.1 Test Suite Expansion

#### Integration Tests with Conductor

```rust
// tests/integration/conductor_tests.rs
use holochain::test_utils::*;

#[tokio::test]
async fn test_patient_consent_flow() {
    // Setup conductor with health DNA
    let conductor = setup_conductor().await;
    let (alice, bob) = setup_agents(&conductor).await;

    // Alice creates patient record
    let patient = create_test_patient();
    let patient_hash = alice.call("patient", "create_patient", patient).await;

    // Bob tries to access without consent - should fail
    let result = bob.call("patient", "get_patient", GetPatientInput {
        patient_hash: patient_hash.clone(),
        is_emergency: false,
    }).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Access denied"));

    // Alice grants consent to Bob
    let consent = create_consent(patient_hash.clone(), bob.agent_pubkey());
    alice.call("consent", "create_consent", consent).await;

    // Bob can now access
    let result = bob.call("patient", "get_patient", GetPatientInput {
        patient_hash: patient_hash.clone(),
        is_emergency: false,
    }).await;
    assert!(result.is_ok());

    // Verify access was logged
    let logs = alice.call("consent", "get_access_logs", patient_hash).await;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].accessor, bob.agent_pubkey());
}
```

#### Negative Tests

```rust
// tests/unit/validation_tests.rs

#[test]
fn test_patient_creation_fails_without_name() {
    let patient = Patient {
        patient_id: "P001".to_string(),
        first_name: encrypt_field("", &test_key()).unwrap(),  // Empty
        last_name: encrypt_field("Doe", &test_key()).unwrap(),
        // ...
    };
    let result = validate_patient(&patient);
    assert!(matches!(result, Ok(ValidateCallbackResult::Invalid(_))));
}

#[test]
fn test_prescription_requires_dea_for_controlled() {
    let prescription = Prescription {
        schedule: Schedule::ScheduleII,
        dea_number: None,  // Missing!
        // ...
    };
    let result = validate_prescription(&prescription);
    assert!(matches!(result, Ok(ValidateCallbackResult::Invalid(_))));
}

#[test]
fn test_matl_score_rejects_out_of_range() {
    let patient = Patient {
        matl_trust_score: 1.5,  // Invalid: > 1.0
        // ...
    };
    let result = validate_patient(&patient);
    assert!(matches!(result, Ok(ValidateCallbackResult::Invalid(_))));
}
```

#### Performance Benchmarks

```rust
// tests/benchmarks/performance.rs
use std::time::Instant;

#[test]
fn benchmark_patient_lookup_by_mrn() {
    let conductor = setup_test_conductor();

    // Create 10,000 patients
    for i in 0..10_000 {
        let patient = create_patient_with_mrn(&format!("MRN{:05}", i));
        conductor.call("patient", "create_patient", patient);
    }

    // Benchmark lookup
    let start = Instant::now();
    for _ in 0..100 {
        conductor.call("patient", "get_patient_by_mrn", "MRN05000".to_string());
    }
    let elapsed = start.elapsed() / 100;

    println!("Average MRN lookup: {:?}", elapsed);
    assert!(elapsed < Duration::from_millis(50), "MRN lookup too slow: {:?}", elapsed);
}

#[test]
fn benchmark_paginated_query() {
    let conductor = setup_test_conductor();
    let patient_hash = create_patient_with_encounters(1000);

    let start = Instant::now();
    let result = conductor.call("records", "get_patient_encounters_paginated",
        GetPatientEncountersInput {
            patient_hash,
            pagination: PaginationInput { offset: 0, limit: 50 },
        }
    );
    let elapsed = start.elapsed();

    println!("Paginated query (50 of 1000): {:?}", elapsed);
    assert!(elapsed < Duration::from_millis(100));
}
```

### 4.2 Documentation

#### API Reference Template

```markdown
# Mycelix-Health API Reference

## Patient Zome

### create_patient

Creates a new patient record.

**Input:**
```json
{
  "patient_id": "string (required)",
  "mrn": "string (optional)",
  "first_name": "string (required, will be encrypted)",
  "last_name": "string (required, will be encrypted)",
  "date_of_birth": "string YYYY-MM-DD (required)",
  "biological_sex": "Male|Female|Intersex|Unknown",
  ...
}
```

**Output:**
```json
{
  "action_hash": "uhCkk...",
  "entry_hash": "uhCEk...",
  "entry": { ... }
}
```

**Errors:**
- `ValidationError`: If required fields missing
- `EncryptionError`: If encryption fails

**Authorization:** Requires patient agent or provider with consent
```

#### Acceptance Criteria

- [ ] 90%+ test coverage
- [ ] All negative cases tested
- [ ] Integration tests with conductor
- [ ] Performance benchmarks passing
- [ ] API reference complete
- [ ] Security guide complete
- [ ] Deployment guide complete

---

## Summary: Implementation Checklist

### Week 1-2: Access Control
- [ ] Create `mycelix-health-shared` crate
- [ ] Implement `require_authorization()`
- [ ] Add access control to patient functions
- [ ] Add access control to records functions
- [ ] Add access control to prescriptions functions

### Week 3-4: Audit Logging
- [ ] Implement automatic logging
- [ ] Add denied access logging
- [ ] Create audit report functions
- [ ] Write audit tests

### Week 5-6: Encryption
- [ ] Add AES-256-GCM encryption utilities
- [ ] Modify Patient entry type
- [ ] Modify ContactInfo type
- [ ] Implement key generation

### Week 7-8: Key Management
- [ ] Implement key storage
- [ ] Implement key rotation
- [ ] Add emergency key escrow
- [ ] Write encryption tests

### Week 9-10: Indexing
- [ ] Implement sharded anchors
- [ ] Add MRN index
- [ ] Add NPI index
- [ ] Migrate existing data patterns

### Week 11-12: Pagination
- [ ] Add pagination to all list endpoints
- [ ] Implement result limits
- [ ] Add has_more flag
- [ ] Performance testing

### Week 13-14: Testing
- [ ] Integration tests with conductor
- [ ] Negative test cases
- [ ] Performance benchmarks
- [ ] Security penetration tests

### Week 15-16: Documentation
- [ ] API reference
- [ ] Security guide
- [ ] Deployment guide
- [ ] HIPAA compliance statement

---

**Document Maintained By**: Development Team
**Last Updated**: January 21, 2026
**Review Schedule**: Weekly during implementation
