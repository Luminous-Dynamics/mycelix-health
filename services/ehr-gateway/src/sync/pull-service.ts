/**
 * Pull Service
 *
 * Handles pulling data from external EHR systems into Mycelix-Health.
 * Transforms FHIR resources to internal Holochain format.
 */

import type { AppClient } from '@holochain/client';
import type { TokenInfo } from '../auth/token-manager.js';
import { GenericFhirAdapter } from '../adapters/generic-fhir.js';
import type {
  FhirPatient,
  FhirObservation,
  FhirCondition,
  FhirMedicationRequest,
  FhirBundle,
  SyncResult,
  SyncDirection,
} from '../types.js';

export interface PullConfig {
  holochainClient: AppClient;
  fhirAdapter: GenericFhirAdapter;
  batchSize?: number;
  maxConcurrent?: number;
  transformers?: ResourceTransformers;
}

export interface ResourceTransformers {
  patient?: (fhir: FhirPatient) => unknown;
  observation?: (fhir: FhirObservation) => unknown;
  condition?: (fhir: FhirCondition) => unknown;
  medicationRequest?: (fhir: FhirMedicationRequest) => unknown;
}

export interface PullOptions {
  resourceTypes?: string[];
  since?: Date;
  patientId?: string;
  includeRelated?: boolean;
}

export class PullService {
  private config: PullConfig;
  private syncResults: SyncResult[] = [];

  constructor(config: PullConfig) {
    this.config = {
      batchSize: 100,
      maxConcurrent: 5,
      ...config,
    };
  }

  /**
   * Pull all data for a patient from EHR
   */
  async pullPatientData(
    patientId: string,
    tokenInfo: TokenInfo,
    options: PullOptions = {}
  ): Promise<SyncResult[]> {
    this.syncResults = [];

    const resourceTypes = options.resourceTypes || [
      'Patient',
      'Observation',
      'Condition',
      'MedicationRequest',
    ];

    for (const resourceType of resourceTypes) {
      try {
        await this.pullResourceType(patientId, resourceType, tokenInfo, options);
      } catch (error) {
        this.recordResult({
          success: false,
          resourceType,
          resourceId: patientId,
          direction: 'pull',
          timestamp: new Date(),
          errors: [(error as Error).message],
        });
      }
    }

    return this.syncResults;
  }

  /**
   * Pull a specific resource type for a patient
   */
  private async pullResourceType(
    patientId: string,
    resourceType: string,
    tokenInfo: TokenInfo,
    options: PullOptions
  ): Promise<void> {
    let bundle: FhirBundle;

    switch (resourceType) {
      case 'Patient':
        const patient = await this.config.fhirAdapter.getPatient(patientId, tokenInfo);
        await this.processPatient(patient);
        break;

      case 'Observation':
        bundle = await this.config.fhirAdapter.getPatientObservations(patientId, tokenInfo);
        await this.processBundleEntries(bundle, 'Observation', this.processObservation.bind(this));
        break;

      case 'Condition':
        bundle = await this.config.fhirAdapter.getPatientConditions(patientId, tokenInfo);
        await this.processBundleEntries(bundle, 'Condition', this.processCondition.bind(this));
        break;

      case 'MedicationRequest':
        bundle = await this.config.fhirAdapter.getPatientMedications(patientId, tokenInfo);
        await this.processBundleEntries(bundle, 'MedicationRequest', this.processMedication.bind(this));
        break;

      default:
        throw new Error(`Unsupported resource type: ${resourceType}`);
    }
  }

  /**
   * Process bundle entries with batching
   */
  private async processBundleEntries<T>(
    bundle: FhirBundle,
    resourceType: string,
    processor: (resource: T) => Promise<void>
  ): Promise<void> {
    if (!bundle.entry) return;

    const entries = bundle.entry.filter(e => e.resource?.resourceType === resourceType);
    const batchSize = this.config.batchSize || 100;

    for (let i = 0; i < entries.length; i += batchSize) {
      const batch = entries.slice(i, i + batchSize);
      await Promise.all(
        batch.map(entry => processor(entry.resource as T))
      );
    }
  }

  /**
   * Process a FHIR Patient resource
   */
  private async processPatient(patient: FhirPatient): Promise<void> {
    try {
      const transformed = this.transformPatient(patient);

      // Store in Holochain via zome call
      await this.config.holochainClient.callZome({
        cap_secret: null,
        role_name: 'health',
        zome_name: 'fhir_mapping',
        fn_name: 'import_fhir_patient',
        payload: {
          fhir_patient: patient,
          internal_data: transformed,
        },
      });

      this.recordResult({
        success: true,
        resourceType: 'Patient',
        resourceId: patient.id || 'unknown',
        direction: 'pull',
        timestamp: new Date(),
        errors: [],
      });
    } catch (error) {
      this.recordResult({
        success: false,
        resourceType: 'Patient',
        resourceId: patient.id || 'unknown',
        direction: 'pull',
        timestamp: new Date(),
        errors: [(error as Error).message],
      });
    }
  }

  /**
   * Process a FHIR Observation resource
   */
  private async processObservation(observation: FhirObservation): Promise<void> {
    try {
      const transformed = this.transformObservation(observation);

      await this.config.holochainClient.callZome({
        cap_secret: null,
        role_name: 'health',
        zome_name: 'fhir_mapping',
        fn_name: 'import_fhir_observation',
        payload: {
          fhir_observation: observation,
          internal_data: transformed,
        },
      });

      this.recordResult({
        success: true,
        resourceType: 'Observation',
        resourceId: observation.id || 'unknown',
        direction: 'pull',
        timestamp: new Date(),
        errors: [],
      });
    } catch (error) {
      this.recordResult({
        success: false,
        resourceType: 'Observation',
        resourceId: observation.id || 'unknown',
        direction: 'pull',
        timestamp: new Date(),
        errors: [(error as Error).message],
      });
    }
  }

  /**
   * Process a FHIR Condition resource
   */
  private async processCondition(condition: FhirCondition): Promise<void> {
    try {
      const transformed = this.transformCondition(condition);

      await this.config.holochainClient.callZome({
        cap_secret: null,
        role_name: 'health',
        zome_name: 'fhir_mapping',
        fn_name: 'import_fhir_condition',
        payload: {
          fhir_condition: condition,
          internal_data: transformed,
        },
      });

      this.recordResult({
        success: true,
        resourceType: 'Condition',
        resourceId: condition.id || 'unknown',
        direction: 'pull',
        timestamp: new Date(),
        errors: [],
      });
    } catch (error) {
      this.recordResult({
        success: false,
        resourceType: 'Condition',
        resourceId: condition.id || 'unknown',
        direction: 'pull',
        timestamp: new Date(),
        errors: [(error as Error).message],
      });
    }
  }

  /**
   * Process a FHIR MedicationRequest resource
   */
  private async processMedication(medication: FhirMedicationRequest): Promise<void> {
    try {
      const transformed = this.transformMedication(medication);

      await this.config.holochainClient.callZome({
        cap_secret: null,
        role_name: 'health',
        zome_name: 'fhir_mapping',
        fn_name: 'import_fhir_medication',
        payload: {
          fhir_medication: medication,
          internal_data: transformed,
        },
      });

      this.recordResult({
        success: true,
        resourceType: 'MedicationRequest',
        resourceId: medication.id || 'unknown',
        direction: 'pull',
        timestamp: new Date(),
        errors: [],
      });
    } catch (error) {
      this.recordResult({
        success: false,
        resourceType: 'MedicationRequest',
        resourceId: medication.id || 'unknown',
        direction: 'pull',
        timestamp: new Date(),
        errors: [(error as Error).message],
      });
    }
  }

  // Transformation functions

  private transformPatient(patient: FhirPatient): unknown {
    if (this.config.transformers?.patient) {
      return this.config.transformers.patient(patient);
    }

    // Default transformation to internal format
    const name = patient.name?.[0];
    return {
      first_name: name?.given?.join(' ') || '',
      last_name: name?.family || '',
      date_of_birth: patient.birthDate || '',
      gender: patient.gender || 'unknown',
      identifiers: patient.identifier?.map(id => ({
        system: id.system,
        value: id.value,
      })) || [],
      contact: {
        phone: patient.telecom?.find(t => t.system === 'phone')?.value,
        email: patient.telecom?.find(t => t.system === 'email')?.value,
        address: patient.address?.[0],
      },
    };
  }

  private transformObservation(observation: FhirObservation): unknown {
    if (this.config.transformers?.observation) {
      return this.config.transformers.observation(observation);
    }

    return {
      code: observation.code.coding?.[0]?.code || '',
      code_system: observation.code.coding?.[0]?.system || '',
      display: observation.code.coding?.[0]?.display || observation.code.text || '',
      value: observation.valueQuantity?.value,
      unit: observation.valueQuantity?.unit,
      effective_date: observation.effectiveDateTime,
      status: observation.status,
      interpretation: observation.interpretation?.[0]?.coding?.[0]?.code,
    };
  }

  private transformCondition(condition: FhirCondition): unknown {
    if (this.config.transformers?.condition) {
      return this.config.transformers.condition(condition);
    }

    return {
      code: condition.code?.coding?.[0]?.code || '',
      code_system: condition.code?.coding?.[0]?.system || '',
      display: condition.code?.coding?.[0]?.display || condition.code?.text || '',
      clinical_status: condition.clinicalStatus?.coding?.[0]?.code || 'unknown',
      verification_status: condition.verificationStatus?.coding?.[0]?.code || 'unknown',
      onset_date: condition.onsetDateTime,
      recorded_date: condition.recordedDate,
    };
  }

  private transformMedication(medication: FhirMedicationRequest): unknown {
    if (this.config.transformers?.medicationRequest) {
      return this.config.transformers.medicationRequest(medication);
    }

    const medicationCode = medication.medicationCodeableConcept;
    const dosage = medication.dosageInstruction?.[0];

    return {
      code: medicationCode?.coding?.[0]?.code || '',
      code_system: medicationCode?.coding?.[0]?.system || '',
      display: medicationCode?.coding?.[0]?.display || medicationCode?.text || '',
      status: medication.status,
      intent: medication.intent,
      dosage_text: dosage?.text,
      route: dosage?.route?.coding?.[0]?.display,
      authored_on: medication.authoredOn,
    };
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
   * Get summary of sync results
   */
  getSummary(): { total: number; success: number; failed: number; byType: Record<string, { success: number; failed: number }> } {
    const summary = {
      total: this.syncResults.length,
      success: 0,
      failed: 0,
      byType: {} as Record<string, { success: number; failed: number }>,
    };

    for (const result of this.syncResults) {
      if (result.success) {
        summary.success++;
      } else {
        summary.failed++;
      }

      if (!summary.byType[result.resourceType]) {
        summary.byType[result.resourceType] = { success: 0, failed: 0 };
      }

      if (result.success) {
        summary.byType[result.resourceType].success++;
      } else {
        summary.byType[result.resourceType].failed++;
      }
    }

    return summary;
  }
}
