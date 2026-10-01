# Clinical Population Accountant Trust v1

## Purpose

Turn privacy-accountant state from a caller-supplied serializable claim into a DNA-rooted, append-only trust lineage suitable for eventually gating interactive differential-privacy release.

This tranche addresses the core threat in P0 #90: a structurally coherent `PrivacyAccountantReceiptV1` is not automatically institutionally trusted budget state.

## Protected vs public state

The full `PrivacyAccountantReceiptV1` remains protected. It may identify the exact query, DP mechanism, and output.

The DHT stores only a privacy-minimized projection:

- release-policy digest;
- accountant instance digest;
- accountant method digest;
- sequence and query count;
- cumulative epsilon/delta;
- exact predecessor state action;
- secret-keyed commitment to the protected receipt.

The secret commitment key never appears on the DHT.

## DNA-rooted authority

`population_accountant.root_authorities` is packaged in DNA properties and is empty by default.

A root may create only a short-lived `PopulationAccountantVerifierAuthorization` for:

- one exact verifier;
- one exact public state digest;
- one exact release policy;
- one exact accountant instance/method;
- one exact sequence and predecessor;
- one exact private-receipt commitment;
- one bounded validity window.

The integrity zome hard-caps authorization duration at 15 minutes.

## Transition validation

For non-genesis states the integrity zome requires:

- the exact predecessor action to exist and decode as trusted accountant state;
- same release-policy/accountant-instance/accountant-method lineage;
- `sequence = predecessor.sequence + 1`;
- `query_count = predecessor.query_count + 1`;
- component-wise non-decreasing cumulative epsilon/delta;
- a new private-receipt commitment;
- exact match to a DNA-root-authorized public projection.

Genesis requires sequence/query count/privacy loss all equal to zero and no predecessor.

## Corrections

Updates and deletes are forbidden. Corrections are append-only and root-authored.

Typed reasons include:

- `EnteredInError`;
- `AccountantImplementationRevoked`;
- `AccountantMethodRevoked`;
- `PolicySuperseded`;
- `DuplicateState`;
- `Other` (requires protected rationale commitment).

## Fork-preserving reducer

`mycelix-clinical-population-accountant-state` never resolves by timestamp or insertion order.

It returns bounded-read states including:

- `ObservedCurrentWithinRead`;
- `ConflictWithinRead` for multiple genesis states, sibling successors, or multiple live leaves;
- `NoCurrentStateWithinRead` for corrected/superseded state;
- `ReviewRequiredWithinRead` when a live descendant depends on semantically invalidated ancestry.

Missing predecessors fail closed.

## Non-claims

This tranche does **not** yet prove global DHT completeness. The reducer can detect only states in the supplied read set.

A high-assurance interactive release therefore still needs:

1. canonical commitment/accountant-lineage state discovery;
2. bounded stable materialization of every admitted state/correction;
3. protected re-binding of the private `PrivacyAccountantReceiptV1` to the public keyed commitment;
4. integration that requires the resulting trusted accountant capability before interactive release.

Until those are complete, #90 remains a promotion blocker for interactive DP release.

## Threat model principle

A DHT-valid transition is evidence that a DNA-authorized verifier published one exact accountant-state projection. It is not proof that unseen sibling states do not exist, and the public keyed commitment is not independently reversible into the protected receipt.
