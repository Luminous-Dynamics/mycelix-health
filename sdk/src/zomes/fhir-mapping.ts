/**
 * FHIR Mapping Zome Client
 *
 * Client for FHIR R4 resource mapping and transformation in Mycelix-Health.
 * Handles bidirectional conversion between internal Holochain types and FHIR.
 */

import type { AppClient, ActionHash, Timestamp } from '@holochain/client';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

// FHIR Identifier
export interface FhirIdentifier {
  system: string;
  value: string;
  use?: string;
  type_text?: string;
}

// FHIR Quantity
export interface FhirQuantity {
  value: number;
  unit: string;
  system?: string;
  code?: string;
}

// FHIR Dosage
export interface FhirDosage {
  text?: string;
  route?: string;
  dose_quantity?: FhirQuantity;
  frequency?: string;
}

// FHIR Patient Mapping
export interface FhirPatientMapping {
  internal_patient_hash: ActionHash;
  fhir_patient_id: string;
  fhir_identifiers: FhirIdentifier[];
  mapping_version: string;
  last_synced: Timestamp;
}

// FHIR Observation Mapping
export interface FhirObservationMapping {
  internal_record_hash: ActionHash;
  fhir_observation_id: string;
  loinc_code: string;
  snomed_code?: string;
  value_quantity?: FhirQuantity;
  effective_datetime: Timestamp;
}

// FHIR Condition Mapping
export interface FhirConditionMapping {
  internal_diagnosis_hash: ActionHash;
  fhir_condition_id: string;
  icd10_code: string;
  snomed_code?: string;
  clinical_status: string;
  verification_status: string;
}

// FHIR Medication Mapping
export interface FhirMedicationMapping {
  internal_medication_hash: ActionHash;
  fhir_medication_id: string;
  rxnorm_code: string;
  ndc_code?: string;
  dosage: FhirDosage;
}

// FHIR Bundle Entry
export interface FhirBundleEntry {
  resource_type: string;
  resource_id: string;
  full_url: string;
  resource_json: string;
}

// FHIR Bundle Record
export interface FhirBundleRecord {
  bundle_id: string;
  bundle_type: string;
  patient_hash: ActionHash;
  entries: FhirBundleEntry[];
  exported_at: Timestamp;
}

// Input types
export interface CreatePatientMappingInput {
  internal_patient_hash: ActionHash;
  fhir_patient_id: string;
  fhir_identifiers: FhirIdentifier[];
}

export interface ExportPatientBundleInput {
  patient_hash: ActionHash;
  include_observations?: boolean;
  include_conditions?: boolean;
  include_medications?: boolean;
}

export interface ImportFhirBundleInput {
  bundle_json: string;
  patient_hash?: ActionHash;
}

/**
 * FHIR Mapping Zome Client
 */
export class FhirMappingClient {
  private readonly roleName: string;
  private readonly zomeName = 'fhir_mapping';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  /**
   * Create a FHIR Patient mapping
   */
  async createPatientMapping(input: CreatePatientMappingInput): Promise<ActionHash> {
    return this.call<ActionHash>('create_fhir_patient_mapping', input);
  }

  /**
   * Get FHIR Patient mapping for an internal patient
   */
  async getPatientMapping(patientHash: ActionHash): Promise<FhirPatientMapping | null> {
    return this.call<FhirPatientMapping | null>('get_fhir_patient_mapping', patientHash);
  }

  /**
   * Create a FHIR Observation mapping
   */
  async createObservationMapping(
    internalRecordHash: ActionHash,
    fhirObservationId: string,
    loincCode: string,
    snomedCode?: string
  ): Promise<ActionHash> {
    return this.call<ActionHash>('create_fhir_observation_mapping', {
      internal_record_hash: internalRecordHash,
      fhir_observation_id: fhirObservationId,
      loinc_code: loincCode,
      snomed_code: snomedCode,
    });
  }

  /**
   * Create a FHIR Condition mapping
   */
  async createConditionMapping(
    internalDiagnosisHash: ActionHash,
    fhirConditionId: string,
    icd10Code: string,
    snomedCode?: string
  ): Promise<ActionHash> {
    return this.call<ActionHash>('create_fhir_condition_mapping', {
      internal_diagnosis_hash: internalDiagnosisHash,
      fhir_condition_id: fhirConditionId,
      icd10_code: icd10Code,
      snomed_code: snomedCode,
    });
  }

  /**
   * Create a FHIR Medication mapping
   */
  async createMedicationMapping(
    internalMedicationHash: ActionHash,
    fhirMedicationId: string,
    rxnormCode: string,
    dosage: FhirDosage,
    ndcCode?: string
  ): Promise<ActionHash> {
    return this.call<ActionHash>('create_fhir_medication_mapping', {
      internal_medication_hash: internalMedicationHash,
      fhir_medication_id: fhirMedicationId,
      rxnorm_code: rxnormCode,
      ndc_code: ndcCode,
      dosage,
    });
  }

  /**
   * Export a patient's data as a FHIR Bundle
   */
  async exportPatientBundle(input: ExportPatientBundleInput): Promise<FhirBundleRecord> {
    return this.call<FhirBundleRecord>('export_patient_bundle', {
      patient_hash: input.patient_hash,
      include_observations: input.include_observations ?? true,
      include_conditions: input.include_conditions ?? true,
      include_medications: input.include_medications ?? true,
    });
  }

  /**
   * Import data from a FHIR Bundle
   */
  async importFhirBundle(input: ImportFhirBundleInput): Promise<ActionHash[]> {
    return this.call<ActionHash[]>('import_fhir_bundle', input);
  }

  /**
   * Validate a LOINC code
   */
  async validateLoincCode(code: string): Promise<boolean> {
    return this.call<boolean>('validate_loinc_code', code);
  }

  /**
   * Validate a SNOMED CT code
   */
  async validateSnomedCode(code: string): Promise<boolean> {
    return this.call<boolean>('validate_snomed_code', code);
  }

  /**
   * Validate an ICD-10 code
   */
  async validateIcd10Code(code: string): Promise<boolean> {
    return this.call<boolean>('validate_icd10_code', code);
  }

  /**
   * Validate an RxNorm code
   */
  async validateRxnormCode(code: string): Promise<boolean> {
    return this.call<boolean>('validate_rxnorm_code', code);
  }

  /**
   * Get all FHIR mappings for a patient
   */
  async getPatientFhirMappings(patientHash: ActionHash): Promise<{
    patient?: FhirPatientMapping;
    observations: FhirObservationMapping[];
    conditions: FhirConditionMapping[];
    medications: FhirMedicationMapping[];
  }> {
    return this.call('get_patient_fhir_mappings', patientHash);
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
        `FHIR Mapping zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
