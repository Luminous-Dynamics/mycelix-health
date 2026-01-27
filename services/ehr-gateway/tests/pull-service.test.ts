/**
 * Pull Service Tests
 *
 * Tests for the PullService with ingest_bundle API alignment.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { PullService } from '../src/sync/pull-service.js';
import type { FhirBundle, IngestReport, FhirPatient } from '../src/types.js';

// Mock FHIR adapter
const mockFhirAdapter = {
  getPatient: vi.fn(),
  getPatientObservations: vi.fn(),
  getPatientConditions: vi.fn(),
  getPatientMedications: vi.fn(),
};

// Mock Holochain client
const mockHolochainClient = {
  callZome: vi.fn(),
};

// Mock token info
const mockTokenInfo = {
  access_token: 'test-token',
  token_type: 'Bearer',
  expires_in: 3600,
  scope: 'patient/*.read',
};

describe('PullService', () => {
  let pullService: PullService;

  beforeEach(() => {
    vi.clearAllMocks();

    pullService = new PullService({
      holochainClient: mockHolochainClient as any,
      fhirAdapter: mockFhirAdapter as any,
      defaultSourceSystem: 'test-ehr',
    });
  });

  describe('ingestBundle', () => {
    it('calls fhir_bridge.ingest_bundle with correct payload', async () => {
      const mockReport: IngestReport = {
        source_system: 'test-ehr',
        total_processed: 1,
        patients_created: 1,
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

      mockHolochainClient.callZome.mockResolvedValueOnce(mockReport);

      const bundle: FhirBundle = {
        resourceType: 'Bundle',
        type: 'collection',
        entry: [
          {
            fullUrl: 'Patient/123',
            resource: {
              resourceType: 'Patient',
              id: '123',
            },
          },
        ],
      };

      const result = await pullService.ingestBundle(bundle, 'test-ehr');

      // Verify the zome call was made with correct parameters
      expect(mockHolochainClient.callZome).toHaveBeenCalledWith({
        cap_secret: undefined,
        role_name: 'health',
        zome_name: 'fhir_bridge',
        fn_name: 'ingest_bundle',
        payload: {
          bundle,
          source_system: 'test-ehr',
        },
      });

      expect(result).toEqual(mockReport);
    });
  });

  describe('pullPatientData', () => {
    it('fetches resources and assembles into a bundle', async () => {
      const mockPatient: FhirPatient = {
        resourceType: 'Patient',
        id: 'patient-123',
        name: [{ family: 'Test', given: ['User'] }],
      };

      const mockConditionsBundle: FhirBundle = {
        resourceType: 'Bundle',
        type: 'searchset',
        entry: [
          {
            fullUrl: 'Condition/cond-1',
            resource: {
              resourceType: 'Condition',
              id: 'cond-1',
              subject: { reference: 'Patient/patient-123' },
            },
          },
        ],
      };

      const mockReport: IngestReport = {
        source_system: 'test-ehr',
        total_processed: 2,
        patients_created: 1,
        patients_updated: 0,
        conditions_created: 1,
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

      mockFhirAdapter.getPatient.mockResolvedValueOnce(mockPatient);
      mockFhirAdapter.getPatientObservations.mockResolvedValueOnce({ resourceType: 'Bundle', type: 'searchset', entry: [] });
      mockFhirAdapter.getPatientConditions.mockResolvedValueOnce(mockConditionsBundle);
      mockFhirAdapter.getPatientMedications.mockResolvedValueOnce({ resourceType: 'Bundle', type: 'searchset', entry: [] });
      mockHolochainClient.callZome.mockResolvedValueOnce(mockReport);

      const result = await pullService.pullPatientData(
        'patient-123',
        mockTokenInfo as any,
        {
          resourceTypes: ['Patient', 'Observation', 'Condition', 'MedicationRequest'],
          sourceSystem: 'test-ehr',
        }
      );

      // Verify the bundle was assembled correctly
      expect(result.bundle.resourceType).toBe('Bundle');
      expect(result.bundle.type).toBe('collection');
      expect(result.bundle.entry).toHaveLength(2); // Patient + 1 Condition

      // Verify ingest was called
      expect(mockHolochainClient.callZome).toHaveBeenCalledTimes(1);
      expect(result.ingestReport).toEqual(mockReport);

      // Verify sync results
      expect(result.syncResults).toHaveLength(4); // One per resource type
      expect(result.syncResults.every(r => r.success)).toBe(true);
    });

    it('handles fetch errors gracefully', async () => {
      const mockReport: IngestReport = {
        source_system: 'test-ehr',
        total_processed: 1,
        patients_created: 1,
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

      const mockPatient: FhirPatient = {
        resourceType: 'Patient',
        id: 'patient-123',
      };

      mockFhirAdapter.getPatient.mockResolvedValueOnce(mockPatient);
      mockFhirAdapter.getPatientObservations.mockRejectedValueOnce(new Error('Network error'));
      mockHolochainClient.callZome.mockResolvedValueOnce(mockReport);

      const result = await pullService.pullPatientData(
        'patient-123',
        mockTokenInfo as any,
        {
          resourceTypes: ['Patient', 'Observation'],
        }
      );

      // Patient should succeed, Observation should fail
      const patientResult = result.syncResults.find(r => r.resourceType === 'Patient');
      const observationResult = result.syncResults.find(r => r.resourceType === 'Observation');

      expect(patientResult?.success).toBe(true);
      expect(observationResult?.success).toBe(false);
      expect(observationResult?.errors).toContain('Network error');

      // Bundle should still contain the successful resources
      expect(result.bundle.entry).toHaveLength(1);
    });

    it('uses default resource types when not specified', async () => {
      const mockPatient: FhirPatient = {
        resourceType: 'Patient',
        id: 'patient-123',
      };

      const emptyBundle: FhirBundle = {
        resourceType: 'Bundle',
        type: 'searchset',
        entry: [],
      };

      const mockReport: IngestReport = {
        source_system: 'test-ehr',
        total_processed: 1,
        patients_created: 1,
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

      mockFhirAdapter.getPatient.mockResolvedValueOnce(mockPatient);
      mockFhirAdapter.getPatientObservations.mockResolvedValueOnce(emptyBundle);
      mockFhirAdapter.getPatientConditions.mockResolvedValueOnce(emptyBundle);
      mockFhirAdapter.getPatientMedications.mockResolvedValueOnce(emptyBundle);
      mockHolochainClient.callZome.mockResolvedValueOnce(mockReport);

      const result = await pullService.pullPatientData(
        'patient-123',
        mockTokenInfo as any
      );

      // Should attempt all 7 default resource types
      expect(result.syncResults.length).toBeGreaterThanOrEqual(4);
    });
  });

  describe('emptyIngestReport', () => {
    it('creates empty report with correct structure', () => {
      const report = PullService.emptyIngestReport('test-system');

      expect(report.source_system).toBe('test-system');
      expect(report.total_processed).toBe(0);
      expect(report.patients_created).toBe(0);
      expect(report.unknown_types).toEqual([]);
      expect(report.parse_errors).toEqual([]);
    });
  });

  describe('getSummary', () => {
    it('correctly summarizes sync results', async () => {
      const mockPatient: FhirPatient = {
        resourceType: 'Patient',
        id: 'patient-123',
      };

      const mockReport: IngestReport = {
        source_system: 'test-ehr',
        total_processed: 1,
        patients_created: 1,
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

      mockFhirAdapter.getPatient.mockResolvedValueOnce(mockPatient);
      mockFhirAdapter.getPatientObservations.mockRejectedValueOnce(new Error('Failed'));
      mockHolochainClient.callZome.mockResolvedValueOnce(mockReport);

      await pullService.pullPatientData(
        'patient-123',
        mockTokenInfo as any,
        {
          resourceTypes: ['Patient', 'Observation'],
        }
      );

      const summary = pullService.getSummary();

      expect(summary.total).toBe(2);
      expect(summary.success).toBe(1);
      expect(summary.failed).toBe(1);
      expect(summary.byType.Patient.success).toBe(1);
      expect(summary.byType.Observation.failed).toBe(1);
    });
  });
});
