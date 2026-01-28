/**
 * Pull Service
 *
 * Handles pulling data from external EHR systems into Mycelix-Health.
 * Fetches FHIR resources and ingests them as a bundle into Holochain
 * using the fhir_bridge zome's ingest_bundle function.
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
  IngestBundleInput,
  IngestReport,
} from '../types.js';

export interface PullConfig {
  holochainClient: AppClient;
  fhirAdapter: GenericFhirAdapter;
  batchSize?: number;
  maxConcurrent?: number;
  /** Default source system identifier */
  defaultSourceSystem?: string;
}

export interface PullOptions {
  /** Which FHIR resource types to fetch */
  resourceTypes?: string[];
  /** Only fetch resources modified since this date */
  since?: Date;
  /** Patient ID in the EHR system */
  patientId?: string;
  /** Include related resources */
  includeRelated?: boolean;
  /** Override the source system identifier */
  sourceSystem?: string;
}

export interface PullResult {
  /** Individual sync results for each resource type fetch */
  syncResults: SyncResult[];
  /** The assembled FHIR bundle that was ingested */
  bundle: FhirBundle;
  /** Report from the Holochain ingest operation */
  ingestReport: IngestReport;
}

/**
 * Default resource types to fetch from EHR
 */
const DEFAULT_RESOURCE_TYPES = [
  'Patient',
  'Observation',
  'Condition',
  'MedicationRequest',
  'AllergyIntolerance',
  'Immunization',
  'Procedure',
];

export class PullService {
  private config: PullConfig;
  private syncResults: SyncResult[] = [];

  constructor(config: PullConfig) {
    this.config = {
      batchSize: 100,
      maxConcurrent: 5,
      defaultSourceSystem: 'unknown-ehr',
      ...config,
    };
  }

  /**
   * Pull all data for a patient from EHR and ingest into Holochain
   *
   * This method:
   * 1. Fetches all requested resource types from the EHR
   * 2. Assembles them into a single FHIR Bundle
   * 3. Calls the fhir_bridge zome's ingest_bundle function
   * 4. Returns the combined results
   */
  async pullPatientData(
    patientId: string,
    tokenInfo: TokenInfo,
    options: PullOptions = {}
  ): Promise<PullResult> {
    this.syncResults = [];

    const resourceTypes = options.resourceTypes || DEFAULT_RESOURCE_TYPES;
    const sourceSystem = options.sourceSystem || this.config.defaultSourceSystem || 'unknown-ehr';

    // Collect all resources into bundle entries
    const bundleEntries: Array<{ fullUrl?: string; resource: unknown }> = [];

    // Fetch each resource type and collect entries
    for (const resourceType of resourceTypes) {
      try {
        const entries = await this.fetchResourceType(patientId, resourceType, tokenInfo, options);
        bundleEntries.push(...entries);

        this.recordResult({
          success: true,
          resourceType,
          resourceId: patientId,
          direction: 'pull',
          timestamp: new Date(),
          errors: [],
        });
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

    // Build the complete FHIR Bundle
    const bundle: FhirBundle = {
      resourceType: 'Bundle',
      id: `import-${Date.now()}`,
      type: 'collection',
      timestamp: new Date().toISOString(),
      total: bundleEntries.length,
      entry: bundleEntries,
    };

    // Ingest the bundle into Holochain
    const ingestReport = await this.ingestBundle(bundle, sourceSystem);

    return {
      syncResults: this.syncResults,
      bundle,
      ingestReport,
    };
  }

  /**
   * Ingest a pre-built FHIR Bundle directly into Holochain
   *
   * Use this when you already have a complete FHIR Bundle
   * (e.g., from a webhook or direct import)
   */
  async ingestBundle(bundle: FhirBundle, sourceSystem: string): Promise<IngestReport> {
    const input: IngestBundleInput = {
      bundle,
      source_system: sourceSystem,
    };

    const result = await this.config.holochainClient.callZome({
      cap_secret: undefined,
      role_name: 'health',
      zome_name: 'fhir_bridge',
      fn_name: 'ingest_bundle',
      payload: input,
    });

    return result as IngestReport;
  }

  /**
   * Fetch a specific resource type for a patient from the EHR
   */
  private async fetchResourceType(
    patientId: string,
    resourceType: string,
    tokenInfo: TokenInfo,
    options: PullOptions
  ): Promise<Array<{ fullUrl?: string; resource: unknown }>> {
    const entries: Array<{ fullUrl?: string; resource: unknown }> = [];

    switch (resourceType) {
      case 'Patient': {
        const patient = await this.config.fhirAdapter.getPatient(patientId, tokenInfo);
        entries.push({
          fullUrl: `Patient/${patient.id}`,
          resource: patient,
        });
        break;
      }

      case 'Observation': {
        const bundle = await this.config.fhirAdapter.getPatientObservations(patientId, tokenInfo);
        if (bundle.entry) {
          for (const entry of bundle.entry) {
            if (entry.resource?.resourceType === 'Observation') {
              entries.push({
                fullUrl: entry.fullUrl,
                resource: entry.resource,
              });
            }
          }
        }
        break;
      }

      case 'Condition': {
        const bundle = await this.config.fhirAdapter.getPatientConditions(patientId, tokenInfo);
        if (bundle.entry) {
          for (const entry of bundle.entry) {
            if (entry.resource?.resourceType === 'Condition') {
              entries.push({
                fullUrl: entry.fullUrl,
                resource: entry.resource,
              });
            }
          }
        }
        break;
      }

      case 'MedicationRequest': {
        const bundle = await this.config.fhirAdapter.getPatientMedications(patientId, tokenInfo);
        if (bundle.entry) {
          for (const entry of bundle.entry) {
            if (entry.resource?.resourceType === 'MedicationRequest' ||
                entry.resource?.resourceType === 'MedicationStatement') {
              entries.push({
                fullUrl: entry.fullUrl,
                resource: entry.resource,
              });
            }
          }
        }
        break;
      }

      case 'AllergyIntolerance': {
        // Check if adapter supports allergies
        if ('getPatientAllergies' in this.config.fhirAdapter) {
          const bundle = await (this.config.fhirAdapter as any).getPatientAllergies(patientId, tokenInfo);
          if (bundle.entry) {
            for (const entry of bundle.entry) {
              if (entry.resource?.resourceType === 'AllergyIntolerance') {
                entries.push({
                  fullUrl: entry.fullUrl,
                  resource: entry.resource,
                });
              }
            }
          }
        }
        break;
      }

      case 'Immunization': {
        // Check if adapter supports immunizations
        if ('getPatientImmunizations' in this.config.fhirAdapter) {
          const bundle = await (this.config.fhirAdapter as any).getPatientImmunizations(patientId, tokenInfo);
          if (bundle.entry) {
            for (const entry of bundle.entry) {
              if (entry.resource?.resourceType === 'Immunization') {
                entries.push({
                  fullUrl: entry.fullUrl,
                  resource: entry.resource,
                });
              }
            }
          }
        }
        break;
      }

      case 'Procedure': {
        // Check if adapter supports procedures
        if ('getPatientProcedures' in this.config.fhirAdapter) {
          const bundle = await (this.config.fhirAdapter as any).getPatientProcedures(patientId, tokenInfo);
          if (bundle.entry) {
            for (const entry of bundle.entry) {
              if (entry.resource?.resourceType === 'Procedure') {
                entries.push({
                  fullUrl: entry.fullUrl,
                  resource: entry.resource,
                });
              }
            }
          }
        }
        break;
      }

      default:
        console.warn(`Unsupported resource type: ${resourceType}`);
    }

    return entries;
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
  getSummary(): {
    total: number;
    success: number;
    failed: number;
    byType: Record<string, { success: number; failed: number }>;
  } {
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

  /**
   * Create an empty IngestReport for error cases
   */
  static emptyIngestReport(sourceSystem: string): IngestReport {
    return {
      report_id: `empty-${Date.now()}`,
      source_system: sourceSystem,
      ingested_at: Date.now() * 1000, // Microseconds to match Holochain Timestamp
      total_processed: 0,
      patients_created: 0,
      patients_updated: 0,
      conditions_created: 0,
      conditions_skipped: 0,
      medications_created: 0,
      medications_skipped: 0,
      allergies_created: 0,
      allergies_skipped: 0,
      immunizations_created: 0,
      immunizations_skipped: 0,
      observations_created: 0,
      observations_skipped: 0,
      procedures_created: 0,
      procedures_skipped: 0,
      unknown_types: [],
      parse_errors: [],
    };
  }
}
