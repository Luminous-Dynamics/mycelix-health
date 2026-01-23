//! Collective Immunity Intelligence Integrity Zome
//!
//! Privacy-preserving public health surveillance that enables:
//! - Real-time outbreak detection without identifying individuals
//! - Vaccination coverage monitoring
//! - Disease spread modeling
//! - Public health response coordination
//!
//! Key Principles:
//! - Zero individual identification possible
//! - Aggregate data only, never individual records
//! - Opt-in participation with clear benefits
//! - Transparent algorithms and governance

use hdi::prelude::*;

/// Define the entry types for the immunity intelligence zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    /// Health event report (privacy-preserving)
    HealthEventReport(HealthEventReport),
    /// Surveillance zone configuration
    SurveillanceZone(SurveillanceZone),
    /// Aggregate alert triggered by patterns
    AggregateAlert(AggregateAlert),
    /// Vaccination coverage snapshot
    VaccinationCoverage(VaccinationCoverage),
    /// Syndromic surveillance report
    SyndromicSurveillance(SyndromicSurveillance),
    /// Public health response
    PublicHealthResponse(PublicHealthResponse),
    /// Immunity status summary (anonymized)
    ImmunityStatus(ImmunityStatus),
    /// Outbreak investigation (aggregated)
    OutbreakInvestigation(OutbreakInvestigation),
}

/// Link types for the immunity intelligence zome
#[hdk_link_types]
pub enum LinkTypes {
    ZoneToReports,
    ZoneToAlerts,
    ZoneToCoverage,
    ActiveSurveillance,
    AlertToResponses,
    OngoingOutbreaks,
    HistoricalPatterns,
}

// ==================== HEALTH EVENT REPORTS ====================

/// Privacy-preserving health event report
/// Contains ONLY aggregated/noisy data, never individual identifiers
#[hdk_entry_helper]
#[derive(Clone)]
pub struct HealthEventReport {
    /// Unique report ID
    pub report_id: String,
    /// Surveillance zone (geographic area)
    pub zone_hash: ActionHash,
    /// Event type being reported
    pub event_type: HealthEventType,
    /// Noisy count (differential privacy applied locally)
    pub noisy_count: f64,
    /// Time bucket (e.g., "2024-01-15-AM" - never exact time)
    pub time_bucket: String,
    /// Age bracket (aggregated)
    pub age_bracket: Option<AgeBracket>,
    /// Report timestamp
    pub reported_at: i64,
    /// Number of original contributors
    pub contributor_count: u32,
    /// Privacy epsilon consumed
    pub epsilon_consumed: f64,
}

/// Types of health events for surveillance
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum HealthEventType {
    /// Respiratory symptoms
    Respiratory { severity: SymptomSeverity },
    /// Gastrointestinal symptoms
    Gastrointestinal { severity: SymptomSeverity },
    /// Fever/influenza-like illness
    InfluenzaLike,
    /// Vaccination event
    Vaccination { vaccine_type: VaccineType },
    /// Lab-confirmed infection (aggregated)
    LabConfirmed { pathogen_category: PathogenCategory },
    /// Emergency department visit (category only)
    EmergencyVisit { category: EDCategory },
    /// Hospitalization (category only)
    Hospitalization { category: HospitalizationCategory },
    /// Unusual cluster detected
    UnusualCluster { description: String },
}

/// Symptom severity levels (anonymized)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SymptomSeverity {
    Mild,
    Moderate,
    Severe,
}

/// Vaccine types for tracking
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum VaccineType {
    COVID19,
    Influenza,
    Pneumococcal,
    Shingles,
    Tetanus,
    MMR,
    Hepatitis,
    Other(String),
}

/// Pathogen categories (never specific patient data)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PathogenCategory {
    Respiratory,
    Enteric,
    VectorBorne,
    BloodBorne,
    Emerging,
}

/// Emergency department visit categories
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum EDCategory {
    Trauma,
    CardiovascularSymptoms,
    RespiratoryDistress,
    NeurologicalSymptoms,
    InfectiousSyndrome,
    Other,
}

/// Hospitalization categories
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum HospitalizationCategory {
    ICU,
    GeneralMedicine,
    Pediatric,
    Obstetric,
    Surgical,
}

/// Age brackets for anonymization
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AgeBracket {
    Child0to4,
    Child5to17,
    Adult18to29,
    Adult30to49,
    Adult50to64,
    Senior65to79,
    Senior80Plus,
}

// ==================== SURVEILLANCE ZONES ====================

/// Geographic surveillance zone
#[hdk_entry_helper]
#[derive(Clone)]
pub struct SurveillanceZone {
    /// Unique zone ID
    pub zone_id: String,
    /// Zone name
    pub name: String,
    /// Geographic level
    pub level: ZoneLevel,
    /// Parent zone (for hierarchical aggregation)
    pub parent_zone: Option<ActionHash>,
    /// Population estimate (for rate calculations)
    pub population_estimate: u64,
    /// Alert thresholds
    pub thresholds: AlertThresholds,
    /// Privacy parameters
    pub privacy_params: ZonePrivacyParams,
    /// Active surveillance
    pub active: bool,
    /// Created at
    pub created_at: i64,
}

/// Geographic levels
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ZoneLevel {
    /// National level
    National,
    /// State/province level
    State,
    /// County/district level
    County,
    /// City/municipality level
    City,
    /// Neighborhood level
    Neighborhood,
    /// Custom defined zone
    Custom(String),
}

/// Alert thresholds for the zone
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlertThresholds {
    /// Threshold for mild alert (events per 100k population)
    pub mild_threshold: f64,
    /// Threshold for moderate alert
    pub moderate_threshold: f64,
    /// Threshold for severe alert
    pub severe_threshold: f64,
    /// Minimum observations before alerting
    pub min_observations: u32,
    /// Baseline comparison period (days)
    pub baseline_period_days: u32,
}

/// Zone-specific privacy parameters
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ZonePrivacyParams {
    /// Minimum contributors for report release
    pub min_contributors: u32,
    /// Epsilon budget per time period
    pub epsilon_budget: f64,
    /// Time bucket granularity (hours)
    pub time_bucket_hours: u32,
    /// Suppress if below this count
    pub suppression_threshold: u32,
}

// ==================== AGGREGATE ALERTS ====================

/// Public health alert (aggregated data only)
#[hdk_entry_helper]
#[derive(Clone)]
pub struct AggregateAlert {
    /// Unique alert ID
    pub alert_id: String,
    /// Zone where alert triggered
    pub zone_hash: ActionHash,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert type
    pub alert_type: AlertType,
    /// Statistical basis for alert
    pub statistical_basis: StatisticalBasis,
    /// Affected age groups (if significant)
    pub affected_age_groups: Vec<AgeBracket>,
    /// Triggered at
    pub triggered_at: i64,
    /// Auto-expires at
    pub expires_at: i64,
    /// Current status
    pub status: AlertStatus,
    /// Public message (if released)
    pub public_message: Option<String>,
}

/// Alert severity levels
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AlertSeverity {
    /// Watching - unusual pattern
    Watch,
    /// Advisory - elevated activity
    Advisory,
    /// Warning - significant increase
    Warning,
    /// Emergency - critical situation
    Emergency,
}

/// Types of alerts
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AlertType {
    /// Unusual increase in specific syndrome
    SyndromicSpike,
    /// Geographic cluster detected
    GeographicCluster,
    /// Vaccination coverage drop
    CoverageDecline,
    /// Multi-zone correlated increase
    RegionalPattern,
    /// Lab-confirmed outbreak
    ConfirmedOutbreak,
}

/// Statistical basis for alert
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatisticalBasis {
    /// Current rate (noisy)
    pub current_rate: f64,
    /// Baseline rate
    pub baseline_rate: f64,
    /// Standard deviations above baseline
    pub z_score: f64,
    /// Time period for comparison
    pub comparison_period_days: u32,
    /// Confidence level
    pub confidence: f64,
}

/// Alert status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AlertStatus {
    Active,
    Investigating,
    Resolved,
    Expired,
    FalseAlarm,
}

// ==================== VACCINATION COVERAGE ====================

/// Vaccination coverage snapshot
#[hdk_entry_helper]
#[derive(Clone)]
pub struct VaccinationCoverage {
    /// Unique coverage ID
    pub coverage_id: String,
    /// Zone hash
    pub zone_hash: ActionHash,
    /// Vaccine type
    pub vaccine_type: VaccineType,
    /// Coverage estimates by age group
    pub coverage_by_age: Vec<AgeCoverage>,
    /// Overall coverage rate (noisy)
    pub overall_coverage: f64,
    /// Margin of error
    pub margin_of_error: f64,
    /// Time period
    pub period_start: i64,
    /// Time period end
    pub period_end: i64,
    /// Sample size (noisy)
    pub sample_size: f64,
    /// Computed at
    pub computed_at: i64,
}

/// Coverage by age bracket
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgeCoverage {
    pub age_bracket: AgeBracket,
    /// Coverage rate (0.0 - 1.0)
    pub coverage_rate: f64,
    /// Sample size (noisy)
    pub sample_size: f64,
}

// ==================== SYNDROMIC SURVEILLANCE ====================

/// Syndromic surveillance report
#[hdk_entry_helper]
#[derive(Clone)]
pub struct SyndromicSurveillance {
    /// Unique report ID
    pub report_id: String,
    /// Zone hash
    pub zone_hash: ActionHash,
    /// Syndrome being tracked
    pub syndrome: SyndromeType,
    /// Current activity level
    pub activity_level: ActivityLevel,
    /// Trend direction
    pub trend: TrendDirection,
    /// Rate per 100k (noisy)
    pub rate_per_100k: f64,
    /// Week of year
    pub week_of_year: u32,
    /// Year
    pub year: u32,
    /// Contributors
    pub contributor_count: u32,
    /// Reported at
    pub reported_at: i64,
}

/// Types of syndromes tracked
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SyndromeType {
    /// Influenza-like illness
    ILI,
    /// Acute respiratory infection
    ARI,
    /// Acute gastroenteritis
    AGI,
    /// Neurological syndrome
    Neurological,
    /// Fever of unknown origin
    FeverUnknown,
    /// Rash illness
    Rash,
}

/// Activity levels
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ActivityLevel {
    Minimal,
    Low,
    Moderate,
    High,
    VeryHigh,
}

/// Trend directions
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TrendDirection {
    Decreasing,
    Stable,
    Increasing,
    RapidIncrease,
}

// ==================== PUBLIC HEALTH RESPONSE ====================

/// Public health response to an alert
#[hdk_entry_helper]
#[derive(Clone)]
pub struct PublicHealthResponse {
    /// Unique response ID
    pub response_id: String,
    /// Alert being responded to
    pub alert_hash: ActionHash,
    /// Response actions taken
    pub actions: Vec<ResponseAction>,
    /// Response status
    pub status: ResponseStatus,
    /// Public communication issued
    pub public_communication: Option<String>,
    /// Resources allocated
    pub resources_allocated: Vec<Resource>,
    /// Initiated at
    pub initiated_at: i64,
    /// Completed at
    pub completed_at: Option<i64>,
    /// Effectiveness assessment
    pub effectiveness: Option<EffectivenessAssessment>,
}

/// Response actions
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseAction {
    pub action_type: ActionType,
    pub description: String,
    pub started_at: i64,
    pub completed_at: Option<i64>,
}

/// Types of response actions
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ActionType {
    Investigation,
    PublicNotification,
    VaccinationCampaign,
    ResourceDeployment,
    Guidelines,
    Monitoring,
}

/// Response status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ResponseStatus {
    Planning,
    Active,
    Monitoring,
    Completed,
    Cancelled,
}

/// Allocated resources
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Resource {
    pub resource_type: String,
    pub quantity: u32,
    pub deployed_at: i64,
}

/// Effectiveness assessment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EffectivenessAssessment {
    pub outcome: ResponseOutcome,
    pub notes: String,
    pub assessed_at: i64,
}

/// Response outcomes
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ResponseOutcome {
    Effective,
    PartiallyEffective,
    Ineffective,
    Inconclusive,
}

// ==================== IMMUNITY STATUS ====================

/// Aggregated immunity status for a zone
#[hdk_entry_helper]
#[derive(Clone)]
pub struct ImmunityStatus {
    /// Unique status ID
    pub status_id: String,
    /// Zone hash
    pub zone_hash: ActionHash,
    /// Immunity type
    pub immunity_type: ImmunityType,
    /// Estimated immune percentage (noisy)
    pub estimated_immune_pct: f64,
    /// Margin of error
    pub margin_of_error: f64,
    /// Sample size (noisy)
    pub sample_size: f64,
    /// As of date
    pub as_of_date: i64,
    /// Confidence interval
    pub confidence_interval: ConfidenceInterval,
}

/// Types of immunity
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ImmunityType {
    /// Vaccine-induced
    VaccineInduced(VaccineType),
    /// Natural infection
    NaturalInfection(String),
    /// Hybrid (both)
    Hybrid(String),
}

/// Confidence interval
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfidenceInterval {
    pub lower: f64,
    pub upper: f64,
    pub level: f64,
}

// ==================== OUTBREAK INVESTIGATION ====================

/// Outbreak investigation (aggregated data only)
#[hdk_entry_helper]
#[derive(Clone)]
pub struct OutbreakInvestigation {
    /// Unique investigation ID
    pub investigation_id: String,
    /// Triggering alert
    pub alert_hash: ActionHash,
    /// Zones involved
    pub affected_zones: Vec<ActionHash>,
    /// Investigation status
    pub status: InvestigationStatus,
    /// Suspected pathogen/cause
    pub suspected_cause: Option<String>,
    /// Confirmed pathogen/cause
    pub confirmed_cause: Option<String>,
    /// Epidemiological summary (aggregated)
    pub epi_summary: EpiSummary,
    /// Investigation findings
    pub findings: Vec<Finding>,
    /// Started at
    pub started_at: i64,
    /// Completed at
    pub completed_at: Option<i64>,
}

/// Investigation status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InvestigationStatus {
    Initiated,
    DataCollection,
    Analysis,
    Concluded,
    Monitoring,
}

/// Epidemiological summary
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EpiSummary {
    /// Total case count (noisy)
    pub case_count: f64,
    /// Hospitalization count (noisy)
    pub hospitalizations: f64,
    /// Mortality count (noisy, suppressed if low)
    pub mortality: Option<f64>,
    /// Attack rate estimate
    pub attack_rate: Option<f64>,
    /// Serial interval estimate
    pub serial_interval_days: Option<f64>,
    /// Reproduction number estimate
    pub r_number: Option<f64>,
}

/// Investigation finding
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Finding {
    pub finding_type: FindingType,
    pub description: String,
    pub confidence: f64,
}

/// Types of findings
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum FindingType {
    TransmissionRoute,
    RiskFactor,
    ProtectiveFactor,
    SourceIdentified,
    RecommendedAction,
}

// ==================== VALIDATION ====================

/// Validate health event report
pub fn validate_health_event_report(
    report: &HealthEventReport,
) -> ExternResult<ValidateCallbackResult> {
    if report.report_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Report ID required".to_string(),
        ));
    }

    if report.contributor_count == 0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Must have at least one contributor".to_string(),
        ));
    }

    if report.epsilon_consumed <= 0.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "Epsilon consumed must be positive".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate surveillance zone
pub fn validate_surveillance_zone(zone: &SurveillanceZone) -> ExternResult<ValidateCallbackResult> {
    if zone.zone_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Zone ID required".to_string(),
        ));
    }

    if zone.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Zone name required".to_string(),
        ));
    }

    if zone.privacy_params.min_contributors < 10 {
        return Ok(ValidateCallbackResult::Invalid(
            "Minimum 10 contributors for privacy".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate aggregate alert
pub fn validate_aggregate_alert(alert: &AggregateAlert) -> ExternResult<ValidateCallbackResult> {
    if alert.alert_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Alert ID required".to_string(),
        ));
    }

    if alert.expires_at <= alert.triggered_at {
        return Ok(ValidateCallbackResult::Invalid(
            "Expiry must be after trigger".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}
