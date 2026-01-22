//! Health Twin Coordinator Zome
//!
//! Provides extern functions for the Digital Health Twin system.

use hdk::prelude::*;
use twin_integrity::*;

// ==================== HEALTH TWIN MANAGEMENT ====================

/// Create a new health twin for a patient
#[hdk_extern]
pub fn create_health_twin(twin: HealthTwin) -> ExternResult<Record> {
    validate_health_twin(&twin)?;

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

    Ok(record)
}

/// Get patient's health twin
#[hdk_extern]
pub fn get_patient_twin(patient_hash: ActionHash) -> ExternResult<Option<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::PatientToTwin)?,
        GetStrategy::default(),
    )?;

    // Get the most recent twin
    if let Some(link) = links.last() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            return get(hash, GetOptions::default());
        }
    }

    Ok(None)
}

/// Update twin's physiological state
#[hdk_extern]
pub fn update_twin_state(input: UpdateTwinStateInput) -> ExternResult<Record> {
    let record = get(input.twin_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Twin not found".to_string())))?;

    let mut twin: HealthTwin = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid twin".to_string())))?;

    twin.physiological_state = input.new_state;
    twin.last_updated = sys_time()?.as_micros() as i64;

    let updated_hash = update_entry(input.twin_hash, &twin)?;

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
    let record = get(input.twin_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Twin not found".to_string())))?;

    let mut twin: HealthTwin = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid twin".to_string())))?;

    twin.risk_factors = input.risk_factors;
    twin.last_updated = sys_time()?.as_micros() as i64;

    let updated_hash = update_entry(input.twin_hash, &twin)?;

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
    let record = get(input.twin_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Twin not found".to_string())))?;

    let mut twin: HealthTwin = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid twin".to_string())))?;

    twin.status = input.status;
    twin.last_updated = sys_time()?.as_micros() as i64;

    let updated_hash = update_entry(input.twin_hash, &twin)?;

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

    Ok(record)
}

/// Get recent data points for a twin
#[hdk_extern]
pub fn get_twin_data_points(input: GetDataPointsInput) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery::try_new(input.twin_hash, LinkTypes::TwinToDataPoints)?,
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

    // Run simulation (MVP: simplified model)
    let results = run_simulation_model(&twin, &simulation);

    simulation.results = Some(results);
    simulation.status = SimulationStatus::Completed;
    simulation.completed_at = Some(sys_time()?.as_micros() as i64);

    let updated_hash = update_entry(input.simulation_hash, &simulation)?;

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

    Ok(simulations)
}

// ==================== PREDICTIONS ====================

/// Generate a prediction
#[hdk_extern]
pub fn generate_prediction(prediction: Prediction) -> ExternResult<Record> {
    validate_prediction(&prediction)?;

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

    Ok(record)
}

/// Get twin's predictions
#[hdk_extern]
pub fn get_twin_predictions(twin_hash: ActionHash) -> ExternResult<Vec<Record>> {
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

    let error = (input.actual_value - prediction.predicted_value).abs();
    let accurate = error <= input.accuracy_threshold.unwrap_or(10.0);

    prediction.outcome = Some(PredictionOutcome {
        actual_value: input.actual_value,
        accurate,
        error,
        recorded_at: sys_time()?.as_micros() as i64,
    });

    let updated_hash = update_entry(input.prediction_hash, &prediction)?;

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
    let config_hash = create_entry(&EntryTypes::TwinConfiguration(config.clone()))?;
    let record = get(config_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find config".to_string())))?;

    // Link to patient
    create_link(
        config.patient_hash.clone(),
        config_hash,
        LinkTypes::TwinToConfig,
        (),
    )?;

    Ok(record)
}

/// Get twin configuration
#[hdk_extern]
pub fn get_twin_configuration(patient_hash: ActionHash) -> ExternResult<Option<Record>> {
    let links = get_links(
        LinkQuery::try_new(patient_hash, LinkTypes::TwinToConfig)?,
        GetStrategy::default(),
    )?;

    if let Some(link) = links.last() {
        if let Some(hash) = link.target.clone().into_action_hash() {
            return get(hash, GetOptions::default());
        }
    }

    Ok(None)
}

// ==================== TRAJECTORIES ====================

/// Create health trajectory
#[hdk_extern]
pub fn create_health_trajectory(trajectory: HealthTrajectory) -> ExternResult<Record> {
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

    Ok(record)
}

/// Get twin's trajectories
#[hdk_extern]
pub fn get_twin_trajectories(twin_hash: ActionHash) -> ExternResult<Vec<Record>> {
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
    let twin_record = get(twin_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Twin not found".to_string())))?;

    let twin: HealthTwin = twin_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid twin".to_string())))?;

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
