# Provider Principal Resolution v1

## Purpose

FHIR `Practitioner` / `PractitionerRole` references are external identity claims. They must not be treated as authenticated Mycelix principals because a string happens to contain a practitioner ID, NPI, name, or display value.

`mycelix-provider-principal` defines a strict resolver between external practitioner identity and a verified Mycelix principal.

## Inputs

`PractitionerReferenceInput` accepts only:

- exact source-system namespace;
- optional FHIR `reference`;
- optional declared resource type;
- optional structured identifier (`system`, `value`).

Display text and names are intentionally absent from the resolver contract.

At least one actual reference or identifier is required.

## Verified provider bindings

`VerifiedProviderPrincipalBinding` is an in-process evidence object. It has no serde implementation and is intended to be constructed only after a trusted adapter verifies:

1. the provider record;
2. provider-record authorship / principal ownership;
3. current binding/status or revocation evidence;
4. exact external references and identifiers attached to that provider.

The binding carries exact integrity digests for the provider record, author-binding evidence, and status evidence.

## Resolution rules

### Reference

v1 accepts relative or source-bound absolute references to:

- `Practitioner/<id>`
- `PractitionerRole/<id>`

It rejects:

- URN / contained references until Bundle-aware resolution exists;
- `_history` version-specific references;
- unsupported resource types;
- absolute references belonging to a different source system;
- declared type conflicting with parsed type.

### Identifier

Identifiers match exact `(system, value)` pairs. No fuzzy matching, name matching, partial matching, or display matching exists.

### Reference + identifier together

Both claims must converge on the same active verified provider binding.

If the reference maps to one provider while the identifier maps to another, resolution fails closed.

### Ambiguity

If multiple active verified bindings satisfy the same external identity claim, v1 returns `AmbiguousPractitioner`; it never selects the first/highest-ranked record.

## Temporal behavior

Bindings may have validity and revocation times. Resolution ignores bindings that are not active at the requested evaluation time.

## Important existing gap

The current legacy provider coordinator checks that only the provider profile creator can update profiles or add licenses/certifications. Those checks are coordinator-side. The provider integrity zome does not yet reproduce those ownership/cross-entry authorization checks at DHT validation.

Therefore a production adapter must **not** construct `VerifiedProviderPrincipalBinding` from a legacy provider record merely because that record exists. It needs a DHT-enforced author-binding lineage or equivalent stronger evidence first.

A follow-on hardening tranche should move provider ownership, license attachment, certification attachment, and relationship authorization into integrity validation, following the pattern already used by the credentials author-binding work.

## Intended composition

`FHIR Practitioner reference`
→ strict parser
→ verified provider/principal binding candidates
→ exact/identifier intersection
→ ambiguity check
→ `ResolvedPractitionerPrincipal`
→ practitioner authority evaluation
→ exact clinical workflow capability

## Non-claims

Resolution proves only that the configured identity evidence unambiguously maps an external practitioner identity to a Mycelix principal. It does not establish licensure, scope of practice, prescribing authority, employment, clinical competence, or legal authorization; those remain separate authority-policy checks.
