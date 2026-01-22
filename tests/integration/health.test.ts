/**
 * Mycelix-Health Integration Tests
 *
 * Full conductor tests using Tryorama for end-to-end validation
 * of the healthcare hApp functionality.
 */

import { runScenario, Scenario, Player } from '@holochain/tryorama';
import { describe, it, expect, beforeAll } from 'vitest';
import { ActionHash, AgentPubKey, encodeHashToBase64 } from '@holochain/client';

const HEALTH_HAPP_PATH = '../../mycelix-health.happ';

// ============================================================================
// Patient Zome Tests
// ============================================================================

describe('Patient Zome', () => {
  it('creates and retrieves a patient', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice] = await scenario.addPlayersWithApps([
        { appBundleSource: { path: HEALTH_HAPP_PATH } },
      ]);

      const patient = {
        patient_id: 'PAT-001',
        first_name: 'Alice',
        last_name: 'Johnson',
        date_of_birth: Date.now() * 1000 - 30 * 365 * 24 * 60 * 60 * 1000000, // 30 years ago
        biological_sex: 'Female',
        blood_type: 'A+',
        contact: {
          email: 'alice@example.com',
          phone: '+1-555-0123',
          address: '123 Main St',
        },
        emergency_contact: {
          name: 'Bob Johnson',
          relationship: 'Spouse',
          phone: '+1-555-0124',
        },
        allergies: [
          {
            allergen: 'Penicillin',
            severity: 'Severe',
            reaction_description: 'Anaphylaxis',
          },
        ],
      };

      const createResult: ActionHash = await alice.cells[0].callZome({
        zome_name: 'patient',
        fn_name: 'create_patient',
        payload: patient,
      });

      expect(createResult).toBeDefined();

      const retrieved = await alice.cells[0].callZome({
        zome_name: 'patient',
        fn_name: 'get_patient',
        payload: createResult,
      });

      expect(retrieved.patient_id).toBe('PAT-001');
      expect(retrieved.first_name).toBe('Alice');
      expect(retrieved.allergies).toHaveLength(1);
    });
  });

  it('searches patients by name', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice] = await scenario.addPlayersWithApps([
        { appBundleSource: { path: HEALTH_HAPP_PATH } },
      ]);

      // Create multiple patients
      const patients = [
        { patient_id: 'PAT-001', first_name: 'Alice', last_name: 'Johnson' },
        { patient_id: 'PAT-002', first_name: 'Bob', last_name: 'Johnson' },
        { patient_id: 'PAT-003', first_name: 'Charlie', last_name: 'Smith' },
      ];

      for (const p of patients) {
        await alice.cells[0].callZome({
          zome_name: 'patient',
          fn_name: 'create_patient',
          payload: {
            ...p,
            date_of_birth: Date.now() * 1000,
            biological_sex: 'Unknown',
            contact: { email: `${p.first_name.toLowerCase()}@example.com` },
            allergies: [],
          },
        });
      }

      const searchResult = await alice.cells[0].callZome({
        zome_name: 'patient',
        fn_name: 'search_patients_by_name',
        payload: 'Johnson',
      });

      expect(searchResult).toHaveLength(2);
    });
  });
});

// ============================================================================
// Consent Zome Tests (HIPAA Compliance)
// ============================================================================

describe('Consent Zome - HIPAA Compliance', () => {
  it('creates consent with required HIPAA fields', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice, bob] = await scenario.addPlayersWithApps([
        { appBundleSource: { path: HEALTH_HAPP_PATH } },
        { appBundleSource: { path: HEALTH_HAPP_PATH } },
      ]);

      const consent = {
        consent_id: 'CONSENT-001',
        patient_id: 'PAT-001',
        grantee: {
          grantee_type: 'Provider',
          agent_key: encodeHashToBase64(bob.agentPubKey),
          organization_name: 'City General Hospital',
        },
        scope: {
          data_categories: ['Demographics', 'Medications', 'Allergies'],
          permissions: ['Read'],
          time_restriction: null,
        },
        purpose: 'Treatment',
        expires_at: Date.now() * 1000 + 365 * 24 * 60 * 60 * 1000000, // 1 year
        document_hash: 'sha256:consent_doc_hash',
        witness_signatures: [],
      };

      const consentHash: ActionHash = await alice.cells[0].callZome({
        zome_name: 'consent',
        fn_name: 'create_consent',
        payload: consent,
      });

      expect(consentHash).toBeDefined();

      // Bob should be able to check authorization
      const isAuthorized = await bob.cells[0].callZome({
        zome_name: 'consent',
        fn_name: 'check_authorization',
        payload: {
          consent_hash: consentHash,
          requesting_agent: bob.agentPubKey,
          data_category: 'Medications',
          action: 'Read',
        },
      });

      expect(isAuthorized).toBe(true);
    });
  });

  it('prevents access after consent revocation', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice, bob] = await scenario.addPlayersWithApps([
        { appBundleSource: { path: HEALTH_HAPP_PATH } },
        { appBundleSource: { path: HEALTH_HAPP_PATH } },
      ]);

      // Create consent
      const consentHash: ActionHash = await alice.cells[0].callZome({
        zome_name: 'consent',
        fn_name: 'create_consent',
        payload: {
          consent_id: 'CONSENT-002',
          patient_id: 'PAT-001',
          grantee: {
            grantee_type: 'Provider',
            agent_key: encodeHashToBase64(bob.agentPubKey),
          },
          scope: {
            data_categories: ['Demographics'],
            permissions: ['Read'],
          },
          purpose: 'Treatment',
          document_hash: 'sha256:doc_hash',
        },
      });

      // Revoke consent
      await alice.cells[0].callZome({
        zome_name: 'consent',
        fn_name: 'revoke_consent',
        payload: consentHash,
      });

      // Authorization should now fail
      const isAuthorized = await bob.cells[0].callZome({
        zome_name: 'consent',
        fn_name: 'check_authorization',
        payload: {
          consent_hash: consentHash,
          requesting_agent: bob.agentPubKey,
          data_category: 'Demographics',
          action: 'Read',
        },
      });

      expect(isAuthorized).toBe(false);
    });
  });

  it('logs all data access for HIPAA audit trail', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice, bob] = await scenario.addPlayersWithApps([
        { appBundleSource: { path: HEALTH_HAPP_PATH } },
        { appBundleSource: { path: HEALTH_HAPP_PATH } },
      ]);

      const consentHash: ActionHash = await alice.cells[0].callZome({
        zome_name: 'consent',
        fn_name: 'create_consent',
        payload: {
          consent_id: 'CONSENT-003',
          patient_id: 'PAT-001',
          grantee: {
            grantee_type: 'Provider',
            agent_key: encodeHashToBase64(bob.agentPubKey),
          },
          scope: {
            data_categories: ['LabResults'],
            permissions: ['Read'],
          },
          purpose: 'Treatment',
          document_hash: 'sha256:doc_hash',
        },
      });

      // Log data access
      const logEntry: ActionHash = await bob.cells[0].callZome({
        zome_name: 'consent',
        fn_name: 'log_data_access',
        payload: {
          log_id: 'LOG-001',
          consent_hash: consentHash,
          data_category: 'LabResults',
          action: 'Read',
          purpose: 'Treatment',
        },
      });

      expect(logEntry).toBeDefined();

      // Patient should be able to see access logs
      const logs = await alice.cells[0].callZome({
        zome_name: 'consent',
        fn_name: 'get_access_logs_for_patient',
        payload: 'PAT-001',
      });

      expect(logs.length).toBeGreaterThan(0);
    });
  });
});

// ============================================================================
// Clinical Trials Zome Tests
// ============================================================================

describe('Trials Zome - FDA Compliance', () => {
  it('creates a clinical trial with required FDA fields', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice] = await scenario.addPlayersWithApps([
        { appBundleSource: { path: HEALTH_HAPP_PATH } },
      ]);

      const trial = {
        trial_id: 'TRIAL-2024-001',
        nct_number: 'NCT05123456',
        title: 'Phase 3 Randomized Controlled Trial',
        phase: 'Phase3',
        study_type: 'Interventional',
        status: 'Recruiting',
        principal_investigator: encodeHashToBase64(alice.agentPubKey),
        sponsor: 'Luminous Therapeutics',
        irb_approval_number: 'IRB-2024-0123',
        target_enrollment: 500,
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
            description: 'Experimental compound',
            arm_group: 'Treatment',
          },
        ],
        outcomes: [
          {
            outcome_type: 'Primary',
            title: 'Overall Response Rate',
            description: 'Proportion with response',
            time_frame: '12 weeks',
          },
        ],
      };

      const trialHash: ActionHash = await alice.cells[0].callZome({
        zome_name: 'trials',
        fn_name: 'create_trial',
        payload: trial,
      });

      expect(trialHash).toBeDefined();

      const retrieved = await alice.cells[0].callZome({
        zome_name: 'trials',
        fn_name: 'get_trial',
        payload: trialHash,
      });

      expect(retrieved.nct_number).toBe('NCT05123456');
      expect(retrieved.irb_approval_number).toBe('IRB-2024-0123');
    });
  });

  it('checks patient eligibility for trial', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice] = await scenario.addPlayersWithApps([
        { appBundleSource: { path: HEALTH_HAPP_PATH } },
      ]);

      // Create trial with eligibility criteria
      const trialHash: ActionHash = await alice.cells[0].callZome({
        zome_name: 'trials',
        fn_name: 'create_trial',
        payload: {
          trial_id: 'TRIAL-002',
          title: 'Age-restricted trial',
          phase: 'Phase2',
          study_type: 'Interventional',
          status: 'Recruiting',
          principal_investigator: encodeHashToBase64(alice.agentPubKey),
          sponsor: 'Test Sponsor',
          irb_approval_number: 'IRB-002',
          target_enrollment: 100,
          eligibility: {
            min_age: 18,
            max_age: 65,
            sex: 'All',
            healthy_volunteers: false,
            inclusion_criteria: [],
            exclusion_criteria: [],
          },
          interventions: [],
          outcomes: [
            { outcome_type: 'Primary', title: 'Test', description: '', time_frame: '4 weeks' },
          ],
        },
      });

      // Check eligibility for patient age 30
      const eligible = await alice.cells[0].callZome({
        zome_name: 'trials',
        fn_name: 'check_eligibility',
        payload: {
          trial_hash: trialHash,
          patient_age: 30,
        },
      });

      expect(eligible.eligible).toBe(true);

      // Check eligibility for patient age 70 (too old)
      const notEligible = await alice.cells[0].callZome({
        zome_name: 'trials',
        fn_name: 'check_eligibility',
        payload: {
          trial_hash: trialHash,
          patient_age: 70,
        },
      });

      expect(notEligible.eligible).toBe(false);
    });
  });

  it('reports adverse events with required fields', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice] = await scenario.addPlayersWithApps([
        { appBundleSource: { path: HEALTH_HAPP_PATH } },
      ]);

      const ae = {
        event_id: 'AE-001',
        participant_id: 'PART-001',
        trial_id: 'TRIAL-001',
        event_term: 'Headache',
        description: 'Mild headache post-dose',
        onset_date: Date.now() * 1000,
        severity: 'Mild',
        seriousness: [],
        is_serious: false,
        is_unexpected: false,
        causality: 'PossiblyRelated',
        outcome: 'Recovered',
        action_taken: ['DrugNotChanged'],
        medwatch_submitted: false,
      };

      const aeHash: ActionHash = await alice.cells[0].callZome({
        zome_name: 'trials',
        fn_name: 'report_adverse_event',
        payload: ae,
      });

      expect(aeHash).toBeDefined();
    });
  });
});

// ============================================================================
// Bridge Zome Tests - Cross-hApp Communication
// ============================================================================

describe('Bridge Zome - Cross-hApp Federation', () => {
  it('registers health hApp with bridge', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice] = await scenario.addPlayersWithApps([
        { appBundleSource: { path: HEALTH_HAPP_PATH } },
      ]);

      const registration = {
        registration_id: 'REG-HEALTH-001',
        mycelix_identity_hash: encodeHashToBase64(alice.agentPubKey),
        happ_id: 'mycelix-health',
        capabilities: [
          'PatientLookup',
          'ProviderVerification',
          'ConsentVerification',
        ],
        federated_data: ['ProviderCredentials', 'PatientConsent'],
        minimum_trust_score: 0.7,
      };

      const regHash: ActionHash = await alice.cells[0].callZome({
        zome_name: 'bridge',
        fn_name: 'register_with_bridge',
        payload: registration,
      });

      expect(regHash).toBeDefined();
    });
  });

  it('creates epistemic claim with classification', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice] = await scenario.addPlayersWithApps([
        { appBundleSource: { path: HEALTH_HAPP_PATH } },
      ]);

      const claim = {
        claim_id: 'CLAIM-001',
        subject: 'Treatment efficacy',
        claim_type: 'Treatment',
        content: 'Treatment shows 80% response rate',
        empirical_level: 2, // E2 = peer-reviewed
        normative_level: 2, // N2 = network agreed
        materiality_level: 2, // M2 = persistent
        supporting_evidence: ['NCT05123456', 'PMID:12345678'],
      };

      const claimHash: ActionHash = await alice.cells[0].callZome({
        zome_name: 'bridge',
        fn_name: 'create_epistemic_claim',
        payload: claim,
      });

      expect(claimHash).toBeDefined();

      const retrieved = await alice.cells[0].callZome({
        zome_name: 'bridge',
        fn_name: 'get_epistemic_claim',
        payload: claimHash,
      });

      expect(retrieved.empirical_level).toBe(2);
      expect(retrieved.supporting_evidence).toHaveLength(2);
    });
  });

  it('aggregates federated reputation scores', async () => {
    await runScenario(async (scenario: Scenario) => {
      const [alice] = await scenario.addPlayersWithApps([
        { appBundleSource: { path: HEALTH_HAPP_PATH } },
      ]);

      const scores = [
        {
          source_happ: 'mycelix-identity',
          score: 0.95,
          weight: 0.25,
          score_type: 'verification',
        },
        {
          source_happ: 'mycelix-health',
          score: 0.88,
          weight: 0.30,
          score_type: 'patient_outcomes',
        },
        {
          source_happ: 'mycelix-health',
          score: 0.82,
          weight: 0.20,
          score_type: 'peer_attestations',
        },
      ];

      const aggregated = await alice.cells[0].callZome({
        zome_name: 'bridge',
        fn_name: 'aggregate_reputation',
        payload: {
          entity_hash: alice.agentPubKey,
          entity_type: 'Provider',
          scores,
        },
      });

      expect(aggregated.aggregated_score).toBeGreaterThan(0);
      expect(aggregated.aggregated_score).toBeLessThanOrEqual(1);
    });
  });
});
