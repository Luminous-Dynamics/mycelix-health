//! International Patient Summary (IPS) Coordinator Zome
//!
//! Provides functions for generating, sharing, and managing IPS documents
//! for cross-border healthcare information exchange.

use hdk::prelude::*;
use ips_integrity::*;

/// Input for generating an IPS document
#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateIpsInput {
    pub patient_hash: ActionHash,
    pub language: String,
    pub country_of_origin: String,
    pub author_organization: String,
    pub sections_to_include: Vec<IpsSection>,
    pub expires_in_days: Option<u32>,
}

/// Input for adding an allergy to IPS
#[derive(Serialize, Deserialize, Debug)]
pub struct AddAllergyInput {
    pub ips_hash: ActionHash,
    pub allergy_id: String,
    pub category: AllergyCategory,
    pub agent_code: String,
    pub coding_system: String,
    pub agent_display: String,
    pub severity: AllergySeverity,
    pub criticality: String,
    pub reactions: Vec<String>,
    pub onset_date: Option<Timestamp>,
    pub verification_status: String,
    pub notes: Option<String>,
}

/// Input for adding a medication to IPS
#[derive(Serialize, Deserialize, Debug)]
pub struct AddMedicationInput {
    pub ips_hash: ActionHash,
    pub medication_id: String,
    pub medication_code: String,
    pub coding_system: String,
    pub medication_display: String,
    pub status: MedicationStatus,
    pub dosage: String,
    pub route_code: Option<String>,
    pub form: Option<String>,
    pub strength: Option<String>,
    pub start_date: Option<Timestamp>,
    pub end_date: Option<Timestamp>,
    pub reason_code: Option<String>,
}

/// Input for adding a problem to IPS
#[derive(Serialize, Deserialize, Debug)]
pub struct AddProblemInput {
    pub ips_hash: ActionHash,
    pub problem_id: String,
    pub condition_code: String,
    pub coding_system: String,
    pub condition_display: String,
    pub clinical_status: ConditionClinicalStatus,
    pub verification_status: String,
    pub severity_code: Option<String>,
    pub body_site: Option<String>,
    pub onset_date: Option<Timestamp>,
    pub abatement_date: Option<Timestamp>,
    pub notes: Option<String>,
}

/// Input for adding an immunization to IPS
#[derive(Serialize, Deserialize, Debug)]
pub struct AddImmunizationInput {
    pub ips_hash: ActionHash,
    pub immunization_id: String,
    pub vaccine_code: String,
    pub coding_system: String,
    pub vaccine_display: String,
    pub occurrence_date: Timestamp,
    pub lot_number: Option<String>,
    pub expiration_date: Option<Timestamp>,
    pub dose_number: Option<u32>,
    pub series_doses: Option<u32>,
    pub route_code: Option<String>,
    pub site_code: Option<String>,
    pub performer: Option<String>,
    pub target_disease: Option<String>,
}

/// Input for sharing IPS across borders
#[derive(Serialize, Deserialize, Debug)]
pub struct ShareIpsInput {
    pub ips_hash: ActionHash,
    pub recipient_country: String,
    pub recipient_organization: String,
    pub recipient_identifier: Option<String>,
    pub purpose: String,
    pub expires_in_days: Option<u32>,
    pub consent_hash: Option<ActionHash>,
    pub translate_to: Option<Vec<String>>,
}

/// Input for translating IPS
#[derive(Serialize, Deserialize, Debug)]
pub struct TranslateIpsInput {
    pub ips_hash: ActionHash,
    pub target_language: String,
    pub translation_method: String,
    pub translator: Option<String>,
}

/// Generate a new IPS document
#[hdk_extern]
pub fn generate_ips(input: GenerateIpsInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;
    let author_hash = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    // Calculate expiration if specified
    let expires_at = input.expires_in_days.map(|days| {
        let duration_micros = (days as i64) * 24 * 60 * 60 * 1_000_000;
        Timestamp::from_micros(now.as_micros() + duration_micros)
    });

    // Ensure required sections are included
    let mut sections = input.sections_to_include;
    let required = vec![
        IpsSection::AllergiesIntolerances,
        IpsSection::MedicationSummary,
        IpsSection::ProblemList,
    ];
    for section in required {
        if !sections.contains(&section) {
            sections.push(section);
        }
    }

    // Generate empty FHIR bundle structure
    let fhir_bundle = generate_empty_fhir_bundle(&input.patient_hash, &input.language)?;

    let ips = InternationalPatientSummary {
        ips_id: format!("ips-{}", now.as_micros()),
        patient_hash: input.patient_hash.clone(),
        status: IpsStatus::Draft,
        version: 1,
        language: input.language.to_lowercase(),
        country_of_origin: input.country_of_origin.to_uppercase(),
        author_organization: input.author_organization,
        author_hash: Some(author_hash),
        generated_at: now,
        custodian: String::new(),
        sections_included: sections,
        fhir_bundle,
        signature: None,
        previous_version_hash: None,
        expires_at,
    };

    let action_hash = create_entry(EntryTypes::InternationalPatientSummary(ips))?;

    // Link from patient
    create_link(
        input.patient_hash,
        action_hash.clone(),
        LinkTypes::PatientToIps,
        (),
    )?;

    // Link to all IPS index
    let all_anchor = anchor_hash("all_ips_documents")?;
    create_link(
        all_anchor,
        action_hash.clone(),
        LinkTypes::AllIpsDocuments,
        (),
    )?;

    Ok(action_hash)
}

/// Get all IPS documents for a patient
#[hdk_extern]
pub fn get_patient_ips_documents(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(patient_hash, LinkTypes::PatientToIps)?, GetStrategy::default())?;

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

/// Finalize and sign an IPS document
#[hdk_extern]
pub fn finalize_ips(ips_hash: ActionHash) -> ExternResult<ActionHash> {
    let _now = sys_time()?;

    let record = get(ips_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("IPS not found".to_string())))?;

    let mut ips: InternationalPatientSummary = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid IPS entry".to_string())))?;

    // Rebuild FHIR bundle with all sections
    let allergies = get_ips_allergies(ips_hash.clone())?;
    let medications = get_ips_medications(ips_hash.clone())?;
    let problems = get_ips_problems(ips_hash.clone())?;
    let immunizations = get_ips_immunizations(ips_hash.clone())?;

    ips.fhir_bundle = build_complete_fhir_bundle(
        &ips,
        &allergies,
        &medications,
        &problems,
        &immunizations,
    )?;

    ips.status = IpsStatus::Current;

    update_entry(ips_hash, ips)
}

/// Add an allergy entry to IPS
#[hdk_extern]
pub fn add_ips_allergy(input: AddAllergyInput) -> ExternResult<ActionHash> {
    let allergy = IpsAllergy {
        allergy_id: input.allergy_id,
        ips_hash: input.ips_hash.clone(),
        category: input.category,
        agent_code: input.agent_code,
        coding_system: input.coding_system,
        agent_display: input.agent_display,
        severity: input.severity,
        criticality: input.criticality,
        reactions: input.reactions,
        onset_date: input.onset_date,
        verification_status: input.verification_status,
        notes: input.notes,
    };

    let action_hash = create_entry(EntryTypes::IpsAllergy(allergy))?;

    create_link(
        input.ips_hash,
        action_hash.clone(),
        LinkTypes::IpsToAllergies,
        (),
    )?;

    Ok(action_hash)
}

/// Get allergies for an IPS
#[hdk_extern]
pub fn get_ips_allergies(ips_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(ips_hash, LinkTypes::IpsToAllergies)?, GetStrategy::default())?;

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

/// Add a medication entry to IPS
#[hdk_extern]
pub fn add_ips_medication(input: AddMedicationInput) -> ExternResult<ActionHash> {
    let medication = IpsMedication {
        medication_id: input.medication_id,
        ips_hash: input.ips_hash.clone(),
        medication_code: input.medication_code,
        coding_system: input.coding_system,
        medication_display: input.medication_display,
        status: input.status,
        dosage: input.dosage,
        route_code: input.route_code,
        form: input.form,
        strength: input.strength,
        start_date: input.start_date,
        end_date: input.end_date,
        reason_code: input.reason_code,
        prescriber_hash: None,
    };

    let action_hash = create_entry(EntryTypes::IpsMedication(medication))?;

    create_link(
        input.ips_hash,
        action_hash.clone(),
        LinkTypes::IpsToMedications,
        (),
    )?;

    Ok(action_hash)
}

/// Get medications for an IPS
#[hdk_extern]
pub fn get_ips_medications(ips_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(ips_hash, LinkTypes::IpsToMedications)?, GetStrategy::default())?;

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

/// Add a problem entry to IPS
#[hdk_extern]
pub fn add_ips_problem(input: AddProblemInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let problem = IpsProblem {
        problem_id: input.problem_id,
        ips_hash: input.ips_hash.clone(),
        condition_code: input.condition_code,
        coding_system: input.coding_system,
        condition_display: input.condition_display,
        clinical_status: input.clinical_status,
        verification_status: input.verification_status,
        severity_code: input.severity_code,
        body_site: input.body_site,
        onset_date: input.onset_date,
        abatement_date: input.abatement_date,
        recorded_date: now,
        notes: input.notes,
    };

    let action_hash = create_entry(EntryTypes::IpsProblem(problem))?;

    create_link(
        input.ips_hash,
        action_hash.clone(),
        LinkTypes::IpsToProblems,
        (),
    )?;

    Ok(action_hash)
}

/// Get problems for an IPS
#[hdk_extern]
pub fn get_ips_problems(ips_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(ips_hash, LinkTypes::IpsToProblems)?, GetStrategy::default())?;

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

/// Add an immunization entry to IPS
#[hdk_extern]
pub fn add_ips_immunization(input: AddImmunizationInput) -> ExternResult<ActionHash> {
    let immunization = IpsImmunization {
        immunization_id: input.immunization_id,
        ips_hash: input.ips_hash.clone(),
        vaccine_code: input.vaccine_code,
        coding_system: input.coding_system,
        vaccine_display: input.vaccine_display,
        occurrence_date: input.occurrence_date,
        lot_number: input.lot_number,
        expiration_date: input.expiration_date,
        dose_number: input.dose_number,
        series_doses: input.series_doses,
        route_code: input.route_code,
        site_code: input.site_code,
        performer: input.performer,
        target_disease: input.target_disease,
    };

    let action_hash = create_entry(EntryTypes::IpsImmunization(immunization))?;

    create_link(
        input.ips_hash,
        action_hash.clone(),
        LinkTypes::IpsToImmunizations,
        (),
    )?;

    Ok(action_hash)
}

/// Get immunizations for an IPS
#[hdk_extern]
pub fn get_ips_immunizations(ips_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(ips_hash, LinkTypes::IpsToImmunizations)?, GetStrategy::default())?;

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

/// Share IPS across borders
#[hdk_extern]
pub fn share_ips(input: ShareIpsInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    // Get IPS to verify it exists and get patient
    let record = get(input.ips_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("IPS not found".to_string())))?;

    let ips: InternationalPatientSummary = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid IPS entry".to_string())))?;

    // Calculate expiration
    let expires_at = input.expires_in_days.map(|days| {
        let duration_micros = (days as i64) * 24 * 60 * 60 * 1_000_000;
        Timestamp::from_micros(now.as_micros() + duration_micros)
    });

    let share_record = IpsShareRecord {
        share_id: format!("share-{}", now.as_micros()),
        ips_hash: input.ips_hash.clone(),
        patient_hash: ips.patient_hash,
        recipient_country: input.recipient_country,
        recipient_organization: input.recipient_organization,
        recipient_identifier: input.recipient_identifier,
        purpose: input.purpose,
        shared_at: now,
        expires_at,
        access_count: 0,
        last_accessed_at: None,
        consent_hash: input.consent_hash,
        was_translated: input.translate_to.is_some(),
        translation_languages: input.translate_to.clone().unwrap_or_default(),
    };

    let action_hash = create_entry(EntryTypes::IpsShareRecord(share_record))?;

    create_link(
        input.ips_hash.clone(),
        action_hash.clone(),
        LinkTypes::IpsToShares,
        (),
    )?;

    // Create translations if requested
    if let Some(languages) = input.translate_to {
        for lang in languages {
            let translate_input = TranslateIpsInput {
                ips_hash: input.ips_hash.clone(),
                target_language: lang,
                translation_method: "machine".to_string(),
                translator: None,
            };
            translate_ips(translate_input)?;
        }
    }

    Ok(action_hash)
}

/// Get share records for an IPS
#[hdk_extern]
pub fn get_ips_shares(ips_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(ips_hash, LinkTypes::IpsToShares)?, GetStrategy::default())?;

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

/// Translate an IPS document
#[hdk_extern]
pub fn translate_ips(input: TranslateIpsInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    // Get original IPS
    let record = get(input.ips_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("IPS not found".to_string())))?;

    let ips: InternationalPatientSummary = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid IPS entry".to_string())))?;

    // In production, would call translation service here
    // For now, just record the translation request
    let translation = IpsTranslation {
        translation_id: format!("trans-{}", now.as_micros()),
        original_ips_hash: input.ips_hash.clone(),
        source_language: ips.language,
        target_language: input.target_language,
        translation_method: input.translation_method,
        translator: input.translator,
        translated_bundle: ips.fhir_bundle.clone(), // Would be translated in production
        quality_score: None,
        human_verified: false,
        verified_at: None,
        created_at: now,
    };

    let action_hash = create_entry(EntryTypes::IpsTranslation(translation))?;

    create_link(
        input.ips_hash,
        action_hash.clone(),
        LinkTypes::IpsToTranslations,
        (),
    )?;

    Ok(action_hash)
}

/// Get translations for an IPS
#[hdk_extern]
pub fn get_ips_translations(ips_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(ips_hash, LinkTypes::IpsToTranslations)?, GetStrategy::default())?;

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

/// Export IPS as FHIR Bundle JSON
#[hdk_extern]
pub fn export_ips_fhir(ips_hash: ActionHash) -> ExternResult<String> {
    let record = get(ips_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("IPS not found".to_string())))?;

    let ips: InternationalPatientSummary = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid IPS entry".to_string())))?;

    Ok(ips.fhir_bundle)
}

// Helper functions

/// Anchor for linking entries
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor: &str) -> ExternResult<AnyLinkableHash> {
    let anchor = Anchor(anchor.to_string());
    Ok(hash_entry(&anchor)?.into())
}

fn generate_empty_fhir_bundle(patient_hash: &ActionHash, language: &str) -> ExternResult<String> {
    // Generate minimal FHIR IPS Bundle structure
    let bundle = format!(
        r#"{{
  "resourceType": "Bundle",
  "id": "ips-bundle-{}",
  "language": "{}",
  "type": "document",
  "timestamp": "{}",
  "entry": []
}}"#,
        patient_hash,
        language,
        sys_time()?.as_micros()
    );
    Ok(bundle)
}

fn build_complete_fhir_bundle(
    _ips: &InternationalPatientSummary,
    _allergies: &[Record],
    _medications: &[Record],
    _problems: &[Record],
    _immunizations: &[Record],
) -> ExternResult<String> {
    // In production, would build complete FHIR IPS Bundle with all sections
    // For now, return placeholder
    Ok(r#"{"resourceType":"Bundle","type":"document","entry":[]}"#.to_string())
}
