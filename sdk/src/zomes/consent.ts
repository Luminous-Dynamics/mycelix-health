/**
 * Consent Zome Client
 *
 * Client for consent management in Mycelix-Health.
 * Handles patient consent grants, revocations, and authorization checks.
 */

import type { AppClient, ActionHash, AgentPubKey, Timestamp } from '@holochain/client';
import type { Consent, ConsentScope, AuthorizationCheck, AuthorizationResult } from '../types';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

/**
 * Input for granting consent
 */
export interface GrantConsentInput {
  grantee: AgentPubKey;
  scope: ConsentScope;
  data_categories: string[];
  purpose: string;
  valid_from?: Timestamp;
  valid_until?: Timestamp;
}

/**
 * Consent record with hash
 */
export interface ConsentRecord {
  hash: ActionHash;
  consent: Consent;
}

/**
 * Consent summary for UI display
 */
export interface ConsentSummary {
  activeCount: number;
  expiredCount: number;
  revokedCount: number;
  grantees: AgentPubKey[];
}

/**
 * Consent Zome Client
 */
export class ConsentClient {
  private readonly roleName: string;
  private readonly zomeName = 'consent';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  /**
   * Grant consent to an entity
   *
   * @param patientHash - Hash of the patient granting consent
   * @param input - Consent details
   * @returns Created consent record
   */
  async grantConsent(
    patientHash: ActionHash,
    input: GrantConsentInput
  ): Promise<ConsentRecord> {
    return this.call<ConsentRecord>('grant_consent', {
      patient_hash: patientHash,
      ...input,
      valid_from: input.valid_from ?? Date.now() * 1000, // microseconds
    });
  }

  /**
   * Revoke an existing consent
   *
   * @param consentHash - Hash of the consent to revoke
   * @returns Updated consent record
   */
  async revokeConsent(consentHash: ActionHash): Promise<ConsentRecord> {
    return this.call<ConsentRecord>('revoke_consent', consentHash);
  }

  /**
   * Get a consent record by hash
   *
   * @param consentHash - Hash of the consent
   * @returns Consent record or null if not found
   */
  async getConsent(consentHash: ActionHash): Promise<Consent | null> {
    return this.call<Consent | null>('get_consent', consentHash);
  }

  /**
   * List all consents granted by a patient
   *
   * @param patientHash - Hash of the patient
   * @returns Array of consent records
   */
  async listPatientConsents(patientHash: ActionHash): Promise<ConsentRecord[]> {
    return this.call<ConsentRecord[]>('list_patient_consents', patientHash);
  }

  /**
   * List all consents granted to a specific grantee
   *
   * @param grantee - Public key of the grantee
   * @returns Array of consent records
   */
  async listGranteeConsents(grantee: AgentPubKey): Promise<ConsentRecord[]> {
    return this.call<ConsentRecord[]>('list_grantee_consents', grantee);
  }

  /**
   * Check if an action is authorized
   *
   * This is the primary method for verifying consent before data access.
   *
   * @param check - Authorization check parameters
   * @returns Authorization result with consent details
   */
  async checkAuthorization(check: AuthorizationCheck): Promise<AuthorizationResult> {
    return this.call<AuthorizationResult>('check_authorization', check);
  }

  /**
   * Check if the current agent is authorized for an action
   *
   * Convenience method that uses the caller's public key.
   *
   * @param patientHash - Hash of the patient
   * @param action - The action to check
   * @param dataCategories - Data categories being accessed
   * @returns Authorization result
   */
  async amIAuthorized(
    patientHash: ActionHash,
    action: string,
    dataCategories: string[]
  ): Promise<AuthorizationResult> {
    return this.checkAuthorization({
      patient_hash: patientHash,
      requester: this.getMyPubKey(),
      action,
      data_categories: dataCategories,
    });
  }

  /**
   * Get active consents count for a patient
   *
   * @param patientHash - Hash of the patient
   * @returns Summary of consent status
   */
  async getConsentSummary(patientHash: ActionHash): Promise<ConsentSummary> {
    const consents = await this.listPatientConsents(patientHash);

    const now = Date.now() * 1000; // microseconds
    let activeCount = 0;
    let expiredCount = 0;
    let revokedCount = 0;
    const grantees: AgentPubKey[] = [];

    for (const record of consents) {
      const c = record.consent;

      if (!c.is_active) {
        revokedCount++;
      } else if (c.valid_until && c.valid_until < now) {
        expiredCount++;
      } else {
        activeCount++;
        if (!grantees.some(g => this.pubKeyEquals(g, c.grantee))) {
          grantees.push(c.grantee);
        }
      }
    }

    return {
      activeCount,
      expiredCount,
      revokedCount,
      grantees,
    };
  }

  /**
   * Update consent scope
   *
   * @param consentHash - Hash of the consent to update
   * @param newScope - New scope level
   * @returns Updated consent record
   */
  async updateConsentScope(
    consentHash: ActionHash,
    newScope: ConsentScope
  ): Promise<ConsentRecord> {
    return this.call<ConsentRecord>('update_consent_scope', {
      consent_hash: consentHash,
      new_scope: newScope,
    });
  }

  /**
   * Extend consent validity
   *
   * @param consentHash - Hash of the consent
   * @param newExpiry - New expiration timestamp
   * @returns Updated consent record
   */
  async extendConsent(
    consentHash: ActionHash,
    newExpiry: Timestamp
  ): Promise<ConsentRecord> {
    return this.call<ConsentRecord>('extend_consent', {
      consent_hash: consentHash,
      valid_until: newExpiry,
    });
  }

  /**
   * Get the current agent's public key
   */
  private getMyPubKey(): AgentPubKey {
    return this.client.myPubKey;
  }

  /**
   * Compare two public keys for equality
   */
  private pubKeyEquals(a: AgentPubKey, b: AgentPubKey): boolean {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (a[i] !== b[i]) return false;
    }
    return true;
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

      if (message.includes('unauthorized') || message.includes('not authorized')) {
        throw new HealthSdkError(
          HealthSdkErrorCode.UNAUTHORIZED,
          message,
          { fnName, payload }
        );
      }

      if (message.includes('expired')) {
        throw new HealthSdkError(
          HealthSdkErrorCode.CONSENT_EXPIRED,
          message,
          { fnName, payload }
        );
      }

      if (message.includes('revoked')) {
        throw new HealthSdkError(
          HealthSdkErrorCode.CONSENT_REVOKED,
          message,
          { fnName, payload }
        );
      }

      throw new HealthSdkError(
        HealthSdkErrorCode.ZOME_CALL_FAILED,
        `Consent zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
