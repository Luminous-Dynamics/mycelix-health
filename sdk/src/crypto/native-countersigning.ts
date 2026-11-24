// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Native clinical countersigning evidence contracts and verifier.
 *
 * This module verifies a supplied, already-materialized Holochain evidence
 * envelope. It does not call a conductor, mint authority, check consent policy,
 * establish provider credentials, or prove that a conductor was compiled with
 * the unstable countersigning feature enabled.
 *
 * A successful result proves only the structural and cryptographic properties
 * explicitly checked below for this evidence envelope.
 */

import blake2b from "@bitgo/blake2b";
import {
  CountersigningSessionStateType,
  encodeHashToBase64,
  hashFrom32AndType,
  type CountersigningSessionState,
  type PreflightRequest,
} from "@holochain/client";
import { decode, encode } from "@msgpack/msgpack";
import type {
  ClinicalApprovalOperationClassV1,
  ClinicalApprovalProposalV1,
  ClinicalApprovalRoleV1,
} from "./governed-validation";
import type { PortableChainActionV1 } from "./source-chain-evidence";

const INTENT_DOMAIN = "MYCELIX-HEALTH-CLINICAL-COUNTERSIGNING-INTENT-V1";
const INTENT_DIGEST_DOMAIN = "MYCELIX-HEALTH-CLINICAL-COUNTERSIGNING-DIGEST-V1";
const REQUIRED_COMMITMENT_BYTES = 32;
const HOLOHASH_BYTES = 39;

export interface ClinicalCountersigningParticipantV1 {
  role: ClinicalApprovalRoleV1;
  agent_binding: string;
  agent_bytes: Uint8Array;
}

export interface ClinicalCountersigningIntentV1 {
  countersigning_version: 1;
  proposal: ClinicalApprovalProposalV1;
  participants: ClinicalCountersigningParticipantV1[];
  session_start_ms: bigint;
  session_end_ms: bigint;
  nonce: Uint8Array;
}

export interface NativeClinicalCountersigningEvidenceV1 {
  evidence_version: 1;
  intent: ClinicalCountersigningIntentV1;
  preflight_request_bytes: Uint8Array;
  app_entry_hash_binding: string;
  app_entry_hash_bytes: Uint8Array;
  app_entry_bytes: Uint8Array;
  countersigned_entry_hash_binding: string;
  countersigned_entry_hash_bytes: Uint8Array;
  countersigned_entry_bytes: Uint8Array;
  participant_actions: PortableChainActionV1[];
}

export interface VerifiedClinicalCountersigningParticipantV1 {
  role: ClinicalApprovalRoleV1;
  agent_binding: string;
  action_hash: string;
  action_seq: number;
  timestamp_micros: bigint;
}

export interface VerifiedNativeClinicalCountersigningEvidenceV1 {
  evidence_version: 1;
  intent_digest: Uint8Array;
  app_entry_hash_binding: string;
  countersigned_entry_hash_binding: string;
  participants: VerifiedClinicalCountersigningParticipantV1[];
  participant_action_hashes: string[];
}

export type NativeCountersigningObservationStateV1 =
  | "missing"
  | "accepted"
  | "signatures_collected"
  | "unknown";

export interface NativeCountersigningSessionObservationV1 {
  observation_version: 1;
  observer_id: string;
  participant_binding: string;
  state: NativeCountersigningObservationStateV1;
  intent_digest: Uint8Array;
  observed_at_ms: bigint;
  nonce: Uint8Array;
  preflight_matches_intent: boolean | null;
  /** Always false: observation alone never establishes a terminal outcome. */
  terminal: false;
}

export interface NativeTerminalCountersigningObservationV1 {
  observation_version: 1;
  observer_id: string;
  participant_binding: string;
  state: "completed" | "abandoned";
  intent_digest: Uint8Array;
  action_hash_binding: string | null;
  observed_at_ms: bigint;
  nonce: Uint8Array;
  terminal: true;
}

type UnknownRecord = Record<string, unknown>;

function asRecord(value: unknown): UnknownRecord | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? value as UnknownRecord
    : null;
}

function isBytes(value: unknown): value is Uint8Array {
  return value instanceof Uint8Array;
}

function bytesEqual(left: Uint8Array, right: Uint8Array): boolean {
  if (left.byteLength !== right.byteLength) return false;
  let difference = 0;
  for (let index = 0; index < left.byteLength; index += 1) {
    difference |= left[index]! ^ right[index]!;
  }
  return difference === 0;
}

function copyBytes(value: Uint8Array): Uint8Array {
  return Uint8Array.from(value);
}

function concatBytes(...values: Uint8Array[]): Uint8Array {
  const length = values.reduce((sum, value) => sum + value.byteLength, 0);
  const combined = new Uint8Array(length);
  let offset = 0;
  for (const value of values) {
    combined.set(value, offset);
    offset += value.byteLength;
  }
  return combined;
}

function blake2b32(value: Uint8Array): Uint8Array {
  const output = new Uint8Array(32);
  blake2b(32).update(value).digest(output);
  return output;
}

function expectBytes(value: unknown, length: number, label: string): Uint8Array {
  if (!isBytes(value) || value.byteLength !== length) {
    throw new Error(`${label} must be exactly ${length} bytes`);
  }
  return value;
}

function expectHash(value: unknown, label: string): Uint8Array {
  return expectBytes(value, HOLOHASH_BYTES, label);
}

function expectNonEmpty(value: unknown, label: string): string {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`${label} must be a non-empty string`);
  }
  return value;
}

function rolesForOperation(operation: ClinicalApprovalOperationClassV1): ClinicalApprovalRoleV1[] {
  switch (operation) {
    case "sensitive_record_release":
      return ["patient", "provider"];
    case "provider_correction":
      return ["provider", "reviewer"];
    case "break_glass_acknowledgement":
      return ["patient", "provider", "reviewer"];
    default: {
      const unreachable: never = operation;
      throw new Error(`Unsupported clinical approval operation: ${String(unreachable)}`);
    }
  }
}

function validateProposal(proposal: ClinicalApprovalProposalV1): void {
  if (proposal.approval_version !== 1) throw new Error("Unsupported clinical approval proposal version");
  expectNonEmpty(proposal.patient_binding, "patient_binding");
  expectNonEmpty(proposal.provider_binding, "provider_binding");
  expectBytes(proposal.mutation_binding_digest, REQUIRED_COMMITMENT_BYTES, "mutation_binding_digest");
  expectBytes(proposal.deployment_evidence_sha256, REQUIRED_COMMITMENT_BYTES, "deployment_evidence_sha256");
  expectBytes(proposal.policy_digest, REQUIRED_COMMITMENT_BYTES, "policy_digest");
  expectBytes(proposal.rationale_sha256, REQUIRED_COMMITMENT_BYTES, "rationale_sha256");
  expectBytes(proposal.nonce, REQUIRED_COMMITMENT_BYTES, "proposal nonce");
  if (proposal.created_at_ms < 0n || proposal.expires_at_ms <= proposal.created_at_ms) {
    throw new Error("Clinical approval proposal validity window is invalid");
  }
}

function validateIntent(intent: ClinicalCountersigningIntentV1): void {
  if (intent.countersigning_version !== 1) throw new Error("Unsupported clinical countersigning version");
  validateProposal(intent.proposal);
  expectBytes(intent.nonce, REQUIRED_COMMITMENT_BYTES, "intent nonce");

  if (intent.session_start_ms < intent.proposal.created_at_ms
    || intent.session_end_ms > intent.proposal.expires_at_ms
    || intent.session_end_ms <= intent.session_start_ms) {
    throw new Error("Countersigning session window is outside proposal validity");
  }

  const expectedRoles = rolesForOperation(intent.proposal.operation_class);
  if (intent.participants.length !== expectedRoles.length) {
    throw new Error("Participant count does not match operation-class requirements");
  }

  const seenRoles = new Set<string>();
  const seenBindings = new Set<string>();
  for (const participant of intent.participants) {
    if (!expectedRoles.includes(participant.role)) {
      throw new Error(`Unexpected participant role ${participant.role}`);
    }
    if (seenRoles.has(participant.role)) throw new Error(`Duplicate participant role ${participant.role}`);
    seenRoles.add(participant.role);

    expectNonEmpty(participant.agent_binding, "participant agent binding");
    const agentBytes = expectHash(participant.agent_bytes, "participant AgentPubKey");
    const canonicalBinding = encodeHashToBase64(agentBytes);
    if (canonicalBinding !== participant.agent_binding) {
      throw new Error("Participant agent binding does not match AgentPubKey bytes");
    }
    if (seenBindings.has(canonicalBinding)) throw new Error("Duplicate participant agent identity");
    seenBindings.add(canonicalBinding);
  }

  for (const role of expectedRoles) {
    if (!seenRoles.has(role)) throw new Error(`Required participant role ${role} is missing`);
  }

  const provider = intent.participants.find((participant) => participant.role === "provider");
  if (provider && provider.agent_binding !== intent.proposal.provider_binding) {
    throw new Error("Provider participant does not match proposal provider binding");
  }
}

/**
 * Produce deterministic, domain-separated MessagePack bytes for the signed
 * clinical approval intent. The caller must store these exact bytes; do not
 * reconstruct them using an unpinned serialization profile.
 */
export async function encodeClinicalCountersigningIntentV1(
  intent: ClinicalCountersigningIntentV1,
): Promise<Uint8Array> {
  validateIntent(intent);
  const transcript = {
    domain: INTENT_DOMAIN,
    countersigning_version: intent.countersigning_version,
    proposal: {
      approval_version: intent.proposal.approval_version,
      operation_class: intent.proposal.operation_class,
      patient_binding: intent.proposal.patient_binding,
      provider_binding: intent.proposal.provider_binding,
      mutation_binding_digest: copyBytes(intent.proposal.mutation_binding_digest),
      deployment_evidence_sha256: copyBytes(intent.proposal.deployment_evidence_sha256),
      policy_digest: copyBytes(intent.proposal.policy_digest),
      rationale_sha256: copyBytes(intent.proposal.rationale_sha256),
      created_at_ms: intent.proposal.created_at_ms,
      expires_at_ms: intent.proposal.expires_at_ms,
      nonce: copyBytes(intent.proposal.nonce),
    },
    participants: intent.participants.map((participant) => ({
      role: participant.role,
      agent_binding: participant.agent_binding,
      agent_bytes: copyBytes(participant.agent_bytes),
    })),
    session_start_ms: intent.session_start_ms,
    session_end_ms: intent.session_end_ms,
    nonce: copyBytes(intent.nonce),
  };
  return encode(transcript, { useBigInt64: true });
}

export async function clinicalCountersigningIntentDigestV1(
  intent: ClinicalCountersigningIntentV1,
): Promise<Uint8Array> {
  const transcript = await encodeClinicalCountersigningIntentV1(intent);
  const domain = new TextEncoder().encode(`${INTENT_DIGEST_DOMAIN}\0`);
  const digest = await crypto.subtle.digest("SHA-256", concatBytes(domain, transcript));
  return new Uint8Array(digest);
}

function decodeMessagePack(bytes: Uint8Array, label: string): unknown {
  try {
    return decode(bytes);
  } catch (error) {
    throw new Error(`${label} MessagePack is malformed: ${error instanceof Error ? error.message : String(error)}`);
  }
}

function getCreateActionBase(preflight: UnknownRecord): UnknownRecord {
  const actionBase = asRecord(preflight.action_base);
  const create = actionBase ? asRecord(actionBase.Create) : null;
  const entryType = create ? asRecord(create.entry_type) : null;
  const app = entryType ? asRecord(entryType.App) : null;
  if (!app || app.visibility !== "Private") {
    throw new Error("Countersigned clinical action base must be a private App entry");
  }
  return app;
}

function assertPreflightMatchesIntentBytes(
  intent: ClinicalCountersigningIntentV1,
  preflight: UnknownRecord,
  intentBytes: Uint8Array,
): void {
  const preflightBytes = preflight.preflight_bytes;
  if (!isBytes(preflightBytes) || !bytesEqual(preflightBytes, intentBytes)) {
    throw new Error("Preflight bytes do not match the exact clinical countersigning intent");
  }

  const sessionTimes = asRecord(preflight.session_times);
  if (!sessionTimes
    || typeof sessionTimes.start !== "bigint"
    || typeof sessionTimes.end !== "bigint"
    || sessionTimes.start !== intent.session_start_ms * 1000n
    || sessionTimes.end !== intent.session_end_ms * 1000n) {
    throw new Error("Preflight session window does not match the clinical intent");
  }

  const signingAgents = preflight.signing_agents;
  if (!Array.isArray(signingAgents) || signingAgents.length !== intent.participants.length) {
    throw new Error("Preflight signer count does not match clinical participants");
  }

  const roleIndex: Record<ClinicalApprovalRoleV1, number> = {
    patient: 1,
    provider: 2,
    reviewer: 3,
  };
  intent.participants.forEach((participant, index) => {
    const signer = signingAgents[index];
    if (!Array.isArray(signer) || signer.length !== 2
      || !isBytes(signer[0]) || !bytesEqual(signer[0], participant.agent_bytes)
      || !Array.isArray(signer[1]) || !signer[1].includes(roleIndex[participant.role])) {
      throw new Error(`Preflight signer binding mismatch for participant ${index}`);
    }
  });
  getCreateActionBase(preflight);
}

async function verifyEd25519(
  publicKeyBytes: Uint8Array,
  signatureBytes: Uint8Array,
  messageBytes: Uint8Array,
): Promise<boolean> {
  if (signatureBytes.byteLength !== 64 || publicKeyBytes.byteLength < 32) return false;
  try {
    const rawPublicKey = publicKeyBytes.slice(-32);
    const key = await crypto.subtle.importKey("raw", rawPublicKey, { name: "Ed25519" }, false, ["verify"]);
    return crypto.subtle.verify(
      { name: "Ed25519" },
      key,
      signatureBytes,
      messageBytes,
    );
  } catch {
    return false;
  }
}

/**
 * Verify that a native countersigned entry and every participant's source-chain
 * Create action are byte-bound, correctly hashed, authored by the declared
 * participant and signed by the exact author's Ed25519 key.
 *
 * This does not authenticate off-chain provider credentials or establish that
 * the remote conductor used an audited/qualified countersigning implementation.
 */
export async function verifyNativeClinicalCountersigningEvidenceV1(
  evidence: NativeClinicalCountersigningEvidenceV1,
): Promise<VerifiedNativeClinicalCountersigningEvidenceV1> {
  if (evidence.evidence_version !== 1) throw new Error("Unsupported native countersigning evidence version");
  validateIntent(evidence.intent);
  const intentBytes = await encodeClinicalCountersigningIntentV1(evidence.intent);
  const decodedPreflight = asRecord(decodeMessagePack(evidence.preflight_request_bytes, "Preflight request"));
  if (!decodedPreflight) throw new Error("Preflight request is not a MessagePack map");
  assertPreflightMatchesIntentBytes(evidence.intent, decodedPreflight, intentBytes);

  const appEntryHash = expectHash(evidence.app_entry_hash_bytes, "App entry hash");
  if (encodeHashToBase64(appEntryHash) !== evidence.app_entry_hash_binding) {
    throw new Error("App entry hash binding mismatch");
  }
  const computedAppEntryHash = hashFrom32AndType(blake2b32(evidence.app_entry_bytes), "Entry");
  if (!bytesEqual(appEntryHash, computedAppEntryHash)) {
    throw new Error("App entry hash does not match exact App entry bytes");
  }

  const appEntry = asRecord(decodeMessagePack(evidence.app_entry_bytes, "App entry"));
  if (!appEntry || appEntry.entry_type !== "App" || !isBytes(appEntry.entry)) {
    throw new Error("App entry has an invalid entry_type or payload");
  }
  const proposalBytes = encode(evidence.intent.proposal, { useBigInt64: true });
  if (!bytesEqual(appEntry.entry, proposalBytes)) {
    throw new Error("App entry payload does not match the exact clinical approval proposal");
  }

  const countersignedEntryHash = expectHash(evidence.countersigned_entry_hash_bytes, "Countersigned entry hash");
  if (encodeHashToBase64(countersignedEntryHash) !== evidence.countersigned_entry_hash_binding) {
    throw new Error("Countersigned entry hash binding mismatch");
  }
  const computedCountersignedHash = hashFrom32AndType(blake2b32(evidence.countersigned_entry_bytes), "Entry");
  if (!bytesEqual(countersignedEntryHash, computedCountersignedHash)) {
    throw new Error("Countersigned entry hash does not match exact CounterSign entry bytes");
  }

  const countersignedEntry = asRecord(decodeMessagePack(evidence.countersigned_entry_bytes, "Countersigned entry"));
  if (!countersignedEntry || countersignedEntry.entry_type !== "CounterSign"
    || !Array.isArray(countersignedEntry.entry) || countersignedEntry.entry.length !== 2) {
    throw new Error("Countersigned entry has an invalid CounterSign envelope");
  }
  const session = asRecord(countersignedEntry.entry[0]);
  const embeddedPayload = countersignedEntry.entry[1];
  if (!session || !isBytes(embeddedPayload) || !bytesEqual(embeddedPayload, appEntry.entry)) {
    throw new Error("Countersigned entry does not bind the exact private App payload");
  }
  const sessionPreflight = asRecord(session.preflight_request);
  if (!sessionPreflight) throw new Error("Countersigned session is missing its preflight request");
  assertPreflightMatchesIntentBytes(evidence.intent, sessionPreflight, intentBytes);

  const responses = session.responses;
  if (!Array.isArray(responses) || responses.length !== evidence.intent.participants.length) {
    throw new Error("Countersigned response count does not match participant count");
  }
  responses.forEach((response, index) => {
    if (!Array.isArray(response) || response.length !== 2) {
      throw new Error(`Countersigned response ${index} is malformed`);
    }
    const responseState = asRecord(response[0]);
    if (!responseState || responseState.agent_index !== index || !isBytes(response[1]) || response[1].length !== 64) {
      throw new Error(`Countersigned response ${index} has invalid participant metadata or signature length`);
    }
    expectHash(responseState.chain_top, `Countersigned response ${index} chain_top`);
    if (!Number.isSafeInteger(responseState.action_seq) || (responseState.action_seq as number) < 0) {
      throw new Error(`Countersigned response ${index} has invalid action sequence`);
    }
  });

  if (!Array.isArray(evidence.participant_actions)
    || evidence.participant_actions.length !== evidence.intent.participants.length) {
    throw new Error("Participant action count does not match clinical participant count");
  }

  const expectedParticipants = new Map(
    evidence.intent.participants.map((participant) => [participant.agent_binding, participant]),
  );
  const seenParticipants = new Set<string>();
  const verifiedParticipants: VerifiedClinicalCountersigningParticipantV1[] = [];

  for (const action of evidence.participant_actions) {
    const authorBytes = expectHash(action.author_bytes, "Source-chain author bytes");
    const authorBinding = encodeHashToBase64(authorBytes);
    const expectedParticipant = expectedParticipants.get(authorBinding);
    if (!expectedParticipant || seenParticipants.has(authorBinding)) {
      throw new Error("Participant action has missing, duplicate, or unexpected author identity");
    }
    seenParticipants.add(authorBinding);

    if (action.author_binding !== authorBinding
      || action.signer_binding !== action.author_binding
      || !bytesEqual(expectHash(action.signer_bytes, "Signer bytes"), authorBytes)) {
      throw new Error("Signer identity must exactly match source-chain author identity");
    }
    if (!isBytes(action.action_bytes) || !isBytes(action.signature_bytes) || !isBytes(action.entry_bytes)) {
      throw new Error("Source-chain action evidence is missing raw bytes");
    }
    if (!bytesEqual(action.entry_bytes, evidence.countersigned_entry_bytes)
      || !bytesEqual(expectHash(action.entry_hash_bytes, "Action entry hash"), countersignedEntryHash)) {
      throw new Error("Source-chain action does not reference the exact countersigned entry bytes");
    }

    const decodedAction = asRecord(decodeMessagePack(action.action_bytes, "Source-chain action"));
    if (!decodedAction || decodedAction.type !== "Create") {
      throw new Error("Participant source-chain action is not a Create");
    }
    if (!isBytes(decodedAction.author) || !bytesEqual(decodedAction.author, authorBytes)) {
      throw new Error("Source-chain action author identity mismatch");
    }
    if (!isBytes(decodedAction.entry_hash) || !bytesEqual(decodedAction.entry_hash, countersignedEntryHash)) {
      throw new Error("Source-chain action entry hash mismatch");
    }
    if (!isBytes(decodedAction.prev_action) || !bytesEqual(decodedAction.prev_action, expectHash(action.previous_action_hash_bytes, "Previous ActionHash"))) {
      throw new Error("Source-chain previous action binding mismatch");
    }
    if (decodedAction.action_seq !== action.action_seq
      || !Number.isSafeInteger(action.action_seq)
      || action.action_seq < 0) {
      throw new Error("Source-chain action sequence mismatch");
    }

    const timestamp = decodedAction.timestamp;
    if (typeof timestamp !== "bigint" || timestamp !== action.timestamp_micros) {
      throw new Error("Source-chain timestamp mismatch");
    }
    const startMicros = evidence.intent.session_start_ms * 1000n;
    const endMicros = evidence.intent.session_end_ms * 1000n;
    if (timestamp < startMicros || timestamp > endMicros) {
      throw new Error("Source-chain action timestamp is outside the countersigning session window");
    }

    const entryType = asRecord(decodedAction.entry_type);
    const appType = entryType ? asRecord(entryType.App) : null;
    if (!appType || appType.visibility !== "Private") {
      throw new Error("Countersigned clinical entry must be a private App entry");
    }

    const computedActionHash = hashFrom32AndType(blake2b32(action.action_bytes), "Action");
    const providedActionHash = expectHash(action.action_hash_bytes, "ActionHash bytes");
    if (!bytesEqual(providedActionHash, computedActionHash)
      || encodeHashToBase64(computedActionHash) !== action.action_hash) {
      throw new Error("Source-chain action hash mismatch");
    }
    if (encodeHashToBase64(expectHash(action.previous_action_hash_bytes, "Previous ActionHash"))
      !== action.previous_action_hash) {
      throw new Error("Previous ActionHash binding mismatch");
    }

    const signatureValid = await verifyEd25519(authorBytes, action.signature_bytes, action.action_bytes);
    if (!signatureValid) throw new Error("Source-chain author signature verification failed");

    verifiedParticipants.push({
      role: expectedParticipant.role,
      agent_binding: authorBinding,
      action_hash: action.action_hash,
      action_seq: action.action_seq,
      timestamp_micros: timestamp,
    });
  }

  if (seenParticipants.size !== evidence.intent.participants.length) {
    throw new Error("Not every declared participant has exactly one source-chain action");
  }

  const intentDigest = await clinicalCountersigningIntentDigestV1(evidence.intent);
  return {
    evidence_version: 1,
    intent_digest: intentDigest,
    app_entry_hash_binding: evidence.app_entry_hash_binding,
    countersigned_entry_hash_binding: evidence.countersigned_entry_hash_binding,
    participants: verifiedParticipants,
    participant_action_hashes: verifiedParticipants.map((participant) => participant.action_hash),
  };
}

function getStateVariant(state: CountersigningSessionState | null): { kind: string; value: unknown } | null {
  if (state === null) return null;
  const record = asRecord(state);
  if (!record) return null;
  const defined = Object.entries(record).filter(([, value]) => value !== undefined);
  return defined.length === 1 ? { kind: defined[0]![0], value: defined[0]![1] } : null;
}

function preflightForSessionState(state: CountersigningSessionState | null): PreflightRequest | null {
  const variant = getStateVariant(state);
  if (!variant) return null;
  if (variant.kind === CountersigningSessionStateType.Accepted) {
    return variant.value as PreflightRequest;
  }
  if (variant.kind === CountersigningSessionStateType.SignaturesCollected
    || variant.kind === CountersigningSessionStateType.Unknown) {
    const value = asRecord(variant.value);
    const preflight = value ? value.preflight_request : null;
    return asRecord(preflight) ? preflight as PreflightRequest : null;
  }
  return null;
}

export async function observeCountersigningSessionStateV1(
  observerId: string,
  participantBinding: string,
  intent: ClinicalCountersigningIntentV1,
  state: CountersigningSessionState | null,
  observedAtMs: bigint,
  nonce: Uint8Array,
): Promise<NativeCountersigningSessionObservationV1> {
  expectNonEmpty(observerId, "observer_id");
  expectNonEmpty(participantBinding, "participant_binding");
  validateIntent(intent);
  if (!intent.participants.some((participant) => participant.agent_binding === participantBinding)) {
    throw new Error("Observation participant binding is not declared in the clinical intent");
  }
  if (observedAtMs < 0n) throw new Error("Observation time must be non-negative");
  expectBytes(nonce, REQUIRED_COMMITMENT_BYTES, "observation nonce");

  const variant = getStateVariant(state);
  const preflight = preflightForSessionState(state);
  let observationState: NativeCountersigningObservationStateV1 = "missing";
  if (variant?.kind === CountersigningSessionStateType.Accepted) observationState = "accepted";
  else if (variant?.kind === CountersigningSessionStateType.SignaturesCollected) observationState = "signatures_collected";
  else if (variant?.kind === CountersigningSessionStateType.Unknown) observationState = "unknown";

  let preflightMatches: boolean | null = null;
  if (preflight) {
    try {
      const intentBytes = await encodeClinicalCountersigningIntentV1(intent);
      assertPreflightMatchesIntentBytes(intent, preflight as unknown as UnknownRecord, intentBytes);
      preflightMatches = true;
    } catch {
      preflightMatches = false;
    }
  }

  return {
    observation_version: 1,
    observer_id: observerId,
    participant_binding: participantBinding,
    state: observationState,
    intent_digest: await clinicalCountersigningIntentDigestV1(intent),
    observed_at_ms: observedAtMs,
    nonce: copyBytes(nonce),
    preflight_matches_intent: preflightMatches,
    terminal: false,
  };
}

export async function createTerminalCountersigningObservationV1(
  observerId: string,
  participantBinding: string,
  intent: ClinicalCountersigningIntentV1,
  state: "completed" | "abandoned",
  actionHashBinding: string | null,
  observedAtMs: bigint,
  nonce: Uint8Array,
): Promise<NativeTerminalCountersigningObservationV1> {
  expectNonEmpty(observerId, "observer_id");
  validateIntent(intent);
  if (!intent.participants.some((participant) => participant.agent_binding === participantBinding)) {
    throw new Error("Terminal observation participant binding is not declared in the intent");
  }
  if (state === "completed" && (!actionHashBinding || actionHashBinding.trim().length === 0)) {
    throw new Error("A completed terminal observation requires an action hash");
  }
  if (actionHashBinding !== null) {
    expectNonEmpty(actionHashBinding, "action_hash_binding");
  }
  if (observedAtMs < 0n) throw new Error("Terminal observation time must be non-negative");
  expectBytes(nonce, REQUIRED_COMMITMENT_BYTES, "terminal observation nonce");
  return {
    observation_version: 1,
    observer_id: observerId,
    participant_binding: participantBinding,
    state,
    intent_digest: await clinicalCountersigningIntentDigestV1(intent),
    action_hash_binding: actionHashBinding,
    observed_at_ms: observedAtMs,
    nonce: copyBytes(nonce),
    terminal: true,
  };
}
