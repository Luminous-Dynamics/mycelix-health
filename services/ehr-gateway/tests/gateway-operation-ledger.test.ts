import { describe, expect, it, vi } from 'vitest';
import {
  GatewayOperationCoordinatorV1,
  type GatewayAdapterExecuteResultV1,
  type GatewayAdapterReconcileResultV1,
  type GatewayExternalOperationAdapterV1,
  type GatewayOperationIntentV1,
  type GatewayOperationReceiptV1,
  type GatewayOperationReauthorizerV1,
  type GatewayOperationStoreV1,
  type GatewayOperationClaimV1,
  type GatewayOperationWriteResultV1,
  type GatewayReauthorizationResultV1,
} from '../src/auth/gateway-operation-ledger.js';

const DIGEST_A = 'a'.repeat(64);
const DIGEST_B = 'b'.repeat(64);
const DIGEST_C = 'c'.repeat(64);

function intent(overrides: Partial<GatewayOperationIntentV1> = {}): GatewayOperationIntentV1 {
  return {
    operationKey: 'tenant-1:principal-1:sync_pull:client-key-1',
    requestDigest: DIGEST_A,
    tenantId: 'tenant-1',
    principalBinding: 'principal-1',
    patientBinding: 'opaque-patient-1',
    connectionId: 'ehr-connection-1',
    action: 'sync_pull',
    resourceSetDigest: DIGEST_B,
    initialAuthorizationReceiptDigest: DIGEST_C,
    ...overrides,
  };
}

class MemoryGatewayOperationStore implements GatewayOperationStoreV1 {
  readonly records = new Map<string, GatewayOperationReceiptV1>();
  unavailable = false;
  failNextCommit = false;
  private nextId = 1;

  async claimOrRead(
    request: GatewayOperationIntentV1,
    workerId: string,
    nowMs: number,
  ): Promise<GatewayOperationClaimV1> {
    if (this.unavailable) return { kind: 'unavailable' };
    const existing = this.records.get(request.operationKey);
    if (existing) {
      if (existing.requestDigest !== request.requestDigest) {
        return { kind: 'key_conflict' };
      }
      switch (existing.state) {
        case 'committed':
          return { kind: 'replay', operation: existing };
        case 'executing':
          return { kind: 'in_progress', operation: existing };
        case 'outcome_unknown':
          return { kind: 'outcome_unknown', operation: existing };
        case 'failed_known_no_effect': {
          const claimed: GatewayOperationReceiptV1 = {
            ...existing,
            state: 'executing',
            version: existing.version + 1,
            attemptCount: existing.attemptCount + 1,
            updatedAtMs: nowMs,
            workerId,
            failureCode: null,
            unknownReason: null,
          };
          this.records.set(request.operationKey, claimed);
          return { kind: 'claimed', operation: claimed };
        }
      }
    }

    const created: GatewayOperationReceiptV1 = {
      operationId: 'operation-' + this.nextId++,
      operationKey: request.operationKey,
      requestDigest: request.requestDigest,
      tenantId: request.tenantId,
      principalBinding: request.principalBinding,
      patientBinding: request.patientBinding,
      connectionId: request.connectionId,
      action: request.action,
      resourceSetDigest: request.resourceSetDigest,
      initialAuthorizationReceiptDigest: request.initialAuthorizationReceiptDigest,
      latestAuthorizationReceiptDigest: request.initialAuthorizationReceiptDigest,
      state: 'executing',
      version: 1,
      attemptCount: 1,
      createdAtMs: nowMs,
      updatedAtMs: nowMs,
      workerId,
      externalReceiptId: null,
      resultDigest: null,
      failureCode: null,
      unknownReason: null,
      reconciledAtMs: null,
    };
    this.records.set(request.operationKey, created);
    return { kind: 'claimed', operation: created };
  }

  async get(operationKey: string): Promise<GatewayOperationReceiptV1 | null> {
    if (this.unavailable) throw new Error('ledger unavailable');
    return this.records.get(operationKey) ?? null;
  }

  async markCommitted(
    operationId: string,
    expectedVersion: number,
    authorizationReceiptDigest: string,
    externalReceiptId: string,
    resultDigest: string,
    nowMs: number,
  ): Promise<GatewayOperationWriteResultV1> {
    if (this.unavailable || this.failNextCommit) {
      this.failNextCommit = false;
      return { kind: 'unavailable' };
    }
    const current = [...this.records.values()].find((r) => r.operationId === operationId);
    if (!current || current.version !== expectedVersion
      || (current.state !== 'executing' && current.state !== 'outcome_unknown')) {
      return { kind: 'version_conflict' };
    }
    const updated: GatewayOperationReceiptV1 = {
      ...current,
      state: 'committed',
      version: current.version + 1,
      updatedAtMs: nowMs,
      latestAuthorizationReceiptDigest: authorizationReceiptDigest,
      externalReceiptId,
      resultDigest,
      failureCode: null,
      unknownReason: null,
      reconciledAtMs: nowMs,
    };
    this.records.set(current.operationKey, updated);
    return { kind: 'updated', operation: updated };
  }

  async markFailedKnownNoEffect(
    operationId: string,
    expectedVersion: number,
    failureCode: string,
    nowMs: number,
  ): Promise<GatewayOperationWriteResultV1> {
    if (this.unavailable) return { kind: 'unavailable' };
    const current = [...this.records.values()].find((r) => r.operationId === operationId);
    if (!current || current.version !== expectedVersion || current.state !== 'executing') {
      return { kind: 'version_conflict' };
    }
    const updated: GatewayOperationReceiptV1 = {
      ...current,
      state: 'failed_known_no_effect',
      version: current.version + 1,
      updatedAtMs: nowMs,
      failureCode,
      workerId: null,
    };
    this.records.set(current.operationKey, updated);
    return { kind: 'updated', operation: updated };
  }

  async markOutcomeUnknown(
    operationId: string,
    expectedVersion: number,
    reason: string,
    nowMs: number,
  ): Promise<GatewayOperationWriteResultV1> {
    if (this.unavailable) return { kind: 'unavailable' };
    const current = [...this.records.values()].find((r) => r.operationId === operationId);
    if (!current || current.version !== expectedVersion || current.state !== 'executing') {
      return { kind: 'version_conflict' };
    }
    const updated: GatewayOperationReceiptV1 = {
      ...current,
      state: 'outcome_unknown',
      version: current.version + 1,
      updatedAtMs: nowMs,
      unknownReason: reason,
      workerId: null,
    };
    this.records.set(current.operationKey, updated);
    return { kind: 'updated', operation: updated };
  }
}

class TestAdapter implements GatewayExternalOperationAdapterV1 {
  supportsRemoteIdempotencyKey = true;
  supportsOutcomeReconciliation = true;
  executeCalls: string[] = [];
  reconcileCalls: string[] = [];
  nextExecute: GatewayAdapterExecuteResultV1 = {
    kind: 'committed',
    receipt: { externalReceiptId: 'ehr-receipt-1', resultDigest: DIGEST_C },
  };
  nextReconcile: GatewayAdapterReconcileResultV1 = {
    kind: 'committed',
    receipt: { externalReceiptId: 'ehr-receipt-1', resultDigest: DIGEST_C },
  };

  async execute(
    request: GatewayOperationIntentV1,
    context: { operationKey: string; authorizationReceiptDigest: string },
  ): Promise<GatewayAdapterExecuteResultV1> {
    this.executeCalls.push(context.operationKey);
    expect(request.operationKey).toBe(context.operationKey);
    return this.nextExecute;
  }

  async reconcile(operation: GatewayOperationReceiptV1): Promise<GatewayAdapterReconcileResultV1> {
    this.reconcileCalls.push(operation.operationKey);
    return this.nextReconcile;
  }
}

function authorizedReauthorizer(): GatewayOperationReauthorizerV1 {
  return {
    reauthorize: vi.fn(async (): Promise<GatewayReauthorizationResultV1> => ({
      authorized: true,
      authorizationReceiptDigest: DIGEST_A,
      expiresAtMs: 10_000,
    })),
  };
}

function coordinator(
  store: MemoryGatewayOperationStore,
  adapter: TestAdapter,
  reauthorizer: GatewayOperationReauthorizerV1 = authorizedReauthorizer(),
): GatewayOperationCoordinatorV1 {
  return new GatewayOperationCoordinatorV1(store, reauthorizer, adapter, 'worker-1', () => 1_000);
}

describe('GatewayOperationCoordinatorV1', () => {
  it('commits one operation and replays the original receipt for duplicate requests', async () => {
    const store = new MemoryGatewayOperationStore();
    const adapter = new TestAdapter();
    const service = coordinator(store, adapter);

    const first = await service.executeOnce(intent());
    const second = await service.executeOnce(intent());

    expect(first.kind).toBe('committed');
    expect(second.kind).toBe('replayed');
    expect(adapter.executeCalls).toEqual([intent().operationKey]);
    expect(store.records.get(intent().operationKey)?.state).toBe('committed');
  });

  it('rejects reuse of an idempotency key with a different request digest', async () => {
    const store = new MemoryGatewayOperationStore();
    const adapter = new TestAdapter();
    const service = coordinator(store, adapter);

    await service.executeOnce(intent());
    const collision = await service.executeOnce(intent({ requestDigest: 'd'.repeat(64) }));

    expect(collision).toEqual({ kind: 'idempotency_conflict' });
    expect(adapter.executeCalls).toHaveLength(1);
  });

  it('revalidates current authority immediately before the external side effect', async () => {
    const store = new MemoryGatewayOperationStore();
    const adapter = new TestAdapter();
    const reauthorizer: GatewayOperationReauthorizerV1 = {
      reauthorize: vi.fn(async () => ({
        authorized: false as const,
        reason: 'grant_revoked',
      })),
    };
    const result = await coordinator(store, adapter, reauthorizer).executeOnce(intent());

    expect(result).toEqual({ kind: 'denied', reason: 'grant_revoked' });
    expect(adapter.executeCalls).toHaveLength(0);
    expect(store.records.get(intent().operationKey)?.state).toBe('failed_known_no_effect');
  });

  it('does not re-execute an operation whose outcome is unknown', async () => {
    const store = new MemoryGatewayOperationStore();
    const adapter = new TestAdapter();
    adapter.nextExecute = { kind: 'outcome_unknown', reason: 'socket_closed_after_write' };
    const service = coordinator(store, adapter);

    const first = await service.executeOnce(intent());
    const second = await service.executeOnce(intent());

    expect(first.kind).toBe('deferred');
    expect(second.kind).toBe('outcome_unknown');
    expect(adapter.executeCalls).toHaveLength(1);
    expect(store.records.get(intent().operationKey)?.state).toBe('outcome_unknown');
  });

  it('reconciles an unknown operation by the same remote idempotency key', async () => {
    const store = new MemoryGatewayOperationStore();
    const adapter = new TestAdapter();
    adapter.nextExecute = { kind: 'outcome_unknown', reason: 'ack_lost' };
    const service = coordinator(store, adapter);

    await service.executeOnce(intent());
    const reconciled = await service.reconcile(intent().operationKey);

    expect(reconciled.kind).toBe('replayed');
    expect(adapter.executeCalls).toHaveLength(1);
    expect(adapter.reconcileCalls).toEqual([intent().operationKey]);
    expect(store.records.get(intent().operationKey)?.state).toBe('committed');
  });

  it('can use same-key remote replay when the adapter has idempotency but no lookup API', async () => {
    const store = new MemoryGatewayOperationStore();
    const adapter = new TestAdapter();
    adapter.supportsOutcomeReconciliation = false;
    adapter.nextExecute = { kind: 'outcome_unknown', reason: 'ack_lost' };
    const service = coordinator(store, adapter);

    await service.executeOnce(intent());
    adapter.nextExecute = {
      kind: 'committed',
      receipt: { externalReceiptId: 'ehr-receipt-2', resultDigest: DIGEST_C },
    };
    const reconciled = await service.reconcile(intent().operationKey);

    expect(reconciled.kind).toBe('replayed');
    expect(adapter.executeCalls).toHaveLength(2);
    expect(adapter.executeCalls[0]).toBe(adapter.executeCalls[1]);
  });

  it('never retries unknown work when the remote adapter has no safe reconciliation capability', async () => {
    const store = new MemoryGatewayOperationStore();
    const adapter = new TestAdapter();
    adapter.supportsOutcomeReconciliation = false;
    adapter.supportsRemoteIdempotencyKey = false;
    adapter.nextExecute = { kind: 'outcome_unknown', reason: 'remote_timeout' };
    const service = coordinator(store, adapter);

    await service.executeOnce(intent());
    const result = await service.reconcile(intent().operationKey);

    expect(result.kind).toBe('outcome_unknown');
    expect(adapter.executeCalls).toHaveLength(1);
    expect(adapter.reconcileCalls).toHaveLength(0);
  });

  it('defers if the operation ledger is unavailable', async () => {
    const store = new MemoryGatewayOperationStore();
    store.unavailable = true;
    const adapter = new TestAdapter();
    const result = await coordinator(store, adapter).executeOnce(intent());

    expect(result).toEqual({ kind: 'deferred', reason: 'operation_ledger_unavailable' });
    expect(adapter.executeCalls).toHaveLength(0);
  });

  it('one of 32 concurrent duplicate attempts can start the external operation', async () => {
    const store = new MemoryGatewayOperationStore();
    const adapter = new TestAdapter();
    let release!: () => void;
    const gate = new Promise<void>((resolve) => { release = resolve; });
    adapter.execute = vi.fn(async (request, context) => {
      adapter.executeCalls.push(context.operationKey);
      await gate;
      return {
        kind: 'committed' as const,
        receipt: { externalReceiptId: 'ehr-receipt-concurrent', resultDigest: DIGEST_C },
      };
    });
    const service = coordinator(store, adapter);

    const tasks = Array.from({ length: 32 }, () => service.executeOnce(intent()));
    await Promise.resolve();
    release();
    const results = await Promise.all(tasks);

    expect(adapter.executeCalls).toHaveLength(1);
    expect(results.filter((r) => r.kind === 'committed')).toHaveLength(1);
    expect(results.filter((r) => r.kind === 'in_progress')).toHaveLength(31);
  });

  it('keeps a committed remote operation unresolved if durable completion write fails', async () => {
    const store = new MemoryGatewayOperationStore();
    store.failNextCommit = true;
    const adapter = new TestAdapter();
    const service = coordinator(store, adapter);

    const first = await service.executeOnce(intent());
    const second = await service.executeOnce(intent());

    expect(first.kind).toBe('deferred');
    expect(second.kind).toBe('in_progress');
    expect(adapter.executeCalls).toHaveLength(1);
  });
});
