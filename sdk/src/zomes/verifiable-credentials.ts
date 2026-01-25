/**
 * Verifiable Credentials Zome Client
 *
 * Client for W3C Verifiable Credentials for health data portability.
 * Part of Phase 6 - Global Scale.
 */

import type { AppClient, ActionHash, Timestamp } from '@holochain/client';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

// Enums
export enum CredentialType {
  ImmunizationRecord = 'ImmunizationRecord',
  LabResult = 'LabResult',
  Prescription = 'Prescription',
  MedicalLicense = 'MedicalLicense',
  InsuranceCoverage = 'InsuranceCoverage',
  HealthCertificate = 'HealthCertificate',
  PatientSummary = 'PatientSummary',
  Custom = 'Custom',
}

export enum CredentialStatus {
  Active = 'Active',
  Suspended = 'Suspended',
  Revoked = 'Revoked',
  Expired = 'Expired',
}

export enum ProofType {
  Ed25519Signature2020 = 'Ed25519Signature2020',
  JsonWebSignature2020 = 'JsonWebSignature2020',
  BbsBlsSignature2020 = 'BbsBlsSignature2020',
}

// Types
export interface VerifiableCredential {
  credential_hash: ActionHash;
  context: string[];
  type: string[];
  credential_type: CredentialType;
  issuer: string;
  issuer_hash: ActionHash;
  issuance_date: Timestamp;
  expiration_date?: Timestamp;
  credential_subject: Record<string, unknown>;
  proof_type: ProofType;
  proof_value: string;
  status: CredentialStatus;
}

export interface VerifiablePresentation {
  presentation_hash: ActionHash;
  context: string[];
  type: string[];
  holder: string;
  holder_hash: ActionHash;
  verifiable_credentials: ActionHash[];
  proof_type: ProofType;
  proof_value: string;
  created_at: Timestamp;
  expires_at?: Timestamp;
}

export interface CredentialSchema {
  schema_hash: ActionHash;
  name: string;
  version: string;
  credential_type: CredentialType;
  properties: Record<string, SchemaProperty>;
  required_properties: string[];
}

export interface SchemaProperty {
  type: string;
  description: string;
  format?: string;
}

export interface RevocationEntry {
  entry_hash: ActionHash;
  credential_hash: ActionHash;
  revoked_at: Timestamp;
  revoked_by: ActionHash;
  reason: string;
}

export interface TrustRelationship {
  relationship_hash: ActionHash;
  trustor_hash: ActionHash;
  trustee_hash: ActionHash;
  trust_type: 'issuer' | 'verifier' | 'both';
  credential_types: CredentialType[];
  established_at: Timestamp;
  expires_at?: Timestamp;
}

// Input types
export interface IssueCredentialInput {
  credential_type: CredentialType;
  subject_hash: ActionHash;
  claims: Record<string, unknown>;
  expiration_date?: Timestamp;
  proof_type?: ProofType;
}

export interface CreatePresentationInput {
  credential_hashes: ActionHash[];
  verifier_hash?: ActionHash;
  purpose: string;
  expires_in_seconds?: number;
}

export interface VerifyCredentialInput {
  credential_hash: ActionHash;
  check_revocation?: boolean;
  check_issuer_trust?: boolean;
}

export interface VerificationResult {
  valid: boolean;
  checks: {
    signature_valid: boolean;
    not_expired: boolean;
    not_revoked: boolean;
    issuer_trusted: boolean;
  };
  errors: string[];
}

/**
 * Verifiable Credentials Zome Client
 */
export class VerifiableCredentialsClient {
  private readonly roleName: string;
  private readonly zomeName = 'verifiable_credentials';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  /**
   * Issue a verifiable credential
   */
  async issueCredential(input: IssueCredentialInput): Promise<ActionHash> {
    return this.call<ActionHash>('issue_credential', input);
  }

  /**
   * Get a credential by hash
   */
  async getCredential(credentialHash: ActionHash): Promise<VerifiableCredential | null> {
    return this.call<VerifiableCredential | null>('get_credential', credentialHash);
  }

  /**
   * Get credentials issued to a subject
   */
  async getSubjectCredentials(subjectHash: ActionHash): Promise<VerifiableCredential[]> {
    return this.call<VerifiableCredential[]>('get_subject_credentials', subjectHash);
  }

  /**
   * Get credentials I've issued
   */
  async getIssuedCredentials(): Promise<VerifiableCredential[]> {
    return this.call<VerifiableCredential[]>('get_issued_credentials', null);
  }

  /**
   * Revoke a credential
   */
  async revokeCredential(credentialHash: ActionHash, reason: string): Promise<ActionHash> {
    return this.call<ActionHash>('revoke_credential', {
      credential_hash: credentialHash,
      reason,
    });
  }

  /**
   * Check if a credential is revoked
   */
  async isRevoked(credentialHash: ActionHash): Promise<boolean> {
    return this.call<boolean>('is_revoked', credentialHash);
  }

  /**
   * Create a verifiable presentation
   */
  async createPresentation(input: CreatePresentationInput): Promise<ActionHash> {
    return this.call<ActionHash>('create_presentation', input);
  }

  /**
   * Get a presentation by hash
   */
  async getPresentation(presentationHash: ActionHash): Promise<VerifiablePresentation | null> {
    return this.call<VerifiablePresentation | null>('get_presentation', presentationHash);
  }

  /**
   * Verify a credential
   */
  async verifyCredential(input: VerifyCredentialInput): Promise<VerificationResult> {
    return this.call<VerificationResult>('verify_credential', input);
  }

  /**
   * Verify a presentation
   */
  async verifyPresentation(presentationHash: ActionHash): Promise<VerificationResult> {
    return this.call<VerificationResult>('verify_presentation', presentationHash);
  }

  /**
   * Register a credential schema
   */
  async registerSchema(
    name: string,
    version: string,
    credentialType: CredentialType,
    properties: Record<string, SchemaProperty>,
    requiredProperties: string[]
  ): Promise<ActionHash> {
    return this.call<ActionHash>('register_schema', {
      name,
      version,
      credential_type: credentialType,
      properties,
      required_properties: requiredProperties,
    });
  }

  /**
   * Get schemas for a credential type
   */
  async getSchemas(credentialType: CredentialType): Promise<CredentialSchema[]> {
    return this.call<CredentialSchema[]>('get_schemas', credentialType);
  }

  /**
   * Establish trust relationship
   */
  async establishTrust(
    trusteeHash: ActionHash,
    trustType: 'issuer' | 'verifier' | 'both',
    credentialTypes: CredentialType[],
    expiresAt?: Timestamp
  ): Promise<ActionHash> {
    return this.call<ActionHash>('establish_trust', {
      trustee_hash: trusteeHash,
      trust_type: trustType,
      credential_types: credentialTypes,
      expires_at: expiresAt,
    });
  }

  /**
   * Get my trusted issuers
   */
  async getTrustedIssuers(): Promise<TrustRelationship[]> {
    return this.call<TrustRelationship[]>('get_trusted_issuers', null);
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
        `Verifiable Credentials zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
