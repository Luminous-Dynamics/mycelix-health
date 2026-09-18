# Symthaea Clinical Admission Preflight V1

## Status

Draft deployment admission preflight layered on top of the independent Symthaea clinical wire verifier.

This layer answers whether one exact canonical Symthaea inference matches one exact Mycelix deployment policy. It does **not** grant clinician-presentation authority.

## Separation of proof lines

```text
canonical Symthaea wire
        ↓
independent Mycelix wire verification
        ↓
SymthaeaClinicalAdmissionPolicyV1
        ↓
AdmittedSymthaeaInferenceV1
        ↓
preflight evidence only
        ↓
separate distribution trust / authority / clinical promotion
```

`AdmittedSymthaeaInferenceV1` is deliberately non-serializable and single-owner, but that does not make it a clinical capability. Its type and API represent preflight evidence only.

## Exact deployment pinning

The v1 policy pins:

- exact wire identity version;
- exact envelope schema version;
- exact Symthaea engine artifact;
- complete model identity, including input/output schemas and training/evaluation/calibration lineages;
- exact runtime digest;
- exact configuration digest;
- exact operation;
- admitted claim kinds;
- admitted evidence stages;
- exact intended-use class;
- required subject namespace when applicable;
- maximum inference age;
- bounded future clock skew;
- whether calibrated probability is required;
- a Mycelix-owned qualification ceiling.

Policy vectors are part of the exact policy identity; their ordering is therefore intentionally digest-significant in v1.

## No evidence-stage self-promotion

`SymthaeaAdmissionCeilingV1` contains only:

- `Experimental`
- `ValidatedOffline`
- `ShadowClinical`

There is intentionally no `SupervisedClinical` variant.

The ceiling is selected by Mycelix deployment policy and is never computed from Symthaea's `ClinicalEvidenceStage`. A producer may claim `ReplicatedClinicalEvidence`; admission still cannot exceed the exact Mycelix ceiling configured by policy.

## Execution binding

V1 rejects post-hoc attachment/rebinding:

- every claimed evidence digest must occur in `execution.input_evidence_digests`;
- when a subject binding exists, its binding-evidence digest must also occur in the execution inputs.

This prevents an inference from being decorated with evidence that was not part of its recorded execution and prevents an existing inference from being rebound to another subject through a new binding object.

## Temporal checks

Admission fails closed when:

- generation precedes recorded execution;
- generation is beyond bounded future skew;
- the inference exceeds deployment maximum age.

The software absolute bounds are one year maximum age and five minutes maximum future skew; deployments may choose much shorter values.

## Calibration checks

When policy requires calibrated probability:

- the inference must declare `Calibrated`;
- the model identity must carry calibration-evidence identity;
- the inference uncertainty must carry calibration-evidence identity;
- those exact calibration evidence identities must agree.

This still does not prove calibration quality or external validity. It proves only exact lineage agreement under the admitted artifact.

## Distribution semantics

A producer-declared `OutOfDistribution` clinical-decision-support inference is rejected immediately.

A producer-declared `InDistribution` value is **not trusted as distribution assurance**. Independent Mycelix distribution assessment, evaluator trust, DNA-rooted runtime admission, and revocation handling remain separate required proof lines.

## Missing evidence

For `ClinicalDecisionSupport`, explicit critical missing evidence blocks admission preflight.

Research-only handling may preserve incomplete artifacts for analysis without implying clinical usability.

## Preflight identity

Successful admission binds a non-serializable preflight object to:

- exact Symthaea wire digest;
- exact Mycelix admission-policy digest;
- exact Mycelix qualification ceiling;
- exact admission-evaluation time.

A domain-separated preflight digest is exposed for later evidence binding, but it is not a presentation credential.

## Non-claims

V1 does not establish:

- producer authenticity by itself;
- scientific correctness;
- clinical effectiveness;
- representative training/evaluation populations;
- external calibration;
- independent OOD validity;
- regulatory status;
- clinician authority;
- diagnosis/treatment authority;
- permission to present to a clinician or patient.
