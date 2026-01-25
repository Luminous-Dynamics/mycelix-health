/**
 * International Patient Summary (IPS) Zome Client
 *
 * Client for creating and managing IPS documents for cross-border health data exchange.
 * Part of Phase 6 - Global Scale.
 */

import type { AppClient, ActionHash, Timestamp } from '@holochain/client';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

// Enums
export enum IpsStatus {
  Draft = 'Draft',
  Finalized = 'Finalized',
  Superseded = 'Superseded',
  EnteredInError = 'EnteredInError',
}

// Types
export interface IpsMedication {
  medication_code: string;
  medication_name: string;
  dosage: string;
  route: string;
  frequency: string;
  start_date?: Timestamp;
  end_date?: Timestamp;
}

export interface IpsAllergy {
  allergen_code: string;
  allergen_name: string;
  reaction: string;
  severity: string;
  onset_date?: Timestamp;
}

export interface IpsCondition {
  condition_code: string;
  condition_name: string;
  clinical_status: string;
  onset_date?: Timestamp;
  resolution_date?: Timestamp;
}

export interface IpsProcedure {
  procedure_code: string;
  procedure_name: string;
  performed_date: Timestamp;
  body_site?: string;
  outcome?: string;
}

export interface IpsImmunization {
  vaccine_code: string;
  vaccine_name: string;
  administered_date: Timestamp;
  lot_number?: string;
  site?: string;
}

export interface InternationalPatientSummary {
  ips_hash: ActionHash;
  patient_hash: ActionHash;
  medications: IpsMedication[];
  allergies: IpsAllergy[];
  conditions: IpsCondition[];
  procedures: IpsProcedure[];
  immunizations: IpsImmunization[];
  status: IpsStatus;
  created_at: Timestamp;
  finalized_at?: Timestamp;
  author_hash: ActionHash;
  language: string;
  jurisdiction: string;
}

export interface IpsExport {
  format: 'fhir_json' | 'fhir_xml' | 'cda';
  content: string;
  generated_at: Timestamp;
}

// Input types
export interface CreateIpsInput {
  patient_hash: ActionHash;
  medications: IpsMedication[];
  allergies: IpsAllergy[];
  conditions: IpsCondition[];
  procedures: IpsProcedure[];
  immunizations: IpsImmunization[];
  language: string;
  jurisdiction: string;
}

export interface UpdateIpsInput {
  ips_hash: ActionHash;
  medications?: IpsMedication[];
  allergies?: IpsAllergy[];
  conditions?: IpsCondition[];
  procedures?: IpsProcedure[];
  immunizations?: IpsImmunization[];
}

/**
 * IPS Zome Client
 */
export class IpsClient {
  private readonly roleName: string;
  private readonly zomeName = 'ips';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  /**
   * Create a new IPS document
   */
  async createIps(input: CreateIpsInput): Promise<ActionHash> {
    return this.call<ActionHash>('create_ips', input);
  }

  /**
   * Get an IPS document by hash
   */
  async getIps(ipsHash: ActionHash): Promise<InternationalPatientSummary | null> {
    return this.call<InternationalPatientSummary | null>('get_ips', ipsHash);
  }

  /**
   * Get the current IPS for a patient
   */
  async getPatientIps(patientHash: ActionHash): Promise<InternationalPatientSummary | null> {
    return this.call<InternationalPatientSummary | null>('get_patient_ips', patientHash);
  }

  /**
   * Update an IPS document (creates a new version)
   */
  async updateIps(input: UpdateIpsInput): Promise<ActionHash> {
    return this.call<ActionHash>('update_ips', input);
  }

  /**
   * Finalize an IPS document
   */
  async finalizeIps(ipsHash: ActionHash): Promise<ActionHash> {
    return this.call<ActionHash>('finalize_ips', ipsHash);
  }

  /**
   * Export IPS to standard format
   */
  async exportIps(ipsHash: ActionHash, format: 'fhir_json' | 'fhir_xml' | 'cda'): Promise<IpsExport> {
    return this.call<IpsExport>('export_ips', { ips_hash: ipsHash, format });
  }

  /**
   * Import IPS from standard format
   */
  async importIps(content: string, format: 'fhir_json' | 'fhir_xml' | 'cda'): Promise<ActionHash> {
    return this.call<ActionHash>('import_ips', { content, format });
  }

  /**
   * Validate IPS completeness
   */
  async validateIps(ipsHash: ActionHash): Promise<{ valid: boolean; issues: string[] }> {
    return this.call<{ valid: boolean; issues: string[] }>('validate_ips', ipsHash);
  }

  /**
   * Get IPS history for a patient
   */
  async getIpsHistory(patientHash: ActionHash): Promise<InternationalPatientSummary[]> {
    return this.call<InternationalPatientSummary[]>('get_ips_history', patientHash);
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
        `IPS zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
