# QUAL-EVID-005 Boundaries

## Established by this reference

The health-side reconciliation theorem is limited to these facts:

1. local #217 state is reduced to a privacy-minimal opaque anchor state;
2. prepare records do not authorize product use;
3. bootstrap is explicit and restricted to checkpoint epoch 1;
4. successor anchor sequence and predecessor are exact;
5. checkpoint epoch rollback/gaps and same-epoch substitution fail closed;
6. security-time floor cannot move backward;
7. external anchor freshness is required at reconciliation and product consumption;
8. finalize binds state + sequence + predecessor + nonce + time floor exactly;
9. restart reconciliation distinguishes local rollback, anchor-behind and conflict;
10. only an anchored head may enter `AnchoredProfile208ProductAuthorityGate`.

## Delegated external theorem

`VerifiedExternalAnchorProof` is not cryptographic proof by itself. Its production implementation must prove:

- canonical external statement identity;
- signature validity;
- signer/witness eligibility;
- quorum and independence requirements;
- trust/revocation state;
- monotonic external history;
- predecessor/sequence continuity;
- nonce/replay handling;
- equivocation/conflict evidence;
- witness-set rotation;
- recovery semantics.

The generic domain-neutral protocol is now source-staged in `Luminous-Dynamics/luminous-dynamics#1962` / draft PR #2023 as `mycelix-monotonic-transparency-core`.

**That generic theorem is not automatically a Health adapter.** Exact cross-repository interoperability is tracked separately by Health #221 and the mapping contract in `GENERIC_TRANSPARENCY_ADAPTER_V1.md`.

```text
#2023 qualified generic semantics
!=
#220 qualified Health reconciliation
!=
#221 qualified cross-repo adapter
```

## Generic adapter requirements

A production Health adapter must bind an exact versioned mapping between:

- Health qualification-lineage identity and generic domain/subject identity;
- Health checkpoint epoch and generic state epoch;
- Health checkpoint/head commitments and one canonical generic state commitment;
- generic anchor sequence/predecessor/nonce/digest and the Health proof contract;
- generic witness-set/quorum/trust identities and Health witness metadata;
- generic conflict/recovery state and Health policy.

Health continues to re-check local current-head lifetime and exact local/external reconciliation even when the generic core has already verified monotonic history.

## Availability rule

Backend unavailable, stale, ambiguous or conflicting means **no anchored authority**. Availability failure must not be converted into local-only authority.

## Non-authority objects

These are not authority:

- `LocalQualificationHead`;
- `ExternalAnchorState`;
- `AnchorUpdateIntent`;
- an unverified backend response;
- a locally newer checkpoint with an older external anchor;
- a generic monotonic head without the qualified #221 Health adapter.

Authority-bearing output begins only at `AnchoredQualificationHead`, after a verified backend proof is reconciled exactly.

## Remaining non-claims

No claim is made about backend cryptographic qualification, #221 interoperability qualification, hardware tamper resistance, global consensus, endpoint compromise, production clock correctness, Patient-v2 activation, clinical validity, or compliance.
