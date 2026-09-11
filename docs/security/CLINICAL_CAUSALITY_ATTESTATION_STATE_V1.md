# Clinical Causality Attestation State v1

Status: experimental/draft. This reducer is not a substitute for DHT query completeness, clinical review, or regulatory interpretation.

## Purpose

Reduce already DHT-valid causal attestation publications and append-only corrections without flattening them into `current: bool` or using last-write-wins.

The public semantic lineage key is the secret-keyed qualified-receipt commitment. The protected receipt preimage remains outside the DHT.

## State semantics

For one receipt commitment:

- equivalent publications with different `attestation_id` values but the same qualification policy and conclusion are idempotent semantic publications;
- different current conclusions or qualification policies are `ProjectionConflict`;
- `DuplicateAttestation` suppresses only the targeted publication action;
- `PolicySuperseded` yields `Superseded` while preserving history;
- `EnteredInError`, `SourceEvidenceRevoked`, `AssessorAuthorityRevoked`, and `EvidenceTrustRevoked` yield `Invalidated` for the semantic commitment lineage;
- `Other` yields `ReviewRequired` when no stronger typed invalidation exists;
- if all known publications are duplicate-suppressed, the lineage is `PublicationSuppressed`.

Known semantic invalidation dominates unknown-effect review, which dominates policy supersession. All correction evidence remains preserved regardless of the derived lifecycle.

## Fail-closed behavior

- same action hash with different attestation payload -> error;
- same correction action hash with different correction payload -> error;
- reused `correction_id` across actions -> error;
- correction target missing from the supplied read set -> error;
- correction receipt commitment differs from its target -> error;
- malformed commitment or qualification-policy digest -> error.

## Read-set boundary

The reducer checks reference closure inside the supplied set. It cannot prove that the caller fetched every correction from the DHT. A runtime adapter must provide a complete-for-purpose DHT-valid read set before this result may be used as current clinical/research state.

## Commitment-key rotation

A secret-keyed commitment intentionally prevents the public DHT from recovering the private receipt identity. Consequently, changing commitment keys may produce a different public commitment for the same private receipt. V1 does not add a public cross-key linkage because doing so would weaken unlinkability.

Deployments that need continuity across key rotation must handle it inside the protected verifier environment or through a separately reviewed privacy-preserving migration protocol. The reducer must not guess equivalence across commitments.

## Non-claims

The reducer does not prove population causal effect, regulatory reportability, completeness of DHT history, physical truth of an observation, or equivalence across different keyed receipt commitments.
