/**
 * Zome Connector Tests
 *
 * Tests for the zome connector that bridges EHR Gateway to Holochain zomes.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  ZomeConnector,
  createZomeConnector,
  type ConsentCheckResult,
  type ProviderVerificationResult,
  type PatientLookupResult,
  type AuditLogEntry,
  type DataCategory,
} from '../src/zome-connector.js';

describe('ZomeConnector', () => {
  let mockClient: any;
  let connector: ZomeConnector;

  beforeEach(() => {
    mockClient = {
      callZome: vi.fn(),
    };
    connector = createZomeConnector(mockClient, 'health');
  });

  describe('Consent Verification', () => {
    it('should check consent with correct zome call', async () => {
      const patientHash = new Uint8Array(39) as any;
      const categories: DataCategory[] = [
        { category: 'Demographics', permission: 'Read' },
        { category: 'LabResults', permission: 'Read' },
      ];

      mockClient.callZome.mockResolvedValue({
        authorized: true,
        consentHash: new Uint8Array(39),
        dataCategories: ['Demographics', 'LabResults'],
        emergencyOverride: false,
      });

      const result = await connector.checkConsent(patientHash, categories);

      expect(mockClient.callZome).toHaveBeenCalledWith({
        cap_secret: null,
        role_name: 'health',
        zome_name: 'consent',
        fn_name: 'check_authorization',
        payload: {
          patient_hash: patientHash,
          data_categories: [
            { category: 'Demographics', permission: 'Read' },
            { category: 'LabResults', permission: 'Read' },
          ],
          is_emergency: false,
          emergency_reason: undefined,
        },
      });

      expect(result.authorized).toBe(true);
    });

    it('should handle consent denial', async () => {
      const patientHash = new Uint8Array(39) as any;
      const categories: DataCategory[] = [
        { category: 'Diagnoses', permission: 'Read' },
      ];

      mockClient.callZome.mockResolvedValue({
        authorized: false,
        dataCategories: ['Diagnoses'],
        emergencyOverride: false,
        denialReason: 'No active consent for this category',
      });

      const result = await connector.checkConsent(patientHash, categories);

      expect(result.authorized).toBe(false);
      expect(result.denialReason).toContain('No active consent');
    });

    it('should handle emergency override', async () => {
      const patientHash = new Uint8Array(39) as any;
      const categories: DataCategory[] = [
        { category: 'AllData', permission: 'Read' },
      ];

      mockClient.callZome.mockResolvedValue({
        authorized: true,
        dataCategories: ['AllData'],
        emergencyOverride: true,
      });

      const result = await connector.checkConsent(
        patientHash,
        categories,
        true,
        'Patient unconscious, emergency care required'
      );

      expect(mockClient.callZome).toHaveBeenCalledWith(
        expect.objectContaining({
          payload: expect.objectContaining({
            is_emergency: true,
            emergency_reason: 'Patient unconscious, emergency care required',
          }),
        })
      );

      expect(result.emergencyOverride).toBe(true);
    });

    it('should return denial on zome error', async () => {
      const patientHash = new Uint8Array(39) as any;
      const categories: DataCategory[] = [
        { category: 'Demographics', permission: 'Read' },
      ];

      mockClient.callZome.mockRejectedValue(new Error('Zome unavailable'));

      const result = await connector.checkConsent(patientHash, categories);

      expect(result.authorized).toBe(false);
      expect(result.denialReason).toContain('Consent check failed');
    });

    it('should map FHIR resource types to consent categories for pull', async () => {
      const patientHash = new Uint8Array(39) as any;

      mockClient.callZome.mockResolvedValue({
        authorized: true,
        dataCategories: ['Demographics', 'LabResults', 'Diagnoses'],
        emergencyOverride: false,
      });

      await connector.checkPullConsent(patientHash, [
        'Patient',
        'Observation',
        'Condition',
      ]);

      // Pull = writing to DHT, so permission is 'Write'
      expect(mockClient.callZome).toHaveBeenCalledWith(
        expect.objectContaining({
          payload: expect.objectContaining({
            data_categories: expect.arrayContaining([
              { category: 'Demographics', permission: 'Write' },
              { category: 'LabResults', permission: 'Write' },
              { category: 'Diagnoses', permission: 'Write' },
            ]),
          }),
        })
      );
    });

    it('should map FHIR resource types to consent categories for push', async () => {
      const patientHash = new Uint8Array(39) as any;

      mockClient.callZome.mockResolvedValue({
        authorized: true,
        dataCategories: ['Medications', 'Allergies'],
        emergencyOverride: false,
      });

      await connector.checkPushConsent(patientHash, [
        'MedicationRequest',
        'AllergyIntolerance',
      ]);

      // Push = reading from DHT, so permission is 'Read'
      expect(mockClient.callZome).toHaveBeenCalledWith(
        expect.objectContaining({
          payload: expect.objectContaining({
            data_categories: expect.arrayContaining([
              { category: 'Medications', permission: 'Read' },
              { category: 'Allergies', permission: 'Read' },
            ]),
          }),
        })
      );
    });
  });

  describe('Provider Verification', () => {
    it('should verify provider by NPI', async () => {
      mockClient.callZome
        .mockResolvedValueOnce({
          action_hash: new Uint8Array(39),
          npi: '1234567890',
        })
        .mockResolvedValueOnce([
          {
            licenseType: 'MD',
            licenseNumber: 'MD12345',
            state: 'TX',
            status: 'Active',
          },
        ])
        .mockResolvedValueOnce({ composite_score: 0.92 });

      const result = await connector.verifyProvider('1234567890');

      expect(result.verified).toBe(true);
      expect(result.providerId).toBe('1234567890');
      expect(result.licenses).toHaveLength(1);
      expect(result.trustScore).toBe(0.92);
    });

    it('should reject provider with no active licenses', async () => {
      mockClient.callZome
        .mockResolvedValueOnce({
          action_hash: new Uint8Array(39),
          npi: '1234567890',
        })
        .mockResolvedValueOnce([
          {
            licenseType: 'MD',
            licenseNumber: 'MD12345',
            state: 'TX',
            status: 'Expired',
          },
        ]);

      const result = await connector.verifyProvider('1234567890');

      expect(result.verified).toBe(false);
      expect(result.denialReason).toContain('No active licenses');
    });

    it('should return denial when provider not found', async () => {
      mockClient.callZome.mockResolvedValue(null);

      const result = await connector.verifyProvider('9999999999');

      expect(result.verified).toBe(false);
      expect(result.denialReason).toContain('Provider not found');
    });

    it('should verify current agent provider profile', async () => {
      mockClient.callZome
        .mockResolvedValueOnce({
          action_hash: new Uint8Array(39),
          npi: '5555555555',
        })
        .mockResolvedValueOnce([
          {
            licenseType: 'RN',
            licenseNumber: 'RN98765',
            state: 'CA',
            status: 'Active',
          },
        ]);

      const result = await connector.verifyProvider();

      expect(mockClient.callZome).toHaveBeenCalledWith(
        expect.objectContaining({
          fn_name: 'get_my_provider_profile',
          payload: null,
        })
      );

      expect(result.verified).toBe(true);
    });
  });

  describe('Patient Lookup', () => {
    it('should lookup patient by MRN', async () => {
      mockClient.callZome.mockResolvedValue({
        action_hash: new Uint8Array(39),
        entry: {
          mrn: 'MRN123456',
          external_ids: { epic: 'E123', cerner: 'C456' },
        },
      });

      const result = await connector.lookupPatient('MRN123456');

      expect(mockClient.callZome).toHaveBeenCalledWith(
        expect.objectContaining({
          fn_name: 'get_patient_by_mrn',
          payload: {
            mrn: 'MRN123456',
            is_emergency: false,
          },
        })
      );

      expect(result.found).toBe(true);
      expect(result.mrn).toBe('MRN123456');
      expect(result.externalIds).toEqual({ epic: 'E123', cerner: 'C456' });
    });

    it('should lookup patient by external ID', async () => {
      mockClient.callZome.mockResolvedValue({
        action_hash: new Uint8Array(39),
        entry: {
          mrn: 'MRN789',
          external_ids: { epic: 'E789' },
        },
      });

      const result = await connector.lookupPatient(undefined, 'epic', 'E789');

      expect(mockClient.callZome).toHaveBeenCalledWith(
        expect.objectContaining({
          fn_name: 'get_patient_by_external_id',
          payload: {
            system: 'epic',
            id: 'E789',
          },
        })
      );

      expect(result.found).toBe(true);
    });

    it('should return not found for unknown patient', async () => {
      mockClient.callZome.mockResolvedValue(null);

      const result = await connector.lookupPatient('UNKNOWN');

      expect(result.found).toBe(false);
      expect(result.patientHash).toBeUndefined();
    });

    it('should register external ID for patient', async () => {
      const patientHash = new Uint8Array(39) as any;
      mockClient.callZome.mockResolvedValue(undefined);

      await connector.registerExternalId(patientHash, 'epic', 'E999');

      expect(mockClient.callZome).toHaveBeenCalledWith({
        cap_secret: null,
        role_name: 'health',
        zome_name: 'patient',
        fn_name: 'add_external_id',
        payload: {
          patient_hash: patientHash,
          system: 'epic',
          id: 'E999',
        },
      });
    });
  });

  describe('Audit Logging', () => {
    it('should log sync operation', async () => {
      mockClient.callZome.mockResolvedValue(undefined);

      const entry: AuditLogEntry = {
        action: 'pull',
        patientHash: new Uint8Array(39) as any,
        externalSystem: 'epic',
        resourceType: 'Observation',
        resourceCount: 15,
        success: true,
        timestamp: Date.now(),
      };

      await connector.logSyncOperation(entry);

      expect(mockClient.callZome).toHaveBeenCalledWith(
        expect.objectContaining({
          zome_name: 'consent',
          fn_name: 'create_access_log',
        })
      );
    });

    it('should handle logging failure gracefully', async () => {
      mockClient.callZome.mockRejectedValue(new Error('Log failed'));

      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      const logSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      const entry: AuditLogEntry = {
        action: 'push',
        patientHash: new Uint8Array(39) as any,
        externalSystem: 'cerner',
        resourceType: 'MedicationRequest',
        resourceCount: 3,
        success: false,
        errorMessage: 'Connection timeout',
        timestamp: Date.now(),
      };

      // Should not throw
      await connector.logSyncOperation(entry);

      expect(consoleSpy).toHaveBeenCalled();
      consoleSpy.mockRestore();
      logSpy.mockRestore();
    });

    it('should log consent check', async () => {
      mockClient.callZome.mockResolvedValue(undefined);

      const patientHash = new Uint8Array(39) as any;
      await connector.logConsentCheck(
        patientHash,
        true,
        ['Demographics', 'Medications'],
        'epic'
      );

      expect(mockClient.callZome).toHaveBeenCalledWith(
        expect.objectContaining({
          payload: expect.objectContaining({
            action: 'consent_check',
            patient_hash: patientHash,
            success: true,
          }),
        })
      );
    });
  });

  describe('Data Validation', () => {
    it('should validate FHIR resource for ingestion', async () => {
      mockClient.callZome.mockResolvedValue({
        valid: true,
        errors: [],
      });

      const patientHash = new Uint8Array(39) as any;
      const fhirObservation = {
        resourceType: 'Observation',
        status: 'final',
        code: { coding: [{ system: 'http://loinc.org', code: '12345-6' }] },
      };

      const result = await connector.validateForIngestion(
        patientHash,
        'Observation',
        fhirObservation
      );

      expect(mockClient.callZome).toHaveBeenCalledWith({
        cap_secret: null,
        role_name: 'health',
        zome_name: 'fhir_bridge',
        fn_name: 'validate_fhir_resource',
        payload: {
          patient_hash: patientHash,
          resource_type: 'Observation',
          data: fhirObservation,
        },
      });

      expect(result.valid).toBe(true);
    });

    it('should return validation errors', async () => {
      mockClient.callZome.mockResolvedValue({
        valid: false,
        errors: ['Missing required field: status', 'Invalid code system'],
      });

      const patientHash = new Uint8Array(39) as any;
      const result = await connector.validateForIngestion(
        patientHash,
        'Observation',
        {}
      );

      expect(result.valid).toBe(false);
      expect(result.errors).toHaveLength(2);
    });
  });

  describe('Combined Operations', () => {
    it('should prepare for pull with all checks passing', async () => {
      // Provider verification
      mockClient.callZome
        .mockResolvedValueOnce({ action_hash: new Uint8Array(39), npi: '1234567890' })
        .mockResolvedValueOnce([{ licenseType: 'MD', status: 'Active' }])
        // Patient lookup
        .mockResolvedValueOnce({
          action_hash: new Uint8Array(39),
          entry: { mrn: 'MRN123' },
        })
        // Consent check
        .mockResolvedValueOnce({
          authorized: true,
          dataCategories: ['Demographics'],
          emergencyOverride: false,
        })
        // Log consent check
        .mockResolvedValueOnce(undefined);

      const result = await connector.prepareForPull(
        'MRN123',
        'epic',
        ['Patient']
      );

      expect(result.authorized).toBe(true);
      expect(result.patientHash).toBeDefined();
      expect(result.errors).toHaveLength(0);
    });

    it('should fail pull preparation when provider not verified', async () => {
      mockClient.callZome.mockResolvedValue(null);

      const result = await connector.prepareForPull(
        'MRN123',
        'epic',
        ['Patient']
      );

      expect(result.authorized).toBe(false);
      expect(result.errors).toContain('Provider not found in system');
    });

    it('should fail pull preparation when patient not found', async () => {
      // Provider verified
      mockClient.callZome
        .mockResolvedValueOnce({ action_hash: new Uint8Array(39), npi: '123' })
        .mockResolvedValueOnce([{ status: 'Active' }])
        // Patient not found
        .mockResolvedValueOnce(null);

      const result = await connector.prepareForPull(
        'UNKNOWN',
        'epic',
        ['Patient']
      );

      expect(result.authorized).toBe(false);
      expect(result.errors).toContain('Patient not found');
    });

    it('should fail pull preparation when consent denied', async () => {
      // Provider verified
      mockClient.callZome
        .mockResolvedValueOnce({ action_hash: new Uint8Array(39), npi: '123' })
        .mockResolvedValueOnce([{ status: 'Active' }])
        // Patient found
        .mockResolvedValueOnce({
          action_hash: new Uint8Array(39),
          entry: { mrn: 'MRN123' },
        })
        // Consent denied
        .mockResolvedValueOnce({
          authorized: false,
          dataCategories: ['Demographics'],
          emergencyOverride: false,
          denialReason: 'No consent on file',
        })
        // Log denial
        .mockResolvedValueOnce(undefined);

      const result = await connector.prepareForPull(
        'MRN123',
        'epic',
        ['Patient']
      );

      expect(result.authorized).toBe(false);
      expect(result.consentResult?.authorized).toBe(false);
    });

    it('should prepare for push with all checks passing', async () => {
      const patientHash = new Uint8Array(39) as any;

      // Provider verification
      mockClient.callZome
        .mockResolvedValueOnce({ action_hash: new Uint8Array(39), npi: '123' })
        .mockResolvedValueOnce([{ status: 'Active' }])
        // Consent check
        .mockResolvedValueOnce({
          authorized: true,
          dataCategories: ['Medications'],
          emergencyOverride: false,
        })
        // Log consent check
        .mockResolvedValueOnce(undefined);

      const result = await connector.prepareForPush(
        patientHash,
        'cerner',
        ['MedicationRequest']
      );

      expect(result.authorized).toBe(true);
      expect(result.errors).toHaveLength(0);
    });
  });

  describe('Factory Function', () => {
    it('should create ZomeConnector with default role', () => {
      const conn = createZomeConnector(mockClient);
      expect(conn).toBeInstanceOf(ZomeConnector);
    });

    it('should create ZomeConnector with custom role', () => {
      const conn = createZomeConnector(mockClient, 'custom_health');
      expect(conn).toBeInstanceOf(ZomeConnector);
    });
  });
});
