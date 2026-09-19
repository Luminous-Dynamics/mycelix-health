# Symthaea Clinical Distribution Assessment V2 Qualification

## Subject

Qualify the exact head implementing `mycelix-symthaea-clinical-distribution-assessment-v2`.

## Software gates

The exact subject must execute and pass:

- `cargo fmt --all -- --check`;
- focused `cargo check -p mycelix-symthaea-clinical-distribution-assessment-v2 --all-targets`;
- focused `cargo clippy -p mycelix-symthaea-clinical-distribution-assessment-v2 --all-targets -- -D warnings`;
- focused tests for the crate;
- the full repository CI gates required by the aggregate qualification PR.

A queued workflow is not qualification evidence.

## Exact-binding adversarial cases

The end-to-end test path must begin from the frozen Symthaea v2 binary conformance vector and construct the real opaque model assertion through independent Mycelix verification, deployment admission, local `ClinicalFact` snapshot binding, and evidence-context composition.

Against that real assertion, reject at least:

- model-assertion digest substitution;
- Symthaea wire-digest substitution;
- evidence-context-digest substitution;
- admission-policy-digest substitution;
- subject substitution;
- model-identity substitution.

## Distribution-policy adversarial cases

Reject at least:

- detector/evaluator substitution;
- reference-population substitution;
- reference-domain substitution;
- zero/invalid artifact or evidence identities;
- unsupported schema/identity versions;
- stale assessment;
- excessive future skew;
- invalid current time.

## Status/evidence adversarial cases

Reject at least:

- non-finite detector score/threshold;
- `InDistribution` with a numerical boundary that implies OOD;
- `OutOfDistribution` with a numerical boundary that implies in-distribution;
- detector-produced status without exact assessment evidence.

Preserve `NotRun`, `Unavailable`, `Indeterminate`, `InDistribution`, and `OutOfDistribution` as distinct states.

## Identity requirements

Demonstrate that changing any of these changes the relevant serializer-independent identity:

- exact model identity or lineage;
- detector identity;
- reference-domain identity;
- structural distribution policy;
- assertion/wire/context/admission binding;
- distribution status;
- numeric boundary;
- assessment evidence;
- assessment time.

## Downstream blocker

Passing this qualification does not close P0 #136.

Before any shadow-clinical composition, a separately reviewable runtime trust adapter must bind this exact validated assessment to:

- deployment evaluator admission;
- bounded known-revocation observation;
- positive short-lived evaluator currentness;
- exact detector and structural-policy identity;
- exact external admission evidence.

## Non-claims

A PASS proves software semantics for the exact subject only. It does not establish evaluator scientific validity, model correctness, calibration, clinical utility, patient applicability, threshold validity, regulatory clearance, practitioner authority, clinician presentation, diagnosis, or treatment authority.
