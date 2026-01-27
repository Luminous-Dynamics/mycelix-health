/**
 * Push Service
 *
 * Handles pushing data from Mycelix-Health to external EHR systems.
 * Transforms internal Holochain format to FHIR resources.
 */

import type { AppClient, ActionHash } from '@holochain/client';
import type { TokenInfo } from '../auth/token-manager.js';
import { GenericFhirAdapter } from '../adapters/generic-fhir.js';
import type {
  FhirPatient,
  FhirObservation,
  FhirCondition,
  FhirMedicationRequest,
  FhirBundle,
  SyncResult,
} from '../types.js';

export interface PushConfig {
  holochainClient: AppClient;
  fhirAdapter: GenericFhirAdapter;
  batchSize?: number;
  validateBeforePush?: boolean;
}

export interface PushOptions {
  resourceTypes?: string[];
  recordHashes?: ActionHash[];
  dryRun?: boolean;
}

export interface InternalPatient {
  action_hash: ActionHash;
  first_name: string;
  last_name: string;
  date_of_birth: string;
  gender: string;
  identifiers: Array<{ system: string; value: string }>;
  contact?: {
    phone?: string;
    email?: string;
    address?: {
      line?: string[];
      city?: string;
      state?: string;
      postalCode?: string;
      country?: string;
    };
  };
}

export interface InternalObservation {
  action_hash: ActionHash;
  patient_hash: ActionHash;
  code: string;
  code_system: string;
  display: string;
  value?: number;
  unit?: string;
  effective_date?: string;
  status: string;
}

export interface InternalCondition {
  action_hash: ActionHash;
  patient_hash: ActionHash;
  code: string;
  code_system: string;
  display: string;
  clinical_status: string;
  verification_status: string;
  onset_date?: string;
}

export interface InternalMedication {
  action_hash: ActionHash;
  patient_hash: ActionHash;
  code: string;
  code_system: string;
  display: string;
  status: string;
  intent: string;
  dosage_text?: string;
  route?: string;
}

export class PushService {
  private config: PushConfig;
  private syncResults: SyncResult[] = [];

  constructor(config: PushConfig) {
    this.config = {
      batchSize: 50,
      validateBeforePush: true,
      ...config,
    };
  }

  /**
   * Push patient data to EHR
   */
  async pushPatientData(
    patientHash: ActionHash,
    tokenInfo: TokenInfo,
    options: PushOptions = {}
  ): Promise<SyncResult[]> {
    this.syncResults = [];

    const resourceTypes = options.resourceTypes || [
      'Patient',
      'Observation',
      'Condition',
      'MedicationRequest',
    ];

    // Get FHIR mappings from Holochain
    const mappings = await this.getFhirMappings(patientHash);

    for (const resourceType of resourceTypes) {
      try {
        await this.pushResourceType(
          patientHash,
          resourceType,
          mappings,
          tokenInfo,
          options
        );
      } catch (error) {
        this.recordResult({
          success: false,
          resourceType,
          resourceId: patientHash.toString(),
          direction: 'push',
          timestamp: new Date(),
          errors: [(error as Error).message],
        });
      }
    }

    return this.syncResults;
  }

  /**
   * Get FHIR mappings from Holochain
   */
  private async getFhirMappings(patientHash: ActionHash): Promise<{
    patient?: { fhir_id?: string };
    observations: Array<{ internal_hash: ActionHash; fhir_id?: string }>;
    conditions: Array<{ internal_hash: ActionHash; fhir_id?: string }>;
    medications: Array<{ internal_hash: ActionHash; fhir_id?: string }>;
  }> {
    const result = await this.config.holochainClient.callZome({
      cap_secret: undefined,
      role_name: 'health',
      zome_name: 'fhir_mapping',
      fn_name: 'get_patient_fhir_mappings',
      payload: patientHash,
    });

    return result as {
      patient?: { fhir_id?: string };
      observations: Array<{ internal_hash: ActionHash; fhir_id?: string }>;
      conditions: Array<{ internal_hash: ActionHash; fhir_id?: string }>;
      medications: Array<{ internal_hash: ActionHash; fhir_id?: string }>;
    };
  }

  /**
   * Push a specific resource type
   */
  private async pushResourceType(
    patientHash: ActionHash,
    resourceType: string,
    mappings: { patient?: { fhir_id?: string }; observations: Array<{ internal_hash: ActionHash; fhir_id?: string }>; conditions: Array<{ internal_hash: ActionHash; fhir_id?: string }>; medications: Array<{ internal_hash: ActionHash; fhir_id?: string }> },
    tokenInfo: TokenInfo,
    options: PushOptions
  ): Promise<void> {
    switch (resourceType) {
      case 'Patient':
        await this.pushPatient(patientHash, mappings.patient, tokenInfo, options);
        break;

      case 'Observation':
        await this.pushObservations(patientHash, mappings.observations, tokenInfo, options);
        break;

      case 'Condition':
        await this.pushConditions(patientHash, mappings.conditions, tokenInfo, options);
        break;

      case 'MedicationRequest':
        await this.pushMedications(patientHash, mappings.medications, tokenInfo, options);
        break;
    }
  }

  /**
   * Push patient to EHR
   */
  private async pushPatient(
    patientHash: ActionHash,
    mapping: { fhir_id?: string } | undefined,
    tokenInfo: TokenInfo,
    options: PushOptions
  ): Promise<void> {
    try {
      // Get internal patient data
      const internalPatient = await this.config.holochainClient.callZome({
        cap_secret: undefined,
        role_name: 'health',
        zome_name: 'patient',
        fn_name: 'get_patient',
        payload: patientHash,
      }) as InternalPatient;

      // Transform to FHIR
      const fhirPatient = this.transformToFhirPatient(internalPatient);

      if (options.dryRun) {
        this.recordResult({
          success: true,
          resourceType: 'Patient',
          resourceId: patientHash.toString(),
          direction: 'push',
          timestamp: new Date(),
          errors: [],
        });
        return;
      }

      // Create or update in EHR
      let result: FhirPatient;
      if (mapping?.fhir_id) {
        fhirPatient.id = mapping.fhir_id;
        result = await this.config.fhirAdapter.updateResource(fhirPatient, tokenInfo);
      } else {
        result = await this.config.fhirAdapter.createResource(fhirPatient, tokenInfo);
      }

      // Update mapping in Holochain
      await this.updateFhirMapping(patientHash, 'Patient', result.id || '');

      this.recordResult({
        success: true,
        resourceType: 'Patient',
        resourceId: result.id || patientHash.toString(),
        direction: 'push',
        timestamp: new Date(),
        errors: [],
      });
    } catch (error) {
      this.recordResult({
        success: false,
        resourceType: 'Patient',
        resourceId: patientHash.toString(),
        direction: 'push',
        timestamp: new Date(),
        errors: [(error as Error).message],
      });
    }
  }

  /**
   * Push observations to EHR
   */
  private async pushObservations(
    patientHash: ActionHash,
    mappings: Array<{ internal_hash: ActionHash; fhir_id?: string }>,
    tokenInfo: TokenInfo,
    options: PushOptions
  ): Promise<void> {
    // Get all observations for patient
    const observations = await this.config.holochainClient.callZome({
      cap_secret: undefined,
      role_name: 'health',
      zome_name: 'medical_records',
      fn_name: 'get_patient_observations',
      payload: patientHash,
    }) as InternalObservation[];

    for (const observation of observations) {
      const mapping = mappings.find(m =>
        m.internal_hash.toString() === observation.action_hash.toString()
      );

      await this.pushObservation(observation, patientHash, mapping, tokenInfo, options);
    }
  }

  /**
   * Push single observation
   */
  private async pushObservation(
    observation: InternalObservation,
    patientHash: ActionHash,
    mapping: { internal_hash: ActionHash; fhir_id?: string } | undefined,
    tokenInfo: TokenInfo,
    options: PushOptions
  ): Promise<void> {
    try {
      const fhirObservation = this.transformToFhirObservation(observation, patientHash);

      if (options.dryRun) {
        this.recordResult({
          success: true,
          resourceType: 'Observation',
          resourceId: observation.action_hash.toString(),
          direction: 'push',
          timestamp: new Date(),
          errors: [],
        });
        return;
      }

      let result: FhirObservation;
      if (mapping?.fhir_id) {
        fhirObservation.id = mapping.fhir_id;
        result = await this.config.fhirAdapter.updateResource(fhirObservation, tokenInfo);
      } else {
        result = await this.config.fhirAdapter.createResource(fhirObservation, tokenInfo);
      }

      await this.updateFhirMapping(observation.action_hash, 'Observation', result.id || '');

      this.recordResult({
        success: true,
        resourceType: 'Observation',
        resourceId: result.id || observation.action_hash.toString(),
        direction: 'push',
        timestamp: new Date(),
        errors: [],
      });
    } catch (error) {
      this.recordResult({
        success: false,
        resourceType: 'Observation',
        resourceId: observation.action_hash.toString(),
        direction: 'push',
        timestamp: new Date(),
        errors: [(error as Error).message],
      });
    }
  }

  /**
   * Push conditions to EHR
   */
  private async pushConditions(
    patientHash: ActionHash,
    mappings: Array<{ internal_hash: ActionHash; fhir_id?: string }>,
    tokenInfo: TokenInfo,
    options: PushOptions
  ): Promise<void> {
    const conditions = await this.config.holochainClient.callZome({
      cap_secret: undefined,
      role_name: 'health',
      zome_name: 'medical_records',
      fn_name: 'get_patient_conditions',
      payload: patientHash,
    }) as InternalCondition[];

    for (const condition of conditions) {
      const mapping = mappings.find(m =>
        m.internal_hash.toString() === condition.action_hash.toString()
      );

      await this.pushCondition(condition, patientHash, mapping, tokenInfo, options);
    }
  }

  /**
   * Push single condition
   */
  private async pushCondition(
    condition: InternalCondition,
    patientHash: ActionHash,
    mapping: { internal_hash: ActionHash; fhir_id?: string } | undefined,
    tokenInfo: TokenInfo,
    options: PushOptions
  ): Promise<void> {
    try {
      const fhirCondition = this.transformToFhirCondition(condition, patientHash);

      if (options.dryRun) {
        this.recordResult({
          success: true,
          resourceType: 'Condition',
          resourceId: condition.action_hash.toString(),
          direction: 'push',
          timestamp: new Date(),
          errors: [],
        });
        return;
      }

      let result: FhirCondition;
      if (mapping?.fhir_id) {
        fhirCondition.id = mapping.fhir_id;
        result = await this.config.fhirAdapter.updateResource(fhirCondition, tokenInfo);
      } else {
        result = await this.config.fhirAdapter.createResource(fhirCondition, tokenInfo);
      }

      await this.updateFhirMapping(condition.action_hash, 'Condition', result.id || '');

      this.recordResult({
        success: true,
        resourceType: 'Condition',
        resourceId: result.id || condition.action_hash.toString(),
        direction: 'push',
        timestamp: new Date(),
        errors: [],
      });
    } catch (error) {
      this.recordResult({
        success: false,
        resourceType: 'Condition',
        resourceId: condition.action_hash.toString(),
        direction: 'push',
        timestamp: new Date(),
        errors: [(error as Error).message],
      });
    }
  }

  /**
   * Push medications to EHR
   */
  private async pushMedications(
    patientHash: ActionHash,
    mappings: Array<{ internal_hash: ActionHash; fhir_id?: string }>,
    tokenInfo: TokenInfo,
    options: PushOptions
  ): Promise<void> {
    const medications = await this.config.holochainClient.callZome({
      cap_secret: undefined,
      role_name: 'health',
      zome_name: 'medical_records',
      fn_name: 'get_patient_medications',
      payload: patientHash,
    }) as InternalMedication[];

    for (const medication of medications) {
      const mapping = mappings.find(m =>
        m.internal_hash.toString() === medication.action_hash.toString()
      );

      await this.pushMedication(medication, patientHash, mapping, tokenInfo, options);
    }
  }

  /**
   * Push single medication
   */
  private async pushMedication(
    medication: InternalMedication,
    patientHash: ActionHash,
    mapping: { internal_hash: ActionHash; fhir_id?: string } | undefined,
    tokenInfo: TokenInfo,
    options: PushOptions
  ): Promise<void> {
    try {
      const fhirMedication = this.transformToFhirMedication(medication, patientHash);

      if (options.dryRun) {
        this.recordResult({
          success: true,
          resourceType: 'MedicationRequest',
          resourceId: medication.action_hash.toString(),
          direction: 'push',
          timestamp: new Date(),
          errors: [],
        });
        return;
      }

      let result: FhirMedicationRequest;
      if (mapping?.fhir_id) {
        fhirMedication.id = mapping.fhir_id;
        result = await this.config.fhirAdapter.updateResource(fhirMedication, tokenInfo);
      } else {
        result = await this.config.fhirAdapter.createResource(fhirMedication, tokenInfo);
      }

      await this.updateFhirMapping(medication.action_hash, 'MedicationRequest', result.id || '');

      this.recordResult({
        success: true,
        resourceType: 'MedicationRequest',
        resourceId: result.id || medication.action_hash.toString(),
        direction: 'push',
        timestamp: new Date(),
        errors: [],
      });
    } catch (error) {
      this.recordResult({
        success: false,
        resourceType: 'MedicationRequest',
        resourceId: medication.action_hash.toString(),
        direction: 'push',
        timestamp: new Date(),
        errors: [(error as Error).message],
      });
    }
  }

  // Transformation functions

  private transformToFhirPatient(internal: InternalPatient): FhirPatient {
    const fhir: FhirPatient = {
      resourceType: 'Patient',
      name: [{
        family: internal.last_name,
        given: internal.first_name.split(' '),
      }],
      birthDate: internal.date_of_birth,
      gender: internal.gender as 'male' | 'female' | 'other' | 'unknown',
      identifier: internal.identifiers.map(id => ({
        system: id.system,
        value: id.value,
      })),
    };

    if (internal.contact) {
      fhir.telecom = [];
      if (internal.contact.phone) {
        fhir.telecom.push({ system: 'phone', value: internal.contact.phone });
      }
      if (internal.contact.email) {
        fhir.telecom.push({ system: 'email', value: internal.contact.email });
      }
      if (internal.contact.address) {
        fhir.address = [internal.contact.address];
      }
    }

    return fhir;
  }

  private transformToFhirObservation(
    internal: InternalObservation,
    patientHash: ActionHash
  ): FhirObservation {
    const fhir: FhirObservation = {
      resourceType: 'Observation',
      status: internal.status,
      code: {
        coding: [{
          system: internal.code_system,
          code: internal.code,
          display: internal.display,
        }],
      },
      subject: {
        reference: `Patient/${patientHash.toString()}`,
      },
    };

    if (internal.value !== undefined && internal.unit) {
      fhir.valueQuantity = {
        value: internal.value,
        unit: internal.unit,
      };
    }

    if (internal.effective_date) {
      fhir.effectiveDateTime = internal.effective_date;
    }

    return fhir;
  }

  private transformToFhirCondition(
    internal: InternalCondition,
    patientHash: ActionHash
  ): FhirCondition {
    return {
      resourceType: 'Condition',
      code: {
        coding: [{
          system: internal.code_system,
          code: internal.code,
          display: internal.display,
        }],
      },
      clinicalStatus: {
        coding: [{
          system: 'http://terminology.hl7.org/CodeSystem/condition-clinical',
          code: internal.clinical_status,
        }],
      },
      verificationStatus: {
        coding: [{
          system: 'http://terminology.hl7.org/CodeSystem/condition-ver-status',
          code: internal.verification_status,
        }],
      },
      subject: {
        reference: `Patient/${patientHash.toString()}`,
      },
      onsetDateTime: internal.onset_date,
    };
  }

  private transformToFhirMedication(
    internal: InternalMedication,
    patientHash: ActionHash
  ): FhirMedicationRequest {
    const fhir: FhirMedicationRequest = {
      resourceType: 'MedicationRequest',
      status: internal.status,
      intent: internal.intent,
      medicationCodeableConcept: {
        coding: [{
          system: internal.code_system,
          code: internal.code,
          display: internal.display,
        }],
      },
      subject: {
        reference: `Patient/${patientHash.toString()}`,
      },
    };

    if (internal.dosage_text) {
      fhir.dosageInstruction = [{
        text: internal.dosage_text,
      }];

      if (internal.route) {
        fhir.dosageInstruction[0].route = {
          text: internal.route,
        };
      }
    }

    return fhir;
  }

  /**
   * Update FHIR mapping in Holochain
   */
  private async updateFhirMapping(
    internalHash: ActionHash,
    resourceType: string,
    fhirId: string
  ): Promise<void> {
    await this.config.holochainClient.callZome({
      cap_secret: undefined,
      role_name: 'health',
      zome_name: 'fhir_mapping',
      fn_name: 'update_fhir_mapping',
      payload: {
        internal_hash: internalHash,
        resource_type: resourceType,
        fhir_id: fhirId,
      },
    });
  }

  /**
   * Record a sync result
   */
  private recordResult(result: SyncResult): void {
    this.syncResults.push(result);
  }

  /**
   * Get all sync results
   */
  getResults(): SyncResult[] {
    return [...this.syncResults];
  }

  /**
   * Get summary
   */
  getSummary(): { total: number; success: number; failed: number } {
    return {
      total: this.syncResults.length,
      success: this.syncResults.filter(r => r.success).length,
      failed: this.syncResults.filter(r => !r.success).length,
    };
  }
}
