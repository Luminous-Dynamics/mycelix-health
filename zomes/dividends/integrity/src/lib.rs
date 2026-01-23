//! Data Dividends Integrity Zome
//!
//! Entry types and validation for the Health Data Dividends system that ensures
//! patients share in the value created when their health data contributes to:
//! - Research publications
//! - Drug development
//! - AI/ML model training
//! - Population health insights
//!
//! Key Principles:
//! - Transparent attribution chain
//! - Smart contract-encoded sharing agreements
//! - Democratic governance of data commons
//! - Individual veto power over data use

use hdi::prelude::*;

/// Define the entry types for the data dividends zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    /// Data contribution record
    DataContribution(DataContribution),
    /// Usage tracking for contributed data
    DataUsage(DataUsage),
    /// Dividend distribution record
    DividendDistribution(DividendDistribution),
    /// Revenue/value event
    RevenueEvent(RevenueEvent),
    /// Patient's dividend preferences
    DividendPreferences(DividendPreferences),
    /// Research project using patient data
    ResearchProject(ResearchProject),
    /// Attribution chain entry
    AttributionChain(AttributionChain),
    /// Dividend pool (collective fund)
    DividendPool(DividendPool),
}

/// Link types for the data dividends zome
#[hdk_link_types]
pub enum LinkTypes {
    PatientToContributions,
    PatientToUsages,
    PatientToDividends,
    PatientToPreferences,
    ProjectToContributions,
    ProjectToUsages,
    ContributionToUsages,
    UsageToRevenue,
    RevenueToDistributions,
    ActiveProjects,
    DividendPools,
    AttributionChains,
}

// ==================== DATA CONTRIBUTIONS ====================

/// Record of a patient's data contribution
#[hdk_entry_helper]
#[derive(Clone)]
pub struct DataContribution {
    /// Unique contribution ID
    pub contribution_id: String,
    /// Patient who contributed
    pub patient_hash: ActionHash,
    /// Type of data contributed
    pub data_type: ContributedDataType,
    /// Categories of data included
    pub data_categories: Vec<DataContributionCategory>,
    /// Hash of the contributed data (not the data itself)
    pub data_hash: [u8; 32],
    /// Size of contribution (records, samples, etc.)
    pub contribution_size: ContributionSize,
    /// Quality score of contribution
    pub quality_score: f32,
    /// Consent reference
    pub consent_hash: ActionHash,
    /// Usage permissions
    pub permitted_uses: Vec<PermittedUse>,
    /// Prohibited uses
    pub prohibited_uses: Vec<ProhibitedUse>,
    /// Contribution timestamp
    pub contributed_at: i64,
    /// Valid until (None = perpetual)
    pub valid_until: Option<i64>,
    /// Whether contribution has been revoked
    pub revoked: bool,
    /// Revocation timestamp
    pub revoked_at: Option<i64>,
}

/// Types of contributed data
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ContributedDataType {
    /// Longitudinal health records
    HealthRecords,
    /// Lab results
    LabResults,
    /// Genomic data
    GenomicData,
    /// Imaging data
    ImagingData,
    /// Wearable device data
    WearableData,
    /// Patient-reported outcomes
    PatientReported,
    /// Treatment outcomes
    TreatmentOutcomes,
    /// Biomarker data
    BiomarkerData,
    /// Aggregated/derived data
    DerivedData,
}

/// Categories of contributed data
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DataContributionCategory {
    Demographics,
    Diagnoses,
    Medications,
    Procedures,
    LabResults,
    VitalSigns,
    Immunizations,
    Allergies,
    FamilyHistory,
    SocialHistory,
    MentalHealth,
    Genomics,
    Imaging,
    Outcomes,
}

/// Size/volume of contribution
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContributionSize {
    /// Number of records
    pub record_count: u64,
    /// Time span (days)
    pub time_span_days: u32,
    /// Data points
    pub data_point_count: u64,
    /// Size in bytes
    pub size_bytes: Option<u64>,
}

/// Permitted use of data
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PermittedUse {
    /// Academic research
    AcademicResearch,
    /// Commercial research
    CommercialResearch,
    /// Drug development
    DrugDevelopment,
    /// AI/ML training
    AITraining,
    /// Public health analysis
    PublicHealth,
    /// Quality improvement
    QualityImprovement,
    /// Population health
    PopulationHealth,
    /// Disease surveillance
    DiseaseSurveillance,
    /// Clinical decision support
    ClinicalDecisionSupport,
}

/// Prohibited use of data
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ProhibitedUse {
    /// Marketing
    Marketing,
    /// Insurance underwriting
    InsuranceUnderwriting,
    /// Employment decisions
    EmploymentDecisions,
    /// Law enforcement
    LawEnforcement,
    /// Re-identification attempts
    ReIdentification,
    /// Sale of raw data
    DataSale,
    /// Weapons development
    WeaponsDevelopment,
}

// ==================== DATA USAGE ====================

/// Record of how contributed data was used
#[hdk_entry_helper]
#[derive(Clone)]
pub struct DataUsage {
    /// Unique usage ID
    pub usage_id: String,
    /// Contribution being used
    pub contribution_hash: ActionHash,
    /// Project using the data
    pub project_hash: ActionHash,
    /// Type of usage
    pub usage_type: UsageType,
    /// How the data was used (description)
    pub usage_description: String,
    /// When usage started
    pub started_at: i64,
    /// When usage ended (None = ongoing)
    pub ended_at: Option<i64>,
    /// Impact metrics
    pub impact_metrics: Option<ImpactMetrics>,
    /// Whether this generated revenue
    pub revenue_generating: bool,
    /// Associated revenue event
    pub revenue_hash: Option<ActionHash>,
}

/// Types of data usage
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum UsageType {
    /// Training an AI model
    ModelTraining,
    /// Research analysis
    ResearchAnalysis,
    /// Drug discovery
    DrugDiscovery,
    /// Clinical trial
    ClinicalTrial,
    /// Publication/paper
    Publication,
    /// Population study
    PopulationStudy,
    /// Quality metric
    QualityMetric,
}

/// Impact metrics from data usage
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImpactMetrics {
    /// Contribution to outcome (0.0 - 1.0)
    pub contribution_weight: f32,
    /// Citations (for publications)
    pub citations: Option<u32>,
    /// Patients benefited (estimated)
    pub patients_benefited: Option<u64>,
    /// Cost savings generated
    pub cost_savings: Option<f64>,
    /// Quality of evidence level
    pub evidence_level: Option<EvidenceLevel>,
}

/// Evidence quality levels
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum EvidenceLevel {
    /// Systematic review
    Level1,
    /// RCT
    Level2,
    /// Cohort study
    Level3,
    /// Case-control
    Level4,
    /// Case series
    Level5,
    /// Expert opinion
    Level6,
}

// ==================== DIVIDENDS ====================

/// Dividend distribution to a patient
#[hdk_entry_helper]
#[derive(Clone)]
pub struct DividendDistribution {
    /// Unique distribution ID
    pub distribution_id: String,
    /// Patient receiving dividend
    pub patient_hash: ActionHash,
    /// Revenue event this is from
    pub revenue_hash: ActionHash,
    /// Contribution this is for
    pub contribution_hash: ActionHash,
    /// Amount distributed
    pub amount: DividendAmount,
    /// Calculation breakdown
    pub calculation: DividendCalculation,
    /// Distribution status
    pub status: DistributionStatus,
    /// Distributed at
    pub distributed_at: i64,
    /// Claimed at (if applicable)
    pub claimed_at: Option<i64>,
}

/// Dividend amount
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DividendAmount {
    /// Amount
    pub value: f64,
    /// Currency/token
    pub currency: DividendCurrency,
    /// USD equivalent (for display)
    pub usd_equivalent: Option<f64>,
}

/// Dividend currency types
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DividendCurrency {
    /// Fiat currency
    Fiat(String), // "USD", "EUR", etc.
    /// Cryptocurrency
    Crypto(String), // "ETH", "BTC", etc.
    /// Health data token
    HealthToken,
    /// Research credits
    ResearchCredits,
    /// In-kind benefit (e.g., free healthcare)
    InKindBenefit,
}

/// How dividend was calculated
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DividendCalculation {
    /// Total revenue from event
    pub total_revenue: f64,
    /// Patient pool share percentage
    pub patient_pool_percent: f32,
    /// This patient's contribution weight
    pub contribution_weight: f32,
    /// Resulting share
    pub calculated_share: f64,
    /// Deductions (if any)
    pub deductions: Vec<Deduction>,
    /// Final amount
    pub final_amount: f64,
}

/// Deduction from dividend
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Deduction {
    /// Deduction type
    pub deduction_type: String,
    /// Amount
    pub amount: f64,
    /// Reason
    pub reason: String,
}

/// Distribution status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DistributionStatus {
    /// Calculated but not distributed
    Pending,
    /// Distribution initiated
    Initiated,
    /// Successfully distributed
    Distributed,
    /// Claimed by patient
    Claimed,
    /// Failed to distribute
    Failed,
    /// Returned to pool (unclaimed)
    ReturnedToPool,
}

// ==================== REVENUE EVENTS ====================

/// Revenue or value generation event
#[hdk_entry_helper]
#[derive(Clone)]
pub struct RevenueEvent {
    /// Unique event ID
    pub event_id: String,
    /// Project that generated revenue
    pub project_hash: ActionHash,
    /// Type of revenue
    pub revenue_type: RevenueType,
    /// Total revenue/value
    pub total_value: f64,
    /// Currency
    pub currency: DividendCurrency,
    /// Description
    pub description: String,
    /// Contributions that contributed
    pub contributing_data: Vec<ActionHash>,
    /// Patient pool allocation percentage
    pub patient_pool_percent: f32,
    /// Event timestamp
    pub event_at: i64,
    /// Distributions created
    pub distributions: Vec<ActionHash>,
    /// Status
    pub status: RevenueEventStatus,
}

/// Types of revenue events
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RevenueType {
    /// Drug sales royalty
    DrugRoyalty,
    /// Publication citation reward
    PublicationReward,
    /// AI model licensing
    ModelLicensing,
    /// Data access fee
    DataAccessFee,
    /// Grant funding
    GrantFunding,
    /// Insurance savings share
    InsuranceSavings,
    /// Quality bonus
    QualityBonus,
    /// Research milestone
    ResearchMilestone,
}

/// Revenue event status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RevenueEventStatus {
    /// Event recorded
    Recorded,
    /// Calculating distributions
    Calculating,
    /// Distributions created
    DistributionsCreated,
    /// Fully distributed
    Distributed,
    /// Disputed
    Disputed,
}

// ==================== DIVIDEND PREFERENCES ====================

/// Patient's dividend preferences
#[hdk_entry_helper]
#[derive(Clone)]
pub struct DividendPreferences {
    /// Patient
    pub patient_hash: ActionHash,
    /// Preferred currency for dividends
    pub preferred_currency: DividendCurrency,
    /// Minimum amount to distribute (aggregate smaller)
    pub minimum_distribution: f64,
    /// Auto-donate percentage to research
    pub auto_donate_percent: f32,
    /// Charities to auto-donate to
    pub auto_donate_recipients: Vec<DonationRecipient>,
    /// Receive notifications for
    pub notification_threshold: f64,
    /// Payout method
    pub payout_method: PayoutMethod,
    /// Updated at
    pub updated_at: i64,
}

/// Donation recipient
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DonationRecipient {
    /// Recipient name
    pub name: String,
    /// Recipient type
    pub recipient_type: RecipientType,
    /// Percentage of donation
    pub percentage: f32,
}

/// Types of donation recipients
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RecipientType {
    /// Medical research organization
    ResearchOrg,
    /// Disease-specific charity
    DiseaseCharity,
    /// Patient advocacy group
    PatientAdvocacy,
    /// Data commons fund
    DataCommons,
    /// Other nonprofit
    Nonprofit,
}

/// Payout method
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PayoutMethod {
    /// Bank transfer
    BankTransfer,
    /// Cryptocurrency wallet
    CryptoWallet,
    /// Healthcare credits
    HealthcareCredits,
    /// Research participation credits
    ResearchCredits,
    /// Charitable donation
    CharitableDonation,
    /// Hold in account
    HoldInAccount,
}

// ==================== RESEARCH PROJECTS ====================

/// Research project using patient data
#[hdk_entry_helper]
#[derive(Clone)]
pub struct ResearchProject {
    /// Unique project ID
    pub project_id: String,
    /// Project name
    pub name: String,
    /// Description
    pub description: String,
    /// Organization conducting research
    pub organization: String,
    /// Project type
    pub project_type: ProjectType,
    /// Start date
    pub start_date: i64,
    /// Expected end date
    pub expected_end_date: Option<i64>,
    /// Actual end date
    pub actual_end_date: Option<i64>,
    /// IRB/Ethics approval reference
    pub ethics_approval: Option<String>,
    /// Revenue sharing terms
    pub revenue_sharing: RevenueSharingTerms,
    /// Status
    pub status: ProjectStatus,
    /// Number of patients contributed
    pub patient_count: u64,
    /// Publications from project
    pub publications: Vec<Publication>,
    /// Revenue events from project
    pub revenue_events: Vec<ActionHash>,
}

/// Types of research projects
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ProjectType {
    /// Academic research
    AcademicResearch,
    /// Commercial R&D
    CommercialRD,
    /// Drug development
    DrugDevelopment,
    /// AI/ML development
    AIDevelopment,
    /// Public health study
    PublicHealth,
    /// Quality improvement
    QualityImprovement,
}

/// Revenue sharing terms
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RevenueSharingTerms {
    /// Patient pool percentage
    pub patient_pool_percent: f32,
    /// Distribution method
    pub distribution_method: DistributionMethod,
    /// Minimum payout
    pub minimum_payout: f64,
    /// Currency
    pub currency: DividendCurrency,
    /// Cap on patient earnings (if any)
    pub earnings_cap: Option<f64>,
}

/// How dividends are distributed
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DistributionMethod {
    /// Equal share among all contributors
    EqualShare,
    /// Weighted by contribution size
    WeightedBySize,
    /// Weighted by data quality
    WeightedByQuality,
    /// Weighted by usage impact
    WeightedByImpact,
    /// Combination
    Hybrid,
}

/// Project status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ProjectStatus {
    /// Recruiting participants
    Recruiting,
    /// Active data collection
    Active,
    /// Analysis phase
    Analysis,
    /// Publication phase
    Publication,
    /// Commercialization
    Commercialization,
    /// Completed
    Completed,
    /// Terminated
    Terminated,
}

/// Publication from research
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Publication {
    /// DOI or identifier
    pub identifier: String,
    /// Title
    pub title: String,
    /// Journal/venue
    pub venue: String,
    /// Publication date
    pub published_at: i64,
    /// Citation count
    pub citations: u32,
    /// Open access
    pub open_access: bool,
}

// ==================== ATTRIBUTION CHAIN ====================

/// Attribution chain for tracking data lineage
#[hdk_entry_helper]
#[derive(Clone)]
pub struct AttributionChain {
    /// Unique chain ID
    pub chain_id: String,
    /// Original data source (patient)
    pub original_source: ActionHash,
    /// Chain of transformations/uses
    pub chain: Vec<AttributionLink>,
    /// Final outcome (if known)
    pub final_outcome: Option<ChainOutcome>,
    /// Created at
    pub created_at: i64,
    /// Last updated
    pub updated_at: i64,
}

/// Link in the attribution chain
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttributionLink {
    /// Step number
    pub step: u32,
    /// Type of transformation/use
    pub link_type: AttributionLinkType,
    /// Actor (researcher, organization, etc.)
    pub actor: String,
    /// Description
    pub description: String,
    /// Timestamp
    pub timestamp: i64,
    /// Hash of related record
    pub record_hash: Option<ActionHash>,
}

/// Types of attribution links
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AttributionLinkType {
    /// Original contribution
    Contribution,
    /// Data aggregation
    Aggregation,
    /// Analysis/computation
    Analysis,
    /// Model training
    ModelTraining,
    /// Publication
    Publication,
    /// Product development
    ProductDevelopment,
    /// Revenue generation
    RevenueGeneration,
}

/// Final outcome of attribution chain
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChainOutcome {
    /// Outcome type
    pub outcome_type: OutcomeType,
    /// Description
    pub description: String,
    /// Value generated
    pub value_generated: Option<f64>,
    /// Patients benefited
    pub patients_benefited: Option<u64>,
}

/// Types of outcomes
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum OutcomeType {
    /// Published research
    Publication,
    /// Approved drug/treatment
    ApprovedTreatment,
    /// AI model in production
    AIModel,
    /// Policy change
    PolicyChange,
    /// Quality improvement
    QualityImprovement,
    /// Cost savings
    CostSavings,
}

// ==================== DIVIDEND POOL ====================

/// Collective dividend pool
#[hdk_entry_helper]
#[derive(Clone)]
pub struct DividendPool {
    /// Unique pool ID
    pub pool_id: String,
    /// Pool name
    pub name: String,
    /// Pool type
    pub pool_type: PoolType,
    /// Current balance
    pub balance: f64,
    /// Currency
    pub currency: DividendCurrency,
    /// Number of beneficiaries
    pub beneficiary_count: u64,
    /// Total distributed to date
    pub total_distributed: f64,
    /// Created at
    pub created_at: i64,
    /// Last distribution at
    pub last_distribution_at: Option<i64>,
}

/// Types of pools
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PoolType {
    /// General commons pool
    GeneralCommons,
    /// Disease-specific pool
    DiseaseSpecific(String),
    /// Research area pool
    ResearchArea(String),
    /// Geographic pool
    Geographic(String),
    /// Organization-specific
    Organizational(String),
}

// ==================== VALIDATION ====================

/// Validate a data contribution
pub fn validate_data_contribution(
    contribution: &DataContribution,
) -> ExternResult<ValidateCallbackResult> {
    if contribution.contribution_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Contribution ID required".to_string(),
        ));
    }

    if contribution.data_categories.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one data category required".to_string(),
        ));
    }

    if contribution.quality_score < 0.0 || contribution.quality_score > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Quality score must be between 0 and 1".to_string(),
        ));
    }

    if contribution.permitted_uses.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one permitted use required".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate a dividend distribution
pub fn validate_dividend_distribution(
    dist: &DividendDistribution,
) -> ExternResult<ValidateCallbackResult> {
    if dist.distribution_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Distribution ID required".to_string(),
        ));
    }

    if dist.amount.value < 0.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Amount cannot be negative".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate a revenue event
pub fn validate_revenue_event(event: &RevenueEvent) -> ExternResult<ValidateCallbackResult> {
    if event.event_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Event ID required".to_string(),
        ));
    }

    if event.total_value < 0.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Total value cannot be negative".to_string(),
        ));
    }

    if event.patient_pool_percent < 0.0 || event.patient_pool_percent > 100.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Patient pool percent must be 0-100".to_string(),
        ));
    }

    if event.contributing_data.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "At least one contribution required".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate a research project
pub fn validate_research_project(
    project: &ResearchProject,
) -> ExternResult<ValidateCallbackResult> {
    if project.project_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Project ID required".to_string(),
        ));
    }

    if project.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Project name required".to_string(),
        ));
    }

    if project.organization.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Organization required".to_string(),
        ));
    }

    let terms = &project.revenue_sharing;
    if terms.patient_pool_percent < 0.0 || terms.patient_pool_percent > 100.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Patient pool percent must be 0-100".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate dividend preferences
pub fn validate_dividend_preferences(
    prefs: &DividendPreferences,
) -> ExternResult<ValidateCallbackResult> {
    if prefs.auto_donate_percent < 0.0 || prefs.auto_donate_percent > 100.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Auto-donate percent must be 0-100".to_string(),
        ));
    }

    if prefs.minimum_distribution < 0.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Minimum distribution cannot be negative".to_string(),
        ));
    }

    // Verify donation recipients sum to 100% if any exist
    if !prefs.auto_donate_recipients.is_empty() {
        let total: f32 = prefs
            .auto_donate_recipients
            .iter()
            .map(|r| r.percentage)
            .sum();
        if (total - 100.0).abs() > 0.01 {
            return Ok(ValidateCallbackResult::Invalid(
                "Donation recipient percentages must sum to 100".to_string(),
            ));
        }
    }

    Ok(ValidateCallbackResult::Valid)
}
