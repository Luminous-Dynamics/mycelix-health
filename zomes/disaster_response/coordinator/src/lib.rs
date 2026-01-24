//! Disaster Response Coordinator Zome
//!
//! Provides functions for emergency and disaster healthcare operations
//! including triage, resource management, and emergency access protocols.

use hdk::prelude::*;
use disaster_response_integrity::*;

/// Input for declaring a disaster
#[derive(Serialize, Deserialize, Debug)]
pub struct DeclareDisasterInput {
    pub disaster_type: DisasterType,
    pub severity: SeverityLevel,
    pub title: String,
    pub description: String,
    pub affected_area: String,
    pub coordinates: Option<(String, String)>,
    pub affected_radius_km: Option<u32>,
    pub declaring_authority: String,
    pub lead_organization: String,
    pub emergency_contacts: Vec<String>,
}

/// Input for creating a triage record
#[derive(Serialize, Deserialize, Debug)]
pub struct TriagePatientInput {
    pub disaster_hash: ActionHash,
    pub patient_hash: Option<ActionHash>,
    pub temp_patient_id: String,
    pub triage_category: TriageCategory,
    pub chief_complaint: String,
    pub vital_signs: Option<String>,
    pub injuries: Vec<String>,
    pub ambulatory: bool,
    pub respiratory_status: String,
    pub mental_status: String,
    pub triage_location: String,
    pub assigned_destination: Option<String>,
}

/// Input for registering a resource
#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterResourceInput {
    pub disaster_hash: ActionHash,
    pub resource_type: ResourceType,
    pub name: String,
    pub quantity_available: u32,
    pub unit: String,
    pub location: String,
    pub organization: String,
    pub expires_at: Option<Timestamp>,
    pub reorder_threshold: Option<u32>,
    pub contact: Option<String>,
}

/// Input for emergency assignment
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateAssignmentInput {
    pub disaster_hash: ActionHash,
    pub provider_hash: ActionHash,
    pub role: String,
    pub assigned_location: String,
    pub shift_start: Timestamp,
    pub shift_end: Timestamp,
    pub supervisor_hash: Option<ActionHash>,
    pub special_skills: Vec<String>,
    pub contact_number: Option<String>,
}

/// Input for patient tracking
#[derive(Serialize, Deserialize, Debug)]
pub struct TrackPatientInput {
    pub disaster_hash: ActionHash,
    pub patient_hash: ActionHash,
    pub status: PatientDisasterStatus,
    pub current_location: Option<String>,
    pub shelter_id: Option<String>,
    pub medical_facility: Option<String>,
    pub requires_medical_attention: bool,
    pub medical_needs: Option<String>,
    pub essential_medications: Vec<String>,
    pub equipment_needs: Vec<String>,
}

/// Input for emergency access
#[derive(Serialize, Deserialize, Debug)]
pub struct RequestEmergencyAccessInput {
    pub disaster_hash: ActionHash,
    pub patient_hash: ActionHash,
    pub accessor_organization: String,
    pub reason: String,
    pub data_categories: Vec<String>,
    pub waiver_justification: Option<String>,
}

/// Input for shelter registration
#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterShelterInput {
    pub disaster_hash: ActionHash,
    pub name: String,
    pub address: String,
    pub coordinates: Option<(String, String)>,
    pub capacity: u32,
    pub medical_staff_count: u32,
    pub has_medical_station: bool,
    pub is_accessible: bool,
    pub accepts_pets: bool,
    pub special_needs_capacity: u32,
    pub contact_number: String,
    pub operating_org: String,
}

/// Declare a new disaster
#[hdk_extern]
pub fn declare_disaster(input: DeclareDisasterInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let disaster = DisasterDeclaration {
        disaster_id: format!("disaster-{}", now.as_micros()),
        disaster_type: input.disaster_type,
        severity: input.severity,
        title: input.title,
        description: input.description,
        affected_area: input.affected_area,
        coordinates: input.coordinates,
        affected_radius_km: input.affected_radius_km,
        started_at: now,
        ended_at: None,
        is_active: true,
        declaring_authority: input.declaring_authority,
        declaration_document_hash: None,
        lead_organization: input.lead_organization,
        emergency_contacts: input.emergency_contacts,
        created_at: now,
        updated_at: now,
    };

    let action_hash = create_entry(EntryTypes::DisasterDeclaration(disaster))?;

    // Link to active disasters
    let active_anchor = anchor_hash("active_disasters")?;
    create_link(
        active_anchor,
        action_hash.clone(),
        LinkTypes::ActiveDisasters,
        (),
    )?;

    // Link to all disasters
    let all_anchor = anchor_hash("all_disasters")?;
    create_link(
        all_anchor,
        action_hash.clone(),
        LinkTypes::AllDisasters,
        (),
    )?;

    Ok(action_hash)
}

/// Get active disasters
#[hdk_extern]
pub fn get_active_disasters(_: ()) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash("active_disasters")?;
    let links = get_links(
        GetLinksInputBuilder::try_new(anchor, LinkTypes::ActiveDisasters)?.build(),
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

/// End a disaster declaration
#[hdk_extern]
pub fn end_disaster(disaster_hash: ActionHash) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let record = get(disaster_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Disaster not found".to_string())))?;

    let mut disaster: DisasterDeclaration = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid disaster entry".to_string())))?;

    disaster.is_active = false;
    disaster.ended_at = Some(now);
    disaster.updated_at = now;

    update_entry(disaster_hash, disaster)
}

/// Triage a patient
#[hdk_extern]
pub fn triage_patient(input: TriagePatientInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;
    let triaged_by = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let triage = TriageRecord {
        triage_id: format!("triage-{}", now.as_micros()),
        disaster_hash: input.disaster_hash.clone(),
        patient_hash: input.patient_hash,
        temp_patient_id: input.temp_patient_id,
        triage_category: input.triage_category.clone(),
        chief_complaint: input.chief_complaint,
        vital_signs: input.vital_signs,
        injuries: input.injuries,
        ambulatory: input.ambulatory,
        respiratory_status: input.respiratory_status,
        mental_status: input.mental_status,
        triage_location: input.triage_location,
        assigned_destination: input.assigned_destination,
        triaged_by,
        triaged_at: now,
        retriage_history: None,
    };

    let action_hash = create_entry(EntryTypes::TriageRecord(triage))?;

    // Link from disaster
    create_link(
        input.disaster_hash.clone(),
        action_hash.clone(),
        LinkTypes::DisasterToTriage,
        (),
    )?;

    // Link by category
    let category_anchor = anchor_hash(&format!(
        "triage_{:?}_{}",
        input.triage_category, input.disaster_hash
    ))?;
    create_link(
        category_anchor,
        action_hash.clone(),
        LinkTypes::TriageByCategory,
        (),
    )?;

    Ok(action_hash)
}

/// Get triage records for a disaster
#[hdk_extern]
pub fn get_disaster_triage(disaster_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(disaster_hash, LinkTypes::DisasterToTriage)?.build(),
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

/// Register a resource
#[hdk_extern]
pub fn register_resource(input: RegisterResourceInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let resource = DisasterResource {
        resource_id: format!("resource-{}", now.as_micros()),
        disaster_hash: input.disaster_hash.clone(),
        resource_type: input.resource_type,
        name: input.name,
        quantity_available: input.quantity_available,
        quantity_in_use: 0,
        unit: input.unit,
        location: input.location,
        status: ResourceStatus::Available,
        expires_at: input.expires_at,
        reorder_threshold: input.reorder_threshold,
        organization: input.organization,
        contact: input.contact,
        updated_at: now,
    };

    let action_hash = create_entry(EntryTypes::DisasterResource(resource))?;

    create_link(
        input.disaster_hash,
        action_hash.clone(),
        LinkTypes::DisasterToResources,
        (),
    )?;

    Ok(action_hash)
}

/// Get resources for a disaster
#[hdk_extern]
pub fn get_disaster_resources(disaster_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(disaster_hash, LinkTypes::DisasterToResources)?.build(),
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

/// Create emergency assignment
#[hdk_extern]
pub fn create_emergency_assignment(input: CreateAssignmentInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let assignment = EmergencyAssignment {
        assignment_id: format!("assign-{}", now.as_micros()),
        disaster_hash: input.disaster_hash.clone(),
        provider_hash: input.provider_hash,
        role: input.role,
        assigned_location: input.assigned_location,
        shift_start: input.shift_start,
        shift_end: input.shift_end,
        status: "pending".to_string(),
        supervisor_hash: input.supervisor_hash,
        special_skills: input.special_skills,
        contact_number: input.contact_number,
        checked_in_at: None,
        checked_out_at: None,
    };

    let action_hash = create_entry(EntryTypes::EmergencyAssignment(assignment))?;

    create_link(
        input.disaster_hash,
        action_hash.clone(),
        LinkTypes::DisasterToAssignments,
        (),
    )?;

    Ok(action_hash)
}

/// Check in for assignment
#[hdk_extern]
pub fn check_in_assignment(assignment_hash: ActionHash) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let record = get(assignment_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Assignment not found".to_string())))?;

    let mut assignment: EmergencyAssignment = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid assignment".to_string())))?;

    assignment.status = "active".to_string();
    assignment.checked_in_at = Some(now);

    update_entry(assignment_hash, assignment)
}

/// Track patient during disaster
#[hdk_extern]
pub fn track_patient(input: TrackPatientInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;
    let reported_by = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let tracking = PatientDisasterTracking {
        tracking_id: format!("track-{}", now.as_micros()),
        disaster_hash: input.disaster_hash.clone(),
        patient_hash: input.patient_hash.clone(),
        status: input.status,
        last_known_location: input.current_location.clone(),
        current_location: input.current_location,
        shelter_id: input.shelter_id,
        medical_facility: input.medical_facility,
        emergency_contact_notified: false,
        requires_medical_attention: input.requires_medical_attention,
        medical_needs: input.medical_needs,
        essential_medications: input.essential_medications,
        equipment_needs: input.equipment_needs,
        reported_by: Some(reported_by),
        updated_at: now,
    };

    let action_hash = create_entry(EntryTypes::PatientDisasterTracking(tracking))?;

    create_link(
        input.disaster_hash,
        action_hash.clone(),
        LinkTypes::DisasterToPatients,
        (),
    )?;

    Ok(action_hash)
}

/// Request emergency access to patient records
#[hdk_extern]
pub fn request_emergency_access(input: RequestEmergencyAccessInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;
    let accessor_hash = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let consent_waived = input.waiver_justification.is_some();

    let access = EmergencyAccess {
        access_id: format!("emerg-access-{}", now.as_micros()),
        disaster_hash: input.disaster_hash,
        patient_hash: input.patient_hash.clone(),
        accessor_hash,
        accessor_organization: input.accessor_organization,
        reason: input.reason,
        data_accessed: input.data_categories,
        accessed_at: now,
        consent_waived,
        waiver_justification: input.waiver_justification,
        follow_up_consent: false,
        consent_obtained_at: None,
    };

    let action_hash = create_entry(EntryTypes::EmergencyAccess(access))?;

    create_link(
        input.patient_hash,
        action_hash.clone(),
        LinkTypes::PatientToEmergencyAccess,
        (),
    )?;

    Ok(action_hash)
}

/// Get emergency access logs for patient
#[hdk_extern]
pub fn get_patient_emergency_access(patient_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(patient_hash, LinkTypes::PatientToEmergencyAccess)?.build(),
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

/// Register a shelter
#[hdk_extern]
pub fn register_shelter(input: RegisterShelterInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let shelter = ShelterHealth {
        shelter_id: format!("shelter-{}", now.as_micros()),
        disaster_hash: input.disaster_hash.clone(),
        name: input.name,
        address: input.address,
        coordinates: input.coordinates,
        capacity: input.capacity,
        current_occupancy: 0,
        medical_staff_count: input.medical_staff_count,
        has_medical_station: input.has_medical_station,
        is_accessible: input.is_accessible,
        accepts_pets: input.accepts_pets,
        special_needs_capacity: input.special_needs_capacity,
        health_alerts: vec![],
        contact_number: input.contact_number,
        operating_org: input.operating_org,
        status: "open".to_string(),
        updated_at: now,
    };

    let action_hash = create_entry(EntryTypes::ShelterHealth(shelter))?;

    create_link(
        input.disaster_hash,
        action_hash.clone(),
        LinkTypes::DisasterToShelters,
        (),
    )?;

    Ok(action_hash)
}

/// Get shelters for a disaster
#[hdk_extern]
pub fn get_disaster_shelters(disaster_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(disaster_hash, LinkTypes::DisasterToShelters)?.build(),
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

/// Request resources
#[hdk_extern]
pub fn request_resources(
    disaster_hash: ActionHash,
    requesting_org: String,
    requesting_location: String,
    resource_type: ResourceType,
    resource_description: String,
    quantity_needed: u32,
    unit: String,
    priority: u32,
    justification: String,
) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    let request = ResourceRequest {
        request_id: format!("req-{}", now.as_micros()),
        disaster_hash: disaster_hash.clone(),
        requesting_org,
        requesting_location,
        resource_type,
        resource_description,
        quantity_needed,
        unit,
        priority,
        justification,
        status: "pending".to_string(),
        approved_by: None,
        fulfilled_by: None,
        requested_at: now,
        fulfilled_at: None,
    };

    let action_hash = create_entry(EntryTypes::ResourceRequest(request))?;

    create_link(
        disaster_hash,
        action_hash.clone(),
        LinkTypes::DisasterToRequests,
        (),
    )?;

    Ok(action_hash)
}

/// Get resource requests for a disaster
#[hdk_extern]
pub fn get_resource_requests(disaster_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(disaster_hash, LinkTypes::DisasterToRequests)?.build(),
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

// Helper function
fn anchor_hash(anchor: &str) -> ExternResult<AnyLinkableHash> {
    let anchor_bytes = anchor.as_bytes().to_vec();
    Ok(AnyLinkableHash::from(
        EntryHash::from_raw_36(
            hdk::hash::hash_keccak256(anchor_bytes)
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?[..36]
                .to_vec(),
        ),
    ))
}
