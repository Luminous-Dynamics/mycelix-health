# Clinical Population Release v1

## Status

Experimental protected-boundary contract. Not a de-identification certification, not a universal privacy guarantee, and not authority to publish PHI.

## Goal

Population evidence can contain case counts, cohort counts, strata and analysis outputs that are useful for medicine/research but unsafe to expose by default. V1 creates a release gate with only two supported modes:

1. `StaticPreSpecifiedReport`
2. `InteractiveDifferentialPrivacy`

There is intentionally no threshold-only interactive-query mode.

## Static pre-specified reports

A static release binds:

- one exact population finding;
- one exact report specification;
- the exact protected aggregate result;
- the exact public output;
- a deployment cell threshold;
- primary suppression evidence;
- complementary/secondary suppression evidence;
- a composition/differencing review artifact.

The software floor is 11. This mirrors the conservative shape of the CMS cell-suppression rule that beneficiary-related cells 1-10 must not be published or derivable. It is **not** a claim that 11 is universally sufficient. Deployment/jurisdiction/research policy may only require a higher threshold.

Static threshold suppression is intentionally limited to a fixed/pre-specified report. It is not accepted as the privacy mechanism for repeated arbitrary slicing.

## Interactive differential privacy

An interactive release binds the exact:

- source population finding;
- query specification;
- output schema;
- public output;
- DP mechanism kind and implementation version;
- implementation digest;
- privacy-unit definition;
- adjacency/neighbouring-dataset definition;
- contribution/clipping bounds;
- sensitivity analysis;
- randomness-source evidence;
- per-release epsilon/delta;
- deployment-selected accountant instance/method;
- predecessor accountant receipt;
- cumulative privacy loss/query count;
- accountant evidence;
- release policy.

NIST SP 800-226 is the design reference for treating differential privacy as a collection of exact guarantees/implementation assumptions rather than a `dp=true` property.

## Privacy parameters

V1 represents epsilon/delta as reduced rational numbers rather than floating-point values. This avoids NaN/infinity/serialization ambiguity and gives one canonical representation for artifact hashing.

The release gate checks:

- delta is within `[0,1]`;
- the mechanism is explicitly allowed by policy;
- per-release privacy loss does not exceed policy;
- cumulative reported privacy loss does not exceed policy;
- query count does not exceed policy;
- accountant lineage is contiguous;
- cumulative reported loss cannot decrease;
- the successor accountant receipt binds the exact query, mechanism and public output.

The gate does **not** implement a differential-privacy accountant or attempt to reproduce its mathematics. The accountant method/version/evidence are explicit inputs.

## Accountant trust boundary

`PrivacyAccountantReceiptV1` is serializable evidence. A malicious caller can serialize a structurally plausible receipt.

Therefore the pure v1 crate proves only:

- canonical receipt shape;
- exact policy binding;
- exact predecessor lineage;
- monotonic bounded counters/loss;
- exact query/mechanism/output binding.

Promotion requires a verifier-owned trust layer that establishes the deployment-selected accountant implementation/state root and prevents arbitrary genesis/state fabrication.

## Release capability

Only a validated request can create the non-serializable `PopulationReleaseCapabilityV1`. Consuming it creates a `PopulationReleaseReceiptV1` bound to the exact request/policy/source/output/mode and release time.

This reduces accidental bypass in safe code. It is not a defense against hostile code that can fabricate every upstream evidence artifact.

## Epistemic invariants

Privacy release never changes scientific evidence class:

- a `SafetySignalCandidateV1` remains hypothesis-generating;
- a `DenominatorBasedAssociationV1` remains an association;
- a `CausalEffectEstimateV1` remains an estimate;
- a `PopulationReplicationV1` remains replication evidence.

Publication is not epistemic promotion.

## Privacy non-claims

V1 does not claim:

- that cell threshold 11 alone prevents re-identification;
- that secondary suppression evidence is correct merely because a digest exists;
- that a DP implementation is correct merely because its descriptor is well formed;
- that an accountant receipt is institutionally trusted merely because its chain is well formed;
- that public output contains no other disclosure channel;
- that composition across independent release systems is automatically accounted for;
- that a released aggregate is clinically or scientifically correct.

## Promotion blockers

Before broad production release:

1. qualify suppression/composition review authority;
2. qualify trusted accountant admission/state roots;
3. execute exact-head CI;
4. execute differencing and overlapping-query adversarial tests;
5. execute accountant replay/fork/budget tests;
6. independently review DP mechanism implementations and sensitivity/contribution bounds;
7. review jurisdiction-specific disclosure requirements;
8. ensure released outputs are the minimum necessary for the approved purpose.
