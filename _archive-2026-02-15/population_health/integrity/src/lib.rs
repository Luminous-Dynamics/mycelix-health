//! Population Health Analytics Integrity Zome
//!
//! Defines entry types for population-level health analytics
//! with differential privacy protections for aggregate statistics.

use hdi::prelude::*;

/// Type of health metric
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum MetricType {
    /// Prevalence (proportion of population with condition)
    Prevalence,
    /// Incidence (new cases over time)
    Incidence,
    /// Mortality rate
    Mortality,
    /// Hospitalization rate
    Hospitalization,
    /// Vaccination coverage
    VaccinationCoverage,
    /// Screening rate
    ScreeningRate,
    /// Treatment adherence
    TreatmentAdherence,
    /// Quality of life score
    QualityOfLife,
    /// Cost per capita
    CostPerCapita,
    /// Custom metric
    Custom(String),
}

/// Geographic granularity level
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GeographicLevel {
    /// National level
    National,
    /// State/Province level
    State,
    /// County level
    County,
    /// ZIP/Postal code level (may require k-anonymity)
    PostalCode,
    /// Census tract (requires strong privacy)
    CensusTract,
    /// Health service area
    HealthServiceArea,
    /// Custom region
    CustomRegion(String),
}

/// Time granularity for aggregation
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TimeGranularity {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

/// Alert severity level
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AlertSeverity {
    /// Informational - notable but not urgent
    Info,
    /// Warning - potential concern
    Warning,
    /// Critical - requires attention
    Critical,
    /// Emergency - immediate action required
    Emergency,
}

/// Type of surveillance alert
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AlertType {
    /// Unusual increase in condition
    AnomalyDetected,
    /// Threshold exceeded
    ThresholdBreached,
    /// Trend change detected
    TrendChange,
    /// Seasonal pattern deviation
    SeasonalDeviation,
    /// Geographic cluster detected
    ClusterDetected,
    /// Data quality issue
    DataQualityAlert,
}

/// An aggregated population health statistic
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PopulationStatistic {
    /// Unique statistic ID
    pub statistic_id: String,
    /// Type of metric
    pub metric_type: MetricType,
    /// Condition or measure (e.g., ICD-10 code, LOINC code)
    pub condition_code: String,
    /// Human-readable condition name
    pub condition_name: String,
    /// Geographic region identifier
    pub region_id: String,
    /// Geographic level
    pub geographic_level: GeographicLevel,
    /// Time period start
    pub period_start: Timestamp,
    /// Time period end
    pub period_end: Timestamp,
    /// Time granularity
    pub time_granularity: TimeGranularity,
    /// The statistic value
    pub value: String,
    /// Unit of measurement
    pub unit: String,
    /// 95% confidence interval lower bound
    pub ci_lower: Option<String>,
    /// 95% confidence interval upper bound
    pub ci_upper: Option<String>,
    /// Population denominator (may be noised)
    pub denominator: u32,
    /// Whether differential privacy was applied
    pub dp_applied: bool,
    /// Epsilon used (if DP applied)
    pub epsilon: Option<String>,
    /// Data sources count
    pub source_count: u32,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Health indicator for a geographic region
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HealthIndicator {
    /// Indicator ID
    pub indicator_id: String,
    /// Indicator name
    pub name: String,
    /// Description
    pub description: String,
    /// Region identifier
    pub region_id: String,
    /// Geographic level
    pub geographic_level: GeographicLevel,
    /// Year or period
    pub period: String,
    /// Composite score (0-100)
    pub score: u32,
    /// Rank within peer group
    pub rank: Option<u32>,
    /// Peer group size
    pub peer_group_size: Option<u32>,
    /// Component scores (JSON)
    pub components: String,
    /// Trend direction (-1, 0, 1)
    pub trend: i32,
    /// Comparison to national benchmark
    pub benchmark_comparison: String,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Disease surveillance report
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct SurveillanceReport {
    /// Report ID
    pub report_id: String,
    /// Condition being monitored
    pub condition_code: String,
    /// Condition name
    pub condition_name: String,
    /// Report period start
    pub period_start: Timestamp,
    /// Report period end
    pub period_end: Timestamp,
    /// Region identifier
    pub region_id: String,
    /// Geographic level
    pub geographic_level: GeographicLevel,
    /// Case count (differentially private)
    pub case_count: u32,
    /// Expected case count (baseline)
    pub expected_count: u32,
    /// Ratio of observed to expected
    pub ratio: String,
    /// Whether alert threshold exceeded
    pub alert_triggered: bool,
    /// Alert severity if triggered
    pub alert_severity: Option<AlertSeverity>,
    /// Age distribution (JSON, DP-protected)
    pub age_distribution: Option<String>,
    /// Gender distribution (JSON, DP-protected)
    pub gender_distribution: Option<String>,
    /// Notes or interpretation
    pub notes: Option<String>,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Public health alert
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PublicHealthAlert {
    /// Alert ID
    pub alert_id: String,
    /// Alert type
    pub alert_type: AlertType,
    /// Severity
    pub severity: AlertSeverity,
    /// Condition code
    pub condition_code: String,
    /// Condition name
    pub condition_name: String,
    /// Affected region
    pub region_id: String,
    /// Geographic level
    pub geographic_level: GeographicLevel,
    /// Alert title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Supporting data (JSON)
    pub supporting_data: String,
    /// Recommended actions
    pub recommendations: Vec<String>,
    /// Alert issued timestamp
    pub issued_at: Timestamp,
    /// Alert expires timestamp
    pub expires_at: Option<Timestamp>,
    /// Whether alert has been acknowledged
    pub acknowledged: bool,
    /// Acknowledged by (if acknowledged)
    pub acknowledged_by: Option<ActionHash>,
    /// Acknowledged timestamp
    pub acknowledged_at: Option<Timestamp>,
}

/// Health disparity analysis
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DisparityAnalysis {
    /// Analysis ID
    pub analysis_id: String,
    /// Metric being analyzed
    pub metric_type: MetricType,
    /// Condition code
    pub condition_code: String,
    /// Reference group description
    pub reference_group: String,
    /// Comparison group description
    pub comparison_group: String,
    /// Stratification dimension (race, income, education, etc.)
    pub stratification: String,
    /// Region
    pub region_id: String,
    /// Period
    pub period: String,
    /// Reference group value
    pub reference_value: String,
    /// Comparison group value
    pub comparison_value: String,
    /// Absolute difference
    pub absolute_difference: String,
    /// Relative difference (ratio or percent)
    pub relative_difference: String,
    /// Statistical significance
    pub p_value: Option<String>,
    /// Confidence interval for difference
    pub difference_ci: Option<String>,
    /// Trend over time (improving, worsening, stable)
    pub trend: String,
    /// DP applied
    pub dp_applied: bool,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Care quality indicator
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct QualityIndicator {
    /// Indicator ID
    pub indicator_id: String,
    /// Measure name (e.g., HEDIS measure)
    pub measure_name: String,
    /// Measure code
    pub measure_code: String,
    /// Domain (preventive care, chronic care, etc.)
    pub domain: String,
    /// Region
    pub region_id: String,
    /// Reporting period
    pub period: String,
    /// Numerator (eligible patients receiving care)
    pub numerator: u32,
    /// Denominator (eligible patients)
    pub denominator: u32,
    /// Rate (percentage)
    pub rate: String,
    /// National benchmark
    pub benchmark: Option<String>,
    /// Percentile ranking
    pub percentile: Option<u32>,
    /// Star rating (1-5)
    pub star_rating: Option<u32>,
    /// Trend from previous period
    pub trend: i32,
    /// DP applied
    pub dp_applied: bool,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Data contribution from a source
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DataContribution {
    /// Contribution ID
    pub contribution_id: String,
    /// Source organization hash
    pub source_hash: ActionHash,
    /// Data type contributed
    pub data_type: String,
    /// Record count (noised if DP)
    pub record_count: u32,
    /// Time period covered start
    pub period_start: Timestamp,
    /// Time period covered end
    pub period_end: Timestamp,
    /// Data quality score (0-100)
    pub quality_score: u32,
    /// Completeness percentage
    pub completeness: u32,
    /// Privacy budget consumed
    pub epsilon_consumed: Option<String>,
    /// Contribution timestamp
    pub contributed_at: Timestamp,
}

/// Entry types for the population health zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    PopulationStatistic(PopulationStatistic),
    HealthIndicator(HealthIndicator),
    SurveillanceReport(SurveillanceReport),
    PublicHealthAlert(PublicHealthAlert),
    DisparityAnalysis(DisparityAnalysis),
    QualityIndicator(QualityIndicator),
    DataContribution(DataContribution),
}

/// Link types for the population health zome
#[hdk_link_types]
pub enum LinkTypes {
    /// All statistics index
    AllStatistics,
    /// Statistics by region
    StatisticsByRegion,
    /// Statistics by condition
    StatisticsByCondition,
    /// Statistics by time period
    StatisticsByPeriod,
    /// Health indicators by region
    IndicatorsByRegion,
    /// Surveillance reports by condition
    SurveillanceByCondition,
    /// Active alerts
    ActiveAlerts,
    /// Alerts by region
    AlertsByRegion,
    /// Disparity analyses
    DisparityAnalyses,
    /// Quality indicators by region
    QualityByRegion,
    /// Data contributions by source
    ContributionsBySource,
}

/// Validation callbacks
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::PopulationStatistic(stat) => validate_statistic(&stat),
                EntryTypes::SurveillanceReport(report) => validate_surveillance(&report),
                EntryTypes::PublicHealthAlert(alert) => validate_alert(&alert),
                _ => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_statistic(stat: &PopulationStatistic) -> ExternResult<ValidateCallbackResult> {
    if stat.statistic_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Statistic ID is required".to_string()));
    }
    if stat.condition_code.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Condition code is required".to_string()));
    }
    if stat.region_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Region ID is required".to_string()));
    }
    // Validate minimum population for privacy
    if stat.denominator < 10 && !stat.dp_applied {
        return Ok(ValidateCallbackResult::Invalid(
            "Small population requires differential privacy".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_surveillance(report: &SurveillanceReport) -> ExternResult<ValidateCallbackResult> {
    if report.report_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Report ID is required".to_string()));
    }
    if report.condition_code.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Condition code is required".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_alert(alert: &PublicHealthAlert) -> ExternResult<ValidateCallbackResult> {
    if alert.alert_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Alert ID is required".to_string()));
    }
    if alert.title.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Alert title is required".to_string()));
    }
    if alert.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Alert description is required".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}
