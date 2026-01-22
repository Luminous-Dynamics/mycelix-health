//! Universal Health Moment Integrity Zome
//!
//! Provides global health context awareness enabling:
//! - Understanding what's happening health-wise in your community
//! - Seasonal and environmental health patterns
//! - Relevant health alerts and recommendations
//! - Connection between individual and collective health
//!
//! Design Philosophy:
//! - Proactive, not reactive health awareness
//! - Privacy-preserving aggregate insights
//! - Contextual relevance based on location, season, demographics
//! - Empowering individuals with collective knowledge

use hdi::prelude::*;

/// Define the entry types for the universal health moment zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    /// Health context for a region at a point in time
    HealthMoment(HealthMoment),
    /// Seasonal health pattern
    SeasonalPattern(SeasonalPattern),
    /// Environmental health factor
    EnvironmentalFactor(EnvironmentalFactor),
    /// Community health pulse (real-time aggregate)
    CommunityPulse(CommunityPulse),
    /// Health advisory for a region
    HealthAdvisory(HealthAdvisory),
    /// Wellness recommendation
    WellnessRecommendation(WellnessRecommendation),
    /// Personal health context (individual's view)
    PersonalContext(PersonalContext),
    /// Global health dashboard entry
    GlobalDashboard(GlobalDashboard),
}

/// Link types for the health moment zome
#[hdk_link_types]
pub enum LinkTypes {
    RegionToMoments,
    SeasonToPatterns,
    ActiveAdvisories,
    PersonalContexts,
    GlobalDashboards,
    MomentToRecommendations,
    EnvironmentalAlerts,
}

// ==================== HEALTH MOMENT ====================

/// A snapshot of health context for a region
#[hdk_entry_helper]
#[derive(Clone)]
pub struct HealthMoment {
    /// Unique moment ID
    pub moment_id: String,
    /// Region identifier (hierarchical: country/state/city)
    pub region: RegionIdentifier,
    /// Timestamp of this moment
    pub timestamp: i64,
    /// Current health conditions
    pub conditions: Vec<ActiveCondition>,
    /// Environmental factors
    pub environmental: Vec<EnvironmentalReading>,
    /// Community health indicators
    pub community_indicators: CommunityIndicators,
    /// Active advisories
    pub active_advisories: Vec<String>,
    /// Seasonal context
    pub seasonal_context: SeasonalContext,
    /// Data quality indicator
    pub data_quality: DataQuality,
}

/// Region identifier (hierarchical)
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RegionIdentifier {
    /// Country code (ISO 3166-1 alpha-2)
    pub country: String,
    /// State/province code
    pub state: Option<String>,
    /// City/locality
    pub city: Option<String>,
    /// Neighborhood/district
    pub district: Option<String>,
}

/// Active health condition in the region
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActiveCondition {
    /// Condition type
    pub condition_type: ConditionType,
    /// Activity level
    pub activity: ActivityLevel,
    /// Trend
    pub trend: Trend,
    /// Age groups most affected
    pub affected_age_groups: Vec<AgeGroup>,
    /// Risk level for general population
    pub general_risk: RiskLevel,
}

/// Types of conditions tracked
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ConditionType {
    /// Respiratory illnesses (flu, cold, COVID)
    Respiratory,
    /// Gastrointestinal illnesses
    Gastrointestinal,
    /// Allergies (seasonal, environmental)
    Allergies,
    /// Vector-borne diseases
    VectorBorne,
    /// Mental health trends
    MentalHealth,
    /// Chronic disease flares
    ChronicFlares,
    /// Injury patterns
    Injuries,
    /// Substance-related
    SubstanceRelated,
    /// Custom condition
    Other(String),
}

/// Activity levels
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ActivityLevel {
    Minimal,
    Low,
    Moderate,
    High,
    VeryHigh,
    Outbreak,
}

/// Trend directions
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Trend {
    Decreasing,
    Stable,
    SlowIncrease,
    RapidIncrease,
    Peaking,
}

/// Age groups
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AgeGroup {
    Infants,
    Children,
    Teens,
    YoungAdults,
    Adults,
    MiddleAge,
    Seniors,
    Elderly,
    All,
}

/// Risk levels
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Negligible,
    Low,
    Moderate,
    Elevated,
    High,
    Severe,
}

/// Environmental reading
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnvironmentalReading {
    pub factor_type: EnvironmentalType,
    pub value: f64,
    pub unit: String,
    pub quality_impact: QualityImpact,
}

/// Environmental factor types
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum EnvironmentalType {
    AirQualityIndex,
    PollenCount,
    UVIndex,
    Humidity,
    Temperature,
    WildfireSmoke,
    Ozone,
    ParticulateMatter,
}

/// Impact on health quality
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum QualityImpact {
    Good,
    Moderate,
    UnhealthyForSensitive,
    Unhealthy,
    VeryUnhealthy,
    Hazardous,
}

/// Community health indicators
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommunityIndicators {
    /// Overall wellness score (0-100)
    pub wellness_score: f64,
    /// Healthcare utilization trend
    pub healthcare_utilization: UtilizationLevel,
    /// Preventive care engagement
    pub preventive_engagement: f64,
    /// Community resilience score
    pub resilience_score: f64,
}

/// Healthcare utilization levels
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum UtilizationLevel {
    BelowNormal,
    Normal,
    AboveNormal,
    High,
    Surge,
}

/// Seasonal context
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SeasonalContext {
    /// Current season
    pub season: Season,
    /// Typical conditions for this time
    pub typical_conditions: Vec<String>,
    /// Deviation from typical
    pub deviation: SeasonalDeviation,
}

/// Seasons
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Season {
    Spring,
    Summer,
    Fall,
    Winter,
    Monsoon,
    Dry,
    Wet,
}

/// Seasonal deviation
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SeasonalDeviation {
    MuchBetterThanTypical,
    BetterThanTypical,
    Typical,
    WorseThanTypical,
    MuchWorseThanTypical,
}

/// Data quality indicator
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataQuality {
    /// Number of data sources
    pub source_count: u32,
    /// Freshness (hours since last update)
    pub freshness_hours: f64,
    /// Confidence level (0-1)
    pub confidence: f64,
}

// ==================== SEASONAL PATTERNS ====================

/// Long-term seasonal health pattern
#[hdk_entry_helper]
#[derive(Clone)]
pub struct SeasonalPattern {
    /// Pattern ID
    pub pattern_id: String,
    /// Region
    pub region: RegionIdentifier,
    /// Season
    pub season: Season,
    /// Typical conditions
    pub typical_conditions: Vec<TypicalCondition>,
    /// Environmental norms
    pub environmental_norms: Vec<EnvironmentalNorm>,
    /// Historical baselines
    pub baselines: HistoricalBaselines,
    /// Last updated
    pub updated_at: i64,
}

/// Typical condition for a season
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TypicalCondition {
    pub condition_type: ConditionType,
    pub typical_activity: ActivityLevel,
    pub peak_weeks: Vec<u32>,
    pub notes: String,
}

/// Environmental norm for a season
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnvironmentalNorm {
    pub factor_type: EnvironmentalType,
    pub typical_range: (f64, f64),
    pub peak_value: f64,
}

/// Historical baselines
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HistoricalBaselines {
    /// Years of data
    pub years_of_data: u32,
    /// Typical wellness score
    pub typical_wellness: f64,
    /// Typical healthcare utilization
    pub typical_utilization: UtilizationLevel,
}

// ==================== ENVIRONMENTAL FACTORS ====================

/// Real-time environmental health factor
#[hdk_entry_helper]
#[derive(Clone)]
pub struct EnvironmentalFactor {
    /// Factor ID
    pub factor_id: String,
    /// Region
    pub region: RegionIdentifier,
    /// Factor type
    pub factor_type: EnvironmentalType,
    /// Current value
    pub current_value: f64,
    /// Unit
    pub unit: String,
    /// Impact level
    pub impact: QualityImpact,
    /// Forecast (next 24-48 hours)
    pub forecast: Option<EnvironmentalForecast>,
    /// Health recommendations
    pub recommendations: Vec<String>,
    /// Recorded at
    pub recorded_at: i64,
}

/// Environmental forecast
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnvironmentalForecast {
    pub forecast_values: Vec<ForecastPoint>,
    pub trend: Trend,
    pub confidence: f64,
}

/// Forecast point
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ForecastPoint {
    pub hours_ahead: u32,
    pub predicted_value: f64,
    pub range: (f64, f64),
}

// ==================== COMMUNITY PULSE ====================

/// Real-time community health pulse
#[hdk_entry_helper]
#[derive(Clone)]
pub struct CommunityPulse {
    /// Pulse ID
    pub pulse_id: String,
    /// Region
    pub region: RegionIdentifier,
    /// Pulse timestamp
    pub timestamp: i64,
    /// Current health sentiment (aggregate)
    pub health_sentiment: HealthSentiment,
    /// Active concerns in community
    pub active_concerns: Vec<CommunityConcern>,
    /// Positive trends
    pub positive_trends: Vec<PositiveTrend>,
    /// Resource availability
    pub resource_status: ResourceStatus,
    /// Data freshness
    pub last_updated: i64,
}

/// Aggregate health sentiment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HealthSentiment {
    /// Overall sentiment (-1 to 1)
    pub overall: f64,
    /// Confidence in health
    pub confidence: f64,
    /// Anxiety level (0-1)
    pub anxiety_level: f64,
    /// Optimism (0-1)
    pub optimism: f64,
}

/// Community health concern
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommunityConcern {
    pub concern_type: ConcernType,
    pub intensity: f64,
    pub affected_percentage: f64,
    pub emerging: bool,
}

/// Types of community concerns
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ConcernType {
    IllnessSpread,
    AirQuality,
    WaterQuality,
    FoodSafety,
    MentalHealthCrisis,
    SubstanceAbuse,
    VaccineHesitancy,
    HealthcareAccess,
    MedicationShortage,
    Other(String),
}

/// Positive community health trend
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PositiveTrend {
    pub trend_type: PositiveTrendType,
    pub magnitude: f64,
    pub description: String,
}

/// Types of positive trends
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PositiveTrendType {
    VaccinationUptake,
    PreventiveCare,
    ExerciseActivity,
    MentalHealthAwareness,
    HealthLiteracy,
    CommunitySupport,
    Other(String),
}

/// Healthcare resource status
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResourceStatus {
    /// Hospital capacity
    pub hospital_capacity: CapacityLevel,
    /// Primary care availability
    pub primary_care: AvailabilityLevel,
    /// Emergency services
    pub emergency_services: AvailabilityLevel,
    /// Pharmacy stock
    pub pharmacy_stock: StockLevel,
}

/// Capacity levels
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CapacityLevel {
    Low,
    Normal,
    High,
    NearCapacity,
    OverCapacity,
}

/// Availability levels
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AvailabilityLevel {
    Excellent,
    Good,
    Limited,
    Scarce,
    Critical,
}

/// Stock levels
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum StockLevel {
    Abundant,
    Normal,
    Low,
    Critical,
    Shortage,
}

// ==================== HEALTH ADVISORY ====================

/// Health advisory for a region
#[hdk_entry_helper]
#[derive(Clone)]
pub struct HealthAdvisory {
    /// Advisory ID
    pub advisory_id: String,
    /// Region
    pub region: RegionIdentifier,
    /// Advisory type
    pub advisory_type: AdvisoryType,
    /// Severity
    pub severity: AdvisorySeverity,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Affected groups
    pub affected_groups: Vec<AgeGroup>,
    /// Recommended actions
    pub recommended_actions: Vec<String>,
    /// Issued at
    pub issued_at: i64,
    /// Expires at
    pub expires_at: Option<i64>,
    /// Status
    pub status: AdvisoryStatus,
    /// Source
    pub source: String,
}

/// Advisory types
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AdvisoryType {
    DiseaseOutbreak,
    EnvironmentalHazard,
    ExtremeWeather,
    MedicationRecall,
    FoodSafety,
    WaterQuality,
    VaccinationCampaign,
    PreventiveCare,
    MentalHealthResource,
    General,
}

/// Advisory severity
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AdvisorySeverity {
    Information,
    Watch,
    Warning,
    Urgent,
    Emergency,
}

/// Advisory status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AdvisoryStatus {
    Active,
    Updated,
    Downgraded,
    Expired,
    Cancelled,
}

// ==================== WELLNESS RECOMMENDATIONS ====================

/// Personalized wellness recommendation
#[hdk_entry_helper]
#[derive(Clone)]
pub struct WellnessRecommendation {
    /// Recommendation ID
    pub recommendation_id: String,
    /// Region context
    pub region: RegionIdentifier,
    /// Recommendation category
    pub category: RecommendationCategory,
    /// Title
    pub title: String,
    /// Description
    pub description: String,
    /// Applicable conditions
    pub applicable_conditions: Vec<ConditionType>,
    /// Target age groups
    pub target_ages: Vec<AgeGroup>,
    /// Priority
    pub priority: Priority,
    /// Evidence level
    pub evidence_level: EvidenceLevel,
    /// Created at
    pub created_at: i64,
    /// Valid until
    pub valid_until: Option<i64>,
}

/// Recommendation categories
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RecommendationCategory {
    Prevention,
    Exercise,
    Nutrition,
    Sleep,
    Stress,
    Screening,
    Vaccination,
    MentalHealth,
    ChronicManagement,
    Environmental,
}

/// Priority levels
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Priority {
    Optional,
    Suggested,
    Recommended,
    StronglyRecommended,
    Critical,
}

/// Evidence levels
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum EvidenceLevel {
    Expert,
    Observational,
    ClinicalStudy,
    MetaAnalysis,
    Guideline,
}

// ==================== PERSONAL CONTEXT ====================

/// Individual's personal health context (private)
#[hdk_entry_helper]
#[derive(Clone)]
pub struct PersonalContext {
    /// Context ID
    pub context_id: String,
    /// Agent hash (private to owner)
    pub agent_hash: AgentPubKey,
    /// Home region
    pub home_region: RegionIdentifier,
    /// Personal risk factors (encrypted/private)
    pub risk_factors: Vec<String>,
    /// Age group
    pub age_group: AgeGroup,
    /// Relevant conditions (private)
    pub relevant_conditions: Vec<ConditionType>,
    /// Notification preferences
    pub notification_prefs: NotificationPreferences,
    /// Last updated
    pub updated_at: i64,
}

/// Notification preferences
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NotificationPreferences {
    pub advisory_threshold: AdvisorySeverity,
    pub environmental_alerts: bool,
    pub seasonal_reminders: bool,
    pub wellness_tips: bool,
    pub community_pulse: bool,
}

// ==================== GLOBAL DASHBOARD ====================

/// Global health dashboard entry
#[hdk_entry_helper]
#[derive(Clone)]
pub struct GlobalDashboard {
    /// Dashboard ID
    pub dashboard_id: String,
    /// Timestamp
    pub timestamp: i64,
    /// Global health score
    pub global_wellness_score: f64,
    /// Regional highlights
    pub regional_highlights: Vec<RegionalHighlight>,
    /// Global concerns
    pub global_concerns: Vec<GlobalConcern>,
    /// Positive global trends
    pub positive_trends: Vec<PositiveTrend>,
    /// Data quality
    pub data_quality: DataQuality,
}

/// Regional highlight
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegionalHighlight {
    pub region: RegionIdentifier,
    pub highlight_type: HighlightType,
    pub description: String,
}

/// Highlight types
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum HighlightType {
    OutbreakAlert,
    RecoverySuccess,
    VaccinationMilestone,
    EnvironmentalAlert,
    ResourceStrain,
    Innovation,
}

/// Global health concern
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GlobalConcern {
    pub concern_type: ConcernType,
    pub affected_regions: u32,
    pub trend: Trend,
    pub priority: Priority,
}

// ==================== VALIDATION ====================

/// Validate health moment
pub fn validate_health_moment(moment: &HealthMoment) -> ExternResult<ValidateCallbackResult> {
    if moment.moment_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Moment ID required".to_string()));
    }

    if moment.region.country.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Country required".to_string()));
    }

    if moment.data_quality.confidence < 0.0 || moment.data_quality.confidence > 1.0 {
        return Ok(ValidateCallbackResult::Invalid("Confidence must be 0-1".to_string()));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate health advisory
pub fn validate_health_advisory(advisory: &HealthAdvisory) -> ExternResult<ValidateCallbackResult> {
    if advisory.advisory_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Advisory ID required".to_string()));
    }

    if advisory.title.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Title required".to_string()));
    }

    if advisory.description.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Description required".to_string()));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate personal context
pub fn validate_personal_context(context: &PersonalContext) -> ExternResult<ValidateCallbackResult> {
    if context.context_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Context ID required".to_string()));
    }

    if context.home_region.country.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Home region required".to_string()));
    }

    Ok(ValidateCallbackResult::Valid)
}
