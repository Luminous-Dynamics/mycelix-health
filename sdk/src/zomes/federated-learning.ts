/**
 * Federated Learning Zome Client
 *
 * Client for privacy-preserving distributed machine learning on health data.
 * Part of Phase 5 - Advanced Research.
 */

import type { AppClient, ActionHash, Timestamp } from '@holochain/client';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

// Enums
export enum ProjectStatus {
  Planning = 'Planning',
  Recruiting = 'Recruiting',
  Training = 'Training',
  Evaluating = 'Evaluating',
  Completed = 'Completed',
  Paused = 'Paused',
  Cancelled = 'Cancelled',
}

export enum RoundStatus {
  Pending = 'Pending',
  Active = 'Active',
  Aggregating = 'Aggregating',
  Completed = 'Completed',
  Failed = 'Failed',
}

export enum AggregationMethod {
  FederatedAveraging = 'FederatedAveraging',
  SecureAggregation = 'SecureAggregation',
  DifferentiallyPrivate = 'DifferentiallyPrivate',
}

// Types
export interface ModelArchitecture {
  name: string;
  version: string;
  input_shape: number[];
  output_shape: number[];
  parameters_count: number;
}

export interface FederatedProject {
  project_hash: ActionHash;
  name: string;
  description: string;
  model_architecture: ModelArchitecture;
  privacy_budget: number;
  min_participants: number;
  status: ProjectStatus;
  coordinator_hash: ActionHash;
  created_at: Timestamp;
}

export interface TrainingRound {
  round_hash: ActionHash;
  project_hash: ActionHash;
  round_number: number;
  status: RoundStatus;
  starting_model_hash: ActionHash;
  aggregated_model_hash?: ActionHash;
  participants: ActionHash[];
  started_at: Timestamp;
  completed_at?: Timestamp;
}

export interface ModelUpdate {
  update_hash: ActionHash;
  round_hash: ActionHash;
  participant_hash: ActionHash;
  gradient_hash: string;
  samples_count: number;
  local_loss: number;
  submitted_at: Timestamp;
}

export interface AggregatedModel {
  model_hash: ActionHash;
  round_hash: ActionHash;
  parameter_count: number;
  participants_aggregated: number;
  total_samples: number;
  global_loss?: number;
  created_at: Timestamp;
}

// Input types
export interface CreateProjectInput {
  name: string;
  description: string;
  model_architecture: ModelArchitecture;
  privacy_budget: number;
  min_participants: number;
}

export interface SubmitUpdateInput {
  round_hash: ActionHash;
  gradient_hash: string;
  samples_count: number;
  local_loss: number;
}

export interface AggregateUpdatesInput {
  round_hash: ActionHash;
  aggregation_method: AggregationMethod;
  global_loss?: number;
  global_metrics?: Record<string, number>;
}

/**
 * Federated Learning Zome Client
 */
export class FederatedLearningClient {
  private readonly roleName: string;
  private readonly zomeName = 'federated_learning';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  /**
   * Create a new federated learning project
   */
  async createProject(input: CreateProjectInput): Promise<ActionHash> {
    return this.call<ActionHash>('create_project', input);
  }

  /**
   * Get a project by hash
   */
  async getProject(projectHash: ActionHash): Promise<FederatedProject | null> {
    return this.call<FederatedProject | null>('get_project', projectHash);
  }

  /**
   * List active projects
   */
  async listActiveProjects(): Promise<FederatedProject[]> {
    return this.call<FederatedProject[]>('list_active_projects', null);
  }

  /**
   * Join a project as a participant
   */
  async joinProject(projectHash: ActionHash): Promise<ActionHash> {
    return this.call<ActionHash>('join_project', projectHash);
  }

  /**
   * Leave a project
   */
  async leaveProject(projectHash: ActionHash): Promise<void> {
    return this.call<void>('leave_project', projectHash);
  }

  /**
   * Start a new training round
   */
  async startRound(projectHash: ActionHash): Promise<ActionHash> {
    return this.call<ActionHash>('start_round', projectHash);
  }

  /**
   * Get current round for a project
   */
  async getCurrentRound(projectHash: ActionHash): Promise<TrainingRound | null> {
    return this.call<TrainingRound | null>('get_current_round', projectHash);
  }

  /**
   * Submit a model update for the current round
   */
  async submitUpdate(input: SubmitUpdateInput): Promise<ActionHash> {
    return this.call<ActionHash>('submit_update', input);
  }

  /**
   * Get updates for a round
   */
  async getRoundUpdates(roundHash: ActionHash): Promise<ModelUpdate[]> {
    return this.call<ModelUpdate[]>('get_round_updates', roundHash);
  }

  /**
   * Aggregate updates for a round (coordinator only)
   */
  async aggregateUpdates(input: AggregateUpdatesInput): Promise<ActionHash> {
    return this.call<ActionHash>('aggregate_updates', input);
  }

  /**
   * Get aggregated model
   */
  async getAggregatedModel(modelHash: ActionHash): Promise<AggregatedModel | null> {
    return this.call<AggregatedModel | null>('get_aggregated_model', modelHash);
  }

  /**
   * Get project models
   */
  async getProjectModels(projectHash: ActionHash): Promise<AggregatedModel[]> {
    return this.call<AggregatedModel[]>('get_project_models', projectHash);
  }

  /**
   * Get my participation in a project
   */
  async getMyParticipation(projectHash: ActionHash): Promise<ModelUpdate[]> {
    return this.call<ModelUpdate[]>('get_my_participation', projectHash);
  }

  private async call<T>(fnName: string, payload: unknown): Promise<T> {
    try {
      const result = await this.client.callZome({
        role_name: this.roleName,
        zome_name: this.zomeName,
        fn_name: fnName,
        payload,
      });
      return result as T;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      throw new HealthSdkError(
        HealthSdkErrorCode.ZOME_CALL_FAILED,
        `Federated Learning zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
