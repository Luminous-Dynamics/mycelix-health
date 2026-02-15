//! Federated Learning Integrity Zome
//!
//! Defines entry types for privacy-preserving distributed machine learning
//! where participants can train models without sharing raw data.

use hdi::prelude::*;

/// Type of machine learning task
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TaskType {
    /// Binary classification (e.g., disease present/absent)
    BinaryClassification,
    /// Multi-class classification (e.g., disease stage)
    MultiClassClassification,
    /// Regression (e.g., predicting lab values)
    Regression,
    /// Survival analysis (e.g., time to event)
    SurvivalAnalysis,
    /// Clustering (e.g., patient subgroups)
    Clustering,
    /// Anomaly detection (e.g., unusual patterns)
    AnomalyDetection,
}

/// Status of a learning project
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ProjectStatus {
    /// Project created, awaiting participants
    Recruiting,
    /// Minimum participants reached, ready to start
    Ready,
    /// Training actively in progress
    Training,
    /// Training paused
    Paused,
    /// Training completed successfully
    Completed,
    /// Project cancelled or failed
    Terminated,
}

/// Status of a training round
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RoundStatus {
    /// Waiting for participants to submit updates
    AwaitingUpdates,
    /// Aggregating participant updates
    Aggregating,
    /// Distributing aggregated model
    Distributing,
    /// Round completed successfully
    Completed,
    /// Round failed (insufficient participants, etc.)
    Failed,
}

/// Type of aggregation strategy
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum AggregationStrategy {
    /// Simple averaging of gradients
    FedAvg,
    /// Weighted by dataset size
    FedProx,
    /// Secure aggregation with encryption
    SecureAggregation,
    /// Byzantine-resilient aggregation
    ByzantineResilient,
    /// Differential privacy with noise addition
    DifferentiallyPrivate,
}

/// Privacy mechanism used
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PrivacyMechanism {
    /// No additional privacy (rely on aggregation)
    None,
    /// Local differential privacy (noise added locally)
    LocalDP {
        epsilon: String, // Stored as string for precision
        delta: String,
    },
    /// Central differential privacy (noise added during aggregation)
    CentralDP {
        epsilon: String,
        delta: String,
    },
    /// Secure multi-party computation
    SecureMPC,
    /// Homomorphic encryption
    HomomorphicEncryption,
}

/// A federated learning project/study
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct LearningProject {
    /// Unique project ID
    pub project_id: String,
    /// Project name
    pub name: String,
    /// Description of the learning objective
    pub description: String,
    /// Type of ML task
    pub task_type: TaskType,
    /// Required data schema (JSON schema as string)
    pub data_schema: String,
    /// Model architecture description (JSON)
    pub model_architecture: String,
    /// Initial model parameters hash (if pre-trained)
    pub initial_model_hash: Option<ActionHash>,
    /// Aggregation strategy
    pub aggregation_strategy: AggregationStrategy,
    /// Privacy mechanism
    pub privacy_mechanism: PrivacyMechanism,
    /// Minimum participants required
    pub min_participants: u32,
    /// Maximum participants allowed
    pub max_participants: Option<u32>,
    /// Minimum samples per participant
    pub min_samples_per_participant: u32,
    /// Number of training rounds planned
    pub planned_rounds: u32,
    /// Current status
    pub status: ProjectStatus,
    /// IRB approval hash (if required)
    pub irb_approval_hash: Option<ActionHash>,
    /// Project coordinator
    pub coordinator_hash: ActionHash,
    /// Created timestamp
    pub created_at: Timestamp,
    /// Updated timestamp
    pub updated_at: Timestamp,
}

/// A participant in a federated learning project
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ProjectParticipant {
    /// Project this participant belongs to
    pub project_hash: ActionHash,
    /// Participant identifier (organization/node)
    pub participant_hash: ActionHash,
    /// Public key for secure communication
    pub public_key: Vec<u8>,
    /// Number of samples available
    pub sample_count: u32,
    /// Data quality score (0-100)
    pub data_quality_score: Option<u32>,
    /// Whether participant is currently active
    pub is_active: bool,
    /// Rounds participated in
    pub rounds_participated: Vec<u32>,
    /// Joined timestamp
    pub joined_at: Timestamp,
    /// Last active timestamp
    pub last_active_at: Timestamp,
}

/// A training round in a federated learning project
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct TrainingRound {
    /// Round ID
    pub round_id: String,
    /// Project hash
    pub project_hash: ActionHash,
    /// Round number (1-indexed)
    pub round_number: u32,
    /// Status
    pub status: RoundStatus,
    /// Global model hash at start of round
    pub starting_model_hash: ActionHash,
    /// Aggregated model hash after round
    pub aggregated_model_hash: Option<ActionHash>,
    /// Expected participants
    pub expected_participants: Vec<ActionHash>,
    /// Participants who submitted updates
    pub submitted_participants: Vec<ActionHash>,
    /// Minimum submissions required
    pub min_submissions: u32,
    /// Learning rate for this round
    pub learning_rate: String,
    /// Batch size recommendation
    pub batch_size: u32,
    /// Local epochs per round
    pub local_epochs: u32,
    /// Deadline for submissions
    pub deadline: Timestamp,
    /// Started timestamp
    pub started_at: Timestamp,
    /// Completed timestamp
    pub completed_at: Option<Timestamp>,
}

/// A model update from a participant
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ModelUpdate {
    /// Update ID
    pub update_id: String,
    /// Round this update belongs to
    pub round_hash: ActionHash,
    /// Participant who submitted
    pub participant_hash: ActionHash,
    /// Encrypted gradient or model delta (for privacy)
    pub encrypted_update: Vec<u8>,
    /// Public key used for encryption
    pub encryption_key_hash: Option<ActionHash>,
    /// Number of samples used for training
    pub samples_used: u32,
    /// Local loss achieved
    pub local_loss: Option<String>,
    /// Local metrics (JSON)
    pub local_metrics: Option<String>,
    /// Noise added (for DP)
    pub noise_scale: Option<String>,
    /// Computation time (seconds)
    pub computation_time: u32,
    /// Submitted timestamp
    pub submitted_at: Timestamp,
}

/// Aggregated model from a training round
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AggregatedModel {
    /// Model ID
    pub model_id: String,
    /// Project hash
    pub project_hash: ActionHash,
    /// Round that produced this model
    pub round_hash: ActionHash,
    /// Model version (round number)
    pub version: u32,
    /// Serialized model parameters
    pub model_parameters: Vec<u8>,
    /// Parameter count
    pub parameter_count: u64,
    /// Number of participants aggregated
    pub participants_aggregated: u32,
    /// Total samples represented
    pub total_samples: u32,
    /// Aggregation method used
    pub aggregation_method: String,
    /// Global loss after aggregation
    pub global_loss: Option<String>,
    /// Global metrics (JSON)
    pub global_metrics: Option<String>,
    /// Hash of previous model
    pub previous_model_hash: Option<ActionHash>,
    /// Created timestamp
    pub created_at: Timestamp,
}

/// Model evaluation results
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ModelEvaluation {
    /// Evaluation ID
    pub evaluation_id: String,
    /// Model being evaluated
    pub model_hash: ActionHash,
    /// Evaluator (participant or coordinator)
    pub evaluator_hash: ActionHash,
    /// Test dataset description
    pub test_set_description: String,
    /// Test samples count
    pub test_samples: u32,
    /// Primary metric name
    pub primary_metric_name: String,
    /// Primary metric value
    pub primary_metric_value: String,
    /// All metrics (JSON)
    pub metrics: String,
    /// Confusion matrix (if classification, JSON)
    pub confusion_matrix: Option<String>,
    /// Evaluation timestamp
    pub evaluated_at: Timestamp,
}

/// Privacy budget tracking for differential privacy
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct PrivacyBudget {
    /// Budget ID
    pub budget_id: String,
    /// Project hash
    pub project_hash: ActionHash,
    /// Participant hash
    pub participant_hash: ActionHash,
    /// Total epsilon budget
    pub total_epsilon: String,
    /// Epsilon consumed so far
    pub consumed_epsilon: String,
    /// Delta parameter
    pub delta: String,
    /// Budget per round
    pub budget_per_round: String,
    /// Rounds budget has been spent on
    pub rounds_spent: Vec<u32>,
    /// Whether budget is exhausted
    pub is_exhausted: bool,
    /// Created timestamp
    pub created_at: Timestamp,
    /// Updated timestamp
    pub updated_at: Timestamp,
}

/// Entry types for the federated learning zome
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    LearningProject(LearningProject),
    ProjectParticipant(ProjectParticipant),
    TrainingRound(TrainingRound),
    ModelUpdate(ModelUpdate),
    AggregatedModel(AggregatedModel),
    ModelEvaluation(ModelEvaluation),
    PrivacyBudget(PrivacyBudget),
}

/// Link types for the federated learning zome
#[hdk_link_types]
pub enum LinkTypes {
    /// All projects index
    AllProjects,
    /// Projects by status
    ProjectsByStatus,
    /// Project to participants
    ProjectToParticipants,
    /// Project to rounds
    ProjectToRounds,
    /// Round to updates
    RoundToUpdates,
    /// Round to aggregated model
    RoundToModel,
    /// Project to models (version history)
    ProjectToModels,
    /// Model to evaluations
    ModelToEvaluations,
    /// Participant to their updates
    ParticipantToUpdates,
    /// Participant to their budget
    ParticipantToBudget,
    /// Active projects
    ActiveProjects,
}

/// Validation callbacks
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::LearningProject(project) => validate_project(&project),
                EntryTypes::TrainingRound(round) => validate_round(&round),
                EntryTypes::ModelUpdate(update) => validate_update(&update),
                EntryTypes::PrivacyBudget(budget) => validate_budget(&budget),
                _ => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_project(project: &LearningProject) -> ExternResult<ValidateCallbackResult> {
    if project.project_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Project ID is required".to_string()));
    }
    if project.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Project name is required".to_string()));
    }
    if project.min_participants == 0 {
        return Ok(ValidateCallbackResult::Invalid("Minimum participants must be at least 1".to_string()));
    }
    if project.planned_rounds == 0 {
        return Ok(ValidateCallbackResult::Invalid("Must plan at least 1 round".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_round(round: &TrainingRound) -> ExternResult<ValidateCallbackResult> {
    if round.round_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Round ID is required".to_string()));
    }
    if round.round_number == 0 {
        return Ok(ValidateCallbackResult::Invalid("Round number must be at least 1".to_string()));
    }
    if round.min_submissions == 0 {
        return Ok(ValidateCallbackResult::Invalid("Minimum submissions must be at least 1".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_update(update: &ModelUpdate) -> ExternResult<ValidateCallbackResult> {
    if update.update_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Update ID is required".to_string()));
    }
    if update.encrypted_update.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Update data is required".to_string()));
    }
    if update.samples_used == 0 {
        return Ok(ValidateCallbackResult::Invalid("Must use at least 1 sample".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_budget(budget: &PrivacyBudget) -> ExternResult<ValidateCallbackResult> {
    if budget.budget_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Budget ID is required".to_string()));
    }
    // Epsilon should be parseable as a positive number
    if budget.total_epsilon.parse::<f64>().map(|e| e <= 0.0).unwrap_or(true) {
        return Ok(ValidateCallbackResult::Invalid("Total epsilon must be positive".to_string()));
    }
    Ok(ValidateCallbackResult::Valid)
}
