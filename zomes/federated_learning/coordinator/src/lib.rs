//! Federated Learning Coordinator Zome
//!
//! Provides functions for managing privacy-preserving distributed machine learning
//! including project management, training rounds, and model aggregation.

use hdk::prelude::*;
use federated_learning_integrity::*;

/// Input for creating a learning project
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateProjectInput {
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub task_type: TaskType,
    pub data_schema: String,
    pub model_architecture: String,
    pub initial_model_hash: Option<ActionHash>,
    pub aggregation_strategy: AggregationStrategy,
    pub privacy_mechanism: PrivacyMechanism,
    pub min_participants: u32,
    pub max_participants: Option<u32>,
    pub min_samples_per_participant: u32,
    pub planned_rounds: u32,
    pub irb_approval_hash: Option<ActionHash>,
}

/// Input for joining a project
#[derive(Serialize, Deserialize, Debug)]
pub struct JoinProjectInput {
    pub project_hash: ActionHash,
    pub public_key: Vec<u8>,
    pub sample_count: u32,
    pub data_quality_score: Option<u32>,
}

/// Input for starting a training round
#[derive(Serialize, Deserialize, Debug)]
pub struct StartRoundInput {
    pub round_id: String,
    pub project_hash: ActionHash,
    pub learning_rate: String,
    pub batch_size: u32,
    pub local_epochs: u32,
    pub deadline_hours: u32,
}

/// Input for submitting a model update
#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitUpdateInput {
    pub update_id: String,
    pub round_hash: ActionHash,
    pub encrypted_update: Vec<u8>,
    pub encryption_key_hash: Option<ActionHash>,
    pub samples_used: u32,
    pub local_loss: Option<String>,
    pub local_metrics: Option<String>,
    pub noise_scale: Option<String>,
    pub computation_time: u32,
}

/// Input for aggregating model updates
#[derive(Serialize, Deserialize, Debug)]
pub struct AggregateInput {
    pub model_id: String,
    pub round_hash: ActionHash,
    pub model_parameters: Vec<u8>,
    pub aggregation_method: String,
    pub global_loss: Option<String>,
    pub global_metrics: Option<String>,
}

/// Input for model evaluation
#[derive(Serialize, Deserialize, Debug)]
pub struct EvaluateModelInput {
    pub evaluation_id: String,
    pub model_hash: ActionHash,
    pub test_set_description: String,
    pub test_samples: u32,
    pub primary_metric_name: String,
    pub primary_metric_value: String,
    pub metrics: String,
    pub confusion_matrix: Option<String>,
}

/// Input for initializing privacy budget
#[derive(Serialize, Deserialize, Debug)]
pub struct InitBudgetInput {
    pub budget_id: String,
    pub project_hash: ActionHash,
    pub total_epsilon: String,
    pub delta: String,
    pub budget_per_round: String,
}

/// Create a new federated learning project
#[hdk_extern]
pub fn create_learning_project(input: CreateProjectInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;
    let coordinator_hash = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let project = LearningProject {
        project_id: input.project_id,
        name: input.name,
        description: input.description,
        task_type: input.task_type,
        data_schema: input.data_schema,
        model_architecture: input.model_architecture,
        initial_model_hash: input.initial_model_hash,
        aggregation_strategy: input.aggregation_strategy,
        privacy_mechanism: input.privacy_mechanism,
        min_participants: input.min_participants,
        max_participants: input.max_participants,
        min_samples_per_participant: input.min_samples_per_participant,
        planned_rounds: input.planned_rounds,
        status: ProjectStatus::Recruiting,
        irb_approval_hash: input.irb_approval_hash,
        coordinator_hash,
        created_at: now,
        updated_at: now,
    };

    let action_hash = create_entry(EntryTypes::LearningProject(project))?;

    // Link from all projects anchor
    let all_anchor = anchor_hash("all_fl_projects")?;
    create_link(all_anchor, action_hash.clone(), LinkTypes::AllProjects, ())?;

    // Link by status
    let status_anchor = anchor_hash("fl_status_recruiting")?;
    create_link(
        status_anchor,
        action_hash.clone(),
        LinkTypes::ProjectsByStatus,
        (),
    )?;

    Ok(action_hash)
}

/// Get all federated learning projects
#[hdk_extern]
pub fn get_all_projects(_: ()) -> ExternResult<Vec<Record>> {
    let anchor = anchor_hash("all_fl_projects")?;
    let links = get_links(LinkQuery::try_new(anchor, LinkTypes::AllProjects)?, GetStrategy::default())?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Get active projects (recruiting or training)
#[hdk_extern]
pub fn get_active_projects(_: ()) -> ExternResult<Vec<Record>> {
    let mut records = Vec::new();

    // Get recruiting projects
    let recruiting_anchor = anchor_hash("fl_status_recruiting")?;
    let recruiting_links = get_links(LinkQuery::try_new(recruiting_anchor, LinkTypes::ProjectsByStatus)?, GetStrategy::default())?;

    for link in recruiting_links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    // Get training projects
    let training_anchor = anchor_hash("fl_status_training")?;
    let training_links = get_links(LinkQuery::try_new(training_anchor, LinkTypes::ProjectsByStatus)?, GetStrategy::default())?;

    for link in training_links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Join a federated learning project
#[hdk_extern]
pub fn join_project(input: JoinProjectInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;
    let participant_hash = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let participant = ProjectParticipant {
        project_hash: input.project_hash.clone(),
        participant_hash: participant_hash.clone(),
        public_key: input.public_key,
        sample_count: input.sample_count,
        data_quality_score: input.data_quality_score,
        is_active: true,
        rounds_participated: vec![],
        joined_at: now,
        last_active_at: now,
    };

    let action_hash = create_entry(EntryTypes::ProjectParticipant(participant))?;

    // Link from project
    create_link(
        input.project_hash.clone(),
        action_hash.clone(),
        LinkTypes::ProjectToParticipants,
        (),
    )?;

    // Check if we have enough participants to start
    check_and_update_project_status(input.project_hash)?;

    Ok(action_hash)
}

/// Get participants for a project
#[hdk_extern]
pub fn get_project_participants(project_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(project_hash, LinkTypes::ProjectToParticipants)?, GetStrategy::default())?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Start a new training round
#[hdk_extern]
pub fn start_training_round(input: StartRoundInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    // Get project and latest model
    let project_record = get(input.project_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Project not found".to_string())))?;

    let project: LearningProject = project_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid project entry".to_string())))?;

    // Get round number
    let existing_rounds = get_project_rounds(input.project_hash.clone())?;
    let round_number = (existing_rounds.len() as u32) + 1;

    // Get starting model (latest aggregated or initial)
    let starting_model_hash = get_latest_model_hash(input.project_hash.clone())?
        .or(project.initial_model_hash)
        .ok_or(wasm_error!(WasmErrorInner::Guest("No starting model available".to_string())))?;

    // Get active participants
    let participants = get_project_participants(input.project_hash.clone())?;
    let expected_participants: Vec<ActionHash> = participants
        .iter()
        .filter_map(|r| {
            r.entry()
                .to_app_option::<ProjectParticipant>()
                .ok()
                .flatten()
                .filter(|p| p.is_active)
                .map(|p| p.participant_hash)
        })
        .collect();

    // Calculate deadline
    let deadline_micros = (input.deadline_hours as i64) * 60 * 60 * 1_000_000;
    let deadline = Timestamp::from_micros(now.as_micros() + deadline_micros);

    let round = TrainingRound {
        round_id: input.round_id,
        project_hash: input.project_hash.clone(),
        round_number,
        status: RoundStatus::AwaitingUpdates,
        starting_model_hash,
        aggregated_model_hash: None,
        expected_participants: expected_participants.clone(),
        submitted_participants: vec![],
        min_submissions: (expected_participants.len() as u32).max(1) / 2, // At least half
        learning_rate: input.learning_rate,
        batch_size: input.batch_size,
        local_epochs: input.local_epochs,
        deadline,
        started_at: now,
        completed_at: None,
    };

    let action_hash = create_entry(EntryTypes::TrainingRound(round))?;

    // Link from project
    create_link(
        input.project_hash,
        action_hash.clone(),
        LinkTypes::ProjectToRounds,
        (),
    )?;

    Ok(action_hash)
}

/// Get training rounds for a project
#[hdk_extern]
pub fn get_project_rounds(project_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(project_hash, LinkTypes::ProjectToRounds)?, GetStrategy::default())?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Submit a model update for a training round
#[hdk_extern]
pub fn submit_model_update(input: SubmitUpdateInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;
    let participant_hash = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let update = ModelUpdate {
        update_id: input.update_id,
        round_hash: input.round_hash.clone(),
        participant_hash: participant_hash.clone(),
        encrypted_update: input.encrypted_update,
        encryption_key_hash: input.encryption_key_hash,
        samples_used: input.samples_used,
        local_loss: input.local_loss,
        local_metrics: input.local_metrics,
        noise_scale: input.noise_scale,
        computation_time: input.computation_time,
        submitted_at: now,
    };

    let action_hash = create_entry(EntryTypes::ModelUpdate(update))?;

    // Link from round
    create_link(
        input.round_hash.clone(),
        action_hash.clone(),
        LinkTypes::RoundToUpdates,
        (),
    )?;

    // Link from participant
    create_link(
        participant_hash,
        action_hash.clone(),
        LinkTypes::ParticipantToUpdates,
        (),
    )?;

    // Update round's submitted participants
    update_round_submissions(input.round_hash)?;

    Ok(action_hash)
}

/// Get updates for a training round
#[hdk_extern]
pub fn get_round_updates(round_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(round_hash, LinkTypes::RoundToUpdates)?, GetStrategy::default())?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Aggregate model updates and create new global model
#[hdk_extern]
pub fn aggregate_updates(input: AggregateInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;

    // Get round info
    let round_record = get(input.round_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Round not found".to_string())))?;

    let round: TrainingRound = round_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid round entry".to_string())))?;

    // Count updates
    let updates = get_round_updates(input.round_hash.clone())?;
    let total_samples: u32 = updates
        .iter()
        .filter_map(|r| {
            r.entry()
                .to_app_option::<ModelUpdate>()
                .ok()
                .flatten()
                .map(|u| u.samples_used)
        })
        .sum();

    let model = AggregatedModel {
        model_id: input.model_id,
        project_hash: round.project_hash.clone(),
        round_hash: input.round_hash.clone(),
        version: round.round_number,
        model_parameters: input.model_parameters,
        parameter_count: 0, // Would be calculated from actual parameters
        participants_aggregated: updates.len() as u32,
        total_samples,
        aggregation_method: input.aggregation_method,
        global_loss: input.global_loss,
        global_metrics: input.global_metrics,
        previous_model_hash: Some(round.starting_model_hash.clone()),
        created_at: now,
    };

    let action_hash = create_entry(EntryTypes::AggregatedModel(model))?;

    // Link from round
    create_link(
        input.round_hash.clone(),
        action_hash.clone(),
        LinkTypes::RoundToModel,
        (),
    )?;

    // Link from project
    let project_hash = round.project_hash.clone();
    create_link(
        project_hash,
        action_hash.clone(),
        LinkTypes::ProjectToModels,
        (),
    )?;

    // Update round status
    let mut updated_round = round;
    updated_round.status = RoundStatus::Completed;
    updated_round.aggregated_model_hash = Some(action_hash.clone());
    updated_round.completed_at = Some(now);
    update_entry(input.round_hash, updated_round)?;

    Ok(action_hash)
}

/// Get all models for a project
#[hdk_extern]
pub fn get_project_models(project_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(project_hash, LinkTypes::ProjectToModels)?, GetStrategy::default())?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Evaluate a model
#[hdk_extern]
pub fn evaluate_model(input: EvaluateModelInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;
    let evaluator_hash = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let evaluation = ModelEvaluation {
        evaluation_id: input.evaluation_id,
        model_hash: input.model_hash.clone(),
        evaluator_hash,
        test_set_description: input.test_set_description,
        test_samples: input.test_samples,
        primary_metric_name: input.primary_metric_name,
        primary_metric_value: input.primary_metric_value,
        metrics: input.metrics,
        confusion_matrix: input.confusion_matrix,
        evaluated_at: now,
    };

    let action_hash = create_entry(EntryTypes::ModelEvaluation(evaluation))?;

    // Link from model
    create_link(
        input.model_hash,
        action_hash.clone(),
        LinkTypes::ModelToEvaluations,
        (),
    )?;

    Ok(action_hash)
}

/// Get evaluations for a model
#[hdk_extern]
pub fn get_model_evaluations(model_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let links = get_links(LinkQuery::try_new(model_hash, LinkTypes::ModelToEvaluations)?, GetStrategy::default())?;

    let mut records = Vec::new();
    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Initialize privacy budget for a participant
#[hdk_extern]
pub fn initialize_privacy_budget(input: InitBudgetInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let agent = agent_info()?.agent_initial_pubkey;
    let participant_hash = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let budget = PrivacyBudget {
        budget_id: input.budget_id,
        project_hash: input.project_hash.clone(),
        participant_hash: participant_hash.clone(),
        total_epsilon: input.total_epsilon,
        consumed_epsilon: "0.0".to_string(),
        delta: input.delta,
        budget_per_round: input.budget_per_round,
        rounds_spent: vec![],
        is_exhausted: false,
        created_at: now,
        updated_at: now,
    };

    let action_hash = create_entry(EntryTypes::PrivacyBudget(budget))?;

    // Link from participant
    create_link(
        participant_hash,
        action_hash.clone(),
        LinkTypes::ParticipantToBudget,
        (),
    )?;

    Ok(action_hash)
}

/// Get privacy budget for current participant in a project
#[hdk_extern]
pub fn get_my_budget(project_hash: ActionHash) -> ExternResult<Option<Record>> {
    let agent = agent_info()?.agent_initial_pubkey;
    let participant_hash = ActionHash::from_raw_36(agent.get_raw_36().to_vec());

    let links = get_links(LinkQuery::try_new(participant_hash, LinkTypes::ParticipantToBudget)?, GetStrategy::default())?;

    for link in links {
        if let Some(hash) = link.target.into_action_hash() {
            if let Some(record) = get(hash, GetOptions::default())? {
                let budget: PrivacyBudget = record
                    .entry()
                    .to_app_option()
                    .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
                    .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid budget".to_string())))?;

                if budget.project_hash == project_hash {
                    return Ok(Some(record));
                }
            }
        }
    }

    Ok(None)
}

// Helper functions

/// Anchor for linking entries
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Anchor(pub String);

fn anchor_hash(anchor: &str) -> ExternResult<AnyLinkableHash> {
    let anchor = Anchor(anchor.to_string());
    Ok(hash_entry(&anchor)?.into())
}

fn check_and_update_project_status(project_hash: ActionHash) -> ExternResult<()> {
    let record = get(project_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Project not found".to_string())))?;

    let mut project: LearningProject = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid project".to_string())))?;

    if project.status == ProjectStatus::Recruiting {
        let participants = get_project_participants(project_hash.clone())?;
        if participants.len() as u32 >= project.min_participants {
            project.status = ProjectStatus::Ready;
            project.updated_at = sys_time()?;
            update_entry(project_hash, project)?;
        }
    }

    Ok(())
}

fn update_round_submissions(round_hash: ActionHash) -> ExternResult<()> {
    let record = get(round_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Round not found".to_string())))?;

    let mut round: TrainingRound = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid round".to_string())))?;

    let updates = get_round_updates(round_hash.clone())?;
    let submitted: Vec<ActionHash> = updates
        .iter()
        .filter_map(|r| {
            r.entry()
                .to_app_option::<ModelUpdate>()
                .ok()
                .flatten()
                .map(|u| u.participant_hash)
        })
        .collect();

    round.submitted_participants = submitted;
    update_entry(round_hash, round)?;

    Ok(())
}

fn get_latest_model_hash(project_hash: ActionHash) -> ExternResult<Option<ActionHash>> {
    let models = get_project_models(project_hash)?;

    let mut latest: Option<(u32, ActionHash)> = None;
    for record in models {
        let model: AggregatedModel = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
            .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid model".to_string())))?;

        match &latest {
            None => latest = Some((model.version, record.action_address().clone())),
            Some((v, _)) if model.version > *v => {
                latest = Some((model.version, record.action_address().clone()))
            }
            _ => {}
        }
    }

    Ok(latest.map(|(_, h)| h))
}
