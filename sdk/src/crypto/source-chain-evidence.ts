// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * Portable source-chain action evidence.
 *
 * This DTO is an evidence transport shape. It is not a Holochain Record and
 * must be independently checked against the exact action bytes and signature.
 */
export interface PortableChainActionV1 {
  action_hash: string;
  action_hash_bytes: Uint8Array;
  previous_action_hash: string;
  previous_action_hash_bytes: Uint8Array;
  author_binding: string;
  author_bytes: Uint8Array;
  signer_binding: string;
  signer_bytes: Uint8Array;
  action_bytes: Uint8Array;
  signature_bytes: Uint8Array;
  entry_hash_bytes: Uint8Array;
  entry_bytes: Uint8Array;
  action_seq: number;
  timestamp_micros: bigint;
}
