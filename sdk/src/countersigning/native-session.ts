// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Read-only countersigning inspection and explicit recovery dispatch helpers.
 *
 * Recovery recommendations are policy suggestions, not authority. The
 * production caller must authenticate the operator, verify consent and tenant
 * scope, and apply the deployment's own forced-resolution policy before calling
 * executeCountersigningRecoveryV1.
 */

import {
  CountersigningSessionStateType,
  encodeHashToBase64,
  type CellId,
  type CountersigningSessionState,
  type PreflightRequest,
} from "@holochain/client";
import {
  observeCountersigningSessionStateV1,
  type ClinicalCountersigningIntentV1,
  type NativeCountersigningSessionObservationV1,
} from "../crypto/native-countersigning";

export type CountersigningRecoveryActionV1 =
  | "wait"
  | "manual_review"
  | "force_publish"
  | "force_abandon";

export interface CountersigningRecoveryPolicyV1 {
  allow_force_publish?: boolean;
  allow_force_abandon?: boolean;
  minimum_resolution_attempts?: number;
}

export interface CountersigningSessionAdminClientV1 {
  getCountersigningSessionState(): Promise<CountersigningSessionState | null>;
  abandonCountersigningSession(): Promise<unknown>;
  publishCountersigningSession(): Promise<unknown>;
}

export interface CountersigningParticipantRuntimeV1 {
  observer_id: string;
  participant_binding: string;
  cell_id: CellId;
  client: CountersigningSessionAdminClientV1;
}

export interface CountersigningParticipantInspectionV1 {
  observer_id: string;
  participant_binding: string;
  observation: NativeCountersigningSessionObservationV1 | null;
  recommended_action: CountersigningRecoveryActionV1;
}

export interface CountersigningParticipantReconciliationV1 {
  observations: CountersigningParticipantInspectionV1[];
  has_missing_state: boolean;
  has_conflicting_preflight: boolean;
  recommended_action: CountersigningRecoveryActionV1;
}

type UnknownRecord = Record<string, unknown>;

function asRecord(value: unknown): UnknownRecord | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? value as UnknownRecord
    : null;
}

function bytesEqual(left: Uint8Array, right: Uint8Array): boolean {
  if (left.byteLength !== right.byteLength) return false;
  let difference = 0;
  for (let index = 0; index < left.byteLength; index += 1) {
    difference |= left[index]! ^ right[index]!;
  }
  return difference === 0;
}

function assertRuntimeParticipantBinding(
  runtime: CountersigningParticipantRuntimeV1,
  intent: ClinicalCountersigningIntentV1,
  expectedBinding?: string,
): void {
  if (!Array.isArray(runtime.cell_id) || runtime.cell_id.length !== 2
    || !(runtime.cell_id[1] instanceof Uint8Array)) {
    throw new Error("Participant runtime has a malformed Holochain cell identity");
  }

  const cellAgent = runtime.cell_id[1];
  const cellBinding = encodeHashToBase64(cellAgent);
  if (runtime.participant_binding !== cellBinding) {
    throw new Error("Participant runtime binding does not match the authenticated cell agent key");
  }
  if (expectedBinding !== undefined && runtime.participant_binding !== expectedBinding) {
    throw new Error("Participant runtime order does not match the clinical intent participant order");
  }

  const participant = intent.participants.find(
    (candidate) => candidate.agent_binding === runtime.participant_binding,
  );
  if (!participant || !(participant.agent_bytes instanceof Uint8Array)
    || !bytesEqual(cellAgent, participant.agent_bytes)) {
    throw new Error("Participant runtime cell agent bytes do not match the exact clinical intent participant");
  }
}

function stateVariant(state: CountersigningSessionState | null): { kind: string; value: unknown } | null {
  if (state === null) return null;
  const record = asRecord(state);
  if (!record) return null;
  const entries = Object.entries(record).filter(([, value]) => value !== undefined);
  return entries.length === 1 ? { kind: entries[0]![0], value: entries[0]![1] } : null;
}

function preflightFromState(state: CountersigningSessionState | null): PreflightRequest | null {
  const variant = stateVariant(state);
  if (!variant) return null;
  if (variant.kind === CountersigningSessionStateType.Accepted) {
    return asRecord(variant.value) ? variant.value as PreflightRequest : null;
  }
  if (variant.kind === CountersigningSessionStateType.SignaturesCollected
    || variant.kind === CountersigningSessionStateType.Unknown) {
    const payload = asRecord(variant.value);
    const preflight = payload ? payload.preflight_request : null;
    return asRecord(preflight) ? preflight as PreflightRequest : null;
  }
  return null;
}

function normalizeDecision(value: unknown): string {
  if (typeof value === "string") {
    const split = value.split(".");
    return split[split.length - 1]!;
  }
  const record = asRecord(value);
  if (!record) return "Unknown";
  const keys = Object.keys(record).filter((key) => record[key] !== undefined);
  return keys.length === 1 ? keys[0]! : "Unknown";
}

function stableValue(value: unknown): string {
  if (value instanceof Uint8Array) {
    return `bytes:${Array.from(value).map((byte) => byte.toString(16).padStart(2, "0")).join("")}`;
  }
  if (typeof value === "bigint") return `bigint:${value.toString()}`;
  if (value === null) return "null";
  if (Array.isArray(value)) return `[${value.map(stableValue).join(",")}]`;
  if (typeof value === "object") {
    const record = value as UnknownRecord;
    const keys = Object.keys(record).sort();
    return `{${keys.map((key) => `${JSON.stringify(key)}:${stableValue(record[key])}`).join(",")}}`;
  }
  return JSON.stringify(value) ?? String(value);
}

function preflightFingerprint(preflight: PreflightRequest | null): string | null {
  if (!preflight) return null;
  return stableValue({
    app_entry_hash: preflight.app_entry_hash,
    signing_agents: preflight.signing_agents,
    session_times: preflight.session_times,
    action_base: preflight.action_base,
    preflight_bytes: preflight.preflight_bytes,
  });
}

/**
 * Return a conservative recovery suggestion. Accepted/signature-collection
 * states always wait. Missing state and conflicting/indeterminate authority
 * outcomes go to manual review. Forced actions require explicit policy flags,
 * a single unanimous decision and a minimum number of resolution attempts.
 */
export function recommendCountersigningRecoveryV1(
  state: CountersigningSessionState | null,
  policy: CountersigningRecoveryPolicyV1 = {},
): { action: CountersigningRecoveryActionV1; reason: string } {
  if (state === null) return { action: "manual_review", reason: "No active session state; terminal outcome cannot be inferred" };

  const variant = stateVariant(state);
  if (!variant) return { action: "manual_review", reason: "Session state is malformed or ambiguous" };
  if (variant.kind === CountersigningSessionStateType.Accepted) {
    return { action: "wait", reason: "Session accepted; await participant signatures" };
  }
  if (variant.kind === CountersigningSessionStateType.SignaturesCollected) {
    return { action: "wait", reason: "Signatures are being collected; no terminal transition is inferred" };
  }
  if (variant.kind !== CountersigningSessionStateType.Unknown) {
    return { action: "manual_review", reason: "Terminal or unsupported session state requires independent review" };
  }

  const unknown = asRecord(variant.value);
  const resolution = unknown ? asRecord(unknown.resolution) : null;
  if (!resolution) return { action: "manual_review", reason: "Unknown session has no resolution evidence" };

  const attempts = typeof resolution.attempts === "number" && Number.isSafeInteger(resolution.attempts)
    ? resolution.attempts
    : 0;
  const minimum = policy.minimum_resolution_attempts ?? 2;
  if (!Number.isSafeInteger(minimum) || minimum < 1) {
    return { action: "manual_review", reason: "Invalid minimum resolution attempt policy" };
  }
  if (attempts < minimum) {
    return { action: "wait", reason: "Configured resolution-attempt threshold has not been met" };
  }

  if (!Array.isArray(resolution.outcomes) || resolution.outcomes.length === 0) {
    return { action: "manual_review", reason: "No participant resolution outcomes are available" };
  }

  const decisions = resolution.outcomes.flatMap((outcome) => {
    const record = asRecord(outcome);
    return record && Array.isArray(record.decisions) ? record.decisions.map(normalizeDecision) : [];
  });
  if (decisions.length === 0) {
    return { action: "manual_review", reason: "No explicit participant completion decisions are available" };
  }

  const uniqueDecisions = Array.from(new Set(decisions));
  if (uniqueDecisions.length !== 1) {
    return { action: "manual_review", reason: "Participant completion decisions conflict" };
  }

  const decision = uniqueDecisions[0]!;
  if (decision === "Complete" && policy.allow_force_publish === true) {
    return { action: "force_publish", reason: "Unanimous completion evidence and explicit force-publish policy" };
  }
  if (decision === "Abandoned" && policy.allow_force_abandon === true) {
    return { action: "force_abandon", reason: "Unanimous abandonment evidence and explicit force-abandon policy" };
  }
  if (decision === "Complete" || decision === "Abandoned") {
    return { action: "manual_review", reason: "Forced transition lacks explicit deployment policy authorization" };
  }
  return { action: "manual_review", reason: "Indeterminate, failed or unsupported participant resolution decision" };
}

async function inspectParticipantSnapshotV1(
  runtime: CountersigningParticipantRuntimeV1,
  intent: ClinicalCountersigningIntentV1,
  observedAtMs: bigint,
  nonce: Uint8Array,
  state: CountersigningSessionState | null,
): Promise<{
  inspection: CountersigningParticipantInspectionV1;
  preflight_fingerprint: string | null;
}> {
  const observation = await observeCountersigningSessionStateV1(
    runtime.observer_id,
    runtime.participant_binding,
    intent,
    state,
    observedAtMs,
    nonce,
  );

  let recommendation = recommendCountersigningRecoveryV1(state);
  if (observation.preflight_matches_intent === false) {
    recommendation = {
      action: "manual_review",
      reason: "Observed participant preflight conflicts with the exact clinical intent",
    };
  }

  return {
    inspection: {
      observer_id: runtime.observer_id,
      participant_binding: runtime.participant_binding,
      observation,
      recommended_action: recommendation.action,
    },
    // This fingerprint is derived from the exact same state snapshot used to
    // construct the signed observation; do not read the conductor a second time.
    preflight_fingerprint: preflightFingerprint(preflightFromState(state)),
  };
}

export async function inspectCountersigningParticipantV1(
  runtime: CountersigningParticipantRuntimeV1,
  intent: ClinicalCountersigningIntentV1,
  observedAtMs: bigint,
  nonce: Uint8Array,
): Promise<CountersigningParticipantInspectionV1> {
  assertRuntimeParticipantBinding(runtime, intent);
  const state = await runtime.client.getCountersigningSessionState();
  const result = await inspectParticipantSnapshotV1(
    runtime,
    intent,
    observedAtMs,
    nonce,
    state,
  );
  return result.inspection;
}

export async function reconcileCountersigningParticipantsV1(
  runtimes: CountersigningParticipantRuntimeV1[],
  intent: ClinicalCountersigningIntentV1,
  observedAtMs: bigint,
  nonceForParticipant: (index: number) => Uint8Array,
): Promise<CountersigningParticipantReconciliationV1> {
  if (runtimes.length !== intent.participants.length) {
    throw new Error("Runtime participant count does not match the clinical intent");
  }
  // Bind participant order and the runtime's actual cell agent key before any
  // conductor reads. A valid session observed through the wrong cell must not
  // be attributed to another participant.
  runtimes.forEach((runtime, index) => {
    const expected = intent.participants[index];
    if (!expected) throw new Error("Clinical intent participant is missing");
    assertRuntimeParticipantBinding(runtime, intent, expected.agent_binding);
  });

  const snapshots = await Promise.all(runtimes.map(async (runtime, index) => {
    try {
      const state = await runtime.client.getCountersigningSessionState();
      return await inspectParticipantSnapshotV1(
        runtime,
        intent,
        observedAtMs,
        nonceForParticipant(index),
        state,
      );
    } catch {
      return {
        inspection: {
          observer_id: runtime.observer_id,
          participant_binding: runtime.participant_binding,
          observation: null,
          recommended_action: "manual_review" as const,
        },
        preflight_fingerprint: null,
      };
    }
  }));
  const inspections = snapshots.map((snapshot) => snapshot.inspection);
  const hasMissingState = inspections.some((inspection) => inspection.observation === null
    || inspection.observation.state === "missing");
  const fingerprints = snapshots
    .map((snapshot) => snapshot.preflight_fingerprint)
    .filter((fingerprint): fingerprint is string => fingerprint !== null);
  const hasConflictingPreflight = new Set(fingerprints).size > 1
    || inspections.some((inspection) => inspection.observation?.preflight_matches_intent === false);

  let recommendedAction: CountersigningRecoveryActionV1 = "wait";
  if (hasMissingState || hasConflictingPreflight) {
    recommendedAction = "manual_review";
  } else {
    const actions = new Set(inspections.map((inspection) => inspection.recommended_action));
    if (actions.size !== 1) {
      recommendedAction = "manual_review";
    } else {
      recommendedAction = actions.values().next().value ?? "manual_review";
    }
  }

  return {
    observations: inspections,
    has_missing_state: hasMissingState,
    has_conflicting_preflight: hasConflictingPreflight,
    recommended_action: recommendedAction,
  };
}

/**
 * Dispatch only a caller-selected forced action. This is not an authorization
 * check: the caller must enforce consent, tenant scope, operator identity and
 * deployment policy before invoking this method.
 */
export async function executeCountersigningRecoveryV1(
  runtime: CountersigningParticipantRuntimeV1,
  action: CountersigningRecoveryActionV1,
): Promise<void> {
  if (action === "force_publish") {
    await runtime.client.publishCountersigningSession();
    return;
  }
  if (action === "force_abandon") {
    await runtime.client.abandonCountersigningSession();
    return;
  }
  throw new Error("An explicit forced recovery decision is required before execution");
}
