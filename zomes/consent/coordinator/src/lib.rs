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
    
    let active: Vec<Record> = all_consents
        .into_iter()
        .filter(|record| {
            if let Some(consent) = record.entry().to_app_option::<Consent>().ok().flatten() {
                matches!(consent.status, ConsentStatus::Active)
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
            // Check if grantee matches
            let grantee_matches = match &consent.grantee {
                ConsentGrantee::Agent(agent) => *agent == input.requestor,
                ConsentGrantee::EmergencyAccess => input.is_emergency,
                _ => false,
            };

            if grantee_matches {
                // Check if data category is covered
                let category_covered = consent.scope.data_categories.iter().any(|cat| {
                    matches!(cat, DataCategory::All) || *cat == input.data_category
                });

                // Check if not excluded
                let not_excluded = !consent.scope.exclusions.contains(&input.data_category);

                // Check if permission is granted
                let permission_granted = consent.permissions.contains(&input.permission);

                if category_covered && not_excluded && permission_granted {
                    return Ok(AuthorizationResult {
                        authorized: true,
                        consent_hash: Some(record.action_address().clone()),
                        reason: "Active consent found".to_string(),
                        permissions: consent.permissions.clone(),
                        emergency_override: false,
                    });
                }
            }
        }
    }

    // Check if emergency access without consent
    if input.is_emergency {
        return Ok(AuthorizationResult {
            authorized: false,
            consent_hash: None,
            reason: "No consent found - emergency override available".to_string(),
            permissions: vec![input.permission],
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
