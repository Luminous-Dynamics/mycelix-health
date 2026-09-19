# Clinical Distribution Runtime Trust V1 — Qualification Checklist

## Integrity / DNA gates

- [ ] packaged DNA has `clinical_distribution_trust.root_authorities: []` and therefore fails closed by default;
- [ ] root config schema version is exact;
- [ ] configured verifier-authorization duration is >0 and <=15 minutes;
- [ ] non-root cannot issue evaluator verifier authorization;
- [ ] root authorization binds one exact verifier and one exact admission-proposal digest;
- [ ] authorization action timestamp must fall inside its own validity window;
- [ ] authorization duration above the DNA/hard cap fails;
- [ ] wrong verifier cannot publish an evaluator admission;
- [ ] admission outside the root authorization window fails;
- [ ] proposal substitution after authorization fails;
- [ ] expired evaluator proposal cannot be newly published;
- [ ] updates/deletes of authorization/admission/revocation entries fail;
- [ ] only DNA roots can create evaluator revocations;
- [ ] revocation must bind exact admission action + exact proposal digest;
- [ ] `Other` revocation requires a nonzero protected rationale commitment;
- [ ] revocation-link deletion fails;
- [ ] revocation link cannot cross admission lineage;
- [ ] referenced authorization is rechecked for DNA-root authorship, shape, and grant window when consumed;
- [ ] referenced admission is rechecked through its exact authorization chain before revocation is accepted;
- [ ] referenced revocation author is rechecked as a DNA root before its link is accepted.

## Read/materialization gates

- [ ] admission fetch uses network-backed read;
- [ ] revocation-link reads use network strategy;
- [ ] duplicate links to the same revocation action are idempotent;
- [ ] non-ActionHash revocation target fails;
- [ ] more than 256 observed revocation links fails closed;
- [ ] every observed revocation target must materialize successfully;
- [ ] every observed revocation must target the requested admission;
- [ ] every observed revocation must bind the exact admission-proposal digest;
- [ ] revocation target set changing between first and second reads forces retry;
- [ ] returned snapshot carries `NetworkBackedStableDoubleRead` explicitly;
- [ ] returned snapshot carries observation start/end times and observed target count;
- [ ] callers never upgrade the snapshot to global completeness or unqualified `Active`.

## Exact source-entry-definition gate

Structural deserialization plus root-author containment is not the final source-type proof. Before production promotion:

- [ ] referenced verifier authorization action is proven to use the exact expected app entry definition from `clinical_distribution_trust_integrity`;
- [ ] referenced qualified admission action is proven to use the exact expected app entry definition;
- [ ] referenced revocation action is proven to use the exact expected app entry definition;
- [ ] no guessed/hard-coded zome index silently changes when DNA ordering changes;
- [ ] conductor test shows a structurally compatible entry from another definition is rejected.

Until this gate is qualified, the runtime zome remains draft even though root-author revalidation contains the direct non-root substitution path.

## Trusted-adapter gates

Before constructing `VerifiedDistributionEvaluatorAdmission` from a runtime snapshot:

- [ ] adapter accepts only the exact snapshot type from the conductor path;
- [ ] adapter preserves the bounded read marker;
- [ ] any observed semantic revocation prevents active admission;
- [ ] admission validity window is enforced at assessment/trust time;
- [ ] exact detector, structural-policy digest, trust-policy digest, and external-admission-evidence digest are mapped without substitution;
- [ ] admission evidence identity is bound to the actual DHT runtime record/action lineage;
- [ ] caller-supplied JSON/synthetic snapshot cannot substitute for conductor provenance;
- [ ] modified-adapter/conductor tests cannot create verified admission without a DHT-valid root-authorized record.

## Non-claims / promotion rule

A stable double read does not prove global DHT completeness or absence of an unpropagated revocation. Passing this checklist does not establish detector scientific validity, model clinical effectiveness, regulatory clearance, or presentation authority.

Keep this tranche draft until exact-head CI and conductor/adversarial tests pass. Model-backed `ClinicalPresentationPermit` issuance remains disabled until the trusted adapter and final promotion composition are separately qualified.
