# Symthaea Clinical Admission Preflight v2

## Purpose

`mycelix-symthaea-clinical-admission-v2` applies an exact Mycelix deployment policy to an independently verified Symthaea binary clinical inference v2 artifact.

It is a preflight boundary only. It does not create clinical presentation authority.

## Independent prerequisites

The input wire bytes are first parsed and semantically verified by `mycelix-symthaea-clinical-wire-v2`, which has no Symthaea dependency.

The admission policy then independently pins what this Mycelix deployment accepts.

## Policy-bound identity

`SymthaeaClinicalAdmissionPolicyV2` pins:

- binary wire version;
- envelope version;
- exact engine identity;
- exact model identity;
- input/output schema digests;
- typed model training/evaluation/calibration lineages;
- runtime digest;
- configuration digest;
- execution operation;
- allowed claim kinds;
- allowed evidence stages;
- intended use;
- subject namespace;
- subject-binding evidence namespace;
- allowed claim-evidence namespaces;
- required claim-evidence namespaces;
- optional producer distribution-evidence namespace;
- maximum age/future skew;
- calibrated-probability requirement;
- Mycelix qualification ceiling.

## Serializer-independent policy digest

The policy identity does not use JSON or serde serialization.

It has an explicit binary framing over every policy field and is hashed under the BLAKE3 derive-key domain:

`mycelix.health.symthaea-clinical-admission-v2-policy.v1`

Changing an admitted evidence namespace, model lineage, runtime, use case, age bound or qualification ceiling therefore changes policy identity.

## Namespace admission

The independent wire verifier establishes that a typed evidence identity is structurally valid.

Admission v2 separately asks whether its namespace is acceptable to this deployment.

For claim evidence:

- every observed namespace must be in the allowed set;
- every namespace in the required set must actually be observed;
- every required namespace must also be allowed.

For clinical-decision-support policies, at least one required claim-evidence namespace is mandatory.

This prevents a deployment from silently accepting a newly introduced evidence domain just because a future producer can serialize it.

## Subject binding

Clinical-decision-support policy requires both:

- an exact subject namespace; and
- an exact subject-binding evidence namespace.

This still does not prove the binding source is trustworthy. The ClinicalFact binding and downstream trust layers remain separate.

## Calibration

When calibrated probability is required:

- inference status must be `Calibrated`;
- a calibrated probability must be present;
- model typed calibration evidence must be present;
- inference typed calibration evidence must be present;
- the two identities must match exactly.

## Distribution state

Producer distribution evidence may be namespace-pinned by policy.

Producer `OutOfDistribution` blocks clinical-decision-support preflight.

Producer `InDistribution` is not independent Mycelix OOD trust and grants no presentation authority. The separate distribution-assessment / evaluator-trust / currentness chain remains mandatory.

## Missing evidence

Critical missing evidence blocks clinical-decision-support preflight.

Missing evidence is preserved rather than converted into a negative finding.

## Qualification ceiling

The only v2 admission ceilings are:

- `Experimental`;
- `ValidatedOffline`;
- `ShadowClinical`.

There is intentionally no `SupervisedClinical` variant.

## Output

Successful admission produces a non-serializable `AdmittedSymthaeaInferenceV2` binding:

- exact independently verified wire digest;
- exact policy digest;
- preflight digest;
- Mycelix qualification ceiling;
- evaluation time.

It is evidence, not authority.

## Non-claims

Successful admission does not establish:

- ClinicalFact truth or binding;
- scientific validity;
- causal validity;
- external validation;
- independent OOD trust;
- evaluator currentness;
- practitioner authority;
- regulatory clearance;
- clinician-presentation authority;
- diagnosis or treatment authority.
