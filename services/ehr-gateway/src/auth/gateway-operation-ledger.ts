// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

import { createHash } from 'node:crypto';
import type { GatewayActionV1 } from './gateway-request-context.js';

export interface GatewayOperationKeyInputV1 {
  tenantId: string;
  principalBinding: string;
  patientBinding: string;
  connectionId: string;
  action: GatewayActionV1;
  clientIdempotencyKey: string;
}

/** Derive a non-PII key scoped to authenticated authority and operation. */
export function deriveGatewayOperationKeyV1(input: GatewayOperationKeyInputV1): string {
  const values = [
    input.tenantId,
    input.principalBinding,
    input.patientBinding,
    input.connectionId,
    input.action,
    input.clientIdempotencyKey,
  ];
  if (values.some((value) => !nonEmpty(value))) {
    throw new Error('gateway_operation_key_input_invalid');
  }
  const hash = createHash('sha256');
  hash.update('mycelix-gateway-operation-key-v1\\0');
  for (const value of values) {
    hash.update(String(value.length));
    hash.update(':');
    hash.update(value);
  }
  return hash.digest('hex');
}

export interface GatewayOperationIntentV1 {
  /** Namespaced by authenticated tenant/principal and action; not a raw header. */
  operationKey: string;
  /** SHA-256 over canonical immutable operation input. */
  requestDigest: string;
  tenantId: string;
  principalBinding: string;
  patientBinding: string;
  connectionId: string;
  action: GatewayActionV1;
  resourceSetDigest: string;
  initialAuthorizationReceiptDigest: string;
}

export type GatewayOperationStateV1 =
  | 'executing'
  | 'committed'
  | 'failed_known_no_effect'
  | 'outcome_unknown';

export interface GatewayOperationReceiptV1 {
  operationId: string;
  operationKey: string;
  requestDigest: string;
  tenantId: string;
  principalBinding: string;
  patientBinding: string;
  connectionId: string;
  action: GatewayActionV1;
  resourceSetDigest: string;
  initialAuthorizationReceiptDigest: string;
  latestAuthorizationReceiptDigest: string;
  state: GatewayOperationStateV1;
  version: number;
  attemptCount: number;
  createdAtMs: number;
  updatedAtMs: number;
  workerId: string | null;
  externalReceiptId: string | null;
  resultDigest: string | null;
  failureCode: string | null;
  unknownReason: string | null;
  reconciledAtMs: number | null;
}

export type GatewayOperationClaimV1 =
  | { kind: 'claimed'; operation: GatewayOperationReceiptV1 }
  | { kind: 'replay'; operation: GatewayOperationReceiptV1 }
  | { kind: 'in_progress'; operation: GatewayOperationReceiptV1 }
  | { kind: 'outcome_unknown'; operation: GatewayOperationReceiptV1 }
  | { kind: 'key_conflict' }
  | { kind: 'unavailable' };

export type GatewayOperationWriteResultV1 =
  | { kind: 'updated'; operation: GatewayOperationReceiptV1 }
  | { kind: 'version_conflict' }
  | { kind: 'unavailable' };

/**
 * Production stores must implement claimOrRead and every state transition
 * durably and atomically. An executing operation is never reclaimed merely
 * because its lease expired. Same key with a different requestDigest conflicts.
 * A failed_known_no_effect row may only be returned as claimed after an atomic
 * retry claim; the new caller must pass fresh authorization before side effect.
 * executing/outcome_unknown must be reconciled, never blindly re-executed.
 */
export interface GatewayOperationStoreV1 {
  claimOrRead(
    intent: GatewayOperationIntentV1,
    workerId: string,
    nowMs: number,
  ): Promise<GatewayOperationClaimV1>;
  get(operationKey: string): Promise<GatewayOperationReceiptV1 | null>;
  markCommitted(
    operationId: string,
    expectedVersion: number,
    authorizationReceiptDigest: string,
    externalReceiptId: string,
    resultDigest: string,
    nowMs: number,
  ): Promise<GatewayOperationWriteResultV1>;
  markFailedKnownNoEffect(
    operationId: string,
    expectedVersion: number,
    failureCode: string,
    nowMs: number,
  ): Promise<GatewayOperationWriteResultV1>;
  markOutcomeUnknown(
    operationId: string,
    expectedVersion: number,
    reason: string,
    nowMs: number,
  ): Promise<GatewayOperationWriteResultV1>;
}

export type GatewayReauthorizationResultV1 =
  | { authorized: true; authorizationReceiptDigest: string; expiresAtMs: number }
  | { authorized: false; reason: string }
  | { unavailable: true; reason: string };

export interface GatewayOperationReauthorizerV1 {
  /** Re-check exact authority and revocation state immediately before side effect. */
  reauthorize(intent: GatewayOperationIntentV1): Promise<GatewayReauthorizationResultV1>;
}

export interface GatewayExternalCommitV1 {
  externalReceiptId: string;
  resultDigest: string;
}

export type GatewayAdapterExecuteResultV1 =
  | { kind: 'committed'; receipt: GatewayExternalCommitV1 }
  | { kind: 'failed_known_no_effect'; failureCode: string }
  | { kind: 'outcome_unknown'; reason: string };

export type GatewayAdapterReconcileResultV1 =
  | { kind: 'committed'; receipt: GatewayExternalCommitV1 }
  | { kind: 'not_applied_proven'; reason: string }
  | { kind: 'outcome_unknown'; reason: string };

/**
 * If remote idempotency is supported, execute MUST bind the operationKey to
 * the remote request and return the original result on same-key replay.
 * Reconciliation may report not_applied_proven only with positive evidence.
 * Timeout, 404, connection reset or missing local row is not proof of no effect.
 */
export interface GatewayExternalOperationAdapterV1 {
  readonly supportsRemoteIdempotencyKey: boolean;
  readonly supportsOutcomeReconciliation: boolean;
  execute(
    intent: GatewayOperationIntentV1,
    context: { operationKey: string; authorizationReceiptDigest: string },
  ): Promise<GatewayAdapterExecuteResultV1>;
  reconcile(operation: GatewayOperationReceiptV1): Promise<GatewayAdapterReconcileResultV1>;
}

export type GatewayOperationResultV1 =
  | { kind: 'committed'; operation: GatewayOperationReceiptV1; replayed: false }
  | { kind: 'replayed'; operation: GatewayOperationReceiptV1 }
  | { kind: 'in_progress'; operation: GatewayOperationReceiptV1 }
  | { kind: 'outcome_unknown'; operation: GatewayOperationReceiptV1; reason: string }
  | { kind: 'denied'; reason: string }
  | { kind: 'deferred'; reason: string }
  | { kind: 'idempotency_conflict' };

function nonEmpty(value: unknown): value is string {
  return typeof value === 'string' && value.trim().length > 0;
}

function digest(value: unknown): value is string {
  return typeof value === 'string' && /^[0-9a-f]{64}$/i.test(value);
}

function validIntent(intent: GatewayOperationIntentV1): boolean {
  return Boolean(
    intent
      && digest(intent.operationKey)
      && digest(intent.requestDigest)
      && nonEmpty(intent.tenantId)
      && nonEmpty(intent.principalBinding)
      && nonEmpty(intent.patientBinding)
      && nonEmpty(intent.connectionId)
      && nonEmpty(intent.action)
      && digest(intent.resourceSetDigest)
      && digest(intent.initialAuthorizationReceiptDigest),
  );
}

function toPublicResult(operation: GatewayOperationReceiptV1): GatewayOperationResultV1 {
  switch (operation.state) {
    case 'committed':
      return { kind: 'replayed', operation };
    case 'executing':
      return { kind: 'in_progress', operation };
    case 'outcome_unknown':
      return {
        kind: 'outcome_unknown',
        operation,
        reason: operation.unknownReason || 'remote_outcome_unknown',
      };
    case 'failed_known_no_effect':
      return {
        kind: 'deferred',
        reason: operation.failureCode || 'known_no_effect_retry_requires_fresh_authorization',
      };
  }
}

export class GatewayOperationCoordinatorV1 {
  constructor(
    private readonly store: GatewayOperationStoreV1,
    private readonly reauthorizer: GatewayOperationReauthorizerV1,
    private readonly adapter: GatewayExternalOperationAdapterV1,
    private readonly workerId: string,
    private readonly clock: () => number = () => Date.now(),
  ) {}

  /** Start one gateway attempt for a scoped key/digest; duplicates never start
   * another independent operation while an earlier outcome is unresolved. */
  async executeOnce(intent: GatewayOperationIntentV1): Promise<GatewayOperationResultV1> {
    if (!validIntent(intent) || !nonEmpty(this.workerId)) {
      return { kind: 'denied', reason: 'operation_intent_invalid' };
    }
    const nowMs = this.clock();
    if (!Number.isSafeInteger(nowMs)) {
      return { kind: 'deferred', reason: 'trusted_clock_unavailable' };
    }

    let claim: GatewayOperationClaimV1;
    try {
      claim = await this.store.claimOrRead(intent, this.workerId, nowMs);
    } catch {
      return { kind: 'deferred', reason: 'operation_ledger_unavailable' };
    }

    switch (claim.kind) {
      case 'key_conflict':
        return { kind: 'idempotency_conflict' };
      case 'unavailable':
        return { kind: 'deferred', reason: 'operation_ledger_unavailable' };
      case 'replay':
        return { kind: 'replayed', operation: claim.operation };
      case 'in_progress':
        return { kind: 'in_progress', operation: claim.operation };
      case 'outcome_unknown':
        return toPublicResult(claim.operation);
      case 'claimed':
        break;
    }

    let authorization: GatewayReauthorizationResultV1;
    try {
      authorization = await this.reauthorizer.reauthorize(intent);
    } catch {
      const write = await this.safeMarkFailed(
        claim.operation,
        'authorization_recheck_unavailable_before_side_effect',
      );
      return write
        ? { kind: 'deferred', reason: 'authorization_recheck_unavailable' }
        : { kind: 'deferred', reason: 'operation_ledger_write_unavailable' };
    }

    if ('unavailable' in authorization) {
      const write = await this.safeMarkFailed(claim.operation, authorization.reason);
      return write
        ? { kind: 'deferred', reason: authorization.reason }
        : { kind: 'deferred', reason: 'operation_ledger_write_unavailable' };
    }
    if (!authorization.authorized) {
      const write = await this.safeMarkFailed(
        claim.operation,
        authorization.reason || 'authority_denied_before_execution',
      );
      return write
        ? { kind: 'denied', reason: authorization.reason }
        : { kind: 'deferred', reason: 'operation_ledger_write_unavailable' };
    }
    if (!digest(authorization.authorizationReceiptDigest)
      || !Number.isSafeInteger(authorization.expiresAtMs)
      || authorization.expiresAtMs <= this.clock()) {
      const write = await this.safeMarkFailed(
        claim.operation,
        'authorization_receipt_invalid_or_expired',
      );
      return write
        ? { kind: 'deferred', reason: 'authorization_receipt_invalid_or_expired' }
        : { kind: 'deferred', reason: 'operation_ledger_write_unavailable' };
    }

    let execution: GatewayAdapterExecuteResultV1;
    try {
      execution = await this.adapter.execute(intent, {
        operationKey: intent.operationKey,
        authorizationReceiptDigest: authorization.authorizationReceiptDigest,
      });
    } catch {
      await this.markUnknown(claim.operation, 'external_outcome_unknown');
      return { kind: 'deferred', reason: 'external_outcome_unknown_reconcile_same_key' };
    }

    if (execution.kind === 'failed_known_no_effect') {
      const write = await this.safeMarkFailed(
        claim.operation,
        execution.failureCode || 'external_operation_failed_without_effect',
      );
      return write
        ? { kind: 'deferred', reason: 'known_no_effect_retry_requires_fresh_authorization' }
        : { kind: 'deferred', reason: 'operation_ledger_write_unavailable' };
    }
    if (execution.kind === 'outcome_unknown') {
      await this.markUnknown(claim.operation, execution.reason);
      return { kind: 'deferred', reason: 'external_outcome_unknown_reconcile_same_key' };
    }

    const receipt = execution.receipt;
    if (!nonEmpty(receipt.externalReceiptId) || !digest(receipt.resultDigest)) {
      await this.markUnknown(claim.operation, 'external_commit_receipt_invalid');
      return { kind: 'deferred', reason: 'external_commit_receipt_invalid' };
    }

    let stored: GatewayOperationWriteResultV1;
    try {
      stored = await this.store.markCommitted(
        claim.operation.operationId,
        claim.operation.version,
        authorization.authorizationReceiptDigest,
        receipt.externalReceiptId,
        receipt.resultDigest,
        this.clock(),
      );
    } catch {
      return { kind: 'deferred', reason: 'commit_acknowledged_but_ledger_write_unknown' };
    }
    if (stored.kind !== 'updated') {
      return { kind: 'deferred', reason: 'commit_acknowledged_but_ledger_write_unknown' };
    }
    return { kind: 'committed', operation: stored.operation, replayed: false };
  }

  /** Reconcile a possibly committed operation; never initiates a fresh key. */
  async reconcile(operationKey: string): Promise<GatewayOperationResultV1> {
    if (!nonEmpty(operationKey)) {
      return { kind: 'denied', reason: 'operation_key_invalid' };
    }
    let operation: GatewayOperationReceiptV1 | null;
    try {
      operation = await this.store.get(operationKey);
    } catch {
      return { kind: 'deferred', reason: 'operation_ledger_unavailable' };
    }
    if (!operation) {
      return { kind: 'deferred', reason: 'operation_not_found_reconcile_only' };
    }
    if (operation.state === 'committed') {
      return { kind: 'replayed', operation };
    }
    if (operation.state === 'failed_known_no_effect') {
      return {
        kind: 'deferred',
        reason: 'known_no_effect_retry_requires_fresh_request_authorization',
      };
    }

    let result: GatewayAdapterReconcileResultV1;
    let reconciliationAuthorizationReceiptDigest = operation.latestAuthorizationReceiptDigest;
    try {
      if (this.adapter.supportsOutcomeReconciliation) {
        result = await this.adapter.reconcile(operation);
      } else if (this.adapter.supportsRemoteIdempotencyKey) {
        // Same-key replay may be the first delivery if the prior request never
        // reached the remote API. Re-authorize before resubmitting it.
        const retryIntent: GatewayOperationIntentV1 = {
          operationKey: operation.operationKey,
          requestDigest: operation.requestDigest,
          tenantId: operation.tenantId,
          principalBinding: operation.principalBinding,
          patientBinding: operation.patientBinding,
          connectionId: operation.connectionId,
          action: operation.action,
          resourceSetDigest: operation.resourceSetDigest,
          initialAuthorizationReceiptDigest: operation.initialAuthorizationReceiptDigest,
        };
        const freshAuthorization = await this.reauthorizer.reauthorize(retryIntent);
        if ('unavailable' in freshAuthorization
          || !freshAuthorization.authorized
          || !digest(freshAuthorization.authorizationReceiptDigest)
          || !Number.isSafeInteger(freshAuthorization.expiresAtMs)
          || freshAuthorization.expiresAtMs <= this.clock()) {
          return {
            kind: 'deferred',
            reason: 'fresh_authority_required_before_same_key_remote_replay',
          };
        }
        reconciliationAuthorizationReceiptDigest = freshAuthorization.authorizationReceiptDigest;
        const replay = await this.adapter.execute(
          retryIntent,
          {
            operationKey: operation.operationKey,
            authorizationReceiptDigest: freshAuthorization.authorizationReceiptDigest,
          },
        );
        if (replay.kind === 'committed') {
          result = replay;
        } else if (replay.kind === 'failed_known_no_effect') {
          result = {
            kind: 'not_applied_proven',
            reason: replay.failureCode,
          };
        } else {
          result = { kind: 'outcome_unknown', reason: replay.reason };
        }
      } else {
        return {
          kind: 'outcome_unknown',
          operation,
          reason: 'adapter_cannot_reconcile_remote_outcome',
        };
      }
    } catch {
      await this.markUnknown(operation, 'reconciliation_service_unavailable');
      return { kind: 'deferred', reason: 'reconciliation_service_unavailable' };
    }

    if (result.kind === 'outcome_unknown') {
      const updated = await this.markUnknown(operation, result.reason);
      return { kind: 'outcome_unknown', operation: updated ?? operation, reason: result.reason };
    }
    if (result.kind === 'not_applied_proven') {
      const write = await this.safeMarkFailed(
        operation,
        'reconciled_no_effect:' + result.reason,
      );
      return write
        ? { kind: 'deferred', reason: 'no_effect_proven_new_attempt_requires_fresh_authorization' }
        : { kind: 'deferred', reason: 'operation_ledger_write_unavailable' };
    }
    if (!nonEmpty(result.receipt.externalReceiptId) || !digest(result.receipt.resultDigest)) {
      await this.markUnknown(operation, 'reconciliation_receipt_invalid');
      return { kind: 'deferred', reason: 'reconciliation_receipt_invalid' };
    }

    let write: GatewayOperationWriteResultV1;
    try {
      write = await this.store.markCommitted(
        operation.operationId,
        operation.version,
        reconciliationAuthorizationReceiptDigest,
        result.receipt.externalReceiptId,
        result.receipt.resultDigest,
        this.clock(),
      );
    } catch {
      return { kind: 'deferred', reason: 'reconciliation_commit_ledger_write_unknown' };
    }
    return write.kind === 'updated'
      ? { kind: 'replayed', operation: write.operation }
      : { kind: 'deferred', reason: 'reconciliation_commit_ledger_write_unknown' };
  }

  private async safeMarkFailed(
    operation: GatewayOperationReceiptV1,
    failureCode: string,
  ): Promise<boolean> {
    try {
      return (await this.store.markFailedKnownNoEffect(
        operation.operationId,
        operation.version,
        failureCode,
        this.clock(),
      )).kind === 'updated';
    } catch {
      return false;
    }
  }

  private async markUnknown(
    operation: GatewayOperationReceiptV1,
    reason: string,
  ): Promise<GatewayOperationReceiptV1 | null> {
    try {
      const result = await this.store.markOutcomeUnknown(
        operation.operationId,
        operation.version,
        reason,
        this.clock(),
      );
      return result.kind === 'updated' ? result.operation : null;
    } catch {
      // The durable row remains executing; future callers still cannot reclaim
      // it. A reconciliation worker/operator must inspect it.
      return null;
    }
  }
}
