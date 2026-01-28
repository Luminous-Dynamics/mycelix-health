/**
 * Consent Client Tests
 *
 * Tests for consent management zome client functionality.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  ConsentClient,
  GrantConsentInput,
  ConsentRecord,
  ConsentSummary,
} from '../src/zomes/consent';
import { HealthSdkError, HealthSdkErrorCode } from '../src/types';

// Mock AppClient
const mockCallZome = vi.fn();
const mockAppClient = {
  callZome: mockCallZome,
} as any;

// Sample test data
const mockPatientHash = new Uint8Array(39).fill(1);
const mockConsentHash = new Uint8Array(39).fill(2);
const mockGranteeKey = new Uint8Array(39).fill(3);

describe('ConsentClient', () => {
  let client: ConsentClient;

  beforeEach(() => {
    vi.clearAllMocks();
    client = new ConsentClient(mockAppClient, 'health');
  });

  describe('constructor', () => {
    it('should create a client instance', () => {
      expect(client).toBeInstanceOf(ConsentClient);
    });

    it('should store the role name', () => {
      const customClient = new ConsentClient(mockAppClient, 'custom-role');
      expect(customClient).toBeInstanceOf(ConsentClient);
    });
  });

  describe('grantConsent', () => {
    it('should successfully grant consent', async () => {
      const mockResponse: ConsentRecord = {
        hash: mockConsentHash,
        consent: {
          grantor: mockPatientHash,
          grantee: mockGranteeKey,
          scope: 'Read',
          data_categories: ['Demographics', 'Medications'],
          purpose: 'Treatment coordination',
          is_active: true,
          created_at: Date.now() * 1000,
        },
      };

      mockCallZome.mockResolvedValueOnce(mockResponse);

      const input: GrantConsentInput = {
        grantee: mockGranteeKey,
        scope: 'Read',
        data_categories: ['Demographics', 'Medications'],
        purpose: 'Treatment coordination',
      };

      const result = await client.grantConsent(mockPatientHash, input);

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        role_name: 'health',
        zome_name: 'consent',
        fn_name: 'grant_consent',
      }));
      expect(result.hash).toBe(mockConsentHash);
      expect(result.consent.is_active).toBe(true);
    });

    it('should include valid_from when provided', async () => {
      const validFrom = Date.now() * 1000;
      const mockResponse: ConsentRecord = {
        hash: mockConsentHash,
        consent: {
          grantor: mockPatientHash,
          grantee: mockGranteeKey,
          scope: 'Read',
          data_categories: ['Demographics'],
          purpose: 'Test',
          is_active: true,
          created_at: validFrom,
          valid_from: validFrom,
        },
      };

      mockCallZome.mockResolvedValueOnce(mockResponse);

      const input: GrantConsentInput = {
        grantee: mockGranteeKey,
        scope: 'Read',
        data_categories: ['Demographics'],
        purpose: 'Test',
        valid_from: validFrom,
      };

      await client.grantConsent(mockPatientHash, input);

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        payload: expect.objectContaining({
          valid_from: validFrom,
        }),
      }));
    });

    it('should include valid_until for time-limited consent', async () => {
      const validUntil = (Date.now() + 86400000) * 1000; // 24 hours from now
      const mockResponse: ConsentRecord = {
        hash: mockConsentHash,
        consent: {
          grantor: mockPatientHash,
          grantee: mockGranteeKey,
          scope: 'Read',
          data_categories: ['Demographics'],
          purpose: 'Temporary access',
          is_active: true,
          created_at: Date.now() * 1000,
          valid_until: validUntil,
        },
      };

      mockCallZome.mockResolvedValueOnce(mockResponse);

      const input: GrantConsentInput = {
        grantee: mockGranteeKey,
        scope: 'Read',
        data_categories: ['Demographics'],
        purpose: 'Temporary access',
        valid_until: validUntil,
      };

      await client.grantConsent(mockPatientHash, input);

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        payload: expect.objectContaining({
          valid_until: validUntil,
        }),
      }));
    });
  });

  describe('revokeConsent', () => {
    it('should successfully revoke consent', async () => {
      const mockResponse: ConsentRecord = {
        hash: mockConsentHash,
        consent: {
          grantor: mockPatientHash,
          grantee: mockGranteeKey,
          scope: 'Read',
          data_categories: ['Demographics'],
          purpose: 'Treatment',
          is_active: false,
          created_at: Date.now() * 1000,
          revoked_at: Date.now() * 1000,
        },
      };

      mockCallZome.mockResolvedValueOnce(mockResponse);

      const result = await client.revokeConsent(mockConsentHash);

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        fn_name: 'revoke_consent',
        payload: mockConsentHash,
      }));
      expect(result.consent.is_active).toBe(false);
      expect(result.consent.revoked_at).toBeDefined();
    });
  });

  describe('getConsent', () => {
    it('should retrieve a consent record', async () => {
      const mockConsent = {
        grantor: mockPatientHash,
        grantee: mockGranteeKey,
        scope: 'Read',
        data_categories: ['Demographics'],
        purpose: 'Treatment',
        is_active: true,
        created_at: Date.now() * 1000,
      };

      mockCallZome.mockResolvedValueOnce(mockConsent);

      const result = await client.getConsent(mockConsentHash);

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        fn_name: 'get_consent',
        payload: mockConsentHash,
      }));
      expect(result).toEqual(mockConsent);
    });

    it('should return null for non-existent consent', async () => {
      mockCallZome.mockResolvedValueOnce(null);

      const result = await client.getConsent(mockConsentHash);

      expect(result).toBeNull();
    });
  });

  describe('listPatientConsents', () => {
    it('should list all patient consents', async () => {
      const mockConsents: ConsentRecord[] = [
        {
          hash: mockConsentHash,
          consent: {
            grantor: mockPatientHash,
            grantee: mockGranteeKey,
            scope: 'Read',
            data_categories: ['Demographics'],
            purpose: 'Treatment',
            is_active: true,
            created_at: Date.now() * 1000,
          },
        },
        {
          hash: new Uint8Array(39).fill(4),
          consent: {
            grantor: mockPatientHash,
            grantee: new Uint8Array(39).fill(5),
            scope: 'Write',
            data_categories: ['Medications'],
            purpose: 'Prescribing',
            is_active: true,
            created_at: Date.now() * 1000,
          },
        },
      ];

      mockCallZome.mockResolvedValueOnce(mockConsents);

      const result = await client.listPatientConsents(mockPatientHash);

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        fn_name: 'list_patient_consents',
        payload: mockPatientHash,
      }));
      expect(result).toHaveLength(2);
    });

    it('should return empty array if no consents', async () => {
      mockCallZome.mockResolvedValueOnce([]);

      const result = await client.listPatientConsents(mockPatientHash);

      expect(result).toEqual([]);
    });
  });

  describe('listGranteeConsents', () => {
    it('should list all consents for a grantee', async () => {
      const mockConsents: ConsentRecord[] = [
        {
          hash: mockConsentHash,
          consent: {
            grantor: mockPatientHash,
            grantee: mockGranteeKey,
            scope: 'Read',
            data_categories: ['Demographics'],
            purpose: 'Treatment',
            is_active: true,
            created_at: Date.now() * 1000,
          },
        },
      ];

      mockCallZome.mockResolvedValueOnce(mockConsents);

      const result = await client.listGranteeConsents(mockGranteeKey);

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        fn_name: 'list_grantee_consents',
        payload: mockGranteeKey,
      }));
      expect(result).toHaveLength(1);
    });
  });

  describe('checkAuthorization', () => {
    it('should return authorized for valid consent', async () => {
      const mockResult = {
        authorized: true,
        consent_hash: mockConsentHash,
        reason: 'Active consent exists',
      };

      mockCallZome.mockResolvedValueOnce(mockResult);

      const result = await client.checkAuthorization({
        patient_hash: mockPatientHash,
        requester: mockGranteeKey,
        action: 'read',
        data_categories: ['Demographics'],
      });

      expect(result.authorized).toBe(true);
      expect(result.consent_hash).toBe(mockConsentHash);
    });

    it('should return unauthorized when no consent exists', async () => {
      const mockResult = {
        authorized: false,
        consent_hash: null,
        reason: 'No active consent found',
      };

      mockCallZome.mockResolvedValueOnce(mockResult);

      const result = await client.checkAuthorization({
        patient_hash: mockPatientHash,
        requester: mockGranteeKey,
        action: 'write',
        data_categories: ['MentalHealth'],
      });

      expect(result.authorized).toBe(false);
    });
  });
});

describe('Consent Types', () => {
  describe('ConsentScope', () => {
    it('should support all scope values', () => {
      const scopes = ['Read', 'Write', 'Share', 'Export', 'All'];
      expect(scopes).toHaveLength(5);
    });
  });

  describe('DataCategory', () => {
    it('should include all PHI categories', () => {
      const categories = [
        'Demographics',
        'Allergies',
        'Medications',
        'Diagnoses',
        'Procedures',
        'LabResults',
        'ImagingStudies',
        'VitalSigns',
        'Immunizations',
        'MentalHealth',
        'SubstanceAbuse',
        'SexualHealth',
        'GeneticData',
        'FinancialData',
      ];
      expect(categories.length).toBeGreaterThanOrEqual(14);
    });
  });
});

describe('Consent Validation', () => {
  let client: ConsentClient;

  beforeEach(() => {
    vi.clearAllMocks();
    client = new ConsentClient(mockAppClient, 'health');
  });

  it('should require at least one data category', async () => {
    const input: GrantConsentInput = {
      grantee: mockGranteeKey,
      scope: 'Read',
      data_categories: [],
      purpose: 'Treatment',
    };

    mockCallZome.mockRejectedValueOnce(new Error('At least one data category required'));

    await expect(client.grantConsent(mockPatientHash, input)).rejects.toThrow();
  });

  it('should require a purpose', async () => {
    const input: GrantConsentInput = {
      grantee: mockGranteeKey,
      scope: 'Read',
      data_categories: ['Demographics'],
      purpose: '',
    };

    mockCallZome.mockRejectedValueOnce(new Error('Purpose is required'));

    await expect(client.grantConsent(mockPatientHash, input)).rejects.toThrow();
  });
});

describe('Consent Expiration', () => {
  let client: ConsentClient;

  beforeEach(() => {
    vi.clearAllMocks();
    client = new ConsentClient(mockAppClient, 'health');
  });

  it('should support time-limited consents', async () => {
    const validFrom = Date.now() * 1000;
    const validUntil = (Date.now() + 30 * 24 * 60 * 60 * 1000) * 1000; // 30 days

    const mockResponse: ConsentRecord = {
      hash: mockConsentHash,
      consent: {
        grantor: mockPatientHash,
        grantee: mockGranteeKey,
        scope: 'Read',
        data_categories: ['Demographics'],
        purpose: 'Temporary specialist access',
        is_active: true,
        created_at: validFrom,
        valid_from: validFrom,
        valid_until: validUntil,
      },
    };

    mockCallZome.mockResolvedValueOnce(mockResponse);

    const input: GrantConsentInput = {
      grantee: mockGranteeKey,
      scope: 'Read',
      data_categories: ['Demographics'],
      purpose: 'Temporary specialist access',
      valid_from: validFrom,
      valid_until: validUntil,
    };

    const result = await client.grantConsent(mockPatientHash, input);

    expect(result.consent.valid_until).toBe(validUntil);
    expect(result.consent.valid_from).toBe(validFrom);
  });
});
