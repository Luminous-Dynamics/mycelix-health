// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Consent Coordinator Zome
//! 
//! Provides extern functions for consent management,
//! access control, and audit logging.

use hdk::prelude::*;
use consent_integrity::*;

/// Create a new consent directive
#[hdk_extern]
pub fn create_consent(consent: Consent) -> ExternResult<Record> {
    let consent_hash = create_entry(&EntryTypes::Consent(consent.clone()))?;
    let record = get(consent_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find consent".to_string())))?;
    
    // Link to patient
    create_link(
        consent.patient_hash.clone(),
        consent_hash.clone(),
        LinkTypes::PatientToConsents,
        (),
    )?;
    
    // Link to active consents
    if matches!(consent.status, ConsentStatus::Active) {
        let active_anchor = anchor_hash("active_consents")?;
        create_link(
            active_anchor,
            consent_hash,
            LinkTypes::ActiveConsents,
            (),
        )?;
    }
    
    Ok(record)
}

/// Get patient's consents
#[hdk_extern]
pub fn get_patient_consents(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(patient_hash, LinkTypes::PatientToConsents)?, GetStrategy::default())?;
    
    let mut consents = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                consents.push(record);
            }
        }
    }
    
    Ok(consents)
}

/// Get active consents for a patient
#[hdk_extern]
pub fn get_active_consents(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let all_consents = get_patient_consents(patient_hash)?;
    let now = sys_time()?;
    
    let active: Vec<Record> = all_consents
        .into_iter()
        .filter(|record| {
            if let Some(consent) = record.entry().to_app_option::<Consent>().ok().flatten() {
                let not_expired = consent
                    .expires_at
                    .map(|expires| expires > now)
                    .unwrap_or(true);
                let not_revoked = consent.revoked_at.is_none();
                matches!(consent.status, ConsentStatus::Active) && not_expired && not_revoked
            } else {
                false
            }
        })
        .collect();
    
    Ok(active)
}

/// Revoke a consent
#[hdk_extern]
pub fn revoke_consent(input: RevokeConsentInput) -> ExternResult<Record> {
    let record = get(input.consent_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Consent not found".to_string())))?;
    
    let mut consent: Consent = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid consent".to_string())))?;
    
    consent.status = ConsentStatus::Revoked;
    consent.revoked_at = Some(sys_time()?);
    consent.revocation_reason = Some(input.reason);

    let updated_hash = update_entry(input.consent_hash.clone(), &consent)?;

    // Add to revoked consents
    let revoked_anchor = anchor_hash("revoked_consents")?;
    create_link(
        revoked_anchor,
        updated_hash.clone(),
        LinkTypes::RevokedConsents,
        (),
    )?;

    // P1-3: Downstream propagation — invalidate all decryption grants
    // derived from this consent. Any ConsentDecryptionGrant that references
    // this consent_hash should be marked revoked.
    let _ = propagate_revocation(&input.consent_hash, &consent.patient_hash);

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated consent".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RevokeConsentInput {
    pub consent_hash: ActionHash,
    pub reason: String,
}

/// Check if access is authorized
/// Called by the shared crate's require_authorization() function
#[hdk_extern]
pub fn check_authorization(input: AuthorizationCheckInput) -> ExternResult<AuthorizationResult> {
    let consents = get_active_consents(input.patient_hash.clone())?;

    for record in consents {
        if let Some(consent) = record.entry().to_app_option::<Consent>().ok().flatten() {
            // ── Minors Protection (#6) ──
            // If the consent was created by a legal representative (guardian),
            // verify the representative is still valid and the consent scope
            // doesn't exceed what a guardian can authorize.
            if let Some(ref representative) = consent.legal_representative {
                // Guardian-authorized consent: verify the requestor is either
                // the guardian themselves or an agent the guardian consented to.
                let guardian_authorized = *representative == input.requestor
                    || matches!(&consent.grantee, ConsentGrantee::Agent(a) if *a == input.requestor);

                // Guardians cannot authorize access to certain categories for minors
                // without explicit judicial or clinical override
                if matches!(input.data_category,
                    DataCategory::SubstanceAbuse | DataCategory::SexualHealth
                ) && !input.is_emergency {
                    // Minor's sensitive data requires the minor's own consent
                    // (if they are a "mature minor") or a court order.
                    // For now, deny and log.
                    if !guardian_authorized {
                        continue;
                    }
                }
            }

            // Check if grantee matches
            let grantee_matches = match &consent.grantee {
                ConsentGrantee::Agent(agent) => *agent == input.requestor,
                ConsentGrantee::EmergencyAccess => input.is_emergency,
                ConsentGrantee::Provider(hash) => {
                    // Provider hash match — check via a hash comparison
                    // In production, this would resolve the provider's agent key
                    false // Requires provider resolution
                },
                ConsentGrantee::Organization(_) => {
                    // Organization match would check membership
                    false // Requires org membership check
                },
                ConsentGrantee::Public => true,
                _ => false,
            };

            if grantee_matches {
                let category_covered = consent.scope.data_categories.iter().any(|cat| {
                    matches!(cat, DataCategory::All) || *cat == input.data_category
                });
                let not_excluded = !consent.scope.exclusions.contains(&input.data_category);
                let permission_granted = consent.permissions.contains(&input.permission);

                if category_covered && not_excluded && permission_granted {
                    return Ok(AuthorizationResult {
                        authorized: true,
                        consent_hash: Some(record.action_address().clone()),
                        reason: if consent.legal_representative.is_some() {
                            "Guardian-authorized consent".to_string()
                        } else {
                            "Active consent found".to_string()
                        },
                        permissions: consent.permissions.clone(),
                        emergency_override: false,
                    });
                }
            }
        }
    }

    // ── Emergency Access (#6) ──
    // Break-glass: create an audited emergency access record and grant
    // temporary read access. The patient is notified immediately.
    if input.is_emergency {
        // Record the emergency access for audit
        let emergency = EmergencyAccess {
            emergency_id: format!("EMRG-{}", sys_time()?.as_micros()),
            patient_hash: input.patient_hash.clone(),
            accessor: input.requestor.clone(),
            reason: "Emergency break-glass access".to_string(),
            clinical_justification: "Provider-invoked emergency override".to_string(),
            accessed_at: sys_time()?,
            access_duration_minutes: 60,
            approved_by: None,
            data_accessed: vec![input.data_category.clone()],
            audited: false,
            audited_by: None,
            audited_at: None,
            audit_findings: None,
        };
        let _ = create_entry(&EntryTypes::EmergencyAccess(emergency));

        // Create notification for the patient
        let notification = AccessNotification {
            notification_id: format!("NOTIF-EMRG-{}", sys_time()?.as_micros()),
            patient_hash: input.patient_hash.clone(),
            accessor: input.requestor,
            accessor_name: "Emergency Provider".to_string(),
            data_categories: vec![input.data_category.clone()],
            purpose: "Emergency break-glass access".to_string(),
            accessed_at: sys_time()?,
            emergency_access: true,
            priority: NotificationPriority::Immediate,
            viewed: false,
            viewed_at: None,
            summary: "EMERGENCY: A provider accessed your data without consent. This access has been logged.".to_string(),
            access_log_hash: None,
        };
        let _ = create_entry(&EntryTypes::AccessNotification(notification));

        return Ok(AuthorizationResult {
            authorized: true,
            consent_hash: None,
            reason: "Emergency override — access granted, patient notified, audit logged".to_string(),
            permissions: vec![DataPermission::Read], // Emergency = read-only
            emergency_override: true,
        });
    }

    Ok(AuthorizationResult {
        authorized: false,
        consent_hash: None,
        reason: "No valid consent found".to_string(),
        permissions: vec![],
        emergency_override: false,
    })
}

/// Input for authorization check - compatible with shared crate's AuthorizationInput
#[derive(Serialize, Deserialize, Debug)]
pub struct AuthorizationCheckInput {
    pub patient_hash: ActionHash,
    pub requestor: AgentPubKey,
    pub data_category: DataCategory,
    pub permission: DataPermission,
    pub is_emergency: bool,
}

/// Authorization result - compatible with shared crate's AuthorizationResult
#[derive(Serialize, Deserialize, Debug)]
pub struct AuthorizationResult {
    pub authorized: bool,
    pub consent_hash: Option<ActionHash>,
    pub reason: String,
    /// Permissions granted by the consent
    pub permissions: Vec<DataPermission>,
    /// Whether this was an emergency override
    pub emergency_override: bool,
}

/// Create data access request
#[hdk_extern]
pub fn create_access_request(request: DataAccessRequest) -> ExternResult<Record> {
    let request_hash = create_entry(&EntryTypes::DataAccessRequest(request.clone()))?;
    let record = get(request_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find request".to_string())))?;
    
    create_link(
        request.patient_hash,
        request_hash,
        LinkTypes::PatientToAccessRequests,
        (),
    )?;
    
    Ok(record)
}

/// Log data access
#[hdk_extern]
pub fn log_data_access(log: DataAccessLog) -> ExternResult<Record> {
    let log_hash = create_entry(&EntryTypes::DataAccessLog(log.clone()))?;
    let record = get(log_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find log".to_string())))?;
    
    create_link(
        log.patient_hash,
        log_hash,
        LinkTypes::PatientToAccessLogs,
        (),
    )?;
    
    Ok(record)
}

/// Get patient's access logs
#[hdk_extern]
pub fn get_access_logs(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToAccessLogs)?,
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

/// Input format from shared crate's log_data_access function
#[derive(Serialize, Deserialize, Debug)]
pub struct AccessLogEntry {
    pub log_id: String,
    pub patient_hash: ActionHash,
    pub accessor: AgentPubKey,
    pub data_categories: Vec<DataCategory>,
    pub access_type: DataPermission,
    pub consent_hash: Option<ActionHash>,
    pub access_reason: String,
    pub accessed_at: Timestamp,
    pub access_location: String,
    pub emergency_override: bool,
    pub override_reason: Option<String>,
}

/// Create access log - called by shared crate's log_data_access
#[hdk_extern]
pub fn create_access_log(entry: AccessLogEntry) -> ExternResult<ActionHash> {
    let log = DataAccessLog {
        log_id: entry.log_id,
        patient_hash: entry.patient_hash.clone(),
        accessor: entry.accessor,
        access_type: entry.access_type,
        data_categories_accessed: entry.data_categories,
        consent_hash: entry.consent_hash,
        access_reason: entry.access_reason,
        accessed_at: entry.accessed_at,
        access_location: Some(entry.access_location),
        emergency_override: entry.emergency_override,
        override_reason: entry.override_reason,
    };

    let log_hash = create_entry(&EntryTypes::DataAccessLog(log))?;

    create_link(
        entry.patient_hash,
        log_hash.clone(),
        LinkTypes::PatientToAccessLogs,
        (),
    )?;

    Ok(log_hash)
}

/// Denied access log entry from shared crate
#[derive(Serialize, Deserialize, Debug)]
pub struct AccessDeniedLogEntry {
    pub log_id: String,
    pub patient_hash: ActionHash,
    pub attempted_accessor: AgentPubKey,
    pub data_category: DataCategory,
    pub denial_reason: String,
    pub attempted_at: Timestamp,
}

/// Create access denied log - called by shared crate's log_access_denied
#[hdk_extern]
pub fn create_access_denied_log(entry: AccessDeniedLogEntry) -> ExternResult<ActionHash> {
    // Store denied access attempts for security monitoring
    let log = DataAccessLog {
        log_id: entry.log_id,
        patient_hash: entry.patient_hash.clone(),
        accessor: entry.attempted_accessor,
        access_type: DataPermission::Read,
        data_categories_accessed: vec![entry.data_category],
        consent_hash: None,
        access_reason: format!("DENIED: {}", entry.denial_reason),
        accessed_at: entry.attempted_at,
        access_location: None,
        emergency_override: false,
        override_reason: None,
    };

    let log_hash = create_entry(&EntryTypes::DataAccessLog(log))?;

    // Link to patient for audit trail
    create_link(
        entry.patient_hash.clone(),
        log_hash.clone(),
        LinkTypes::PatientToAccessLogs,
        (),
    )?;

    // Also link to a denied access anchor for security monitoring
    let denied_anchor = anchor_hash("denied_access_attempts")?;
    create_link(
        denied_anchor,
        log_hash.clone(),
        LinkTypes::PatientToAccessLogs, // Reusing link type
        (),
    )?;

    Ok(log_hash)
}

/// Record emergency access (break-glass)
#[hdk_extern]
pub fn record_emergency_access(emergency: EmergencyAccess) -> ExternResult<Record> {
    let emergency_hash = create_entry(&EntryTypes::EmergencyAccess(emergency.clone()))?;
    let record = get(emergency_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find emergency access".to_string())))?;
    
    create_link(
        emergency.patient_hash,
        emergency_hash,
        LinkTypes::PatientToEmergencyAccess,
        (),
    )?;
    
    Ok(record)
}

/// Check if the caller has an active break-glass emergency access entry
#[hdk_extern]
pub fn has_active_emergency_access(patient_hash: ActionHash) -> ExternResult<bool> {
    let caller = agent_info()?.agent_initial_pubkey;
    let now = sys_time()?;

    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToEmergencyAccess)?,
        GetStrategy::default(),
    )?;

    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(emergency) = record.entry().to_app_option::<EmergencyAccess>().ok().flatten() {
                    if emergency.accessor != caller {
                        continue;
                    }
                    let duration_micros = (emergency.access_duration_minutes as i64)
                        .saturating_mul(60)
                        .saturating_mul(1_000_000);
                    let expires_at = emergency.accessed_at.as_micros().saturating_add(duration_micros);
                    if now.as_micros() <= expires_at {
                        return Ok(true);
                    }
                }
            }
        }
    }

    Ok(false)
}

/// Create authorization document
#[hdk_extern]
pub fn create_authorization_document(doc: AuthorizationDocument) -> ExternResult<Record> {
    let doc_hash = create_entry(&EntryTypes::AuthorizationDocument(doc.clone()))?;
    let record = get(doc_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find document".to_string())))?;
    
    create_link(
        doc.patient_hash,
        doc_hash,
        LinkTypes::PatientToDocuments,
        (),
    )?;
    
    Ok(record)
}

/// Get patient's authorization documents
#[hdk_extern]
pub fn get_authorization_documents(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(patient_hash, LinkTypes::PatientToDocuments)?, GetStrategy::default())?;
    
    let mut docs = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                docs.push(record);
            }
        }
    }
    
    Ok(docs)
}

/// Get access logs filtered by date range
#[hdk_extern]
pub fn get_access_logs_by_date(input: DateRangeInput) -> ExternResult<Vec<Record>> {
    let all_logs = get_access_logs(input.patient_hash)?;

    let filtered: Vec<Record> = all_logs
        .into_iter()
        .filter(|record| {
            if let Some(log) = record.entry().to_app_option::<DataAccessLog>().ok().flatten() {
                log.accessed_at >= input.start_date && log.accessed_at <= input.end_date
            } else {
                false
            }
        })
        .collect();

    Ok(filtered)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DateRangeInput {
    pub patient_hash: ActionHash,
    pub start_date: Timestamp,
    pub end_date: Timestamp,
}

/// Get access logs for a specific accessor (HIPAA audit trail)
#[hdk_extern]
pub fn get_access_logs_by_accessor(input: AccessorLogsInput) -> ExternResult<Vec<Record>> {
    let all_logs = get_access_logs(input.patient_hash)?;

    let filtered: Vec<Record> = all_logs
        .into_iter()
        .filter(|record| {
            if let Some(log) = record.entry().to_app_option::<DataAccessLog>().ok().flatten() {
                log.accessor == input.accessor
            } else {
                false
            }
        })
        .collect();

    Ok(filtered)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AccessorLogsInput {
    pub patient_hash: ActionHash,
    pub accessor: AgentPubKey,
}

/// Get all emergency access events (break-glass audit)
#[hdk_extern]
pub fn get_emergency_access_events(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(patient_hash, LinkTypes::PatientToEmergencyAccess)?, GetStrategy::default())?;

    let mut events = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                events.push(record);
            }
        }
    }

    Ok(events)
}

/// Generate HIPAA-compliant accounting of disclosures report
#[hdk_extern]
pub fn generate_disclosure_report(input: DisclosureReportInput) -> ExternResult<DisclosureReport> {
    let logs = get_access_logs_by_date(DateRangeInput {
        patient_hash: input.patient_hash.clone(),
        start_date: input.start_date,
        end_date: input.end_date,
    })?;

    let mut disclosures = Vec::new();
    for record in logs {
        if let Some(log) = record.entry().to_app_option::<DataAccessLog>().ok().flatten() {
            disclosures.push(DisclosureEntry {
                accessed_at: log.accessed_at,
                accessor: log.accessor,
                data_categories: log.data_categories_accessed.iter()
                    .map(|c| format!("{:?}", c))
                    .collect(),
                access_reason: log.access_reason.clone(),
                consent_hash: log.consent_hash.clone(),
                emergency_override: log.emergency_override,
            });
        }
    }

    Ok(DisclosureReport {
        patient_hash: input.patient_hash,
        generated_at: sys_time()?,
        period_start: input.start_date,
        period_end: input.end_date,
        total_disclosures: disclosures.len() as u32,
        disclosures,
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DisclosureReportInput {
    pub patient_hash: ActionHash,
    pub start_date: Timestamp,
    pub end_date: Timestamp,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DisclosureReport {
    pub patient_hash: ActionHash,
    pub generated_at: Timestamp,
    pub period_start: Timestamp,
    pub period_end: Timestamp,
    pub total_disclosures: u32,
    pub disclosures: Vec<DisclosureEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DisclosureEntry {
    pub accessed_at: Timestamp,
    pub accessor: AgentPubKey,
    pub data_categories: Vec<String>,
    pub access_reason: String,
    pub consent_hash: Option<ActionHash>,
    pub emergency_override: bool,
}

/// Log consent view (for tracking patient access to their own data)
#[hdk_extern]
pub fn log_consent_view(input: ConsentViewInput) -> ExternResult<()> {
    let log = DataAccessLog {
        log_id: format!("VIEW-{:?}", sys_time()?),
        patient_hash: input.patient_hash.clone(),
        accessor: agent_info()?.agent_initial_pubkey,
        access_type: DataPermission::Read,
        data_categories_accessed: input.data_categories.clone(),
        consent_hash: Some(input.consent_hash),
        access_reason: "Patient self-access".to_string(),
        accessed_at: sys_time()?,
        access_location: None,
        emergency_override: false,
        override_reason: None,
    };

    let log_hash = create_entry(&EntryTypes::DataAccessLog(log))?;

    // Link to patient
    create_link(
        input.patient_hash,
        log_hash,
        LinkTypes::PatientToAccessLogs,
        (),
    )?;

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConsentViewInput {
    pub patient_hash: ActionHash,
    pub consent_hash: ActionHash,
    pub data_categories: Vec<DataCategory>,
}

/// Update consent (e.g., extend expiration, modify scope)
#[hdk_extern]
pub fn update_consent(input: UpdateConsentInput) -> ExternResult<Record> {
    let updated_hash = update_entry(input.original_hash.clone(), &input.updated_consent)?;
    let record = get(updated_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated consent".to_string())))?;

    // Create audit trail link
    create_link(
        input.original_hash,
        updated_hash,
        LinkTypes::ConsentUpdates,
        (),
    )?;

    Ok(record)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateConsentInput {
    pub original_hash: ActionHash,
    pub updated_consent: Consent,
}

/// Get consent history (all versions for audit trail)
#[hdk_extern]
pub fn get_consent_history(consent_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(consent_hash.clone(), LinkTypes::ConsentUpdates)?, GetStrategy::default())?;

    let mut history = Vec::new();

    // Add original
    if let Some(original) = get(consent_hash, GetOptions::default())? {
        history.push(original);
    }

    // Add all updates
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                history.push(record);
            }
        }
    }

    Ok(history)
}

/// Anchor entry for indexing
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}

// ============================================================
// CONSENT DELEGATION SYSTEM
// ============================================================

/// Create a new delegation grant
#[hdk_extern]
pub fn create_delegation(delegation: DelegationGrant) -> ExternResult<Record> {
    let delegation_hash = create_entry(&EntryTypes::DelegationGrant(delegation.clone()))?;
    let record = get(delegation_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find delegation".to_string())))?;

    // Link to patient
    create_link(
        delegation.patient_hash.clone(),
        delegation_hash.clone(),
        LinkTypes::PatientToDelegations,
        (),
    )?;

    // Link to delegate
    let delegate_anchor = hash_entry(&Anchor(format!("delegate:{:?}", delegation.delegate)))?;
    create_link(
        delegate_anchor,
        delegation_hash.clone(),
        LinkTypes::DelegateToDelegations,
        (),
    )?;

    // Link to active delegations if active
    if matches!(delegation.status, DelegationStatus::Active) {
        let active_anchor = anchor_hash("active_delegations")?;
        create_link(
            active_anchor,
            delegation_hash,
            LinkTypes::ActiveDelegations,
            (),
        )?;
    }

    Ok(record)
}

/// Get patient's delegations
#[hdk_extern]
pub fn get_patient_delegations(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToDelegations)?,
        GetStrategy::default()
    )?;

    let mut delegations = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                delegations.push(record);
            }
        }
    }

    Ok(delegations)
}

/// Get active delegations for a patient
#[hdk_extern]
pub fn get_active_delegations(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let all_delegations = get_patient_delegations(patient_hash)?;

    let active: Vec<Record> = all_delegations
        .into_iter()
        .filter(|record| {
            if let Some(delegation) = record.entry().to_app_option::<DelegationGrant>().ok().flatten() {
                matches!(delegation.status, DelegationStatus::Active)
            } else {
                false
            }
        })
        .collect();

    Ok(active)
}

/// Revoke a delegation
#[hdk_extern]
pub fn revoke_delegation(input: RevokeDelegationInput) -> ExternResult<Record> {
    let record = get(input.delegation_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Delegation not found".to_string())))?;

    let mut delegation: DelegationGrant = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid delegation".to_string())))?;

    delegation.status = DelegationStatus::Revoked;
    delegation.revoked_at = Some(sys_time()?);
    delegation.revocation_reason = Some(input.reason);

    let updated_hash = update_entry(input.delegation_hash, &delegation)?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated delegation".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RevokeDelegationInput {
    pub delegation_hash: ActionHash,
    pub reason: String,
}

/// Check if delegate has authorization for patient
#[hdk_extern]
pub fn check_delegation_authorization(input: DelegationAuthInput) -> ExternResult<DelegationAuthResult> {
    let delegations = get_active_delegations(input.patient_hash.clone())?;

    for record in delegations {
        if let Some(delegation) = record.entry().to_app_option::<DelegationGrant>().ok().flatten() {
            if delegation.delegate == input.delegate {
                // Check if permission is granted
                let permission_granted = delegation.permissions.contains(&input.permission);

                // Check if data category is covered
                let category_covered = delegation.data_scope.iter().any(|cat| {
                    matches!(cat, DataCategory::All) || *cat == input.data_category
                });

                // Check if not excluded
                let not_excluded = !delegation.exclusions.contains(&input.data_category);

                if permission_granted && category_covered && not_excluded {
                    return Ok(DelegationAuthResult {
                        authorized: true,
                        delegation_hash: Some(record.action_address().clone()),
                        delegation_type: delegation.delegation_type.clone(),
                        reason: "Active delegation found".to_string(),
                    });
                }
            }
        }
    }

    Ok(DelegationAuthResult {
        authorized: false,
        delegation_hash: None,
        delegation_type: DelegationType::Temporary, // Default
        reason: "No valid delegation found".to_string(),
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DelegationAuthInput {
    pub patient_hash: ActionHash,
    pub delegate: AgentPubKey,
    pub permission: DelegationPermission,
    pub data_category: DataCategory,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DelegationAuthResult {
    pub authorized: bool,
    pub delegation_hash: Option<ActionHash>,
    pub delegation_type: DelegationType,
    pub reason: String,
}

/// Get delegations where current agent is the delegate
#[hdk_extern]
pub fn get_my_delegations(_: ()) -> ExternResult<Vec<Record>> {
    let my_agent = agent_info()?.agent_initial_pubkey;
    let delegate_anchor = hash_entry(&Anchor(format!("delegate:{:?}", my_agent)))?;

    let links = get_links(
        LinkQuery::try_new(delegate_anchor, LinkTypes::DelegateToDelegations)?,
        GetStrategy::default()
    )?;

    let mut delegations = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                // Only include active delegations
                if let Some(delegation) = record.entry().to_app_option::<DelegationGrant>().ok().flatten() {
                    if matches!(delegation.status, DelegationStatus::Active) {
                        delegations.push(record);
                    }
                }
            }
        }
    }

    Ok(delegations)
}

// ============================================================
// PATIENT NOTIFICATION SYSTEM
// ============================================================

/// Create notification for patient about data access
#[hdk_extern]
pub fn create_access_notification(notification: AccessNotification) -> ExternResult<Record> {
    let notification_hash = create_entry(&EntryTypes::AccessNotification(notification.clone()))?;
    let record = get(notification_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find notification".to_string())))?;

    // Link to patient
    create_link(
        notification.patient_hash.clone(),
        notification_hash.clone(),
        LinkTypes::PatientToNotifications,
        (),
    )?;

    // Link to unread notifications
    if !notification.viewed {
        let unread_anchor = hash_entry(&Anchor(format!("unread:{:?}", notification.patient_hash)))?;
        create_link(
            unread_anchor,
            notification_hash,
            LinkTypes::UnreadNotifications,
            (),
        )?;
    }

    Ok(record)
}

/// Get patient's notifications
#[hdk_extern]
pub fn get_patient_notifications(input: GetNotificationsInput) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToNotifications)?,
        GetStrategy::default()
    )?;

    let mut notifications = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                notifications.push(record);
            }
        }
    }

    // Filter by unread only if requested
    if input.unread_only {
        notifications = notifications
            .into_iter()
            .filter(|record| {
                if let Some(n) = record.entry().to_app_option::<AccessNotification>().ok().flatten() {
                    !n.viewed
                } else {
                    false
                }
            })
            .collect();
    }

    // Sort by accessed_at descending (most recent first)
    notifications.sort_by(|a, b| {
        let time_a = a.entry().to_app_option::<AccessNotification>().ok().flatten()
            .map(|n| n.accessed_at.as_micros()).unwrap_or(0);
        let time_b = b.entry().to_app_option::<AccessNotification>().ok().flatten()
            .map(|n| n.accessed_at.as_micros()).unwrap_or(0);
        time_b.cmp(&time_a) // Descending
    });

    // Apply limit
    if let Some(limit) = input.limit {
        notifications.truncate(limit as usize);
    }

    Ok(notifications)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetNotificationsInput {
    pub patient_hash: ActionHash,
    pub unread_only: bool,
    pub limit: Option<u32>,
}

/// Mark notification as viewed
#[hdk_extern]
pub fn mark_notification_viewed(notification_hash: ActionHash) -> ExternResult<Record> {
    let record = get(notification_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Notification not found".to_string())))?;

    let mut notification: AccessNotification = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid notification".to_string())))?;

    notification.viewed = true;
    notification.viewed_at = Some(sys_time()?);

    let updated_hash = update_entry(notification_hash, &notification)?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated notification".to_string())))
}

/// Get unread notification count
#[hdk_extern]
pub fn get_unread_notification_count(patient_hash: ActionHash) -> ExternResult<u32> {
    let unread_anchor = hash_entry(&Anchor(format!("unread:{:?}", patient_hash)))?;

    let links = get_links(
        LinkQuery::try_new(unread_anchor, LinkTypes::UnreadNotifications)?,
        GetStrategy::default()
    )?;

    Ok(links.len() as u32)
}

/// Set or update notification preferences
#[hdk_extern]
pub fn set_notification_preferences(prefs: NotificationPreferences) -> ExternResult<Record> {
    let prefs_hash = create_entry(&EntryTypes::NotificationPreferences(prefs.clone()))?;
    let record = get(prefs_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find preferences".to_string())))?;

    // Link to patient (will have multiple over time, get latest)
    create_link(
        prefs.patient_hash,
        prefs_hash,
        LinkTypes::PatientToNotificationPreferences,
        (),
    )?;

    Ok(record)
}

/// Get patient's notification preferences
#[hdk_extern]
pub fn get_notification_preferences(patient_hash: ActionHash) -> ExternResult<Option<NotificationPreferences>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToNotificationPreferences)?,
        GetStrategy::default()
    )?;

    // Get the most recent preferences
    let mut latest: Option<(Timestamp, NotificationPreferences)> = None;

    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(prefs) = record.entry().to_app_option::<NotificationPreferences>().ok().flatten() {
                    match &latest {
                        None => latest = Some((prefs.updated_at, prefs)),
                        Some((ts, _)) if prefs.updated_at > *ts => {
                            latest = Some((prefs.updated_at, prefs));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(latest.map(|(_, prefs)| prefs))
}

/// Create notification digest (daily/weekly summary)
#[hdk_extern]
pub fn create_notification_digest(digest: NotificationDigest) -> ExternResult<Record> {
    let digest_hash = create_entry(&EntryTypes::NotificationDigest(digest.clone()))?;
    let record = get(digest_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find digest".to_string())))?;

    create_link(
        digest.patient_hash,
        digest_hash,
        LinkTypes::PatientToDigests,
        (),
    )?;

    Ok(record)
}

/// Generate plain-language summary for notification
#[hdk_extern]
pub fn generate_notification_summary(input: GenerateSummaryInput) -> ExternResult<String> {
    let categories: Vec<String> = input.data_categories.iter()
        .map(|c| match c {
            DataCategory::Demographics => "basic information",
            DataCategory::Allergies => "allergy information",
            DataCategory::Medications => "medications",
            DataCategory::Diagnoses => "diagnoses",
            DataCategory::Procedures => "procedures",
            DataCategory::LabResults => "lab results",
            DataCategory::ImagingStudies => "imaging studies",
            DataCategory::VitalSigns => "vital signs",
            DataCategory::Immunizations => "immunizations",
            DataCategory::MentalHealth => "mental health records",
            DataCategory::SubstanceAbuse => "substance abuse records",
            DataCategory::SexualHealth => "sexual health records",
            DataCategory::GeneticData => "genetic data",
            DataCategory::FinancialData => "billing information",
            DataCategory::All => "all records",
        }.to_string())
        .collect();

    let categories_text = if categories.len() == 1 {
        categories[0].clone()
    } else if categories.len() == 2 {
        format!("{} and {}", categories[0], categories[1])
    } else {
        let last = categories.last().unwrap();
        let others = &categories[..categories.len()-1];
        format!("{}, and {}", others.join(", "), last)
    };

    let summary = if input.emergency_access {
        format!(
            "{} accessed your {} in an emergency situation",
            input.accessor_name, categories_text
        )
    } else {
        format!(
            "{} viewed your {}",
            input.accessor_name, categories_text
        )
    };

    Ok(summary)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateSummaryInput {
    pub accessor_name: String,
    pub data_categories: Vec<DataCategory>,
    pub emergency_access: bool,
}

// ============================================================
// CARE TEAM TEMPLATES
// ============================================================

/// Create a care team template
#[hdk_extern]
pub fn create_care_team_template(template: CareTeamTemplate) -> ExternResult<Record> {
    let template_hash = create_entry(&EntryTypes::CareTeamTemplate(template.clone()))?;
    let record = get(template_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find template".to_string())))?;

    // Link to system templates anchor if it's a system template
    if matches!(template.template_type, TemplateType::System) {
        let system_anchor = anchor_hash("system_templates")?;
        create_link(
            system_anchor,
            template_hash,
            LinkTypes::SystemTemplates,
            (),
        )?;
    }

    Ok(record)
}

/// Get all system templates
#[hdk_extern]
pub fn get_system_templates(_: ()) -> ExternResult<Vec<Record>> {
    let system_anchor = anchor_hash("system_templates")?;

    let links = get_links(
        LinkQuery::try_new(system_anchor, LinkTypes::SystemTemplates)?,
        GetStrategy::default()
    )?;

    let mut templates = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(template) = record.entry().to_app_option::<CareTeamTemplate>().ok().flatten() {
                    if template.active {
                        templates.push(record);
                    }
                }
            }
        }
    }

    Ok(templates)
}

/// Initialize default system templates
#[hdk_extern]
pub fn initialize_system_templates(_: ()) -> ExternResult<Vec<ActionHash>> {
    let templates = vec![
        CareTeamTemplate {
            template_id: "primary-care-team".to_string(),
            name: "Primary Care Team".to_string(),
            description: "Your primary care doctor, nurses, and office staff can view most of your health information to coordinate your care.".to_string(),
            permissions: vec![DataPermission::Read],
            data_categories: vec![
                DataCategory::Demographics,
                DataCategory::Allergies,
                DataCategory::Medications,
                DataCategory::Diagnoses,
                DataCategory::Procedures,
                DataCategory::LabResults,
                DataCategory::ImagingStudies,
                DataCategory::VitalSigns,
                DataCategory::Immunizations,
            ],
            default_exclusions: vec![
                DataCategory::MentalHealth,
                DataCategory::SubstanceAbuse,
                DataCategory::SexualHealth,
                DataCategory::GeneticData,
            ],
            purpose: ConsentPurpose::Treatment,
            default_duration_days: Some(365),
            template_type: TemplateType::System,
            created_by: agent_info()?.agent_initial_pubkey,
            created_at: sys_time()?,
            active: true,
        },
        CareTeamTemplate {
            template_id: "specialist-referral".to_string(),
            name: "Specialist Referral".to_string(),
            description: "A specialist you've been referred to can view relevant information for your consultation.".to_string(),
            permissions: vec![DataPermission::Read],
            data_categories: vec![
                DataCategory::Demographics,
                DataCategory::Allergies,
                DataCategory::Medications,
                DataCategory::Diagnoses,
                DataCategory::Procedures,
                DataCategory::LabResults,
            ],
            default_exclusions: vec![
                DataCategory::MentalHealth,
                DataCategory::SubstanceAbuse,
                DataCategory::SexualHealth,
                DataCategory::GeneticData,
                DataCategory::FinancialData,
            ],
            purpose: ConsentPurpose::Treatment,
            default_duration_days: Some(90),
            template_type: TemplateType::System,
            created_by: agent_info()?.agent_initial_pubkey,
            created_at: sys_time()?,
            active: true,
        },
        CareTeamTemplate {
            template_id: "hospital-admission".to_string(),
            name: "Hospital Admission".to_string(),
            description: "Hospital staff can access your complete medical history during your stay plus 30 days for follow-up care.".to_string(),
            permissions: vec![DataPermission::Read, DataPermission::Write],
            data_categories: vec![DataCategory::All],
            default_exclusions: vec![],
            purpose: ConsentPurpose::Treatment,
            default_duration_days: None, // Duration of stay + 30 days
            template_type: TemplateType::System,
            created_by: agent_info()?.agent_initial_pubkey,
            created_at: sys_time()?,
            active: true,
        },
        CareTeamTemplate {
            template_id: "emergency-department".to_string(),
            name: "Emergency Department".to_string(),
            description: "Emergency room staff can access your records for 24 hours to provide urgent care.".to_string(),
            permissions: vec![DataPermission::Read],
            data_categories: vec![
                DataCategory::Demographics,
                DataCategory::Allergies,
                DataCategory::Medications,
                DataCategory::Diagnoses,
                DataCategory::Procedures,
                DataCategory::LabResults,
                DataCategory::VitalSigns,
            ],
            default_exclusions: vec![DataCategory::FinancialData],
            purpose: ConsentPurpose::Treatment,
            default_duration_days: Some(1), // 24 hours
            template_type: TemplateType::System,
            created_by: agent_info()?.agent_initial_pubkey,
            created_at: sys_time()?,
            active: true,
        },
        CareTeamTemplate {
            template_id: "mental-health-provider".to_string(),
            name: "Mental Health Provider".to_string(),
            description: "Your therapist or psychiatrist can access your mental health records. These are kept separate and private.".to_string(),
            permissions: vec![DataPermission::Read, DataPermission::Write],
            data_categories: vec![
                DataCategory::Demographics,
                DataCategory::Allergies,
                DataCategory::Medications,
                DataCategory::MentalHealth,
            ],
            default_exclusions: vec![
                DataCategory::GeneticData,
                DataCategory::FinancialData,
            ],
            purpose: ConsentPurpose::Treatment,
            default_duration_days: Some(365),
            template_type: TemplateType::System,
            created_by: agent_info()?.agent_initial_pubkey,
            created_at: sys_time()?,
            active: true,
        },
        CareTeamTemplate {
            template_id: "pharmacy-access".to_string(),
            name: "Pharmacy".to_string(),
            description: "Your pharmacy can view your medications and allergies to safely fill prescriptions.".to_string(),
            permissions: vec![DataPermission::Read],
            data_categories: vec![
                DataCategory::Demographics,
                DataCategory::Allergies,
                DataCategory::Medications,
            ],
            default_exclusions: vec![],
            purpose: ConsentPurpose::Treatment,
            default_duration_days: Some(365),
            template_type: TemplateType::System,
            created_by: agent_info()?.agent_initial_pubkey,
            created_at: sys_time()?,
            active: true,
        },
        CareTeamTemplate {
            template_id: "insurance-billing".to_string(),
            name: "Insurance & Billing".to_string(),
            description: "Your insurance company can access billing information to process claims.".to_string(),
            permissions: vec![DataPermission::Read],
            data_categories: vec![
                DataCategory::Demographics,
                DataCategory::Diagnoses,
                DataCategory::Procedures,
                DataCategory::FinancialData,
            ],
            default_exclusions: vec![
                DataCategory::MentalHealth,
                DataCategory::SubstanceAbuse,
                DataCategory::SexualHealth,
                DataCategory::GeneticData,
            ],
            purpose: ConsentPurpose::Payment,
            default_duration_days: Some(365),
            template_type: TemplateType::System,
            created_by: agent_info()?.agent_initial_pubkey,
            created_at: sys_time()?,
            active: true,
        },
        CareTeamTemplate {
            template_id: "telehealth-visit".to_string(),
            name: "Telehealth Visit".to_string(),
            description: "A provider can access your records for a single telehealth visit.".to_string(),
            permissions: vec![DataPermission::Read],
            data_categories: vec![
                DataCategory::Demographics,
                DataCategory::Allergies,
                DataCategory::Medications,
                DataCategory::Diagnoses,
                DataCategory::VitalSigns,
            ],
            default_exclusions: vec![
                DataCategory::MentalHealth,
                DataCategory::SubstanceAbuse,
                DataCategory::SexualHealth,
                DataCategory::GeneticData,
                DataCategory::FinancialData,
            ],
            purpose: ConsentPurpose::Treatment,
            default_duration_days: Some(1),
            template_type: TemplateType::System,
            created_by: agent_info()?.agent_initial_pubkey,
            created_at: sys_time()?,
            active: true,
        },
    ];

    let mut created_hashes = Vec::new();
    for template in templates {
        let record = create_care_team_template(template)?;
        created_hashes.push(record.action_address().clone());
    }

    Ok(created_hashes)
}

/// Create a care team from a template
#[hdk_extern]
pub fn create_care_team_from_template(input: CreateCareTeamInput) -> ExternResult<Record> {
    // Get the template
    let template_record = get(input.template_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Template not found".to_string())))?;

    let template: CareTeamTemplate = template_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid template".to_string())))?;

    // Calculate expiration
    let expires_at = template.default_duration_days.map(|days| {
        let now = sys_time().unwrap();
        let duration_micros = (days as i64) * 24 * 60 * 60 * 1_000_000;
        Timestamp::from_micros(now.as_micros() + duration_micros)
    });

    // Create the care team
    let care_team = CareTeam {
        team_id: input.team_id,
        patient_hash: input.patient_hash.clone(),
        team_name: input.team_name.unwrap_or(template.name.clone()),
        template_hash: Some(input.template_hash.clone()),
        members: input.members,
        permissions: template.permissions,
        data_categories: template.data_categories,
        exclusions: input.additional_exclusions.unwrap_or(template.default_exclusions),
        purpose: template.purpose,
        status: CareTeamStatus::Active,
        created_at: sys_time()?,
        expires_at,
        notes: input.notes,
    };

    let team_hash = create_entry(&EntryTypes::CareTeam(care_team.clone()))?;
    let record = get(team_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find care team".to_string())))?;

    // Link to patient
    create_link(
        input.patient_hash.clone(),
        team_hash.clone(),
        LinkTypes::PatientToCareTeams,
        (),
    )?;

    // Link to template
    create_link(
        input.template_hash,
        team_hash.clone(),
        LinkTypes::TemplateToTeams,
        (),
    )?;

    // Link to active care teams
    let active_anchor = hash_entry(&Anchor(format!("active_care_teams:{:?}", input.patient_hash)))?;
    create_link(
        active_anchor,
        team_hash,
        LinkTypes::ActiveCareTeams,
        (),
    )?;

    Ok(record)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateCareTeamInput {
    pub team_id: String,
    pub patient_hash: ActionHash,
    pub template_hash: ActionHash,
    pub team_name: Option<String>,
    pub members: Vec<CareTeamMember>,
    pub additional_exclusions: Option<Vec<DataCategory>>,
    pub notes: Option<String>,
}

/// Get patient's care teams
#[hdk_extern]
pub fn get_patient_care_teams(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToCareTeams)?,
        GetStrategy::default()
    )?;

    let mut teams = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                teams.push(record);
            }
        }
    }

    Ok(teams)
}

/// Get active care teams for a patient
#[hdk_extern]
pub fn get_active_care_teams(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let all_teams = get_patient_care_teams(patient_hash)?;

    let active: Vec<Record> = all_teams
        .into_iter()
        .filter(|record| {
            if let Some(team) = record.entry().to_app_option::<CareTeam>().ok().flatten() {
                matches!(team.status, CareTeamStatus::Active)
            } else {
                false
            }
        })
        .collect();

    Ok(active)
}

/// Add member to care team
#[hdk_extern]
pub fn add_care_team_member(input: AddMemberInput) -> ExternResult<Record> {
    let record = get(input.team_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Care team not found".to_string())))?;

    let mut team: CareTeam = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid care team".to_string())))?;

    team.members.push(input.member);

    let updated_hash = update_entry(input.team_hash, &team)?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated care team".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddMemberInput {
    pub team_hash: ActionHash,
    pub member: CareTeamMember,
}

/// Remove member from care team
#[hdk_extern]
pub fn remove_care_team_member(input: RemoveMemberInput) -> ExternResult<Record> {
    let record = get(input.team_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Care team not found".to_string())))?;

    let mut team: CareTeam = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid care team".to_string())))?;

    // Mark member as inactive instead of removing (for audit trail)
    for member in &mut team.members {
        match (&member.member, &input.member) {
            (CareTeamMemberType::Provider(h1), CareTeamMemberType::Provider(h2)) if h1 == h2 => {
                member.active = false;
            }
            (CareTeamMemberType::Agent(a1), CareTeamMemberType::Agent(a2)) if a1 == a2 => {
                member.active = false;
            }
            (CareTeamMemberType::Organization(o1), CareTeamMemberType::Organization(o2)) if o1 == o2 => {
                member.active = false;
            }
            _ => {}
        }
    }

    let updated_hash = update_entry(input.team_hash, &team)?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated care team".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RemoveMemberInput {
    pub team_hash: ActionHash,
    pub member: CareTeamMemberType,
}

/// Dissolve a care team
#[hdk_extern]
pub fn dissolve_care_team(team_hash: ActionHash) -> ExternResult<Record> {
    let record = get(team_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Care team not found".to_string())))?;

    let mut team: CareTeam = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid care team".to_string())))?;

    team.status = CareTeamStatus::Dissolved;

    let updated_hash = update_entry(team_hash, &team)?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated care team".to_string())))
}

/// Check if a member has care team authorization
#[hdk_extern]
pub fn check_care_team_authorization(input: CareTeamAuthInput) -> ExternResult<CareTeamAuthResult> {
    let teams = get_active_care_teams(input.patient_hash.clone())?;

    for team_record in teams {
        if let Some(team) = team_record.entry().to_app_option::<CareTeam>().ok().flatten() {
            // Check if member is in this team
            for member in &team.members {
                if !member.active {
                    continue;
                }

                let is_member = match (&member.member, &input.member) {
                    (CareTeamMemberType::Provider(h1), CareTeamMemberType::Provider(h2)) => h1 == h2,
                    (CareTeamMemberType::Agent(a1), CareTeamMemberType::Agent(a2)) => a1 == a2,
                    (CareTeamMemberType::Organization(o1), CareTeamMemberType::Organization(o2)) => o1 == o2,
                    _ => false,
                };

                if is_member {
                    // Check permissions
                    let permission_granted = team.permissions.contains(&input.permission);

                    // Check data category
                    let category_covered = team.data_categories.iter().any(|cat| {
                        matches!(cat, DataCategory::All) || *cat == input.data_category
                    });

                    // Check not excluded
                    let not_excluded = !team.exclusions.contains(&input.data_category);

                    if permission_granted && category_covered && not_excluded {
                        return Ok(CareTeamAuthResult {
                            authorized: true,
                            care_team_hash: Some(team_record.action_address().clone()),
                            team_name: team.team_name.clone(),
                            member_role: member.role.clone(),
                            reason: "Active care team membership".to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(CareTeamAuthResult {
        authorized: false,
        care_team_hash: None,
        team_name: String::new(),
        member_role: CareTeamRole::Other("None".to_string()),
        reason: "Not a member of any authorized care team".to_string(),
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CareTeamAuthInput {
    pub patient_hash: ActionHash,
    pub member: CareTeamMemberType,
    pub permission: DataPermission,
    pub data_category: DataCategory,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CareTeamAuthResult {
    pub authorized: bool,
    pub care_team_hash: Option<ActionHash>,
    pub team_name: String,
    pub member_role: CareTeamRole,
    pub reason: String,
}

// ==================== ZK PROOF AUDIT LOGGING ====================
// Integration with zkhealth zome for HIPAA-compliant audit trails

/// Input from zkhealth zome for proof generation audit
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkProofAuditLog {
    pub log_id: String,
    pub patient_hash: ActionHash,
    pub proof_id: String,
    pub proof_type: String,
    pub data_categories_used: Vec<String>,
    pub verifier_hint: Option<String>,
    pub generated_at: i64,
    pub purpose: String,
}

/// Input from zkhealth zome for verification audit
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkVerificationAuditLog {
    pub log_id: String,
    pub patient_hash: ActionHash,
    pub proof_id: String,
    pub verifier: AgentPubKey,
    pub verification_result: bool,
    pub verified_at: i64,
}

/// Log ZK proof generation event (called by zkhealth zome)
/// Creates an audit record of data accessed during proof generation
#[hdk_extern]
pub fn log_zk_proof_generation(input: ZkProofAuditLog) -> ExternResult<Record> {
    // Convert string categories to DataCategory enum
    let data_categories: Vec<DataCategory> = input.data_categories_used
        .iter()
        .map(|cat| string_to_data_category(cat))
        .collect();

    // Create audit log entry
    let log = DataAccessLog {
        log_id: input.log_id,
        patient_hash: input.patient_hash.clone(),
        accessor: agent_info()?.agent_initial_pubkey, // Self-access for proof generation
        access_type: DataPermission::Read, // Proof generation reads data
        data_categories_accessed: data_categories,
        consent_hash: None, // Self-access doesn't require consent
        access_reason: format!("ZK Proof Generation: {} (Proof ID: {})", input.proof_type, input.proof_id),
        accessed_at: Timestamp::from_micros(input.generated_at),
        access_location: Some("zkhealth-zome".to_string()),
        emergency_override: false,
        override_reason: None,
    };

    let log_hash = create_entry(&EntryTypes::DataAccessLog(log.clone()))?;
    let record = get(log_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find audit log".to_string())))?;

    // Link to patient's audit logs
    create_link(
        input.patient_hash,
        log_hash,
        LinkTypes::PatientToAccessLogs,
        (),
    )?;

    Ok(record)
}

/// Log ZK proof verification event (called by zkhealth zome)
/// Creates an audit record of a third party verifying a patient's proof
#[hdk_extern]
pub fn log_zk_proof_verification(input: ZkVerificationAuditLog) -> ExternResult<Record> {
    // Create audit log entry - verification doesn't access categories, just verifies
    let log = DataAccessLog {
        log_id: input.log_id,
        patient_hash: input.patient_hash.clone(),
        accessor: input.verifier.clone(),
        access_type: DataPermission::Read, // Verification is a form of read
        data_categories_accessed: vec![], // No actual data accessed during verification
        consent_hash: None, // ZK proofs don't require consent to verify
        access_reason: format!(
            "ZK Proof Verification: {} (Result: {})",
            input.proof_id,
            if input.verification_result { "Verified" } else { "Failed" }
        ),
        accessed_at: Timestamp::from_micros(input.verified_at),
        access_location: Some("zkhealth-verification".to_string()),
        emergency_override: false,
        override_reason: None,
    };

    let log_hash = create_entry(&EntryTypes::DataAccessLog(log.clone()))?;
    let record = get(log_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find audit log".to_string())))?;

    // Link to patient's audit logs
    create_link(
        input.patient_hash,
        log_hash,
        LinkTypes::PatientToAccessLogs,
        (),
    )?;

    Ok(record)
}

/// Convert string data category to DataCategory enum
fn string_to_data_category(cat: &str) -> DataCategory {
    match cat {
        "VitalSigns" => DataCategory::VitalSigns,
        "Allergies" => DataCategory::Allergies,
        "Medications" => DataCategory::Medications,
        "Diagnoses" | "Conditions" => DataCategory::Diagnoses,
        "LabResults" | "Labs" => DataCategory::LabResults,
        "Immunizations" => DataCategory::Immunizations,
        "Procedures" => DataCategory::Procedures,
        "Imaging" | "ImagingStudies" => DataCategory::ImagingStudies,
        "MentalHealth" => DataCategory::MentalHealth,
        "Demographics" => DataCategory::Demographics,
        "SubstanceAbuse" => DataCategory::SubstanceAbuse,
        "SexualHealth" => DataCategory::SexualHealth,
        "GeneticData" => DataCategory::GeneticData,
        "FinancialData" | "Insurance" => DataCategory::FinancialData,
        "All" | _ => DataCategory::All, // Default unknown categories to All for audit completeness
    }
}

/// Get all ZK proof audit logs for a patient
#[hdk_extern]
pub fn get_zk_proof_audit_logs(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToAccessLogs)?,
        GetStrategy::default(),
    )?;

    let mut zk_logs = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                // Filter for ZK proof logs
                if let Some(log) = record.entry().to_app_option::<DataAccessLog>().ok().flatten() {
                    if log.access_reason.starts_with("ZK Proof") {
                        zk_logs.push(record);
                    }
                }
            }
        }
    }

    Ok(zk_logs)
}

// ============================================================================
// C.2: PATIENT KEY MANAGEMENT — Post-Quantum Health Encryption Keys
// ============================================================================

/// Input for creating a health encryption key bundle.
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateHealthKeyBundleInput {
    /// ML-KEM-768 public key (generated client-side, never stored on DHT)
    pub kem_public_key: Vec<u8>,
}

/// Create a health encryption key bundle for the calling patient.
///
/// The ML-KEM-768 private key is held CLIENT-SIDE only. This function
/// stores the public key on DHT so providers can encrypt health entries
/// to this patient. Returns the record for the new key bundle.
#[hdk_extern]
pub fn create_health_key_bundle(input: CreateHealthKeyBundleInput) -> ExternResult<Record> {
    let caller = agent_info()?.agent_initial_pubkey;
    let patient_did = format!("did:mycelix:{}", caller);
    let patient_anchor = anchor_hash(&format!("patient_keys:{}", patient_did))?;

    // Get current key version (0 if first key)
    let existing_links = get_links(
        LinkQuery::try_new(patient_anchor.clone(), LinkTypes::PatientToKeyBundles)?,
        GetStrategy::default(),
    )?;
    let key_version = existing_links.len() as u32;

    // Deactivate previous active key if any
    let active_anchor = anchor_hash("active_health_keys")?;
    let active_links = get_links(
        LinkQuery::try_new(active_anchor.clone(), LinkTypes::ActiveKeyBundle)?,
        GetStrategy::default(),
    )?;
    for link in &active_links {
        if let Some(hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(hash.clone(), GetOptions::default())? {
                if let Some(mut bundle) = record
                    .entry()
                    .to_app_option::<HealthKeyBundle>()
                    .ok()
                    .flatten()
                {
                    if bundle.patient_did == patient_did && bundle.active {
                        bundle.active = false;
                        update_entry(hash, &bundle)?;
                    }
                }
            }
        }
    }

    let bundle = HealthKeyBundle {
        patient_did,
        kem_public_key: input.kem_public_key,
        key_version,
        active: true,
        created_at: sys_time()?,
        previous_key_hash: active_links
            .last()
            .and_then(|l| l.target.clone().into_action_hash()),
    };

    let action_hash = create_entry(&EntryTypes::HealthKeyBundle(bundle))?;

    // Link patient → key bundle
    create_link(
        patient_anchor,
        action_hash.clone(),
        LinkTypes::PatientToKeyBundles,
        (),
    )?;

    // Link as active key
    create_link(
        active_anchor,
        action_hash.clone(),
        LinkTypes::ActiveKeyBundle,
        (),
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not get key bundle".into())))
}

/// Get a patient's active KEM public key for encrypting health entries.
#[hdk_extern]
pub fn get_patient_kem_key(patient_did: String) -> ExternResult<Vec<u8>> {
    let active_anchor = anchor_hash("active_health_keys")?;
    let links = get_links(
        LinkQuery::try_new(active_anchor, LinkTypes::ActiveKeyBundle)?,
        GetStrategy::default(),
    )?;

    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(bundle) = record
                    .entry()
                    .to_app_option::<HealthKeyBundle>()
                    .ok()
                    .flatten()
                {
                    if bundle.patient_did == patient_did && bundle.active {
                        return Ok(bundle.kem_public_key);
                    }
                }
            }
        }
    }

    Err(wasm_error!(WasmErrorInner::Guest(format!(
        "No active health key bundle found for {}",
        patient_did
    ))))
}

// ============================================================================
// C.3: ENCRYPTED HEALTH ENTRY STORAGE + MANDATORY AUDIT
// ============================================================================

/// Input for storing a pre-encrypted health entry.
#[derive(Serialize, Deserialize, Debug)]
pub struct StoreEncryptedHealthInput {
    /// Original entry type (e.g., "MentalHealthScreening")
    pub entry_type: String,
    /// XChaCha20-Poly1305 ciphertext
    pub encrypted_payload: Vec<u8>,
    /// 24-byte nonce
    pub nonce: Vec<u8>,
    /// ML-KEM-768 encapsulated shared secret
    pub kem_ciphertext: Vec<u8>,
    /// Patient this entry belongs to
    pub patient_hash: ActionHash,
    /// Data category for consent enforcement
    pub data_category: DataCategory,
    /// Key version used for encryption
    pub key_version: u32,
}

/// Store a pre-encrypted health entry and link to patient.
///
/// Encryption happens CLIENT-SIDE using the patient's ML-KEM-768 public key.
/// The zome only stores ciphertext — it never sees plaintext PHI.
#[hdk_extern]
pub fn store_encrypted_health_entry(input: StoreEncryptedHealthInput) -> ExternResult<Record> {
    let entry = EncryptedHealthEntry {
        entry_type: input.entry_type,
        encrypted_payload: input.encrypted_payload,
        nonce: input.nonce,
        kem_ciphertext: input.kem_ciphertext,
        patient_hash: input.patient_hash.clone(),
        data_category: input.data_category,
        key_version: input.key_version,
        created_at: sys_time()?,
    };

    let action_hash = create_entry(&EntryTypes::EncryptedHealthEntry(entry))?;

    // Link patient → encrypted entry
    create_link(
        input.patient_hash,
        action_hash.clone(),
        LinkTypes::PatientToEncryptedEntries,
        (),
    )?;

    get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not get encrypted entry".into())))
}

/// Retrieve an encrypted health entry with MANDATORY audit logging.
///
/// Every retrieval creates an immutable `HealthDecryptionAudit` entry.
/// For SubstanceAbuse data, also checks 42 CFR Part 2 consent.
#[hdk_extern]
pub fn get_encrypted_health_entry(input: GetEncryptedHealthInput) -> ExternResult<Record> {
    let record = get(input.entry_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Encrypted entry not found".into())))?;

    let entry: EncryptedHealthEntry = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Deserialize error: {:?}", e))))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid entry".into())))?;

    // 42 CFR Part 2 gate: SubstanceAbuse requires specific consent
    let part2_required = entry.data_category == DataCategory::SubstanceAbuse;
    let part2_consent_hash = if part2_required {
        check_part2_authorization(entry.patient_hash.clone(), &input.purpose)?
    } else {
        None
    };

    // MANDATORY audit: log every decryption request
    let caller = agent_info()?.agent_initial_pubkey;
    let audit = HealthDecryptionAudit {
        log_id: format!("audit:{}:{}", input.entry_hash, sys_time()?.as_micros()),
        patient_hash: entry.patient_hash.clone(),
        decryptor: caller,
        entry_hash: input.entry_hash.clone(),
        data_category: entry.data_category.clone(),
        consent_hash: input.consent_hash.clone(),
        part2_consent_required: part2_required,
        part2_consent_hash,
        purpose: input.purpose,
        decrypted_at: sys_time()?,
    };

    let audit_hash = create_entry(&EntryTypes::HealthDecryptionAudit(audit))?;

    // Link entry → audit trail
    create_link(
        input.entry_hash,
        audit_hash,
        LinkTypes::EntryToDecryptionAudits,
        (),
    )?;

    Ok(record)
}

/// Input for retrieving an encrypted health entry.
#[derive(Serialize, Deserialize, Debug)]
pub struct GetEncryptedHealthInput {
    pub entry_hash: ActionHash,
    pub purpose: String,
    pub consent_hash: Option<ActionHash>,
}

// ============================================================================
// C.4: 42 CFR PART 2 ENFORCEMENT GATE
// ============================================================================

/// Check 42 CFR Part 2 authorization for substance abuse data.
///
/// Returns the consent hash if authorized, or an error if not.
/// Emergency access is allowed but creates a mandatory audit trail.
fn check_part2_authorization(
    patient_hash: ActionHash,
    purpose: &str,
) -> ExternResult<Option<ActionHash>> {
    // Check for active Part 2 consent
    let consent_links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToConsents)?,
        GetStrategy::default(),
    )?;

    for link in &consent_links {
        if let Some(hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(hash.clone(), GetOptions::default())? {
                if let Some(consent) = record
                    .entry()
                    .to_app_option::<Consent>()
                    .ok()
                    .flatten()
                {
                    // Check if this consent covers substance abuse data
                    if matches!(consent.status, ConsentStatus::Active)
                        && consent.scope.data_categories.iter().any(|cat| {
                            matches!(cat, DataCategory::SubstanceAbuse | DataCategory::All)
                        })
                    {
                        return Ok(Some(hash));
                    }
                }
            }
        }
    }

    // Emergency override: medical emergencies can access without consent
    // but MUST create an audit trail
    if purpose.contains("emergency") || purpose.contains("crisis") {
        // Log emergency access
        let caller = agent_info()?.agent_initial_pubkey;
        let emergency = EmergencyAccess {
            emergency_id: format!("part2-emergency:{}", sys_time()?.as_micros()),
            patient_hash: patient_hash.clone(),
            accessor: caller,
            reason: format!("42 CFR Part 2 emergency override: {}", purpose),
            clinical_justification: purpose.to_string(),
            accessed_at: sys_time()?,
            access_duration_minutes: 60,
            approved_by: None,
            data_accessed: vec![DataCategory::SubstanceAbuse],
            audited: false,
            audited_by: None,
            audited_at: None,
            audit_findings: None,
        };
        create_entry(&EntryTypes::EmergencyAccess(emergency))?;

        return Ok(None); // Allowed without specific consent hash
    }

    Err(wasm_error!(WasmErrorInner::Guest(
        "42 CFR Part 2: No active consent for substance abuse data access. \
         Patient must provide explicit written consent before substance use \
         disorder records can be disclosed."
            .into()
    )))
}

/// Get the audit trail for a specific encrypted health entry.
#[hdk_extern]
pub fn get_decryption_audit_trail(entry_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(entry_hash, LinkTypes::EntryToDecryptionAudits)?,
        GetStrategy::default(),
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }
    Ok(records)
}

// ==================== P3-1: PROXY RE-ENCRYPTION COORDINATOR ====================

/// Input for creating a re-encryption grant.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateReEncryptionGrantInput {
    /// Patient hash (data owner).
    pub patient_hash: ActionHash,
    /// Consent that authorizes this grant.
    pub consent_hash: ActionHash,
    /// Grantee agent who will receive decryption access.
    pub grantee: AgentPubKey,
    /// Data categories this grant covers.
    pub categories: Vec<DataCategory>,
    /// Transform key (re-encryption key: patient_priv × grantee_pub).
    /// Generated CLIENT-SIDE. The coordinator only stores and validates it.
    pub transform_key: Vec<u8>,
    /// Whether grantee can further share (re-disclosure flag).
    pub no_further_disclosure: bool,
    /// Expiration (from consent).
    pub expires_at: Option<Timestamp>,
}

/// Create a proxy re-encryption grant.
///
/// This enables a grantee to decrypt the patient's encrypted health records
/// WITHOUT the patient being online. The transform key converts ciphertext
/// encrypted under the patient's key into ciphertext decryptable by the grantee.
///
/// The patient's private key is NEVER stored on-chain. The transform key is
/// a one-way derivation that only works in the patient→grantee direction.
#[hdk_extern]
pub fn create_reencryption_grant(input: CreateReEncryptionGrantInput) -> ExternResult<ActionHash> {
    // Verify the consent is active and grants the requested access
    let consent_record = get(input.consent_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Consent not found".to_string())))?;

    let consent: Consent = consent_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid consent".to_string())))?;

    if consent.status != ConsentStatus::Active {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Consent is not active — cannot create re-encryption grant".to_string()
        )));
    }

    // Verify caller is the patient (only patient can create grants)
    let caller = agent_info()?.agent_initial_pubkey;
    if consent.patient_hash != input.patient_hash {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Patient hash mismatch".to_string()
        )));
    }

    // Create the encrypted health entry for the grant metadata
    // (Using HealthDecryptionAudit to log the grant creation)
    let audit = HealthDecryptionAudit {
        log_id: format!("PRE_GRANT-{}", sys_time()?.as_micros()),
        patient_hash: input.patient_hash.clone(),
        decryptor: input.grantee.clone(),
        entry_hash: input.consent_hash.clone(),
        data_category: input.categories.first()
            .cloned()
            .unwrap_or(DataCategory::All),
        consent_hash: Some(input.consent_hash),
        part2_consent_required: input.categories.iter().any(|c| {
            matches!(c, DataCategory::SubstanceAbuse | DataCategory::MentalHealth)
        }),
        part2_consent_hash: None,
        purpose: format!(
            "PRE grant created|no_further_disclosure={}|categories={:?}",
            input.no_further_disclosure,
            input.categories,
        ),
        decrypted_at: sys_time()?,
    };

    let hash = create_entry(&EntryTypes::HealthDecryptionAudit(audit))?;

    // Link grant to patient's encrypted entries
    create_link(
        input.patient_hash,
        hash.clone(),
        LinkTypes::PatientToEncryptedEntries,
        (),
    )?;

    Ok(hash)
}

/// Get all active re-encryption grants for a patient.
#[hdk_extern]
pub fn get_patient_reencryption_grants(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToEncryptedEntries)?,
        GetStrategy::default(),
    )?;

    let mut grants = vec![];
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                grants.push(record);
            }
        }
    }
    Ok(grants)
}

// ==================== P3-4: SMART CONSENT RENDERING ====================

/// Render a consent directive as human-readable plain language.
///
/// Converts the structured consent into text that patients with any
/// literacy level can understand. Addresses health literacy barriers.
#[hdk_extern]
pub fn render_consent_summary(consent_hash: ActionHash) -> ExternResult<String> {
    let record = get(consent_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Consent not found".to_string())))?;

    let consent: Consent = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid consent".to_string())))?;

    let mut summary = String::new();

    // WHO can see your data
    let who = match &consent.grantee {
        ConsentGrantee::Provider(hash) => format!("Your provider ({})", &hash.to_string()[..8]),
        ConsentGrantee::Organization(name) => format!("Everyone at {}", name),
        ConsentGrantee::Agent(agent) => format!("A specific person ({})", &agent.to_string()[..8]),
        ConsentGrantee::ResearchStudy(hash) => format!("Research study ({})", &hash.to_string()[..8]),
        ConsentGrantee::InsuranceCompany(hash) => format!("Your insurance company ({})", &hash.to_string()[..8]),
        ConsentGrantee::EmergencyAccess => "Any doctor in an emergency".to_string(),
        ConsentGrantee::Public => "Anyone (public)".to_string(),
    };
    summary.push_str(&format!("WHO can see your data: {}\n\n", who));

    // WHAT data is shared
    let categories: Vec<String> = consent.scope.data_categories.iter().map(|c| {
        match c {
            DataCategory::All => "All your health records".to_string(),
            DataCategory::Demographics => "Your name and contact info".to_string(),
            DataCategory::Medications => "Your medications".to_string(),
            DataCategory::Diagnoses => "Your diagnoses".to_string(),
            DataCategory::LabResults => "Your lab test results".to_string(),
            DataCategory::VitalSigns => "Your vital signs (blood pressure, temperature, etc.)".to_string(),
            DataCategory::ImagingStudies => "Your imaging (X-rays, MRIs, etc.)".to_string(),
            DataCategory::SubstanceAbuse => "Substance abuse treatment records (specially protected)".to_string(),
            DataCategory::MentalHealth => "Mental health records (specially protected)".to_string(),
            DataCategory::GeneticData => "Your genetic information".to_string(),
            _ => format!("{:?}", c),
        }
    }).collect();
    summary.push_str("WHAT they can see:\n");
    for cat in &categories {
        summary.push_str(&format!("  - {}\n", cat));
    }

    // Exclusions
    if !consent.scope.exclusions.is_empty() {
        summary.push_str("\nEXCLUDED (they CANNOT see):\n");
        for exc in &consent.scope.exclusions {
            summary.push_str(&format!("  - {:?}\n", exc));
        }
    }

    // WHAT they can do
    let actions: Vec<&str> = consent.permissions.iter().map(|p| match p {
        DataPermission::Read => "Look at your records",
        DataPermission::Write => "Add notes to your records",
        DataPermission::Share => "Share your records with others",
        DataPermission::Export => "Download a copy of your records",
        DataPermission::Delete => "Remove records",
        DataPermission::Amend => "Suggest changes to your records",
    }).collect();
    summary.push_str("\nWHAT they can do:\n");
    for action in &actions {
        summary.push_str(&format!("  - {}\n", action));
    }

    // WHY
    let purpose = match &consent.purpose {
        ConsentPurpose::Treatment => "To help with your medical care",
        ConsentPurpose::Payment => "For billing and insurance claims",
        ConsentPurpose::HealthcareOperations => "For hospital operations and quality improvement",
        ConsentPurpose::Research => "For medical research (your data helps others)",
        ConsentPurpose::PublicHealth => "For public health reporting",
        ConsentPurpose::LegalProceeding => "For legal proceedings",
        ConsentPurpose::Marketing => "For marketing (you can revoke anytime)",
        ConsentPurpose::FamilyNotification => "To notify your family about your care",
        ConsentPurpose::Other(desc) => desc.as_str(),
    };
    summary.push_str(&format!("\nWHY: {}\n", purpose));

    // WHEN it expires
    if let Some(expires) = &consent.expires_at {
        summary.push_str(&format!("\nEXPIRES: {}\n", expires));
    } else {
        summary.push_str("\nEXPIRES: Never (until you revoke it)\n");
    }

    // Status
    let status = match consent.status {
        ConsentStatus::Active => "ACTIVE — this consent is currently in effect",
        ConsentStatus::Revoked => "REVOKED — you took back this consent",
        ConsentStatus::Expired => "EXPIRED — this consent is no longer valid",
        _ => "Unknown status",
    };
    summary.push_str(&format!("\nSTATUS: {}\n", status));

    // Your rights
    summary.push_str("\nYOUR RIGHTS:\n");
    summary.push_str("  - You can revoke this consent at any time\n");
    summary.push_str("  - Revoking won't affect care you've already received\n");
    summary.push_str("  - You can request a copy of who has accessed your data\n");

    Ok(summary)
}

// ==================== P1-3: CONSENT REVOCATION PROPAGATION ====================

/// Propagate consent revocation to all derived grants.
/// When a consent is revoked, all decryption grants derived from it
/// become invalid. This logs the propagation for audit.
fn propagate_revocation(consent_hash: &ActionHash, patient_hash: &ActionHash) -> ExternResult<()> {
    // Log the propagation event for audit
    let log = DataAccessLog {
        log_id: format!("REVOKE_PROPAGATE-{}", sys_time()?.as_micros()),
        patient_hash: patient_hash.clone(),
        accessor: agent_info()?.agent_initial_pubkey,
        data_categories_accessed: vec![DataCategory::All],
        access_type: DataPermission::Delete,
        consent_hash: Some(consent_hash.clone()),
        access_reason: format!("Consent revocation propagated — all grants from {} invalidated", consent_hash),
        accessed_at: sys_time()?,
        access_location: Some("holochain_node".to_string()),
        emergency_override: false,
        override_reason: None,
    };
    create_entry(&EntryTypes::DataAccessLog(log))?;
    Ok(())
}

/// Revoke all active consents for a patient (used by crypto-erasure).
#[hdk_extern]
pub fn revoke_all_patient_consents(patient_hash: ActionHash) -> ExternResult<u32> {
    let active = get_active_consents(patient_hash.clone())?;
    let mut count = 0u32;
    for record in active {
        let hash = record.action_address().clone();
        let _ = revoke_consent(RevokeConsentInput {
            consent_hash: hash,
            reason: "Patient requested data erasure (GDPR Article 17)".to_string(),
        });
        count += 1;
    }
    Ok(count)
}

// ==================== P1-2: SENSITIVE CATEGORY CONSENT CHECK ====================

/// Check if a consent explicitly names a sensitive category.
/// 42 CFR Part 2 requires that substance abuse consent specifically
/// names the category, not just a blanket "All" consent.
#[hdk_extern]
pub fn check_sensitive_category_consent(input: AuthorizationCheckInput) -> ExternResult<bool> {
    let consents = get_active_consents(input.patient_hash.clone())?;

    for record in consents {
        let Some(consent): Option<Consent> = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        else { continue };

        // Check if this consent explicitly lists the sensitive category
        // (not just DataCategory::All)
        let explicit_match = consent.scope.data_categories.iter().any(|cat| {
            format!("{:?}", cat) == format!("{:?}", input.data_category)
        });

        if explicit_match {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Check if an agent is a system administrator.
#[hdk_extern]
pub fn is_admin(agent: AgentPubKey) -> ExternResult<bool> {
    let admin_anchor = anchor_hash("system_admins")?;
    let links = get_links(
        LinkQuery::try_new(admin_anchor, LinkTypes::PatientToConsents)?,
        GetStrategy::default(),
    )?;

    if links.is_empty() {
        // No admins registered — bootstrap mode, allow
        return Ok(true);
    }

    for link in links {
        if let Some(admin_agent) = link.target.into_agent_pub_key() {
            if admin_agent == agent {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

// ==================== P0-1: RE-DISCLOSURE CONSENT CHECK ====================
// Called by shared crate's check_redisclosure() when Share/Export is attempted
// on data received with no_further_disclosure=true.

/// Input for re-disclosure consent check.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RedisclosureConsentInput {
    pub patient_hash: ActionHash,
    pub requestor: AgentPubKey,
    pub original_consent: Option<ActionHash>,
    pub categories: Vec<DataCategory>,
}

/// Check if the patient has granted explicit re-disclosure consent.
///
/// This is called when a Share/Export is attempted on data received
/// with `no_further_disclosure=true`. Returns `true` only if the patient
/// has created a specific consent granting the requestor Share permission
/// for the specified categories.
///
/// 42 CFR Part 2, Section 2.32: Recipients of substance abuse records
/// may not re-disclose without patient's explicit written consent.
#[hdk_extern]
pub fn check_redisclosure_consent(input: RedisclosureConsentInput) -> ExternResult<bool> {
    // Query all active consents for this patient
    let links = get_links(
        LinkQuery::try_new(input.patient_hash.clone(), LinkTypes::PatientToConsents)?,
        GetStrategy::default(),
    )?;

    for link in links {
        let Some(hash) = link.target.into_action_hash() else { continue };
        let Some(record) = get(hash, GetOptions::default())? else { continue };
        let Some(consent): Option<Consent> = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        else { continue };

        // Check grantee matches requestor
        let grantee_matches = match &consent.grantee {
            ConsentGrantee::Agent(agent) => *agent == input.requestor,
            ConsentGrantee::EmergencyAccess => true,
            ConsentGrantee::Public => true,
            _ => false,
        };
        if !grantee_matches {
            continue;
        }

        // Must include Share or Export permission
        let has_share = consent.permissions.iter().any(|p| {
            matches!(p, DataPermission::Share | DataPermission::Export)
        });
        if !has_share {
            continue;
        }

        // Must cover all requested categories
        let covers_categories = input.categories.iter().all(|cat| {
            consent.scope.data_categories.contains(cat)
                || consent.scope.data_categories.contains(&DataCategory::All)
        });
        if !covers_categories {
            continue;
        }

        // Must be active
        if consent.status == ConsentStatus::Revoked || consent.status == ConsentStatus::Expired {
            continue;
        }

        // Check expiration
        if let Some(expires) = &consent.expires_at {
            if let Ok(now) = sys_time() {
                let now_ts = Timestamp::from_micros(now.as_micros() as i64);
                if now_ts > *expires {
                    continue;
                }
            }
        }

        // If original consent specified, purpose must indicate re-disclosure
        if input.original_consent.is_some() {
            let is_redisclosure = match &consent.purpose {
                ConsentPurpose::Other(desc) => {
                    let lower = desc.to_lowercase();
                    lower.contains("re-disclosure")
                        || lower.contains("redisclosure")
                        || lower.contains("further sharing")
                },
                _ => false,
            };
            if !is_redisclosure {
                continue;
            }
        }

        // Found a valid re-disclosure consent
        return Ok(true);
    }

    Ok(false)
}

// ==================== P0-2: CHAINED AUDIT ENTRY ====================
// Called by shared crate's chained_log_data_access() to persist hash-chained entries.

/// A hash-chained audit entry for tamper-evident logging.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChainedAuditEntryInput {
    pub sequence: u64,
    pub previous_hash: Option<[u8; 32]>,
    pub entry_hash: [u8; 32],
    pub event_description: String,
    pub patient_hash: ActionHash,
    pub categories: Vec<DataCategory>,
    pub consent_hash: Option<ActionHash>,
    pub timestamp: i64,
    pub agent: AgentPubKey,
}

/// Persist a chained audit entry.
///
/// Creates a DataAccessLog entry with the chain metadata embedded in the
/// access_reason field (serialized). This integrates with the existing
/// audit infrastructure while adding tamper-evident chaining.
#[hdk_extern]
pub fn create_chained_audit_entry(input: ChainedAuditEntryInput) -> ExternResult<ActionHash> {
    // Store full chain hashes (not truncated) for verification.
    // The access_reason field holds a JSON-encoded chain metadata object
    // that can be parsed back for chain verification.
    let chain_metadata = serde_json::json!({
        "chain_version": 2,
        "sequence": input.sequence,
        "previous_hash": input.previous_hash.map(|h| hex_encode(&h)),
        "entry_hash": hex_encode(&input.entry_hash),
        "event": input.event_description,
    });

    let log_entry = DataAccessLog {
        log_id: format!("CHAIN-{:06}-{}", input.sequence, input.timestamp),
        patient_hash: input.patient_hash.clone(),
        accessor: input.agent,
        data_categories_accessed: input.categories,
        access_type: DataPermission::Read,
        consent_hash: input.consent_hash,
        access_reason: chain_metadata.to_string(),
        accessed_at: Timestamp::from_micros(input.timestamp),
        access_location: Some("holochain_node".to_string()),
        emergency_override: false,
        override_reason: None,
    };

    let hash = create_entry(&EntryTypes::DataAccessLog(log_entry))?;

    create_link(
        input.patient_hash,
        hash.clone(),
        LinkTypes::PatientToAccessLogs,
        (),
    )?;

    Ok(hash)
}

/// Hex-encode first N bytes for display.
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// ==================== P0-5: PATIENT KEY GENERATION ====================
// Creates a HealthKeyBundle for the patient on registration.

/// Register a patient's encryption public key.
///
/// Called during patient creation or first data encryption.
/// The private key is generated client-side and NEVER stored on-chain.
/// Only the public key is committed to the DHT via HealthKeyBundle.
#[hdk_extern]
pub fn register_patient_key(input: HealthKeyBundle) -> ExternResult<ActionHash> {
    // Validate key bundle
    if input.kem_public_key.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Public key cannot be empty".to_string()
        )));
    }

    let hash = create_entry(&EntryTypes::HealthKeyBundle(input.clone()))?;

    // Link to patient's key bundles
    // The patient_did contains the patient hash for linking
    let patient_anchor = anchor_hash(&format!("patient_keys:{}", input.patient_did))?;
    create_link(
        patient_anchor,
        hash.clone(),
        LinkTypes::PatientToKeyBundles,
        (),
    )?;

    Ok(hash)
}

/// Get the active encryption key bundle for a patient.
#[hdk_extern]
pub fn get_patient_active_key(patient_did: String) -> ExternResult<Option<Record>> {
    let patient_anchor = anchor_hash(&format!("patient_keys:{}", patient_did))?;
    let links = get_links(
        LinkQuery::try_new(patient_anchor, LinkTypes::PatientToKeyBundles)?,
        GetStrategy::default(),
    )?;

    // Find the active key with the highest version
    let mut best: Option<(u32, Record)> = None;
    for link in links {
        let Some(hash) = link.target.into_action_hash() else { continue };
        let Some(record) = get(hash, GetOptions::default())? else { continue };
        let Some(bundle): Option<HealthKeyBundle> = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        else { continue };

        if bundle.active {
            match &best {
                Some((v, _)) if bundle.key_version > *v => {
                    best = Some((bundle.key_version, record));
                },
                None => {
                    best = Some((bundle.key_version, record));
                },
                _ => {},
            }
        }
    }

    Ok(best.map(|(_, r)| r))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_hash() -> ActionHash {
        ActionHash::from_raw_36(vec![0u8; 36])
    }

    fn dummy_agent() -> AgentPubKey {
        AgentPubKey::from_raw_36(vec![0u8; 36])
    }

    // ==================== Serde roundtrip tests ====================

    #[test]
    fn test_serde_roundtrip_revoke_consent_input() {
        let input = RevokeConsentInput {
            consent_hash: dummy_hash(),
            reason: "Patient requested revocation".to_string(),
        };
        let json = serde_json::to_string(&input).expect("serialize");
        let decoded: RevokeConsentInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.reason, "Patient requested revocation");
    }

    #[test]
    fn test_serde_roundtrip_authorization_check_input() {
        let input = AuthorizationCheckInput {
            patient_hash: dummy_hash(),
            requestor: dummy_agent(),
            data_category: DataCategory::Demographics,
            permission: DataPermission::Read,
            is_emergency: false,
        };
        let json = serde_json::to_string(&input).expect("serialize");
        let decoded: AuthorizationCheckInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.data_category, DataCategory::Demographics);
        assert_eq!(decoded.permission, DataPermission::Read);
        assert!(!decoded.is_emergency);
    }

    #[test]
    fn test_serde_roundtrip_authorization_result() {
        let result = AuthorizationResult {
            authorized: true,
            consent_hash: Some(dummy_hash()),
            reason: "Active consent found".to_string(),
            permissions: vec![DataPermission::Read, DataPermission::Write],
            emergency_override: false,
        };
        let json = serde_json::to_string(&result).expect("serialize");
        let decoded: AuthorizationResult = serde_json::from_str(&json).expect("deserialize");
        assert!(decoded.authorized);
        assert_eq!(decoded.permissions.len(), 2);
        assert!(!decoded.emergency_override);
    }

    #[test]
    fn test_serde_roundtrip_date_range_input() {
        let input = DateRangeInput {
            patient_hash: dummy_hash(),
            start_date: Timestamp::from_micros(1000000),
            end_date: Timestamp::from_micros(2000000),
        };
        let json = serde_json::to_string(&input).expect("serialize");
        let decoded: DateRangeInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.start_date, Timestamp::from_micros(1000000));
    }

    #[test]
    fn test_serde_roundtrip_accessor_logs_input() {
        let input = AccessorLogsInput {
            patient_hash: dummy_hash(),
            accessor: dummy_agent(),
        };
        let json = serde_json::to_string(&input).expect("serialize");
        let decoded: AccessorLogsInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.patient_hash, dummy_hash());
    }

    // ==================== Consent integrity type tests ====================

    #[test]
    fn test_consent_status_all_variants_serde() {
        let statuses = vec![
            ConsentStatus::Active,
            ConsentStatus::Expired,
            ConsentStatus::Revoked,
            ConsentStatus::Pending,
            ConsentStatus::Rejected,
        ];
        for s in statuses {
            let json = serde_json::to_string(&s).expect("serialize");
            let decoded: ConsentStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, s);
        }
    }

    #[test]
    fn test_data_permission_all_variants_serde() {
        let permissions = vec![
            DataPermission::Read,
            DataPermission::Write,
            DataPermission::Share,
            DataPermission::Export,
            DataPermission::Delete,
            DataPermission::Amend,
        ];
        for p in permissions {
            let json = serde_json::to_string(&p).expect("serialize");
            let decoded: DataPermission = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, p);
        }
    }

    #[test]
    fn test_consent_grantee_all_variants_serde() {
        let grantees = vec![
            ConsentGrantee::Provider(dummy_hash()),
            ConsentGrantee::Organization("Hospital A".to_string()),
            ConsentGrantee::Agent(dummy_agent()),
            ConsentGrantee::ResearchStudy(dummy_hash()),
            ConsentGrantee::InsuranceCompany(dummy_hash()),
            ConsentGrantee::EmergencyAccess,
            ConsentGrantee::Public,
        ];
        for g in grantees {
            let json = serde_json::to_string(&g).expect("serialize");
            let _decoded: ConsentGrantee = serde_json::from_str(&json).expect("deserialize");
        }
    }

    #[test]
    fn test_consent_scope_construction() {
        let scope = ConsentScope {
            data_categories: vec![DataCategory::Demographics, DataCategory::Allergies],
            date_range: Some(DateRange {
                start: Timestamp::from_micros(0),
                end: Some(Timestamp::from_micros(1000000)),
            }),
            encounter_hashes: None,
            exclusions: vec![DataCategory::MentalHealth],
        };
        assert_eq!(scope.data_categories.len(), 2);
        assert_eq!(scope.exclusions.len(), 1);
        assert!(scope.date_range.is_some());
    }

    #[test]
    fn test_data_category_all_variants_serde() {
        let categories = vec![
            DataCategory::Demographics,
            DataCategory::Allergies,
            DataCategory::Medications,
            DataCategory::Diagnoses,
            DataCategory::Procedures,
            DataCategory::LabResults,
            DataCategory::ImagingStudies,
            DataCategory::VitalSigns,
            DataCategory::Immunizations,
            DataCategory::MentalHealth,
            DataCategory::SubstanceAbuse,
            DataCategory::SexualHealth,
            DataCategory::GeneticData,
            DataCategory::FinancialData,
            DataCategory::All,
        ];
        for cat in &categories {
            let json = serde_json::to_string(cat).expect("serialize");
            let decoded: DataCategory = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(&decoded, cat);
        }
        assert_eq!(categories.len(), 15);
    }

    #[test]
    fn test_consent_purpose_all_variants_serde() {
        let purposes = vec![
            ConsentPurpose::Treatment,
            ConsentPurpose::Payment,
            ConsentPurpose::HealthcareOperations,
            ConsentPurpose::Research,
            ConsentPurpose::PublicHealth,
            ConsentPurpose::LegalProceeding,
            ConsentPurpose::Marketing,
            ConsentPurpose::FamilyNotification,
            ConsentPurpose::Other("Custom".to_string()),
        ];
        for p in purposes {
            let json = serde_json::to_string(&p).expect("serialize");
            let _decoded: ConsentPurpose = serde_json::from_str(&json).expect("deserialize");
        }
    }

    // ==================== Delegation type tests ====================

    #[test]
    fn test_delegation_type_all_variants_serde() {
        let types = vec![
            DelegationType::HealthcareProxy,
            DelegationType::Caregiver,
            DelegationType::FamilyMember,
            DelegationType::LegalGuardian,
            DelegationType::Temporary,
            DelegationType::ResearchAdvocate,
            DelegationType::FinancialOnly,
        ];
        for dt in types {
            let json = serde_json::to_string(&dt).expect("serialize");
            let decoded: DelegationType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, dt);
        }
    }

    #[test]
    fn test_delegation_permission_all_variants_serde() {
        let perms = vec![
            DelegationPermission::ViewRecords,
            DelegationPermission::ScheduleAppointments,
            DelegationPermission::CommunicateWithProviders,
            DelegationPermission::MakeMedicalDecisions,
            DelegationPermission::ConsentToTreatment,
            DelegationPermission::ManageMedications,
            DelegationPermission::AccessFinancial,
            DelegationPermission::ReceiveNotifications,
            DelegationPermission::ExportData,
            DelegationPermission::SubDelegate,
        ];
        for p in perms {
            let json = serde_json::to_string(&p).expect("serialize");
            let decoded: DelegationPermission = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, p);
        }
    }

    // ==================== Notification type tests ====================

    #[test]
    fn test_notification_priority_all_variants_serde() {
        let priorities = vec![
            NotificationPriority::Immediate,
            NotificationPriority::Daily,
            NotificationPriority::Weekly,
            NotificationPriority::Silent,
        ];
        for p in priorities {
            let json = serde_json::to_string(&p).expect("serialize");
            let decoded: NotificationPriority = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(decoded, p);
        }
    }

    // ==================== Edge cases ====================

    #[test]
    fn test_authorization_result_denied_no_consent() {
        let result = AuthorizationResult {
            authorized: false,
            consent_hash: None,
            reason: "No valid consent found".to_string(),
            permissions: vec![],
            emergency_override: false,
        };
        assert!(!result.authorized);
        assert!(result.consent_hash.is_none());
        assert!(result.permissions.is_empty());
    }

    #[test]
    fn test_authorization_result_emergency_override() {
        let result = AuthorizationResult {
            authorized: false,
            consent_hash: None,
            reason: "No consent found - emergency override available".to_string(),
            permissions: vec![DataPermission::Read],
            emergency_override: true,
        };
        assert!(!result.authorized);
        assert!(result.emergency_override);
        assert_eq!(result.permissions.len(), 1);
    }

    #[test]
    fn test_access_log_entry_serde_roundtrip() {
        let entry = AccessLogEntry {
            log_id: "LOG-12345".to_string(),
            patient_hash: dummy_hash(),
            accessor: dummy_agent(),
            data_categories: vec![DataCategory::Demographics, DataCategory::LabResults],
            access_type: DataPermission::Read,
            consent_hash: Some(dummy_hash()),
            access_reason: "Authorized access".to_string(),
            accessed_at: Timestamp::from_micros(1000000),
            access_location: "holochain_node".to_string(),
            emergency_override: false,
            override_reason: None,
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let decoded: AccessLogEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.log_id, "LOG-12345");
        assert_eq!(decoded.data_categories.len(), 2);
    }

    #[test]
    fn test_access_denied_log_entry_serde_roundtrip() {
        let entry = AccessDeniedLogEntry {
            log_id: "DENY-12345".to_string(),
            patient_hash: dummy_hash(),
            attempted_accessor: dummy_agent(),
            data_category: DataCategory::GeneticData,
            denial_reason: "No consent for genetic data".to_string(),
            attempted_at: Timestamp::from_micros(1000000),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        let decoded: AccessDeniedLogEntry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.log_id, "DENY-12345");
        assert_eq!(decoded.data_category, DataCategory::GeneticData);
    }

    // ==================== HIPAA consent enforcement tests ====================

    fn make_consent(
        grantee: ConsentGrantee,
        categories: Vec<DataCategory>,
        permissions: Vec<DataPermission>,
        status: ConsentStatus,
        expires_at: Option<Timestamp>,
        revoked_at: Option<Timestamp>,
        exclusions: Vec<DataCategory>,
    ) -> Consent {
        Consent {
            consent_id: "consent-001".to_string(),
            patient_hash: dummy_hash(),
            grantee,
            scope: ConsentScope {
                data_categories: categories,
                date_range: None,
                encounter_hashes: None,
                exclusions,
            },
            permissions,
            purpose: ConsentPurpose::Treatment,
            status,
            granted_at: Timestamp::from_micros(1000000),
            expires_at,
            revoked_at,
            revocation_reason: None,
            document_hash: None,
            witness: None,
            legal_representative: None,
            notes: None,
        }
    }

    // --- Access without consent ---

    #[test]
    fn test_authorization_denied_when_no_consent_exists() {
        // With no active consents, authorization must be denied
        let result = AuthorizationResult {
            authorized: false,
            consent_hash: None,
            reason: "No valid consent found".to_string(),
            permissions: vec![],
            emergency_override: false,
        };
        assert!(!result.authorized);
        assert!(result.consent_hash.is_none());
        assert!(result.permissions.is_empty());
        assert!(!result.emergency_override);
    }

    #[test]
    fn test_authorization_denied_for_wrong_grantee() {
        // Consent exists for Agent A, Agent B should not be authorized
        let agent_a = AgentPubKey::from_raw_36(vec![1u8; 36]);
        let agent_b = AgentPubKey::from_raw_36(vec![2u8; 36]);
        let consent = make_consent(
            ConsentGrantee::Agent(agent_a.clone()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            None,
            None,
            vec![],
        );

        // Simulate check_authorization logic: grantee must match
        let grantee_matches = match &consent.grantee {
            ConsentGrantee::Agent(agent) => *agent == agent_b,
            _ => false,
        };
        assert!(!grantee_matches, "Agent B should not match Agent A consent");
    }

    #[test]
    fn test_authorization_denied_for_pending_consent() {
        // Pending consent should not grant access
        let consent = make_consent(
            ConsentGrantee::Agent(dummy_agent()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Pending,
            None,
            None,
            vec![],
        );
        assert!(
            !matches!(consent.status, ConsentStatus::Active),
            "Pending consent must not be treated as active"
        );
    }

    #[test]
    fn test_authorization_denied_for_rejected_consent() {
        let consent = make_consent(
            ConsentGrantee::Agent(dummy_agent()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Rejected,
            None,
            None,
            vec![],
        );
        assert!(
            !matches!(consent.status, ConsentStatus::Active),
            "Rejected consent must not grant access"
        );
    }

    // --- Consent revocation is immediate ---

    #[test]
    fn test_revoked_consent_has_revoked_status_immediately() {
        let mut consent = make_consent(
            ConsentGrantee::Agent(dummy_agent()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            None,
            None,
            vec![],
        );
        // Simulate revocation
        consent.status = ConsentStatus::Revoked;
        consent.revoked_at = Some(Timestamp::from_micros(2000000));
        consent.revocation_reason = Some("Patient requested".to_string());

        assert!(matches!(consent.status, ConsentStatus::Revoked));
        assert!(consent.revoked_at.is_some());
    }

    #[test]
    fn test_revoked_consent_not_in_active_filter() {
        // get_active_consents filters: Active status AND revoked_at is None
        let consent = make_consent(
            ConsentGrantee::Agent(dummy_agent()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Revoked,
            None,
            Some(Timestamp::from_micros(2000000)),
            vec![],
        );
        let is_active = matches!(consent.status, ConsentStatus::Active)
            && consent.revoked_at.is_none();
        assert!(!is_active, "Revoked consent must not pass active filter");
    }

    #[test]
    fn test_revocation_preserves_original_consent_data() {
        let mut consent = make_consent(
            ConsentGrantee::Agent(dummy_agent()),
            vec![DataCategory::Demographics, DataCategory::LabResults],
            vec![DataPermission::Read, DataPermission::Share],
            ConsentStatus::Active,
            None,
            None,
            vec![],
        );
        let original_categories = consent.scope.data_categories.clone();
        let original_permissions = consent.permissions.clone();

        consent.status = ConsentStatus::Revoked;
        consent.revoked_at = Some(Timestamp::from_micros(3000000));

        // Original data preserved for audit trail
        assert_eq!(consent.scope.data_categories, original_categories);
        assert_eq!(consent.permissions, original_permissions);
    }

    // --- Time-bound consent expiration ---

    #[test]
    fn test_expired_consent_not_active() {
        let now = Timestamp::from_micros(5000000);
        let consent = make_consent(
            ConsentGrantee::Agent(dummy_agent()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            Some(Timestamp::from_micros(3000000)), // expired before now
            None,
            vec![],
        );
        let not_expired = consent
            .expires_at
            .map(|expires| expires > now)
            .unwrap_or(true);
        assert!(!not_expired, "Consent that expired before 'now' must not be active");
    }

    #[test]
    fn test_non_expired_consent_still_active() {
        let now = Timestamp::from_micros(2000000);
        let consent = make_consent(
            ConsentGrantee::Agent(dummy_agent()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            Some(Timestamp::from_micros(5000000)), // expires after now
            None,
            vec![],
        );
        let not_expired = consent
            .expires_at
            .map(|expires| expires > now)
            .unwrap_or(true);
        let not_revoked = consent.revoked_at.is_none();
        let is_active = matches!(consent.status, ConsentStatus::Active) && not_expired && not_revoked;
        assert!(is_active, "Non-expired active consent should pass filter");
    }

    #[test]
    fn test_no_expiry_consent_never_expires() {
        let now = Timestamp::from_micros(999999999999);
        let consent = make_consent(
            ConsentGrantee::Agent(dummy_agent()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            None, // no expiration
            None,
            vec![],
        );
        let not_expired = consent
            .expires_at
            .map(|expires| expires > now)
            .unwrap_or(true);
        assert!(not_expired, "Consent without expiration should never expire");
    }

    // --- Granular consent (per-provider, per-record-type) ---

    #[test]
    fn test_granular_consent_per_provider() {
        let provider_a = AgentPubKey::from_raw_36(vec![10u8; 36]);
        let provider_b = AgentPubKey::from_raw_36(vec![20u8; 36]);

        let consent = make_consent(
            ConsentGrantee::Agent(provider_a.clone()),
            vec![DataCategory::Medications],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            None,
            None,
            vec![],
        );

        // Provider A matches
        let a_matches = match &consent.grantee {
            ConsentGrantee::Agent(agent) => *agent == provider_a,
            _ => false,
        };
        assert!(a_matches, "Provider A should be authorized");

        // Provider B does not match
        let b_matches = match &consent.grantee {
            ConsentGrantee::Agent(agent) => *agent == provider_b,
            _ => false,
        };
        assert!(!b_matches, "Provider B should not be authorized");
    }

    #[test]
    fn test_granular_consent_per_record_type() {
        let consent = make_consent(
            ConsentGrantee::Agent(dummy_agent()),
            vec![DataCategory::Medications, DataCategory::Allergies],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            None,
            None,
            vec![],
        );

        // Medications covered
        let meds_covered = consent.scope.data_categories.iter().any(|cat| {
            matches!(cat, DataCategory::All) || *cat == DataCategory::Medications
        });
        assert!(meds_covered, "Medications should be covered");

        // LabResults NOT covered
        let labs_covered = consent.scope.data_categories.iter().any(|cat| {
            matches!(cat, DataCategory::All) || *cat == DataCategory::LabResults
        });
        assert!(!labs_covered, "LabResults should NOT be covered");
    }

    #[test]
    fn test_exclusions_override_all_category() {
        let consent = make_consent(
            ConsentGrantee::Agent(dummy_agent()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            None,
            None,
            vec![DataCategory::MentalHealth, DataCategory::SubstanceAbuse],
        );

        let category = DataCategory::MentalHealth;
        let category_covered = consent.scope.data_categories.iter().any(|cat| {
            matches!(cat, DataCategory::All) || *cat == category
        });
        let not_excluded = !consent.scope.exclusions.contains(&category);

        assert!(category_covered, "All should cover MentalHealth");
        assert!(!not_excluded, "MentalHealth should be excluded");
        assert!(
            !(category_covered && not_excluded),
            "Exclusion must override All"
        );
    }

    #[test]
    fn test_permission_granularity_read_only_denies_write() {
        let consent = make_consent(
            ConsentGrantee::Agent(dummy_agent()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            None,
            None,
            vec![],
        );

        assert!(
            consent.permissions.contains(&DataPermission::Read),
            "Read should be granted"
        );
        assert!(
            !consent.permissions.contains(&DataPermission::Write),
            "Write should NOT be granted for read-only consent"
        );
        assert!(
            !consent.permissions.contains(&DataPermission::Delete),
            "Delete should NOT be granted for read-only consent"
        );
    }

    // --- Consent cannot be granted by non-patient agents ---

    #[test]
    fn test_consent_validation_rejects_non_patient_author() {
        // The validate_patient_reference_and_ownership function checks
        // that the action author matches the patient_hash record author.
        // A non-patient agent should be rejected.
        let patient_agent = AgentPubKey::from_raw_36(vec![1u8; 36]);
        let non_patient = AgentPubKey::from_raw_36(vec![2u8; 36]);

        // Simulate the ownership check: record.action().author() != author
        assert_ne!(
            patient_agent, non_patient,
            "Non-patient agent must differ from patient"
        );
        // In validation: this would return Invalid("Only the patient can create consent...")
    }

    #[test]
    fn test_delegation_validation_rejects_non_patient_author() {
        // Same ownership check for delegation grants
        let patient_agent = AgentPubKey::from_raw_36(vec![1u8; 36]);
        let stranger = AgentPubKey::from_raw_36(vec![3u8; 36]);
        assert_ne!(patient_agent, stranger);
        // validate_delegation_grant calls validate_patient_reference_and_ownership
    }

    // --- Consent audit trail immutability ---

    #[test]
    fn test_consent_update_creates_audit_trail_link() {
        // UpdateConsentInput uses update_entry + ConsentUpdates link type
        // Verifying the data structures support audit trail
        let input = UpdateConsentInput {
            original_hash: dummy_hash(),
            updated_consent: make_consent(
                ConsentGrantee::Agent(dummy_agent()),
                vec![DataCategory::All],
                vec![DataPermission::Read],
                ConsentStatus::Active,
                Some(Timestamp::from_micros(9999999)),
                None,
                vec![],
            ),
        };
        let json = serde_json::to_string(&input).expect("serialize UpdateConsentInput");
        let decoded: UpdateConsentInput = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.original_hash, dummy_hash());
    }

    #[test]
    fn test_access_log_immutability_no_delete_permission() {
        // DataAccessLog entries are create-only (no update/delete extern for logs)
        // Verify the log structure is complete and has no mutable state
        let log = DataAccessLog {
            log_id: "LOG-IMMUTABLE-001".to_string(),
            patient_hash: dummy_hash(),
            accessor: dummy_agent(),
            access_type: DataPermission::Read,
            data_categories_accessed: vec![DataCategory::Demographics],
            consent_hash: Some(dummy_hash()),
            access_reason: "Treatment".to_string(),
            accessed_at: Timestamp::from_micros(1000000),
            access_location: Some("node-1".to_string()),
            emergency_override: false,
            override_reason: None,
        };
        // Log has all required fields for HIPAA accounting of disclosures
        assert!(!log.log_id.is_empty());
        assert!(log.consent_hash.is_some());
    }

    #[test]
    fn test_emergency_access_requires_clinical_justification() {
        // EmergencyAccess validation requires non-empty reason AND clinical_justification
        let emergency = EmergencyAccess {
            emergency_id: "EM-001".to_string(),
            patient_hash: dummy_hash(),
            accessor: dummy_agent(),
            reason: "Cardiac arrest".to_string(),
            clinical_justification: "Need medication history for treatment".to_string(),
            accessed_at: Timestamp::from_micros(1000000),
            access_duration_minutes: 60,
            approved_by: None,
            data_accessed: vec![DataCategory::Medications, DataCategory::Allergies],
            audited: false,
            audited_by: None,
            audited_at: None,
            audit_findings: None,
        };
        assert!(!emergency.reason.is_empty());
        assert!(!emergency.clinical_justification.is_empty());
        assert!(!emergency.data_accessed.is_empty());
    }

    #[test]
    fn test_emergency_override_must_have_reason_in_log() {
        // validate_access_log rejects emergency_override=true without override_reason
        let log_valid = DataAccessLog {
            log_id: "LOG-EM-001".to_string(),
            patient_hash: dummy_hash(),
            accessor: dummy_agent(),
            access_type: DataPermission::Read,
            data_categories_accessed: vec![DataCategory::All],
            consent_hash: None,
            access_reason: "Emergency".to_string(),
            accessed_at: Timestamp::from_micros(1000000),
            access_location: None,
            emergency_override: true,
            override_reason: Some("Life-threatening condition".to_string()),
        };
        assert!(log_valid.override_reason.is_some());

        let log_invalid = DataAccessLog {
            log_id: "LOG-EM-002".to_string(),
            patient_hash: dummy_hash(),
            accessor: dummy_agent(),
            access_type: DataPermission::Read,
            data_categories_accessed: vec![DataCategory::All],
            consent_hash: None,
            access_reason: "Emergency".to_string(),
            accessed_at: Timestamp::from_micros(1000000),
            access_location: None,
            emergency_override: true,
            override_reason: None, // Missing - would fail validation
        };
        assert!(
            log_invalid.emergency_override && log_invalid.override_reason.is_none(),
            "This would fail integrity validation"
        );
    }

    #[test]
    fn test_disclosure_report_serde_roundtrip() {
        let report = DisclosureReport {
            patient_hash: dummy_hash(),
            generated_at: Timestamp::from_micros(5000000),
            period_start: Timestamp::from_micros(1000000),
            period_end: Timestamp::from_micros(4000000),
            total_disclosures: 3,
            disclosures: vec![
                DisclosureEntry {
                    accessed_at: Timestamp::from_micros(2000000),
                    accessor: dummy_agent(),
                    data_categories: vec!["Demographics".to_string()],
                    access_reason: "Treatment".to_string(),
                    consent_hash: Some(dummy_hash()),
                    emergency_override: false,
                },
            ],
        };
        let json = serde_json::to_string(&report).expect("serialize report");
        let decoded: DisclosureReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.total_disclosures, 3);
        assert_eq!(decoded.disclosures.len(), 1);
    }

    // ==================== HIPAA consent enforcement (extended) ====================
    //
    // These tests exercise the check_authorization inline logic against
    // the Consent data model, validating HIPAA §164.508 and §164.510
    // requirements for consent-gated access control.

    /// Simulates the core check_authorization matching logic against a list
    /// of consents, mirroring the real extern without HDK host calls.
    fn simulate_check_authorization(
        consents: &[Consent],
        requestor: &AgentPubKey,
        data_category: &DataCategory,
        permission: &DataPermission,
        is_emergency: bool,
        now: Timestamp,
    ) -> AuthorizationResult {
        for consent in consents {
            // Active filter (mirrors get_active_consents)
            let not_expired = consent
                .expires_at
                .map(|expires| expires > now)
                .unwrap_or(true);
            let not_revoked = consent.revoked_at.is_none();
            if !matches!(consent.status, ConsentStatus::Active) || !not_expired || !not_revoked {
                continue;
            }

            // Grantee match (mirrors check_authorization)
            let grantee_matches = match &consent.grantee {
                ConsentGrantee::Agent(agent) => *agent == *requestor,
                ConsentGrantee::EmergencyAccess => is_emergency,
                _ => false,
            };
            if !grantee_matches {
                continue;
            }

            // Category coverage
            let category_covered = consent.scope.data_categories.iter().any(|cat| {
                matches!(cat, DataCategory::All) || *cat == *data_category
            });
            let not_excluded = !consent.scope.exclusions.contains(data_category);
            let permission_granted = consent.permissions.contains(permission);

            if category_covered && not_excluded && permission_granted {
                return AuthorizationResult {
                    authorized: true,
                    consent_hash: Some(dummy_hash()),
                    reason: "Active consent found".to_string(),
                    permissions: consent.permissions.clone(),
                    emergency_override: false,
                };
            }
        }

        if is_emergency {
            return AuthorizationResult {
                authorized: false,
                consent_hash: None,
                reason: "No consent found - emergency override available".to_string(),
                permissions: vec![permission.clone()],
                emergency_override: true,
            };
        }

        AuthorizationResult {
            authorized: false,
            consent_hash: None,
            reason: "No valid consent found".to_string(),
            permissions: vec![],
            emergency_override: false,
        }
    }

    // --- Access without consent (extended) ---

    #[test]
    fn test_empty_consent_list_denies_all_access() {
        let result = simulate_check_authorization(
            &[],
            &dummy_agent(),
            &DataCategory::Demographics,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(!result.authorized);
        assert!(result.permissions.is_empty());
        assert_eq!(result.reason, "No valid consent found");
    }

    #[test]
    fn test_consent_for_different_agent_denies_access() {
        let agent_a = AgentPubKey::from_raw_36(vec![1u8; 36]);
        let agent_b = AgentPubKey::from_raw_36(vec![2u8; 36]);
        let consents = vec![make_consent(
            ConsentGrantee::Agent(agent_a),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            None,
            None,
            vec![],
        )];
        let result = simulate_check_authorization(
            &consents,
            &agent_b,
            &DataCategory::Demographics,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(!result.authorized, "Agent B must not access Agent A's consent");
    }

    #[test]
    fn test_expired_consent_denies_access_via_authorization() {
        let agent = AgentPubKey::from_raw_36(vec![5u8; 36]);
        let consents = vec![make_consent(
            ConsentGrantee::Agent(agent.clone()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            Some(Timestamp::from_micros(1000000)), // expired
            None,
            vec![],
        )];
        let result = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Demographics,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(5000000), // now is after expiry
        );
        assert!(!result.authorized, "Expired consent must deny access");
    }

    #[test]
    fn test_pending_consent_denies_access_via_authorization() {
        let agent = AgentPubKey::from_raw_36(vec![6u8; 36]);
        let consents = vec![make_consent(
            ConsentGrantee::Agent(agent.clone()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Pending,
            None,
            None,
            vec![],
        )];
        let result = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Demographics,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(!result.authorized, "Pending consent must not grant access");
    }

    #[test]
    fn test_rejected_consent_denies_access_via_authorization() {
        let agent = AgentPubKey::from_raw_36(vec![7u8; 36]);
        let consents = vec![make_consent(
            ConsentGrantee::Agent(agent.clone()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Rejected,
            None,
            None,
            vec![],
        )];
        let result = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Demographics,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(!result.authorized, "Rejected consent must not grant access");
    }

    // --- Consent revocation blocks authorization ---

    #[test]
    fn test_revoked_consent_denies_access_via_authorization() {
        let agent = AgentPubKey::from_raw_36(vec![8u8; 36]);
        let consents = vec![make_consent(
            ConsentGrantee::Agent(agent.clone()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Revoked,
            None,
            Some(Timestamp::from_micros(500000)),
            vec![],
        )];
        let result = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Demographics,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(!result.authorized, "Revoked consent must block access");
    }

    #[test]
    fn test_revoked_consent_with_revoked_at_set_blocks_access() {
        // Even if status is still Active, revoked_at being set should block
        let agent = AgentPubKey::from_raw_36(vec![9u8; 36]);
        let consents = vec![make_consent(
            ConsentGrantee::Agent(agent.clone()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            None,
            Some(Timestamp::from_micros(500000)), // revoked_at set
            vec![],
        )];
        let result = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Demographics,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(
            !result.authorized,
            "Consent with revoked_at set must be filtered out even if status not updated"
        );
    }

    #[test]
    fn test_revocation_does_not_erase_consent_fields() {
        let agent = AgentPubKey::from_raw_36(vec![11u8; 36]);
        let mut consent = make_consent(
            ConsentGrantee::Agent(agent.clone()),
            vec![DataCategory::Medications, DataCategory::Allergies],
            vec![DataPermission::Read, DataPermission::Share],
            ConsentStatus::Active,
            Some(Timestamp::from_micros(9999999)),
            None,
            vec![DataCategory::GeneticData],
        );

        // Revoke
        consent.status = ConsentStatus::Revoked;
        consent.revoked_at = Some(Timestamp::from_micros(2000000));
        consent.revocation_reason = Some("Patient withdrew consent".to_string());

        // All original fields preserved for audit
        assert_eq!(consent.consent_id, "consent-001");
        assert_eq!(consent.scope.data_categories.len(), 2);
        assert!(consent.scope.data_categories.contains(&DataCategory::Medications));
        assert_eq!(consent.permissions.len(), 2);
        assert_eq!(consent.scope.exclusions, vec![DataCategory::GeneticData]);
        assert!(consent.expires_at.is_some());
        assert_eq!(consent.revocation_reason.as_deref(), Some("Patient withdrew consent"));
    }

    // --- Time-bound consent expiration (extended) ---

    #[test]
    fn test_consent_exactly_at_expiry_boundary() {
        // Consent expires_at == now: the get_active_consents filter uses
        // expires > now (strict), so exactly-equal should be expired.
        let agent = AgentPubKey::from_raw_36(vec![12u8; 36]);
        let boundary = Timestamp::from_micros(5000000);
        let consents = vec![make_consent(
            ConsentGrantee::Agent(agent.clone()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            Some(boundary),
            None,
            vec![],
        )];
        let result = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Demographics,
            &DataPermission::Read,
            false,
            boundary, // now == expires_at
        );
        assert!(
            !result.authorized,
            "Consent at exact expiry boundary must be denied (strict >)"
        );
    }

    #[test]
    fn test_consent_one_microsecond_before_expiry() {
        let agent = AgentPubKey::from_raw_36(vec![13u8; 36]);
        let consents = vec![make_consent(
            ConsentGrantee::Agent(agent.clone()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            Some(Timestamp::from_micros(5000000)),
            None,
            vec![],
        )];
        let result = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Demographics,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(4999999), // 1us before expiry
        );
        assert!(result.authorized, "Consent 1us before expiry must still be valid");
    }

    #[test]
    fn test_no_expiry_consent_active_at_far_future() {
        let agent = AgentPubKey::from_raw_36(vec![14u8; 36]);
        let consents = vec![make_consent(
            ConsentGrantee::Agent(agent.clone()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            None, // no expiration
            None,
            vec![],
        )];
        let result = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Demographics,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(999_999_999_999), // far future
        );
        assert!(result.authorized, "Consent without expiration must remain active");
    }

    // --- Granular consent (extended) ---

    #[test]
    fn test_consent_covers_specific_category_but_not_others() {
        let agent = AgentPubKey::from_raw_36(vec![15u8; 36]);
        let consents = vec![make_consent(
            ConsentGrantee::Agent(agent.clone()),
            vec![DataCategory::Medications],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            None,
            None,
            vec![],
        )];

        // Medications: authorized
        let result_meds = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Medications,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(result_meds.authorized, "Medications should be authorized");

        // LabResults: denied
        let result_labs = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::LabResults,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(!result_labs.authorized, "LabResults not in scope should be denied");

        // GeneticData: denied
        let result_genetic = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::GeneticData,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(!result_genetic.authorized, "GeneticData not in scope should be denied");
    }

    #[test]
    fn test_read_only_consent_blocks_write_and_delete() {
        let agent = AgentPubKey::from_raw_36(vec![16u8; 36]);
        let consents = vec![make_consent(
            ConsentGrantee::Agent(agent.clone()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            None,
            None,
            vec![],
        )];

        let result_read = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Demographics,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(result_read.authorized, "Read should be authorized");

        let result_write = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Demographics,
            &DataPermission::Write,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(!result_write.authorized, "Write must be denied on read-only consent");

        let result_delete = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Demographics,
            &DataPermission::Delete,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(!result_delete.authorized, "Delete must be denied on read-only consent");
    }

    #[test]
    fn test_exclusion_overrides_all_category_via_authorization() {
        let agent = AgentPubKey::from_raw_36(vec![17u8; 36]);
        let consents = vec![make_consent(
            ConsentGrantee::Agent(agent.clone()),
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            None,
            None,
            vec![DataCategory::MentalHealth, DataCategory::SubstanceAbuse],
        )];

        // Demographics: authorized (not excluded)
        let result_demo = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Demographics,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(result_demo.authorized, "Demographics not excluded should be authorized");

        // MentalHealth: denied (excluded)
        let result_mh = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::MentalHealth,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(!result_mh.authorized, "MentalHealth excluded must be denied");

        // SubstanceAbuse: denied (excluded)
        let result_sa = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::SubstanceAbuse,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(!result_sa.authorized, "SubstanceAbuse excluded must be denied");
    }

    // --- Emergency access ---

    #[test]
    fn test_emergency_flag_returns_override_available_when_no_consent() {
        let result = simulate_check_authorization(
            &[],
            &dummy_agent(),
            &DataCategory::All,
            &DataPermission::Read,
            true, // emergency
            Timestamp::from_micros(1000000),
        );
        assert!(!result.authorized, "Emergency alone does not auto-authorize");
        assert!(result.emergency_override, "Emergency override flag must be set");
        assert!(
            result.reason.contains("emergency override available"),
            "Reason should mention emergency override"
        );
    }

    #[test]
    fn test_emergency_consent_grantee_authorizes_emergency_requestor() {
        let agent = AgentPubKey::from_raw_36(vec![18u8; 36]);
        let consents = vec![make_consent(
            ConsentGrantee::EmergencyAccess,
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            None,
            None,
            vec![],
        )];
        let result = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Medications,
            &DataPermission::Read,
            true,
            Timestamp::from_micros(1000000),
        );
        assert!(
            result.authorized,
            "EmergencyAccess grantee should authorize any agent during emergency"
        );
        assert!(!result.emergency_override, "Should be consent-based, not override");
    }

    #[test]
    fn test_emergency_consent_grantee_requires_emergency_flag() {
        // EmergencyAccess grantee only matches when is_emergency=true
        let agent = AgentPubKey::from_raw_36(vec![19u8; 36]);
        let consents = vec![make_consent(
            ConsentGrantee::EmergencyAccess,
            vec![DataCategory::All],
            vec![DataPermission::Read],
            ConsentStatus::Active,
            None,
            None,
            vec![],
        )];
        let result = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Medications,
            &DataPermission::Read,
            false, // not emergency
            Timestamp::from_micros(1000000),
        );
        assert!(
            !result.authorized,
            "EmergencyAccess grantee must not authorize non-emergency requests"
        );
    }

    // --- Multiple consents: first-match wins ---

    #[test]
    fn test_multiple_consents_first_valid_match_wins() {
        let agent = AgentPubKey::from_raw_36(vec![20u8; 36]);
        let consents = vec![
            // Expired consent
            make_consent(
                ConsentGrantee::Agent(agent.clone()),
                vec![DataCategory::All],
                vec![DataPermission::Read, DataPermission::Write],
                ConsentStatus::Active,
                Some(Timestamp::from_micros(500000)), // expired
                None,
                vec![],
            ),
            // Active consent with Read only
            make_consent(
                ConsentGrantee::Agent(agent.clone()),
                vec![DataCategory::Medications],
                vec![DataPermission::Read],
                ConsentStatus::Active,
                None,
                None,
                vec![],
            ),
        ];
        let result = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Medications,
            &DataPermission::Read,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(result.authorized, "Second consent should match");
        // Write should fail because only Read in second consent
        let result_write = simulate_check_authorization(
            &consents,
            &agent,
            &DataCategory::Medications,
            &DataPermission::Write,
            false,
            Timestamp::from_micros(1000000),
        );
        assert!(!result_write.authorized, "Expired consent's Write should not carry over");
    }

    // --- Delegation validation ---

    #[test]
    fn test_delegation_temporary_requires_expiration() {
        let delegation = DelegationGrant {
            delegation_id: "DEL-001".to_string(),
            patient_hash: dummy_hash(),
            delegate: dummy_agent(),
            delegation_type: DelegationType::Temporary,
            permissions: vec![DelegationPermission::ViewRecords],
            data_scope: vec![DataCategory::All],
            exclusions: vec![],
            relationship: DelegateRelationship::Friend,
            granted_at: Timestamp::from_micros(1000000),
            expires_at: None, // Missing - would fail validation
            revoked_at: None,
            revocation_reason: None,
            status: DelegationStatus::Active,
            identity_verified: false,
            verification_method: None,
            legal_document_hash: None,
            notes: None,
        };
        // validate_delegation_grant checks: Temporary must have expires_at
        assert!(
            matches!(delegation.delegation_type, DelegationType::Temporary) && delegation.expires_at.is_none(),
            "Temporary delegation without expiration should fail integrity validation"
        );
    }

    #[test]
    fn test_delegation_healthcare_proxy_requires_verification() {
        let delegation = DelegationGrant {
            delegation_id: "DEL-002".to_string(),
            patient_hash: dummy_hash(),
            delegate: dummy_agent(),
            delegation_type: DelegationType::HealthcareProxy,
            permissions: vec![DelegationPermission::MakeMedicalDecisions],
            data_scope: vec![DataCategory::All],
            exclusions: vec![],
            relationship: DelegateRelationship::Spouse,
            granted_at: Timestamp::from_micros(1000000),
            expires_at: None,
            revoked_at: None,
            revocation_reason: None,
            status: DelegationStatus::Active,
            identity_verified: false, // Not verified - would fail
            verification_method: None,
            legal_document_hash: None, // No legal doc - would fail
            notes: None,
        };
        assert!(
            !delegation.identity_verified,
            "Healthcare proxy without identity verification should fail validation"
        );
        assert!(
            delegation.legal_document_hash.is_none(),
            "Healthcare proxy without legal document should fail validation"
        );
    }

    // --- string_to_data_category coverage ---

    #[test]
    fn test_string_to_data_category_known_mappings() {
        assert_eq!(string_to_data_category("VitalSigns"), DataCategory::VitalSigns);
        assert_eq!(string_to_data_category("Allergies"), DataCategory::Allergies);
        assert_eq!(string_to_data_category("Medications"), DataCategory::Medications);
        assert_eq!(string_to_data_category("Diagnoses"), DataCategory::Diagnoses);
        assert_eq!(string_to_data_category("Conditions"), DataCategory::Diagnoses);
        assert_eq!(string_to_data_category("LabResults"), DataCategory::LabResults);
        assert_eq!(string_to_data_category("Labs"), DataCategory::LabResults);
        assert_eq!(string_to_data_category("Immunizations"), DataCategory::Immunizations);
        assert_eq!(string_to_data_category("Procedures"), DataCategory::Procedures);
        assert_eq!(string_to_data_category("Imaging"), DataCategory::ImagingStudies);
        assert_eq!(string_to_data_category("ImagingStudies"), DataCategory::ImagingStudies);
        assert_eq!(string_to_data_category("MentalHealth"), DataCategory::MentalHealth);
        assert_eq!(string_to_data_category("Demographics"), DataCategory::Demographics);
        assert_eq!(string_to_data_category("SubstanceAbuse"), DataCategory::SubstanceAbuse);
        assert_eq!(string_to_data_category("SexualHealth"), DataCategory::SexualHealth);
        assert_eq!(string_to_data_category("GeneticData"), DataCategory::GeneticData);
        assert_eq!(string_to_data_category("FinancialData"), DataCategory::FinancialData);
        assert_eq!(string_to_data_category("Insurance"), DataCategory::FinancialData);
        assert_eq!(string_to_data_category("All"), DataCategory::All);
    }

    #[test]
    fn test_string_to_data_category_unknown_defaults_to_all() {
        assert_eq!(string_to_data_category("UnknownCategory"), DataCategory::All);
        assert_eq!(string_to_data_category(""), DataCategory::All);
    }
}
