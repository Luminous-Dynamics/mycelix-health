# Symthaea Clinical Distribution Runtime Trust V3

## Status

Structural runtime-trust contract for assertion-native Symthaea clinical distribution/OOD evidence.

This contract composes two independently established proof lines:

1. `ValidatedSymthaeaClinicalDistributionAssessmentV2`, which binds one OOD/distribution result to the exact Symthaea model assertion, wire artifact, evidence context, admission policy, subject, model identity, detector, reference domain, and structural distribution policy; and
2. `VerifiedDistributionEvaluatorCurrentnessV1`, which binds one exact DNA-rooted evaluator admission to both a bounded revocation observation and a positive short-lived evaluator lease.

The output is `SymthaeaDistributionTrustReceiptV3`, an opaque, non-cloneable, non-serializable proof value.

## Trust theorem

A v3 receipt may be produced only when all of the following are true:

- the validated assessment belongs to the supplied Symthaea structural distribution policy;
- the deployable evaluator trust policy binds the canonical v3 runtime bridge of that exact structural policy;
- the deployable evaluator trust policy binds the canonical v3 runtime bridge of that exact detector;
- the runtime currentness proof belongs to the supplied currentness policy;
- the runtime currentness proof binds the same canonical detector bridge;
- the runtime currentness proof binds the same canonical structural-policy bridge;
- the runtime currentness proof binds the exact evaluator trust-policy digest;
- evaluator currentness was established no later than the assessment time;
- evaluator currentness had not expired at assessment time;
- the assessment existed no later than trust-evaluation time; and
- the same evaluator-currentness proof remained active at trust-evaluation time.

The central ordering invariant is:

`currentness.verified_at <= assessment.assessed_at < currentness.valid_until`

with:

`assessment.assessed_at <= trusted_at < currentness.valid_until`.

This is a lease-before-use rule. An assessment produced before positive evaluator currentness exists cannot become trusted retrospectively merely because the evaluator is admitted or leased later.

## Canonical bridge identities

The generic runtime admission machinery uses `ArtifactIdentity` and `ContentDigest`, while the Symthaea-v2 structural path uses versioned fixed-width identities.

V3 does not reinterpret or relabel the raw 32-byte values.

Instead:

- the complete versioned `DistributionArtifactIdentityV2` is framed and BLAKE3-hashed under `mycelix.health.symthaea-distribution-detector-runtime-bridge.v1`; and
- the already domain-separated Symthaea structural-policy digest is BLAKE3-hashed again under `mycelix.health.symthaea-distribution-policy-runtime-bridge.v1`.

Both bridge results use the runtime `blake3-256` content-digest representation. Evaluation re-derives both values independently and rejects caller-supplied trust policies or runtime-currentness proofs that differ.

## Receipt bindings

The v3 receipt and receipt digest retain all of the following explicitly:

- exact model-assertion digest;
- exact Symthaea wire digest;
- exact evidence-context digest;
- exact Symthaea admission-policy digest;
- exact subject identity;
- exact model-identity digest;
- exact validated distribution-assessment digest;
- exact Symthaea structural-policy digest;
- canonical runtime detector identity;
- canonical runtime structural-policy digest;
- exact evaluator trust-policy digest;
- exact external evaluator-admission evidence digest;
- exact currentness-policy digest;
- exact currentness-proof digest;
- assessment status;
- assessment time;
- currentness establishment time;
- currentness expiry time; and
- trust-evaluation time.

This explicit redundancy is intentional. Downstream composition can compare important boundaries directly without reconstructing hidden producer state.

## Authority boundary

A v3 receipt is distribution-evaluator trust evidence only.

It does not grant:

- clinician presentation authority;
- patient-facing notification authority;
- diagnosis authority;
- prescribing or treatment authority;
- medication administration authority;
- autonomous action authority; or
- permission to upgrade a research or shadow-clinical inference into a clinical recommendation.

No v3 API writes a diagnosis, prescription, treatment plan, CDS card, patient alert, or clinical record.

## Non-claims

A valid v3 receipt does **not** prove that:

- the model is clinically valid or clinically effective;
- the model is calibrated for the patient or target population;
- `InDistribution` means the inference is correct or safe;
- an OOD detector is scientifically adequate merely because its runtime identity is trusted;
- the bounded revocation observation is globally complete;
- no later revocation exists elsewhere on the network;
- the positive lease remains valid after the recorded currentness expiry;
- the underlying dataset or reference population is unbiased or representative;
- any regulatory requirement has been satisfied; or
- passing software tests constitutes clinical validation or regulatory clearance.

## Relationship to legacy trust-v2

`mycelix-clinical-distribution-trust-v2` remains the legacy capsule-bound path. V3 does not wrap or upgrade its `ClinicalEvidenceCapsule` trust receipt.

The assertion-native path is:

`VerifiedSymthaeaModelAssertionV1`
→ `ValidatedSymthaeaClinicalDistributionAssessmentV2`
→ `VerifiedDistributionEvaluatorCurrentnessV1`
→ `SymthaeaDistributionTrustReceiptV3`.

Keeping the paths separate prevents an implicit conversion from a probabilistic Symthaea model assertion into a legacy clinical evaluation state.

## Qualification boundary

Unit tests can establish structural composition behavior and exact fail-closed substitutions. They cannot establish DNA-root authenticity, conductor/network behavior, global revocation completeness, clinical effectiveness, or regulatory status.

Runtime qualification must therefore preserve the independently qualified DNA-rooted admission, lease, bounded-read, and currentness evidence lineages and bind the exact component head used by this v3 contract.