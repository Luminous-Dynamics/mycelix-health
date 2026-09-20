# Qualification Receipt Core

Reference semantics for QUAL-EVID-001 (#209).

## Core theorem

```text
well-formed statement
!= authentic statement
!= trusted approver
!= policy-authorized bundle
!= fresh/non-replayed receipt
!= admissible qualification
```

This crate freezes the structural admission theorem that production cryptographic, trust-store, governance, time and persistence adapters must satisfy.

## Canonical statement

`QualificationStatement::transcript_bytes()` uses the domain:

```text
MYCELIX-HEALTH-QUALIFICATION-RECEIPT-V1
```

and binds:

- finite `ReceiptKind`;
- exact contract digest/version;
- qualification-lineage ID;
- subject digest;
- dependency-set digest;
- optional deployment-evidence digest;
- optional context/cutover digest;
- outcome;
- issue/expiry times;
- unique nonce;
- monotonic sequence;
- exact predecessor receipt;
- governance-policy digest;
- trust-store digest;
- finite admission scope;
- claim-profile digest.

Sequence 1 requires no predecessor; later sequence values require one.

## Finite receipt vocabulary

V1 includes:

- ReferenceQualification
- DeploymentQualification
- MigrationResultQualification
- CompositionCommitmentQualification
- DurablePrepareQualification
- DurableActivationQualification
- AuditCheckpointQualification
- MonitoringAssessmentQualification
- P0ClosureQualification

There is no free-form receipt-kind string.

## Governed admission

`GovernanceRule` independently constrains:

- minimum approvers;
- minimum distinct organizations;
- required roles;
- maximum attestation age;
- maximum statement lifetime.

Roles align with the existing validator-policy family:

- Clinical
- Privacy
- Compliance
- Safety

The policy additionally binds the exact policy digest, trust-store digest, and expected deployment evidence.

## Exact receipt binding

The public API does not expose the lower-level raw attestation verifier objects.

Every `BoundApproverVerification` names the exact `ReceiptDigest` whose canonical statement was verified. `verify_governed_qualification` refuses a mixed bundle containing an attestation verified for another receipt.

Only a successful governed bundle emits `VerifiedQualification`.

## Fail-closed checks

Admission rejects at least:

- unverified statement commitment;
- non-Qualified outcome;
- wrong receipt kind;
- policy/trust-store/deployment mismatch;
- not-yet-valid or expired statement;
- statement lifetime exceeding policy;
- duplicate approver;
- duplicate signer key;
- unverified signature adapter result;
- untrusted/inactive/revoked/compromised approver;
- attestation outside the statement window;
- future or stale attestation;
- insufficient approver threshold;
- insufficient organization diversity;
- missing required roles;
- attestation bound to a different receipt digest.

## Monotonic per-lineage ledger

`QualificationLedger` is deliberately scoped to one `QualificationLineageId`; it does not pretend to establish a single global distributed order.

For each lineage:

- first receipt is sequence 1 with no predecessor;
- later receipts require exact `head.sequence + 1`;
- predecessor must be the exact current head receipt;
- nonce reuse is rejected;
- stale/gapped sequence is rejected;
- clock rollback is rejected;
- exact replay of an already-admitted immutable receipt is idempotent.

The production persistence/checkpoint theorem remains separate.

## Supersession / abandonment

A separately verified governance disposition may mark an admitted receipt:

- Superseded; or
- Abandoned.

Inactive receipts are no longer admissible. Disposition epochs are monotonic and exact disposition replay is idempotent.

The reference models the admission semantics only; signature verification and durable/distributed disposition checkpoints remain external.

## Intended production integration

Production code should move away from APIs such as:

```text
from_verified_adapter(..., qualified = true)
```

and instead accept an immutable governed token derived from the verified receipt bundle, conceptually:

```text
VerifiedQualification<T>
```

The exact type adapter for #204/#206/#208 is follow-on integration work.

## Reused project foundations

This theorem is designed to extend—not replace—the patterns already present in:

- `docs/release/DEPLOYMENT_EVIDENCE_PROTOCOL.md`;
- `scripts/validator-policy.py`;
- `scripts/clinical-remediation-governance.py`;
- `scripts/clinical-remediation-approval.py`.

Those existing systems provide the direction for deterministic signing, signer/trust-store management, role/organization policy and offline approval ceremonies.

## Non-claims

This crate does not:

- implement Ed25519 or ML-DSA verification;
- load or validate the real production trust store;
- prove that an external `signature_verified` assertion is true;
- compute receipt hashes;
- provide trusted time;
- durably consume nonces;
- persist a distributed monotonic ledger;
- provide global consensus ordering;
- establish the truth of the underlying clinical/scientific theorem;
- establish legal/regulatory compliance.

It establishes only the structural canonicalization, governed-admission and per-lineage replay semantics for independently verified inputs.
