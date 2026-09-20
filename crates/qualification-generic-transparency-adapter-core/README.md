# Mycelix Health Generic Transparency Adapter Core

QUAL-EVID-007 / Health #221 freezes the semantic compatibility contract between Mycelix Health's external anti-rollback theorem (#216/#220) and the domain-neutral monotonic transparency protocol extracted in `Luminous-Dynamics/luminous-dynamics#1962` / draft PR #2023.

## Core theorem

```text
qualified generic current-head artifact
+ exact Health domain/subject/state mapping
+ separately qualified Health security-time floor
+ Health's own #220 reconciliation
=
Health-compatible VerifiedExternalAnchorProof
```

Similar fields are not sufficient interoperability.

## Exact state mapping

The generic state commitment is derived from a canonical V1 transcript containing, in order:

1. `MYCELIX-HEALTH-QUALIFICATION-ANTI-ROLLBACK-STATE-V1\0`;
2. fixed Health generic domain ID;
3. Health lineage-binding digest;
4. Health checkpoint digest;
5. checkpoint epoch as big-endian `u64`;
6. #217 head sequence as big-endian `u64`;
7. lineage-state commitment;
8. #217 verified-head digest.

The generic subject ID is exactly the opaque Health lineage-binding digest. It is not a patient identifier and no PHI is included.

## Three evidence boundaries

The adapter requires independent typed inputs:

- `VerifiedGenericAcceptedAnchorV1`: one already-current/conflict-free generic accepted anchor;
- `VerifiedGenericStateCommitmentDeriverV1`: qualified derivation of the generic state commitment from the exact transcript;
- `VerifiedHealthSecurityTimeFloorV1`: rollback-resistant Health time-floor evidence bound to the exact generic anchor/state.

There is no caller-supplied `verified: bool`.

## Conservative recovery policy

V1 rejects generic heads carrying a recovery authority epoch or resolved-recovery receipt. Generic conflict recovery does not automatically imply that Health policy accepts the recovered lineage.

A future adapter version may admit recovered heads only after their recovery identity/history is explicitly mapped and qualified.

## Defense in depth

Even after this adapter binds a generic head, Health #220 independently rechecks:

- external protocol/domain identity;
- sequence/predecessor shape;
- external freshness;
- security-time floor;
- local versus externally anchored state;
- checkpoint epoch/head sequence;
- exact prepare/finalize identity.

This adapter does not replace Health reconciliation.

## Cross-repo dependency rule

This crate intentionally does not depend on draft #2023. It freezes the accepted artifact contract that #2038 should eventually expose from the generic core. A production cross-repo dependency should be introduced only after both sides have independently qualified evidence.

```text
#2023 PASS
!= #220 PASS
!= #221 PASS
```

## Non-claims

This reference does not establish the generic backend's signatures/quorum/trust correctness, state-commitment cryptography, trusted time implementation, durable storage, Patient-v2 activation, clinical validity, or regulatory compliance.
