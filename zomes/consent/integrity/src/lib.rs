//! Patient Consent and Data Access Authorization Integrity Zome
//!
//! Defines entry types for granular consent management, access control,
//! and audit logging with HIPAA alignment.

use hdi::prelude::*;

/// Consent directive from patient
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Consent {
    pub consent_id: String,
    pub patient_hash: ActionHash,
    /// Who can access the data
    pub grantee: ConsentGrantee,
    /// What data is covered
    pub scope: ConsentScope,
    /// What actions are permitted
    pub permissions: Vec<DataPermission>,
    /// Purpose of the consent
    pub purpose: ConsentPurpose,
    pub status: ConsentStatus,
    /// When consent was given
    pub granted_at: Timestamp,
    /// When consent expires (if applicable)
    pub expires_at: Option<Timestamp>,
    /// Was this consent revoked
    pub revoked_at: Option<Timestamp>,
    pub revocation_reason: Option<String>,
    /// Link to signed consent document
    pub document_hash: Option<EntryHash>,
    /// Witness to consent (if required)
    pub witness: Option<AgentPubKey>,
    /// Legal guardian if patient is minor/incapacitated
    pub legal_representative: Option<AgentPubKey>,
    /// Notes
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ConsentGrantee {
    /// Specific provider
    Provider(ActionHash),
    /// All providers at an organization
    Organization(String),
    /// Specific agent/user
    Agent(AgentPubKey),
    /// Research study
    ResearchStudy(ActionHash),
    /// Insurance company
    InsuranceCompany(ActionHash),
    /// Emergency access (any provider)
    EmergencyAccess,
    /// Public (for anonymized research data)
    Public,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ConsentScope {
    /// All data or specific categories
    pub data_categories: Vec<DataCategory>,
    /// Specific date range
    pub date_range: Option<DateRange>,
    /// Specific encounters
    pub encounter_hashes: Option<Vec<ActionHash>>,
    /// Exclusions
    pub exclusions: Vec<DataCategory>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DateRange {
    pub start: Timestamp,
    pub end: Option<Timestamp>,
}

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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DataPermission {
    Read,
    Write,
    Share,
    Export,
    Delete,
    Amend,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ConsentPurpose {
    Treatment,
    Payment,
    HealthcareOperations,
    Research,
    PublicHealth,
    LegalProceeding,
    Marketing,
    FamilyNotification,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ConsentStatus {
    Active,
    Expired,
    Revoked,
    Pending,
    Rejected,
}

/// Data access request
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DataAccessRequest {
    pub request_id: String,
    pub requestor: AgentPubKey,
    pub patient_hash: ActionHash,
    pub data_categories: Vec<DataCategory>,
    pub purpose: ConsentPurpose,
    pub justification: String,
    pub urgency: AccessUrgency,
    pub status: RequestStatus,
    pub requested_at: Timestamp,
    pub responded_at: Option<Timestamp>,
    pub response_by: Option<AgentPubKey>,
    pub resulting_consent: Option<ActionHash>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AccessUrgency {
    Emergency,
    Urgent,
    Routine,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RequestStatus {
    Pending,
    Approved,
    Denied,
    PartiallyApproved,
    Expired,
    Withdrawn,
}

/// Audit log entry for data access
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DataAccessLog {
    pub log_id: String,
    pub patient_hash: ActionHash,
    pub accessor: AgentPubKey,
    pub access_type: DataPermission,
    pub data_categories_accessed: Vec<DataCategory>,
    pub consent_hash: Option<ActionHash>,
    pub access_reason: String,
    pub accessed_at: Timestamp,
    /// IP address or system identifier (for audit)
    pub access_location: Option<String>,
    /// Was this an emergency override?
    pub emergency_override: bool,
    pub override_reason: Option<String>,
}

/// Break-glass emergency access record
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct EmergencyAccess {
    pub emergency_id: String,
    pub patient_hash: ActionHash,
    pub accessor: AgentPubKey,
    pub reason: String,
    pub clinical_justification: String,
    pub accessed_at: Timestamp,
    pub access_duration_minutes: u32,
    /// Supervisor who approved (if required)
    pub approved_by: Option<AgentPubKey>,
    /// Data accessed during emergency
    pub data_accessed: Vec<DataCategory>,
    /// Follow-up audit completed
    pub audited: bool,
    pub audited_by: Option<AgentPubKey>,
    pub audited_at: Option<Timestamp>,
    pub audit_findings: Option<String>,
}

/// HIPAA authorization document
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AuthorizationDocument {
    pub document_id: String,
    pub patient_hash: ActionHash,
    pub document_type: AuthorizationType,
    pub content_hash: EntryHash,
    pub signed_at: Timestamp,
    pub patient_signature: Vec<u8>,
    pub witness_signature: Option<Vec<u8>>,
    pub valid_until: Option<Timestamp>,
    pub revocable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum AuthorizationType {
    GeneralConsent,
    ResearchAuthorization,
    ReleaseOfInformation,
    AdvanceDirective,
    PowerOfAttorney,
    DNR,
    POLST,
    OrganDonation,
}

// ============================================================
// CONSENT DELEGATION SYSTEM
// ============================================================

/// Delegation grant allowing a delegate to act on behalf of patient
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DelegationGrant {
    pub delegation_id: String,
    pub patient_hash: ActionHash,
    /// The person being granted delegation powers
    pub delegate: AgentPubKey,
    /// Type of delegation
    pub delegation_type: DelegationType,
    /// What the delegate can do
    pub permissions: Vec<DelegationPermission>,
    /// What data categories the delegate can access
    pub data_scope: Vec<DataCategory>,
    /// Categories explicitly excluded
    pub exclusions: Vec<DataCategory>,
    /// Relationship to patient
    pub relationship: DelegateRelationship,
    /// When delegation was granted
    pub granted_at: Timestamp,
    /// When delegation expires
    pub expires_at: Option<Timestamp>,
    /// Has this been revoked?
    pub revoked_at: Option<Timestamp>,
    pub revocation_reason: Option<String>,
    /// Status
    pub status: DelegationStatus,
    /// Verification of delegate identity
    pub identity_verified: bool,
    pub verification_method: Option<String>,
    /// Legal documentation (if required)
    pub legal_document_hash: Option<EntryHash>,
    /// Notes
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DelegationType {
    /// Full healthcare proxy - can make all medical decisions
    HealthcareProxy,
    /// Caregiver - can view and coordinate care
    Caregiver,
    /// Family member - limited view access
    FamilyMember,
    /// Legal guardian (for minors or incapacitated)
    LegalGuardian,
    /// Temporary delegation (surgery recovery, travel)
    Temporary,
    /// Research participant advocate
    ResearchAdvocate,
    /// Financial only (billing, insurance)
    FinancialOnly,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DelegationPermission {
    /// View health records
    ViewRecords,
    /// Schedule appointments
    ScheduleAppointments,
    /// Communicate with providers
    CommunicateWithProviders,
    /// Make medical decisions
    MakeMedicalDecisions,
    /// Consent to treatment
    ConsentToTreatment,
    /// Manage medications
    ManageMedications,
    /// Access billing/insurance
    AccessFinancial,
    /// Receive notifications
    ReceiveNotifications,
    /// Export data
    ExportData,
    /// Grant further delegations (for healthcare proxy)
    SubDelegate,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DelegateRelationship {
    Spouse,
    Parent,
    Child,
    Sibling,
    Grandparent,
    Grandchild,
    LegalGuardian,
    PowerOfAttorney,
    CaregiverProfessional,
    CaregiverFamily,
    Friend,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DelegationStatus {
    Active,
    Expired,
    Revoked,
    Pending,
    Suspended,
}

// ============================================================
// PATIENT NOTIFICATION SYSTEM
// ============================================================

/// Notification sent to patient about data access
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AccessNotification {
    pub notification_id: String,
    pub patient_hash: ActionHash,
    /// Who accessed the data
    pub accessor: AgentPubKey,
    /// Human-readable accessor name
    pub accessor_name: String,
    /// What was accessed
    pub data_categories: Vec<DataCategory>,
    /// Why it was accessed
    pub purpose: String,
    /// When access occurred
    pub accessed_at: Timestamp,
    /// Was this emergency access?
    pub emergency_access: bool,
    /// Notification priority
    pub priority: NotificationPriority,
    /// Has patient viewed this notification?
    pub viewed: bool,
    pub viewed_at: Option<Timestamp>,
    /// Plain language summary
    pub summary: String,
    /// Link to full access log
    pub access_log_hash: Option<ActionHash>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum NotificationPriority {
    /// Immediate notification (emergency, new provider, sensitive data)
    Immediate,
    /// Include in daily digest
    Daily,
    /// Include in weekly summary
    Weekly,
    /// Silent (trusted provider, routine)
    Silent,
}

/// Patient's notification preferences
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct NotificationPreferences {
    pub patient_hash: ActionHash,
    /// Default notification level
    pub default_priority: NotificationPriority,
    /// Always notify immediately for these
    pub immediate_categories: Vec<DataCategory>,
    /// Never notify for these trusted agents
    pub silent_agents: Vec<AgentPubKey>,
    /// Always notify immediately for emergency access
    pub notify_emergency_access: bool,
    /// Always notify for new providers
    pub notify_new_providers: bool,
    /// Daily digest time (hour of day, 0-23)
    pub daily_digest_hour: Option<u8>,
    /// Weekly summary day (0=Sunday, 6=Saturday)
    pub weekly_summary_day: Option<u8>,
    /// Email notifications enabled
    pub email_enabled: bool,
    pub email_address: Option<String>,
    /// Push notifications enabled
    pub push_enabled: bool,
    /// SMS notifications enabled
    pub sms_enabled: bool,
    pub phone_number: Option<String>,
    /// Last updated
    pub updated_at: Timestamp,
}

/// Notification digest (daily/weekly summary)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct NotificationDigest {
    pub digest_id: String,
    pub patient_hash: ActionHash,
    pub digest_type: DigestType,
    /// Period covered
    pub period_start: Timestamp,
    pub period_end: Timestamp,
    /// Summary of access events
    pub total_access_events: u32,
    pub unique_accessors: u32,
    pub categories_accessed: Vec<DataCategory>,
    pub emergency_accesses: u32,
    /// Was digest viewed?
    pub viewed: bool,
    pub viewed_at: Option<Timestamp>,
    /// Generated at
    pub created_at: Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DigestType {
    Daily,
    Weekly,
    Monthly,
}

// ============================================================
// CARE TEAM TEMPLATES
// ============================================================

/// Pre-defined care team template for quick consent
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CareTeamTemplate {
    pub template_id: String,
    /// Name of the template
    pub name: String,
    /// Description for patients
    pub description: String,
    /// What this template allows
    pub permissions: Vec<DataPermission>,
    /// Data categories included
    pub data_categories: Vec<DataCategory>,
    /// Categories always excluded from template
    pub default_exclusions: Vec<DataCategory>,
    /// Purpose of access
    pub purpose: ConsentPurpose,
    /// How long consent lasts by default
    pub default_duration_days: Option<u32>,
    /// Is this a system template or user-created?
    pub template_type: TemplateType,
    /// Who created this template
    pub created_by: AgentPubKey,
    pub created_at: Timestamp,
    /// Is this template active?
    pub active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TemplateType {
    /// System-provided templates
    System,
    /// Organization-specific templates
    Organization(String),
    /// Patient-created personal templates
    Personal,
}

/// Pre-defined system templates
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SystemTemplate {
    /// Primary care team (PCP + nurses + staff)
    PrimaryCareTeam,
    /// Specialist referral (time-limited, category-specific)
    SpecialistReferral,
    /// Hospital admission (duration of stay + 30 days)
    HospitalAdmission,
    /// Emergency department (24-hour auto-expire)
    EmergencyDepartment,
    /// Mental health provider (enhanced privacy)
    MentalHealthProvider,
    /// Pharmacy (medications only)
    PharmacyAccess,
    /// Insurance/billing (financial only)
    InsuranceBilling,
    /// Clinical trial participation
    ClinicalTrial,
    /// Telehealth visit (single encounter)
    TelehealthVisit,
    /// Second opinion consultation
    SecondOpinion,
}

/// Care team membership
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CareTeam {
    pub team_id: String,
    pub patient_hash: ActionHash,
    pub team_name: String,
    /// Template used (if any)
    pub template_hash: Option<ActionHash>,
    /// Members of the care team
    pub members: Vec<CareTeamMember>,
    /// What the team can access
    pub permissions: Vec<DataPermission>,
    pub data_categories: Vec<DataCategory>,
    pub exclusions: Vec<DataCategory>,
    /// Purpose
    pub purpose: ConsentPurpose,
    /// Status
    pub status: CareTeamStatus,
    /// When team was formed
    pub created_at: Timestamp,
    /// When team access expires
    pub expires_at: Option<Timestamp>,
    /// Notes
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CareTeamMember {
    /// Provider or agent
    pub member: CareTeamMemberType,
    /// Role on the team
    pub role: CareTeamRole,
    /// When they joined
    pub joined_at: Timestamp,
    /// Are they currently active?
    pub active: bool,
    /// Any member-specific permission overrides
    pub permission_overrides: Option<Vec<DataPermission>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CareTeamMemberType {
    Provider(ActionHash),
    Organization(String),
    Agent(AgentPubKey),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CareTeamRole {
    PrimaryCarePhysician,
    Specialist,
    Nurse,
    NursePractitioner,
    PhysicianAssistant,
    Pharmacist,
    CaseManager,
    SocialWorker,
    Therapist,
    Dietitian,
    PhysicalTherapist,
    AdministrativeStaff,
    BillingSpecialist,
    Other(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CareTeamStatus {
    Active,
    Inactive,
    Dissolved,
    Expired,
}

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Consent(Consent),
    DataAccessRequest(DataAccessRequest),
    DataAccessLog(DataAccessLog),
    EmergencyAccess(EmergencyAccess),
    AuthorizationDocument(AuthorizationDocument),
    // Consent Delegation
    DelegationGrant(DelegationGrant),
    // Patient Notifications
    AccessNotification(AccessNotification),
    NotificationPreferences(NotificationPreferences),
    NotificationDigest(NotificationDigest),
    // Care Team Templates
    CareTeamTemplate(CareTeamTemplate),
    CareTeam(CareTeam),
}

#[hdk_link_types]
pub enum LinkTypes {
    PatientToConsents,
    PatientToAccessRequests,
    PatientToAccessLogs,
    ConsentToLogs,
    PatientToEmergencyAccess,
    PatientToDocuments,
    GranteeToConsents,
    ActiveConsents,
    RevokedConsents,
    ConsentUpdates,
    // Consent Delegation links
    PatientToDelegations,
    DelegateToDelegations,
    ActiveDelegations,
    // Patient Notification links
    PatientToNotifications,
    PatientToNotificationPreferences,
    PatientToDigests,
    UnreadNotifications,
    // Care Team links
    PatientToCareTeams,
    CareTeamToMembers,
    TemplateToTeams,
    SystemTemplates,
    ActiveCareTeams,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::Consent(c) => validate_consent(&c),
                EntryTypes::DataAccessRequest(r) => validate_access_request(&r),
                EntryTypes::DataAccessLog(l) => validate_access_log(&l),
                EntryTypes::EmergencyAccess(e) => validate_emergency_access(&e),
                EntryTypes::AuthorizationDocument(d) => validate_authorization(&d),
                EntryTypes::DelegationGrant(d) => validate_delegation_grant(&d),
                EntryTypes::AccessNotification(n) => validate_access_notification(&n),
                EntryTypes::NotificationPreferences(p) => validate_notification_preferences(&p),
                EntryTypes::NotificationDigest(d) => validate_notification_digest(&d),
                EntryTypes::CareTeamTemplate(t) => validate_care_team_template(&t),
                EntryTypes::CareTeam(t) => validate_care_team(&t),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_consent(consent: &Consent) -> ExternResult<ValidateCallbackResult> {
    if consent.consent_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Consent ID is required".to_string(),
        ));
    }
    if consent.permissions.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one permission must be granted".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_access_request(request: &DataAccessRequest) -> ExternResult<ValidateCallbackResult> {
    if request.request_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Request ID is required".to_string(),
        ));
    }
    if request.justification.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Justification is required for data access requests".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_access_log(log: &DataAccessLog) -> ExternResult<ValidateCallbackResult> {
    if log.log_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Log ID is required".to_string(),
        ));
    }
    if log.emergency_override && log.override_reason.is_none() {
        return Ok(ValidateCallbackResult::Invalid(
            "Override reason is required for emergency access".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_emergency_access(emergency: &EmergencyAccess) -> ExternResult<ValidateCallbackResult> {
    if emergency.emergency_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Emergency ID is required".to_string(),
        ));
    }
    if emergency.reason.is_empty() || emergency.clinical_justification.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Emergency access requires reason and clinical justification".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_authorization(doc: &AuthorizationDocument) -> ExternResult<ValidateCallbackResult> {
    if doc.document_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Document ID is required".to_string(),
        ));
    }
    if doc.patient_signature.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Patient signature is required".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

// ============================================================
// VALIDATION: CONSENT DELEGATION
// ============================================================

fn validate_delegation_grant(delegation: &DelegationGrant) -> ExternResult<ValidateCallbackResult> {
    if delegation.delegation_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Delegation ID is required".to_string(),
        ));
    }
    if delegation.permissions.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one permission must be granted".to_string(),
        ));
    }
    if delegation.data_scope.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Data scope must specify at least one category".to_string(),
        ));
    }
    // Healthcare proxy and legal guardian require identity verification
    if matches!(
        delegation.delegation_type,
        DelegationType::HealthcareProxy | DelegationType::LegalGuardian
    ) {
        if !delegation.identity_verified {
            return Ok(ValidateCallbackResult::Invalid(
                "Healthcare proxy and legal guardian delegations require identity verification"
                    .to_string(),
            ));
        }
        if delegation.legal_document_hash.is_none() {
            return Ok(ValidateCallbackResult::Invalid(
                "Healthcare proxy and legal guardian require legal documentation".to_string(),
            ));
        }
    }
    // Temporary delegation must have expiration
    if matches!(delegation.delegation_type, DelegationType::Temporary)
        && delegation.expires_at.is_none() {
            return Ok(ValidateCallbackResult::Invalid(
                "Temporary delegations must have an expiration date".to_string(),
            ));
        }
    Ok(ValidateCallbackResult::Valid)
}

// ============================================================
// VALIDATION: PATIENT NOTIFICATIONS
// ============================================================

fn validate_access_notification(
    notification: &AccessNotification,
) -> ExternResult<ValidateCallbackResult> {
    if notification.notification_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Notification ID is required".to_string(),
        ));
    }
    if notification.accessor_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Accessor name is required for patient-friendly notifications".to_string(),
        ));
    }
    if notification.summary.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Plain language summary is required".to_string(),
        ));
    }
    if notification.data_categories.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Data categories accessed must be specified".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_notification_preferences(
    prefs: &NotificationPreferences,
) -> ExternResult<ValidateCallbackResult> {
    // Validate daily digest hour
    if let Some(hour) = prefs.daily_digest_hour {
        if hour > 23 {
            return Ok(ValidateCallbackResult::Invalid(
                "Daily digest hour must be 0-23".to_string(),
            ));
        }
    }
    // Validate weekly summary day
    if let Some(day) = prefs.weekly_summary_day {
        if day > 6 {
            return Ok(ValidateCallbackResult::Invalid(
                "Weekly summary day must be 0-6 (Sunday-Saturday)".to_string(),
            ));
        }
    }
    // If email enabled, email address required
    if prefs.email_enabled && prefs.email_address.is_none() {
        return Ok(ValidateCallbackResult::Invalid(
            "Email address required when email notifications enabled".to_string(),
        ));
    }
    // If SMS enabled, phone number required
    if prefs.sms_enabled && prefs.phone_number.is_none() {
        return Ok(ValidateCallbackResult::Invalid(
            "Phone number required when SMS notifications enabled".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_notification_digest(
    digest: &NotificationDigest,
) -> ExternResult<ValidateCallbackResult> {
    if digest.digest_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Digest ID is required".to_string(),
        ));
    }
    // Period end must be after period start
    if digest.period_end.as_micros() <= digest.period_start.as_micros() {
        return Ok(ValidateCallbackResult::Invalid(
            "Period end must be after period start".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

// ============================================================
// VALIDATION: CARE TEAM TEMPLATES
// ============================================================

fn validate_care_team_template(
    template: &CareTeamTemplate,
) -> ExternResult<ValidateCallbackResult> {
    if template.template_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Template ID is required".to_string(),
        ));
    }
    if template.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Template name is required".to_string(),
        ));
    }
    if template.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Template description is required for patient understanding".to_string(),
        ));
    }
    if template.permissions.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Template must specify at least one permission".to_string(),
        ));
    }
    if template.data_categories.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Template must specify at least one data category".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_care_team(team: &CareTeam) -> ExternResult<ValidateCallbackResult> {
    if team.team_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Team ID is required".to_string(),
        ));
    }
    if team.team_name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Team name is required".to_string(),
        ));
    }
    if team.members.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Care team must have at least one member".to_string(),
        ));
    }
    if team.permissions.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Care team must have at least one permission".to_string(),
        ));
    }
    if team.data_categories.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Care team must specify data categories".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
