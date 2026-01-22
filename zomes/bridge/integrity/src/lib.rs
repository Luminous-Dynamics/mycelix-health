//! Mycelix Bridge for Cross-hApp Health Data Federation Integrity Zome
//! 
//! Defines entry types for cross-hApp communication, reputation federation,
//! and ecosystem integration following the Mycelix bridge protocol.

use hdi::prelude::*;

/// Bridge registration for health data federation
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HealthBridgeRegistration {
    pub registration_id: String,
    /// The Mycelix identity hash
    pub mycelix_identity_hash: ActionHash,
    /// hApp this registration is for
    pub happ_id: String,
    /// Capabilities being offered
    pub capabilities: Vec<HealthCapability>,
    /// Data categories available for federation
    pub federated_data: Vec<FederatedDataType>,
    /// Trust requirements for data sharing
    pub minimum_trust_score: f64,
    /// Registration timestamp
    pub registered_at: Timestamp,
    /// Status
    pub status: BridgeStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum HealthCapability {
    PatientLookup,
    ProviderVerification,
    RecordSharing,
    ConsentVerification,
    ClaimsSubmission,
    TrialEnrollment,
    EpistemicClaims,
    ReputationFederation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum FederatedDataType {
    Demographics,
    MedicalHistory,
    Medications,
    Allergies,
    LabResults,
    Immunizations,
    ProviderCredentials,
    InsuranceCoverage,
    ConsentRecords,
    ClinicalTrialEligibility,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BridgeStatus {
    Active,
    Suspended,
    Revoked,
    Pending,
}

/// Cross-hApp health data query
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HealthDataQuery {
    pub query_id: String,
    pub requesting_agent: AgentPubKey,
    pub requesting_happ: String,
    /// Patient identifier (from Mycelix identity)
    pub patient_identity_hash: ActionHash,
    /// What data is being requested
    pub data_types: Vec<FederatedDataType>,
    /// Purpose of the query
    pub purpose: QueryPurpose,
    /// Consent hash authorizing this query
    pub consent_hash: ActionHash,
    /// Query timestamp
    pub queried_at: Timestamp,
    /// Response status
    pub status: QueryStatus,
    /// Response data hash (if completed)
    pub response_hash: Option<ActionHash>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum QueryPurpose {
    Treatment,
    EmergencyCare,
    Referral,
    Research,
    Insurance,
    PublicHealth,
    QualityReporting,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum QueryStatus {
    Pending,
    Authorized,
    InProgress,
    Completed,
    Denied,
    Expired,
    Failed,
}

/// Health data query response
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HealthDataResponse {
    pub response_id: String,
    pub query_hash: ActionHash,
    pub responding_agent: AgentPubKey,
    pub responding_happ: String,
    /// Data being returned (encrypted reference)
    pub data_reference: EntryHash,
    /// MATL trust score of the data source
    pub source_trust_score: f64,
    /// Epistemic classification of the data
    pub epistemic_level: EpistemicLevel,
    /// Response timestamp
    pub responded_at: Timestamp,
    /// Data freshness (when was source data last updated)
    pub data_timestamp: Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EpistemicLevel {
    E0Unverified,
    E1Verified,
    E2Replicated,
    E3Consensus,
}

/// Provider credential verification request (cross-hApp)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ProviderVerificationRequest {
    pub request_id: String,
    pub requesting_agent: AgentPubKey,
    pub requesting_happ: String,
    /// Provider to verify
    pub provider_hash: ActionHash,
    /// What credentials to verify
    pub verification_types: Vec<CredentialVerificationType>,
    /// Request timestamp
    pub requested_at: Timestamp,
    /// Verification result
    pub status: VerificationStatus,
    pub result_hash: Option<ActionHash>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CredentialVerificationType {
    License,
    BoardCertification,
    DEA,
    NPI,
    HospitalPrivileges,
    MalpracticeHistory,
    SanctionsCheck,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum VerificationStatus {
    Pending,
    Verified,
    Failed,
    PartiallyVerified,
    Expired,
}

/// Provider verification result
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ProviderVerificationResult {
    pub result_id: String,
    pub request_hash: ActionHash,
    pub provider_hash: ActionHash,
    /// Verification results per type
    pub verifications: Vec<CredentialVerification>,
    /// Overall trust score
    pub composite_trust_score: f64,
    /// Timestamp of verification
    pub verified_at: Timestamp,
    /// Valid until
    pub valid_until: Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CredentialVerification {
    pub credential_type: CredentialVerificationType,
    pub verified: bool,
    pub verification_source: String,
    pub details: Option<String>,
    pub trust_contribution: f64,
}

/// Health epistemic claim for federation
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HealthEpistemicClaim {
    pub claim_id: String,
    pub claimant: AgentPubKey,
    pub subject_hash: ActionHash,
    /// The health-related claim
    pub claim_type: HealthClaimType,
    pub claim_content: String,
    /// Evidence supporting the claim
    pub evidence_hashes: Vec<ActionHash>,
    /// Epistemic classification
    pub classification: EpistemicClassification,
    /// MATL trust score
    pub matl_score: f64,
    /// Creation timestamp
    pub created_at: Timestamp,
    /// Verification status
    pub verified: bool,
    pub verified_by: Vec<AgentPubKey>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum HealthClaimType {
    Diagnosis,
    Treatment,
    Outcome,
    ProviderCompetency,
    FacilityQuality,
    MedicationEfficacy,
    AdverseEvent,
    ResearchFinding,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EpistemicClassification {
    /// Empirical level (E0-E3)
    pub empirical_level: u8,
    /// Materiality level (M0-M3)
    pub materiality_level: u8,
    /// Normative level (N0-N3)
    pub normative_level: u8,
}

/// Reputation federation record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HealthReputationFederation {
    pub federation_id: String,
    pub entity_hash: ActionHash,
    pub entity_type: HealthEntityType,
    /// Reputation scores from different hApps
    pub scores: Vec<FederatedScore>,
    /// Aggregated score
    pub aggregated_score: f64,
    /// Last aggregation timestamp
    pub aggregated_at: Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum HealthEntityType {
    Patient,
    Provider,
    Facility,
    Pharmacy,
    Insurer,
    Researcher,
    Trial,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FederatedScore {
    pub source_happ: String,
    pub score: f64,
    pub weight: f64,
    pub score_type: String,
    pub timestamp: Timestamp,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    HealthBridgeRegistration(HealthBridgeRegistration),
    HealthDataQuery(HealthDataQuery),
    HealthDataResponse(HealthDataResponse),
    ProviderVerificationRequest(ProviderVerificationRequest),
    ProviderVerificationResult(ProviderVerificationResult),
    HealthEpistemicClaim(HealthEpistemicClaim),
    HealthReputationFederation(HealthReputationFederation),
}

#[hdk_link_types]
pub enum LinkTypes {
    IdentityToRegistrations,
    QueryToResponses,
    ProviderToVerifications,
    EntityToClaims,
    EntityToReputation,
    PendingQueries,
    ActiveRegistrations,
    ClaimsByType,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::HealthBridgeRegistration(r) => validate_registration(&r),
                EntryTypes::HealthDataQuery(q) => validate_query(&q),
                EntryTypes::HealthDataResponse(r) => validate_response(&r),
                EntryTypes::ProviderVerificationRequest(r) => validate_verification_request(&r),
                EntryTypes::ProviderVerificationResult(r) => validate_verification_result(&r),
                EntryTypes::HealthEpistemicClaim(c) => validate_claim(&c),
                EntryTypes::HealthReputationFederation(f) => validate_federation(&f),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_registration(reg: &HealthBridgeRegistration) -> ExternResult<ValidateCallbackResult> {
    if reg.registration_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Registration ID is required".to_string(),
        ));
    }
    if reg.happ_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "hApp ID is required".to_string(),
        ));
    }
    if reg.minimum_trust_score < 0.0 || reg.minimum_trust_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Trust score must be between 0.0 and 1.0".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_query(query: &HealthDataQuery) -> ExternResult<ValidateCallbackResult> {
    if query.query_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Query ID is required".to_string(),
        ));
    }
    if query.data_types.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one data type must be requested".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_response(response: &HealthDataResponse) -> ExternResult<ValidateCallbackResult> {
    if response.response_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Response ID is required".to_string(),
        ));
    }
    if response.source_trust_score < 0.0 || response.source_trust_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Trust score must be between 0.0 and 1.0".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_verification_request(req: &ProviderVerificationRequest) -> ExternResult<ValidateCallbackResult> {
    if req.request_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Request ID is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_verification_result(result: &ProviderVerificationResult) -> ExternResult<ValidateCallbackResult> {
    if result.result_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Result ID is required".to_string(),
        ));
    }
    if result.composite_trust_score < 0.0 || result.composite_trust_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Trust score must be between 0.0 and 1.0".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_claim(claim: &HealthEpistemicClaim) -> ExternResult<ValidateCallbackResult> {
    if claim.claim_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Claim ID is required".to_string(),
        ));
    }
    if claim.matl_score < 0.0 || claim.matl_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "MATL score must be between 0.0 and 1.0".to_string(),
        ));
    }
    // Validate epistemic classification ranges
    if claim.classification.empirical_level > 3 
        || claim.classification.materiality_level > 3 
        || claim.classification.normative_level > 3 {
        return Ok(ValidateCallbackResult::Invalid(
            "Epistemic levels must be 0-3".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_federation(fed: &HealthReputationFederation) -> ExternResult<ValidateCallbackResult> {
    if fed.federation_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Federation ID is required".to_string(),
        ));
    }
    if fed.aggregated_score < 0.0 || fed.aggregated_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Aggregated score must be between 0.0 and 1.0".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
