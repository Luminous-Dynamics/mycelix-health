//! Disaster Response Integrity Zome
//!
//! Defines entry types for emergency and disaster healthcare operations
//! including triage, resource management, and emergency access protocols.

use hdi::prelude::*;

/// Type of disaster or emergency
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DisasterType {
    /// Natural disasters
    Earthquake,
    Hurricane,
    Tornado,
    Flood,
    Wildfire,
    Tsunami,
    /// Human-caused
    MassCasualty,
    ChemicalSpill,
    IndustrialAccident,
    TerroristAttack,
    /// Health emergencies
    Pandemic,
    Outbreak,
    MassPoisoning,
    /// Infrastructure
    PowerOutage,
    WaterContamination,
    CommunicationsFailure,
    /// Other
    Other(String),
}

/// Disaster severity level
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SeverityLevel {
    /// Local, single facility
    Level1,
    /// Multiple facilities, county-wide
    Level2,
    /// Regional, multi-county
    Level3,
    /// State-wide emergency
    Level4,
    /// National emergency
    Level5,
}

/// Triage category (START triage)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TriageCategory {
    /// Immediate (red) - life-threatening
    Immediate,
    /// Delayed (yellow) - serious but can wait
    Delayed,
    /// Minor (green) - walking wounded
    Minor,
    /// Expectant (gray) - unlikely to survive
    Expectant,
    /// Deceased (black)
    Deceased,
}

/// Resource type
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ResourceType {
    /// Medical personnel
    Personnel,
    /// Medical supplies
    Supplies,
    /// Medical equipment
    Equipment,
    /// Hospital beds
    Beds,
    /// Ambulances/transport
    Transport,
    /// Blood products
    BloodProducts,
    /// Medications
    Medications,
    /// Shelter capacity
    ShelterCapacity,
}

/// Resource status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ResourceStatus {
    Available,
    InUse,
    Depleted,
    Incoming,
    Reserved,
    Unknown,
}

/// Patient status during disaster
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PatientDisasterStatus {
    /// Unaffected
    Unaffected,
    /// Displaced, location unknown
    Displaced,
    /// At shelter
    AtShelter,
    /// At medical facility
    AtMedicalFacility,
    /// Missing
    Missing,
    /// Found/located
    Located,
    /// Evacuated
    Evacuated,
    /// Deceased
    Deceased,
    /// Unknown
    Unknown,
}

/// A declared disaster or emergency
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DisasterDeclaration {
    /// Disaster ID
    pub disaster_id: String,
    /// Disaster type
    pub disaster_type: DisasterType,
    /// Severity level
    pub severity: SeverityLevel,
    /// Title/name
    pub title: String,
    /// Description
    pub description: String,
    /// Affected area description
    pub affected_area: String,
    /// Geographic coordinates (center)
    pub coordinates: Option<(String, String)>,
    /// Affected radius (km)
    pub affected_radius_km: Option<u32>,
    /// Start timestamp
    pub started_at: Timestamp,
    /// End timestamp (if concluded)
    pub ended_at: Option<Timestamp>,
    /// Is currently active
    pub is_active: bool,
    /// Declaring authority
    pub declaring_authority: String,
    /// Declaration document hash
    pub declaration_document_hash: Option<ActionHash>,
    /// Lead response organization
    pub lead_organization: String,
    /// Emergency contact numbers
    pub emergency_contacts: Vec<String>,
    /// Created timestamp
    pub created_at: Timestamp,
    /// Updated timestamp
    pub updated_at: Timestamp,
}

/// Triage record for mass casualty
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TriageRecord {
    /// Triage ID
    pub triage_id: String,
    /// Disaster hash
    pub disaster_hash: ActionHash,
    /// Patient hash (if known)
    pub patient_hash: Option<ActionHash>,
    /// Temporary patient ID (if unidentified)
    pub temp_patient_id: String,
    /// Triage category
    pub triage_category: TriageCategory,
    /// Chief complaint
    pub chief_complaint: String,
    /// Vital signs at triage
    pub vital_signs: Option<String>,
    /// Injuries description
    pub injuries: Vec<String>,
    /// Ambulatory (can walk)
    pub ambulatory: bool,
    /// Respiratory status
    pub respiratory_status: String,
    /// Mental status (AVPU)
    pub mental_status: String,
    /// Triage location
    pub triage_location: String,
    /// Assigned destination
    pub assigned_destination: Option<String>,
    /// Triaged by (provider hash)
    pub triaged_by: ActionHash,
    /// Triage timestamp
    pub triaged_at: Timestamp,
    /// Re-triage history (JSON)
    pub retriage_history: Option<String>,
}

/// Resource inventory during disaster
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DisasterResource {
    /// Resource ID
    pub resource_id: String,
    /// Disaster hash
    pub disaster_hash: ActionHash,
    /// Resource type
    pub resource_type: ResourceType,
    /// Resource name/description
    pub name: String,
    /// Quantity available
    pub quantity_available: u32,
    /// Quantity in use
    pub quantity_in_use: u32,
    /// Unit of measure
    pub unit: String,
    /// Location
    pub location: String,
    /// Status
    pub status: ResourceStatus,
    /// Expiration (for supplies)
    pub expires_at: Option<Timestamp>,
    /// Reorder threshold
    pub reorder_threshold: Option<u32>,
    /// Managing organization
    pub organization: String,
    /// Contact for resource
    pub contact: Option<String>,
    /// Last updated
    pub updated_at: Timestamp,
}

/// Emergency personnel assignment
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct EmergencyAssignment {
    /// Assignment ID
    pub assignment_id: String,
    /// Disaster hash
    pub disaster_hash: ActionHash,
    /// Provider hash
    pub provider_hash: ActionHash,
    /// Role/position
    pub role: String,
    /// Assigned location
    pub assigned_location: String,
    /// Shift start
    pub shift_start: Timestamp,
    /// Shift end
    pub shift_end: Timestamp,
    /// Status (active, completed, cancelled)
    pub status: String,
    /// Supervisor hash
    pub supervisor_hash: Option<ActionHash>,
    /// Special skills
    pub special_skills: Vec<String>,
    /// Contact number
    pub contact_number: Option<String>,
    /// Check-in timestamp
    pub checked_in_at: Option<Timestamp>,
    /// Check-out timestamp
    pub checked_out_at: Option<Timestamp>,
}

/// Patient disaster status tracking
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PatientDisasterTracking {
    /// Tracking ID
    pub tracking_id: String,
    /// Disaster hash
    pub disaster_hash: ActionHash,
    /// Patient hash
    pub patient_hash: ActionHash,
    /// Current status
    pub status: PatientDisasterStatus,
    /// Last known location
    pub last_known_location: Option<String>,
    /// Current location
    pub current_location: Option<String>,
    /// Shelter ID (if at shelter)
    pub shelter_id: Option<String>,
    /// Medical facility (if admitted)
    pub medical_facility: Option<String>,
    /// Emergency contact notified
    pub emergency_contact_notified: bool,
    /// Requires medical attention
    pub requires_medical_attention: bool,
    /// Medical needs summary
    pub medical_needs: Option<String>,
    /// Essential medications
    pub essential_medications: Vec<String>,
    /// Medical equipment needs
    pub equipment_needs: Vec<String>,
    /// Reported by
    pub reported_by: Option<ActionHash>,
    /// Last update
    pub updated_at: Timestamp,
}

/// Emergency health record access
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct EmergencyAccess {
    /// Access ID
    pub access_id: String,
    /// Disaster hash
    pub disaster_hash: ActionHash,
    /// Patient hash
    pub patient_hash: ActionHash,
    /// Accessor hash (provider)
    pub accessor_hash: ActionHash,
    /// Accessor organization
    pub accessor_organization: String,
    /// Reason for access
    pub reason: String,
    /// Data accessed (categories)
    pub data_accessed: Vec<String>,
    /// Access timestamp
    pub accessed_at: Timestamp,
    /// Consent was waived (emergency)
    pub consent_waived: bool,
    /// Waiver justification
    pub waiver_justification: Option<String>,
    /// Follow-up consent obtained
    pub follow_up_consent: bool,
    /// Follow-up consent timestamp
    pub consent_obtained_at: Option<Timestamp>,
}

/// Shelter health operations
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ShelterHealth {
    /// Shelter ID
    pub shelter_id: String,
    /// Disaster hash
    pub disaster_hash: ActionHash,
    /// Shelter name
    pub name: String,
    /// Address
    pub address: String,
    /// Coordinates
    pub coordinates: Option<(String, String)>,
    /// Total capacity
    pub capacity: u32,
    /// Current occupancy
    pub current_occupancy: u32,
    /// Medical staff on site
    pub medical_staff_count: u32,
    /// Has medical station
    pub has_medical_station: bool,
    /// Accessible (ADA compliant)
    pub is_accessible: bool,
    /// Accepts pets
    pub accepts_pets: bool,
    /// Special needs capacity
    pub special_needs_capacity: u32,
    /// Current health alerts
    pub health_alerts: Vec<String>,
    /// Contact number
    pub contact_number: String,
    /// Operating organization
    pub operating_org: String,
    /// Status (open, full, closed)
    pub status: String,
    /// Last updated
    pub updated_at: Timestamp,
}

/// Resource request during disaster
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ResourceRequest {
    /// Request ID
    pub request_id: String,
    /// Disaster hash
    pub disaster_hash: ActionHash,
    /// Requesting organization
    pub requesting_org: String,
    /// Requesting location
    pub requesting_location: String,
    /// Resource type
    pub resource_type: ResourceType,
    /// Resource name/description
    pub resource_description: String,
    /// Quantity needed
    pub quantity_needed: u32,
    /// Unit
    pub unit: String,
    /// Priority (1-5)
    pub priority: u32,
    /// Justification
    pub justification: String,
    /// Status (pending, approved, fulfilled, denied)
    pub status: String,
    /// Approved by
    pub approved_by: Option<ActionHash>,
    /// Fulfilled by
    pub fulfilled_by: Option<String>,
    /// Requested at
    pub requested_at: Timestamp,
    /// Fulfilled at
    pub fulfilled_at: Option<Timestamp>,
}

/// Entry types for the disaster response zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    DisasterDeclaration(DisasterDeclaration),
    TriageRecord(TriageRecord),
    DisasterResource(DisasterResource),
    EmergencyAssignment(EmergencyAssignment),
    PatientDisasterTracking(PatientDisasterTracking),
    EmergencyAccess(EmergencyAccess),
    ShelterHealth(ShelterHealth),
    ResourceRequest(ResourceRequest),
}

/// Link types for the disaster response zome
#[hdk_link_types]
pub enum LinkTypes {
    /// Active disasters
    ActiveDisasters,
    /// All disasters
    AllDisasters,
    /// Disaster to triage records
    DisasterToTriage,
    /// Disaster to resources
    DisasterToResources,
    /// Disaster to assignments
    DisasterToAssignments,
    /// Disaster to patient tracking
    DisasterToPatients,
    /// Disaster to shelters
    DisasterToShelters,
    /// Disaster to resource requests
    DisasterToRequests,
    /// Patient to emergency access logs
    PatientToEmergencyAccess,
    /// Triage by category
    TriageByCategory,
}

/// Validation callbacks
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::DisasterDeclaration(disaster) => validate_disaster(&disaster),
                EntryTypes::TriageRecord(triage) => validate_triage(&triage),
                EntryTypes::EmergencyAccess(access) => validate_access(&access),
                _ => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_disaster(disaster: &DisasterDeclaration) -> ExternResult<ValidateCallbackResult> {
    if disaster.disaster_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Disaster ID is required".to_string()));
    }
    if disaster.title.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Disaster title is required".to_string()));
    }
    if disaster.affected_area.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Affected area is required".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_triage(triage: &TriageRecord) -> ExternResult<ValidateCallbackResult> {
    if triage.triage_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Triage ID is required".to_string()));
    }
    if triage.temp_patient_id.is_empty() && triage.patient_hash.is_none() {
        return Ok(ValidateCallbackResult::Invalid(
            "Either patient hash or temp ID is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_access(access: &EmergencyAccess) -> ExternResult<ValidateCallbackResult> {
    if access.access_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Access ID is required".to_string()));
    }
    if access.reason.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Access reason is required".to_string()));
    }
    if access.consent_waived && access.waiver_justification.is_none() {
        return Ok(ValidateCallbackResult::Invalid(
            "Waiver justification required when consent is waived".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
