//! Health Twin Integrity Zome
//!
//! Entry types and validation for the Digital Health Twin system - a living
//! model of patient physiology that enables:
//! - "What if" scenario simulation
//! - Predictive health trajectories
//! - Treatment outcome previews
//! - Personalized medicine at scale
//!
//! This MVP focuses on:
//! - Basic physiological modeling
//! - Simple predictions (risk scores)
//! - Treatment scenario comparison
//! - Health trajectory visualization data

use hdi::prelude::*;

/// Define the entry types for the health twin zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    /// The health twin itself (digital model)
    HealthTwin(HealthTwin),
    /// Data point feeding the twin
    TwinDataPoint(TwinDataPoint),
    /// Simulation scenario
    Simulation(Simulation),
    /// Prediction result
    Prediction(Prediction),
    /// Twin configuration/preferences
    TwinConfiguration(TwinConfiguration),
    /// Health trajectory (time series)
    HealthTrajectory(HealthTrajectory),
    /// Model update (when twin learns)
    ModelUpdate(ModelUpdate),
}

/// Link types for the health twin zome
#[hdk_link_types]
pub enum LinkTypes {
    PatientToTwin,
    TwinToDataPoints,
    TwinToSimulations,
    TwinToPredictions,
    TwinToConfig,
    TwinToTrajectories,
    TwinToUpdates,
    ActiveTwins,
}

// ==================== HEALTH TWIN ====================

/// The digital health twin - a living model of patient physiology
#[hdk_entry_helper]
#[derive(Clone)]
pub struct HealthTwin {
    /// Unique twin ID
    pub twin_id: String,
    /// Patient this twin represents
    pub patient_hash: ActionHash,
    /// Twin creation timestamp
    pub created_at: i64,
    /// Last update timestamp
    pub last_updated: i64,
    /// Model version
    pub model_version: String,
    /// Current physiological state
    pub physiological_state: PhysiologicalState,
    /// Risk factors computed from data
    pub risk_factors: Vec<RiskFactor>,
    /// Baseline health metrics
    pub baseline_metrics: BaselineMetrics,
    /// Active conditions being modeled
    pub modeled_conditions: Vec<ModeledCondition>,
    /// Data sources feeding the twin
    pub data_sources: Vec<DataSourceInfo>,
    /// Model confidence (0.0 - 1.0)
    pub confidence: f32,
    /// Twin status
    pub status: TwinStatus,
}

/// Current physiological state of the twin
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PhysiologicalState {
    /// Cardiovascular state
    pub cardiovascular: CardiovascularState,
    /// Metabolic state
    pub metabolic: MetabolicState,
    /// Respiratory state
    pub respiratory: Option<RespiratoryState>,
    /// Renal state
    pub renal: Option<RenalState>,
    /// Hepatic state
    pub hepatic: Option<HepaticState>,
    /// Neurological state
    pub neurological: Option<NeurologicalState>,
    /// Immunological state
    pub immunological: Option<ImmunologicalState>,
    /// Overall health score (0-100)
    pub overall_health_score: u8,
    /// State computed at
    pub computed_at: i64,
}

/// Cardiovascular physiological state
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CardiovascularState {
    /// Resting heart rate (bpm)
    pub resting_hr: Option<u8>,
    /// Systolic blood pressure
    pub systolic_bp: Option<u16>,
    /// Diastolic blood pressure
    pub diastolic_bp: Option<u16>,
    /// Heart rate variability (ms)
    pub hrv_ms: Option<f32>,
    /// Estimated ejection fraction
    pub ejection_fraction: Option<f32>,
    /// Cardiovascular age vs chronological
    pub cv_age_offset_years: Option<i8>,
    /// 10-year cardiovascular risk %
    pub ten_year_cv_risk: Option<f32>,
}

/// Metabolic physiological state
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetabolicState {
    /// BMI
    pub bmi: Option<f32>,
    /// Estimated basal metabolic rate
    pub bmr_kcal: Option<u16>,
    /// Fasting glucose (mg/dL)
    pub fasting_glucose: Option<u16>,
    /// HbA1c %
    pub hba1c: Option<f32>,
    /// Total cholesterol
    pub total_cholesterol: Option<u16>,
    /// LDL cholesterol
    pub ldl: Option<u16>,
    /// HDL cholesterol
    pub hdl: Option<u16>,
    /// Triglycerides
    pub triglycerides: Option<u16>,
    /// Metabolic syndrome risk score
    pub metabolic_syndrome_risk: Option<f32>,
}

/// Respiratory physiological state
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RespiratoryState {
    /// Resting respiratory rate
    pub resting_rr: Option<u8>,
    /// FEV1 % predicted
    pub fev1_percent: Option<f32>,
    /// FVC % predicted
    pub fvc_percent: Option<f32>,
    /// SpO2 at rest
    pub spo2_rest: Option<u8>,
    /// Lung age vs chronological
    pub lung_age_offset_years: Option<i8>,
}

/// Renal physiological state
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RenalState {
    /// Estimated GFR (mL/min/1.73m²)
    pub egfr: Option<f32>,
    /// Creatinine (mg/dL)
    pub creatinine: Option<f32>,
    /// BUN (mg/dL)
    pub bun: Option<f32>,
    /// CKD stage (1-5)
    pub ckd_stage: Option<u8>,
}

/// Hepatic physiological state
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HepaticState {
    /// ALT (U/L)
    pub alt: Option<u16>,
    /// AST (U/L)
    pub ast: Option<u16>,
    /// Bilirubin (mg/dL)
    pub bilirubin: Option<f32>,
    /// Albumin (g/dL)
    pub albumin: Option<f32>,
    /// Fatty liver index
    pub fatty_liver_index: Option<f32>,
}

/// Neurological state
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NeurologicalState {
    /// Cognitive score (if assessed)
    pub cognitive_score: Option<u8>,
    /// Sleep quality score
    pub sleep_quality_score: Option<u8>,
    /// Stress index
    pub stress_index: Option<f32>,
    /// Mental health score
    pub mental_health_score: Option<u8>,
}

/// Immunological state
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImmunologicalState {
    /// White blood cell count (K/uL)
    pub wbc: Option<f32>,
    /// Inflammatory markers elevated
    pub inflammation_elevated: bool,
    /// CRP (mg/L) if available
    pub crp: Option<f32>,
    /// Vaccination coverage score
    pub vaccination_coverage: Option<u8>,
}

/// Risk factor identified by the twin
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RiskFactor {
    /// Risk factor name
    pub name: String,
    /// Category
    pub category: RiskCategory,
    /// Current risk level (0.0 - 1.0)
    pub risk_level: f32,
    /// Trend (improving, stable, worsening)
    pub trend: RiskTrend,
    /// Contributing factors
    pub contributors: Vec<String>,
    /// Modifiable through lifestyle
    pub modifiable: bool,
    /// Recommended interventions
    pub interventions: Vec<String>,
}

/// Risk categories
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RiskCategory {
    Cardiovascular,
    Metabolic,
    Oncological,
    Respiratory,
    Renal,
    Hepatic,
    Neurological,
    Mental,
    Infectious,
    Musculoskeletal,
    Other(String),
}

/// Risk trend
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RiskTrend {
    Improving,
    Stable,
    Worsening,
    Unknown,
}

/// Baseline metrics for comparison
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BaselineMetrics {
    /// Baseline established at
    pub established_at: i64,
    /// Age at baseline
    pub age_at_baseline: u8,
    /// Weight at baseline (kg)
    pub weight_kg: Option<f32>,
    /// Height (cm)
    pub height_cm: Option<u16>,
    /// Baseline blood pressure
    pub baseline_bp: Option<(u16, u16)>,
    /// Baseline resting HR
    pub baseline_hr: Option<u8>,
    /// Baseline cholesterol panel
    pub baseline_lipids: Option<(u16, u16, u16, u16)>, // total, ldl, hdl, trig
    /// Baseline glucose
    pub baseline_glucose: Option<u16>,
}

/// Condition being actively modeled
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModeledCondition {
    /// Condition name
    pub condition: String,
    /// ICD-10 code if available
    pub icd10_code: Option<String>,
    /// Onset date
    pub onset_date: Option<i64>,
    /// Current stage/severity
    pub current_stage: Option<String>,
    /// Projected progression (months to next stage)
    pub months_to_progression: Option<u32>,
    /// Control status
    pub control_status: ConditionControl,
}

/// How well a condition is controlled
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ConditionControl {
    WellControlled,
    ModeratelyControlled,
    PoorlyControlled,
    Uncontrolled,
    Unknown,
}

/// Information about data sources feeding the twin
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataSourceInfo {
    /// Source type
    pub source_type: DataSourceType,
    /// Last data from this source
    pub last_data_at: i64,
    /// Number of data points
    pub data_point_count: u64,
    /// Data quality score
    pub quality_score: f32,
}

/// Types of data sources
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DataSourceType {
    /// Electronic health records
    EHR,
    /// Lab results
    Laboratory,
    /// Wearable devices
    Wearable,
    /// Patient self-reported
    SelfReported,
    /// Imaging studies
    Imaging,
    /// Pharmacy data
    Pharmacy,
    /// Genetic data
    Genetic,
    /// Social determinants
    SocialDeterminants,
}

/// Twin status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TwinStatus {
    /// Active and updating
    Active,
    /// Paused by patient
    Paused,
    /// Insufficient data
    InsufficientData,
    /// Being recalibrated
    Calibrating,
    /// Archived
    Archived,
}

// ==================== DATA POINTS ====================

/// Data point feeding the twin
#[hdk_entry_helper]
#[derive(Clone)]
pub struct TwinDataPoint {
    /// Unique data point ID
    pub data_point_id: String,
    /// Twin this feeds
    pub twin_hash: ActionHash,
    /// Data type
    pub data_type: TwinDataType,
    /// Value (JSON serialized for flexibility)
    pub value: String,
    /// Unit
    pub unit: Option<String>,
    /// Timestamp of measurement
    pub measured_at: i64,
    /// Source
    pub source: DataSourceType,
    /// Quality/confidence
    pub quality: DataQuality,
    /// Whether this updated the model
    pub triggered_update: bool,
    /// Ingested at
    pub ingested_at: i64,
}

/// Types of twin data
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TwinDataType {
    VitalSign(VitalSignType),
    LabResult(String),
    Medication(String),
    Diagnosis(String),
    Procedure(String),
    Lifestyle(LifestyleType),
    Symptom(String),
    BiometricReading(String),
    GeneticMarker(String),
    SocialDeterminant(String),
}

/// Vital sign types
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum VitalSignType {
    HeartRate,
    BloodPressure,
    Temperature,
    RespiratoryRate,
    SpO2,
    Weight,
    Height,
    BMI,
}

/// Lifestyle data types
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum LifestyleType {
    SleepDuration,
    SleepQuality,
    PhysicalActivity,
    Diet,
    StressLevel,
    Smoking,
    AlcoholConsumption,
    Hydration,
}

/// Data quality assessment
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DataQuality {
    /// Clinical-grade, validated
    Clinical,
    /// Consumer device, reasonable accuracy
    Consumer,
    /// Self-reported, subjective
    SelfReported,
    /// Derived/calculated
    Derived,
    /// Unknown quality
    Unknown,
}

// ==================== SIMULATIONS ====================

/// Simulation scenario
#[hdk_entry_helper]
#[derive(Clone)]
pub struct Simulation {
    /// Unique simulation ID
    pub simulation_id: String,
    /// Twin running the simulation
    pub twin_hash: ActionHash,
    /// Scenario type
    pub scenario_type: ScenarioType,
    /// Scenario name
    pub name: String,
    /// Description
    pub description: String,
    /// Interventions being simulated
    pub interventions: Vec<SimulatedIntervention>,
    /// Time horizon (months)
    pub time_horizon_months: u32,
    /// Simulation results
    pub results: Option<SimulationResults>,
    /// Created at
    pub created_at: i64,
    /// Completed at
    pub completed_at: Option<i64>,
    /// Status
    pub status: SimulationStatus,
}

/// Types of simulation scenarios
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ScenarioType {
    /// What if I take this medication?
    MedicationChange,
    /// What if I change my lifestyle?
    LifestyleChange,
    /// What if I have this procedure?
    ProcedureOutcome,
    /// What if I don't treat this?
    NoTreatment,
    /// What if I follow this treatment plan?
    TreatmentPlan,
    /// Comparing multiple options
    Comparison,
    /// Long-term prognosis
    Prognosis,
}

/// Simulated intervention
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimulatedIntervention {
    /// Intervention type
    pub intervention_type: InterventionType,
    /// Specific intervention
    pub intervention: String,
    /// Start timing (months from now)
    pub start_month: u32,
    /// Duration (months, None = ongoing)
    pub duration_months: Option<u32>,
    /// Compliance assumption (0.0 - 1.0)
    pub compliance_rate: f32,
}

/// Types of interventions
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InterventionType {
    Medication,
    Surgery,
    Lifestyle,
    Therapy,
    Device,
    Supplement,
    Monitoring,
}

/// Simulation results
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimulationResults {
    /// Projected outcomes
    pub outcomes: Vec<ProjectedOutcome>,
    /// Comparison to baseline (no intervention)
    pub baseline_comparison: BaselineComparison,
    /// Confidence in results
    pub confidence: f32,
    /// Caveats/limitations
    pub caveats: Vec<String>,
    /// Computed at
    pub computed_at: i64,
}

/// Projected outcome from simulation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectedOutcome {
    /// Metric being projected
    pub metric: String,
    /// Current value
    pub current_value: f32,
    /// Projected value at time horizon
    pub projected_value: f32,
    /// Change percentage
    pub change_percent: f32,
    /// Confidence interval (low, high)
    pub confidence_interval: (f32, f32),
    /// Trajectory over time
    pub trajectory: Vec<TrajectoryPoint>,
}

/// Point in a trajectory
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrajectoryPoint {
    /// Months from now
    pub month: u32,
    /// Projected value
    pub value: f32,
    /// Confidence at this point
    pub confidence: f32,
}

/// Comparison to baseline scenario
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BaselineComparison {
    /// Risk reduction from intervention
    pub risk_reduction_percent: f32,
    /// Quality-adjusted life years gained
    pub qaly_gained: Option<f32>,
    /// Cost impact (positive = cost, negative = savings)
    pub cost_impact: Option<f32>,
    /// Side effect risk increase
    pub side_effect_risk: f32,
}

/// Simulation status
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SimulationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

// ==================== PREDICTIONS ====================

/// Prediction from the twin
#[hdk_entry_helper]
#[derive(Clone)]
pub struct Prediction {
    /// Unique prediction ID
    pub prediction_id: String,
    /// Twin making prediction
    pub twin_hash: ActionHash,
    /// What's being predicted
    pub prediction_type: PredictionType,
    /// Prediction target (what metric/event)
    pub target: String,
    /// Time horizon
    pub horizon: PredictionHorizon,
    /// Predicted value/probability
    pub predicted_value: f32,
    /// Confidence interval
    pub confidence_interval: (f32, f32),
    /// Model used
    pub model_id: String,
    /// Features that drove prediction
    pub key_features: Vec<PredictionFeature>,
    /// Generated at
    pub generated_at: i64,
    /// Valid until
    pub valid_until: i64,
    /// Was prediction accurate (filled in later)
    pub outcome: Option<PredictionOutcome>,
}

/// Types of predictions
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PredictionType {
    /// Risk score (probability of event)
    RiskScore,
    /// Future value prediction
    ValuePrediction,
    /// Time to event
    TimeToEvent,
    /// Treatment response
    TreatmentResponse,
    /// Hospitalization risk
    HospitalizationRisk,
    /// Mortality risk
    MortalityRisk,
}

/// Prediction time horizon
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PredictionHorizon {
    OneMonth,
    ThreeMonths,
    SixMonths,
    OneYear,
    FiveYears,
    TenYears,
    TwentyYears,
    Lifetime,
}

/// Feature that influenced prediction
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PredictionFeature {
    /// Feature name
    pub name: String,
    /// Feature value
    pub value: f32,
    /// Importance (SHAP-like value)
    pub importance: f32,
    /// Direction of influence
    pub direction: InfluenceDirection,
}

/// Direction of feature influence
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum InfluenceDirection {
    IncreasesRisk,
    DecreasesRisk,
    Neutral,
}

/// Actual outcome for prediction validation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PredictionOutcome {
    /// Actual value/event
    pub actual_value: f32,
    /// Was prediction accurate
    pub accurate: bool,
    /// Error magnitude
    pub error: f32,
    /// Recorded at
    pub recorded_at: i64,
}

// ==================== CONFIGURATION ====================

/// Twin configuration
#[hdk_entry_helper]
#[derive(Clone)]
pub struct TwinConfiguration {
    /// Patient
    pub patient_hash: ActionHash,
    /// Data sources to use
    pub enabled_sources: Vec<DataSourceType>,
    /// Conditions to focus on
    pub focus_conditions: Vec<String>,
    /// Risk factors to monitor
    pub monitored_risks: Vec<RiskCategory>,
    /// Prediction types to generate
    pub enabled_predictions: Vec<PredictionType>,
    /// Auto-simulation preferences
    pub auto_simulation: AutoSimulationPrefs,
    /// Privacy level
    pub privacy_level: TwinPrivacyLevel,
    /// Updated at
    pub updated_at: i64,
}

/// Auto-simulation preferences
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AutoSimulationPrefs {
    /// Run simulations on new medications
    pub simulate_new_medications: bool,
    /// Run simulations on lab results
    pub simulate_lab_changes: bool,
    /// Days between automatic projections
    pub projection_interval_days: u32,
}

/// Privacy level for the twin
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TwinPrivacyLevel {
    /// Fully private, no federated learning
    FullPrivacy,
    /// Contribute anonymized insights
    ContributeInsights,
    /// Participate in federated learning
    FederatedLearning,
}

// ==================== TRAJECTORIES ====================

/// Health trajectory over time
#[hdk_entry_helper]
#[derive(Clone)]
pub struct HealthTrajectory {
    /// Unique trajectory ID
    pub trajectory_id: String,
    /// Twin
    pub twin_hash: ActionHash,
    /// Metric being tracked
    pub metric: String,
    /// Time series data
    pub data_points: Vec<TrajectoryDataPoint>,
    /// Trend analysis
    pub trend: TrendAnalysis,
    /// Computed at
    pub computed_at: i64,
}

/// Data point in trajectory
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrajectoryDataPoint {
    /// Timestamp
    pub timestamp: i64,
    /// Actual value
    pub actual: f32,
    /// Predicted value (for comparison)
    pub predicted: Option<f32>,
}

/// Trend analysis
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrendAnalysis {
    /// Overall trend direction
    pub direction: RiskTrend,
    /// Slope (units per month)
    pub slope: f32,
    /// R-squared of trend line
    pub r_squared: f32,
    /// Notable inflection points
    pub inflection_points: Vec<i64>,
}

// ==================== MODEL UPDATES ====================

/// Record of model update
#[hdk_entry_helper]
#[derive(Clone)]
pub struct ModelUpdate {
    /// Unique update ID
    pub update_id: String,
    /// Twin updated
    pub twin_hash: ActionHash,
    /// Previous model version
    pub previous_version: String,
    /// New model version
    pub new_version: String,
    /// Update reason
    pub reason: ModelUpdateReason,
    /// Data points that triggered update
    pub triggering_data: Vec<ActionHash>,
    /// Parameters changed
    pub parameters_changed: Vec<String>,
    /// Updated at
    pub updated_at: i64,
}

/// Reasons for model update
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ModelUpdateReason {
    /// New data received
    NewData,
    /// Periodic recalibration
    Recalibration,
    /// Prediction accuracy improvement
    AccuracyImprovement,
    /// New condition diagnosed
    NewCondition,
    /// Federated learning update
    FederatedUpdate,
    /// User feedback
    UserFeedback,
}

// ==================== VALIDATION ====================

/// Validate a health twin
pub fn validate_health_twin(twin: &HealthTwin) -> ExternResult<ValidateCallbackResult> {
    if twin.twin_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Twin ID required".to_string()));
    }

    if twin.confidence < 0.0 || twin.confidence > 1.0 {
        return Ok(ValidateCallbackResult::Invalid("Confidence must be between 0 and 1".to_string()));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate a simulation
pub fn validate_simulation(sim: &Simulation) -> ExternResult<ValidateCallbackResult> {
    if sim.simulation_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Simulation ID required".to_string()));
    }

    if sim.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Simulation name required".to_string()));
    }

    if sim.time_horizon_months == 0 {
        return Ok(ValidateCallbackResult::Invalid("Time horizon must be positive".to_string()));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate a prediction
pub fn validate_prediction(pred: &Prediction) -> ExternResult<ValidateCallbackResult> {
    if pred.prediction_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Prediction ID required".to_string()));
    }

    if pred.target.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Prediction target required".to_string()));
    }

    // Confidence interval should be ordered
    if pred.confidence_interval.0 > pred.confidence_interval.1 {
        return Ok(ValidateCallbackResult::Invalid("Invalid confidence interval".to_string()));
    }

    Ok(ValidateCallbackResult::Valid)
}

