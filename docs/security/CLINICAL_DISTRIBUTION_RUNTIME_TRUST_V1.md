# Clinical Distribution Runtime Trust V1

## Status

Draft DNA-rooted runtime trust boundary stacked on #105 and #102.

This tranche makes evaluator admission a DHT-valid, DNA-root-authorized fact. It still does not unlock model-backed clinical presentation; conductor/adaptor qualification must demonstrate that only actual DHT-valid admission records are converted into the in-process verified-admission type.

## Trust split

The clinical-AI distribution chain is deliberately separated into four proof lines:

1. **Structural distribution preflight** (#102): exact capsule/model/subject/detector/reference-domain/freshness binding.
2. **Pure evaluator trust composition** (#105): exact trust policy + non-serializable verified admission + non-serializable trust receipt.
3. **DNA-rooted runtime admission** (this tranche): proves which evaluator admission records are valid in this deployment.
4. **Trusted conductor/adaptor qualification** (follow-up): proves the in-process verified-admission value was constructed from the exact DHT-valid runtime record and its observed revocation lineage.

No lower layer is silently upgraded into a higher one.

## Fail-closed DNA configuration

`dna/dna.yaml` adds:

```yaml
clinical_distribution_trust:
  schema_version: 1
  root_authorities: []
  max_verifier_authorization_duration_micros: 900000000
```

The empty root set intentionally disables evaluator admission in the packaged DNA. A test/production deployment must deliberately repack the DNA with trusted root `AgentPubKey`s, changing the DNA hash.

The integrity zome hard-caps exact-target verifier authorizations at 15 minutes even if a deployment attempts to package a larger value.

## Exact-target authorization

A DNA root may create `DistributionEvaluatorVerifierAuthorization` for exactly:

- one verifier `AgentPubKey`;
- one exact admission-proposal digest;
- one short validity window.

This is not a reusable evaluator role grant.

## Admission proposal

`DistributionEvaluatorAdmissionProposalV1` is independently digestible before an authorization exists and carries only trust metadata:

- schema version;
- admission ID;
- exact detector/evaluator artifact identity;
- exact structural distribution-policy digest;
- exact pure trust-policy digest;
- external admission-evidence digest;
- evaluator admission validity window.

It carries no patient identity, model output, diagnosis, clinical finding, or raw OOD result.

## Qualified admission

`QualifiedDistributionEvaluatorAdmission` contains:

- the exact admission proposal;
- the root-issued verifier-authorization `ActionHash`.

Integrity validation requires:

- DHT-valid referenced authorization;
- admission author equals authorization grantee;
- admission action timestamp falls inside the short authorization window;
- proposal digest exactly matches the root-authorized proposal;
- proposal is not already expired at publication.

A modified coordinator cannot bypass these rules because they live in integrity validation.

## Append-only revocation

Evaluator admissions are immutable. Updates and deletes are rejected.

A DNA root may append `DistributionEvaluatorAdmissionRevocation` bound to:

- exact admission action;
- exact admission-proposal digest;
- typed revocation reason;
- optional protected rationale commitment.

`Other` requires a nonzero rationale commitment.

Revocation links are append-only and must be authored by the revocation-record author.

Reasons include:

- entered in error;
- evaluator compromised;
- evaluator retired;
- trust policy superseded;
- distribution policy superseded;
- external admission evidence revoked;
- other with protected rationale commitment.

## Important Holochain semantics

`must_get_*` intentionally ignores later update/delete metadata. Therefore this design does **not** pretend that deleting an authorization or admission revokes historical authority.

Root authorizations are exact-target and short-lived. Evaluator admission lifecycle is represented by append-only revocation evidence that the read/adaptor layer must materialize.

## Trusted-adaptor boundary

The runtime DHT record itself is serializable. The non-serializable `VerifiedDistributionEvaluatorAdmission` in #105 must be constructed only after a trusted conductor/adaptor has:

- fetched the exact DHT-valid qualified admission action;
- confirmed the expected entry definition/zome identity where required;
- materialized its revocation links for the required observation boundary;
- rejected an admission whose active revocation lineage makes it unusable;
- mapped the exact proposal fields into the pure trust type without substitution;
- bound admission evidence to the actual runtime record identity.

A caller-supplied JSON object or fabricated snapshot is not equivalent to conductor provenance.

## Non-claims

This tranche does not establish:

- global DHT completeness;
- absence of an unpropagated revocation;
- scientific validity of the OOD detector;
- model clinical effectiveness;
- regulatory clearance;
- clinical presentation authority.

Model-backed clinician presentation remains blocked until the conductor/adaptor qualification and final promotion composition are separately reviewed and qualified.
