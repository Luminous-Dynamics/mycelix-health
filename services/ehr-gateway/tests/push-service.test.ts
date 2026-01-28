/**
 * Push Service Tests
 *
 * Tests for pushing data from Mycelix-Health to external EHR systems.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { PushService, PushConfig, PushOptions } from '../src/sync/push-service.js';
import type { TokenInfo } from '../src/auth/token-manager.js';
import type { AppClient, ActionHash } from '@holochain/client';

// Mock token info
const mockTokenInfo: TokenInfo = {
  accessToken: 'test-access-token',
  tokenType: 'Bearer',
  expiresAt: Date.now() + 3600000, // 1 hour from now
  scope: 'patient/*.read patient/*.write',
  patientId: 'patient-123',
};

// Mock Holochain data
const mockPatientHash = new Uint8Array(39) as ActionHash;
const mockObservationHash = new Uint8Array(39) as ActionHash;
const mockConditionHash = new Uint8Array(39) as ActionHash;
const mockMedicationHash = new Uint8Array(39) as ActionHash;

const mockInternalPatient = {
  action_hash: mockPatientHash,
  first_name: 'John',
  last_name: 'Doe',
  date_of_birth: '1980-05-15',
  gender: 'male',
  identifiers: [
    { system: 'http://hospital.org/mrn', value: 'MRN12345' },
  ],
  contact: {
    phone: '555-123-4567',
    email: 'john.doe@example.com',
    address: {
      line: ['123 Main St'],
      city: 'Springfield',
      state: 'IL',
      postalCode: '62701',
      country: 'US',
    },
  },
};

const mockInternalObservation = {
  action_hash: mockObservationHash,
  patient_hash: mockPatientHash,
  code: '85354-9',
  code_system: 'http://loinc.org',
  display: 'Blood pressure panel',
  value: 120,
  unit: 'mmHg',
  effective_date: '2024-01-15T10:30:00Z',
  status: 'final',
};

const mockInternalCondition = {
  action_hash: mockConditionHash,
  patient_hash: mockPatientHash,
  code: 'E11.9',
  code_system: 'http://hl7.org/fhir/sid/icd-10-cm',
  display: 'Type 2 diabetes mellitus without complications',
  clinical_status: 'active',
  verification_status: 'confirmed',
  onset_date: '2020-03-01',
};

const mockInternalMedication = {
  action_hash: mockMedicationHash,
  patient_hash: mockPatientHash,
  code: '860975',
  code_system: 'http://www.nlm.nih.gov/research/umls/rxnorm',
  display: 'Metformin 500mg tablet',
  status: 'active',
  intent: 'order',
  dosage_text: 'Take 1 tablet by mouth twice daily',
  route: 'oral',
};

const mockFhirMappings = {
  patient: { fhir_id: undefined },
  observations: [],
  conditions: [],
  medications: [],
};

// Mock FHIR adapter
const createMockFhirAdapter = () => ({
  createResource: vi.fn().mockImplementation(async (resource: unknown) => ({
    ...resource,
    id: `fhir-${Date.now()}`,
  })),
  updateResource: vi.fn().mockImplementation(async (resource: unknown) => resource),
  getResource: vi.fn(),
  searchResources: vi.fn(),
  deleteResource: vi.fn(),
});

// Mock Holochain client
const createMockHolochainClient = () => ({
  callZome: vi.fn().mockImplementation(async ({ fn_name, payload }) => {
    switch (fn_name) {
      case 'get_patient_fhir_mappings':
        return mockFhirMappings;
      case 'get_patient':
        return mockInternalPatient;
      case 'get_patient_observations':
        return [mockInternalObservation];
      case 'get_patient_conditions':
        return [mockInternalCondition];
      case 'get_patient_medications':
        return [mockInternalMedication];
      case 'update_fhir_mapping':
        return null;
      default:
        return null;
    }
  }),
});

describe('PushService', () => {
  let pushService: PushService;
  let mockHolochainClient: ReturnType<typeof createMockHolochainClient>;
  let mockFhirAdapter: ReturnType<typeof createMockFhirAdapter>;

  beforeEach(() => {
    mockHolochainClient = createMockHolochainClient();
    mockFhirAdapter = createMockFhirAdapter();

    const config: PushConfig = {
      holochainClient: mockHolochainClient as unknown as AppClient,
      fhirAdapter: mockFhirAdapter as any,
      batchSize: 50,
      validateBeforePush: true,
    };

    pushService = new PushService(config);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('constructor', () => {
    it('should create instance with default config', () => {
      expect(pushService).toBeInstanceOf(PushService);
    });

    it('should override default batchSize', () => {
      const customConfig: PushConfig = {
        holochainClient: mockHolochainClient as unknown as AppClient,
        fhirAdapter: mockFhirAdapter as any,
        batchSize: 100,
      };
      const customService = new PushService(customConfig);
      expect(customService).toBeInstanceOf(PushService);
    });
  });

  describe('pushPatientData', () => {
    it('should push all resource types by default', async () => {
      const results = await pushService.pushPatientData(mockPatientHash, mockTokenInfo);

      // Should have results for Patient, Observation, Condition, MedicationRequest
      expect(results.length).toBe(4);
      expect(results.filter(r => r.success).length).toBe(4);
    });

    it('should push only specified resource types', async () => {
      const options: PushOptions = {
        resourceTypes: ['Patient', 'Condition'],
      };

      const results = await pushService.pushPatientData(mockPatientHash, mockTokenInfo, options);

      expect(results.length).toBe(2);
      expect(results.some(r => r.resourceType === 'Patient')).toBe(true);
      expect(results.some(r => r.resourceType === 'Condition')).toBe(true);
      expect(results.some(r => r.resourceType === 'Observation')).toBe(false);
    });

    it('should handle dry run mode', async () => {
      const options: PushOptions = {
        dryRun: true,
      };

      const results = await pushService.pushPatientData(mockPatientHash, mockTokenInfo, options);

      // Should succeed without actually calling FHIR adapter create/update
      expect(results.filter(r => r.success).length).toBe(4);
      expect(mockFhirAdapter.createResource).not.toHaveBeenCalled();
      expect(mockFhirAdapter.updateResource).not.toHaveBeenCalled();
    });

    it('should call FHIR adapter for new resources', async () => {
      await pushService.pushPatientData(mockPatientHash, mockTokenInfo);

      // Patient should be created (no existing fhir_id)
      expect(mockFhirAdapter.createResource).toHaveBeenCalled();
    });

    it('should update FHIR mappings after successful push', async () => {
      await pushService.pushPatientData(mockPatientHash, mockTokenInfo);

      // Should update mapping for each successfully pushed resource
      const mappingCalls = mockHolochainClient.callZome.mock.calls.filter(
        call => call[0].fn_name === 'update_fhir_mapping'
      );
      expect(mappingCalls.length).toBeGreaterThan(0);
    });

    it('should handle errors gracefully', async () => {
      mockFhirAdapter.createResource.mockRejectedValueOnce(new Error('Network error'));

      const results = await pushService.pushPatientData(mockPatientHash, mockTokenInfo);

      // Should have some failures but not crash
      expect(results.some(r => !r.success)).toBe(true);
      const failedResult = results.find(r => !r.success);
      expect(failedResult?.errors).toContain('Network error');
    });
  });

  describe('getResults', () => {
    it('should return copy of results array', async () => {
      await pushService.pushPatientData(mockPatientHash, mockTokenInfo);

      const results1 = pushService.getResults();
      const results2 = pushService.getResults();

      expect(results1).not.toBe(results2);
      expect(results1).toEqual(results2);
    });
  });

  describe('getSummary', () => {
    it('should return correct summary', async () => {
      await pushService.pushPatientData(mockPatientHash, mockTokenInfo);

      const summary = pushService.getSummary();

      expect(summary.total).toBe(4);
      expect(summary.success).toBe(4);
      expect(summary.failed).toBe(0);
    });

    it('should count failures correctly', async () => {
      mockFhirAdapter.createResource
        .mockRejectedValueOnce(new Error('Error 1'))
        .mockRejectedValueOnce(new Error('Error 2'));

      await pushService.pushPatientData(mockPatientHash, mockTokenInfo);

      const summary = pushService.getSummary();

      expect(summary.failed).toBeGreaterThan(0);
      expect(summary.success + summary.failed).toBe(summary.total);
    });
  });

  describe('FHIR Transformation', () => {
    it('should transform patient to valid FHIR format', async () => {
      await pushService.pushPatientData(mockPatientHash, mockTokenInfo, {
        resourceTypes: ['Patient'],
      });

      const createCall = mockFhirAdapter.createResource.mock.calls[0];
      const fhirPatient = createCall[0];

      expect(fhirPatient.resourceType).toBe('Patient');
      expect(fhirPatient.name[0].family).toBe('Doe');
      expect(fhirPatient.name[0].given).toContain('John');
      expect(fhirPatient.birthDate).toBe('1980-05-15');
      expect(fhirPatient.gender).toBe('male');
    });

    it('should transform observation to valid FHIR format', async () => {
      await pushService.pushPatientData(mockPatientHash, mockTokenInfo, {
        resourceTypes: ['Observation'],
      });

      const createCall = mockFhirAdapter.createResource.mock.calls.find(
        call => call[0].resourceType === 'Observation'
      );
      const fhirObservation = createCall?.[0];

      expect(fhirObservation.resourceType).toBe('Observation');
      expect(fhirObservation.code.coding[0].code).toBe('85354-9');
      expect(fhirObservation.valueQuantity?.value).toBe(120);
    });

    it('should transform condition to valid FHIR format', async () => {
      await pushService.pushPatientData(mockPatientHash, mockTokenInfo, {
        resourceTypes: ['Condition'],
      });

      const createCall = mockFhirAdapter.createResource.mock.calls.find(
        call => call[0].resourceType === 'Condition'
      );
      const fhirCondition = createCall?.[0];

      expect(fhirCondition.resourceType).toBe('Condition');
      expect(fhirCondition.code.coding[0].code).toBe('E11.9');
      expect(fhirCondition.clinicalStatus.coding[0].code).toBe('active');
    });

    it('should transform medication to valid FHIR format', async () => {
      await pushService.pushPatientData(mockPatientHash, mockTokenInfo, {
        resourceTypes: ['MedicationRequest'],
      });

      const createCall = mockFhirAdapter.createResource.mock.calls.find(
        call => call[0].resourceType === 'MedicationRequest'
      );
      const fhirMedication = createCall?.[0];

      expect(fhirMedication.resourceType).toBe('MedicationRequest');
      expect(fhirMedication.medicationCodeableConcept.coding[0].code).toBe('860975');
      expect(fhirMedication.status).toBe('active');
      expect(fhirMedication.intent).toBe('order');
    });
  });

  describe('Update vs Create', () => {
    it('should update existing resources with fhir_id', async () => {
      const mappingsWithExisting = {
        patient: { fhir_id: 'existing-patient-id' },
        observations: [{ internal_hash: mockObservationHash, fhir_id: 'existing-obs-id' }],
        conditions: [],
        medications: [],
      };

      mockHolochainClient.callZome.mockImplementation(async ({ fn_name }) => {
        switch (fn_name) {
          case 'get_patient_fhir_mappings':
            return mappingsWithExisting;
          case 'get_patient':
            return mockInternalPatient;
          case 'get_patient_observations':
            return [mockInternalObservation];
          case 'get_patient_conditions':
            return [];
          case 'get_patient_medications':
            return [];
          default:
            return null;
        }
      });

      await pushService.pushPatientData(mockPatientHash, mockTokenInfo, {
        resourceTypes: ['Patient', 'Observation'],
      });

      // Should update patient (has fhir_id)
      const updateCalls = mockFhirAdapter.updateResource.mock.calls;
      expect(updateCalls.length).toBeGreaterThan(0);

      const patientUpdate = updateCalls.find(call => call[0].resourceType === 'Patient');
      expect(patientUpdate?.[0].id).toBe('existing-patient-id');
    });

    it('should create new resources without fhir_id', async () => {
      await pushService.pushPatientData(mockPatientHash, mockTokenInfo, {
        resourceTypes: ['Patient'],
      });

      expect(mockFhirAdapter.createResource).toHaveBeenCalled();
    });
  });

  describe('Error Handling', () => {
    it('should continue processing after single resource failure', async () => {
      // First call fails, rest succeed
      mockFhirAdapter.createResource
        .mockRejectedValueOnce(new Error('Patient push failed'))
        .mockResolvedValue({ id: 'success-id' });

      const results = await pushService.pushPatientData(mockPatientHash, mockTokenInfo);

      // Should have both success and failure
      const failures = results.filter(r => !r.success);
      const successes = results.filter(r => r.success);

      expect(failures.length).toBeGreaterThan(0);
      expect(successes.length).toBeGreaterThan(0);
    });

    it('should capture error messages in results', async () => {
      mockFhirAdapter.createResource.mockRejectedValue(new Error('Validation failed: missing required field'));

      const results = await pushService.pushPatientData(mockPatientHash, mockTokenInfo, {
        resourceTypes: ['Patient'],
      });

      const failedResult = results.find(r => !r.success);
      expect(failedResult?.errors).toContain('Validation failed: missing required field');
    });

    it('should handle Holochain zome call failures', async () => {
      mockHolochainClient.callZome.mockRejectedValue(new Error('Zome not found'));

      await expect(
        pushService.pushPatientData(mockPatientHash, mockTokenInfo)
      ).rejects.toThrow('Zome not found');
    });
  });

  describe('Result Recording', () => {
    it('should record timestamp for each result', async () => {
      const before = Date.now();
      const results = await pushService.pushPatientData(mockPatientHash, mockTokenInfo);
      const after = Date.now();

      for (const result of results) {
        const timestamp = result.timestamp.getTime();
        expect(timestamp).toBeGreaterThanOrEqual(before);
        expect(timestamp).toBeLessThanOrEqual(after);
      }
    });

    it('should record direction as push for all results', async () => {
      const results = await pushService.pushPatientData(mockPatientHash, mockTokenInfo);

      for (const result of results) {
        expect(result.direction).toBe('push');
      }
    });

    it('should record resource type correctly', async () => {
      const results = await pushService.pushPatientData(mockPatientHash, mockTokenInfo);

      const resourceTypes = results.map(r => r.resourceType);
      expect(resourceTypes).toContain('Patient');
      expect(resourceTypes).toContain('Observation');
      expect(resourceTypes).toContain('Condition');
      expect(resourceTypes).toContain('MedicationRequest');
    });
  });
});

describe('PushService Integration Scenarios', () => {
  let pushService: PushService;
  let mockHolochainClient: ReturnType<typeof createMockHolochainClient>;
  let mockFhirAdapter: ReturnType<typeof createMockFhirAdapter>;

  beforeEach(() => {
    mockHolochainClient = createMockHolochainClient();
    mockFhirAdapter = createMockFhirAdapter();

    pushService = new PushService({
      holochainClient: mockHolochainClient as unknown as AppClient,
      fhirAdapter: mockFhirAdapter as any,
    });
  });

  it('should handle patient with multiple observations', async () => {
    const observations = [
      { ...mockInternalObservation, action_hash: new Uint8Array(39) },
      { ...mockInternalObservation, action_hash: new Uint8Array(39), code: '8310-5', display: 'Body temperature' },
      { ...mockInternalObservation, action_hash: new Uint8Array(39), code: '8867-4', display: 'Heart rate' },
    ];

    mockHolochainClient.callZome.mockImplementation(async ({ fn_name }) => {
      switch (fn_name) {
        case 'get_patient_fhir_mappings':
          return { patient: {}, observations: [], conditions: [], medications: [] };
        case 'get_patient':
          return mockInternalPatient;
        case 'get_patient_observations':
          return observations;
        case 'get_patient_conditions':
          return [];
        case 'get_patient_medications':
          return [];
        default:
          return null;
      }
    });

    const results = await pushService.pushPatientData(mockPatientHash, mockTokenInfo, {
      resourceTypes: ['Observation'],
    });

    // Should have 3 observation results
    expect(results.filter(r => r.resourceType === 'Observation').length).toBe(3);
  });

  it('should handle patient with no clinical data', async () => {
    mockHolochainClient.callZome.mockImplementation(async ({ fn_name }) => {
      switch (fn_name) {
        case 'get_patient_fhir_mappings':
          return { patient: {}, observations: [], conditions: [], medications: [] };
        case 'get_patient':
          return mockInternalPatient;
        case 'get_patient_observations':
          return [];
        case 'get_patient_conditions':
          return [];
        case 'get_patient_medications':
          return [];
        default:
          return null;
      }
    });

    const results = await pushService.pushPatientData(mockPatientHash, mockTokenInfo);

    // Should only have patient result
    expect(results.length).toBe(1);
    expect(results[0].resourceType).toBe('Patient');
  });

  it('should handle partial sync (only new data)', async () => {
    const mappingsWithSome = {
      patient: { fhir_id: 'existing-patient' },
      observations: [{ internal_hash: mockObservationHash, fhir_id: 'existing-obs' }],
      conditions: [], // Condition is new
      medications: [{ internal_hash: mockMedicationHash, fhir_id: 'existing-med' }],
    };

    mockHolochainClient.callZome.mockImplementation(async ({ fn_name }) => {
      switch (fn_name) {
        case 'get_patient_fhir_mappings':
          return mappingsWithSome;
        case 'get_patient':
          return mockInternalPatient;
        case 'get_patient_observations':
          return [mockInternalObservation];
        case 'get_patient_conditions':
          return [mockInternalCondition];
        case 'get_patient_medications':
          return [mockInternalMedication];
        default:
          return null;
      }
    });

    await pushService.pushPatientData(mockPatientHash, mockTokenInfo);

    // Condition should be created (new)
    const createCalls = mockFhirAdapter.createResource.mock.calls;
    const conditionCreate = createCalls.find(call => call[0].resourceType === 'Condition');
    expect(conditionCreate).toBeDefined();

    // Patient, Observation, Medication should be updated (existing)
    const updateCalls = mockFhirAdapter.updateResource.mock.calls;
    expect(updateCalls.length).toBe(3);
  });
});
