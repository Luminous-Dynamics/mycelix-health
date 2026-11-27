// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Clinical approval proposal primitives used by the native countersigning
 * evidence verifier. These are wire-contract types, not authorization tokens.
 */

export type ClinicalApprovalRoleV1 = "patient" | "provider" | "reviewer";

export type ClinicalApprovalOperationClassV1 =
  | "sensitive_record_release"
  | "provider_correction"
  | "break_glass_acknowledgement";

export interface ClinicalApprovalProposalV1 {
  approval_version: 1;
  operation_class: ClinicalApprovalOperationClassV1;
  /** Opaque, typed Holochain ActionHash encoded as Holochain base64. */
  patient_binding: string;
  /** Opaque, typed Holochain AgentPubKey encoded as Holochain base64. */
  provider_binding: string;
  mutation_binding_digest: Uint8Array;
  deployment_evidence_sha256: Uint8Array;
  policy_digest: Uint8Array;
  rationale_sha256: Uint8Array;
  created_at_ms: bigint;
  expires_at_ms: bigint;
  nonce: Uint8Array;
}
