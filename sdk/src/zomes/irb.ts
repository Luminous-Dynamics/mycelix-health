/**
 * IRB Zome Client
 *
 * Client for Institutional Review Board protocol management and ethical review.
 * Part of Phase 5 - Advanced Research.
 */

import type { AppClient, ActionHash, Timestamp } from '@holochain/client';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

// Enums
export enum ProtocolStatus {
  Draft = 'Draft',
  Submitted = 'Submitted',
  UnderReview = 'UnderReview',
  RevisionRequested = 'RevisionRequested',
  Approved = 'Approved',
  Rejected = 'Rejected',
  Expired = 'Expired',
  Withdrawn = 'Withdrawn',
}

export enum ReviewType {
  Exempt = 'Exempt',
  Expedited = 'Expedited',
  FullBoard = 'FullBoard',
  Continuing = 'Continuing',
  Amendment = 'Amendment',
}

export enum ReviewerRole {
  Chair = 'Chair',
  ViceChair = 'ViceChair',
  ScientificReviewer = 'ScientificReviewer',
  EthicsReviewer = 'EthicsReviewer',
  CommunityMember = 'CommunityMember',
  Regulatory = 'Regulatory',
}

export enum VoteType {
  Approve = 'Approve',
  ConditionalApprove = 'ConditionalApprove',
  Table = 'Table',
  Disapprove = 'Disapprove',
  Abstain = 'Abstain',
}

// Types
export interface ProtocolSubmission {
  protocol_hash: ActionHash;
  protocol_id: string;
  title: string;
  pi_hash: ActionHash;
  institution: string;
  review_type: ReviewType;
  status: ProtocolStatus;
  version: number;
  submitted_at?: Timestamp;
  approved_at?: Timestamp;
  expiration_date?: Timestamp;
}

export interface IrbMember {
  member_hash: ActionHash;
  name: string;
  role: ReviewerRole;
  credentials: string[];
  expertise: string[];
  institution: string;
  is_active: boolean;
}

export interface ProtocolReview {
  review_hash: ActionHash;
  protocol_hash: ActionHash;
  reviewer_hash: ActionHash;
  comments: string;
  vote: VoteType;
  conditions?: string[];
  created_at: Timestamp;
}

export interface IrbMeeting {
  meeting_hash: ActionHash;
  meeting_date: Timestamp;
  protocols_reviewed: ActionHash[];
  attendees: ActionHash[];
  minutes?: string;
}

// Input types
export interface CreateProtocolInput {
  title: string;
  institution: string;
  review_type: ReviewType;
  description: string;
  methodology: string;
  risk_assessment: string;
  consent_process: string;
  data_management: string;
}

export interface SubmitReviewInput {
  protocol_hash: ActionHash;
  comments: string;
  vote: VoteType;
  conditions?: string[];
}

/**
 * IRB Zome Client
 */
export class IrbClient {
  private readonly roleName: string;
  private readonly zomeName = 'irb';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  /**
   * Create a new protocol submission
   */
  async createProtocol(input: CreateProtocolInput): Promise<ActionHash> {
    return this.call<ActionHash>('create_protocol_submission', input);
  }

  /**
   * Get a protocol by hash
   */
  async getProtocol(protocolHash: ActionHash): Promise<ProtocolSubmission | null> {
    return this.call<ProtocolSubmission | null>('get_protocol', protocolHash);
  }

  /**
   * Submit a protocol for review
   */
  async submitForReview(protocolHash: ActionHash): Promise<ActionHash> {
    return this.call<ActionHash>('submit_for_review', protocolHash);
  }

  /**
   * Get protocols pending review
   */
  async getPendingProtocols(): Promise<ProtocolSubmission[]> {
    return this.call<ProtocolSubmission[]>('get_pending_protocols', null);
  }

  /**
   * Submit a review for a protocol
   */
  async submitReview(input: SubmitReviewInput): Promise<ActionHash> {
    return this.call<ActionHash>('submit_review', input);
  }

  /**
   * Get reviews for a protocol
   */
  async getProtocolReviews(protocolHash: ActionHash): Promise<ProtocolReview[]> {
    return this.call<ProtocolReview[]>('get_protocol_reviews', protocolHash);
  }

  /**
   * Approve a protocol
   */
  async approveProtocol(protocolHash: ActionHash, expirationDate: Timestamp): Promise<ActionHash> {
    return this.call<ActionHash>('approve_protocol', {
      protocol_hash: protocolHash,
      expiration_date: expirationDate,
    });
  }

  /**
   * Reject a protocol
   */
  async rejectProtocol(protocolHash: ActionHash, reason: string): Promise<ActionHash> {
    return this.call<ActionHash>('reject_protocol', {
      protocol_hash: protocolHash,
      reason,
    });
  }

  /**
   * Request revisions
   */
  async requestRevisions(protocolHash: ActionHash, comments: string): Promise<ActionHash> {
    return this.call<ActionHash>('request_revisions', {
      protocol_hash: protocolHash,
      comments,
    });
  }

  /**
   * Get my protocols (as PI)
   */
  async getMyProtocols(): Promise<ProtocolSubmission[]> {
    return this.call<ProtocolSubmission[]>('get_my_protocols', null);
  }

  /**
   * Get IRB members
   */
  async getMembers(): Promise<IrbMember[]> {
    return this.call<IrbMember[]>('get_irb_members', null);
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
        `IRB zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
