/**
 * Zome Connector
 *
 * Connects EHR Gateway services to Holochain zomes for:
 * - Consent verification before data sync
 * - Provider credential verification
 * - Patient record validation
 * - Audit logging
 *
 * This module bridges the gap between the FHIR gateway and the actual
 * mycelix-health Holochain zomes.
 */

import type { AppClient, ActionHash, AgentPubKey } from '@holochain/client';

// ============================================================================
// Types
// ============================================================================

export interface ConsentCheckResult {
  authorized: boolean;
  consentHash?: ActionHash;
  dataCategories: string[];
  expiresAt?: number;
  emergencyOverride: boolean;
  denialReason?: string;
}

export interface ProviderVerificationResult {
  verified: boolean;
  providerId?: string;
  providerHash?: ActionHash;
  licenses: ProviderLicense[];
  trustScore?: number;
  denialReason?: string;
}

export interface ProviderLicense {
  licenseType: string;
  licenseNumber: string;
  state: string;
  status: 'Active' | 'Expired' | 'Suspended' | 'Revoked';
  expiresAt?: number;
}

export interface PatientLookupResult {
  found: boolean;
  patientHash?: ActionHash;
  mrn?: string;
  externalIds: Record<string, string>;
}

export interface AuditLogEntry {
  action: 'pull' | 'push' | 'consent_check' | 'provider_verify';
  patientHash?: ActionHash;
  providerHash?: ActionHash;
  externalSystem: string;
  resourceType: string;
  resourceCount: number;
  success: boolean;
  errorMessage?: string;
  timestamp: number;
  metadata?: Record<string, unknown>;
}

export interface DataCategory {
  category: string;
  permission: 'Read' | 'Write' | 'Delete';
}

// ============================================================================
// Zome Connector Class
// ============================================================================

export class ZomeConnector {
  constructor(
    private readonly client: AppClient,
    private readonly roleName: string = 'health'
  ) {}

  // --------------------------------------------------------------------------
  // Consent Verification
  // --------------------------------------------------------------------------

  /**
   * Check if the current agent has consent to access patient data
   *
   * This MUST be called before any pull/push operation to ensure
   * HIPAA-compliant access control.
   */
  async checkConsent(
    patientHash: ActionHash,
    dataCategories: DataCategory[],
    isEmergency: boolean = false,
    emergencyReason?: string
  ): Promise<ConsentCheckResult> {
    try {
      const result = await this.client.callZome({
        cap_secret: null,
        role_name: this.roleName,
        zome_name: 'consent',
        fn_name: 'check_authorization',
        payload: {
          patient_hash: patientHash,
          data_categories: dataCategories.map(dc => ({
            category: dc.category,
            permission: dc.permission,
          })),
          is_emergency: isEmergency,
          emergency_reason: emergencyReason,
        },
      });

      return result as ConsentCheckResult;
    } catch (error) {
      return {
        authorized: false,
        dataCategories: dataCategories.map(dc => dc.category),
        emergencyOverride: false,
        denialReason: `Consent check failed: ${(error as Error).message}`,
      };
    }
  }

  /**
   * Check consent for pulling data from external EHR
   */
  async checkPullConsent(
    patientHash: ActionHash,
    resourceTypes: string[]
  ): Promise<ConsentCheckResult> {
    const categories = this.mapResourceTypesToCategories(resourceTypes, 'Write');
    return this.checkConsent(patientHash, categories);
  }

  /**
   * Check consent for pushing data to external EHR
   */
  async checkPushConsent(
    patientHash: ActionHash,
    resourceTypes: string[]
  ): Promise<ConsentCheckResult> {
    const categories = this.mapResourceTypesToCategories(resourceTypes, 'Read');
    return this.checkConsent(patientHash, categories);
  }

  /**
   * Map FHIR resource types to consent data categories
   */
  private mapResourceTypesToCategories(
    resourceTypes: string[],
    permission: 'Read' | 'Write'
  ): DataCategory[] {
    const categoryMap: Record<string, string> = {
      Patient: 'Demographics',
      Observation: 'LabResults',
      Condition: 'Diagnoses',
      MedicationRequest: 'Medications',
      MedicationStatement: 'Medications',
      AllergyIntolerance: 'Allergies',
      Immunization: 'Immunizations',
      Procedure: 'Procedures',
      Encounter: 'Encounters',
      DiagnosticReport: 'LabResults',
      DocumentReference: 'Documents',
    };

    return resourceTypes.map(rt => ({
      category: categoryMap[rt] || 'Other',
      permission,
    }));
  }

  // --------------------------------------------------------------------------
  // Provider Verification
  // --------------------------------------------------------------------------

  /**
   * Verify provider credentials before allowing EHR sync
   *
   * Ensures only authorized healthcare providers can access patient data.
   */
  async verifyProvider(
    providerNpi?: string,
    providerAgentKey?: AgentPubKey
  ): Promise<ProviderVerificationResult> {
    try {
      // Try to find provider by NPI or agent key
      let providerRecord;

      if (providerNpi) {
        providerRecord = await this.client.callZome({
          cap_secret: null,
          role_name: this.roleName,
          zome_name: 'provider',
          fn_name: 'get_provider_by_npi',
          payload: providerNpi,
        });
      } else if (providerAgentKey) {
        providerRecord = await this.client.callZome({
          cap_secret: null,
          role_name: this.roleName,
          zome_name: 'provider',
          fn_name: 'get_provider_by_agent',
          payload: providerAgentKey,
        });
      } else {
        // Get current agent's provider record
        providerRecord = await this.client.callZome({
          cap_secret: null,
          role_name: this.roleName,
          zome_name: 'provider',
          fn_name: 'get_my_provider_profile',
          payload: null,
        });
      }

      if (!providerRecord) {
        return {
          verified: false,
          licenses: [],
          denialReason: 'Provider not found in system',
        };
      }

      // Verify licenses are active
      const licenses = await this.client.callZome({
        cap_secret: null,
        role_name: this.roleName,
        zome_name: 'provider',
        fn_name: 'get_provider_licenses',
        payload: (providerRecord as any).action_hash,
      }) as ProviderLicense[];

      const activeLicenses = licenses.filter(l => l.status === 'Active');

      if (activeLicenses.length === 0) {
        return {
          verified: false,
          providerId: (providerRecord as any).npi,
          providerHash: (providerRecord as any).action_hash,
          licenses,
          denialReason: 'No active licenses found',
        };
      }

      // Get trust score from MATL
      let trustScore: number | undefined;
      try {
        const matlResult = await this.client.callZome({
          cap_secret: null,
          role_name: this.roleName,
          zome_name: 'bridge',
          fn_name: 'get_provider_trust_score',
          payload: (providerRecord as any).action_hash,
        });
        trustScore = (matlResult as any)?.composite_score;
      } catch {
        // Trust score is optional
      }

      return {
        verified: true,
        providerId: (providerRecord as any).npi,
        providerHash: (providerRecord as any).action_hash,
        licenses: activeLicenses,
        trustScore,
      };
    } catch (error) {
      return {
        verified: false,
        licenses: [],
        denialReason: `Provider verification failed: ${(error as Error).message}`,
      };
    }
  }

  // --------------------------------------------------------------------------
  // Patient Lookup
  // --------------------------------------------------------------------------

  /**
   * Look up patient by MRN or external ID
   */
  async lookupPatient(
    mrn?: string,
    externalSystem?: string,
    externalId?: string
  ): Promise<PatientLookupResult> {
    try {
      let result;

      if (mrn) {
        result = await this.client.callZome({
          cap_secret: null,
          role_name: this.roleName,
          zome_name: 'patient',
          fn_name: 'get_patient_by_mrn',
          payload: {
            mrn,
            is_emergency: false,
          },
        });
      } else if (externalSystem && externalId) {
        result = await this.client.callZome({
          cap_secret: null,
          role_name: this.roleName,
          zome_name: 'patient',
          fn_name: 'get_patient_by_external_id',
          payload: {
            system: externalSystem,
            id: externalId,
          },
        });
      }

      if (!result) {
        return {
          found: false,
          externalIds: {},
        };
      }

      return {
        found: true,
        patientHash: (result as any).action_hash,
        mrn: (result as any).entry?.mrn,
        externalIds: (result as any).entry?.external_ids || {},
      };
    } catch (error) {
      return {
        found: false,
        externalIds: {},
      };
    }
  }

  /**
   * Register external ID for a patient
   */
  async registerExternalId(
    patientHash: ActionHash,
    externalSystem: string,
    externalId: string
  ): Promise<void> {
    await this.client.callZome({
      cap_secret: null,
      role_name: this.roleName,
      zome_name: 'patient',
      fn_name: 'add_external_id',
      payload: {
        patient_hash: patientHash,
        system: externalSystem,
        id: externalId,
      },
    });
  }

  // --------------------------------------------------------------------------
  // Audit Logging
  // --------------------------------------------------------------------------

  /**
   * Log an EHR sync operation to the audit trail
   */
  async logSyncOperation(entry: AuditLogEntry): Promise<void> {
    try {
      await this.client.callZome({
        cap_secret: null,
        role_name: this.roleName,
        zome_name: 'consent',
        fn_name: 'create_access_log',
        payload: {
          patient_hash: entry.patientHash,
          action: entry.action,
          data_categories: [entry.resourceType],
          external_system: entry.externalSystem,
          resource_count: entry.resourceCount,
          success: entry.success,
          error_message: entry.errorMessage,
          metadata: entry.metadata,
        },
      });
    } catch (error) {
      // Log to console if zome logging fails
      console.error('Failed to log sync operation:', error);
      console.log('Audit entry:', JSON.stringify(entry));
    }
  }

  /**
   * Log a consent check operation
   */
  async logConsentCheck(
    patientHash: ActionHash,
    authorized: boolean,
    categories: string[],
    externalSystem: string
  ): Promise<void> {
    await this.logSyncOperation({
      action: 'consent_check',
      patientHash,
      externalSystem,
      resourceType: categories.join(','),
      resourceCount: categories.length,
      success: authorized,
      errorMessage: authorized ? undefined : 'Consent denied',
      timestamp: Date.now(),
    });
  }

  // --------------------------------------------------------------------------
  // Data Validation
  // --------------------------------------------------------------------------

  /**
   * Validate FHIR data before ingestion
   */
  async validateForIngestion(
    patientHash: ActionHash,
    resourceType: string,
    data: unknown
  ): Promise<{ valid: boolean; errors: string[] }> {
    try {
      const result = await this.client.callZome({
        cap_secret: null,
        role_name: this.roleName,
        zome_name: 'fhir_bridge',
        fn_name: 'validate_fhir_resource',
        payload: {
          patient_hash: patientHash,
          resource_type: resourceType,
          data,
        },
      });

      return result as { valid: boolean; errors: string[] };
    } catch (error) {
      return {
        valid: false,
        errors: [`Validation failed: ${(error as Error).message}`],
      };
    }
  }

  // --------------------------------------------------------------------------
  // Combined Operations
  // --------------------------------------------------------------------------

  /**
   * Prepare for pull operation with all necessary checks
   *
   * Returns authorization to proceed with the pull.
   */
  async prepareForPull(
    patientMrn: string,
    externalSystem: string,
    resourceTypes: string[]
  ): Promise<{
    authorized: boolean;
    patientHash?: ActionHash;
    consentResult?: ConsentCheckResult;
    providerResult?: ProviderVerificationResult;
    errors: string[];
  }> {
    const errors: string[] = [];

    // 1. Verify provider
    const providerResult = await this.verifyProvider();
    if (!providerResult.verified) {
      errors.push(providerResult.denialReason || 'Provider verification failed');
      return { authorized: false, providerResult, errors };
    }

    // 2. Look up patient
    const patientResult = await this.lookupPatient(patientMrn);
    if (!patientResult.found || !patientResult.patientHash) {
      errors.push('Patient not found');
      return { authorized: false, providerResult, errors };
    }

    // 3. Check consent
    const consentResult = await this.checkPullConsent(
      patientResult.patientHash,
      resourceTypes
    );
    if (!consentResult.authorized) {
      errors.push(consentResult.denialReason || 'Consent not granted');
      await this.logConsentCheck(
        patientResult.patientHash,
        false,
        consentResult.dataCategories,
        externalSystem
      );
      return {
        authorized: false,
        patientHash: patientResult.patientHash,
        consentResult,
        providerResult,
        errors,
      };
    }

    // Log successful consent check
    await this.logConsentCheck(
      patientResult.patientHash,
      true,
      consentResult.dataCategories,
      externalSystem
    );

    return {
      authorized: true,
      patientHash: patientResult.patientHash,
      consentResult,
      providerResult,
      errors: [],
    };
  }

  /**
   * Prepare for push operation with all necessary checks
   */
  async prepareForPush(
    patientHash: ActionHash,
    externalSystem: string,
    resourceTypes: string[]
  ): Promise<{
    authorized: boolean;
    consentResult?: ConsentCheckResult;
    providerResult?: ProviderVerificationResult;
    errors: string[];
  }> {
    const errors: string[] = [];

    // 1. Verify provider
    const providerResult = await this.verifyProvider();
    if (!providerResult.verified) {
      errors.push(providerResult.denialReason || 'Provider verification failed');
      return { authorized: false, providerResult, errors };
    }

    // 2. Check consent for reading data to push
    const consentResult = await this.checkPushConsent(patientHash, resourceTypes);
    if (!consentResult.authorized) {
      errors.push(consentResult.denialReason || 'Consent not granted for data sharing');
      await this.logConsentCheck(
        patientHash,
        false,
        consentResult.dataCategories,
        externalSystem
      );
      return {
        authorized: false,
        consentResult,
        providerResult,
        errors,
      };
    }

    // Log successful consent check
    await this.logConsentCheck(
      patientHash,
      true,
      consentResult.dataCategories,
      externalSystem
    );

    return {
      authorized: true,
      consentResult,
      providerResult,
      errors: [],
    };
  }
}

// ============================================================================
// Factory Function
// ============================================================================

/**
 * Create a ZomeConnector instance
 */
export function createZomeConnector(
  client: AppClient,
  roleName: string = 'health'
): ZomeConnector {
  return new ZomeConnector(client, roleName);
}

// ============================================================================
// Exports
// ============================================================================

export default {
  ZomeConnector,
  createZomeConnector,
};
