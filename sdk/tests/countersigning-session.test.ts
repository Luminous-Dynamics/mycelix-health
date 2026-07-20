import {
  CountersigningSessionStateType,
  SessionCompletionDecisionType,
  encodeHashToBase64,
  hashFrom32AndType,
  type CellId,
  type CountersigningSessionState,
  type PreflightRequest,
} from '@holochain/client';
import { describe, expect, it } from 'vitest';
import {
  executeCountersigningRecoveryV1,
  inspectCountersigningParticipantV1,
  reconcileCountersigningParticipantsV1,
  recommendCountersigningRecoveryV1,
  type CountersigningSessionAdminClientV1,
} from '../src/countersigning/native-session';
import {
  encodeClinicalCountersigningIntentV1,
  type ClinicalCountersigningIntentV1,
} from '../src/crypto/native-countersigning';

class MockSessionClient implements CountersigningSessionAdminClientV1 {
  published = 0;
  abandoned = 0;
  constructor(public state: CountersigningSessionState | null) {}
  async getCountersigningSessionState(): Promise<CountersigningSessionState | null> { return this.state; }
  async abandonCountersigningSession(): Promise<null> { this.abandoned += 1; return null; }
  async publishCountersigningSession(): Promise<null> { this.published += 1; return null; }
}

function agent(fill: number): Uint8Array { return hashFrom32AndType(new Uint8Array(32).fill(fill), 'Agent'); }
function cell(agentBytes: Uint8Array): CellId { return [hashFrom32AndType(new Uint8Array(32).fill(99), 'Dna'), agentBytes]; }

async function intentAndPreflight(): Promise<{ intent: ClinicalCountersigningIntentV1; preflight: PreflightRequest }> {
  const patient = agent(1);
  const provider = agent(2);
  const intent: ClinicalCountersigningIntentV1 = {
    countersigning_version: 1,
    proposal: {
      approval_version: 1,
      operation_class: 'sensitive_record_release',
      patient_binding: encodeHashToBase64(hashFrom32AndType(new Uint8Array(32).fill(3), 'Action')),
      provider_binding: encodeHashToBase64(provider),
      mutation_binding_digest: new Uint8Array(32).fill(4),
      deployment_evidence_sha256: new Uint8Array(32).fill(5),
      policy_digest: new Uint8Array(32).fill(6),
      rationale_sha256: new Uint8Array(32).fill(7),
      created_at_ms: 1_000n,
      expires_at_ms: 301_000n,
      nonce: new Uint8Array(32).fill(8),
    },
    participants: [
      { role: 'patient', agent_binding: encodeHashToBase64(patient), agent_bytes: patient },
      { role: 'provider', agent_binding: encodeHashToBase64(provider), agent_bytes: provider },
    ],
    session_start_ms: 10_000n,
    session_end_ms: 130_000n,
    nonce: new Uint8Array(32).fill(9),
  };
  return {
    intent,
    preflight: {
      app_entry_hash: hashFrom32AndType(new Uint8Array(32).fill(10), 'Entry'),
      signing_agents: [[patient, [1]], [provider, [2]]],
      enzyme_index: undefined,
      session_times: { start: 10_000_000n, end: 130_000_000n },
      action_base: { Create: { entry_type: { App: { entry_index: 7, zome_index: 5, visibility: 'Private' } } } },
      preflight_bytes: await encodeClinicalCountersigningIntentV1(intent),
    },
  };
}

function unknownState(preflight: PreflightRequest, decisions: unknown[], attempts = 2): CountersigningSessionState {
  return {
    [CountersigningSessionStateType.Unknown]: {
      preflight_request: preflight,
      resolution: {
        required_reason: 'Timeout' as never,
        attempts,
        outcomes: [{ agent: preflight.signing_agents[0]![0], decisions: decisions as never }],
      },
      force_abandon: false,
      force_publish: false,
    },
  };
}

describe('native countersigning recovery orchestration', () => {
  it('waits while a session is accepted', async () => {
    const { preflight } = await intentAndPreflight();
    expect(recommendCountersigningRecoveryV1({ [CountersigningSessionStateType.Accepted]: preflight }).action).toBe('wait');
  });

  it('waits while signatures are being collected', async () => {
    const { preflight } = await intentAndPreflight();
    const state: CountersigningSessionState = { [CountersigningSessionStateType.SignaturesCollected]: { preflight_request: preflight, signature_bundles: [] } };
    expect(recommendCountersigningRecoveryV1(state).action).toBe('wait');
  });

  it('does not infer completion from a missing active session', () => {
    expect(recommendCountersigningRecoveryV1(null)).toMatchObject({ action: 'manual_review' });
  });

  it('requires explicit policy before forced publication', async () => {
    const { preflight } = await intentAndPreflight();
    const complete = unknownState(preflight, [{ Complete: {} }]);
    expect(recommendCountersigningRecoveryV1(complete).action).toBe('manual_review');
    expect(recommendCountersigningRecoveryV1(complete, { allow_force_publish: true, allow_force_abandon: false, minimum_resolution_attempts: 1 }).action).toBe('force_publish');
  });

  it('requires explicit policy before forced abandonment', async () => {
    const { preflight } = await intentAndPreflight();
    const abandoned = unknownState(preflight, [SessionCompletionDecisionType.Abandoned]);
    expect(recommendCountersigningRecoveryV1(abandoned).action).toBe('manual_review');
    expect(recommendCountersigningRecoveryV1(abandoned, { allow_force_publish: false, allow_force_abandon: true, minimum_resolution_attempts: 1 }).action).toBe('force_abandon');
  });

  it('routes conflicting, indeterminate, and failed authority outcomes to review', async () => {
    const { preflight } = await intentAndPreflight();
    expect(recommendCountersigningRecoveryV1(unknownState(preflight, [{ Complete: {} }, SessionCompletionDecisionType.Abandoned])).action).toBe('manual_review');
    expect(recommendCountersigningRecoveryV1(unknownState(preflight, [SessionCompletionDecisionType.Indeterminate])).action).toBe('manual_review');
    expect(recommendCountersigningRecoveryV1(unknownState(preflight, [SessionCompletionDecisionType.Failed])).action).toBe('manual_review');
  });

  it('waits until the configured recovery-attempt threshold is met', async () => {
    const { preflight } = await intentAndPreflight();
    const state = unknownState(preflight, [{ Complete: {} }], 1);
    expect(recommendCountersigningRecoveryV1(state, { allow_force_publish: true, allow_force_abandon: false, minimum_resolution_attempts: 2 }).action).toBe('wait');
  });

  it('inspects a participant and emits a non-terminal observation', async () => {
    const { intent, preflight } = await intentAndPreflight();
    const client = new MockSessionClient({ [CountersigningSessionStateType.Accepted]: preflight });
    const inspected = await inspectCountersigningParticipantV1({ observer_id: 'patient-conductor', participant_binding: intent.participants[0]!.agent_binding, cell_id: cell(Uint8Array.from(intent.participants[0]!.agent_bytes)), client }, intent, 20_000n, new Uint8Array(32).fill(20));
    expect(inspected.observation?.state).toBe('accepted');
    expect(inspected.recommended_action).toBe('wait');
  });

  it('detects split-view preflights across participants', async () => {
    const { intent, preflight } = await intentAndPreflight();
    const altered: PreflightRequest = { ...preflight, app_entry_hash: hashFrom32AndType(new Uint8Array(32).fill(55), 'Entry') };
    const clients = [new MockSessionClient({ [CountersigningSessionStateType.Accepted]: preflight }), new MockSessionClient({ [CountersigningSessionStateType.Accepted]: altered })];
    const report = await reconcileCountersigningParticipantsV1(intent.participants.map((participant, index) => ({ observer_id: `conductor-${index}`, participant_binding: participant.agent_binding, cell_id: cell(Uint8Array.from(participant.agent_bytes)), client: clients[index]! })), intent, 20_000n, (index) => new Uint8Array(32).fill(30 + index));
    expect(report.has_conflicting_preflight).toBe(true);
    expect(report.recommended_action).toBe('manual_review');
  });

  it('routes missing participant state to manual review', async () => {
    const { intent, preflight } = await intentAndPreflight();
    const clients = [new MockSessionClient({ [CountersigningSessionStateType.Accepted]: preflight }), new MockSessionClient(null)];
    const report = await reconcileCountersigningParticipantsV1(intent.participants.map((participant, index) => ({ observer_id: `conductor-${index}`, participant_binding: participant.agent_binding, cell_id: cell(Uint8Array.from(participant.agent_bytes)), client: clients[index]! })), intent, 20_000n, (index) => new Uint8Array(32).fill(40 + index));
    expect(report.recommended_action).toBe('manual_review');
    expect(report.has_missing_state).toBe(true);
  });

  it('executes only explicit forced decisions', async () => {
    const { intent } = await intentAndPreflight();
    const client = new MockSessionClient(null);
    const runtime = { observer_id: 'patient-conductor', participant_binding: intent.participants[0]!.agent_binding, cell_id: cell(Uint8Array.from(intent.participants[0]!.agent_bytes)), client };
    await executeCountersigningRecoveryV1(runtime, 'force_publish');
    await executeCountersigningRecoveryV1(runtime, 'force_abandon');
    expect(client.published).toBe(1);
    expect(client.abandoned).toBe(1);
    await expect(executeCountersigningRecoveryV1(runtime, 'wait')).rejects.toThrow(/explicit/i);
  });
});
