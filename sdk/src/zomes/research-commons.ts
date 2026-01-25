/**
 * Research Commons Zome Client
 *
 * Client for managing open research data sharing and collaborative research in Mycelix-Health.
 * Part of Phase 5 - Advanced Research.
 */

import type { AppClient, ActionHash, Timestamp } from '@holochain/client';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

// Enums
export enum DatasetAccessLevel {
  Public = 'Public',
  Registered = 'Registered',
  Controlled = 'Controlled',
  Private = 'Private',
}

export enum LicenseType {
  CC0 = 'CC0',
  CCBY = 'CCBY',
  CCBYSA = 'CCBYSA',
  CCBYNC = 'CCBYNC',
  Custom = 'Custom',
}

export enum DataQualityScore {
  Excellent = 'Excellent',
  Good = 'Good',
  Fair = 'Fair',
  Poor = 'Poor',
  Unknown = 'Unknown',
}

export enum AccessRequestStatus {
  Pending = 'Pending',
  Approved = 'Approved',
  Denied = 'Denied',
  Expired = 'Expired',
}

// Types
export interface DatasetMetadata {
  title: string;
  description: string;
  keywords: string[];
  version: string;
  doi?: string;
  citation?: string;
}

export interface ResearchDataset {
  dataset_hash: ActionHash;
  metadata: DatasetMetadata;
  access_level: DatasetAccessLevel;
  license: LicenseType;
  quality_score: DataQualityScore;
  contributor_hash: ActionHash;
  record_count: number;
  created_at: Timestamp;
  updated_at?: Timestamp;
}

export interface DataUseAgreement {
  agreement_hash: ActionHash;
  dataset_hash: ActionHash;
  researcher_hash: ActionHash;
  purpose: string;
  terms: string;
  expiration_date: Timestamp;
  approved_by?: ActionHash;
  approved_at?: Timestamp;
  status: AccessRequestStatus;
}

export interface ContributionCredit {
  credit_hash: ActionHash;
  dataset_hash: ActionHash;
  contributor_hash: ActionHash;
  contribution_type: string;
  credit_percentage: number;
  acknowledged: boolean;
}

// Input types
export interface CreateDatasetInput {
  metadata: DatasetMetadata;
  access_level: DatasetAccessLevel;
  license: LicenseType;
  record_count: number;
}

export interface RequestAccessInput {
  dataset_hash: ActionHash;
  purpose: string;
  proposed_terms: string;
  expiration_date: Timestamp;
}

export interface ApproveAccessInput {
  agreement_hash: ActionHash;
  final_terms?: string;
}

/**
 * Research Commons Zome Client
 */
export class ResearchCommonsClient {
  private readonly roleName: string;
  private readonly zomeName = 'research_commons';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  /**
   * Create a new research dataset
   */
  async createDataset(input: CreateDatasetInput): Promise<ActionHash> {
    return this.call<ActionHash>('create_dataset', input);
  }

  /**
   * Get a dataset by hash
   */
  async getDataset(datasetHash: ActionHash): Promise<ResearchDataset | null> {
    return this.call<ResearchDataset | null>('get_dataset', datasetHash);
  }

  /**
   * Search datasets by keyword
   */
  async searchDatasets(keyword: string): Promise<ResearchDataset[]> {
    return this.call<ResearchDataset[]>('search_datasets', keyword);
  }

  /**
   * List public datasets
   */
  async listPublicDatasets(): Promise<ResearchDataset[]> {
    return this.call<ResearchDataset[]>('list_public_datasets', null);
  }

  /**
   * Request access to a controlled dataset
   */
  async requestAccess(input: RequestAccessInput): Promise<ActionHash> {
    return this.call<ActionHash>('request_access', input);
  }

  /**
   * Approve a data use agreement
   */
  async approveAccess(input: ApproveAccessInput): Promise<ActionHash> {
    return this.call<ActionHash>('approve_access', input);
  }

  /**
   * Deny an access request
   */
  async denyAccess(agreementHash: ActionHash, reason: string): Promise<ActionHash> {
    return this.call<ActionHash>('deny_access', { agreement_hash: agreementHash, reason });
  }

  /**
   * Get pending access requests for datasets I own
   */
  async getPendingRequests(): Promise<DataUseAgreement[]> {
    return this.call<DataUseAgreement[]>('get_pending_requests', null);
  }

  /**
   * Add contribution credit
   */
  async addContributionCredit(
    datasetHash: ActionHash,
    contributorHash: ActionHash,
    contributionType: string,
    creditPercentage: number
  ): Promise<ActionHash> {
    return this.call<ActionHash>('add_contribution_credit', {
      dataset_hash: datasetHash,
      contributor_hash: contributorHash,
      contribution_type: contributionType,
      credit_percentage: creditPercentage,
    });
  }

  /**
   * Get credits for a dataset
   */
  async getDatasetCredits(datasetHash: ActionHash): Promise<ContributionCredit[]> {
    return this.call<ContributionCredit[]>('get_dataset_credits', datasetHash);
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
        `Research Commons zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
