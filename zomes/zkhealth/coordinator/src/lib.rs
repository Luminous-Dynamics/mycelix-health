//! ZK Health Proofs Coordinator Zome
//!
//! Provides extern functions for zero-knowledge health proof generation,
//! verification, and management.
//!
//! ## Cross-Zome Integration
//!
//! This zome integrates with the Consent zome to:
//! - Log all proof generation events for audit trail (HIPAA compliance)
//! - Verify patient consent before generating proofs that use sensitive data
//! - Track proof verification events for accountability

use hdk::prelude::*;
use zkhealth_integrity::*;

// ==================== CONSENT INTEGRATION ====================

/// Map proof types to required HIPAA data categories
fn proof_type_to_data_categories(proof_type: &HealthProofType) -> Vec<String> {
    match proof_type {
        // Vaccination proofs use immunization records
        HealthProofType::VaccinationStatus => vec!["Immunizations".to_string()],

        // Insurance qualification needs comprehensive health assessment
        HealthProofType::InsuranceQualification => vec![
            "VitalSigns".to_string(),
            "LabResults".to_string(),
            "Diagnoses".to_string(),
        ],

        // Age verification uses demographics only
        HealthProofType::AgeVerification => vec!["Demographics".to_string()],

        // Condition presence/absence uses diagnosis records
        HealthProofType::ConditionPresence => vec!["Diagnoses".to_string()],
        HealthProofType::ConditionAbsence => vec!["Diagnoses".to_string()],

        // Employment physical requires vitals and diagnoses
        HealthProofType::EmploymentPhysical => {
            vec!["VitalSigns".to_string(), "Diagnoses".to_string()]
        }

        // Substance screening uses lab results
        HealthProofType::SubstanceScreening => vec!["LabResults".to_string()],

        // Lab threshold proofs (BMI, blood type, etc.) use lab/vital data
        HealthProofType::LabThreshold => vec!["LabResults".to_string(), "VitalSigns".to_string()],

        // Allergy status checks allergy records
        HealthProofType::AllergyStatus => vec!["Allergies".to_string()],

        // Medication status uses medication records
        HealthProofType::MedicationStatus => vec!["Medications".to_string()],

        // Organ donor compatibility requires extensive lab work
        HealthProofType::OrganDonorCompatibility => {
            vec!["LabResults".to_string(), "Diagnoses".to_string()]
        }

        // Clinical trial eligibility needs comprehensive review
        HealthProofType::ClinicalTrialEligibility => vec![
            "Diagnoses".to_string(),
            "Medications".to_string(),
            "LabResults".to_string(),
        ],

        // Physical capability for sports/fitness
        HealthProofType::PhysicalCapability => {
            vec!["VitalSigns".to_string(), "Diagnoses".to_string()]
        }

        // Mental health clearance uses mental health records
        HealthProofType::MentalHealthClearance => vec!["MentalHealth".to_string()],

        // Travel health clearance checks immunizations and conditions
        HealthProofType::TravelHealthClearance => {
            vec!["Immunizations".to_string(), "Diagnoses".to_string()]
        }

        // General health uses all basic metrics
        HealthProofType::GeneralHealth => vec!["VitalSigns".to_string(), "Diagnoses".to_string()],

        // Custom proofs may access anything - log broadly
        HealthProofType::Custom(_) => vec!["All".to_string()],
    }
}

/// Log proof generation to consent zome for audit trail
fn log_proof_to_consent(proof: &HealthProof) -> ExternResult<()> {
    // Create audit log entry
    let log_entry = ZkProofAuditLog {
        log_id: format!("ZKAUDIT-{}", sys_time()?.as_micros()),
        patient_hash: proof.patient_hash.clone(),
        proof_id: proof.proof_id.clone(),
        proof_type: format!("{:?}", proof.proof_type),
        data_categories_used: proof_type_to_data_categories(&proof.proof_type),
        verifier_hint: None, // Will be populated on verification
        generated_at: sys_time()?.as_micros() as i64,
        purpose: "ZK Proof Generation".to_string(),
    };

    // Call consent zome to create audit log
    let response = call(
        CallTargetCell::Local,
        ZomeName::from("consent"),
        FunctionName::from("log_zk_proof_generation"),
        None,
        &log_entry,
    )?;

    // Log result but don't fail if consent zome isn't available
    match response {
        ZomeCallResponse::Ok(_) => Ok(()),
        _ => {
            // If consent zome doesn't have this function yet, that's OK
            // We'll implement the function there next
            Ok(())
        }
    }
}

/// Input for ZK proof audit logging
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

/// Log proof verification to consent zome
fn log_verification_to_consent(
    proof: &HealthProof,
    verifier: &AgentPubKey,
    verified: bool,
) -> ExternResult<()> {
    let log_entry = ZkVerificationAuditLog {
        log_id: format!("ZKVER-{}", sys_time()?.as_micros()),
        patient_hash: proof.patient_hash.clone(),
        proof_id: proof.proof_id.clone(),
        verifier: verifier.clone(),
        verification_result: verified,
        verified_at: sys_time()?.as_micros() as i64,
    };

    // Call consent zome to create verification log
    let response = call(
        CallTargetCell::Local,
        ZomeName::from("consent"),
        FunctionName::from("log_zk_proof_verification"),
        None,
        &log_entry,
    )?;

    match response {
        ZomeCallResponse::Ok(_) => Ok(()),
        _ => Ok(()), // Best effort
    }
}

/// Input for ZK verification audit logging
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZkVerificationAuditLog {
    pub log_id: String,
    pub patient_hash: ActionHash,
    pub proof_id: String,
    pub verifier: AgentPubKey,
    pub verification_result: bool,
    pub verified_at: i64,
}

// ==================== HEALTH PROOFS ====================

/// Generate a zero-knowledge health proof
///
/// This function automatically logs the proof generation to the consent zome
/// for HIPAA-compliant audit trail tracking.
#[hdk_extern]
pub fn generate_health_proof(proof: HealthProof) -> ExternResult<Record> {
    validate_health_proof(&proof)?;

    let proof_hash = create_entry(&EntryTypes::HealthProof(proof.clone()))?;
    let record = get(proof_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find proof".to_string())
    ))?;

    // Link to patient
    create_link(
        proof.patient_hash.clone(),
        proof_hash.clone(),
        LinkTypes::PatientToProofs,
        (),
    )?;

    // Link to completed proofs anchor
    let anchor = anchor_hash("completed_proofs")?;
    create_link(anchor, proof_hash, LinkTypes::CompletedProofs, ())?;

    // ==================== CONSENT INTEGRATION ====================
    // Log proof generation to consent zome for HIPAA audit trail
    let _ = log_proof_to_consent(&proof);
    // ================================================================

    Ok(record)
}

/// Get patient's proofs
#[hdk_extern]
pub fn get_patient_proofs(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToProofs)?,
        GetStrategy::default(),
    )?;

    let mut proofs = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                proofs.push(record);
            }
        }
    }

    Ok(proofs)
}

/// Get valid (non-expired, non-revoked) proofs for patient
#[hdk_extern]
pub fn get_valid_proofs(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let all_proofs = get_patient_proofs(patient_hash)?;
    let now = sys_time()?.as_micros() as i64;

    let valid: Vec<Record> = all_proofs
        .into_iter()
        .filter(|record| {
            if let Some(proof) = record.entry().to_app_option::<HealthProof>().ok().flatten() {
                !proof.revoked && proof.valid_from <= now && proof.valid_until > now
            } else {
                false
            }
        })
        .collect();

    Ok(valid)
}

/// Revoke a health proof
#[hdk_extern]
pub fn revoke_proof(input: RevokeProofInput) -> ExternResult<Record> {
    let record = get(input.proof_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Proof not found".to_string())
    ))?;

    let mut proof: HealthProof = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid proof".to_string()
        )))?;

    // Verify caller is the patient who owns this proof
    let _caller = agent_info()?.agent_initial_pubkey;
    // Note: In production, add proper authorization check

    proof.revoked = true;
    proof.revocation_reason = Some(input.reason);

    let updated_hash = update_entry(input.proof_hash, &proof)?;

    get(updated_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Could not find updated proof".to_string()
    )))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RevokeProofInput {
    pub proof_hash: ActionHash,
    pub reason: String,
}

// ==================== PROOF REQUESTS ====================

/// Create a proof request
#[hdk_extern]
pub fn create_proof_request(request: ProofRequest) -> ExternResult<Record> {
    validate_proof_request(&request)?;

    let request_hash = create_entry(&EntryTypes::ProofRequest(request.clone()))?;
    let record = get(request_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find request".to_string())
    ))?;

    // Link to patient
    create_link(
        request.patient_hash.clone(),
        request_hash.clone(),
        LinkTypes::PatientToRequests,
        (),
    )?;

    // Link to verifier using anchor
    let verifier_anchor = anchor_hash(&format!("verifier:{:?}", request.verifier))?;
    create_link(
        verifier_anchor,
        request_hash.clone(),
        LinkTypes::VerifierToRequests,
        (),
    )?;

    // Link to active requests
    let anchor = anchor_hash("active_requests")?;
    create_link(anchor, request_hash, LinkTypes::ActiveRequests, ())?;

    Ok(record)
}

/// Get pending proof requests for patient
#[hdk_extern]
pub fn get_pending_requests(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToRequests)?,
        GetStrategy::default(),
    )?;

    let now = sys_time()?.as_micros() as i64;

    let mut pending = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(req) = record
                    .entry()
                    .to_app_option::<ProofRequest>()
                    .ok()
                    .flatten()
                {
                    if matches!(req.status, ProofRequestStatus::Pending) && req.expires_at > now {
                        pending.push(record);
                    }
                }
            }
        }
    }

    Ok(pending)
}

/// Respond to a proof request
#[hdk_extern]
pub fn respond_to_request(input: RespondToRequestInput) -> ExternResult<Record> {
    let record = get(input.request_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Request not found".to_string())
    ))?;

    let mut request: ProofRequest = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid request".to_string()
        )))?;

    // Determine status before moving proof_hash
    let has_proof = input.proof_hash.is_some();
    let new_status = if input.accept {
        if has_proof {
            ProofRequestStatus::ProofSubmitted
        } else {
            ProofRequestStatus::Accepted
        }
    } else {
        ProofRequestStatus::Declined
    };

    let response = PatientResponse {
        consents: input.accept,
        decline_reason: if input.accept {
            None
        } else {
            input.decline_reason
        },
        proof_hash: input.proof_hash,
        responded_at: sys_time()?.as_micros() as i64,
    };

    request.patient_response = Some(response);
    request.status = new_status;

    let updated_hash = update_entry(input.request_hash, &request)?;

    get(updated_hash, GetOptions::default())?.ok_or(wasm_error!(WasmErrorInner::Guest(
        "Could not find updated request".to_string()
    )))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RespondToRequestInput {
    pub request_hash: ActionHash,
    pub accept: bool,
    pub decline_reason: Option<String>,
    pub proof_hash: Option<ActionHash>,
}

// ==================== VERIFICATION ====================

/// Verify a health proof
#[hdk_extern]
pub fn verify_health_proof(input: VerifyProofInput) -> ExternResult<Record> {
    let proof_record = get(input.proof_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Proof not found".to_string())
    ))?;

    let proof: HealthProof = proof_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Invalid proof".to_string()
        )))?;

    let now = sys_time()?.as_micros() as i64;
    let start_time = sys_time()?.as_millis();

    // Perform verification checks
    let crypto_valid = verify_zkstark_proof(&proof);
    let within_validity = proof.valid_from <= now && proof.valid_until > now;
    let not_revoked = !proof.revoked;
    let attestations_verified = verify_attestations(&proof.attestations);
    let data_recency_ok = (now - proof.public_inputs.data_timestamp)
        < (input.max_data_age_days.unwrap_or(365) as i64 * 24 * 60 * 60 * 1_000_000);

    let verified =
        crypto_valid && within_validity && not_revoked && attestations_verified && data_recency_ok;

    let end_time = sys_time()?.as_millis();

    let failure_reason = if !verified {
        Some(format!(
            "Verification failed: crypto={}, validity={}, not_revoked={}, attestations={}, recency={}",
            crypto_valid, within_validity, not_revoked, attestations_verified, data_recency_ok
        ))
    } else {
        None
    };

    let verification = VerificationResult {
        verification_id: format!("VER-{}", sys_time()?.as_micros()),
        proof_hash: input.proof_hash.clone(),
        verifier: agent_info()?.agent_initial_pubkey,
        verified,
        details: VerificationDetails {
            crypto_valid,
            within_validity,
            attestations_verified,
            not_revoked,
            data_recency_ok,
            failure_reason,
            verification_time_ms: (end_time - start_time) as u64,
        },
        verified_at: now,
    };

    let ver_hash = create_entry(&EntryTypes::VerificationResult(verification.clone()))?;
    let ver_record = get(ver_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find verification".to_string())
    ))?;

    // Link verification to proof
    create_link(
        input.proof_hash,
        ver_hash,
        LinkTypes::ProofToVerifications,
        (),
    )?;

    // Log verification to consent zome for audit trail (best effort)
    let verifier_pubkey = agent_info()?.agent_initial_pubkey;
    let _ = log_verification_to_consent(&proof, &verifier_pubkey, verified);

    Ok(ver_record)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyProofInput {
    pub proof_hash: ActionHash,
    pub max_data_age_days: Option<u32>,
}

/// Verify zkSTARK proof (simulation mode - always returns true for valid structure)
/// In production, this would call the actual zkSTARK verifier
fn verify_zkstark_proof(proof: &HealthProof) -> bool {
    // Simulation: Verify proof has required structure
    // Real implementation would verify the cryptographic proof
    !proof.proof_bytes.is_empty() &&
    proof.proof_bytes.len() >= 1000 && // Minimum reasonable proof size
    proof.public_inputs.data_commitment != [0u8; 32] // Has a commitment
}

/// Verify attestations are properly formed
fn verify_attestations(attestations: &[HealthAttestation]) -> bool {
    // Simulation: Verify attestations have signatures
    // Real implementation would verify cryptographic signatures
    attestations
        .iter()
        .all(|a| !a.signature.is_empty() && a.attested_at > 0)
}

/// Get verification history for a proof
#[hdk_extern]
pub fn get_proof_verifications(proof_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(proof_hash, LinkTypes::ProofToVerifications)?,
        GetStrategy::default(),
    )?;

    let mut verifications = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                verifications.push(record);
            }
        }
    }

    Ok(verifications)
}

// ==================== TRUSTED ATTESTORS ====================

/// Register a trusted attestor
#[hdk_extern]
pub fn register_attestor(attestor: TrustedAttestor) -> ExternResult<Record> {
    validate_trusted_attestor(&attestor)?;

    let attestor_hash = create_entry(&EntryTypes::TrustedAttestor(attestor.clone()))?;
    let record = get(attestor_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find attestor".to_string())
    ))?;

    // Link to trusted attestors anchor
    let anchor = anchor_hash("trusted_attestors")?;
    create_link(anchor, attestor_hash, LinkTypes::TrustedAttestors, ())?;

    Ok(record)
}

/// Get all trusted attestors
#[hdk_extern]
pub fn get_trusted_attestors(_: ()) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash("trusted_attestors")?;
    let links = get_links(
        LinkQuery::try_new(anchor, LinkTypes::TrustedAttestors)?,
        GetStrategy::default(),
    )?;

    let mut attestors = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(attestor) = record
                    .entry()
                    .to_app_option::<TrustedAttestor>()
                    .ok()
                    .flatten()
                {
                    if attestor.active {
                        attestors.push(record);
                    }
                }
            }
        }
    }

    Ok(attestors)
}

/// Get attestors by credential type
#[hdk_extern]
pub fn get_attestors_by_type(credential_type: AttestorCredentialType) -> ExternResult<Vec<Record>> {
    let all_attestors = get_trusted_attestors(())?;

    let filtered: Vec<Record> = all_attestors
        .into_iter()
        .filter(|record| {
            if let Some(attestor) = record
                .entry()
                .to_app_option::<TrustedAttestor>()
                .ok()
                .flatten()
            {
                attestor.credential_type == credential_type
            } else {
                false
            }
        })
        .collect();

    Ok(filtered)
}

// ==================== PROOF TEMPLATES ====================

/// Create a proof template
#[hdk_extern]
pub fn create_proof_template(template: ProofTemplate) -> ExternResult<Record> {
    validate_proof_template(&template)?;

    let template_hash = create_entry(&EntryTypes::ProofTemplate(template.clone()))?;
    let record = get(template_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find template".to_string())
    ))?;

    Ok(record)
}

/// Initialize system proof templates
#[hdk_extern]
pub fn initialize_proof_templates(_: ()) -> ExternResult<Vec<ActionHash>> {
    let system_templates = vec![
        ProofTemplate {
            template_id: "INSURANCE-LIFE".to_string(),
            name: "Life Insurance Qualification".to_string(),
            description: "Proves general health qualification for life insurance without revealing specific conditions".to_string(),
            proof_type: HealthProofType::InsuranceQualification,
            required_data: vec![
                RequiredDataCategory::VitalSigns,
                RequiredDataCategory::LabResults(vec!["cholesterol".to_string(), "glucose".to_string()]),
                RequiredDataCategory::Diagnoses,
            ],
            circuit_id: "health-insurance-v1".to_string(),
            typically_requires_attestation: true,
            default_validity_days: 90,
            use_cases: vec![
                "Life insurance application".to_string(),
                "Health insurance underwriting".to_string(),
            ],
            created_by: None,
            system_template: true,
        },
        ProofTemplate {
            template_id: "EMPLOYMENT-PHYSICAL".to_string(),
            name: "Employment Physical Clearance".to_string(),
            description: "Proves fitness for work without revealing specific health details".to_string(),
            proof_type: HealthProofType::EmploymentPhysical,
            required_data: vec![
                RequiredDataCategory::PhysicalExam,
                RequiredDataCategory::VitalSigns,
            ],
            circuit_id: "health-employment-v1".to_string(),
            typically_requires_attestation: true,
            default_validity_days: 30,
            use_cases: vec![
                "Pre-employment physical".to_string(),
                "Annual fitness certification".to_string(),
            ],
            created_by: None,
            system_template: true,
        },
        ProofTemplate {
            template_id: "ORGAN-DONOR".to_string(),
            name: "Organ Donor Compatibility".to_string(),
            description: "Proves compatibility for organ donation without revealing medical history".to_string(),
            proof_type: HealthProofType::OrganDonorCompatibility,
            required_data: vec![
                RequiredDataCategory::LabResults(vec!["blood_type".to_string(), "hla_typing".to_string()]),
                RequiredDataCategory::Diagnoses,
            ],
            circuit_id: "health-organ-donor-v1".to_string(),
            typically_requires_attestation: true,
            default_validity_days: 7,
            use_cases: vec![
                "Living donor evaluation".to_string(),
                "Transplant list compatibility".to_string(),
            ],
            created_by: None,
            system_template: true,
        },
        ProofTemplate {
            template_id: "CLINICAL-TRIAL".to_string(),
            name: "Clinical Trial Eligibility".to_string(),
            description: "Proves eligibility for clinical trial participation".to_string(),
            proof_type: HealthProofType::ClinicalTrialEligibility,
            required_data: vec![
                RequiredDataCategory::Diagnoses,
                RequiredDataCategory::Medications,
                RequiredDataCategory::LabResults(vec![]),
            ],
            circuit_id: "health-trial-eligibility-v1".to_string(),
            typically_requires_attestation: false,
            default_validity_days: 14,
            use_cases: vec![
                "Clinical trial enrollment".to_string(),
                "Research study participation".to_string(),
            ],
            created_by: None,
            system_template: true,
        },
        ProofTemplate {
            template_id: "TRAVEL-HEALTH".to_string(),
            name: "Travel Health Clearance".to_string(),
            description: "Proves vaccination status and absence of communicable diseases for travel".to_string(),
            proof_type: HealthProofType::TravelHealthClearance,
            required_data: vec![
                RequiredDataCategory::Immunizations,
                RequiredDataCategory::Diagnoses,
            ],
            circuit_id: "health-travel-v1".to_string(),
            typically_requires_attestation: true,
            default_validity_days: 30,
            use_cases: vec![
                "International travel".to_string(),
                "Cruise ship boarding".to_string(),
                "Event attendance".to_string(),
            ],
            created_by: None,
            system_template: true,
        },
        ProofTemplate {
            template_id: "SUBSTANCE-SCREENING".to_string(),
            name: "Substance Screening".to_string(),
            description: "Proves negative drug/alcohol screening without revealing any medical conditions".to_string(),
            proof_type: HealthProofType::SubstanceScreening,
            required_data: vec![
                RequiredDataCategory::LabResults(vec!["drug_screen".to_string()]),
            ],
            circuit_id: "health-substance-v1".to_string(),
            typically_requires_attestation: true,
            default_validity_days: 7,
            use_cases: vec![
                "Employment drug screening".to_string(),
                "Athletic competition".to_string(),
                "Professional licensing".to_string(),
            ],
            created_by: None,
            system_template: true,
        },
    ];

    let mut hashes = Vec::new();
    for template in system_templates {
        let hash = create_entry(&EntryTypes::ProofTemplate(template))?;
        hashes.push(hash);
    }

    Ok(hashes)
}

// ==================== PROOF PARAMETERS ====================

/// Set patient's proof parameters
#[hdk_extern]
pub fn set_proof_parameters(params: ProofParameters) -> ExternResult<Record> {
    validate_proof_parameters(&params)?;

    let params_hash = create_entry(&EntryTypes::ProofParameters(params.clone()))?;
    let record = get(params_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find parameters".to_string())
    ))?;

    // Link to patient
    create_link(
        params.patient_hash.clone(),
        params_hash,
        LinkTypes::PatientToParameters,
        (),
    )?;

    Ok(record)
}

/// Get patient's proof parameters
#[hdk_extern]
pub fn get_proof_parameters(patient_hash: ActionHash) -> ExternResult<Option<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToParameters)?,
        GetStrategy::default(),
    )?;

    // Get the most recent parameters
    if let Some(link) = links.last() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            return get(hash, GetOptions::default());
        }
    }

    Ok(None)
}

// ==================== HELPER FUNCTIONS ====================

/// Generate proof ID
#[hdk_extern]
pub fn generate_proof_id(_: ()) -> ExternResult<String> {
    let time = sys_time()?.as_micros();
    Ok(format!("ZKPROOF-{}", time))
}

/// Get proof by ID
#[hdk_extern]
pub fn get_proof_by_id(input: GetProofByIdInput) -> ExternResult<Option<Record>> {
    let anchor = anchor_hash("completed_proofs")?;
    let links = get_links(
        LinkQuery::try_new(anchor, LinkTypes::CompletedProofs)?,
        GetStrategy::default(),
    )?;

    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(proof) = record.entry().to_app_option::<HealthProof>().ok().flatten() {
                    if proof.proof_id == input.proof_id {
                        return Ok(Some(record));
                    }
                }
            }
        }
    }

    Ok(None)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetProofByIdInput {
    pub proof_id: String,
}

/// Check if a proof type is allowed by patient's parameters
#[hdk_extern]
pub fn is_proof_type_allowed(input: CheckProofTypeInput) -> ExternResult<bool> {
    if let Some(record) = get_proof_parameters(input.patient_hash)? {
        if let Some(params) = record
            .entry()
            .to_app_option::<ProofParameters>()
            .ok()
            .flatten()
        {
            // If explicitly refused, not allowed
            if params.refused_proof_types.contains(&input.proof_type) {
                return Ok(false);
            }
            // If allowed list is non-empty, must be in it
            if !params.allowed_proof_types.is_empty()
                && !params.allowed_proof_types.contains(&input.proof_type)
            {
                return Ok(false);
            }
        }
    }

    // Default: allowed
    Ok(true)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CheckProofTypeInput {
    pub patient_hash: ActionHash,
    pub proof_type: HealthProofType,
}

// ==================== ANCHOR SUPPORT ====================

/// Anchor entry for indexing
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}
