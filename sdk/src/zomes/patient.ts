/**
 * Patient Zome Client
 *
 * Client for patient record management in Mycelix-Health.
 */

import type { AppClient, ActionHash, AgentPubKey } from '@holochain/client';
import type { Patient, ContactInfo, EmergencyContact } from '../types';
import { HealthSdkError, HealthSdkErrorCode } from '../types';

/**
 * Input for creating a patient record
 */
export interface CreatePatientInput {
  first_name: string;
  last_name: string;
  date_of_birth: string;
  mrn?: string;
  contact: ContactInfo;
  emergency_contacts?: EmergencyContact[];
  allergies?: string[];
  medications?: string[];
  insurance_id?: string;
}

/**
 * Patient search criteria
 */
export interface PatientSearchCriteria {
  name?: string;
  mrn?: string;
  date_of_birth?: string;
  limit?: number;
}

/**
 * Patient record with hash
 */
export interface PatientRecord {
  hash: ActionHash;
  patient: Patient;
}

/**
 * Patient Zome Client
 */
export class PatientClient {
  private readonly roleName: string;
  private readonly zomeName = 'patient';

  constructor(
    private readonly client: AppClient,
    roleName: string
  ) {
    this.roleName = roleName;
  }

  /**
   * Create a new patient record
   *
   * @param input - Patient information
   * @returns Created patient record with hash
   */
  async createPatient(input: CreatePatientInput): Promise<PatientRecord> {
    const result = await this.call<PatientRecord>('create_patient', {
      ...input,
      emergency_contacts: input.emergency_contacts ?? [],
      allergies: input.allergies ?? [],
      medications: input.medications ?? [],
    });
    return result;
  }

  /**
   * Get a patient record by hash
   *
   * @param patientHash - Hash of the patient record
   * @returns Patient record or null if not found
   */
  async getPatient(patientHash: ActionHash): Promise<Patient | null> {
    return this.call<Patient | null>('get_patient', patientHash);
  }

  /**
   * Get the current agent's patient record
   *
   * @returns Patient record for the current agent, or null if none exists
   */
  async getMyPatient(): Promise<PatientRecord | null> {
    return this.call<PatientRecord | null>('get_my_patient', null);
  }

  /**
   * Update a patient record
   *
   * @param patientHash - Hash of the record to update
   * @param updates - Partial patient data to update
   * @returns Updated patient record
   */
  async updatePatient(
    patientHash: ActionHash,
    updates: Partial<CreatePatientInput>
  ): Promise<PatientRecord> {
    return this.call<PatientRecord>('update_patient', {
      original_hash: patientHash,
      updates,
    });
  }

  /**
   * Search for patients by criteria
   *
   * @param criteria - Search criteria
   * @returns Matching patient records
   */
  async searchPatients(criteria: PatientSearchCriteria): Promise<PatientRecord[]> {
    if (criteria.name) {
      return this.call<PatientRecord[]>('search_patients_by_name', {
        name: criteria.name,
        limit: criteria.limit ?? 50,
      });
    }

    if (criteria.mrn) {
      const result = await this.call<PatientRecord | null>('get_patient_by_mrn', criteria.mrn);
      return result ? [result] : [];
    }

    throw new HealthSdkError(
      HealthSdkErrorCode.INVALID_INPUT,
      'Search criteria must include either name or mrn',
      { criteria }
    );
  }

  /**
   * Add an allergy to a patient record
   *
   * @param patientHash - Hash of the patient
   * @param allergy - Allergy to add
   */
  async addAllergy(patientHash: ActionHash, allergy: string): Promise<void> {
    await this.call<void>('add_allergy', {
      patient_hash: patientHash,
      allergy,
    });
  }

  /**
   * Add a medication to a patient record
   *
   * @param patientHash - Hash of the patient
   * @param medication - Medication to add
   */
  async addMedication(patientHash: ActionHash, medication: string): Promise<void> {
    await this.call<void>('add_medication', {
      patient_hash: patientHash,
      medication,
    });
  }

  /**
   * Set the primary care provider for a patient
   *
   * @param patientHash - Hash of the patient
   * @param providerKey - Agent public key of the provider
   */
  async setPrimaryProvider(
    patientHash: ActionHash,
    providerKey: AgentPubKey
  ): Promise<void> {
    await this.call<void>('set_primary_provider', {
      patient_hash: patientHash,
      provider: providerKey,
    });
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
        `Patient zome call failed: ${message}`,
        { fnName, payload }
      );
    }
  }
}
