# Medication Dispense Slot Adjudication v1

## Purpose

A dispense preflight proves local clinical/authority/evidence coherence. It does not prove exclusive ownership of a refill slot.

This tranche adds the first runtime serialization boundary for that scarce resource.

## Why DHT observation is not a lock

Holochain provides agent-local atomic source-chain transactions and deterministic DHT validation, but validators do not share one globally linearizable database view. A validation rule that asks "does no competing slot record exist anywhere?" is therefore not a sound uniqueness primitive.

Holochain countersigning is also not treated as a double-spend oracle. The upstream documentation explicitly distinguishes atomic multi-party agreement from prevention of double spending. Countersigning is additionally behind the unstable-countersigning feature in the Holochain 0.6.x line used by this workspace.

V1 therefore does not make countersigning a safety-critical dependency.

## V1 model: deterministic narrow allocator

DNA properties contain an ordered set of allocator AgentPubKeys:

```yaml
medication_dispense:
  schema_version: 1
  slot_allocators: []
  max_slot_authorization_duration_micros: 300000000
```

The empty set intentionally disables finalized dispensing.

When configured, a MedicationRequest artifact selects exactly one allocator by:

1. validating the digest as `MedicationRequestArtifact`;
2. reading the first eight digest bytes as big-endian `u64`;
3. taking modulo the DNA-pinned allocator-list length.

The allocator-list ordering is part of DNA properties and therefore part of the DNA hash.

This serializes only one medication-order/refill lineage at a time. It does not introduce a global healthcare lock.

## Exact one-shot authorization

`MedicationDispenseSlotAuthorization` is short-lived and bound to:

- one allocator-selected medication artifact;
- one activation semantic receipt;
- one activation-state snapshot digest;
- one dispense request;
- one dispense preflight receipt;
- one slot index;
- one pharmacist AgentPubKey;
- one private pharmacist-principal binding;
- one pharmacy record/affiliation/status evidence set;
- one authority policy;
- one dispense policy;
- one exact final semantic receipt digest;
- one validity window of at most five minutes.

The final dispense action timestamp must fall inside that window.

## Refill continuity

Slot 0 has no predecessor.

Slot N>0 must reference a DHT-valid `FinalizedMedicationDispense` for slot N-1 with the same medication artifact and activation semantic receipt.

This prevents a valid allocator from skipping over missing finalized slots or moving a prior dispense from another medication lineage into the chain.

## Finalization

`FinalizedMedicationDispense` is authored by the pharmacist AgentPubKey named as the authorization grantee. Its privacy-minimized attestation must hash to the exact `MedicationDispenseReceipt` authorized by the allocator and must reproduce the exact evidence/policy/slot fields from the authorization.

The final receipt attestation does not duplicate medication name, diagnosis, dose instructions, patient identity, or other raw PHI. Those details remain committed through the request/preflight/artifact digests.

A final receipt, unlike a preflight receipt, can be counted as a consumed slot by later dispense-ledger snapshots.

## Allocator equivocation

V1 makes an explicit trust assumption: the selected allocator is expected not to authorize two incompatible outcomes for the same semantic refill slot.

If that assumption is violated, `AllocatorEquivocationEvidence` can reference two DHT-valid slot authorizations. Validation independently verifies that both were authored by the same DNA-selected allocator and target the same medication + activation + slot while carrying conflicting authorization payloads.

This provides objective, append-only evidence of allocator misbehavior.

It does **not** prove that a conflicting physical release could not happen before the conflict propagated. That is a v1 non-claim.

## Safety / availability tradeoff

V1 intentionally chooses a small CP-like dependency for a very narrow scarce resource:

- when the selected allocator is reachable and non-equivocating, a refill slot can be serialized;
- when it is unavailable, final dispensing should fail closed or enter a separately designed emergency/manual process;
- another allocator does not automatically take over, because automatic failover without consensus can recreate the double-allocation problem.

Future availability work can add controlled allocator rotation or threshold witnessing, but it must preserve one authoritative slot decision per medication lineage.

## Why not unstable countersigning in v1

Countersigning could later be useful for pharmacist + allocator co-commitment or optional majority witnesses, but it is not the uniqueness primitive in v1 because:

- the current upstream feature is unstable;
- ordinary countersigning does not itself prevent double spending;
- optional witness/lightweight-consensus designs need their own liveness and failure analysis.

## Append-only model

Slot authorizations, finalized dispenses, and equivocation evidence are append-only. Updates/deletes are rejected by integrity validation.

V1 intentionally defines no discovery links. Privacy-safe discovery and a canonical dispense read model should be reviewed separately instead of leaking patient-medication relationships through convenient global indexes.

## Threat model / non-claims

V1 does not claim safety against a fully compromised selected allocator that intentionally equivocated before detection. It also does not claim that software can prevent a malicious pharmacy from physically handing out medication twice outside the system.

It does establish:

- deterministic allocator responsibility;
- no request-selected allocator;
- exact-target short-lived authorization;
- pharmacist-authored finalization;
- predecessor refill continuity;
- cryptographic separation of preflight vs slot authorization vs final receipt;
- objective allocator-equivocation evidence;
- fail-closed default deployment configuration.

## Qualification required

Before promotion, exact-head CI and conductor tests should prove:

- empty allocator configuration disables slot authorization;
- wrong allocator cannot authorize a slot;
- deterministic allocator selection is stable for fixed DNA properties;
- slot 0 rejects a predecessor;
- slot N>0 requires finalized N-1 in the same medication/activation lineage;
- wrong pharmacist cannot finalize;
- stale authorization cannot finalize;
- request/preflight/policy/pharmacy/slot substitution fails;
- finalized receipt digest must exactly match authorization;
- update/delete attempts fail;
- conflicting same-slot authorizations generate verifiable equivocation evidence;
- modified coordinators cannot bypass the integrity rules.
