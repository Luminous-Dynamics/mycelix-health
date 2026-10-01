# Clinical Population Interactive Release Context V1

## Purpose

Interactive differential-privacy accounting and scientific attribution are separate concerns.
A privacy accountant can correctly charge an exact query/output while the application later
mislabels that output as belonging to a different source finding or output schema. V1 closes
that substitution gap with one domain-separated protected release-context identity.

## Context identity

`InteractivePopulationReleaseContextV1` binds exactly:

- release-policy digest;
- source-finding epistemic class;
- exact source-finding digest;
- query-specification digest;
- output-schema digest;
- public-output digest;
- exact DP-mechanism descriptor digest.

The canonical bytes are hashed under `DigestDomain::ClinicalPopulationReleaseContext`.
The variant was appended to the shared digest-domain enum so existing variant ordering is
not changed.

## Accountant binding

For the high-assurance interactive path, the protected `accountant_after` receipt MUST set
`accountant_evidence_digest` to the exact release-context digest. The trusted release gate
recomputes the context from the request and refuses authorization on any mismatch.

Therefore changing only the source finding, output schema, query, mechanism, public output,
or release policy requires a different accountant evidence commitment.

## Trust boundary

This binding proves attribution consistency between the protected accountant transition and
the interactive release request. It does NOT prove:

- that the source finding itself is scientifically correct;
- that the output schema is clinically appropriate;
- that the DP implementation is correctly implemented;
- that the canonical DHT read is globally complete;
- that a serialized trusted-release receipt is reusable authority.

Those remain separate evidence/qualification boundaries.

## Privacy

The exact release-context digest stays in the protected accountant receipt and trusted release
receipt. The public accountant-state projection continues to expose only a keyed commitment to
the full protected receipt; it does not reveal the source finding, query, schema, or output.

## Adversarial invariants

- source-finding substitution changes the release-context digest;
- output-schema substitution changes the release-context digest;
- a trusted interactive release fails if `accountant_after.accountant_evidence_digest` does not
  equal the recomputed context digest;
- a deserialized trusted release receipt rejects a context digest outside the dedicated domain;
- the context digest is preserved in the consumed trusted release receipt.

## Promotion blockers

This tranche remains draft until exact-head CI executes successfully. Broad interactive release
also remains blocked on the conductor/race qualification tracked in #92 and the legacy interactive
authority ambiguity tracked in #95. Canonical publication/deduplication remains separately tracked
in #98.
