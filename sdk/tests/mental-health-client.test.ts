/**
 * Mental Health Client Tests
 *
 * Tests for Mental Health zome client functionality including
 * screenings, mood tracking, safety plans, and 42 CFR Part 2 consent.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  MentalHealthClient,
  MentalHealthInstrument,
  Severity,
  CrisisLevel,
  TreatmentModality,
  SafetyPlanStatus,
  SubstanceCategory,
  Part2ConsentType,
  MentalHealthScreening,
  MoodEntry,
  MentalHealthTreatmentPlan,
  SafetyPlan,
  CrisisEvent,
  Part2Consent,
  TherapyNote,
  CreateScreeningInput,
  CreateMoodEntryInput,
  CreateSafetyPlanInput,
  CreateCrisisEventInput,
  CreatePart2ConsentInput,
} from '../src/zomes/mental-health';

// Mock AppClient
const mockCallZome = vi.fn();
const mockAppClient = {
  callZome: mockCallZome,
} as any;

describe('Mental Health Enums', () => {
  describe('MentalHealthInstrument', () => {
    it('should have all screening instruments', () => {
      expect(MentalHealthInstrument.PHQ9).toBe('PHQ9');
      expect(MentalHealthInstrument.PHQ2).toBe('PHQ2');
      expect(MentalHealthInstrument.GAD7).toBe('GAD7');
      expect(MentalHealthInstrument.CSSRS).toBe('CSSRS');
      expect(MentalHealthInstrument.CAGE).toBe('CAGE');
      expect(MentalHealthInstrument.AUDIT).toBe('AUDIT');
      expect(MentalHealthInstrument.DAST10).toBe('DAST10');
      expect(MentalHealthInstrument.PCL5).toBe('PCL5');
      expect(MentalHealthInstrument.MDQ).toBe('MDQ');
      expect(MentalHealthInstrument.EPDS).toBe('EPDS');
      expect(MentalHealthInstrument.PSC17).toBe('PSC17');
      expect(MentalHealthInstrument.Custom).toBe('Custom');
    });
  });

  describe('Severity', () => {
    it('should have all severity levels', () => {
      expect(Severity.None).toBe('None');
      expect(Severity.Minimal).toBe('Minimal');
      expect(Severity.Mild).toBe('Mild');
      expect(Severity.Moderate).toBe('Moderate');
      expect(Severity.ModeratelySevere).toBe('ModeratelySevere');
      expect(Severity.Severe).toBe('Severe');
    });
  });

  describe('CrisisLevel', () => {
    it('should have all crisis levels', () => {
      expect(CrisisLevel.None).toBe('None');
      expect(CrisisLevel.LowRisk).toBe('LowRisk');
      expect(CrisisLevel.ModerateRisk).toBe('ModerateRisk');
      expect(CrisisLevel.HighRisk).toBe('HighRisk');
      expect(CrisisLevel.Imminent).toBe('Imminent');
    });
  });

  describe('TreatmentModality', () => {
    it('should have all treatment modalities', () => {
      expect(TreatmentModality.IndividualTherapy).toBe('IndividualTherapy');
      expect(TreatmentModality.GroupTherapy).toBe('GroupTherapy');
      expect(TreatmentModality.FamilyTherapy).toBe('FamilyTherapy');
      expect(TreatmentModality.Medication).toBe('Medication');
      expect(TreatmentModality.IntensiveOutpatient).toBe('IntensiveOutpatient');
      expect(TreatmentModality.PartialHospitalization).toBe('PartialHospitalization');
      expect(TreatmentModality.Inpatient).toBe('Inpatient');
      expect(TreatmentModality.CrisisIntervention).toBe('CrisisIntervention');
      expect(TreatmentModality.PeerSupport).toBe('PeerSupport');
      expect(TreatmentModality.Telehealth).toBe('Telehealth');
      expect(TreatmentModality.Other).toBe('Other');
    });
  });

  describe('SafetyPlanStatus', () => {
    it('should have all status values', () => {
      expect(SafetyPlanStatus.Active).toBe('Active');
      expect(SafetyPlanStatus.NeedsUpdate).toBe('NeedsUpdate');
      expect(SafetyPlanStatus.Expired).toBe('Expired');
      expect(SafetyPlanStatus.NotApplicable).toBe('NotApplicable');
    });
  });

  describe('SubstanceCategory', () => {
    it('should have all substance categories', () => {
      expect(SubstanceCategory.Alcohol).toBe('Alcohol');
      expect(SubstanceCategory.Cannabis).toBe('Cannabis');
      expect(SubstanceCategory.Opioids).toBe('Opioids');
      expect(SubstanceCategory.Stimulants).toBe('Stimulants');
      expect(SubstanceCategory.Sedatives).toBe('Sedatives');
      expect(SubstanceCategory.Hallucinogens).toBe('Hallucinogens');
      expect(SubstanceCategory.Tobacco).toBe('Tobacco');
      expect(SubstanceCategory.Other).toBe('Other');
    });
  });

  describe('Part2ConsentType', () => {
    it('should have all 42 CFR Part 2 consent types', () => {
      expect(Part2ConsentType.GeneralDisclosure).toBe('GeneralDisclosure');
      expect(Part2ConsentType.RedisclosureProhibited).toBe('RedisclosureProhibited');
      expect(Part2ConsentType.MedicalEmergency).toBe('MedicalEmergency');
      expect(Part2ConsentType.Research).toBe('Research');
      expect(Part2ConsentType.CourtOrder).toBe('CourtOrder');
      expect(Part2ConsentType.AuditEvaluation).toBe('AuditEvaluation');
    });
  });
});

describe('Mental Health Interfaces', () => {
  describe('MentalHealthScreening', () => {
    it('should create valid PHQ-9 screening', () => {
      const screening: Partial<MentalHealthScreening> = {
        patientHash: new Uint8Array(32),
        providerHash: new Uint8Array(32),
        instrument: MentalHealthInstrument.PHQ9,
        screeningDate: Date.now() * 1000,
        rawScore: 15,
        severity: Severity.ModeratelySevere,
        responses: [
          ['Little interest or pleasure', 2],
          ['Feeling down, depressed', 2],
          ['Trouble sleeping', 2],
          ['Feeling tired', 2],
          ['Poor appetite', 1],
          ['Feeling bad about yourself', 2],
          ['Trouble concentrating', 2],
          ['Moving slowly or fidgeting', 1],
          ['Thoughts of self-harm', 1],
        ],
        interpretation: 'Moderately severe depression',
        followUpRecommended: true,
        crisisIndicatorsPresent: true,
        createdAt: Date.now() * 1000,
      };

      expect(screening.rawScore).toBe(15);
      expect(screening.severity).toBe(Severity.ModeratelySevere);
      expect(screening.responses).toHaveLength(9);
      expect(screening.crisisIndicatorsPresent).toBe(true);
    });
  });

  describe('MoodEntry', () => {
    it('should create valid mood entry', () => {
      const entry: Partial<MoodEntry> = {
        patientHash: new Uint8Array(32),
        entryDate: Date.now() * 1000,
        moodScore: 6,
        anxietyScore: 4,
        sleepQuality: 7,
        sleepHours: 7.5,
        energyLevel: 5,
        medicationsTaken: true,
        activities: ['exercise', 'meditation'],
        triggers: ['work stress'],
        copingStrategiesUsed: ['deep breathing', 'journaling'],
        createdAt: Date.now() * 1000,
      };

      expect(entry.moodScore).toBe(6);
      expect(entry.activities).toContain('exercise');
      expect(entry.copingStrategiesUsed).toHaveLength(2);
    });
  });

  describe('MentalHealthTreatmentPlan', () => {
    it('should create valid treatment plan', () => {
      const plan: Partial<MentalHealthTreatmentPlan> = {
        patientHash: new Uint8Array(32),
        providerHash: new Uint8Array(32),
        primaryDiagnosisIcd10: 'F32.1',
        secondaryDiagnoses: ['F41.1'],
        treatmentGoals: [
          {
            goalId: 'goal-1',
            description: 'Reduce depressive symptoms',
            progress: 'In progress',
            interventions: ['CBT', 'Medication'],
          },
        ],
        modalities: [TreatmentModality.IndividualTherapy, TreatmentModality.Medication],
        medications: [
          {
            name: 'Sertraline',
            rxnormCode: '312938',
            dosage: '50mg',
            frequency: 'Daily',
            prescriberHash: new Uint8Array(32),
            startDate: Date.now() * 1000,
            targetSymptoms: ['depression', 'anxiety'],
            sideEffectsReported: [],
          },
        ],
        sessionFrequency: 'Weekly',
        status: 'Active',
        createdAt: Date.now() * 1000,
        updatedAt: Date.now() * 1000,
      };

      expect(plan.primaryDiagnosisIcd10).toBe('F32.1');
      expect(plan.modalities).toContain(TreatmentModality.IndividualTherapy);
      expect(plan.medications).toHaveLength(1);
    });
  });

  describe('SafetyPlan', () => {
    it('should create valid safety plan', () => {
      const plan: Partial<SafetyPlan> = {
        patientHash: new Uint8Array(32),
        providerHash: new Uint8Array(32),
        warningSigns: ['Isolating', 'Not sleeping', 'Increased irritability'],
        internalCopingStrategies: ['Go for a walk', 'Listen to music', 'Deep breathing'],
        peopleForDistraction: [
          { name: 'John', relationship: 'Friend', phone: '555-0123' },
        ],
        peopleForHelp: [
          { name: 'Jane', relationship: 'Sister', phone: '555-0124' },
        ],
        professionalsToContact: [
          { name: 'Dr. Smith', phone: '555-0125', availableHours: '9am-5pm' },
        ],
        crisisLine988: true,
        additionalCrisisResources: ['Crisis Text Line: Text HOME to 741741'],
        environmentSafetySteps: ['Remove medications from easy access'],
        reasonsForLiving: ['Family', 'Pets', 'Future goals'],
        status: SafetyPlanStatus.Active,
        createdAt: Date.now() * 1000,
        lastReviewed: Date.now() * 1000,
        nextReviewDate: Date.now() * 1000 + 86400 * 30 * 1000000,
      };

      expect(plan.warningSigns).toHaveLength(3);
      expect(plan.crisisLine988).toBe(true);
      expect(plan.reasonsForLiving).toContain('Family');
    });
  });

  describe('CrisisEvent', () => {
    it('should create valid crisis event', () => {
      const event: Partial<CrisisEvent> = {
        patientHash: new Uint8Array(32),
        reporterHash: new Uint8Array(32),
        eventDate: Date.now() * 1000,
        crisisLevel: CrisisLevel.ModerateRisk,
        suicidalIdeation: true,
        homicidalIdeation: false,
        selfHarm: false,
        substanceIntoxication: false,
        psychoticSymptoms: false,
        description: 'Patient reported passive suicidal thoughts',
        interventionTaken: 'Safety plan review, risk assessment',
        disposition: 'Outpatient with increased monitoring',
        followUpPlan: 'Phone check-in tomorrow, session in 2 days',
        safetyPlanReviewed: true,
        createdAt: Date.now() * 1000,
      };

      expect(event.crisisLevel).toBe(CrisisLevel.ModerateRisk);
      expect(event.suicidalIdeation).toBe(true);
      expect(event.safetyPlanReviewed).toBe(true);
    });
  });

  describe('Part2Consent', () => {
    it('should create valid 42 CFR Part 2 consent', () => {
      const consent: Partial<Part2Consent> = {
        patientHash: new Uint8Array(32),
        consentType: Part2ConsentType.GeneralDisclosure,
        disclosingProgram: 'ABC Treatment Center',
        recipientName: 'Dr. Primary Care',
        purpose: 'Coordination of care',
        informationToDisclose: ['Assessment', 'Treatment progress'],
        substancesCovered: [SubstanceCategory.Alcohol, SubstanceCategory.Opioids],
        effectiveDate: Date.now() * 1000,
        rightToRevokeExplained: true,
        patientSignatureDate: Date.now() * 1000,
        isRevoked: false,
        createdAt: Date.now() * 1000,
      };

      expect(consent.consentType).toBe(Part2ConsentType.GeneralDisclosure);
      expect(consent.substancesCovered).toContain(SubstanceCategory.Alcohol);
      expect(consent.rightToRevokeExplained).toBe(true);
      expect(consent.isRevoked).toBe(false);
    });
  });

  describe('TherapyNote', () => {
    it('should create valid therapy note', () => {
      const note: Partial<TherapyNote> = {
        patientHash: new Uint8Array(32),
        providerHash: new Uint8Array(32),
        sessionDate: Date.now() * 1000,
        sessionType: TreatmentModality.IndividualTherapy,
        durationMinutes: 50,
        presentingConcerns: 'Increased anxiety related to work',
        interventionsUsed: ['CBT', 'Cognitive restructuring'],
        patientResponse: 'Engaged well, practiced techniques',
        riskAssessment: CrisisLevel.LowRisk,
        planForNextSession: 'Continue anxiety management',
        isPsychotherapyNote: true,
        createdAt: Date.now() * 1000,
      };

      expect(note.durationMinutes).toBe(50);
      expect(note.isPsychotherapyNote).toBe(true);
      expect(note.riskAssessment).toBe(CrisisLevel.LowRisk);
    });
  });
});

describe('MentalHealthClient', () => {
  let client: MentalHealthClient;

  beforeEach(() => {
    mockCallZome.mockReset();
    client = new MentalHealthClient(mockAppClient, 'mycelix-health');
  });

  describe('constructor', () => {
    it('should create client with default role name', () => {
      const testClient = new MentalHealthClient(mockAppClient);
      expect(testClient).toBeInstanceOf(MentalHealthClient);
    });

    it('should create client with custom role name', () => {
      const testClient = new MentalHealthClient(mockAppClient, 'custom-role');
      expect(testClient).toBeInstanceOf(MentalHealthClient);
    });
  });

  describe('Screening methods', () => {
    it('should create screening', async () => {
      const mockRecord = { entry: {}, signed_action: {} };
      mockCallZome.mockResolvedValue(mockRecord);

      const input: CreateScreeningInput = {
        patientHash: new Uint8Array(32),
        instrument: MentalHealthInstrument.PHQ9,
        responses: [
          ['Question 1', 2],
          ['Question 2', 1],
        ],
      };

      const result = await client.createScreening(input);

      expect(mockCallZome).toHaveBeenCalledWith({
        role_name: 'mycelix-health',
        zome_name: 'mental_health',
        fn_name: 'create_screening',
        payload: input,
      });
      expect(result).toEqual(mockRecord);
    });

    it('should get patient screenings', async () => {
      const mockRecords = [{ entry: {} }, { entry: {} }];
      mockCallZome.mockResolvedValue(mockRecords);

      const patientHash = new Uint8Array(32);
      const result = await client.getPatientScreenings(patientHash);

      expect(mockCallZome).toHaveBeenCalledWith({
        role_name: 'mycelix-health',
        zome_name: 'mental_health',
        fn_name: 'get_patient_screenings',
        payload: patientHash,
      });
      expect(result).toHaveLength(2);
    });
  });

  describe('Mood tracking methods', () => {
    it('should create mood entry', async () => {
      const mockRecord = { entry: {} };
      mockCallZome.mockResolvedValue(mockRecord);

      const input: CreateMoodEntryInput = {
        patientHash: new Uint8Array(32),
        moodScore: 7,
        anxietyScore: 3,
        sleepQuality: 8,
        energyLevel: 6,
        medicationsTaken: true,
        activities: ['exercise'],
        triggers: [],
        copingStrategiesUsed: [],
      };

      const result = await client.createMoodEntry(input);

      expect(mockCallZome).toHaveBeenCalledWith({
        role_name: 'mycelix-health',
        zome_name: 'mental_health',
        fn_name: 'create_mood_entry',
        payload: input,
      });
      expect(result).toEqual(mockRecord);
    });

    it('should get patient mood entries', async () => {
      mockCallZome.mockResolvedValue([]);

      const patientHash = new Uint8Array(32);
      await client.getPatientMoodEntries(patientHash);

      expect(mockCallZome).toHaveBeenCalledWith({
        role_name: 'mycelix-health',
        zome_name: 'mental_health',
        fn_name: 'get_patient_mood_entries',
        payload: patientHash,
      });
    });
  });

  describe('Safety plan methods', () => {
    it('should create safety plan', async () => {
      const mockRecord = { entry: {} };
      mockCallZome.mockResolvedValue(mockRecord);

      const input: CreateSafetyPlanInput = {
        patientHash: new Uint8Array(32),
        warningSigns: ['Isolating'],
        internalCopingStrategies: ['Walk'],
        peopleForDistraction: [],
        peopleForHelp: [],
        professionalsToContact: [],
        crisisLine988: true,
        additionalCrisisResources: [],
        environmentSafetySteps: [],
        reasonsForLiving: ['Family'],
      };

      const result = await client.createSafetyPlan(input);

      expect(mockCallZome).toHaveBeenCalledWith({
        role_name: 'mycelix-health',
        zome_name: 'mental_health',
        fn_name: 'create_safety_plan',
        payload: input,
      });
      expect(result).toEqual(mockRecord);
    });

    it('should get patient safety plan', async () => {
      const mockRecord = { entry: {} };
      mockCallZome.mockResolvedValue(mockRecord);

      const patientHash = new Uint8Array(32);
      const result = await client.getPatientSafetyPlan(patientHash);

      expect(mockCallZome).toHaveBeenCalledWith({
        role_name: 'mycelix-health',
        zome_name: 'mental_health',
        fn_name: 'get_patient_safety_plan',
        payload: patientHash,
      });
      expect(result).toEqual(mockRecord);
    });

    it('should update safety plan', async () => {
      const mockRecord = { entry: {} };
      mockCallZome.mockResolvedValue(mockRecord);

      const originalHash = new Uint8Array(32);
      const updatedPlan: CreateSafetyPlanInput = {
        patientHash: new Uint8Array(32),
        warningSigns: ['Isolating', 'Insomnia'],
        internalCopingStrategies: ['Walk', 'Music'],
        peopleForDistraction: [],
        peopleForHelp: [],
        professionalsToContact: [],
        crisisLine988: true,
        additionalCrisisResources: [],
        environmentSafetySteps: [],
        reasonsForLiving: ['Family', 'Goals'],
      };

      const result = await client.updateSafetyPlan(originalHash, updatedPlan);

      expect(mockCallZome).toHaveBeenCalledWith({
        role_name: 'mycelix-health',
        zome_name: 'mental_health',
        fn_name: 'update_safety_plan',
        payload: { originalHash, updatedPlan },
      });
      expect(result).toEqual(mockRecord);
    });
  });

  describe('Crisis management methods', () => {
    it('should create crisis event', async () => {
      const mockRecord = { entry: {} };
      mockCallZome.mockResolvedValue(mockRecord);

      const input: CreateCrisisEventInput = {
        patientHash: new Uint8Array(32),
        crisisLevel: CrisisLevel.ModerateRisk,
        suicidalIdeation: true,
        homicidalIdeation: false,
        selfHarm: false,
        substanceIntoxication: false,
        psychoticSymptoms: false,
        description: 'Crisis event',
        interventionTaken: 'Safety plan review',
        disposition: 'Outpatient',
        followUpPlan: 'Tomorrow',
        safetyPlanReviewed: true,
      };

      const result = await client.createCrisisEvent(input);

      expect(mockCallZome).toHaveBeenCalledWith({
        role_name: 'mycelix-health',
        zome_name: 'mental_health',
        fn_name: 'create_crisis_event',
        payload: input,
      });
      expect(result).toEqual(mockRecord);
    });

    it('should get patient crisis events', async () => {
      mockCallZome.mockResolvedValue([]);

      const patientHash = new Uint8Array(32);
      await client.getPatientCrisisEvents(patientHash);

      expect(mockCallZome).toHaveBeenCalledWith({
        role_name: 'mycelix-health',
        zome_name: 'mental_health',
        fn_name: 'get_patient_crisis_events',
        payload: patientHash,
      });
    });
  });

  describe('42 CFR Part 2 consent methods', () => {
    it('should create Part 2 consent', async () => {
      const mockRecord = { entry: {} };
      mockCallZome.mockResolvedValue(mockRecord);

      const input: CreatePart2ConsentInput = {
        patientHash: new Uint8Array(32),
        consentType: Part2ConsentType.GeneralDisclosure,
        disclosingProgram: 'Treatment Center',
        recipientName: 'Dr. Care',
        purpose: 'Care coordination',
        informationToDisclose: ['Assessment'],
        substancesCovered: [SubstanceCategory.Alcohol],
      };

      const result = await client.createPart2Consent(input);

      expect(mockCallZome).toHaveBeenCalledWith({
        role_name: 'mycelix-health',
        zome_name: 'mental_health',
        fn_name: 'create_part2_consent',
        payload: input,
      });
      expect(result).toEqual(mockRecord);
    });

    it('should get patient Part 2 consents', async () => {
      mockCallZome.mockResolvedValue([]);

      const patientHash = new Uint8Array(32);
      await client.getPatientPart2Consents(patientHash);

      expect(mockCallZome).toHaveBeenCalledWith({
        role_name: 'mycelix-health',
        zome_name: 'mental_health',
        fn_name: 'get_patient_part2_consents',
        payload: patientHash,
      });
    });

    it('should revoke Part 2 consent', async () => {
      const mockRecord = { entry: {} };
      mockCallZome.mockResolvedValue(mockRecord);

      const consentHash = new Uint8Array(32);
      const result = await client.revokePart2Consent(consentHash);

      expect(mockCallZome).toHaveBeenCalledWith({
        role_name: 'mycelix-health',
        zome_name: 'mental_health',
        fn_name: 'revoke_part2_consent',
        payload: consentHash,
      });
      expect(result).toEqual(mockRecord);
    });
  });

  describe('Therapy notes methods', () => {
    it('should create therapy note', async () => {
      const mockRecord = { entry: {} };
      mockCallZome.mockResolvedValue(mockRecord);

      const note: TherapyNote = {
        patientHash: new Uint8Array(32),
        providerHash: new Uint8Array(32),
        sessionDate: Date.now() * 1000,
        sessionType: TreatmentModality.IndividualTherapy,
        durationMinutes: 50,
        presentingConcerns: 'Anxiety',
        interventionsUsed: ['CBT'],
        patientResponse: 'Good',
        planForNextSession: 'Continue',
        isPsychotherapyNote: true,
        createdAt: Date.now() * 1000,
      };

      const result = await client.createTherapyNote(note);

      expect(mockCallZome).toHaveBeenCalledWith({
        role_name: 'mycelix-health',
        zome_name: 'mental_health',
        fn_name: 'create_therapy_note',
        payload: note,
      });
      expect(result).toEqual(mockRecord);
    });

    it('should get patient therapy notes', async () => {
      mockCallZome.mockResolvedValue([]);

      const patientHash = new Uint8Array(32);
      await client.getPatientTherapyNotes(patientHash);

      expect(mockCallZome).toHaveBeenCalledWith({
        role_name: 'mycelix-health',
        zome_name: 'mental_health',
        fn_name: 'get_patient_therapy_notes',
        payload: patientHash,
      });
    });
  });
});
