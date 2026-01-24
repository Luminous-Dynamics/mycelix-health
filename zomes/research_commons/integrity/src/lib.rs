//! Research Commons Integrity Zome
//!
//! Defines entry types for de-identified data sharing, research datasets,
//! and data access agreements following HIPAA Safe Harbor guidelines.

use hdi::prelude::*;

/// De-identification method used
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DeidentificationMethod {
    /// HIPAA Safe Harbor - 18 identifiers removed
    SafeHarbor,
    /// HIPAA Expert Determination - statistical analysis
    ExpertDetermination,
    /// Limited Dataset - dates and geographic info retained
    LimitedDataset,
    /// Synthetic data generation
    Synthetic,
    /// K-anonymity with specified k value
    KAnonymity { k: u32 },
    /// L-diversity with specified l value
    LDiversity { l: u32 },
    /// T-closeness with specified t value
    TCloseness { t: f64 },
    /// Differential privacy with epsilon value
    DifferentialPrivacy { epsilon: f64 },
}

/// Data category available in the commons
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ResearchDataCategory {
    Demographics,
    Diagnoses,
    Procedures,
    Medications,
    LabResults,
    VitalSigns,
    Imaging,
    Genomics,
    Outcomes,
    SocialDeterminants,
    MentalHealth,
    Behavioral,
}

/// Access level for research data
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DataAccessLevel {
    /// Public access - fully de-identified
    Public,
    /// Registered researcher access
    Registered,
    /// Approved project access
    Approved,
    /// Restricted - requires special approval
    Restricted,
}

/// Status of a dataset in the commons
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DatasetStatus {
    /// Draft - being prepared
    Draft,
    /// Under review for compliance
    UnderReview,
    /// Available for access
    Available,
    /// Temporarily suspended
    Suspended,
    /// Permanently retired
    Retired,
}

/// Status of a data access request
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AccessRequestStatus {
    Submitted,
    UnderReview,
    Approved,
    Denied,
    Expired,
    Revoked,
}

/// A de-identified dataset available in the research commons
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ResearchDataset {
    /// Unique identifier for the dataset
    pub dataset_id: String,
    /// Human-readable title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Data categories included
    pub categories: Vec<ResearchDataCategory>,
    /// De-identification method used
    pub deidentification_method: DeidentificationMethod,
    /// Access level required
    pub access_level: DataAccessLevel,
    /// Current status
    pub status: DatasetStatus,
    /// Number of records in dataset
    pub record_count: u64,
    /// Date range of data (start, end as YYYYMMDD)
    pub date_range: Option<(u32, u32)>,
    /// Geographic scope (e.g., "US", "Northeast US", "California")
    pub geographic_scope: Option<String>,
    /// Age range of subjects
    pub age_range: Option<(u32, u32)>,
    /// ICD-10 codes for conditions included
    pub condition_codes: Vec<String>,
    /// Data custodian/owner
    pub custodian_hash: ActionHash,
    /// IRB approval reference (if applicable)
    pub irb_approval_hash: Option<ActionHash>,
    /// Terms of use document hash
    pub terms_hash: Option<ActionHash>,
    /// Citation requirements
    pub citation: String,
    /// DOI if published
    pub doi: Option<String>,
    /// Creation timestamp
    pub created_at: Timestamp,
    /// Last updated timestamp
    pub updated_at: Timestamp,
}

/// Data access agreement for using a dataset
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DataAccessAgreement {
    /// Unique agreement ID
    pub agreement_id: String,
    /// Dataset being accessed
    pub dataset_hash: ActionHash,
    /// Researcher requesting access
    pub researcher_hash: ActionHash,
    /// Institution of the researcher
    pub institution: String,
    /// Research project title
    pub project_title: String,
    /// Research purpose description
    pub purpose: String,
    /// Planned analyses
    pub planned_analyses: Vec<String>,
    /// Expected outputs (publications, etc.)
    pub expected_outputs: Vec<String>,
    /// Current status
    pub status: AccessRequestStatus,
    /// Approval date (if approved)
    pub approved_at: Option<Timestamp>,
    /// Approving authority
    pub approved_by: Option<ActionHash>,
    /// Access expiration date
    pub expires_at: Option<Timestamp>,
    /// Special conditions
    pub conditions: Vec<String>,
    /// Whether re-identification attempts are prohibited
    pub no_reidentification_pledge: bool,
    /// Whether data can be shared with collaborators
    pub sharing_allowed: bool,
    /// Maximum collaborators if sharing allowed
    pub max_collaborators: Option<u32>,
    /// Submission timestamp
    pub submitted_at: Timestamp,
}

/// Record of data access for audit
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DataAccessLog {
    /// Agreement under which access occurred
    pub agreement_hash: ActionHash,
    /// Dataset accessed
    pub dataset_hash: ActionHash,
    /// Researcher who accessed
    pub researcher_hash: ActionHash,
    /// Type of access
    pub access_type: String,
    /// Query or operation performed
    pub operation: String,
    /// Number of records accessed
    pub records_accessed: u64,
    /// Access timestamp
    pub accessed_at: Timestamp,
    /// IP hash (for audit, not identification)
    pub ip_hash: Option<String>,
}

/// A contribution of de-identified data to the commons
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DataContribution {
    /// Contribution ID
    pub contribution_id: String,
    /// Original patient hash (for dividends, encrypted)
    pub patient_hash_encrypted: Vec<u8>,
    /// Dataset this contributes to
    pub dataset_hash: ActionHash,
    /// Categories contributed
    pub categories: Vec<ResearchDataCategory>,
    /// Number of records contributed
    pub record_count: u64,
    /// Consent hash authorizing contribution
    pub consent_hash: ActionHash,
    /// Whether eligible for data dividends
    pub dividend_eligible: bool,
    /// Contribution timestamp
    pub contributed_at: Timestamp,
}

/// Data quality metrics for a dataset
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DataQualityReport {
    /// Dataset being assessed
    pub dataset_hash: ActionHash,
    /// Overall quality score (0-100)
    pub quality_score: u32,
    /// Completeness percentage
    pub completeness: f64,
    /// Accuracy assessment
    pub accuracy_score: Option<f64>,
    /// Consistency score
    pub consistency_score: f64,
    /// Timeliness (how recent the data is)
    pub timeliness_score: f64,
    /// Specific quality issues found
    pub issues: Vec<String>,
    /// Recommendations for improvement
    pub recommendations: Vec<String>,
    /// Assessment date
    pub assessed_at: Timestamp,
    /// Assessor
    pub assessed_by: ActionHash,
}

/// Entry types for the research commons zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    ResearchDataset(ResearchDataset),
    DataAccessAgreement(DataAccessAgreement),
    DataAccessLog(DataAccessLog),
    DataContribution(DataContribution),
    DataQualityReport(DataQualityReport),
}

/// Link types for the research commons zome
#[hdk_link_types]
pub enum LinkTypes {
    /// All available datasets
    AllDatasets,
    /// Datasets by category
    DatasetsByCategory,
    /// Datasets by access level
    DatasetsByAccessLevel,
    /// Dataset to access agreements
    DatasetToAgreements,
    /// Researcher to their agreements
    ResearcherToAgreements,
    /// Dataset to access logs
    DatasetToAccessLogs,
    /// Dataset to contributions
    DatasetToContributions,
    /// Patient to their contributions
    PatientToContributions,
    /// Dataset to quality reports
    DatasetToQualityReports,
}

/// Validation callbacks
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::ResearchDataset(dataset) => validate_dataset(&dataset),
                EntryTypes::DataAccessAgreement(agreement) => validate_agreement(&agreement),
                EntryTypes::DataAccessLog(log) => validate_access_log(&log),
                EntryTypes::DataContribution(contribution) => validate_contribution(&contribution),
                EntryTypes::DataQualityReport(report) => validate_quality_report(&report),
            },
            OpEntry::UpdateEntry { app_entry, .. } => match app_entry {
                EntryTypes::ResearchDataset(dataset) => validate_dataset(&dataset),
                EntryTypes::DataAccessAgreement(agreement) => validate_agreement(&agreement),
                _ => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterCreateLink { .. } => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterDeleteLink { .. } => Ok(ValidateCallbackResult::Valid),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_dataset(dataset: &ResearchDataset) -> ExternResult<ValidateCallbackResult> {
    if dataset.dataset_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Dataset ID is required".to_string()));
    }
    if dataset.title.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Dataset title is required".to_string()));
    }
    if dataset.categories.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("At least one data category is required".to_string()));
    }
    if dataset.record_count == 0 {
        return Ok(ValidateCallbackResult::Invalid("Dataset must contain records".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_agreement(agreement: &DataAccessAgreement) -> ExternResult<ValidateCallbackResult> {
    if agreement.agreement_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Agreement ID is required".to_string()));
    }
    if agreement.project_title.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Project title is required".to_string()));
    }
    if agreement.purpose.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Research purpose is required".to_string()));
    }
    if !agreement.no_reidentification_pledge {
        return Ok(ValidateCallbackResult::Invalid("Re-identification prohibition pledge is required".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_access_log(log: &DataAccessLog) -> ExternResult<ValidateCallbackResult> {
    if log.operation.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Operation description is required".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_contribution(contribution: &DataContribution) -> ExternResult<ValidateCallbackResult> {
    if contribution.contribution_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Contribution ID is required".to_string()));
    }
    if contribution.categories.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("At least one data category is required".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_quality_report(report: &DataQualityReport) -> ExternResult<ValidateCallbackResult> {
    if report.quality_score > 100 {
        return Ok(ValidateCallbackResult::Invalid("Quality score must be 0-100".to_string()));
    }
    if report.completeness < 0.0 || report.completeness > 1.0 {
        return Ok(ValidateCallbackResult::Invalid("Completeness must be 0.0-1.0".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}
