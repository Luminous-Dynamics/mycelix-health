/**
 * CDS Client Tests
 *
 * Tests for Clinical Decision Support zome client functionality.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  CdsClient,
  InteractionSeverity,
  AlertType,
  AlertPriority,
  GuidelineCategory,
  EvidenceLevel,
  DrugInteraction,
  DrugAllergyInteraction,
  ClinicalAlert,
  ClinicalGuideline,
  InteractionCheckResponse,
} from '../src/zomes/cds';
import { HealthSdkError, HealthSdkErrorCode } from '../src/types';

// Mock AppClient
const mockCallZome = vi.fn();
const mockAppClient = {
  callZome: mockCallZome,
} as any;

describe('CDS Types', () => {
  describe('InteractionSeverity', () => {
    it('should have correct severity values', () => {
      const severities: InteractionSeverity[] = [
        'Contraindicated',
        'Major',
        'Moderate',
        'Minor',
        'Unknown',
      ];
      expect(severities).toHaveLength(5);
    });
  });

  describe('AlertType', () => {
    it('should have all alert types', () => {
      const types: AlertType[] = [
        'DrugInteraction',
        'AllergyAlert',
        'DoseWarning',
        'LabResult',
        'Preventive',
        'Diagnostic',
        'Custom',
      ];
      expect(types).toHaveLength(7);
    });
  });

  describe('AlertPriority', () => {
    it('should have correct priority levels', () => {
      const priorities: AlertPriority[] = ['Critical', 'High', 'Medium', 'Low', 'Info'];
      expect(priorities).toHaveLength(5);
    });
  });

  describe('GuidelineCategory', () => {
    it('should have all categories', () => {
      const categories: GuidelineCategory[] = [
        'Screening',
        'Prevention',
        'Diagnosis',
        'Treatment',
        'Monitoring',
        'Referral',
      ];
      expect(categories).toHaveLength(6);
    });
  });

  describe('EvidenceLevel', () => {
    it('should have all evidence levels', () => {
      const levels: EvidenceLevel[] = ['A', 'B', 'C', 'D', 'Expert'];
      expect(levels).toHaveLength(5);
    });
  });
});

describe('DrugInteraction interface', () => {
  it('should create valid drug interaction', () => {
    const interaction: DrugInteraction = {
      drug_a_rxnorm: '197381',
      drug_a_name: 'Warfarin',
      drug_b_rxnorm: '161',
      drug_b_name: 'Aspirin',
      severity: 'Major',
      description: 'Increased bleeding risk',
      mechanism: 'Both affect platelet function',
      management: 'Monitor INR, consider alternative',
      references: ['PMID:12345678'],
    };

    expect(interaction.drug_a_rxnorm).toBe('197381');
    expect(interaction.severity).toBe('Major');
    expect(interaction.references).toHaveLength(1);
  });

  it('should allow optional mechanism field', () => {
    const interaction: DrugInteraction = {
      drug_a_rxnorm: '197381',
      drug_a_name: 'Drug A',
      drug_b_rxnorm: '123456',
      drug_b_name: 'Drug B',
      severity: 'Minor',
      description: 'Minor interaction',
      management: 'No action needed',
      references: [],
    };

    expect(interaction.mechanism).toBeUndefined();
  });
});

describe('DrugAllergyInteraction interface', () => {
  it('should create valid allergy interaction', () => {
    const allergyInteraction: DrugAllergyInteraction = {
      drug_rxnorm: '723',
      drug_name: 'Amoxicillin',
      allergen: 'Penicillin',
      cross_reactivity_risk: 'Major',
      description: 'Cross-reactivity with penicillin allergy',
      alternatives: ['Azithromycin', 'Fluoroquinolones'],
    };

    expect(allergyInteraction.drug_name).toBe('Amoxicillin');
    expect(allergyInteraction.cross_reactivity_risk).toBe('Major');
    expect(allergyInteraction.alternatives).toContain('Azithromycin');
  });
});

describe('ClinicalAlert interface', () => {
  it('should create valid clinical alert', () => {
    const alert: ClinicalAlert = {
      patient_hash: new Uint8Array(32),
      alert_type: 'DrugInteraction',
      priority: 'High',
      title: 'Drug Interaction Warning',
      message: 'Warfarin-Aspirin interaction detected',
      acknowledged: false,
      created_at: Date.now() * 1000,
      action_required: true,
    };

    expect(alert.alert_type).toBe('DrugInteraction');
    expect(alert.acknowledged).toBe(false);
    expect(alert.action_required).toBe(true);
  });

  it('should allow optional fields', () => {
    const alert: ClinicalAlert = {
      patient_hash: new Uint8Array(32),
      alert_type: 'Preventive',
      priority: 'Low',
      title: 'Screening Reminder',
      message: 'Annual checkup due',
      acknowledged: false,
      created_at: Date.now() * 1000,
      action_required: false,
    };

    expect(alert.hash).toBeUndefined();
    expect(alert.source_reference).toBeUndefined();
    expect(alert.acknowledged_by).toBeUndefined();
    expect(alert.expires_at).toBeUndefined();
  });
});

describe('InteractionCheckResponse interface', () => {
  it('should create valid response with interactions', () => {
    const response: InteractionCheckResponse = {
      checked_at: Date.now() * 1000,
      drug_interactions: [
        {
          drug_a_rxnorm: '197381',
          drug_a_name: 'Warfarin',
          drug_b_rxnorm: '161',
          drug_b_name: 'Aspirin',
          severity: 'Major',
          description: 'Bleeding risk',
          management: 'Monitor closely',
          references: [],
        },
      ],
      allergy_interactions: [],
      has_contraindications: false,
      has_major_interactions: true,
      summary: '1 major interaction found',
    };

    expect(response.drug_interactions).toHaveLength(1);
    expect(response.has_major_interactions).toBe(true);
  });

  it('should create valid response with no interactions', () => {
    const response: InteractionCheckResponse = {
      checked_at: Date.now() * 1000,
      drug_interactions: [],
      allergy_interactions: [],
      has_contraindications: false,
      has_major_interactions: false,
      summary: 'No interactions found',
    };

    expect(response.drug_interactions).toHaveLength(0);
    expect(response.allergy_interactions).toHaveLength(0);
  });
});

describe('CdsClient', () => {
  let client: CdsClient;

  beforeEach(() => {
    mockCallZome.mockReset();
    client = new CdsClient(mockAppClient, 'health');
  });

  describe('constructor', () => {
    it('should create client with role name', () => {
      const testClient = new CdsClient(mockAppClient, 'test-role');
      expect(testClient).toBeInstanceOf(CdsClient);
    });
  });

  describe('checkDrugInteractions', () => {
    it('should call zome with correct parameters', async () => {
      const interactions: DrugInteraction[] = [];
      mockCallZome.mockResolvedValue(interactions);

      const result = await client.checkDrugInteractions(['197381', '161']);

      expect(mockCallZome).toHaveBeenCalledWith({
        role_name: 'health',
        zome_name: 'cds',
        fn_name: 'check_drug_interactions',
        payload: { medications: ['197381', '161'] },
      });
      expect(result).toEqual(interactions);
    });

    it('should return interactions when found', async () => {
      const interactions: DrugInteraction[] = [
        {
          drug_a_rxnorm: '197381',
          drug_a_name: 'Warfarin',
          drug_b_rxnorm: '161',
          drug_b_name: 'Aspirin',
          severity: 'Major',
          description: 'Bleeding risk',
          management: 'Monitor',
          references: [],
        },
      ];
      mockCallZome.mockResolvedValue(interactions);

      const result = await client.checkDrugInteractions(['197381', '161']);
      expect(result).toHaveLength(1);
      expect(result[0].severity).toBe('Major');
    });
  });

  describe('checkAllergyConflicts', () => {
    it('should call zome with medications and allergies', async () => {
      mockCallZome.mockResolvedValue([]);

      await client.checkAllergyConflicts(['723'], ['Penicillin']);

      expect(mockCallZome).toHaveBeenCalledWith({
        role_name: 'health',
        zome_name: 'cds',
        fn_name: 'check_allergy_conflicts',
        payload: { medications: ['723'], allergies: ['Penicillin'] },
      });
    });

    it('should return allergy conflicts when found', async () => {
      const conflicts: DrugAllergyInteraction[] = [
        {
          drug_rxnorm: '723',
          drug_name: 'Amoxicillin',
          allergen: 'Penicillin',
          cross_reactivity_risk: 'Major',
          description: 'Cross-reactivity',
          alternatives: ['Azithromycin'],
        },
      ];
      mockCallZome.mockResolvedValue(conflicts);

      const result = await client.checkAllergyConflicts(['723'], ['Penicillin']);
      expect(result).toHaveLength(1);
      expect(result[0].allergen).toBe('Penicillin');
    });
  });

  describe('performInteractionCheck', () => {
    it('should perform full interaction check', async () => {
      const response: InteractionCheckResponse = {
        checked_at: Date.now() * 1000,
        drug_interactions: [],
        allergy_interactions: [],
        has_contraindications: false,
        has_major_interactions: false,
        summary: 'No interactions',
      };
      mockCallZome.mockResolvedValue(response);

      const result = await client.performInteractionCheck({
        patient_hash: new Uint8Array(32),
        rxnorm_codes: ['197381'],
        allergies: [],
      });

      expect(result.summary).toBe('No interactions');
    });
  });

  describe('createAlert', () => {
    it('should create alert with required fields', async () => {
      const alertHash = new Uint8Array(32);
      mockCallZome.mockResolvedValue(alertHash);

      const result = await client.createAlert({
        patient_hash: new Uint8Array(32),
        alert_type: 'DrugInteraction',
        priority: 'High',
        title: 'Test Alert',
        message: 'Test message',
      });

      expect(mockCallZome).toHaveBeenCalledWith(
        expect.objectContaining({
          fn_name: 'create_clinical_alert',
          payload: expect.objectContaining({
            action_required: false,
          }),
        })
      );
      expect(result).toEqual(alertHash);
    });

    it('should pass action_required when specified', async () => {
      mockCallZome.mockResolvedValue(new Uint8Array(32));

      await client.createAlert({
        patient_hash: new Uint8Array(32),
        alert_type: 'AllergyAlert',
        priority: 'Critical',
        title: 'Allergy Alert',
        message: 'Severe allergy detected',
        action_required: true,
      });

      expect(mockCallZome).toHaveBeenCalledWith(
        expect.objectContaining({
          payload: expect.objectContaining({
            action_required: true,
          }),
        })
      );
    });
  });

  describe('getPatientAlerts', () => {
    it('should get alerts excluding acknowledged by default', async () => {
      mockCallZome.mockResolvedValue([]);

      await client.getPatientAlerts(new Uint8Array(32));

      expect(mockCallZome).toHaveBeenCalledWith(
        expect.objectContaining({
          fn_name: 'get_patient_alerts',
          payload: expect.objectContaining({
            include_acknowledged: false,
          }),
        })
      );
    });

    it('should include acknowledged alerts when requested', async () => {
      mockCallZome.mockResolvedValue([]);

      await client.getPatientAlerts(new Uint8Array(32), true);

      expect(mockCallZome).toHaveBeenCalledWith(
        expect.objectContaining({
          payload: expect.objectContaining({
            include_acknowledged: true,
          }),
        })
      );
    });
  });

  describe('acknowledgeAlert', () => {
    it('should acknowledge alert without action', async () => {
      mockCallZome.mockResolvedValue(undefined);

      await client.acknowledgeAlert(new Uint8Array(32));

      expect(mockCallZome).toHaveBeenCalledWith(
        expect.objectContaining({
          fn_name: 'acknowledge_alert',
          payload: expect.objectContaining({
            action_taken: undefined,
          }),
        })
      );
    });

    it('should acknowledge alert with action taken', async () => {
      mockCallZome.mockResolvedValue(undefined);

      await client.acknowledgeAlert(new Uint8Array(32), 'Reviewed and adjusted dosage');

      expect(mockCallZome).toHaveBeenCalledWith(
        expect.objectContaining({
          payload: expect.objectContaining({
            action_taken: 'Reviewed and adjusted dosage',
          }),
        })
      );
    });
  });

  describe('getCriticalAlerts', () => {
    it('should filter for high priority alerts requiring action', async () => {
      const alerts: ClinicalAlert[] = [
        {
          patient_hash: new Uint8Array(32),
          alert_type: 'DrugInteraction',
          priority: 'Critical',
          title: 'Critical Alert',
          message: 'Urgent',
          acknowledged: false,
          created_at: Date.now() * 1000,
          action_required: true,
        },
        {
          patient_hash: new Uint8Array(32),
          alert_type: 'Preventive',
          priority: 'Low',
          title: 'Low Priority',
          message: 'Reminder',
          acknowledged: false,
          created_at: Date.now() * 1000,
          action_required: false,
        },
        {
          patient_hash: new Uint8Array(32),
          alert_type: 'LabResult',
          priority: 'High',
          title: 'Lab Alert',
          message: 'Abnormal result',
          acknowledged: false,
          created_at: Date.now() * 1000,
          action_required: true,
        },
      ];
      mockCallZome.mockResolvedValue(alerts);

      const result = await client.getCriticalAlerts(new Uint8Array(32));

      expect(result).toHaveLength(2);
      expect(result.every(a => a.priority === 'Critical' || a.priority === 'High')).toBe(true);
      expect(result.every(a => a.action_required)).toBe(true);
    });
  });

  describe('registerDrugInteraction', () => {
    it('should register new drug interaction', async () => {
      const hash = new Uint8Array(32);
      mockCallZome.mockResolvedValue(hash);

      const result = await client.registerDrugInteraction({
        drug_a_rxnorm: '197381',
        drug_a_name: 'Warfarin',
        drug_b_rxnorm: '161',
        drug_b_name: 'Aspirin',
        severity: 'Major',
        description: 'Bleeding risk',
        management: 'Monitor INR',
        references: [],
      });

      expect(mockCallZome).toHaveBeenCalledWith(
        expect.objectContaining({
          fn_name: 'register_drug_interaction',
        })
      );
      expect(result).toEqual(hash);
    });
  });

  describe('createGuideline', () => {
    it('should create clinical guideline', async () => {
      const hash = new Uint8Array(32);
      mockCallZome.mockResolvedValue(hash);

      const result = await client.createGuideline({
        guideline_id: 'USPSTF-001',
        title: 'Colorectal Cancer Screening',
        category: 'Screening',
        condition_codes: ['Z12.11'],
        description: 'Adults 45-75 should be screened',
        recommendations: [
          {
            text: 'Screen adults aged 45-75',
            strength: 'Strong',
            evidence_level: 'A',
          },
        ],
        source: 'USPSTF',
        evidence_level: 'A',
        last_reviewed: Date.now() * 1000,
        version: '2021',
      });

      expect(mockCallZome).toHaveBeenCalledWith(
        expect.objectContaining({
          fn_name: 'create_clinical_guideline',
        })
      );
      expect(result).toEqual(hash);
    });
  });

  describe('error handling', () => {
    it('should throw HealthSdkError on zome call failure', async () => {
      mockCallZome.mockRejectedValue(new Error('Network error'));

      await expect(client.checkDrugInteractions(['123'])).rejects.toThrow(HealthSdkError);
    });

    it('should include error details in HealthSdkError', async () => {
      mockCallZome.mockRejectedValue(new Error('Zome not found'));

      try {
        await client.checkDrugInteractions(['123']);
        expect.fail('Should have thrown');
      } catch (error) {
        expect(error).toBeInstanceOf(HealthSdkError);
        expect((error as HealthSdkError).code).toBe(HealthSdkErrorCode.ZOME_CALL_FAILED);
        expect((error as HealthSdkError).message).toContain('CDS zome call failed');
      }
    });
  });
});
