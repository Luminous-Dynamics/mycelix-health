# Clinical Authority Policy v1

## Purpose

Mycelix-Health already has DHT-level author binding for credential issuance and issuer-only immutable revocation. That proves who committed a credential/revocation. It does **not** by itself prove that the issuer is a legitimate licensing authority, that the credential covers the relevant jurisdiction/scope, or that a practitioner may perform a particular clinical action.

`mycelix-clinical-authority` defines the policy/evidence boundary between verified credential records and a concrete clinical authority capability.

## Three separate questions

The design keeps these distinct:

1. **Claims semantics** — what does the credential say?
2. **Credential resolution** — did a trusted adapter verify the source record, holder binding, issuer authority, validity window, and revocation state?
3. **Action policy** — does the resulting verified evidence satisfy the exact policy for this action, jurisdiction, scope, target artifact, and evaluation time?

None may substitute for another.

## Typed practitioner claims

`PractitionerLicenseClaimsV1` contains only the authority-relevant subset:

- schema version;
- coded license class;
- coded jurisdictions;
- coded scopes.

It deliberately omits names, license numbers, addresses, and other identifiers not needed for policy evaluation.

The parser uses a strict v1 JSON schema (`deny_unknown_fields`). Legacy/generic claim JSON remains readable as a credential record but cannot silently qualify for a v1 authority permit.

## Resolved credential evidence

`ResolvedCredentialEvidence` is intentionally not serializable. A trusted adapter should construct it only after checking:

- holder/principal binding;
- credential record integrity;
- issuer authority/trust for the credential kind and jurisdiction;
- validity dates;
- revocation/status evidence;
- typed jurisdiction and scope claims.

The object binds three independent evidence digests:

1. credential record digest;
2. issuer-authority evidence digest;
3. status/revocation-check evidence digest.

This matters because author binding alone does not establish that the author is a recognized licensing board or delegated authority.

## Authority request

An `AuthorityRequest` binds:

- exact principal;
- exact target artifact digest;
- exact policy digest;
- authority purpose;
- exact jurisdiction;
- required credential kinds;
- required scopes;
- evaluation time.

There is no wildcard jurisdiction behavior in v1.

## Credential composition

A policy may require multiple independent credentials.

For example, a controlled-substance prescribing policy can require both:

- `PractitionerLicense` with a prescribing scope; and
- `ControlledSubstanceRegistration` with the required schedule scope.

The current Mycelix credential zome only has a generic `PractitionerLicense` credential type, so controlled-substance registration remains a future credential/adaptor integration rather than being fabricated from the practitioner license.

## Temporal semantics

Credential evidence is evaluated at the request's exact time:

- before validity start -> not yet valid;
- at/after revocation -> revoked;
- at/after validity end -> expired;
- otherwise active.

Historical evaluation may therefore distinguish an action taken before a later revocation from one attempted after revocation.

## Opaque capability

On success, `evaluate_authority()` returns `AuthorityPermit`.

The permit intentionally implements none of:

- `Clone`;
- `Copy`;
- `Serialize`;
- `Deserialize`.

It is bound to one principal, target artifact, policy, purpose, jurisdiction, time, and supporting evidence set. Downstream code should consume it by ownership for the one authorized workflow step.

The permit is not a durable role token or wire credential.

## Integration targets

### Quarantine review

A quarantine reviewer should need:

- authority request target = exact quarantine event digest;
- purpose = `ClinicalReview` (or a narrower future reconciliation purpose);
- jurisdiction/policy derived from the clinical source/workflow;
- required practitioner/reviewer scope;
- resolved current credential evidence.

Only then should the system accept a `Pending -> UnderReview` or resolution transition as authorized.

### Medication ordering

An active `MedicationOrder` candidate should not become an actionable prescription from FHIR `intent=order` alone.

A prescribing workflow should require:

1. typed medication-order semantics;
2. target digest of that exact order;
3. appropriate jurisdiction;
4. practitioner-license credential evidence;
5. prescribing scope;
6. additional registration/schedule evidence when policy requires it;
7. an authority permit consumed by the activation/signing workflow.

## Trust-registry gap

The current health credential zome binds `issuer_did` to the committing agent and enforces issuer-only revocation. A follow-on tranche still needs a typed trust registry/policy describing **which issuers are authorized to issue which credential kinds for which jurisdictions/scopes**.

Until that exists, an adapter must not equate a self-consistent issuer DID with a recognized licensing authority.

## Non-goals

v1 does not:

- verify a licensing board over the public internet;
- define national/state professional licensing law;
- define all clinical scopes globally;
- make generic credential JSON authoritative;
- make a provider clinically competent merely because a credential is active;
- authorize treatment, prescribing, dispensing, or research outside the exact policy request;
- replace human/institutional credentialing processes where legally required.

## Next

1. Add typed issuer-authority/trust registry records.
2. Build a Holochain credential adapter that resolves current credential + immutable revocation lineage and emits `ResolvedCredentialEvidence`.
3. Bind quarantine review transitions to an authority permit.
4. Bind medication activation/signing to an authority permit.
5. Extend credential kinds for controlled-substance registrations and other independent authorities rather than hiding them inside generic JSON.
6. Add supersession/rotation semantics for licensing authorities and policy artifacts.
