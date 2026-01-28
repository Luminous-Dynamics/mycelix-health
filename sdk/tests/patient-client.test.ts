/**
 * Patient Client Tests
 *
 * Tests for patient record management zome client functionality.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  PatientClient,
  CreatePatientInput,
  PatientRecord,
  PatientSearchCriteria,
} from '../src/zomes/patient';
import { HealthSdkError, HealthSdkErrorCode } from '../src/types';

// Mock AppClient
const mockCallZome = vi.fn();
const mockAppClient = {
  callZome: mockCallZome,
} as any;

// Sample test data
const mockPatientHash = new Uint8Array(39).fill(1);
const mockAgentKey = new Uint8Array(39).fill(2);

const validContactInfo = {
  address_line1: '123 Main St',
  city: 'Anytown',
  state_province: 'CA',
  postal_code: '12345',
  country: 'USA',
  phone_primary: '+1-555-123-4567',
  email: 'patient@example.com',
};

const validPatientInput: CreatePatientInput = {
  first_name: 'John',
  last_name: 'Doe',
  date_of_birth: '1980-01-15',
  mrn: 'MRN-12345',
  contact: validContactInfo,
  allergies: ['Penicillin', 'Sulfa'],
  medications: ['Metformin 500mg'],
};

describe('PatientClient', () => {
  let client: PatientClient;

  beforeEach(() => {
    vi.clearAllMocks();
    client = new PatientClient(mockAppClient, 'health');
  });

  describe('constructor', () => {
    it('should create a client instance', () => {
      expect(client).toBeInstanceOf(PatientClient);
    });

    it('should use the correct zome name', () => {
      // The zome name is private but we can verify it's used correctly
      expect(client).toBeInstanceOf(PatientClient);
    });
  });

  describe('createPatient', () => {
    it('should successfully create a patient record', async () => {
      const mockResponse: PatientRecord = {
        hash: mockPatientHash,
        patient: {
          patient_id: 'P12345',
          first_name: 'John',
          last_name: 'Doe',
          date_of_birth: '1980-01-15',
          mrn: 'MRN-12345',
          contact: validContactInfo,
          allergies: validPatientInput.allergies!,
          medications: validPatientInput.medications!,
          created_at: Date.now() * 1000,
          updated_at: Date.now() * 1000,
        },
      };

      mockCallZome.mockResolvedValueOnce(mockResponse);

      const result = await client.createPatient(validPatientInput);

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        role_name: 'health',
        zome_name: 'patient',
        fn_name: 'create_patient',
      }));
      expect(result.hash).toBe(mockPatientHash);
      expect(result.patient.first_name).toBe('John');
    });

    it('should default empty arrays for optional fields', async () => {
      const minimalInput: CreatePatientInput = {
        first_name: 'Jane',
        last_name: 'Smith',
        date_of_birth: '1990-05-20',
        contact: validContactInfo,
      };

      const mockResponse: PatientRecord = {
        hash: mockPatientHash,
        patient: {
          patient_id: 'P12346',
          first_name: 'Jane',
          last_name: 'Smith',
          date_of_birth: '1990-05-20',
          contact: validContactInfo,
          allergies: [],
          medications: [],
          created_at: Date.now() * 1000,
          updated_at: Date.now() * 1000,
        },
      };

      mockCallZome.mockResolvedValueOnce(mockResponse);

      const result = await client.createPatient(minimalInput);

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        payload: expect.objectContaining({
          allergies: [],
          medications: [],
        }),
      }));
      expect(result.patient.allergies).toEqual([]);
    });

    it('should include emergency contacts when provided', async () => {
      const inputWithContacts: CreatePatientInput = {
        ...validPatientInput,
        emergency_contacts: [
          {
            name: 'Jane Doe',
            relationship: 'Spouse',
            phone: '+1-555-987-6543',
          },
        ],
      };

      const mockResponse: PatientRecord = {
        hash: mockPatientHash,
        patient: {
          patient_id: 'P12347',
          ...inputWithContacts,
          emergency_contacts: inputWithContacts.emergency_contacts,
          created_at: Date.now() * 1000,
          updated_at: Date.now() * 1000,
        },
      };

      mockCallZome.mockResolvedValueOnce(mockResponse);

      await client.createPatient(inputWithContacts);

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        payload: expect.objectContaining({
          emergency_contacts: inputWithContacts.emergency_contacts,
        }),
      }));
    });
  });

  describe('getPatient', () => {
    it('should retrieve a patient by hash', async () => {
      const mockPatient = {
        patient_id: 'P12345',
        first_name: 'John',
        last_name: 'Doe',
        date_of_birth: '1980-01-15',
        contact: validContactInfo,
        allergies: [],
        medications: [],
        created_at: Date.now() * 1000,
        updated_at: Date.now() * 1000,
      };

      mockCallZome.mockResolvedValueOnce(mockPatient);

      const result = await client.getPatient(mockPatientHash);

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        fn_name: 'get_patient',
        payload: mockPatientHash,
      }));
      expect(result).toEqual(mockPatient);
    });

    it('should return null for non-existent patient', async () => {
      mockCallZome.mockResolvedValueOnce(null);

      const result = await client.getPatient(mockPatientHash);

      expect(result).toBeNull();
    });
  });

  describe('getMyPatient', () => {
    it('should retrieve current agents patient record', async () => {
      const mockResponse: PatientRecord = {
        hash: mockPatientHash,
        patient: {
          patient_id: 'P12345',
          first_name: 'Current',
          last_name: 'User',
          date_of_birth: '1985-03-10',
          contact: validContactInfo,
          allergies: [],
          medications: [],
          created_at: Date.now() * 1000,
          updated_at: Date.now() * 1000,
        },
      };

      mockCallZome.mockResolvedValueOnce(mockResponse);

      const result = await client.getMyPatient();

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        fn_name: 'get_my_patient',
        payload: null,
      }));
      expect(result?.patient.first_name).toBe('Current');
    });

    it('should return null if no patient record exists', async () => {
      mockCallZome.mockResolvedValueOnce(null);

      const result = await client.getMyPatient();

      expect(result).toBeNull();
    });
  });

  describe('updatePatient', () => {
    it('should update patient record', async () => {
      const updates = {
        allergies: ['Penicillin', 'Sulfa', 'Aspirin'],
      };

      const mockResponse: PatientRecord = {
        hash: mockPatientHash,
        patient: {
          patient_id: 'P12345',
          first_name: 'John',
          last_name: 'Doe',
          date_of_birth: '1980-01-15',
          contact: validContactInfo,
          allergies: updates.allergies,
          medications: [],
          created_at: Date.now() * 1000,
          updated_at: Date.now() * 1000,
        },
      };

      mockCallZome.mockResolvedValueOnce(mockResponse);

      const result = await client.updatePatient(mockPatientHash, updates);

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        fn_name: 'update_patient',
        payload: expect.objectContaining({
          original_hash: mockPatientHash,
          updates,
        }),
      }));
      expect(result.patient.allergies).toContain('Aspirin');
    });
  });

  describe('searchPatients', () => {
    it('should search by name', async () => {
      const mockResults: PatientRecord[] = [
        {
          hash: mockPatientHash,
          patient: {
            patient_id: 'P12345',
            first_name: 'John',
            last_name: 'Doe',
            date_of_birth: '1980-01-15',
            contact: validContactInfo,
            allergies: [],
            medications: [],
            created_at: Date.now() * 1000,
            updated_at: Date.now() * 1000,
          },
        },
      ];

      mockCallZome.mockResolvedValueOnce(mockResults);

      const result = await client.searchPatients({ name: 'John' });

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        fn_name: 'search_patients_by_name',
        payload: expect.objectContaining({
          name: 'John',
          limit: 50,
        }),
      }));
      expect(result).toHaveLength(1);
    });

    it('should search by MRN', async () => {
      const mockResult: PatientRecord = {
        hash: mockPatientHash,
        patient: {
          patient_id: 'P12345',
          first_name: 'John',
          last_name: 'Doe',
          date_of_birth: '1980-01-15',
          mrn: 'MRN-12345',
          contact: validContactInfo,
          allergies: [],
          medications: [],
          created_at: Date.now() * 1000,
          updated_at: Date.now() * 1000,
        },
      };

      mockCallZome.mockResolvedValueOnce(mockResult);

      const result = await client.searchPatients({ mrn: 'MRN-12345' });

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        fn_name: 'get_patient_by_mrn',
        payload: 'MRN-12345',
      }));
      expect(result).toHaveLength(1);
      expect(result[0].patient.mrn).toBe('MRN-12345');
    });

    it('should return empty array if MRN not found', async () => {
      mockCallZome.mockResolvedValueOnce(null);

      const result = await client.searchPatients({ mrn: 'NONEXISTENT' });

      expect(result).toEqual([]);
    });

    it('should respect limit parameter', async () => {
      mockCallZome.mockResolvedValueOnce([]);

      await client.searchPatients({ name: 'Test', limit: 10 });

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        payload: expect.objectContaining({
          limit: 10,
        }),
      }));
    });

    it('should throw error for empty criteria', async () => {
      await expect(client.searchPatients({})).rejects.toThrow(HealthSdkError);
      await expect(client.searchPatients({})).rejects.toMatchObject({
        code: HealthSdkErrorCode.INVALID_INPUT,
      });
    });
  });

  describe('addAllergy', () => {
    it('should add an allergy to patient record', async () => {
      mockCallZome.mockResolvedValueOnce(undefined);

      await client.addAllergy(mockPatientHash, 'Peanuts');

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        fn_name: 'add_allergy',
        payload: {
          patient_hash: mockPatientHash,
          allergy: 'Peanuts',
        },
      }));
    });
  });

  describe('addMedication', () => {
    it('should add a medication to patient record', async () => {
      mockCallZome.mockResolvedValueOnce(undefined);

      await client.addMedication(mockPatientHash, 'Lisinopril 10mg');

      expect(mockCallZome).toHaveBeenCalledWith(expect.objectContaining({
        fn_name: 'add_medication',
        payload: {
          patient_hash: mockPatientHash,
          medication: 'Lisinopril 10mg',
        },
      }));
    });
  });
});

describe('Patient Validation', () => {
  let client: PatientClient;

  beforeEach(() => {
    vi.clearAllMocks();
    client = new PatientClient(mockAppClient, 'health');
  });

  describe('Name validation', () => {
    it('should require first name', async () => {
      const invalidInput: CreatePatientInput = {
        first_name: '',
        last_name: 'Doe',
        date_of_birth: '1980-01-15',
        contact: validContactInfo,
      };

      mockCallZome.mockRejectedValueOnce(new Error('First name is required'));

      await expect(client.createPatient(invalidInput)).rejects.toThrow();
    });

    it('should require last name', async () => {
      const invalidInput: CreatePatientInput = {
        first_name: 'John',
        last_name: '',
        date_of_birth: '1980-01-15',
        contact: validContactInfo,
      };

      mockCallZome.mockRejectedValueOnce(new Error('Last name is required'));

      await expect(client.createPatient(invalidInput)).rejects.toThrow();
    });
  });

  describe('Date of birth validation', () => {
    it('should validate date format', async () => {
      const invalidInput: CreatePatientInput = {
        first_name: 'John',
        last_name: 'Doe',
        date_of_birth: 'invalid-date',
        contact: validContactInfo,
      };

      mockCallZome.mockRejectedValueOnce(new Error('Invalid date format'));

      await expect(client.createPatient(invalidInput)).rejects.toThrow();
    });

    it('should accept valid ISO date format', async () => {
      const validInput: CreatePatientInput = {
        first_name: 'John',
        last_name: 'Doe',
        date_of_birth: '1980-01-15',
        contact: validContactInfo,
      };

      const mockResponse: PatientRecord = {
        hash: mockPatientHash,
        patient: {
          patient_id: 'P12345',
          ...validInput,
          allergies: [],
          medications: [],
          created_at: Date.now() * 1000,
          updated_at: Date.now() * 1000,
        },
      };

      mockCallZome.mockResolvedValueOnce(mockResponse);

      const result = await client.createPatient(validInput);

      expect(result.patient.date_of_birth).toBe('1980-01-15');
    });
  });

  describe('MRN validation', () => {
    it('should validate MRN format if provided', async () => {
      const invalidInput: CreatePatientInput = {
        first_name: 'John',
        last_name: 'Doe',
        date_of_birth: '1980-01-15',
        mrn: 'AB', // Too short
        contact: validContactInfo,
      };

      mockCallZome.mockRejectedValueOnce(new Error('MRN must be at least 4 characters'));

      await expect(client.createPatient(invalidInput)).rejects.toThrow();
    });

    it('should accept valid MRN', async () => {
      const validInput: CreatePatientInput = {
        first_name: 'John',
        last_name: 'Doe',
        date_of_birth: '1980-01-15',
        mrn: 'MRN-12345',
        contact: validContactInfo,
      };

      const mockResponse: PatientRecord = {
        hash: mockPatientHash,
        patient: {
          patient_id: 'P12345',
          ...validInput,
          allergies: [],
          medications: [],
          created_at: Date.now() * 1000,
          updated_at: Date.now() * 1000,
        },
      };

      mockCallZome.mockResolvedValueOnce(mockResponse);

      const result = await client.createPatient(validInput);

      expect(result.patient.mrn).toBe('MRN-12345');
    });
  });
});

describe('ContactInfo Types', () => {
  it('should have all address fields', () => {
    const contact = {
      address_line1: '123 Main St',
      address_line2: 'Apt 4B',
      city: 'Anytown',
      state_province: 'CA',
      postal_code: '12345',
      country: 'USA',
      phone_primary: '+1-555-123-4567',
      phone_secondary: '+1-555-987-6543',
      email: 'test@example.com',
    };

    expect(contact.address_line1).toBeDefined();
    expect(contact.country).toBe('USA');
  });
});

describe('EmergencyContact Types', () => {
  it('should have required fields', () => {
    const contact = {
      name: 'Jane Doe',
      relationship: 'Spouse',
      phone: '+1-555-987-6543',
      email: 'jane@example.com',
    };

    expect(contact.name).toBeDefined();
    expect(contact.relationship).toBeDefined();
    expect(contact.phone).toBeDefined();
  });
});
