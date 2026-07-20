import blake2b from '@bitgo/blake2b';
import {
  CountersigningSessionStateType,
  encodeHashToBase64,
  hashFrom32AndType,
  type CountersigningSessionState,
  type PreflightRequest,
} from '@holochain/client';
import { encode } from '@msgpack/msgpack';
import { describe, expect, it } from 'vitest';
import {
  clinicalCountersigningIntentDigestV1,
  createTerminalCountersigningObservationV1,
  encodeClinicalCountersigningIntentV1,
  observeCountersigningSessionStateV1,
  verifyNativeClinicalCountersigningEvidenceV1,
  type ClinicalCountersigningIntentV1,
  type NativeClinicalCountersigningEvidenceV1,
} from '../src/crypto/native-countersigning';
import type { ClinicalApprovalProposalV1, ClinicalApprovalRoleV1 } from '../src/crypto/governed-validation';
import type { PortableChainActionV1 } from '../src/crypto/source-chain-evidence';

type TestKeypair = { publicKey: Uint8Array; privateKey: CryptoKey };
type ParticipantFixture = {
  role: ClinicalApprovalRoleV1;
  keypair: TestKeypair;
  agent: Uint8Array;
  binding: string;
};

function arrayBuffer(value: Uint8Array): ArrayBuffer { return Uint8Array.from(value).buffer; }
function blake2b32(value: Uint8Array): Uint8Array {
  const out = new Uint8Array(32);
  blake2b(32).update(value).digest(out);
  return out;
}
async function generateKeypair(): Promise<TestKeypair> {
  const generated = await crypto.subtle.generateKey('Ed25519', true, ['sign', 'verify']);
  return {
    publicKey: new Uint8Array(await crypto.subtle.exportKey('raw', generated.publicKey)),
    privateKey: generated.privateKey,
  };
}
async function participant(role: ClinicalApprovalRoleV1): Promise<ParticipantFixture> {
  const keypair = await generateKeypair();
  const agent = hashFrom32AndType(keypair.publicKey, 'Agent');
  return { role, keypair, agent, binding: encodeHashToBase64(agent) };
}
function proposal(operation: ClinicalApprovalProposalV1['operation_class']): ClinicalApprovalProposalV1 {
  return {
    approval_version: 1,
    operation_class: operation,
    patient_binding: encodeHashToBase64(hashFrom32AndType(new Uint8Array(32).fill(40), 'Action')),
    provider_binding: encodeHashToBase64(hashFrom32AndType(new Uint8Array(32).fill(41), 'Agent')),
    mutation_binding_digest: new Uint8Array(32).fill(1),
    deployment_evidence_sha256: new Uint8Array(32).fill(2),
    policy_digest: new Uint8Array(32).fill(3),
    rationale_sha256: new Uint8Array(32).fill(4),
    created_at_ms: 1_700_000_000_000n,
    expires_at_ms: 1_700_000_300_000n,
    nonce: new Uint8Array(32).fill(5),
  };
}

async function signedCountersignedCreate(
  participantFixture: ParticipantFixture,
  countersignedEntryBytes: Uint8Array,
  sequence: number,
  timestampMicros: bigint,
): Promise<PortableChainActionV1> {
  const entryHash = hashFrom32AndType(blake2b32(countersignedEntryBytes), 'Entry');
  const previous = hashFrom32AndType(new Uint8Array(32).fill(sequence + 1), 'Action');
  const actionBytes = encode({
    type: 'Create',
    author: participantFixture.agent,
    timestamp: timestampMicros,
    action_seq: sequence,
    prev_action: previous,
    entry_type: { App: { entry_index: 7, zome_index: 5, visibility: 'Private' } },
    entry_hash: entryHash,
  }, { useBigInt64: true });
  const actionHash = hashFrom32AndType(blake2b32(actionBytes), 'Action');
  return {
    action_hash: encodeHashToBase64(actionHash),
    action_hash_bytes: actionHash,
    previous_action_hash: encodeHashToBase64(previous),
    previous_action_hash_bytes: previous,
    author_binding: participantFixture.binding,
    author_bytes: participantFixture.agent,
    signer_binding: participantFixture.binding,
    signer_bytes: participantFixture.agent,
    action_bytes: actionBytes,
    signature_bytes: new Uint8Array(await crypto.subtle.sign('Ed25519', participantFixture.keypair.privateKey, arrayBuffer(actionBytes))),
    entry_hash_bytes: entryHash,
    entry_bytes: countersignedEntryBytes,
    action_seq: sequence,
    timestamp_micros: timestampMicros,
  };
}

async function fixture(operation: ClinicalApprovalProposalV1['operation_class'] = 'sensitive_record_release'): Promise<{
  evidence: NativeClinicalCountersigningEvidenceV1;
  preflight: PreflightRequest;
  participants: ParticipantFixture[];
}> {
  const roles: ClinicalApprovalRoleV1[] = operation === 'sensitive_record_release'
    ? ['patient', 'provider']
    : operation === 'provider_correction'
      ? ['provider', 'reviewer']
      : ['patient', 'provider', 'reviewer'];
  const participants = await Promise.all(roles.map(participant));
  const intent: ClinicalCountersigningIntentV1 = {
    countersigning_version: 1,
    proposal: proposal(operation),
    participants: participants.map((item) => ({ role: item.role, agent_binding: item.binding, agent_bytes: item.agent })),
    session_start_ms: 1_700_000_010_000n,
    session_end_ms: 1_700_000_130_000n,
    nonce: new Uint8Array(32).fill(9),
  };
  const appPayload = encode(intent.proposal, { useBigInt64: true });
  const appEntryBytes = encode({ entry_type: 'App', entry: appPayload }, { useBigInt64: true });
  const appEntryHash = hashFrom32AndType(blake2b32(appEntryBytes), 'Entry');
  const preflight: PreflightRequest = {
    app_entry_hash: appEntryHash,
    signing_agents: participants.map((item) => [item.agent, [item.role === 'patient' ? 1 : item.role === 'provider' ? 2 : 3]]),
    enzyme_index: undefined,
    session_times: {
      start: BigInt(intent.session_start_ms) * 1000n,
      end: BigInt(intent.session_end_ms) * 1000n,
    },
    action_base: { Create: { entry_type: { App: { entry_index: 7, zome_index: 5, visibility: 'Private' } } } },
    preflight_bytes: await encodeClinicalCountersigningIntentV1(intent),
  };
  const preflightRequestBytes = encode(preflight, { useBigInt64: true, ignoreUndefined: true });
  const session = {
    preflight_request: preflight,
    responses: participants.map((_, index) => [
      {
        agent_index: index,
        chain_top: hashFrom32AndType(new Uint8Array(32).fill(70 + index), 'Action'),
        action_seq: 100 + index,
      },
      new Uint8Array(64).fill(80 + index),
    ]),
  };
  const countersignedEntryBytes = encode({ entry_type: 'CounterSign', entry: [session, appPayload] }, { useBigInt64: true, ignoreUndefined: true });
  const countersignedEntryHash = hashFrom32AndType(blake2b32(countersignedEntryBytes), 'Entry');
  const participantActions = await Promise.all(participants.map((item, index) => signedCountersignedCreate(
    item,
    countersignedEntryBytes,
    200 + index,
    (BigInt(intent.session_start_ms) + BigInt(1_000 + index)) * 1000n,
  )));
  return {
    participants,
    preflight,
    evidence: {
      evidence_version: 1,
      intent,
      preflight_request_bytes: preflightRequestBytes,
      app_entry_hash_binding: encodeHashToBase64(appEntryHash),
      app_entry_hash_bytes: appEntryHash,
      app_entry_bytes: appEntryBytes,
      countersigned_entry_hash_binding: encodeHashToBase64(countersignedEntryHash),
      countersigned_entry_hash_bytes: countersignedEntryHash,
      countersigned_entry_bytes: countersignedEntryBytes,
      participant_actions: participantActions,
    },
  };
}

describe('native clinical countersigning evidence', () => {
  it('verifies patient/provider sensitive-release countersigning', async () => {
    const { evidence } = await fixture();
    const verified = await verifyNativeClinicalCountersigningEvidenceV1(evidence);
    expect(verified.participants).toHaveLength(2);
    expect(verified.participant_action_hashes).toHaveLength(2);
  });

  it('verifies provider/reviewer correction countersigning', async () => {
    const { evidence } = await fixture('provider_correction');
    await expect(verifyNativeClinicalCountersigningEvidenceV1(evidence)).resolves.toMatchObject({ participants: expect.any(Array) });
  });

  it('requires patient/provider/reviewer for break-glass acknowledgement', async () => {
    const { evidence } = await fixture('break_glass_acknowledgement');
    const verified = await verifyNativeClinicalCountersigningEvidenceV1(evidence);
    expect(verified.participants).toHaveLength(3);
  });

  it('rejects substituted preflight bytes', async () => {
    const { evidence } = await fixture();
    evidence.preflight_request_bytes = Uint8Array.from(evidence.preflight_request_bytes);
    (evidence.preflight_request_bytes as Uint8Array)[10] ^= 1;
    await expect(verifyNativeClinicalCountersigningEvidenceV1(evidence)).rejects.toThrow(/preflight|malformed|decode/i);
  });

  it('rejects substituted app entry bytes', async () => {
    const { evidence } = await fixture();
    evidence.app_entry_bytes = Uint8Array.from(evidence.app_entry_bytes);
    (evidence.app_entry_bytes as Uint8Array)[5] ^= 1;
    await expect(verifyNativeClinicalCountersigningEvidenceV1(evidence)).rejects.toThrow(/hash/i);
  });

  it('rejects a missing participant action', async () => {
    const { evidence } = await fixture();
    evidence.participant_actions.pop();
    await expect(verifyNativeClinicalCountersigningEvidenceV1(evidence)).rejects.toThrow(/count|participant/i);
  });

  it('rejects an action authored outside the session window', async () => {
    const { evidence, participants } = await fixture();
    evidence.participant_actions[0] = await signedCountersignedCreate(
      participants[0]!,
      Uint8Array.from(evidence.countersigned_entry_bytes),
      200,
      (BigInt(evidence.intent.session_end_ms) + 1n) * 1000n,
    );
    await expect(verifyNativeClinicalCountersigningEvidenceV1(evidence)).rejects.toThrow(/window/i);
  });

  it('rejects a signer that is not the source-chain author', async () => {
    const { evidence } = await fixture();
    const other = await participant('provider');
    const action = evidence.participant_actions[0]!;
    action.signer_binding = other.binding;
    action.signer_bytes = other.agent;
    action.signature_bytes = new Uint8Array(await crypto.subtle.sign('Ed25519', other.keypair.privateKey, arrayBuffer(Uint8Array.from(action.action_bytes))));
    await expect(verifyNativeClinicalCountersigningEvidenceV1(evidence)).rejects.toThrow(/author identity/i);
  });

  it('observes accepted and unknown conductor states without guessing a terminal result', async () => {
    const { evidence, preflight } = await fixture();
    const accepted: CountersigningSessionState = { [CountersigningSessionStateType.Accepted]: preflight };
    const acceptedObservation = await observeCountersigningSessionStateV1('patient-conductor', evidence.intent.participants[0]!.agent_binding, evidence.intent, accepted, 1_700_000_020_000n, new Uint8Array(32).fill(11));
    expect(acceptedObservation.state).toBe('accepted');

    const unknown: CountersigningSessionState = {
      [CountersigningSessionStateType.Unknown]: {
        preflight_request: preflight,
        resolution: { required_reason: 'Timeout' as never, attempts: 2, outcomes: [] },
        force_abandon: false,
        force_publish: false,
      },
    };
    const unknownObservation = await observeCountersigningSessionStateV1('patient-conductor', evidence.intent.participants[0]!.agent_binding, evidence.intent, unknown, 1_700_000_140_000n, new Uint8Array(32).fill(12));
    expect(unknownObservation.state).toBe('unknown');
  });

  it('requires explicit action evidence for a completed terminal observation', async () => {
    const { evidence } = await fixture();
    await expect(createTerminalCountersigningObservationV1('observer', evidence.intent.participants[0]!.agent_binding, evidence.intent, 'completed', null, 1_700_000_140_000n, new Uint8Array(32).fill(13))).rejects.toThrow(/action hash/i);
    const observation = await createTerminalCountersigningObservationV1('observer', evidence.intent.participants[0]!.agent_binding, evidence.intent, 'completed', evidence.participant_actions[0]!.action_hash, 1_700_000_140_000n, new Uint8Array(32).fill(14));
    expect(observation.state).toBe('completed');
    expect(await clinicalCountersigningIntentDigestV1(evidence.intent)).toEqual(Uint8Array.from(observation.intent_digest));
  });
});
