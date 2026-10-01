# Clinical Causality Attestation v1

Status: experimental/draft. Source review is not executable, clinical, regulatory, or deployment qualification.

## Purpose

A `QualifiedCausalAssessmentReceiptV1` is a private audit artifact. Deserializing it does not establish DHT provenance or authorize publication. This zome adds a separate DNA-rooted publication boundary.

## Public statement

The public DHT statement is intentionally narrow:

> A verifier explicitly authorized by a DNA-pinned causal root attested, within a short validity window, that one protected qualified causal receipt under one exact qualification policy had the stated conservative causal conclusion.

The DHT does not re-run clinical causal reasoning.

## Privacy boundary

The public attestation does **not** contain:

- patient identity or subject reference;
- medication/product identity;
- symptoms, diagnoses, laboratory values, dose, route, or administration time;
- observed-event or exposure-association contents;
- individual evidence-factor details;
- assessor identity;
- the raw private qualified-receipt digest.

Instead, the public path carries a secret-keyed opaque commitment to the qualified receipt. A deterministic public hash of the private receipt is deliberately avoided to reduce correlation and candidate-confirmation risk.

The root/verifier environment must protect the commitment key. The DHT validates only commitment shape and exact authorization binding; it cannot independently recompute a secret-keyed commitment.

## Root authorization

`clinical_causality.root_authorities` is read from DNA properties and ships empty by default. A deployment must intentionally repack the DNA with trusted root `AgentPubKey`s.

A root authorization is bound to one exact:

- verifier agent;
- opaque qualified-receipt commitment;
- public attestation digest;
- qualification-policy digest;
- causal conclusion;
- validity window.

The validity window is hard-capped at 15 minutes. It is not a reusable verifier role.

When an attestation consumes an authorization, validation also re-checks that the referenced authorization action was authored by a current DNA-pinned causal root. This contains much of the structurally-compatible-entry risk tracked in P0 #72, but exact source-entry-definition proof remains a promotion requirement.

## Append-only correction

Attestations, authorizations, corrections, and correction links cannot be updated/deleted. A correction is a separate root-authored entry bound to the exact attestation action and receipt commitment.

Typed correction reasons include:

- `EnteredInError`;
- `SourceEvidenceRevoked`;
- `AssessorAuthorityRevoked`;
- `EvidenceTrustRevoked`;
- `PolicySuperseded`;
- `DuplicateAttestation`;
- `Other` (requires a protected rationale commitment).

A later read model must preserve correction semantics; `DuplicateAttestation` should not necessarily mean the underlying private causal assessment was false.

## Non-claims

This zome does not prove:

- that the physical clinical event occurred;
- that the private evidence is true merely because a root verified its lineage;
- population-level causal effect;
- regulatory reportability;
- that public attestation counts are epidemiologically meaningful;
- perfect anonymity or unlinkability;
- exact cross-zome entry identity until P0 #72 is resolved;
- clinical qualification of the software.

## Future population evidence

Population pharmacovigilance must use a separate privacy-preserving export/aggregation layer. The causal-attestation DHT must not become an accidental case registry simply because aggregation is convenient.
