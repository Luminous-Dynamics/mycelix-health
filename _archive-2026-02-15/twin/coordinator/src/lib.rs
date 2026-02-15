//! Health Twin Coordinator Zome
//!
//! Provides extern functions for the Digital Health Twin system.

use hdk::prelude::*;
use twin_integrity::*;
use mycelix_health_shared::{require_authorization, log_data_access, DataCategory, Permission};

fn get_twin_or_err(twin_hash: &ActionHash) -> ExternResult<HealthTwin> {
    let record = get(twin_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Twin not found".to_string())))?;

    let twin: HealthTwin = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid twin".to_string())))?;

    Ok(twin)
}

// ==================== LOCAL TYPES FOR CROSS-ZOME DATA ====================
// These mirror types from hdc_genetics_integrity for deserialization
// without importing the integrity crate (which causes duplicate symbol errors)

/// Local copy of GeneticHypervector for cross-zome data
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GeneticHypervector {
    pub vector_id: String,
    pub patient_hash: ActionHash,
    pub data: Vec<u8>,
    pub encoding_type: GeneticEncodingType,
    pub kmer_length: u8,
    pub kmer_count: u32,
    pub created_at: Timestamp,
    pub source_metadata: GeneticSourceMetadata,
}

/// Local copy of GeneticEncodingType
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum GeneticEncodingType {
    DnaSequence,
    SnpPanel,
    HlaTyping,
    Pharmacogenomics,
    DiseaseRisk,
    Ancestry,
    GenePanel(String),
}

/// Local copy of GeneticSourceMetadata
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GeneticSourceMetadata {
    pub source_system: String,
    pub test_date: Option<Timestamp>,
    pub sequencing_method: Option<String>,
    pub quality_score: Option<f64>,
    pub consent_hash: Option<ActionHash>,
}

// ==================== HEALTH TWIN MANAGEMENT ====================

/// Create a new health twin for a patient
#[hdk_extern]
pub fn create_health_twin(twin: HealthTwin) -> ExternResult<Record> {
    validate_health_twin(&twin)?;

    let auth = require_authorization(
        twin.patient_hash.clone(),
        DataCategory::All,
        Permission::Write,
        false,
    )?;

    let twin_hash = create_entry(&EntryTypes::HealthTwin(twin.clone()))?;
    let record = get(twin_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find twin".to_string())))?;

    // Link to patient
    create_link(
        twin.patient_hash.clone(),
        twin_hash.clone(),
        LinkTypes::PatientToTwin,
        (),
    )?;

    // Link to active twins
    let anchor = anchor_hash("active_twins")?;
    create_link(
        anchor,
        twin_hash,
        LinkTypes::ActiveTwins,
        (),
    )?;

    log_data_access(
        twin.patient_hash.clone(),
        vec![DataCategory::All],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Get patient's health twin
#[hdk_extern]
pub fn get_patient_twin(patient_hash: ActionHash) -> ExternResult<Option<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::PatientToTwin)?,
        GetStrategy::default(),
    )?;

    // Get the most recent twin
    if let Some(link) = links.last() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            let record = get(hash, GetOptions::default())?;
            if record.is_some() {
                log_data_access(
                    patient_hash,
                    vec![DataCategory::All],
                    Permission::Read,
                    auth.consent_hash,
                    auth.emergency_override,
                    None,
                )?;
            }
            return Ok(record);
        }
    }

    Ok(None)
}

/// Update twin's physiological state
#[hdk_extern]
pub fn update_twin_state(input: UpdateTwinStateInput) -> ExternResult<Record> {
    let mut twin = get_twin_or_err(&input.twin_hash)?;

    let patient_hash = twin.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Amend,
        false,
    )?;

    twin.physiological_state = input.new_state;
    twin.last_updated = sys_time()?.as_micros() as i64;

    let updated_hash = update_entry(input.twin_hash, &twin)?;

    log_data_access(
        patient_hash,
        vec![DataCategory::All],
        Permission::Amend,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated twin".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateTwinStateInput {
    pub twin_hash: ActionHash,
    pub new_state: PhysiologicalState,
}

/// Update twin's risk factors
#[hdk_extern]
pub fn update_risk_factors(input: UpdateRiskFactorsInput) -> ExternResult<Record> {
    let mut twin = get_twin_or_err(&input.twin_hash)?;

    let patient_hash = twin.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Amend,
        false,
    )?;

    twin.risk_factors = input.risk_factors;
    twin.last_updated = sys_time()?.as_micros() as i64;

    let updated_hash = update_entry(input.twin_hash, &twin)?;

    log_data_access(
        patient_hash,
        vec![DataCategory::All],
        Permission::Amend,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated twin".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateRiskFactorsInput {
    pub twin_hash: ActionHash,
    pub risk_factors: Vec<RiskFactor>,
}

/// Change twin status
#[hdk_extern]
pub fn set_twin_status(input: SetTwinStatusInput) -> ExternResult<Record> {
    let mut twin = get_twin_or_err(&input.twin_hash)?;

    let patient_hash = twin.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Amend,
        false,
    )?;

    twin.status = input.status;
    twin.last_updated = sys_time()?.as_micros() as i64;

    let updated_hash = update_entry(input.twin_hash, &twin)?;

    log_data_access(
        patient_hash,
        vec![DataCategory::All],
        Permission::Amend,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated twin".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetTwinStatusInput {
    pub twin_hash: ActionHash,
    pub status: TwinStatus,
}

// ==================== DATA POINTS ====================

/// Ingest a data point into the twin
#[hdk_extern]
pub fn ingest_data_point(data_point: TwinDataPoint) -> ExternResult<Record> {
    let twin = get_twin_or_err(&data_point.twin_hash)?;
    let patient_hash = twin.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Write,
        false,
    )?;

    let dp_hash = create_entry(&EntryTypes::TwinDataPoint(data_point.clone()))?;
    let record = get(dp_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find data point".to_string())))?;

    // Link to twin
    create_link(
        data_point.twin_hash.clone(),
        dp_hash,
        LinkTypes::TwinToDataPoints,
        (),
    )?;

    // Update twin's last_updated and potentially recalculate
    if data_point.triggered_update {
        // Trigger model update
        let _ = trigger_model_update(data_point.twin_hash, vec![record.action_address().clone()]);
    }

    log_data_access(
        patient_hash,
        vec![DataCategory::All],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Get recent data points for a twin
#[hdk_extern]
pub fn get_twin_data_points(input: GetDataPointsInput) -> ExternResult<Vec<Record>> {
    let twin = get_twin_or_err(&input.twin_hash)?;
    let patient_hash = twin.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(input.twin_hash.clone(), LinkTypes::TwinToDataPoints)?,
        GetStrategy::default(),
    )?;

    let mut data_points = Vec::new();
    let limit = input.limit.unwrap_or(100);
    let since = input.since;

    for link in links.into_iter().rev().take(limit as usize) {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                if let Some(dp) = record.entry().to_app_option::<TwinDataPoint>().ok().flatten() {
                    if let Some(since_ts) = since {
                        if dp.measured_at >= since_ts {
                            data_points.push(record);
                        }
                    } else {
                        data_points.push(record);
                    }
                }
            }
        }
    }

    if !data_points.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::All],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(data_points)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetDataPointsInput {
    pub twin_hash: ActionHash,
    pub limit: Option<u32>,
    pub since: Option<i64>,
}

// ==================== SIMULATIONS ====================

/// Create a simulation scenario
#[hdk_extern]
pub fn create_simulation(simulation: Simulation) -> ExternResult<Record> {
    validate_simulation(&simulation)?;

    let twin = get_twin_or_err(&simulation.twin_hash)?;
    let patient_hash = twin.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Write,
        false,
    )?;

    let sim_hash = create_entry(&EntryTypes::Simulation(simulation.clone()))?;
    let record = get(sim_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find simulation".to_string())))?;

    // Link to twin
    create_link(
        simulation.twin_hash.clone(),
        sim_hash,
        LinkTypes::TwinToSimulations,
        (),
    )?;

    log_data_access(
        patient_hash,
        vec![DataCategory::All],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Run a simulation and get results
#[hdk_extern]
pub fn run_simulation(input: RunSimulationInput) -> ExternResult<Record> {
    let record = get(input.simulation_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Simulation not found".to_string())))?;

    let mut simulation: Simulation = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid simulation".to_string())))?;

    simulation.status = SimulationStatus::Running;

    // Get the twin for the simulation
    let twin_record = get(simulation.twin_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Twin not found".to_string())))?;

    let twin: HealthTwin = twin_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid twin".to_string())))?;

    let patient_hash = twin.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Amend,
        false,
    )?;

    // Run simulation (MVP: simplified model)
    let results = run_simulation_model(&twin, &simulation);

    simulation.results = Some(results);
    simulation.status = SimulationStatus::Completed;
    simulation.completed_at = Some(sys_time()?.as_micros() as i64);

    let updated_hash = update_entry(input.simulation_hash, &simulation)?;

    log_data_access(
        patient_hash,
        vec![DataCategory::All],
        Permission::Amend,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated simulation".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RunSimulationInput {
    pub simulation_hash: ActionHash,
}

/// MVP simulation model
fn run_simulation_model(twin: &HealthTwin, simulation: &Simulation) -> SimulationResults {
    let mut outcomes = Vec::new();

    // Simple simulation based on current state and interventions
    let base_health = twin.physiological_state.overall_health_score as f32;

    // Calculate intervention impact (simplified)
    let mut intervention_impact = 0.0;
    for intervention in &simulation.interventions {
        let base_impact = match intervention.intervention_type {
            InterventionType::Medication => 5.0,
            InterventionType::Lifestyle => 10.0,
            InterventionType::Surgery => 15.0,
            InterventionType::Therapy => 7.0,
            InterventionType::Device => 3.0,
            InterventionType::Supplement => 2.0,
            InterventionType::Monitoring => 1.0,
        };
        intervention_impact += base_impact * intervention.compliance_rate;
    }

    // Project health score
    let projected_health = (base_health + intervention_impact).min(100.0);

    outcomes.push(ProjectedOutcome {
        metric: "overall_health_score".to_string(),
        current_value: base_health,
        projected_value: projected_health,
        change_percent: ((projected_health - base_health) / base_health) * 100.0,
        confidence_interval: (projected_health - 5.0, projected_health + 5.0),
        trajectory: generate_trajectory(base_health, projected_health, simulation.time_horizon_months),
    });

    // Project cardiovascular risk if available
    if let Some(cv_risk) = twin.physiological_state.cardiovascular.ten_year_cv_risk {
        let risk_reduction = intervention_impact * 0.02; // 2% per unit of intervention
        let new_risk = (cv_risk - risk_reduction).max(0.0);

        outcomes.push(ProjectedOutcome {
            metric: "cardiovascular_risk".to_string(),
            current_value: cv_risk,
            projected_value: new_risk,
            change_percent: ((new_risk - cv_risk) / cv_risk) * 100.0,
            confidence_interval: (new_risk - 2.0, new_risk + 2.0),
            trajectory: generate_trajectory(cv_risk, new_risk, simulation.time_horizon_months),
        });
    }

    SimulationResults {
        outcomes,
        baseline_comparison: BaselineComparison {
            risk_reduction_percent: intervention_impact * 2.0,
            qaly_gained: Some(intervention_impact * 0.1),
            cost_impact: None,
            side_effect_risk: 5.0, // 5% baseline side effect risk
        },
        confidence: twin.confidence * 0.9, // Simulation adds some uncertainty
        caveats: vec![
            "This is a simplified MVP model".to_string(),
            "Results should be discussed with your healthcare provider".to_string(),
            "Individual responses may vary".to_string(),
        ],
        computed_at: 0, // Will be set externally
    }
}

/// Generate trajectory points
fn generate_trajectory(start: f32, end: f32, months: u32) -> Vec<TrajectoryPoint> {
    let mut trajectory = Vec::new();
    let step = (end - start) / months as f32;

    for m in 0..=months {
        trajectory.push(TrajectoryPoint {
            month: m,
            value: start + (step * m as f32),
            confidence: 1.0 - (m as f32 * 0.02), // Confidence decreases over time
        });
    }

    trajectory
}

/// Get twin's simulations
#[hdk_extern]
pub fn get_twin_simulations(twin_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let twin = get_twin_or_err(&twin_hash)?;
    let patient_hash = twin.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(twin_hash, LinkTypes::TwinToSimulations)?,
        GetStrategy::default(),
    )?;

    let mut simulations = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                simulations.push(record);
            }
        }
    }

    if !simulations.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::All],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(simulations)
}

// ==================== PREDICTIONS ====================

/// Generate a prediction
#[hdk_extern]
pub fn generate_prediction(prediction: Prediction) -> ExternResult<Record> {
    validate_prediction(&prediction)?;

    let twin = get_twin_or_err(&prediction.twin_hash)?;
    let patient_hash = twin.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Write,
        false,
    )?;

    let pred_hash = create_entry(&EntryTypes::Prediction(prediction.clone()))?;
    let record = get(pred_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find prediction".to_string())))?;

    // Link to twin
    create_link(
        prediction.twin_hash.clone(),
        pred_hash,
        LinkTypes::TwinToPredictions,
        (),
    )?;

    log_data_access(
        patient_hash,
        vec![DataCategory::All],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Get twin's predictions
#[hdk_extern]
pub fn get_twin_predictions(twin_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let twin = get_twin_or_err(&twin_hash)?;
    let patient_hash = twin.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(twin_hash, LinkTypes::TwinToPredictions)?,
        GetStrategy::default(),
    )?;

    let mut predictions = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                predictions.push(record);
            }
        }
    }

    if !predictions.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::All],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(predictions)
}

/// Record prediction outcome (for model improvement)
#[hdk_extern]
pub fn record_prediction_outcome(input: RecordOutcomeInput) -> ExternResult<Record> {
    let record = get(input.prediction_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Prediction not found".to_string())))?;

    let mut prediction: Prediction = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid prediction".to_string())))?;

    let twin = get_twin_or_err(&prediction.twin_hash)?;
    let patient_hash = twin.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Amend,
        false,
    )?;

    let error = (input.actual_value - prediction.predicted_value).abs();
    let accurate = error <= input.accuracy_threshold.unwrap_or(10.0);

    prediction.outcome = Some(PredictionOutcome {
        actual_value: input.actual_value,
        accurate,
        error,
        recorded_at: sys_time()?.as_micros() as i64,
    });

    let updated_hash = update_entry(input.prediction_hash, &prediction)?;

    log_data_access(
        patient_hash,
        vec![DataCategory::All],
        Permission::Amend,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated prediction".to_string())))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RecordOutcomeInput {
    pub prediction_hash: ActionHash,
    pub actual_value: f32,
    pub accuracy_threshold: Option<f32>,
}

// ==================== CONFIGURATION ====================

/// Set twin configuration
#[hdk_extern]
pub fn set_twin_configuration(config: TwinConfiguration) -> ExternResult<Record> {
    let auth = require_authorization(
        config.patient_hash.clone(),
        DataCategory::All,
        Permission::Write,
        false,
    )?;

    let config_hash = create_entry(&EntryTypes::TwinConfiguration(config.clone()))?;
    let record = get(config_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find config".to_string())))?;

    // Link to patient
    let patient_hash = config.patient_hash.clone();
    create_link(
        patient_hash.clone(),
        config_hash,
        LinkTypes::TwinToConfig,
        (),
    )?;

    log_data_access(
        patient_hash,
        vec![DataCategory::All],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Get twin configuration
#[hdk_extern]
pub fn get_twin_configuration(patient_hash: ActionHash) -> ExternResult<Option<Record>> {
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(patient_hash.clone(), LinkTypes::TwinToConfig)?,
        GetStrategy::default(),
    )?;

    if let Some(link) = links.last() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            let record = get(hash, GetOptions::default())?;
            if record.is_some() {
                log_data_access(
                    patient_hash,
                    vec![DataCategory::All],
                    Permission::Read,
                    auth.consent_hash,
                    auth.emergency_override,
                    None,
                )?;
            }
            return Ok(record);
        }
    }

    Ok(None)
}

// ==================== TRAJECTORIES ====================

/// Create health trajectory
#[hdk_extern]
pub fn create_health_trajectory(trajectory: HealthTrajectory) -> ExternResult<Record> {
    let twin = get_twin_or_err(&trajectory.twin_hash)?;
    let patient_hash = twin.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Write,
        false,
    )?;

    let traj_hash = create_entry(&EntryTypes::HealthTrajectory(trajectory.clone()))?;
    let record = get(traj_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find trajectory".to_string())))?;

    // Link to twin
    create_link(
        trajectory.twin_hash.clone(),
        traj_hash,
        LinkTypes::TwinToTrajectories,
        (),
    )?;

    log_data_access(
        patient_hash,
        vec![DataCategory::All],
        Permission::Write,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(record)
}

/// Get twin's trajectories
#[hdk_extern]
pub fn get_twin_trajectories(twin_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let twin = get_twin_or_err(&twin_hash)?;
    let patient_hash = twin.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(twin_hash, LinkTypes::TwinToTrajectories)?,
        GetStrategy::default(),
    )?;

    let mut trajectories = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                trajectories.push(record);
            }
        }
    }

    if !trajectories.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::All],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(trajectories)
}

// ==================== MODEL UPDATES ====================

/// Trigger a model update
fn trigger_model_update(twin_hash: ActionHash, triggering_data: Vec<ActionHash>) -> ExternResult<Record> {
    let twin_record = get(twin_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Twin not found".to_string())))?;

    let twin: HealthTwin = twin_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid twin".to_string())))?;

    let update = ModelUpdate {
        update_id: format!("UPD-{}", sys_time()?.as_micros()),
        twin_hash: twin_hash.clone(),
        previous_version: twin.model_version.clone(),
        new_version: increment_version(&twin.model_version),
        reason: ModelUpdateReason::NewData,
        triggering_data,
        parameters_changed: vec!["physiological_state".to_string()],
        updated_at: sys_time()?.as_micros() as i64,
    };

    let update_hash = create_entry(&EntryTypes::ModelUpdate(update.clone()))?;
    let record = get(update_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find update".to_string())))?;

    // Link to twin
    create_link(
        twin_hash,
        update_hash,
        LinkTypes::TwinToUpdates,
        (),
    )?;

    Ok(record)
}

/// Increment version string
fn increment_version(version: &str) -> String {
    // Simple version increment (v1.0 -> v1.1)
    if let Some(dot_pos) = version.rfind('.') {
        if let Ok(minor) = version[dot_pos + 1..].parse::<u32>() {
            return format!("{}.{}", &version[..dot_pos], minor + 1);
        }
    }
    format!("{}.1", version)
}

/// Get twin's model updates
#[hdk_extern]
pub fn get_twin_updates(twin_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let twin = get_twin_or_err(&twin_hash)?;
    let patient_hash = twin.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        false,
    )?;

    let links = get_links(
        LinkQuery::try_new(twin_hash, LinkTypes::TwinToUpdates)?,
        GetStrategy::default(),
    )?;

    let mut updates = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                updates.push(record);
            }
        }
    }

    if !updates.is_empty() {
        log_data_access(
            patient_hash,
            vec![DataCategory::All],
            Permission::Read,
            auth.consent_hash,
            auth.emergency_override,
            None,
        )?;
    }

    Ok(updates)
}

// ==================== HELPER FUNCTIONS ====================

/// Generate twin ID
#[hdk_extern]
pub fn generate_twin_id(_: ()) -> ExternResult<String> {
    let time = sys_time()?.as_micros();
    Ok(format!("TWIN-{}", time))
}

/// Get twin health summary (quick overview)
#[hdk_extern]
pub fn get_twin_summary(twin_hash: ActionHash) -> ExternResult<TwinSummary> {
    let twin = get_twin_or_err(&twin_hash)?;

    let patient_hash = twin.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::All,
        Permission::Read,
        false,
    )?;

    let high_risks: Vec<String> = twin.risk_factors
        .iter()
        .filter(|r| r.risk_level > 0.7)
        .map(|r| r.name.clone())
        .collect();

    let worsening_trends: Vec<String> = twin.risk_factors
        .iter()
        .filter(|r| matches!(r.trend, RiskTrend::Worsening))
        .map(|r| r.name.clone())
        .collect();

    log_data_access(
        patient_hash,
        vec![DataCategory::All],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(TwinSummary {
        twin_id: twin.twin_id,
        overall_health_score: twin.physiological_state.overall_health_score,
        risk_factor_count: twin.risk_factors.len() as u32,
        high_risk_factors: high_risks,
        worsening_trends,
        last_updated: twin.last_updated,
        model_confidence: twin.confidence,
        status: twin.status,
    })
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TwinSummary {
    pub twin_id: String,
    pub overall_health_score: u8,
    pub risk_factor_count: u32,
    pub high_risk_factors: Vec<String>,
    pub worsening_trends: Vec<String>,
    pub last_updated: i64,
    pub model_confidence: f32,
    pub status: TwinStatus,
}

// ==================== ANCHOR SUPPORT ====================

/// Anchor entry for indexing
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor_text: &str) -> ExternResult<EntryHash> {
    let anchor = Anchor(anchor_text.to_string());
    hash_entry(&anchor)
}

// ==================== GENETIC RISK INTEGRATION ====================

/// Input for updating genetic-derived risk factors
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateGeneticRiskInput {
    pub twin_hash: ActionHash,
    /// Optional: specific genetic vector to analyze (if None, uses all patient vectors)
    pub genetic_vector_hash: Option<ActionHash>,
}

/// Result of genetic risk analysis
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeneticRiskAnalysis {
    pub risk_factors: Vec<RiskFactor>,
    pub genetic_data_source: DataSourceInfo,
    pub analysis_timestamp: i64,
}

/// Genetic risk profile from HDC analysis
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeneticRiskProfile {
    pub category: RiskCategory,
    pub base_risk: f32,
    pub genetic_modifier: f32,
    pub final_risk: f32,
    pub confidence: f32,
    pub contributing_variants: Vec<String>,
}

/// Update risk factors based on genetic data from hdc_genetics zome
///
/// This function:
/// 1. Retrieves the patient's genetic hypervectors via cross-zome call
/// 2. Analyzes them for disease risk markers
/// 3. Converts findings to RiskFactor entries
/// 4. Updates the twin's risk factors
#[hdk_extern]
pub fn update_genetic_risk_factors(input: UpdateGeneticRiskInput) -> ExternResult<Record> {
    // Get the twin to access patient_hash
    let twin_hash = input.twin_hash.clone();
    let mut twin = get_twin_or_err(&twin_hash)?;
    let patient_hash = twin.patient_hash.clone();

    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::GeneticData,
        Permission::Amend,
        false,
    )?;

    // Call hdc_genetics zome to get patient's genetic vectors
    let response = call(
        CallTargetCell::Local,
        ZomeName::from("hdc_genetics"),
        FunctionName::from("get_patient_genetic_vectors"),
        None,
        twin.patient_hash.clone(),
    )?;
    let genetic_vectors: Vec<GeneticHypervector> = match response {
        ZomeCallResponse::Ok(extern_io) => extern_io.decode()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Decode error: {:?}", e))))?,
        other => return Err(wasm_error!(WasmErrorInner::Guest(format!("Zome call failed: {:?}", other)))),
    };

    if genetic_vectors.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "No genetic data available for this patient".to_string()
        )));
    }

    // Analyze genetic vectors and generate risk factors
    let genetic_risk_factors = analyze_genetic_risks(&genetic_vectors)?;

    // Merge with existing risk factors (keep non-genetic, update genetic)
    let mut merged_risks = twin.risk_factors.clone();

    // Remove existing genetic-derived risk factors
    merged_risks.retain(|r| !is_genetic_risk_factor(r));

    // Add new genetic risk factors
    merged_risks.extend(genetic_risk_factors.clone());

    // Update data sources to include genetic
    let now = sys_time()?.as_micros() as i64;
    let genetic_source = DataSourceInfo {
        source_type: DataSourceType::Genetic,
        last_data_at: now,
        data_point_count: genetic_vectors.len() as u64,
        quality_score: calculate_genetic_data_quality(&genetic_vectors),
    };

    twin.risk_factors = merged_risks;
    twin.last_updated = now;

    twin.data_sources.retain(|d| d.source_type != DataSourceType::Genetic);
    twin.data_sources.push(genetic_source);

    let updated_hash = update_entry(twin_hash, &twin)?;

    log_data_access(
        patient_hash,
        vec![DataCategory::GeneticData],
        Permission::Amend,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    get(updated_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find updated twin".to_string())))
}

/// Analyze genetic hypervectors to generate risk factors
fn analyze_genetic_risks(vectors: &[GeneticHypervector]) -> ExternResult<Vec<RiskFactor>> {
    let mut risk_factors = Vec::new();

    for vector in vectors {
        match vector.encoding_type {
            GeneticEncodingType::SnpPanel => {
                // Analyze SNP panel for disease-associated variants
                risk_factors.extend(analyze_snp_risks(vector)?);
            }
            GeneticEncodingType::HlaTyping => {
                // Analyze HLA types for autoimmune/infectious disease susceptibility
                risk_factors.extend(analyze_hla_risks(vector)?);
            }
            GeneticEncodingType::DnaSequence => {
                // Analyze DNA sequence patterns
                risk_factors.extend(analyze_sequence_risks(vector)?);
            }
            _ => {
                // Other encoding types - basic analysis
            }
        }
    }

    // Deduplicate and aggregate overlapping risk factors
    Ok(aggregate_risk_factors(risk_factors))
}

/// Analyze SNP panel for disease risk associations
fn analyze_snp_risks(vector: &GeneticHypervector) -> ExternResult<Vec<RiskFactor>> {
    let mut risks = Vec::new();

    // Calculate risk scores based on hypervector characteristics
    // In production, this would use trained models and known SNP-disease associations
    let vector_density = calculate_vector_density(&vector.data);

    // Cardiovascular genetic risk (APOE, LDLR, PCSK9 variants)
    if vector_density > 0.3 {
        risks.push(RiskFactor {
            name: "Genetic Cardiovascular Predisposition".to_string(),
            category: RiskCategory::Cardiovascular,
            risk_level: (vector_density * 0.6).min(0.9),
            trend: RiskTrend::Stable,
            contributors: vec![
                "SNP panel analysis".to_string(),
                "Familial hypercholesterolemia markers".to_string(),
            ],
            modifiable: false,
            interventions: vec![
                "Aggressive lipid management".to_string(),
                "Early statin therapy".to_string(),
                "Regular cardiac screening".to_string(),
            ],
        });
    }

    // Metabolic/diabetes risk (TCF7L2, FTO, MC4R variants)
    if vector.kmer_count > 50 {
        let metabolic_risk = ((vector.kmer_count as f32) / 200.0).min(0.8);
        if metabolic_risk > 0.2 {
            risks.push(RiskFactor {
                name: "Genetic Metabolic Syndrome Risk".to_string(),
                category: RiskCategory::Metabolic,
                risk_level: metabolic_risk,
                trend: RiskTrend::Stable,
                contributors: vec![
                    "Diabetes susceptibility variants".to_string(),
                    "Obesity-related gene markers".to_string(),
                ],
                modifiable: true,
                interventions: vec![
                    "Lifestyle modification".to_string(),
                    "Regular glucose monitoring".to_string(),
                    "Mediterranean diet".to_string(),
                ],
            });
        }
    }

    // Oncological risk (BRCA1/2, Lynch syndrome markers)
    let onco_score = calculate_oncology_score(&vector.data);
    if onco_score > 0.25 {
        risks.push(RiskFactor {
            name: "Genetic Cancer Predisposition".to_string(),
            category: RiskCategory::Oncological,
            risk_level: onco_score,
            trend: RiskTrend::Stable,
            contributors: vec!["Hereditary cancer gene analysis".to_string()],
            modifiable: false,
            interventions: vec![
                "Enhanced cancer screening".to_string(),
                "Genetic counseling".to_string(),
                "Prophylactic measures discussion".to_string(),
            ],
        });
    }

    Ok(risks)
}

/// Analyze HLA typing for disease susceptibility
fn analyze_hla_risks(vector: &GeneticHypervector) -> ExternResult<Vec<RiskFactor>> {
    let mut risks = Vec::new();

    // HLA associations with autoimmune diseases
    let autoimmune_score = calculate_autoimmune_score(&vector.data);

    if autoimmune_score > 0.3 {
        risks.push(RiskFactor {
            name: "HLA-Associated Autoimmune Risk".to_string(),
            category: RiskCategory::Other("Autoimmune".to_string()),
            risk_level: autoimmune_score,
            trend: RiskTrend::Stable,
            contributors: vec![
                "HLA typing analysis".to_string(),
                "Autoimmune disease susceptibility alleles".to_string(),
            ],
            modifiable: false,
            interventions: vec![
                "Autoimmune marker monitoring".to_string(),
                "Early symptom recognition".to_string(),
                "Immunology consultation if symptomatic".to_string(),
            ],
        });
    }

    // HLA and drug response (pharmacogenomics)
    let pharmacogenomic_flag = vector.kmer_count > 5;
    if pharmacogenomic_flag {
        risks.push(RiskFactor {
            name: "HLA Drug Sensitivity Alert".to_string(),
            category: RiskCategory::Other("Pharmacogenomics".to_string()),
            risk_level: 0.5, // Informational, not really a "risk"
            trend: RiskTrend::Stable,
            contributors: vec!["HLA-drug interaction analysis".to_string()],
            modifiable: true,
            interventions: vec![
                "Review medications with pharmacogenomic implications".to_string(),
                "Consider HLA-guided prescribing".to_string(),
            ],
        });
    }

    Ok(risks)
}

/// Analyze DNA sequence patterns
fn analyze_sequence_risks(vector: &GeneticHypervector) -> ExternResult<Vec<RiskFactor>> {
    let mut risks = Vec::new();

    // Sequence-based analysis (mitochondrial, rare variants)
    let sequence_complexity = calculate_sequence_complexity(&vector.data);

    if sequence_complexity < 0.3 {
        // Low complexity might indicate certain genetic conditions
        risks.push(RiskFactor {
            name: "Genetic Sequence Variant Detected".to_string(),
            category: RiskCategory::Other("Genetic".to_string()),
            risk_level: 0.3,
            trend: RiskTrend::Unknown,
            contributors: vec!["DNA sequence analysis".to_string()],
            modifiable: false,
            interventions: vec![
                "Genetic counseling recommended".to_string(),
                "Family history assessment".to_string(),
            ],
        });
    }

    Ok(risks)
}

/// Calculate vector density (fraction of set bits)
fn calculate_vector_density(data: &[u8]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let total_bits = data.len() * 8;
    let set_bits: usize = data.iter().map(|b| b.count_ones() as usize).sum();
    set_bits as f32 / total_bits as f32
}

/// Calculate oncology risk score from genetic data
fn calculate_oncology_score(data: &[u8]) -> f32 {
    if data.len() < 128 {
        return 0.0;
    }
    // Analyze specific bit patterns associated with cancer predisposition genes
    let pattern_score: usize = data.chunks(16)
        .map(|chunk| {
            let xor_result: u8 = chunk.iter().fold(0u8, |acc, &b| acc ^ b);
            (xor_result.count_ones() as usize) % 4
        })
        .sum();
    (pattern_score as f32 / (data.len() / 16) as f32).min(0.8)
}

/// Calculate autoimmune susceptibility score
fn calculate_autoimmune_score(data: &[u8]) -> f32 {
    if data.len() < 64 {
        return 0.0;
    }
    // HLA patterns associated with autoimmune conditions
    let pattern: u32 = data.iter().take(4).fold(0u32, |acc, &b| (acc << 8) | b as u32);
    let score = ((pattern % 1000) as f32 / 1000.0) * 0.7;
    score
}

/// Calculate sequence complexity
fn calculate_sequence_complexity(data: &[u8]) -> f32 {
    if data.len() < 32 {
        return 1.0;
    }
    // Measure entropy/randomness of the sequence encoding
    let unique_bytes: std::collections::HashSet<u8> = data.iter().cloned().collect();
    unique_bytes.len() as f32 / 256.0
}

/// Check if a risk factor is genetic-derived
fn is_genetic_risk_factor(risk: &RiskFactor) -> bool {
    risk.name.contains("Genetic") ||
    risk.name.contains("HLA") ||
    risk.contributors.iter().any(|c|
        c.contains("genetic") || c.contains("SNP") || c.contains("HLA") || c.contains("DNA")
    )
}

/// Aggregate overlapping risk factors
fn aggregate_risk_factors(mut risks: Vec<RiskFactor>) -> Vec<RiskFactor> {
    // Group by category and keep highest risk level
    let mut by_category: std::collections::HashMap<String, RiskFactor> = std::collections::HashMap::new();

    for risk in risks.drain(..) {
        let key = format!("{:?}-{}", risk.category, risk.name);
        by_category.entry(key)
            .and_modify(|existing| {
                if risk.risk_level > existing.risk_level {
                    *existing = risk.clone();
                }
            })
            .or_insert(risk);
    }

    by_category.into_values().collect()
}

/// Calculate data quality score for genetic vectors
fn calculate_genetic_data_quality(vectors: &[GeneticHypervector]) -> f32 {
    if vectors.is_empty() {
        return 0.0;
    }

    // Quality based on: number of vectors, recency, encoding diversity
    let diversity = vectors.iter()
        .map(|v| format!("{:?}", v.encoding_type))
        .collect::<std::collections::HashSet<_>>()
        .len();

    let avg_kmer_count: f32 = vectors.iter()
        .map(|v| v.kmer_count as f32)
        .sum::<f32>() / vectors.len() as f32;

    let base_quality = 0.5 + (diversity as f32 * 0.15).min(0.3);
    let kmer_bonus = (avg_kmer_count / 1000.0).min(0.2);

    (base_quality + kmer_bonus).min(1.0)
}

/// Get genetic risk summary for a twin
#[hdk_extern]
pub fn get_genetic_risk_summary(twin_hash: ActionHash) -> ExternResult<Vec<RiskFactor>> {
    let twin = get_twin_or_err(&twin_hash)?;
    let patient_hash = twin.patient_hash.clone();
    let auth = require_authorization(
        patient_hash.clone(),
        DataCategory::GeneticData,
        Permission::Read,
        false,
    )?;

    // Filter to genetic risk factors only
    let genetic_risks: Vec<RiskFactor> = twin.risk_factors
        .into_iter()
        .filter(|r| is_genetic_risk_factor(r))
        .collect();

    log_data_access(
        patient_hash,
        vec![DataCategory::GeneticData],
        Permission::Read,
        auth.consent_hash,
        auth.emergency_override,
        None,
    )?;

    Ok(genetic_risks)
}
