# Generic Transparency Adapter Contract V1

This document freezes the intended compatibility boundary between Mycelix Health QUAL-EVID-005/#216/#220 and the domain-neutral monotonic transparency work in `Luminous-Dynamics/luminous-dynamics#1962` / draft PR #2023.

This file is a **mapping contract**, not evidence that the cross-repository adapter is qualified. The executable adapter is tracked separately by Health #221.

## Domain separator

The Health adapter must use its own generic transparency domain identity. It must not reuse verifier-transparency or fabrication domains.

Canonical domain label:

```text
MYCELIX-HEALTH-QUALIFICATION-ANTI-ROLLBACK-V1
```

The actual generic `DomainId` derivation algorithm and bytes must be frozen by #221 before production use.

## Subject identity

The generic `MonotonicSubjectId` must bind the exact Health qualification-lineage binding. It must not be derived from patient identity, agent identity, raw PHI, record category, or clinical content.

## Generic state mapping

Health `ExternalAnchorState` maps conceptually as follows:

| Health field | Generic field / commitment role |
| --- | --- |
| `lineage_binding_digest` | subject-identity input + state-commitment input |
| `checkpoint_epoch` | `SubjectState.state_epoch` |
| `checkpoint_digest` | state-commitment input |
| `head_sequence` | state-commitment input; independently rechecked by Health |
| `lineage_state_commitment` | state-commitment input |
| `verified_head_digest` | state-commitment input |

The generic `StateCommitment` must be computed from a versioned, domain-separated canonical transcript containing **all six** Health fields above in a frozen order/framing.

One-bit changes to any of those fields must change the generic state commitment.

## Anchor mapping

Generic monotonic anchor identity maps to the Health `VerifiedExternalAnchorProof` boundary:

- generic anchor sequence -> Health `anchor_sequence()`;
- generic predecessor digest -> Health `previous_anchor_digest()`;
- generic anchor nonce -> Health `anchor_nonce()`;
- generic anchor digest -> Health `anchor_digest()`;
- generic witness-set epoch -> Health `witness_set_epoch()`;
- generic witness-quorum digest -> Health `witness_quorum_digest()`;
- generic trust-snapshot digest -> Health `trust_snapshot_digest()`;
- generic observed/expiry window -> Health observed/expiry window.

Health must re-check sequence/predecessor/freshness/state reconciliation even when the generic core already checked them.

## Security-time floor

The generic core does not replace Health's qualification/security-time theorem. The Health adapter must supply or derive a separately qualified external security-time floor compatible with #220.

A fresh generic anchor never makes an expired/not-yet-valid #217 head authoritative.

## Conflict and recovery

If the generic core has unresolved `TemporalConflictEvidence`, there is no generic current head and therefore no Health `VerifiedExternalAnchorProof` capable of authorizing product use.

If a generic conflict has been governed/recovered, #221 must define which recovery identity/history Health requires before accepting the recovered head. Health must not silently treat recovery as if the conflict never occurred.

## Version drift

The executable adapter must fail closed on:

- unsupported generic protocol version;
- unsupported Health adapter version;
- changed Health or generic domain separators;
- changed subject-ID derivation;
- changed state-commitment framing or field set/order;
- unsupported witness/recovery semantics;
- schema changes not explicitly qualified by a new adapter version.

## Privacy

Only opaque commitments/counters cross into the generic transparency domain. No raw patient identity, PHI, clinical assertion, care-record payload, or qualification prose is required.

## Evidence rule

```text
#2023 green
    !=
#220 green
    !=
#221 interoperability qualified
```

The cross-repository adapter requires its own exact evidence lineage.
