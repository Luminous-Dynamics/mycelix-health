// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
/**
 * Mycelix-Health Integration Tests
 *
 * Full conductor tests using Tryorama for end-to-end validation
 * of the healthcare hApp functionality.
 */

import { runScenario, Scenario, Player } from '@holochain/tryorama';
import { describe, it, expect, beforeAll } from 'vitest';
import {
  ActionHash,
  AgentPubKey,
  fakeActionHash,
  Record as HolochainRecord,
} from '@holochain/client';
import { decode } from '@msgpack/msgpack';
import { resolve } from 'node:path';

const HEALTH_HAPP_PATH = '../../mycelix-health.happ';

type PatientEntryForTest = {
  patient_id: string;
  first_name: string;
  last_name: string;
  allergies: Array<{ allergen: string; reaction: string }>;
};

type TrialEntryForTest = {
  nct_number: string | null;
  irb_approved: boolean;
};

type EpistemicClaimForTest = {
  claim_id: string;
  claim_content: string;
  classification: {
    empirical_level: number;
    materiality_level: number;
    normative_level: number;
  };
  evidence_hashes: ActionHash[];
};

function actionHashOf(record: HolochainRecord): ActionHash {
  return record.signed_action.hashed.hash;
}

function decodeAppRecord<T>(record: HolochainRecord | null): T {
  if (record === null) {
    throw new Error('Expected a present Holochain record');
  }
  if (!('Present' in record.entry)) {
    throw new Error('Holochain record does not contain a visible entry');
  }
  const entry = record.entry.Present;
  if (entry.entry_type !== 'App') {
    throw new Error('Expected an application entry');
  }
  return decode(entry.entry) as T;
}

async function createPatientFixture(
  player: Player,
  patientId = 'PAT-001',
  firstName = 'Alice',
  lastName = 'Johnson',
): Promise<{ hash: ActionHash; record: HolochainRecord }> {
  const now = Date.now() * 1000;
  const record = await player.cells[0].callZome<HolochainRecord>({
    zome_name: 'patient',
    fn_name: 'create_patient',
    payload: {
      patient_id: patientId,
      mrn: null,
      first_name: firstName,
      last_name: lastName,
      date_of_birth: '1996-04-15',
      biological_sex: 'Female',
      gender_identity: null,
      blood_type: 'APositive',
      contact: {
        address_line1: '123 Main Street',
        address_line2: null,
        city: 'Testville',
        state_province: 'TX',
        postal_code: '75001',
        country: 'US',
        phone_primary: '+1-555-0100',
        phone_secondary: null,
        email: 'patient@example.test',
      },
      emergency_contact: {
        name: 'Jordan Johnson',
        relationship: 'Sibling',
        phone: '+1-555-0101',
        email: null,
      },
      primary_language: 'en',
      allergies: [
        {
          allergen: 'Penicillin',
          reaction: 'Anaphylaxis',
          severity: 'Severe',
          verified: false,
          verified_by: null,
          verified_at: null,
        },
      ],
      conditions: [],
      medications: [],
      mycelix_identity_hash: null,
      matl_trust_score: 0.9,
      created_at: now,
      updated_at: now,
    },
  });
  return { hash: actionHashOf(record), record };
}

async function createConsentFixture(
  grantor: Player,
  patientHash: ActionHash,
  grantee: AgentPubKey,
  options: {
    dataCategories?: string[];
    permissions?: string[];
    consentId?: string;
  } = {},
): Promise<{ hash: ActionHash; record: HolochainRecord }> {
  const now = Date.now() * 1000;
  const record = await grantor.cells[0].callZome<HolochainRecord>({
    zome_name: 'consent',
    fn_name: 'create_consent',
    payload: {
      consent_id: options.consentId ?? 'CONSENT-INTEGRATION-001',
      patient_hash: patientHash,
      grantee: { Agent: grantee },
      scope: {
        data_categories: options.dataCategories ?? ['Demographics', 'Medications'],
        date_range: null,
        encounter_hashes: null,
        exclusions: [],
      },
      permissions: options.permissions ?? ['Read'],
      purpose: 'Treatment',
      status: 'Active',
      granted_at: now,
      expires_at: now + 365 * 24 * 60 * 60 * 1_000_000,
      revoked_at: null,
      revocation_reason: null,
      document_hash: null,
      witness: null,
      legal_representative: null,
      notes: null,
    },
  });
  return { hash: actionHashOf(record), record };
}

function clinicalTrialFixture(principalInvestigator: AgentPubKey, trialId: string) {
  const now = Date.now() * 1000;
  return {
    trial_id: trialId,
    nct_number: 'NCT05123456',
    title: 'Phase 3 Randomized Controlled Trial',
    short_title: 'Phase 3 Trial',
    description: 'Synthetic integration-test clinical trial record.',
    phase: 'Phase3',
    study_type: 'Interventional',
    status: 'Recruiting',
    principal_investigator: principalInvestigator,
    sponsor: 'Luminous Therapeutics',
    collaborators: [],
    target_enrollment: 500,
    current_enrollment: 0,
    start_date: now,
    estimated_end_date: null,
    actual_end_date: null,
    eligibility: {
      min_age: 18,
      max_age: 65,
      sex: 'All',
      healthy_volunteers: false,
      inclusion_criteria: ['Confirmed diagnosis'],
      exclusion_criteria: ['Pregnancy'],
    },
    interventions: [
      {
        intervention_type: 'Drug',
        name: 'LUM-001',
        description: 'Synthetic test intervention',
        arm_group: 'Treatment',
      },
    ],
    primary_outcomes: [
      {
        measure: 'Overall response rate',
        time_frame: '12 weeks',
        description: 'Synthetic primary outcome',
      },
    ],
    secondary_outcomes: [],
    irb_approved: true,
    irb_approval_date: now,
    irb_expiration_date: null,
    desci_publication_hash: null,
    epistemic_level: 'E1PeerReviewed',
    matl_trust_score: 0.8,
    created_at: now,
    updated_at: now,
  };
}

// ============================================================================
// Patient Zome Tests
// ============================================================================

describe('Patient Zome', () => {
  it('creates and retrieves a patient', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice] = await scenario.addPlayersWithApps([
        { appBundleSource: { type: 'path', value: resolve(HEALTH_HAPP_PATH) } },
      ]);

      const { hash: patientHash } = await createPatientFixture(
        alice,
        'PAT-001',
        'Alice',
        'Johnson',
      );
      await createConsentFixture(alice, patientHash, alice.agentPubKey, {
        dataCategories: ['Demographics'],
      });

      const retrievedRecord = await alice.cells[0].callZome<HolochainRecord | null>({
        zome_name: 'patient',
        fn_name: 'get_patient',
        payload: {
          patient_hash: patientHash,
          is_emergency: false,
          emergency_reason: null,
        },
      });

      expect(retrievedRecord).not.toBeNull();
      const retrieved = decodeAppRecord<PatientEntryForTest>(retrievedRecord);
      expect(retrieved.patient_id).toBe('PAT-001');
      expect(retrieved.first_name).toBe('Alice');
      expect(retrieved.allergies).toHaveLength(1);
    });
  });

  it('requires admin authorization before searching across patients', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice] = await scenario.addPlayersWithApps([
        { appBundleSource: { type: 'path', value: resolve(HEALTH_HAPP_PATH) } },
      ]);

      await expect(
        alice.cells[0].callZome<HolochainRecord[]>({
          zome_name: 'patient',
          fn_name: 'search_patients_by_name',
          payload: { name: 'Johnson' },
        }),
      ).rejects.toThrow();
    });
  });
});

// ============================================================================
// Consent Zome Tests (HIPAA Compliance)
// ============================================================================

describe('Consent Zome - HIPAA Compliance', () => {
  it('creates consent with scoped authorization', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice, bob] = await scenario.addPlayersWithApps([
        { appBundleSource: { type: 'path', value: resolve(HEALTH_HAPP_PATH) } },
        { appBundleSource: { type: 'path', value: resolve(HEALTH_HAPP_PATH) } },
      ]);
      const { hash: patientHash } = await createPatientFixture(alice, 'PAT-CONSENT-001');
      await createConsentFixture(alice, patientHash, bob.agentPubKey, {
        dataCategories: ['Medications'],
        permissions: ['Read'],
        consentId: 'CONSENT-001',
      });

      const authorization = await bob.cells[0].callZome<{ authorized: boolean }>({
        zome_name: 'consent',
        fn_name: 'check_authorization',
        payload: {
          patient_hash: patientHash,
          requestor: bob.agentPubKey,
          data_category: 'Medications',
          permission: 'Read',
          is_emergency: false,
        },
      });

      expect(authorization.authorized).toBe(true);
    });
  });

  it('prevents access after consent revocation', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice, bob] = await scenario.addPlayersWithApps([
        { appBundleSource: { type: 'path', value: resolve(HEALTH_HAPP_PATH) } },
        { appBundleSource: { type: 'path', value: resolve(HEALTH_HAPP_PATH) } },
      ]);
      const { hash: patientHash } = await createPatientFixture(alice, 'PAT-CONSENT-002');
      const { hash: consentHash } = await createConsentFixture(
        alice,
        patientHash,
        bob.agentPubKey,
        { dataCategories: ['Demographics'], consentId: 'CONSENT-002' },
      );

      await alice.cells[0].callZome<HolochainRecord>({
        zome_name: 'consent',
        fn_name: 'revoke_consent',
        payload: {
          consent_hash: consentHash,
          reason: 'Patient withdrew consent in integration test',
        },
      });

      const authorization = await bob.cells[0].callZome<{ authorized: boolean }>({
        zome_name: 'consent',
        fn_name: 'check_authorization',
        payload: {
          patient_hash: patientHash,
          requestor: bob.agentPubKey,
          data_category: 'Demographics',
          permission: 'Read',
          is_emergency: false,
        },
      });

      expect(authorization.authorized).toBe(false);
    });
  });

  it('logs successful patient reads in the append-only access trail', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice, bob] = await scenario.addPlayersWithApps([
        { appBundleSource: { type: 'path', value: resolve(HEALTH_HAPP_PATH) } },
        { appBundleSource: { type: 'path', value: resolve(HEALTH_HAPP_PATH) } },
      ]);
      const { hash: patientHash } = await createPatientFixture(alice, 'PAT-CONSENT-003');
      await createConsentFixture(alice, patientHash, bob.agentPubKey, {
        dataCategories: ['Demographics'],
        permissions: ['Read'],
        consentId: 'CONSENT-003',
      });

      const retrieved = await bob.cells[0].callZome<HolochainRecord | null>({
        zome_name: 'patient',
        fn_name: 'get_patient',
        payload: {
          patient_hash: patientHash,
          is_emergency: false,
          emergency_reason: null,
        },
      });
      expect(retrieved).not.toBeNull();

      const logs = await alice.cells[0].callZome<HolochainRecord[]>({
        zome_name: 'consent',
        fn_name: 'get_access_logs',
        payload: patientHash,
      });
      expect(logs.length).toBeGreaterThan(0);
    });
  });

});

// ============================================================================
// Clinical Trials Zome Tests
// ============================================================================

describe('Trials Zome - FDA Compliance', () => {
  it('creates and retrieves a clinical trial with its declared fields', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice] = await scenario.addPlayersWithApps([
        { appBundleSource: { type: 'path', value: resolve(HEALTH_HAPP_PATH) } },
      ]);

      const created = await alice.cells[0].callZome<HolochainRecord>({
        zome_name: 'trials',
        fn_name: 'create_trial',
        payload: clinicalTrialFixture(alice.agentPubKey, 'TRIAL-2024-001'),
      });
      const trialHash = actionHashOf(created);

      const retrievedRecord = await alice.cells[0].callZome<HolochainRecord | null>({
        zome_name: 'trials',
        fn_name: 'get_trial',
        payload: trialHash,
      });
      expect(retrievedRecord).not.toBeNull();

      const retrieved = decodeAppRecord<TrialEntryForTest>(retrievedRecord);
      expect(retrieved.nct_number).toBe('NCT05123456');
      expect(retrieved.irb_approved).toBe(true);
    });
  });

  it('checks patient eligibility for trial', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice] = await scenario.addPlayersWithApps([
        { appBundleSource: { type: 'path', value: resolve(HEALTH_HAPP_PATH) } },
      ]);

      const created = await alice.cells[0].callZome<HolochainRecord>({
        zome_name: 'trials',
        fn_name: 'create_trial',
        payload: clinicalTrialFixture(alice.agentPubKey, 'TRIAL-ELIGIBILITY-001'),
      });
      const trialHash = actionHashOf(created);
      const { hash: patientHash } = await createPatientFixture(
        alice,
        'PAT-ELIGIBILITY-001',
      );

      const eligible = await alice.cells[0].callZome<{ eligible: boolean }>({
        zome_name: 'trials',
        fn_name: 'check_eligibility',
        payload: {
          trial_hash: trialHash,
          patient_hash: patientHash,
          patient_age: 30,
        },
      });
      expect(eligible.eligible).toBe(true);

      const notEligible = await alice.cells[0].callZome<{ eligible: boolean }>({
        zome_name: 'trials',
        fn_name: 'check_eligibility',
        payload: {
          trial_hash: trialHash,
          patient_hash: patientHash,
          patient_age: 70,
        },
      });
      expect(notEligible.eligible).toBe(false);
    });
  });

  it('reports an adverse event for an enrolled participant with valid consent', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice] = await scenario.addPlayersWithApps([
        { appBundleSource: { type: 'path', value: resolve(HEALTH_HAPP_PATH) } },
      ]);
      const { hash: patientHash } = await createPatientFixture(alice, 'PAT-AE-001');
      const { hash: consentHash } = await createConsentFixture(
        alice,
        patientHash,
        alice.agentPubKey,
        {
          dataCategories: ['All'],
          permissions: ['Write'],
          consentId: 'CONSENT-AE-001',
        },
      );
      const trialRecord = await alice.cells[0].callZome<HolochainRecord>({
        zome_name: 'trials',
        fn_name: 'create_trial',
        payload: clinicalTrialFixture(alice.agentPubKey, 'TRIAL-AE-001'),
      });
      const trialHash = actionHashOf(trialRecord);
      const now = Date.now() * 1000;

      const participantRecord = await alice.cells[0].callZome<HolochainRecord>({
        zome_name: 'trials',
        fn_name: 'enroll_participant',
        payload: {
          participant_id: 'PART-AE-001',
          trial_hash: trialHash,
          patient_hash: patientHash,
          consent_hash: consentHash,
          enrollment_date: now,
          withdrawal_date: null,
          withdrawal_reason: null,
          arm_assignment: 'Treatment',
          status: 'Enrolled',
          blinded: false,
          screening_passed: true,
          screening_date: now,
          enrollment_site: 'Synthetic Test Site',
          primary_contact: alice.agentPubKey,
        },
      });
      const participantHash = actionHashOf(participantRecord);

      const adverseEvent = {
        event_id: 'AE-001',
        participant_hash: participantHash,
        trial_hash: trialHash,
        event_term: 'Headache',
        description: 'Synthetic mild headache event for integration testing.',
        onset_date: now,
        resolution_date: null,
        ongoing: false,
        severity: 'Mild',
        seriousness: [],
        is_serious: false,
        is_unexpected: false,
        causality: 'PossiblyRelated',
        outcome: 'Recovered',
        action_taken: ['NoneRequired'],
        reported_by: alice.agentPubKey,
        reported_at: now,
        medwatch_submitted: false,
        medwatch_date: null,
      };

      const eventRecord = await alice.cells[0].callZome<HolochainRecord>({
        zome_name: 'trials',
        fn_name: 'report_adverse_event',
        payload: adverseEvent,
      });

      expect(actionHashOf(eventRecord)).toBeDefined();
    });
  });
});

// ============================================================================
// Bridge Zome Tests - Cross-hApp Communication
// ============================================================================

describe('Bridge Zome - Cross-hApp Federation', () => {
  it('registers health hApp with bridge using an action hash', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice] = await scenario.addPlayersWithApps([
        { appBundleSource: { type: 'path', value: resolve(HEALTH_HAPP_PATH) } },
      ]);

      const registration = {
        registration_id: 'REG-HEALTH-001',
        mycelix_identity_hash: await fakeActionHash(0x42),
        capabilities: [
          'PatientLookup',
          'ProviderVerification',
          'ConsentVerification',
          'EpistemicClaims',
        ],
        federated_data: ['ProviderCredentials', 'ConsentRecords'],
        minimum_trust_score: 0.7,
      };

      const record = await alice.cells[0].callZome<HolochainRecord>({
        zome_name: 'bridge',
        fn_name: 'register_with_bridge',
        payload: registration,
      });

      expect(record).toBeDefined();
      expect(actionHashOf(record)).toBeDefined();
    });
  });

  it('creates and retrieves an epistemic claim through the entity index', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice] = await scenario.addPlayersWithApps([
        { appBundleSource: { type: 'path', value: resolve(HEALTH_HAPP_PATH) } },
      ]);
      const { hash: subjectHash } = await createPatientFixture(alice, 'PAT-CLAIM-001');

      const claimHashRecord = await alice.cells[0].callZome<HolochainRecord>({
        zome_name: 'bridge',
        fn_name: 'create_epistemic_claim',
        payload: {
          claim_id: 'CLAIM-001',
          claimant: alice.agentPubKey,
          subject_hash: subjectHash,
          claim_type: 'Treatment',
          claim_content: 'Synthetic test claim; not a clinical efficacy assertion.',
          evidence_hashes: [],
          classification: {
            empirical_level: 2,
            materiality_level: 2,
            normative_level: 2,
          },
          matl_score: 0.8,
          created_at: Date.now() * 1000,
          verified: false,
          verified_by: [],
        },
      });
      expect(claimHashRecord).toBeDefined();

      const records = await alice.cells[0].callZome<HolochainRecord[]>({
        zome_name: 'bridge',
        fn_name: 'get_entity_claims',
        payload: subjectHash,
      });
      const retrieved = records
        .map((record) => decodeAppRecord<EpistemicClaimForTest>(record))
        .find((claim) => claim.claim_id === 'CLAIM-001');

      expect(retrieved).toBeDefined();
      expect(retrieved?.classification.empirical_level).toBe(2);
      expect(retrieved?.evidence_hashes).toHaveLength(0);
    });
  });

  it('aggregates federated reputation scores with complete source records', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice] = await scenario.addPlayersWithApps([
        { appBundleSource: { type: 'path', value: resolve(HEALTH_HAPP_PATH) } },
      ]);
      const now = Date.now() * 1000;
      const result = await alice.cells[0].callZome<{
        federation_id: string;
        aggregated_score: number;
        scores: Array<{ timestamp: number }>;
      }>({
        zome_name: 'bridge',
        fn_name: 'aggregate_reputation',
        payload: {
          federation_id: 'FEDERATION-TEST-001',
          entity_hash: await fakeActionHash(0x43),
          entity_type: 'Provider',
          scores: [
            {
              source_happ: 'mycelix-identity',
              score: 0.95,
              weight: 0.25,
              score_type: 'verification',
              timestamp: now,
            },
            {
              source_happ: 'mycelix-health',
              score: 0.88,
              weight: 0.3,
              score_type: 'patient_outcomes',
              timestamp: now,
            },
            {
              source_happ: 'mycelix-health',
              score: 0.82,
              weight: 0.2,
              score_type: 'peer_attestations',
              timestamp: now,
            },
          ],
        },
      });

      expect(result.federation_id).toBe('FEDERATION-TEST-001');
      expect(result.aggregated_score).toBeCloseTo(
        (0.95 * 0.25 + 0.88 * 0.3 + 0.82 * 0.2) / (0.25 + 0.3 + 0.2),
        10,
      );
      expect(result.scores).toHaveLength(3);
      expect(result.scores.every((score) => score.timestamp === now)).toBe(true);
    });
  });
});
