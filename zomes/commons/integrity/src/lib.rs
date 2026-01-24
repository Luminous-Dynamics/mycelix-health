//! Health as a Commons Integrity Zome
//!
//! Enables privacy-preserving collective health insights through:
//! - Differential privacy for aggregate queries
//! - Privacy-preserving data contribution
//! - Democratic governance of health data commons
//! - Collective benefit from shared health knowledge
//!
//! Key Principles:
//! - Individual privacy protected via differential privacy
//! - Collective insights benefit all
//! - Democratic governance of data commons
//! - No re-identification possible from outputs

use hdi::prelude::*;

/// Define the entry types for the health commons zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    /// Data pool configuration
    DataPool(DataPool),
    /// Privacy-preserving contribution
    PrivacyContribution(PrivacyContribution),
    /// Aggregate query request
    AggregateQuery(AggregateQuery),
    /// Query result with privacy guarantees
    QueryResult(QueryResult),
    /// Governance proposal for data commons
    GovernanceProposal(GovernanceProposal),
    /// Vote on governance proposal
    GovernanceVote(GovernanceVote),
    /// Privacy budget allocation
    PrivacyBudget(PrivacyBudget),
    /// Collective insight (derived knowledge)
    CollectiveInsight(CollectiveInsight),
    /// Privacy budget ledger entry for formal DP tracking
    BudgetLedgerEntry(BudgetLedgerEntry),
}

/// Link types for the health commons zome
#[hdk_link_types]
pub enum LinkTypes {
    ActiveDataPools,
    PoolToContributions,
    PoolToQueries,
    ContributorToPools,
    QueryToResults,
    ActiveProposals,
    ProposalToVotes,
    PoolToInsights,
    PatientToBudgets,
    /// Links patient+pool combination to their budget ledger entry
    /// Base: hash(patient_hash + pool_hash), Target: BudgetLedgerEntry
    PatientPoolToBudgetLedger,
}

// ==================== DATA POOLS ====================

/// A collective data pool for health insights
#[hdk_entry_helper]
#[derive(Clone)]
pub struct DataPool {
    /// Unique pool ID
    pub pool_id: String,
    /// Pool name
    pub name: String,
    /// Description of pool purpose
    pub description: String,
    /// Type of data collected
    pub data_type: PoolDataType,
    /// Required data categories
    pub required_categories: Vec<PoolDataCategory>,
    /// Privacy parameters
    pub privacy_params: PrivacyParameters,
    /// Minimum contributors for query eligibility
    pub min_contributors: u32,
    /// Current contributor count
    pub contributor_count: u32,
    /// Governance model
    pub governance: GovernanceModel,
    /// Who can query this pool
    pub query_permissions: QueryPermissions,
    /// Pool status
    pub status: PoolStatus,
    /// Created at
    pub created_at: i64,
    /// Created by
    pub created_by: AgentPubKey,
}

/// Types of data pools
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PoolDataType {
    /// Disease prevalence and outcomes
    DiseaseOutcomes,
    /// Medication effectiveness
    MedicationEffectiveness,
    /// Treatment patterns
    TreatmentPatterns,
    /// Vital signs trends
    VitalSignsTrends,
    /// Lab result distributions
    LabDistributions,
    /// Geographic health patterns
    GeographicPatterns,
    /// Age-based health trends
    AgeTrends,
    /// Custom research pool
    CustomResearch(String),
}

/// Categories of data for pool contribution
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PoolDataCategory {
    Demographics,
    Diagnoses,
    Medications,
    Procedures,
    LabResults,
    VitalSigns,
    Outcomes,
    LifestyleFactors,
}

/// Differential privacy parameters
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrivacyParameters {
    /// Epsilon value for differential privacy (lower = more private)
    pub epsilon: f64,
    /// Delta value for relaxed DP (probability of privacy failure)
    pub delta: f64,
    /// Noise mechanism
    pub noise_mechanism: NoiseMechanism,
    /// Sensitivity bound for queries
    pub sensitivity_bound: f64,
    /// Minimum aggregation size (k-anonymity)
    pub min_aggregation: u32,
}

/// Noise mechanisms for differential privacy
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum NoiseMechanism {
    /// Laplace noise for counting queries
    Laplace,
    /// Gaussian noise for real-valued queries
    Gaussian,
    /// Exponential mechanism for categorical queries
    Exponential,
    /// Randomized response for yes/no queries
    RandomizedResponse,
}

/// Governance models for data pools
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GovernanceModel {
    /// Democratic - one contributor, one vote
    Democratic,
    /// Weighted by contribution size
    ContributionWeighted,
    /// Council of elected stewards
    StewardCouncil,
    /// Combination of above
    Hybrid,
}

/// Who can query the pool
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueryPermissions {
    /// Public researchers
    pub public_researchers: bool,
    /// Commercial entities
    pub commercial: bool,
    /// Government/public health
    pub government: bool,
    /// Other patients (for benchmarking)
    pub patients: bool,
    /// Specific approved entities
    pub approved_entities: Vec<AgentPubKey>,
}

/// Pool status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PoolStatus {
    /// Accepting contributions
    Active,
    /// Temporarily paused
    Paused,
    /// Closed to new contributions
    Closed,
    /// Archived (read-only)
    Archived,
}

// ==================== PRIVACY CONTRIBUTIONS ====================

/// Privacy-preserving contribution to a data pool
#[hdk_entry_helper]
#[derive(Clone)]
pub struct PrivacyContribution {
    /// Unique contribution ID
    pub contribution_id: String,
    /// Pool being contributed to
    pub pool_hash: ActionHash,
    /// Contributor (patient)
    pub contributor: ActionHash,
    /// Locally processed aggregate values (not raw data)
    /// These are already locally differentially private
    pub local_aggregates: Vec<LocalAggregate>,
    /// Privacy budget consumed
    pub budget_consumed: f64,
    /// Verification hash (proves contribution without revealing data)
    pub verification_hash: [u8; 32],
    /// Contributed at
    pub contributed_at: i64,
    /// Whether contribution can be withdrawn
    pub revocable: bool,
}

/// Locally computed aggregate (privacy-preserving)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalAggregate {
    /// Category of data
    pub category: PoolDataCategory,
    /// Aggregated value type
    pub value_type: AggregateValueType,
    /// The noisy aggregate value
    pub noisy_value: f64,
    /// Local noise added (for verification)
    pub local_noise_variance: f64,
}

/// Types of aggregated values
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AggregateValueType {
    /// Count (e.g., number of diagnoses)
    Count,
    /// Sum (e.g., total medication doses)
    Sum,
    /// Mean (e.g., average blood pressure)
    Mean,
    /// Variance (e.g., spread of values)
    Variance,
    /// Binary indicator (e.g., has condition)
    Binary,
    /// Histogram bucket
    HistogramBucket(String),
}

// ==================== AGGREGATE QUERIES ====================

/// Request for aggregate data from pool
#[hdk_entry_helper]
#[derive(Clone)]
pub struct AggregateQuery {
    /// Unique query ID
    pub query_id: String,
    /// Pool being queried
    pub pool_hash: ActionHash,
    /// Who is querying
    pub requester: AgentPubKey,
    /// Organization name
    pub organization: String,
    /// Purpose of query
    pub purpose: QueryPurpose,
    /// Query specification
    pub query_spec: QuerySpecification,
    /// Requested privacy level
    pub requested_epsilon: f64,
    /// Status
    pub status: QueryStatus,
    /// Requested at
    pub requested_at: i64,
    /// Approved at
    pub approved_at: Option<i64>,
    /// Executed at
    pub executed_at: Option<i64>,
}

/// Purpose of the query
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum QueryPurpose {
    /// Academic research
    AcademicResearch,
    /// Public health surveillance
    PublicHealth,
    /// Quality improvement
    QualityImprovement,
    /// Policy development
    PolicyDevelopment,
    /// Patient benchmarking
    PatientBenchmark,
    /// Commercial research
    CommercialResearch,
}

/// Query specification
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QuerySpecification {
    /// Type of query
    pub query_type: QueryType,
    /// Data categories needed
    pub categories: Vec<PoolDataCategory>,
    /// Filters to apply
    pub filters: Vec<QueryFilter>,
    /// Grouping dimensions
    pub group_by: Vec<String>,
    /// Time range
    pub time_range: Option<TimeRange>,
}

/// Types of aggregate queries
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum QueryType {
    /// Simple count
    Count,
    /// Sum of values
    Sum,
    /// Average
    Average,
    /// Percentiles
    Percentile(u8),
    /// Histogram
    Histogram,
    /// Correlation
    Correlation,
    /// Trend analysis
    Trend,
}

/// Query filter
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueryFilter {
    /// Field to filter
    pub field: String,
    /// Comparison operator
    pub operator: FilterOperator,
    /// Value to compare
    pub value: String,
}

/// Filter operators
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    Contains,
    InRange,
}

/// Time range for queries
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimeRange {
    pub start: i64,
    pub end: i64,
}

/// Query status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum QueryStatus {
    /// Awaiting governance approval
    Pending,
    /// Approved, waiting for execution
    Approved,
    /// Currently executing
    Executing,
    /// Completed successfully
    Completed,
    /// Rejected by governance
    Rejected,
    /// Failed during execution
    Failed,
}

// ==================== QUERY RESULTS ====================

/// Privacy-preserving query result
#[hdk_entry_helper]
#[derive(Clone)]
pub struct QueryResult {
    /// Unique result ID
    pub result_id: String,
    /// Query that produced this result
    pub query_hash: ActionHash,
    /// Pool queried
    pub pool_hash: ActionHash,
    /// Number of contributors in result
    pub contributor_count: u32,
    /// The differentially private result
    pub dp_result: DifferentiallyPrivateResult,
    /// Privacy parameters used
    pub actual_epsilon: f64,
    /// Confidence interval
    pub confidence_interval: Option<ConfidenceInterval>,
    /// Computed at
    pub computed_at: i64,
    /// Valid until (results may expire)
    pub valid_until: Option<i64>,
}

/// Differentially private result value
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DifferentiallyPrivateResult {
    /// Type of result
    pub result_type: ResultType,
    /// Noisy value(s)
    pub values: Vec<NoisyValue>,
    /// Total noise added
    pub noise_magnitude: f64,
    /// Whether k-anonymity threshold was met
    pub k_anonymity_met: bool,
}

/// Types of results
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ResultType {
    Scalar,
    Vector,
    Histogram,
    TimeSeries,
    Correlation,
}

/// A noisy value with its metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NoisyValue {
    /// Label (for grouped results)
    pub label: Option<String>,
    /// The noisy value
    pub value: f64,
    /// Estimated standard error
    pub standard_error: Option<f64>,
}

/// Confidence interval for result
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfidenceInterval {
    pub lower: f64,
    pub upper: f64,
    pub confidence_level: f64,
}

// ==================== GOVERNANCE ====================

/// Governance proposal for data pool
#[hdk_entry_helper]
#[derive(Clone)]
pub struct GovernanceProposal {
    /// Unique proposal ID
    pub proposal_id: String,
    /// Pool being governed
    pub pool_hash: ActionHash,
    /// Proposer
    pub proposer: ActionHash,
    /// Type of proposal
    pub proposal_type: ProposalType,
    /// Description
    pub description: String,
    /// Detailed specification
    pub specification: String,
    /// Voting period start
    pub voting_start: i64,
    /// Voting period end
    pub voting_end: i64,
    /// Required approval threshold (0.0 - 1.0)
    pub approval_threshold: f64,
    /// Current status
    pub status: ProposalStatus,
    /// Votes for
    pub votes_for: u32,
    /// Votes against
    pub votes_against: u32,
    /// Created at
    pub created_at: i64,
}

/// Types of governance proposals
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ProposalType {
    /// Change privacy parameters
    ChangePrivacyParams,
    /// Change query permissions
    ChangeQueryPermissions,
    /// Approve/deny query request
    ApproveQuery,
    /// Change governance model
    ChangeGovernance,
    /// Close/archive pool
    ClosePool,
    /// Add approved entity
    AddApprovedEntity,
    /// Remove approved entity
    RemoveApprovedEntity,
    /// Distribute collective benefits
    DistributeBenefits,
}

/// Proposal status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Executed,
    Expired,
}

/// Vote on a proposal
#[hdk_entry_helper]
#[derive(Clone)]
pub struct GovernanceVote {
    /// Proposal being voted on
    pub proposal_hash: ActionHash,
    /// Voter (contributor hash)
    pub voter: ActionHash,
    /// Vote choice
    pub vote: VoteChoice,
    /// Voting weight
    pub weight: u32,
    /// Vote reason (optional)
    pub reason: Option<String>,
    /// Voted at
    pub voted_at: i64,
}

/// Vote choices
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum VoteChoice {
    For,
    Against,
    Abstain,
}

// ==================== PRIVACY BUDGET ====================

/// Individual privacy budget allocation
#[hdk_entry_helper]
#[derive(Clone)]
pub struct PrivacyBudget {
    /// Patient
    pub patient_hash: ActionHash,
    /// Pool
    pub pool_hash: ActionHash,
    /// Total budget allocated
    pub total_budget: f64,
    /// Budget consumed
    pub consumed_budget: f64,
    /// Budget period start
    pub period_start: i64,
    /// Budget period end
    pub period_end: i64,
    /// Auto-renew
    pub auto_renew: bool,
}

// ==================== BUDGET LEDGER (Formal DP Tracking) ====================

/// Budget ledger entry for formal differential privacy tracking
///
/// This entry provides persistent, verifiable tracking of privacy budget
/// consumption with proper composition theorem accounting.
///
/// # Privacy Guarantee
///
/// By tracking epsilon consumption, we ensure that the total privacy loss
/// across all queries is bounded. When the budget is exhausted, no more
/// queries can be answered, maintaining the mathematical privacy guarantee.
///
/// # Composition
///
/// Supports both basic composition (Σεᵢ) and advanced composition
/// (√(2k ln(1/δ')) · ε) for tighter bounds when many queries are needed.
#[hdk_entry_helper]
#[derive(Clone)]
pub struct BudgetLedgerEntry {
    /// Hash of the patient contributing data
    pub patient_hash: ActionHash,
    /// Hash of the pool being queried
    pub pool_hash: ActionHash,
    /// Total epsilon budget allocated for this patient-pool pair
    pub total_epsilon: f64,
    /// Epsilon consumed so far (under basic composition)
    pub consumed_epsilon: f64,
    /// Total delta budget (for (ε, δ)-DP mechanisms)
    pub total_delta: f64,
    /// Delta consumed so far
    pub consumed_delta: f64,
    /// Number of queries answered using this budget
    pub query_count: u32,
    /// Individual epsilon values for each query (enables advanced composition)
    pub epsilon_history: Vec<f64>,
    /// Composition method being used
    pub composition_method: BudgetCompositionMethod,
    /// When the budget was created
    pub created_at: i64,
    /// When the budget was last updated
    pub last_updated: i64,
    /// When the budget period ends (for renewal)
    pub period_end: Option<i64>,
    /// Whether to auto-renew when period ends
    pub auto_renew: bool,
}

/// Composition method for privacy budget accounting
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum BudgetCompositionMethod {
    /// Basic composition: total ε = Σεᵢ
    /// Simple but provides loose bounds
    Basic,
    /// Advanced composition with delta' parameter
    /// Provides tighter bounds: ε' = √(2k ln(1/δ')) · ε_avg + k · ε_avg · (e^ε_avg - 1)
    Advanced {
        /// Additional delta used for composition bound
        delta_prime: f64,
    },
}

// ==================== COLLECTIVE INSIGHTS ====================

/// Collective insight derived from pool
#[hdk_entry_helper]
#[derive(Clone)]
pub struct CollectiveInsight {
    /// Unique insight ID
    pub insight_id: String,
    /// Pool that generated this insight
    pub pool_hash: ActionHash,
    /// Type of insight
    pub insight_type: InsightType,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Key findings
    pub findings: Vec<Finding>,
    /// Supporting query hashes
    pub supporting_queries: Vec<ActionHash>,
    /// Confidence level
    pub confidence: ConfidenceLevel,
    /// Generated at
    pub generated_at: i64,
    /// Generated by (can be automated or curator)
    pub generated_by: Option<AgentPubKey>,
    /// Public visibility
    pub public: bool,
}

/// Types of collective insights
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InsightType {
    /// Prevalence estimate
    PrevalenceEstimate,
    /// Treatment effectiveness
    TreatmentEffectiveness,
    /// Risk factor identification
    RiskFactor,
    /// Geographic pattern
    GeographicPattern,
    /// Temporal trend
    TemporalTrend,
    /// Correlation discovery
    Correlation,
    /// Anomaly detection
    Anomaly,
}

/// A specific finding from analysis
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Finding {
    /// Finding statement
    pub statement: String,
    /// Statistical support
    pub statistic: Option<String>,
    /// P-value or confidence
    pub confidence: Option<f64>,
    /// Actionable recommendation
    pub recommendation: Option<String>,
}

/// Confidence levels for insights
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ConfidenceLevel {
    /// High confidence, strong evidence
    High,
    /// Medium confidence, moderate evidence
    Medium,
    /// Low confidence, preliminary evidence
    Low,
    /// Exploratory, needs validation
    Exploratory,
}

// ==================== VALIDATION ====================

/// Validate data pool
pub fn validate_data_pool(pool: &DataPool) -> ExternResult<ValidateCallbackResult> {
    if pool.pool_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Pool ID required".to_string()));
    }

    if pool.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Pool name required".to_string()));
    }

    // Validate privacy parameters
    if pool.privacy_params.epsilon <= 0.0 {
        return Ok(ValidateCallbackResult::Invalid("Epsilon must be positive".to_string()));
    }

    if pool.privacy_params.delta < 0.0 || pool.privacy_params.delta >= 1.0 {
        return Ok(ValidateCallbackResult::Invalid("Delta must be in [0, 1)".to_string()));
    }

    if pool.min_contributors < 10 {
        return Ok(ValidateCallbackResult::Invalid("Minimum 10 contributors required for privacy".to_string()));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate privacy contribution
pub fn validate_privacy_contribution(contrib: &PrivacyContribution) -> ExternResult<ValidateCallbackResult> {
    if contrib.contribution_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Contribution ID required".to_string()));
    }

    if contrib.local_aggregates.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("At least one aggregate required".to_string()));
    }

    if contrib.budget_consumed <= 0.0 {
        return Ok(ValidateCallbackResult::Invalid("Budget consumed must be positive".to_string()));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate aggregate query
pub fn validate_aggregate_query(query: &AggregateQuery) -> ExternResult<ValidateCallbackResult> {
    if query.query_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Query ID required".to_string()));
    }

    if query.organization.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Organization required".to_string()));
    }

    if query.requested_epsilon <= 0.0 {
        return Ok(ValidateCallbackResult::Invalid("Requested epsilon must be positive".to_string()));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate governance proposal
pub fn validate_governance_proposal(proposal: &GovernanceProposal) -> ExternResult<ValidateCallbackResult> {
    if proposal.proposal_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Proposal ID required".to_string()));
    }

    if proposal.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Description required".to_string()));
    }

    if proposal.voting_end <= proposal.voting_start {
        return Ok(ValidateCallbackResult::Invalid("Voting end must be after start".to_string()));
    }

    if proposal.approval_threshold <= 0.0 || proposal.approval_threshold > 1.0 {
        return Ok(ValidateCallbackResult::Invalid("Approval threshold must be in (0, 1]".to_string()));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate governance vote
pub fn validate_governance_vote(vote: &GovernanceVote) -> ExternResult<ValidateCallbackResult> {
    if vote.weight == 0 {
        return Ok(ValidateCallbackResult::Invalid("Vote weight must be positive".to_string()));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate budget ledger entry
pub fn validate_budget_ledger_entry(entry: &BudgetLedgerEntry) -> ExternResult<ValidateCallbackResult> {
    // Epsilon must be positive
    if entry.total_epsilon <= 0.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Total epsilon must be positive".to_string()
        ));
    }

    // Consumed cannot exceed total
    if entry.consumed_epsilon > entry.total_epsilon {
        return Ok(ValidateCallbackResult::Invalid(
            format!(
                "Consumed epsilon ({}) cannot exceed total ({})",
                entry.consumed_epsilon, entry.total_epsilon
            )
        ));
    }

    // Consumed must be non-negative
    if entry.consumed_epsilon < 0.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Consumed epsilon must be non-negative".to_string()
        ));
    }

    // Delta constraints
    if entry.total_delta < 0.0 || entry.total_delta >= 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Total delta must be in [0, 1)".to_string()
        ));
    }

    if entry.consumed_delta < 0.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Consumed delta must be non-negative".to_string()
        ));
    }

    if entry.consumed_delta > entry.total_delta && entry.total_delta > 0.0 {
        return Ok(ValidateCallbackResult::Invalid(
            format!(
                "Consumed delta ({}) cannot exceed total ({})",
                entry.consumed_delta, entry.total_delta
            )
        ));
    }

    // Query count must match epsilon history length
    if entry.query_count as usize != entry.epsilon_history.len() {
        return Ok(ValidateCallbackResult::Invalid(
            format!(
                "Query count ({}) must match epsilon history length ({})",
                entry.query_count, entry.epsilon_history.len()
            )
        ));
    }

    // All epsilon values in history must be positive
    for (i, eps) in entry.epsilon_history.iter().enumerate() {
        if *eps <= 0.0 {
            return Ok(ValidateCallbackResult::Invalid(
                format!("Epsilon value at index {} must be positive", i)
            ));
        }
    }

    // Validate composition method
    if let BudgetCompositionMethod::Advanced { delta_prime } = entry.composition_method {
        if delta_prime <= 0.0 || delta_prime >= 1.0 {
            return Ok(ValidateCallbackResult::Invalid(
                "Advanced composition delta_prime must be in (0, 1)".to_string()
            ));
        }
    }

    Ok(ValidateCallbackResult::Valid)
}
