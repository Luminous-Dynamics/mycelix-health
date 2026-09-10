# Medication Administration Occurrence Binding Attestation v1

Status: **experimental / qualification-required**.

This tranche admits the private occurrence-binding artifact from PR #69 into a DHT-valid, privacy-minimized network fact.

## Threat model

A serialized occurrence or occurrence-binding digest is not automatically trustworthy. A modified ordinary coordinator must not be able to declare that an administration receipt belongs to an arbitrary occurrence.

The integrity zome therefore requires:

1. a DHT-valid `QualifiedMedicationAdministration` action;
2. a privacy-minimized occurrence-binding attestation that exactly matches that administration's receipt/event/medication digests;
3. a DNA-pinned root authority to issue a short-lived authorization for one exact verifier;
4. the authorization to bind one exact public attestation digest and one exact private binding digest;
5. the named verifier to publish inside the authorization window.

## Fail-closed deployment

DNA properties ship with:

```yaml
medication_administration_occurrence:
  schema_version: 1
  root_authorities: []
  max_verifier_authorization_duration_micros: 900000000
```

With an empty root list, no occurrence verifier authorization can validate. A deployment must intentionally repack the DNA with trusted root AgentPubKeys.

## Public payload

The public attestation contains only:

- referenced qualified-administration action hash;
- occurrence-binding digest;
- administration receipt digest;
- administration event digest;
- administration occurrence digest;
- medication artifact digest;
- dosage index.

It does not require the patient identity, dose amount, route/site, schedule windows, PRN nonce, clinical notes, or resolver provenance to be public.

## Exact source binding

Integrity validation fetches the referenced qualified-administration record with `must_get_valid_record` and requires its receipt/event/medication digests to exactly match the occurrence attestation. An occurrence binding therefore cannot float free of the DHT administration publication it classifies.

## Exact-target verifier authorization

A DNA-rooted authorization is bound to:

- exact administration action;
- exact attestation digest;
- exact private occurrence-binding digest;
- exact administration receipt/event/occurrence/medication digests;
- exact dosage index;
- exact verifier;
- short validity interval.

The maximum v1 interval is 15 minutes and deployments may choose less.

## Corrections

Occurrence bindings are append-only. Updates/deletes are rejected. Corrections are separate records with typed reasons:

- `EnteredInError`;
- `WrongOccurrence`;
- `SourceEvidenceRevoked`;
- `DuplicateDocumentation`;
- `Other` with protected rationale commitment.

A future occurrence-aware reducer must preserve the same distinction used elsewhere: duplicate-documentation is publication-scoped; semantic correction invalidates the binding lineage.

## Non-claims

This zome does not prove:

- the physical medication administration occurred;
- the private occurrence-binding bytes from their digest alone;
- institutional trust in a schedule resolver by itself;
- global query completeness;
- causal effect of administration on later observations.

The DNA-rooted verifier is explicitly attesting that it revalidated the protected binding/evidence before requesting authorization.

## Qualification gates

Before promotion require:

- exact-head native workspace build/test/fmt/clippy;
- WASM build of both occurrence zomes;
- empty-root fail-closed conductor test;
- non-root authorization rejection;
- wrong verifier rejection;
- expired/future authorization rejection;
- changed attestation/receipt/event/occurrence/medication/dosage rejection;
- wrong referenced administration rejection;
- update/delete rejection;
- correction-authority and cross-lineage link rejection;
- modified-coordinator adversarial test;
- reducer tests proving caller-selected `administration_id` no longer controls occurrence conflict grouping.
